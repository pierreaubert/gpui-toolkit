//! Compute-stage contract for mesh reductions and contour preparation.
//!
//! The production renderer can replace the reference implementation with a
//! wgpu/Metal dispatch without changing callers.  Keeping the reference path
//! here is intentional: it gives headless builds the exact tie-break and
//! shared-edge semantics used by [`crate::mesh::MarchingTriangles`], and makes
//! CPU/GPU differential tests meaningful on machines without an adapter.

use crate::mesh::{
    ContourBand, IsolineSegment, MarchingTriangles, MeshTopology, MeshValidationError, ScalarField,
    TriangleMesh,
};
use std::sync::Arc;

struct AdapterCompute {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    field_pipeline: wgpu::ComputePipeline,
    field_bind_group_layout: wgpu::BindGroupLayout,
    edge_pipeline: wgpu::ComputePipeline,
    triangle_pipeline: wgpu::ComputePipeline,
    contour_bind_group_layout: wgpu::BindGroupLayout,
}

fn storage_binding(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn uniform_binding(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

impl AdapterCompute {
    fn try_new() -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .ok()?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("MeshCompute device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::default(),
        }))
        .ok()?;
        let device = Arc::new(device);
        let queue = Arc::new(queue);
        let field_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("mesh_compute_field_layout"),
                entries: &[storage_binding(0, true), storage_binding(1, false)],
            });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mesh_compute_field_pipeline_layout"),
            bind_group_layouts: &[Some(&field_bind_group_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mesh_compute_shader"),
            source: wgpu::ShaderSource::Wgsl(super::compute_shaders::MESH_COMPUTE_WGSL.into()),
        });
        let field_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("mesh_compute_field_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("field_min_max"),
            compilation_options: Default::default(),
            cache: None,
        });
        let contour_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("mesh_compute_contour_layout"),
                entries: &[
                    storage_binding(0, true),
                    storage_binding(2, true),
                    storage_binding(3, true),
                    storage_binding(4, false),
                    uniform_binding(5),
                    storage_binding(6, true),
                    storage_binding(7, false),
                ],
            });
        let contour_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("mesh_compute_contour_pipeline_layout"),
                bind_group_layouts: &[Some(&contour_bind_group_layout)],
                immediate_size: 0,
            });
        let edge_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("mesh_compute_edge_pipeline"),
            layout: Some(&contour_pipeline_layout),
            module: &shader,
            entry_point: Some("edge_intersections"),
            compilation_options: Default::default(),
            cache: None,
        });
        let triangle_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("mesh_compute_triangle_pipeline"),
            layout: Some(&contour_pipeline_layout),
            module: &shader,
            entry_point: Some("triangle_segments"),
            compilation_options: Default::default(),
            cache: None,
        });
        Some(Self {
            device,
            queue,
            field_pipeline,
            field_bind_group_layout,
            edge_pipeline,
            triangle_pipeline,
            contour_bind_group_layout,
        })
    }

    fn field_min_max(&self, values: &[f32]) -> Result<Option<[f32; 2]>, ()> {
        if values.is_empty() {
            return Ok(None);
        }
        let workgroups = values.len().div_ceil(256);
        let input = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh_compute_values"),
            size: (values.len() * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&input, 0, bytemuck::cast_slice(values));
        let partials = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh_compute_partials"),
            size: (workgroups * 16) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh_compute_partials_readback"),
            size: (workgroups * 16) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mesh_compute_field_bind_group"),
            layout: &self.field_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: partials.as_entire_binding(),
                },
            ],
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("mesh_compute_field_encoder"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("mesh_compute_field_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.field_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups as u32, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&partials, 0, &staging, 0, (workgroups * 16) as u64);
        self.queue.submit(std::iter::once(encoder.finish()));

        let slice = staging.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        let _ = self.device.poll(wgpu::PollType::Wait {
            submission_index: Default::default(),
            timeout: Some(std::time::Duration::from_secs(5)),
        });
        if !matches!(
            receiver.recv_timeout(std::time::Duration::from_secs(5)),
            Ok(Ok(()))
        ) {
            return Err(());
        }
        let data = slice.get_mapped_range();
        let mut range = [f32::INFINITY, f32::NEG_INFINITY];
        let mut valid = false;
        for partial in data.chunks_exact(16) {
            let minimum = f32::from_le_bytes(partial[0..4].try_into().map_err(|_| ())?);
            let maximum = f32::from_le_bytes(partial[4..8].try_into().map_err(|_| ())?);
            let has_values = u32::from_le_bytes(partial[8..12].try_into().map_err(|_| ())?) != 0;
            if has_values {
                range[0] = range[0].min(minimum);
                range[1] = range[1].max(maximum);
                valid = true;
            }
        }
        drop(data);
        staging.unmap();
        Ok(valid.then_some(range))
    }

    /// Run the two-pass contour pipeline and read one fixed-size slot per
    /// triangle. Slots are deliberately indexed by triangle rather than
    /// appended atomically, so workgroup scheduling cannot change the CPU
    /// golden-reference order.
    fn marching_segments(
        &self,
        mesh: &TriangleMesh,
        field: &ScalarField,
        topology: &MeshTopology,
        levels: &[f32],
    ) -> Result<Vec<IsolineSegment>, ()> {
        if levels.is_empty() {
            return Ok(Vec::new());
        }
        if mesh.positions.is_empty()
            || mesh.triangles.is_empty()
            || field.values.len() != mesh.positions.len()
            || topology.unique_edges.is_empty()
            || topology.triangle_edges.len() != mesh.triangles.len()
        {
            return Err(());
        }
        if let Some(valid) = field.valid.as_deref()
            && valid.len() != field.values.len()
        {
            return Err(());
        }

        // Rebase positions around the bounds midpoint before converting to
        // f32. This preserves useful mantissa bits for large world-space
        // coordinates; the origin is restored after readback.
        let mut bounds_min = [f64::INFINITY; 3];
        let mut bounds_max = [f64::NEG_INFINITY; 3];
        for position in mesh.positions.iter() {
            for axis in 0..3 {
                let value = position[axis];
                if !value.is_finite() {
                    return Err(());
                }
                bounds_min[axis] = bounds_min[axis].min(value);
                bounds_max[axis] = bounds_max[axis].max(value);
            }
        }
        let origin: [f64; 3] =
            std::array::from_fn(|axis| bounds_min[axis] * 0.5 + bounds_max[axis] * 0.5);
        let positions: Vec<[f32; 4]> = mesh
            .positions
            .iter()
            .map(|position| {
                let relative = [
                    (position[0] - origin[0]) as f32,
                    (position[1] - origin[1]) as f32,
                    (position[2] - origin[2]) as f32,
                ];
                if relative.iter().all(|value| value.is_finite()) {
                    [relative[0], relative[1], relative[2], 0.0]
                } else {
                    [f32::NAN; 4]
                }
            })
            .collect();
        if positions
            .iter()
            .any(|position| !position[..3].iter().all(|value| value.is_finite()))
        {
            return Err(());
        }

        let valid = field.valid.as_deref();
        let mut values = Vec::with_capacity(field.values.len());
        for (index, &value) in field.values.iter().enumerate() {
            if valid.is_some_and(|mask| !mask[index]) {
                // The gather pass then naturally suppresses every triangle
                // touching a masked vertex: only its valid-valid edge can
                // produce a hit, so it cannot reach the two-hit threshold.
                values.push(f32::NAN);
            } else {
                let value = value as f32;
                if !value.is_finite() {
                    return Err(());
                }
                values.push(value);
            }
        }

        let edge_count = topology.unique_edges.len();
        let triangle_count = topology.triangle_edges.len();
        if edge_count > u32::MAX as usize || triangle_count > u32::MAX as usize {
            return Err(());
        }
        if topology.unique_edges.iter().any(|[a, b]| {
            *a as usize >= positions.len()
                || *b as usize >= positions.len()
                || *a as usize >= values.len()
                || *b as usize >= values.len()
        }) || topology
            .triangle_edges
            .iter()
            .any(|edges| edges.iter().any(|&edge| edge as usize >= edge_count))
        {
            return Err(());
        }

        let create_buffer = |label: &str, bytes: &[u8], usage: wgpu::BufferUsages| {
            let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: bytes.len() as u64,
                usage: usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.queue.write_buffer(&buffer, 0, bytes);
            buffer
        };
        let values_buffer = create_buffer(
            "mesh_compute_contour_values",
            bytemuck::cast_slice(&values),
            wgpu::BufferUsages::STORAGE,
        );
        let positions_buffer = create_buffer(
            "mesh_compute_contour_positions",
            bytemuck::cast_slice(&positions),
            wgpu::BufferUsages::STORAGE,
        );
        let edges_buffer = create_buffer(
            "mesh_compute_contour_edges",
            bytemuck::cast_slice(&topology.unique_edges),
            wgpu::BufferUsages::STORAGE,
        );
        let triangle_edges_buffer = create_buffer(
            "mesh_compute_contour_triangle_edges",
            bytemuck::cast_slice(&topology.triangle_edges),
            wgpu::BufferUsages::STORAGE,
        );
        let edge_hits_size = edge_count.checked_mul(32).ok_or(())? as u64;
        let segments_size = triangle_count.checked_mul(48).ok_or(())? as u64;
        let edge_hits = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh_compute_contour_edge_hits"),
            size: edge_hits_size,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let segments = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh_compute_contour_segments"),
            size: segments_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh_compute_contour_segments_readback"),
            size: segments_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let level_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh_compute_contour_level"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mesh_compute_contour_bind_group"),
            layout: &self.contour_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: values_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: positions_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: edges_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: edge_hits.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: level_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: triangle_edges_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: segments.as_entire_binding(),
                },
            ],
        });

        let mut output = Vec::new();
        for &level in levels {
            if !level.is_finite() {
                return Err(());
            }
            self.queue.write_buffer(
                &level_buffer,
                0,
                bytemuck::cast_slice(&[level, 0.0_f32, 0.0, 0.0]),
            );
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("mesh_compute_contour_encoder"),
                });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("mesh_compute_edge_pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.edge_pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(edge_count.div_ceil(256) as u32, 1, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("mesh_compute_triangle_pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.triangle_pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(triangle_count.div_ceil(256) as u32, 1, 1);
            }
            encoder.copy_buffer_to_buffer(&segments, 0, &staging, 0, segments_size);
            self.queue.submit(std::iter::once(encoder.finish()));

            let slice = staging.slice(..);
            let (sender, receiver) = std::sync::mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
            let _ = self.device.poll(wgpu::PollType::Wait {
                submission_index: Default::default(),
                timeout: Some(std::time::Duration::from_secs(5)),
            });
            if !matches!(
                receiver.recv_timeout(std::time::Duration::from_secs(5)),
                Ok(Ok(()))
            ) {
                return Err(());
            }
            let data = slice.get_mapped_range();
            for chunk in data.chunks_exact(48) {
                let valid = u32::from_le_bytes([chunk[32], chunk[33], chunk[34], chunk[35]]);
                if valid == 0 {
                    continue;
                }
                let start = [
                    f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as f64 + origin[0],
                    f32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]) as f64 + origin[1],
                ];
                let end = [
                    f32::from_le_bytes([chunk[16], chunk[17], chunk[18], chunk[19]]) as f64
                        + origin[0],
                    f32::from_le_bytes([chunk[20], chunk[21], chunk[22], chunk[23]]) as f64
                        + origin[1],
                ];
                if !start
                    .iter()
                    .chain(end.iter())
                    .all(|value| value.is_finite())
                {
                    drop(data);
                    staging.unmap();
                    return Err(());
                }
                output.push(IsolineSegment {
                    level: level as f64,
                    start,
                    end,
                });
            }
            drop(data);
            staging.unmap();
        }
        Ok(output)
    }
}

