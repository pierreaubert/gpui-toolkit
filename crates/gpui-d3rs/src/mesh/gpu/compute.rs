//! Compute-stage contract for mesh reductions and contour preparation.
//!
//! The production renderer can replace the reference implementation with a
//! wgpu/Metal dispatch without changing callers.  Keeping the reference path
//! here is intentional: it gives headless builds the exact tie-break and
//! shared-edge semantics used by [`crate::mesh::MarchingTriangles`], and makes
//! CPU/GPU differential tests meaningful on machines without an adapter.

use crate::mesh::{
    ContourBand, CoordinateAxis, IsolineSegment, MarchingTriangles, MeshTopology,
    MeshValidationError, ScalarField, TriangleMesh, project_2d,
};
use std::cell::Cell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock};

const COMPUTE_TIMESTAMP_QUERY_COUNT: u32 = 2;
const COMPUTE_TIMESTAMP_QUERY_BYTES: u64 =
    COMPUTE_TIMESTAMP_QUERY_COUNT as u64 * std::mem::size_of::<u64>() as u64;

/// Optional adapter timestamp instrumentation for one synchronous compute
/// operation. Compute methods already wait for their readback buffer, so the
/// timestamp readback can be resolved in the same submission without adding a
/// second asynchronous state machine. The normal path remains unchanged until
/// `SOTF_GPU_TIMESTAMPS=1` is explicitly requested.
struct AdapterComputeTiming {
    query_set: Option<wgpu::QuerySet>,
    resolve_buffer: Option<wgpu::Buffer>,
    readback_buffer: Option<wgpu::Buffer>,
    timestamp_period_ns: f32,
    last_gpu_time_ns: Cell<u64>,
    gpu_time_count: Cell<u64>,
}

impl AdapterComputeTiming {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, enabled: bool) -> Self {
        if !enabled {
            return Self {
                query_set: None,
                resolve_buffer: None,
                readback_buffer: None,
                timestamp_period_ns: 0.0,
                last_gpu_time_ns: Cell::new(0),
                gpu_time_count: Cell::new(0),
            };
        }

        Self {
            query_set: Some(device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("mesh_compute_timestamps"),
                count: COMPUTE_TIMESTAMP_QUERY_COUNT,
                ty: wgpu::QueryType::Timestamp,
            })),
            resolve_buffer: Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("mesh_compute_timestamp_resolve"),
                size: COMPUTE_TIMESTAMP_QUERY_BYTES,
                usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::QUERY_RESOLVE,
                mapped_at_creation: false,
            })),
            readback_buffer: Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("mesh_compute_timestamp_readback"),
                size: COMPUTE_TIMESTAMP_QUERY_BYTES,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            })),
            timestamp_period_ns: queue.get_timestamp_period(),
            last_gpu_time_ns: Cell::new(0),
            gpu_time_count: Cell::new(0),
        }
    }

    fn enabled(&self) -> bool {
        self.query_set.is_some()
    }

    fn writes(
        &self,
        beginning: Option<u32>,
        end: Option<u32>,
    ) -> Option<wgpu::ComputePassTimestampWrites<'_>> {
        self.query_set
            .as_ref()
            .map(|query_set| wgpu::ComputePassTimestampWrites {
                query_set,
                beginning_of_pass_write_index: beginning,
                end_of_pass_write_index: end,
            })
    }

    fn finish(&self, encoder: &mut wgpu::CommandEncoder, active: bool) {
        if !active {
            return;
        }
        let (Some(query_set), Some(resolve_buffer), Some(readback_buffer)) = (
            self.query_set.as_ref(),
            self.resolve_buffer.as_ref(),
            self.readback_buffer.as_ref(),
        ) else {
            return;
        };
        encoder.resolve_query_set(
            query_set,
            0..COMPUTE_TIMESTAMP_QUERY_COUNT,
            resolve_buffer,
            0,
        );
        encoder.copy_buffer_to_buffer(
            resolve_buffer,
            0,
            readback_buffer,
            0,
            COMPUTE_TIMESTAMP_QUERY_BYTES,
        );
    }

    fn readback(&self, device: &wgpu::Device) {
        let Some(readback_buffer) = self.readback_buffer.as_ref() else {
            return;
        };
        let slice = readback_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        let _ = device.poll(wgpu::PollType::Wait {
            submission_index: Default::default(),
            timeout: Some(std::time::Duration::from_secs(5)),
        });
        if !matches!(
            receiver.recv_timeout(std::time::Duration::from_secs(5)),
            Ok(Ok(()))
        ) {
            return;
        }
        let data = slice.get_mapped_range();
        let Some(timestamps) = bytemuck::try_cast_slice::<u8, u64>(&data).ok() else {
            drop(data);
            readback_buffer.unmap();
            return;
        };
        let Some(&[start, end]) = timestamps.get(..2) else {
            drop(data);
            readback_buffer.unmap();
            return;
        };
        let ticks = end.wrapping_sub(start);
        let nanos = (ticks as f64 * f64::from(self.timestamp_period_ns)).round();
        drop(data);
        readback_buffer.unmap();
        if nanos.is_finite() && nanos > 0.0 {
            self.last_gpu_time_ns.set(nanos.min(u64::MAX as f64) as u64);
            self.gpu_time_count
                .set(self.gpu_time_count.get().saturating_add(1));
        }
    }

    fn last_gpu_time_ns(&self) -> u64 {
        self.last_gpu_time_ns.get()
    }

    fn gpu_time_count(&self) -> u64 {
        self.gpu_time_count.get()
    }
}

struct AdapterCompute {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    backend: wgpu::Backend,
    timing: AdapterComputeTiming,
    field_pipeline: wgpu::ComputePipeline,
    field_bind_group_layout: wgpu::BindGroupLayout,
    edge_pipeline: wgpu::ComputePipeline,
    triangle_pipeline: wgpu::ComputePipeline,
    band_pipeline: wgpu::ComputePipeline,
    contour_bind_group_layout: wgpu::BindGroupLayout,
    contour_inputs: Mutex<Option<CachedComputeInputs>>,
    band_inputs: Mutex<Option<CachedComputeInputs>>,
}

/// GPU-resident immutable inputs for one contour topology and scalar field.
/// The cache deliberately excludes contour levels, which are copied through a
/// small uniform buffer for every dispatch batch.
struct CachedComputeInputs {
    fingerprint: u64,
    values: Arc<wgpu::Buffer>,
    positions: Arc<wgpu::Buffer>,
    edges: Arc<wgpu::Buffer>,
    topology: Arc<wgpu::Buffer>,
}

fn compute_input_fingerprint(
    values: &[f32],
    positions: &[[f32; 4]],
    edges: &[[u32; 2]],
    topology: &[[u32; 3]],
) -> u64 {
    let mut hasher = DefaultHasher::new();
    values.len().hash(&mut hasher);
    positions.len().hash(&mut hasher);
    edges.len().hash(&mut hasher);
    topology.len().hash(&mut hasher);
    for &value in values {
        value.to_bits().hash(&mut hasher);
    }
    for position in positions {
        for &value in position {
            value.to_bits().hash(&mut hasher);
        }
    }
    edges.hash(&mut hasher);
    topology.hash(&mut hasher);
    hasher.finish()
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
        let backend = adapter.get_info().backend;
        // The wgpu/Naga version used by this workspace lowers this shader via
        // FXC on DX11. FXC cannot compile its storage-buffer kernel, whereas
        // the CPU implementation is the documented golden-reference fallback.
        // Do not create a pipeline that would panic during initialization.
        if backend == wgpu::Backend::Dx11 {
            return None;
        }
        let timestamp_features = adapter.features();
        let timing_enabled = std::env::var_os("SOTF_GPU_TIMESTAMPS").as_deref()
            == Some(std::ffi::OsStr::new("1"))
            && timestamp_features.contains(wgpu::Features::TIMESTAMP_QUERY)
            && timestamp_features.contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES);
        let required_features = if timing_enabled {
            wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES
        } else {
            wgpu::Features::empty()
        };
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("MeshCompute device"),
            required_features,
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::default(),
        }))
        .ok()?;
        let device = Arc::new(device);
        let queue = Arc::new(queue);
        let timing = AdapterComputeTiming::new(&device, &queue, timing_enabled);
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
        let band_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("mesh_compute_band_pipeline"),
            layout: Some(&contour_pipeline_layout),
            module: &shader,
            entry_point: Some("triangle_bands"),
            compilation_options: Default::default(),
            cache: None,
        });
        Some(Self {
            device,
            queue,
            backend,
            timing,
            field_pipeline,
            field_bind_group_layout,
            edge_pipeline,
            triangle_pipeline,
            band_pipeline,
            contour_bind_group_layout,
            contour_inputs: Mutex::new(None),
            band_inputs: Mutex::new(None),
        })
    }

    fn field_min_max(&self, values: &[f32]) -> Result<Option<[f32; 2]>, ()> {
        if values.is_empty() {
            return Ok(None);
        }
        let workgroups = values.len().div_ceil(256);
        let input = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh_compute_values"),
            size: std::mem::size_of_val(values) as u64,
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
        let timing_active = self.timing.enabled();
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("mesh_compute_field_pass"),
                timestamp_writes: self.timing.writes(Some(0), Some(1)),
            });
            pass.set_pipeline(&self.field_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups as u32, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&partials, 0, &staging, 0, (workgroups * 16) as u64);
        self.timing.finish(&mut encoder, timing_active);
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
        self.timing.readback(&self.device);
        Ok(valid.then_some(range))
    }

    /// Run the two-pass contour pipeline and read one fixed-size slot per
    /// triangle. Slots are deliberately indexed by triangle rather than
    /// appended atomically, so workgroup scheduling cannot change the CPU
    /// golden-reference order.
    fn marching_segments_indexed(
        &self,
        mesh: &TriangleMesh,
        field: &ScalarField,
        topology: &MeshTopology,
        levels: &[f32],
    ) -> Result<Vec<(usize, IsolineSegment)>, ()> {
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

        let input_fingerprint = compute_input_fingerprint(
            &values,
            &positions,
            &topology.unique_edges,
            &topology.triangle_edges,
        );
        let (values_buffer, positions_buffer, edges_buffer, triangle_edges_buffer) = {
            let mut cached = self.contour_inputs.lock().map_err(|_| ())?;
            let cache_miss = match cached.as_ref() {
                Some(inputs) => inputs.fingerprint != input_fingerprint,
                None => true,
            };
            if cache_miss {
                let create_buffer = |label: &str, bytes: &[u8], usage: wgpu::BufferUsages| {
                    let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some(label),
                        size: bytes.len() as u64,
                        usage: usage | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    self.queue.write_buffer(&buffer, 0, bytes);
                    Arc::new(buffer)
                };
                *cached = Some(CachedComputeInputs {
                    fingerprint: input_fingerprint,
                    values: create_buffer(
                        "mesh_compute_contour_values",
                        bytemuck::cast_slice(&values),
                        wgpu::BufferUsages::STORAGE,
                    ),
                    positions: create_buffer(
                        "mesh_compute_contour_positions",
                        bytemuck::cast_slice(&positions),
                        wgpu::BufferUsages::STORAGE,
                    ),
                    edges: create_buffer(
                        "mesh_compute_contour_edges",
                        bytemuck::cast_slice(&topology.unique_edges),
                        wgpu::BufferUsages::STORAGE,
                    ),
                    topology: create_buffer(
                        "mesh_compute_contour_triangle_edges",
                        bytemuck::cast_slice(&topology.triangle_edges),
                        wgpu::BufferUsages::STORAGE,
                    ),
                });
            }
            let inputs = cached.as_ref().ok_or(())?;
            (
                Arc::clone(&inputs.values),
                Arc::clone(&inputs.positions),
                Arc::clone(&inputs.edges),
                Arc::clone(&inputs.topology),
            )
        };
        let edge_hits_size = edge_count.checked_mul(32).ok_or(())? as u64;
        let segments_size = triangle_count.checked_mul(112).ok_or(())? as u64;
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
        let staging_size = segments_size
            .checked_mul(u64::try_from(levels.len()).map_err(|_| ())?)
            .ok_or(())?;
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh_compute_contour_segments_readback"),
            size: staging_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let level_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh_compute_contour_level"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut level_sources = Vec::with_capacity(levels.len());
        for &level in levels {
            if !level.is_finite() {
                return Err(());
            }
            let source = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("mesh_compute_contour_level_source"),
                size: 16,
                usage: wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.queue.write_buffer(
                &source,
                0,
                bytemuck::cast_slice(&[level, 0.0_f32, 0.0, 0.0]),
            );
            level_sources.push(source);
        }
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

        let timing_active = self.timing.enabled();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("mesh_compute_contour_encoder"),
            });
        for (level_index, source) in level_sources.iter().enumerate() {
            encoder.copy_buffer_to_buffer(source, 0, &level_buffer, 0, 16);
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("mesh_compute_edge_pass"),
                    timestamp_writes: self.timing.writes((level_index == 0).then_some(0), None),
                });
                pass.set_pipeline(&self.edge_pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(edge_count.div_ceil(256) as u32, 1, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("mesh_compute_triangle_pass"),
                    timestamp_writes: self
                        .timing
                        .writes(None, (level_index + 1 == level_sources.len()).then_some(1)),
                });
                pass.set_pipeline(&self.triangle_pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(triangle_count.div_ceil(256) as u32, 1, 1);
            }
            let output_offset = u64::try_from(level_index)
                .map_err(|_| ())?
                .checked_mul(segments_size)
                .ok_or(())?;
            encoder.copy_buffer_to_buffer(&segments, 0, &staging, output_offset, segments_size);
        }
        self.timing.finish(&mut encoder, timing_active);
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
        let decoded = (|| -> Result<Vec<(usize, IsolineSegment)>, ()> {
            let bytes_per_level = usize::try_from(segments_size).map_err(|_| ())?;
            let mut output = Vec::new();
            for (level_index, &level) in levels.iter().enumerate() {
                let byte_start = level_index.checked_mul(bytes_per_level).ok_or(())?;
                let byte_end = byte_start.checked_add(bytes_per_level).ok_or(())?;
                let level_data = data.get(byte_start..byte_end).ok_or(())?;
                for chunk in level_data.chunks_exact(112) {
                    let valid = u32::from_le_bytes([chunk[96], chunk[97], chunk[98], chunk[99]]);
                    if valid == 0 {
                        continue;
                    }
                    let start = [
                        f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as f64
                            + origin[0],
                        f32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]) as f64
                            + origin[1],
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
                        return Err(());
                    }
                    output.push((
                        level_index,
                        IsolineSegment {
                            level: level as f64,
                            start,
                            end,
                        },
                    ));
                }
            }
            Ok(output)
        })();
        drop(data);
        staging.unmap();
        self.timing.readback(&self.device);
        decoded
    }

    fn marching_segments(
        &self,
        mesh: &TriangleMesh,
        field: &ScalarField,
        topology: &MeshTopology,
        levels: &[f32],
    ) -> Result<Vec<IsolineSegment>, ()> {
        self.marching_segments_indexed(mesh, field, topology, levels)
            .map(|segments| segments.into_iter().map(|(_, segment)| segment).collect())
    }

    /// Dispatch one deterministic, fixed-size clipped-polygon slot per input
    /// triangle for each closed scalar band. This is intentionally read back
    /// rather than rendered directly: MeshPlot's retained contour cache and
    /// SVG path both consume the same `ContourBand` representation.
    fn band_triangles(
        &self,
        mesh: &TriangleMesh,
        field: &ScalarField,
        topology: &MeshTopology,
        levels: &[f32],
    ) -> Result<Vec<ContourBand>, ()> {
        if levels.len() < 2
            || mesh.positions.is_empty()
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
                [
                    (position[0] - origin[0]) as f32,
                    (position[1] - origin[1]) as f32,
                    (position[2] - origin[2]) as f32,
                    0.0,
                ]
            })
            .collect();
        if positions
            .iter()
            .any(|position| !position[..3].iter().all(|value| value.is_finite()))
        {
            return Err(());
        }
        let mut values = Vec::with_capacity(field.values.len());
        for (index, &value) in field.values.iter().enumerate() {
            if field.valid.as_deref().is_some_and(|valid| !valid[index]) {
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
        let triangle_count = mesh.triangles.len();
        if edge_count > u32::MAX as usize
            || triangle_count > u32::MAX as usize
            || topology.unique_edges.iter().any(|[a, b]| {
                *a as usize >= positions.len()
                    || *b as usize >= positions.len()
                    || *a as usize >= values.len()
                    || *b as usize >= values.len()
            })
            || mesh.triangles.iter().any(|triangle| {
                triangle
                    .iter()
                    .any(|&vertex| vertex as usize >= positions.len())
            })
        {
            return Err(());
        }
        let input_fingerprint = compute_input_fingerprint(
            &values,
            &positions,
            &topology.unique_edges,
            mesh.triangles.as_ref(),
        );
        let (values_buffer, positions_buffer, edges_buffer, triangles_buffer) = {
            let mut cached = self.band_inputs.lock().map_err(|_| ())?;
            let cache_miss = match cached.as_ref() {
                Some(inputs) => inputs.fingerprint != input_fingerprint,
                None => true,
            };
            if cache_miss {
                let create_buffer = |label: &str, bytes: &[u8], usage: wgpu::BufferUsages| {
                    let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some(label),
                        size: bytes.len() as u64,
                        usage: usage | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    self.queue.write_buffer(&buffer, 0, bytes);
                    Arc::new(buffer)
                };
                *cached = Some(CachedComputeInputs {
                    fingerprint: input_fingerprint,
                    values: create_buffer(
                        "mesh_compute_band_values",
                        bytemuck::cast_slice(&values),
                        wgpu::BufferUsages::STORAGE,
                    ),
                    positions: create_buffer(
                        "mesh_compute_band_positions",
                        bytemuck::cast_slice(&positions),
                        wgpu::BufferUsages::STORAGE,
                    ),
                    edges: create_buffer(
                        "mesh_compute_band_edges",
                        bytemuck::cast_slice(&topology.unique_edges),
                        wgpu::BufferUsages::STORAGE,
                    ),
                    topology: create_buffer(
                        "mesh_compute_band_triangles",
                        bytemuck::cast_slice(mesh.triangles.as_ref()),
                        wgpu::BufferUsages::STORAGE,
                    ),
                });
            }
            let inputs = cached.as_ref().ok_or(())?;
            (
                Arc::clone(&inputs.values),
                Arc::clone(&inputs.positions),
                Arc::clone(&inputs.edges),
                Arc::clone(&inputs.topology),
            )
        };
        let edge_hits_size = edge_count.checked_mul(32).ok_or(())? as u64;
        let output_size = triangle_count.checked_mul(112).ok_or(())? as u64;
        let edge_hits = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh_compute_band_unused_edge_hits"),
            size: edge_hits_size,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let output = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh_compute_band_output"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let band_count = levels.len() - 1;
        let staging_size = output_size
            .checked_mul(u64::try_from(band_count).map_err(|_| ())?)
            .ok_or(())?;
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh_compute_band_readback"),
            size: staging_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let levels_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh_compute_band_levels"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut band_sources = Vec::with_capacity(band_count);
        for pair in levels.windows(2) {
            let [lower, upper] = [pair[0], pair[1]];
            if !lower.is_finite() || !upper.is_finite() || lower > upper {
                return Err(());
            }
            let source = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("mesh_compute_band_levels_source"),
                size: 16,
                usage: wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.queue.write_buffer(
                &source,
                0,
                bytemuck::cast_slice(&[lower, upper, 0.0_f32, 0.0]),
            );
            band_sources.push(source);
        }
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mesh_compute_band_bind_group"),
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
                    resource: levels_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: triangles_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: output.as_entire_binding(),
                },
            ],
        });

        let timing_active = self.timing.enabled();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("mesh_compute_band_encoder"),
            });
        for (band_index, source) in band_sources.iter().enumerate() {
            encoder.copy_buffer_to_buffer(source, 0, &levels_buffer, 0, 16);
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("mesh_compute_band_pass"),
                    timestamp_writes: self.timing.writes(
                        (band_index == 0).then_some(0),
                        (band_index + 1 == band_sources.len()).then_some(1),
                    ),
                });
                pass.set_pipeline(&self.band_pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(triangle_count.div_ceil(256) as u32, 1, 1);
            }
            let output_offset = u64::try_from(band_index)
                .map_err(|_| ())?
                .checked_mul(output_size)
                .ok_or(())?;
            encoder.copy_buffer_to_buffer(&output, 0, &staging, output_offset, output_size);
        }
        self.timing.finish(&mut encoder, timing_active);
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
        let decoded = (|| -> Result<Vec<ContourBand>, ()> {
            let bytes_per_band = usize::try_from(output_size).map_err(|_| ())?;
            let mut bands = Vec::with_capacity(band_count);
            for (band_index, pair) in levels.windows(2).enumerate() {
                let [lower, upper] = [pair[0], pair[1]];
                let byte_start = band_index.checked_mul(bytes_per_band).ok_or(())?;
                let byte_end = byte_start.checked_add(bytes_per_band).ok_or(())?;
                let band_data = data.get(byte_start..byte_end).ok_or(())?;
                let mut band = ContourBand {
                    lower: Some(lower as f64),
                    upper: Some(upper as f64),
                    positions: Vec::new(),
                    triangles: Vec::new(),
                };
                for chunk in band_data.chunks_exact(112) {
                    let valid = u32::from_le_bytes(chunk[96..100].try_into().map_err(|_| ())?);
                    let count = u32::from_le_bytes(chunk[100..104].try_into().map_err(|_| ())?);
                    if valid == 0 || !(3..=6).contains(&count) {
                        continue;
                    }
                    let base = u32::try_from(band.positions.len()).map_err(|_| ())?;
                    for point in 0..count as usize {
                        let offset = point * 16;
                        let position = [
                            f32::from_le_bytes(
                                chunk[offset..offset + 4].try_into().map_err(|_| ())?,
                            ) as f64
                                + origin[0],
                            f32::from_le_bytes(
                                chunk[offset + 4..offset + 8].try_into().map_err(|_| ())?,
                            ) as f64
                                + origin[1],
                        ];
                        if !position.iter().all(|value| value.is_finite()) {
                            return Err(());
                        }
                        band.positions.push(position);
                    }
                    for offset in 1..count - 1 {
                        band.triangles
                            .push([base, base + offset, base + offset + 1]);
                    }
                }
                bands.push(band);
            }
            Ok(bands)
        })();
        drop(data);
        staging.unmap();
        self.timing.readback(&self.device);
        decoded
    }
}