/// A mesh compute service.
pub struct MeshCompute {
    /// True when no adapter-backed reduction is available.
    ///
    /// Contour preparation remains on the CPU golden path even when this is
    /// false; the field lets a host report the reduction backend explicitly.
    pub reference_backend: bool,
    adapter: Option<AdapterCompute>,
}

impl MeshCompute {
    /// Construct a compute service.
    ///
    /// This returns `Some` even without a graphics adapter. Adapter-backed
    /// reductions and isoline readback are opportunistic; all operations keep
    /// the CPU golden implementation as a fallback.
    #[must_use]
    pub fn try_new() -> Option<Self> {
        let adapter = AdapterCompute::try_new();
        Some(Self {
            reference_backend: adapter.is_none(),
            adapter,
        })
    }

    /// Whether at least one operation is backed by an adapter compute pass.
    #[must_use]
    pub fn adapter_backed(&self) -> bool {
        self.adapter.is_some()
    }

    /// Return the finite min/max of a field, ignoring NaN and infinities.
    #[must_use]
    pub fn field_min_max(&self, values: &[f32]) -> Option<[f32; 2]> {
        if let Some(adapter) = &self.adapter {
            if let Ok(result) = adapter.field_min_max(values) {
                return result;
            }
        }
        let mut range = [f32::INFINITY, f32::NEG_INFINITY];
        for &value in values {
            if value.is_finite() {
                range[0] = range[0].min(value);
                range[1] = range[1].max(value);
            }
        }
        range[0].is_finite().then_some(range)
    }