/// The backend that produced the most recent reduction result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshComputeBackend {
    /// An adapter-backed compute pass produced the result.
    Adapter,
    /// The deterministic CPU reference implementation produced the result.
    CpuReference,
}

impl MeshComputeBackend {
    /// Stable machine-readable backend label for host diagnostics.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Adapter => "adapter",
            Self::CpuReference => "cpu_reference",
        }
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
    last_backend: Cell<MeshComputeBackend>,
}

/// Return the process-wide mesh compute service.
///
/// Adapter discovery, device creation, and pipeline compilation are expensive.
/// Keeping their ownership in d3rs lets every chart host share the same service
/// instead of independently constructing GPU state. The mutex also serializes
/// the service's synchronous readbacks and last-backend diagnostic state.
#[must_use]
pub fn shared_mesh_compute() -> &'static Mutex<MeshCompute> {
    static COMPUTE: OnceLock<Mutex<MeshCompute>> = OnceLock::new();
    COMPUTE.get_or_init(|| {
        Mutex::new(MeshCompute::try_new().unwrap_or_else(MeshCompute::cpu_reference))
    })
}

impl MeshCompute {
    /// Construct a deterministic CPU-only compute service.
    ///
    /// This is useful for hosts that intentionally disable adapter work and
    /// for differential tests that need to exercise fallback reporting even
    /// when the machine running the test has a usable graphics adapter.
    #[must_use]
    pub fn cpu_reference() -> Self {
        Self {
            reference_backend: true,
            adapter: None,
            last_backend: Cell::new(MeshComputeBackend::CpuReference),
        }
    }