    /// Produce isolines using adapter compute when available, with the CPU
    /// golden-reference implementation as the deterministic fallback.
    pub fn marching_segments(
        &self,
        mesh: &TriangleMesh,
        field: &ScalarField,
        topology: &MeshTopology,
        levels: &[f32],
    ) -> Result<Vec<IsolineSegment>, MeshValidationError> {
        validate_contour_inputs(mesh, field, levels)?;
        let marching = MarchingTriangles::new(
            mesh,
            field,
            topology,
            crate::mesh::CoordinateAxis::X,
            crate::mesh::CoordinateAxis::Y,
        )?;
        if let Some(adapter) = &self.adapter
            && let Ok(segments) = adapter.marching_segments(mesh, field, topology, levels)
        {
            return Ok(segments);
        }
        let levels = levels.iter().map(|&level| level as f64).collect::<Vec<_>>();
        Ok(marching.isolines(&levels))
    }

    /// Produce filled contour bands using the CPU golden-reference clipper.
    pub fn band_triangles(
        &self,
        mesh: &TriangleMesh,
        field: &ScalarField,
        topology: &MeshTopology,
        levels: &[f32],
    ) -> Result<Vec<ContourBand>, MeshValidationError> {
        validate_contour_inputs(mesh, field, levels)?;
        let marching = MarchingTriangles::new(
            mesh,
            field,
            topology,
            crate::mesh::CoordinateAxis::X,
            crate::mesh::CoordinateAxis::Y,
        )?;
        let levels = levels.iter().map(|&level| level as f64).collect::<Vec<_>>();
        Ok(marching.filled_bands(&levels))
    }

    /// Flatten the already-deduplicated edge list for an upload.
    #[must_use]
    pub fn unique_edge_indices(&self, topology: &MeshTopology) -> Vec<u32> {
        topology
            .unique_edges
            .iter()
            .flat_map(|edge| edge.iter().copied())
            .collect()
    }
}

/// Validate inputs before either backend touches topology-indexed arrays.
/// `MarchingTriangles` intentionally assumes validated mesh/field contracts;
/// the compute service is a public boundary and must not turn an adapter miss
/// into a CPU indexing panic.
fn validate_contour_inputs(
    mesh: &TriangleMesh,
    field: &ScalarField,
    levels: &[f32],
) -> Result<(), MeshValidationError> {
    if mesh.positions.is_empty() {
        return Err(MeshValidationError::EmptyPositions);
    }
    if mesh.triangles.is_empty() {
        return Err(MeshValidationError::EmptyTriangles);
    }
    if field.values.len() != mesh.positions.len() {
        return Err(MeshValidationError::FieldLengthMismatch {
            field_id: field.id.to_string(),
            values: field.values.len(),
            expected: mesh.positions.len(),
            association: field.association,
        });
    }
    if let Some(valid) = field.valid.as_deref()
        && valid.len() != field.values.len()
    {
        return Err(MeshValidationError::MaskLengthMismatch {
            mask: valid.len(),
            values: field.values.len(),
        });
    }
    for (index, &value) in field.values.iter().enumerate() {
        if field.valid.as_deref().is_some_and(|valid| !valid[index]) {
            continue;
        }
        if !value.is_finite() {
            return Err(MeshValidationError::NonFiniteValue { index });
        }
    }
    if levels.iter().any(|level| !level.is_finite()) {
        return Err(MeshValidationError::InvalidContourLevels);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn fixture() -> (TriangleMesh, ScalarField, MeshTopology) {
        let mesh = TriangleMesh {
            id: "compute".into(),
            positions: Arc::from([
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ]),
            triangles: Arc::from([[0, 1, 2], [1, 3, 2]]),
            vertex_ids: None,
            cell_ids: None,
        };
        let field = ScalarField {
            id: "field".into(),
            label: "field".into(),
            unit: None,
            values: Arc::from([0.0, 1.0, 0.0, 1.0]),
            association: crate::mesh::ScalarAssociation::Vertex,
            valid: None,
        };
        let topology = MeshTopology::build(&mesh.triangles);
        (mesh, field, topology)
    }

    #[test]
    fn reference_reduction_ignores_nan() {
        let compute = MeshCompute::try_new().unwrap();
        assert_eq!(
            compute.field_min_max(&[f32::NAN, -2.0, 4.0]),
            Some([-2.0, 4.0])
        );
        assert_eq!(compute.field_min_max(&[f32::NAN]), None);
    }

    #[test]
    fn reference_marching_matches_shared_edge_count() {
        let (mesh, field, topology) = fixture();
        let segments = MeshCompute::try_new()
            .unwrap()
            .marching_segments(&mesh, &field, &topology, &[0.5])
            .unwrap();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].start, segments[1].end);
    }

    #[test]
    fn reference_edge_indices_are_deduplicated() {
        let (_, _, topology) = fixture();
        let indices = MeshCompute::try_new()
            .unwrap()
            .unique_edge_indices(&topology);
        assert_eq!(indices.len(), topology.unique_edges.len() * 2);
    }

    #[test]
    fn shader_contract_has_edge_indexed_gather_and_on_level_tie_break() {
        let wgsl = super::super::compute_shaders::MESH_COMPUTE_WGSL;
        let msl = super::super::compute_shaders::MESH_COMPUTE_MSL;
        assert!(wgsl.contains("fn field_min_max"));
        assert!(wgsl.contains("fn edge_intersections"));
        assert!(wgsl.contains("fn triangle_segments"));
        assert!(wgsl.contains("(a >= level) == (b >= level)"));
        assert!(wgsl.contains("position: vec4<f32>"));
        assert!(wgsl.contains("edge_hits[triangle.e0]"));
        assert!(msl.contains("kernel void triangle_segments"));
    }
}