    /// Construct a compute service.
    ///
    /// This returns `Some` even without a graphics adapter. Adapter-backed
    /// reductions and isoline readback are opportunistic; all operations keep
    /// the CPU golden implementation as a fallback.
    #[must_use]
    pub fn try_new() -> Option<Self> {
        let adapter = AdapterCompute::try_new();
        let last_backend = if adapter.is_some() {
            MeshComputeBackend::Adapter
        } else {
            MeshComputeBackend::CpuReference
        };
        Some(Self {
            reference_backend: adapter.is_none(),
            adapter,
            last_backend: Cell::new(last_backend),
        })
    }

    /// Return the best backend available when this service was constructed.
    #[must_use]
    pub fn available_backend(&self) -> MeshComputeBackend {
        if self.adapter.is_some() {
            MeshComputeBackend::Adapter
        } else {
            MeshComputeBackend::CpuReference
        }
    }

    /// Return the backend that produced the most recent reduction result.
    #[must_use]
    pub fn last_backend(&self) -> MeshComputeBackend {
        self.last_backend.get()
    }

    /// Whether at least one operation is backed by an adapter compute pass.
    #[must_use]
    pub fn adapter_backed(&self) -> bool {
        self.adapter.is_some()
    }

    /// Return the native backend used by adapter-backed compute, when one is
    /// available. This lets platform QA distinguish Metal, Vulkan, and other
    /// adapter paths while keeping CPU-reference fallback explicit.
    #[must_use]
    pub fn adapter_backend(&self) -> Option<wgpu::Backend> {
        self.adapter.as_ref().map(|adapter| adapter.backend)
    }

    /// Whether opt-in adapter timestamp instrumentation is active for this
    /// compute service. Unsupported adapters and normal runs report false.
    #[must_use]
    pub fn adapter_gpu_timing_enabled(&self) -> bool {
        self.adapter
            .as_ref()
            .is_some_and(|adapter| adapter.timing.enabled())
    }

    /// Number of completed adapter compute timestamp samples.
    #[must_use]
    pub fn adapter_gpu_time_count(&self) -> u64 {
        self.adapter
            .as_ref()
            .map_or(0, |adapter| adapter.timing.gpu_time_count())
    }

    /// Most recently completed adapter compute duration in nanoseconds.
    #[must_use]
    pub fn adapter_gpu_time_ns(&self) -> u64 {
        self.adapter
            .as_ref()
            .map_or(0, |adapter| adapter.timing.last_gpu_time_ns())
    }

    /// Return the finite min/max of a field, ignoring NaN and infinities.
    #[must_use]
    pub fn field_min_max(&self, values: &[f32]) -> Option<[f32; 2]> {
        if let Some(adapter) = &self.adapter
            && let Ok(result) = adapter.field_min_max(values)
        {
            self.last_backend.set(MeshComputeBackend::Adapter);
            return result;
        }
        self.last_backend.set(MeshComputeBackend::CpuReference);
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
            self.last_backend.set(MeshComputeBackend::Adapter);
            return Ok(segments);
        }
        self.last_backend.set(MeshComputeBackend::CpuReference);
        let levels = levels.iter().map(|&level| level as f64).collect::<Vec<_>>();
        Ok(marching.isolines(&levels))
    }

    /// Produce isolines in an arbitrary projected mesh plane. Adapter work
    /// receives a compact XY projection; when no adapter is available (or it
    /// rejects the request), the CPU golden path uses the original axes.
    ///
    /// Keeping the caller's f64 levels on the public boundary means the GPU
    /// path never leaks f32 level quantisation into exported/accessible data.
    pub fn marching_segments_projected(
        &self,
        mesh: &TriangleMesh,
        field: &ScalarField,
        topology: &MeshTopology,
        horizontal: CoordinateAxis,
        vertical: CoordinateAxis,
        levels: &[f64],
    ) -> Result<Vec<IsolineSegment>, MeshValidationError> {
        if levels.iter().any(|level| !level.is_finite()) {
            return Err(MeshValidationError::InvalidContourLevels);
        }
        let levels_f32 = levels.iter().map(|&level| level as f32).collect::<Vec<_>>();
        if levels_f32.iter().any(|level| !level.is_finite()) {
            return Err(MeshValidationError::InvalidContourLevels);
        }
        validate_contour_inputs(mesh, field, &levels_f32)?;
        if field.association != crate::mesh::ScalarAssociation::Vertex {
            return Err(MeshValidationError::ContoursRequireVertexField);
        }
        if let Some(adapter) = &self.adapter {
            let projected = TriangleMesh {
                id: mesh.id.clone(),
                positions: mesh
                    .positions
                    .iter()
                    .map(|&point| {
                        let [x, y] = project_2d(horizontal, vertical, point);
                        [x, y, 0.0]
                    })
                    .collect::<Vec<_>>()
                    .into(),
                triangles: mesh.triangles.clone(),
                vertex_ids: None,
                cell_ids: None,
            };
            // The adapter returns XY; uploading projected coordinates makes
            // those values exactly the requested chart plane.
            if let Ok(segments) =
                adapter.marching_segments_indexed(&projected, field, topology, &levels_f32)
            {
                let segments = segments
                    .into_iter()
                    .map(|(level_index, mut segment)| {
                        segment.level = levels[level_index];
                        segment
                    })
                    .collect();
                self.last_backend.set(MeshComputeBackend::Adapter);
                return Ok(segments);
            }
        }
        self.last_backend.set(MeshComputeBackend::CpuReference);
        let marching = MarchingTriangles::new(mesh, field, topology, horizontal, vertical)?;
        Ok(marching.isolines(levels))
    }

    /// Produce XY filled contour bands using adapter compute when available,
    /// with the CPU golden-reference clipper as a deterministic fallback.
    pub fn band_triangles(
        &self,
        mesh: &TriangleMesh,
        field: &ScalarField,
        topology: &MeshTopology,
        levels: &[f32],
    ) -> Result<Vec<ContourBand>, MeshValidationError> {
        validate_contour_inputs(mesh, field, levels)?;
        let levels_f64 = levels.iter().map(|&level| level as f64).collect::<Vec<_>>();
        if let Some(adapter) = &self.adapter
            && let Ok(bands) = adapter.band_triangles(mesh, field, topology, levels)
        {
            self.last_backend.set(MeshComputeBackend::Adapter);
            return Ok(bands);
        }
        self.last_backend.set(MeshComputeBackend::CpuReference);
        let marching = MarchingTriangles::new(
            mesh,
            field,
            topology,
            crate::mesh::CoordinateAxis::X,
            crate::mesh::CoordinateAxis::Y,
        )?;
        Ok(marching.filled_bands(&levels_f64))
    }

    /// Produce filled contour bands in an arbitrary projected mesh plane.
    /// The adapter receives a compact XY projection while callers retain the
    /// original f64 reported level boundaries and a CPU fallback for every
    /// unavailable/failed adapter path.
    pub fn band_triangles_projected(
        &self,
        mesh: &TriangleMesh,
        field: &ScalarField,
        topology: &MeshTopology,
        horizontal: CoordinateAxis,
        vertical: CoordinateAxis,
        levels: &[f64],
    ) -> Result<Vec<ContourBand>, MeshValidationError> {
        if levels.iter().any(|level| !level.is_finite()) {
            return Err(MeshValidationError::InvalidContourLevels);
        }
        let levels_f32 = levels.iter().map(|&level| level as f32).collect::<Vec<_>>();
        if levels_f32.iter().any(|level| !level.is_finite()) {
            return Err(MeshValidationError::InvalidContourLevels);
        }
        validate_contour_inputs(mesh, field, &levels_f32)?;
        if field.association != crate::mesh::ScalarAssociation::Vertex {
            return Err(MeshValidationError::ContoursRequireVertexField);
        }
        if let Some(adapter) = &self.adapter {
            let projected = TriangleMesh {
                id: mesh.id.clone(),
                positions: mesh
                    .positions
                    .iter()
                    .map(|&point| {
                        let [x, y] = project_2d(horizontal, vertical, point);
                        [x, y, 0.0]
                    })
                    .collect::<Vec<_>>()
                    .into(),
                triangles: mesh.triangles.clone(),
                vertex_ids: None,
                cell_ids: None,
            };
            if let Ok(mut bands) = adapter.band_triangles(&projected, field, topology, &levels_f32)
            {
                for (band, boundaries) in bands.iter_mut().zip(levels.windows(2)) {
                    band.lower = Some(boundaries[0]);
                    band.upper = Some(boundaries[1]);
                }
                self.last_backend.set(MeshComputeBackend::Adapter);
                return Ok(bands);
            }
        }
        self.last_backend.set(MeshComputeBackend::CpuReference);
        let marching = MarchingTriangles::new(mesh, field, topology, horizontal, vertical)?;
        Ok(marching.filled_bands(levels))
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
    fn compute_reports_available_and_last_backend() {
        let compute = MeshCompute::try_new().unwrap();
        assert_eq!(
            compute.reference_backend,
            matches!(
                compute.available_backend(),
                MeshComputeBackend::CpuReference
            )
        );
        assert_eq!(
            compute.adapter_backed(),
            matches!(compute.available_backend(), MeshComputeBackend::Adapter)
        );
        assert_eq!(
            compute.available_backend().as_str(),
            match compute.available_backend() {
                MeshComputeBackend::Adapter => "adapter",
                MeshComputeBackend::CpuReference => "cpu_reference",
            }
        );

        let (mesh, field, topology) = fixture();
        let _ = compute
            .marching_segments(&mesh, &field, &topology, &[0.5])
            .expect("the reference fallback must always be available");
        assert!(matches!(
            compute.last_backend(),
            MeshComputeBackend::Adapter | MeshComputeBackend::CpuReference
        ));
        if !compute.adapter_backed() {
            assert_eq!(compute.last_backend(), MeshComputeBackend::CpuReference);
        }
    }

    #[test]
    fn cpu_reference_reports_fallback_for_every_reduction_path() {
        let compute = MeshCompute::cpu_reference();
        let (mesh, field, topology) = fixture();

        assert_eq!(
            compute.available_backend(),
            MeshComputeBackend::CpuReference
        );
        assert_eq!(compute.last_backend(), MeshComputeBackend::CpuReference);
        assert_eq!(compute.field_min_max(&[0.0, 1.0]), Some([0.0, 1.0]));
        assert_eq!(compute.last_backend(), MeshComputeBackend::CpuReference);

        compute
            .marching_segments(&mesh, &field, &topology, &[0.5])
            .expect("CPU isolines");
        assert_eq!(compute.last_backend(), MeshComputeBackend::CpuReference);

        compute
            .marching_segments_projected(
                &mesh,
                &field,
                &topology,
                CoordinateAxis::X,
                CoordinateAxis::Y,
                &[0.5],
            )
            .expect("CPU projected isolines");
        assert_eq!(compute.last_backend(), MeshComputeBackend::CpuReference);

        compute
            .band_triangles(&mesh, &field, &topology, &[0.5])
            .expect("CPU filled bands");
        assert_eq!(compute.last_backend(), MeshComputeBackend::CpuReference);

        compute
            .band_triangles_projected(
                &mesh,
                &field,
                &topology,
                CoordinateAxis::X,
                CoordinateAxis::Y,
                &[0.5],
            )
            .expect("CPU projected filled bands");
        assert_eq!(compute.last_backend(), MeshComputeBackend::CpuReference);
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
        assert!(wgsl.contains("(a >= levels.lower) == (b >= levels.lower)"));
        assert!(wgsl.contains("position: vec4<f32>"));
        assert!(wgsl.contains("edge_hits[triangle.e0]"));
        assert!(msl.contains("kernel void triangle_segments"));
    }
}
