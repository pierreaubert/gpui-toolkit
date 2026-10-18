//! GPU compute for flow diagnostics through wgpu.
//!
//! The per-tetrahedron Jacobian kernel runs on the host GPU: Metal on
//! Apple Silicon, Vulkan (including CUDA-class NVIDIA adapters) on Linux,
//! Direct3D/Vulkan on Windows, and WebGPU in the browser. Point averaging
//! and the vorticity/Q/divergence reductions stay on the host in `f64`,
//! so GPU results agree with the CPU golden reference to `f32` gradient
//! precision. Positions are centre-rebased before the `f32` conversion,
//! which is the precision contract for large coordinates.

// Rust guideline compliant 2026-02-21

use crate::TransformError;
use gpui_mesh::{Association, DataSet, Field};
use std::sync::Arc;

/// WGSL per-tetrahedron Jacobian kernel (one invocation per tetrahedron).
pub const GRADIENT_WGSL: &str = r"
struct GradientUniforms {
    tet_count: u32,
};

@group(0) @binding(0) var<uniform> uniforms: GradientUniforms;
@group(0) @binding(1) var<storage, read> positions: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> velocities: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> tets: array<vec4<u32>>;
@group(0) @binding(4) var<storage, read_write> jacobians: array<f32>;
@group(0) @binding(5) var<storage, read_write> volumes: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let tet_index = id.x;
    if (tet_index >= uniforms.tet_count) {
        return;
    }
    let nodes = tets[tet_index];
    let p0 = positions[nodes.x].xyz;
    let p1 = positions[nodes.y].xyz;
    let p2 = positions[nodes.z].xyz;
    let p3 = positions[nodes.w].xyz;
    let m = mat3x3<f32>(p1 - p0, p2 - p0, p3 - p0);
    let det = determinant(m);
    let base = tet_index * 9u;
    if (abs(det) < 1e-20) {
        for (var i: u32 = 0u; i < 9u; i = i + 1u) {
            jacobians[base + i] = 0.0;
        }
        volumes[tet_index] = 0.0;
        return;
    }
    volumes[tet_index] = abs(det) / 6.0;
    // (M^-1)^T via the adjugate: columns are pairwise cross products of
    // the edge columns. WGSL has no matrix inverse builtin.
    let inv_t = mat3x3<f32>(
        cross(m[1], m[2]) / det,
        cross(m[2], m[0]) / det,
        cross(m[0], m[1]) / det,
    );
    let v0 = velocities[nodes.x].xyz;
    for (var component: u32 = 0u; component < 3u; component = component + 1u) {
        let dv = vec3<f32>(
            velocities[nodes.y][component] - v0[component],
            velocities[nodes.z][component] - v0[component],
            velocities[nodes.w][component] - v0[component],
        );
        let g = inv_t * dv;
        jacobians[base + component * 3u + 0u] = g.x;
        jacobians[base + component * 3u + 1u] = g.y;
        jacobians[base + component * 3u + 2u] = g.z;
    }
}
";

/// GPU failures; callers fall back to the CPU filters.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum GpuError {
    /// No suitable GPU adapter is available.
    #[error("no GPU adapter available for compute")]
    NoAdapter,
    /// Device creation failed.
    #[error("GPU device creation failed: {0}")]
    Device(String),
    /// A GPU buffer could not be read back.
    #[error("GPU readback failed: {0}")]
    Readback(String),
}

use gpui_mesh::tetrahedra as tetrahedralize;

/// A retained wgpu compute context for flow diagnostics.
#[derive(Debug)]
pub struct GpuFlow {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
    /// Backend name reported by the adapter (for diagnostics).
    pub backend: String,
}

impl GpuFlow {
    /// Creates a compute context on the best available adapter.
    ///
    /// # Errors
    ///
    /// Returns [`GpuError::NoAdapter`] when no adapter exists; callers are
    /// expected to fall back to the CPU filters.
    pub fn new() -> Result<Self, GpuError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .map_err(|_| GpuError::NoAdapter)?;
        let backend = format!("{:?}", adapter.get_info().backend);
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("gpui-dataset-transformation compute"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::default(),
        }))
        .map_err(|e| GpuError::Device(e.to_string()))?;
        let device = Arc::new(device);
        let queue = Arc::new(queue);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gradient compute"),
            source: wgpu::ShaderSource::Wgsl(GRADIENT_WGSL.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gradient layout"),
            entries: &[
                uniform_binding(0),
                storage_binding(1),
                storage_binding(2),
                storage_binding(3),
                storage_binding(4),
                storage_binding(5),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gradient pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("gradient pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        Ok(Self {
            device,
            queue,
            pipeline,
            layout,
            backend,
        })
    }

    /// Computes per-point velocity Jacobians on the GPU.
    ///
    /// # Errors
    ///
    /// Returns an error when the field is missing or not a vector field,
    /// or when GPU execution fails.
    pub fn point_jacobians(
        &self,
        dataset: &DataSet,
        field_name: &str,
    ) -> Result<Vec<[[f64; 3]; 3]>, TransformError> {
        let field = dataset
            .field(Association::Point, field_name)
            .ok_or_else(|| TransformError::FieldNotFound(field_name.to_string()))?;
        let vectors = field
            .as_vector()
            .ok_or_else(|| TransformError::WrongFieldShape {
                field: field_name.to_string(),
                reason: "GPU flow diagnostics require a 3-vector field".into(),
            })?;
        let mut tets: Vec<[u32; 4]> = Vec::new();
        for cell in 0..dataset.cell_count() {
            let kind = dataset.cells.types[cell];
            if kind.is_volume() {
                let nodes = dataset.cells.cell(cell).unwrap_or(&[]);
                tets.extend(tetrahedralize(kind, nodes));
            }
        }
        if tets.is_empty() {
            return Err(TransformError::Degenerate(
                "dataset holds no volume cells".into(),
            ));
        }
        let centre = dataset.bounds().map_or([0.0; 3], |b| b.center());
        let positions: Vec<[f32; 4]> = dataset
            .positions
            .iter()
            .map(|p| {
                [
                    (p[0] - centre[0]) as f32,
                    (p[1] - centre[1]) as f32,
                    (p[2] - centre[2]) as f32,
                    0.0,
                ]
            })
            .collect();
        let velocities: Vec<[f32; 4]> = vectors
            .iter()
            .map(|v| [v[0] as f32, v[1] as f32, v[2] as f32, 0.0])
            .collect();
        let (jacobians, volumes) = self
            .run_kernel(&positions, &velocities, &tets)
            .map_err(|e| TransformError::Degenerate(e.to_string()))?;

        // Volume-weighted point averaging in f64 on the host.
        let mut acc = vec![([[0.0f64; 3]; 3], 0.0f64); dataset.point_count()];
        for (index, tet) in tets.iter().enumerate() {
            let volume = f64::from(volumes[index]);
            if volume <= 0.0 {
                continue;
            }
            for &node in tet {
                let entry = &mut acc[node as usize];
                for (component, row) in entry.0.iter_mut().enumerate() {
                    for (axis, value) in row.iter_mut().enumerate() {
                        *value += f64::from(jacobians[index * 9 + component * 3 + axis]) * volume;
                    }
                }
                entry.1 += volume;
            }
        }
        Ok(acc
            .into_iter()
            .map(|(mut jac, volume)| {
                if volume > 0.0 {
                    for row in &mut jac {
                        for value in row.iter_mut() {
                            *value /= volume;
                        }
                    }
                }
                jac
            })
            .collect())
    }

    /// Appends vorticity, Q-criterion, and divergence computed on the GPU.
    ///
    /// # Errors
    ///
    /// Returns an error when the field is unsuitable or GPU execution fails.
    pub fn flow_diagnostics(
        &self,
        dataset: &DataSet,
        field_name: &str,
    ) -> Result<DataSet, TransformError> {
        let jacobian = self.point_jacobians(dataset, field_name)?;
        let vorticity: Vec<[f64; 3]> = jacobian
            .iter()
            .map(|j| [j[2][1] - j[1][2], j[0][2] - j[2][0], j[1][0] - j[0][1]])
            .collect();
        let divergence: Vec<f64> = jacobian
            .iter()
            .map(|j| j[0][0] + j[1][1] + j[2][2])
            .collect();
        let q: Vec<f64> = jacobian
            .iter()
            .map(|j| {
                let mut omega2 = 0.0;
                let mut strain2 = 0.0;
                for (a, row) in j.iter().enumerate() {
                    for (b, &jab) in row.iter().enumerate() {
                        let omega = 0.5 * (jab - j[b][a]);
                        let strain = f64::midpoint(jab, j[b][a]);
                        omega2 += omega * omega;
                        strain2 += strain * strain;
                    }
                }
                0.5 * (omega2 - strain2)
            })
            .collect();
        let mut result = dataset.clone();
        result
            .point_data
            .push(Field::vector("vorticity", vorticity));
        result.point_data.push(Field::scalar("q_criterion", q));
        result
            .point_data
            .push(Field::scalar("divergence", divergence));
        Ok(result)
    }

    fn run_kernel(
        &self,
        positions: &[[f32; 4]],
        velocities: &[[f32; 4]],
        tets: &[[u32; 4]],
    ) -> Result<(Vec<f32>, Vec<f32>), GpuError> {
        let device = &self.device;
        let make_buffer = |label: &str, bytes: &[u8], usage: wgpu::BufferUsages| {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: bytes.len().max(4) as u64,
                usage,
                mapped_at_creation: false,
            });
            self.queue.write_buffer(&buffer, 0, bytes);
            buffer
        };
        let storage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        let positions_buffer = make_buffer("positions", bytemuck::cast_slice(positions), storage);
        let velocities_buffer =
            make_buffer("velocities", bytemuck::cast_slice(velocities), storage);
        let tets_buffer = make_buffer("tets", bytemuck::cast_slice(tets), storage);
        let jacobian_bytes = (tets.len() * 9 * 4) as u64;
        let jacobians_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("jacobians"),
            size: jacobian_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let volume_bytes = (tets.len() * 4) as u64;
        let volumes_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("volumes"),
            size: volume_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let uniforms_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&uniforms_buffer, 0, &(tets.len() as u32).to_le_bytes());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gradient bind group"),
            layout: &self.layout,
            entries: &[
                bind(0, &uniforms_buffer),
                bind(1, &positions_buffer),
                bind(2, &velocities_buffer),
                bind(3, &tets_buffer),
                bind(4, &jacobians_buffer),
                bind(5, &volumes_buffer),
            ],
        });
        let readback_jacobians = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("jacobian readback"),
            size: jacobian_bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let readback_volumes = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("volume readback"),
            size: volume_bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("gradient encoder"),
        });
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("gradient pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups((tets.len() as u32).div_ceil(64), 1, 1);
        drop(pass);
        encoder.copy_buffer_to_buffer(&jacobians_buffer, 0, &readback_jacobians, 0, jacobian_bytes);
        encoder.copy_buffer_to_buffer(&volumes_buffer, 0, &readback_volumes, 0, volume_bytes);
        self.queue.submit([encoder.finish()]);
        let jacobians = read_f32_buffer(device, &readback_jacobians, tets.len() * 9)?;
        let volumes = read_f32_buffer(device, &readback_volumes, tets.len())?;
        Ok((jacobians, volumes))
    }
}

fn uniform_binding(index: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: index,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn storage_binding(index: u32) -> wgpu::BindGroupLayoutEntry {
    storage_binding_rw(index, index != 4 && index != 5)
}

fn storage_binding_rw(index: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: index,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn bind(index: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding: index,
        resource: buffer.as_entire_binding(),
    }
}

fn read_f32_buffer(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
    count: usize,
) -> Result<Vec<f32>, GpuError> {
    let slice = buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    let _ = device.poll(wgpu::PollType::Wait {
        submission_index: Default::default(),
        timeout: Some(std::time::Duration::from_secs(10)),
    });
    match receiver.recv_timeout(std::time::Duration::from_secs(10)) {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return Err(GpuError::Readback(e.to_string())),
        Err(e) => return Err(GpuError::Readback(e.to_string())),
    }
    let data = slice.get_mapped_range();
    let values: Vec<f32> = bytemuck::cast_slice(&data[..count * 4]).to_vec();
    drop(data);
    buffer.unmap();
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_mesh::Field;

    #[test]
    fn gpu_matches_cpu_on_solid_rotation_when_available() {
        let Some(gpu) = test_gpu() else {
            // No adapter on this host; CPU remains the reference path.
            return;
        };
        let mut ds = DataSet::structured_grid("g", [4, 4, 4], [-1.0; 3], [0.5; 3]).unwrap();
        let velocity: Vec<[f64; 3]> = ds
            .positions
            .iter()
            .map(|p| [-2.0 * p[1], 2.0 * p[0], 0.0])
            .collect();
        ds.point_data.push(Field::vector("velocity", velocity));
        let gpu_out = gpu.flow_diagnostics(&ds, "velocity").unwrap();
        let cpu_out = crate::vorticity(&ds, "velocity", "vorticity_cpu").unwrap();
        let gpu_v = gpu_out
            .field(Association::Point, "vorticity")
            .unwrap()
            .as_vector()
            .unwrap();
        let cpu_v = cpu_out
            .field(Association::Point, "vorticity_cpu")
            .unwrap()
            .as_vector()
            .unwrap();
        // f32 GPU gradients: agreement to 1e-3 on an O(1) field.
        for (g, c) in gpu_v.iter().zip(cpu_v.iter()) {
            assert!((g[2] - c[2]).abs() < 1e-3, "gpu {g:?} vs cpu {c:?}");
        }
        let q = gpu_out
            .field(Association::Point, "q_criterion")
            .unwrap()
            .as_scalar()
            .unwrap();
        assert!(q.iter().all(|v| *v > 3.0), "backend {}", gpu.backend);
    }
}

/// Point-reduction kernel: one invocation per point gathers its adjacent
/// tetrahedra, averages their Jacobians, and writes only per-point
/// vorticity, Q, and divergence. Readback is therefore proportional to
/// the point count, not the tetrahedron count, which is what makes the
/// GPU path win on large meshes and per-frame time-series updates.
pub const POINT_DIAGNOSTICS_WGSL: &str = r"
struct PointUniforms {
    point_count: u32,
};

@group(0) @binding(0) var<uniform> uniforms: PointUniforms;
@group(0) @binding(1) var<storage, read> positions: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> velocities: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> tets: array<vec4<u32>>;
@group(0) @binding(4) var<storage, read> point_tet_offsets: array<u32>;
@group(0) @binding(5) var<storage, read> point_tets: array<u32>;
@group(0) @binding(6) var<storage, read_write> vorticity: array<vec4<f32>>;
@group(0) @binding(7) var<storage, read_write> q_values: array<f32>;
@group(0) @binding(8) var<storage, read_write> divergence: array<f32>;

fn tet_jacobian(nodes: vec4<u32>, inv_volume_sum: ptr<function, f32>) -> mat3x3<f32> {
    let p0 = positions[nodes.x].xyz;
    let p1 = positions[nodes.y].xyz;
    let p2 = positions[nodes.z].xyz;
    let p3 = positions[nodes.w].xyz;
    let m = mat3x3<f32>(p1 - p0, p2 - p0, p3 - p0);
    let det = determinant(m);
    let volume = abs(det) / 6.0;
    *inv_volume_sum = volume;
    if (abs(det) < 1e-20) {
        return mat3x3<f32>(vec3<f32>(0.0), vec3<f32>(0.0), vec3<f32>(0.0));
    }
    let inv_t = mat3x3<f32>(
        cross(m[1], m[2]) / det,
        cross(m[2], m[0]) / det,
        cross(m[0], m[1]) / det,
    );
    let v0 = velocities[nodes.x].xyz;
    var jac = mat3x3<f32>(vec3<f32>(0.0), vec3<f32>(0.0), vec3<f32>(0.0));
    for (var component: u32 = 0u; component < 3u; component = component + 1u) {
        let dv = vec3<f32>(
            velocities[nodes.y][component] - v0[component],
            velocities[nodes.z][component] - v0[component],
            velocities[nodes.w][component] - v0[component],
        );
        let g = inv_t * dv;
        jac[component] = g;
    }
    return jac;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let point = id.x;
    if (point >= uniforms.point_count) {
        return;
    }
    var accumulated = mat3x3<f32>(vec3<f32>(0.0), vec3<f32>(0.0), vec3<f32>(0.0));
    var total_volume = 0.0;
    let start = point_tet_offsets[point];
    let end = point_tet_offsets[point + 1u];
    for (var i: u32 = start; i < end; i = i + 1u) {
        var volume = 0.0;
        let jac = tet_jacobian(tets[point_tets[i]], &volume);
        if (volume > 0.0) {
            for (var c: u32 = 0u; c < 3u; c = c + 1u) {
                accumulated[c] = accumulated[c] + jac[c] * volume;
            }
            total_volume = total_volume + volume;
        }
    }
    if (total_volume > 0.0) {
        for (var c: u32 = 0u; c < 3u; c = c + 1u) {
            accumulated[c] = accumulated[c] / total_volume;
        }
    }
    let j = accumulated;
    // Indexing is j[component][axis], exactly as on the CPU path.
    vorticity[point] = vec4<f32>(
        j[2][1] - j[1][2],
        j[0][2] - j[2][0],
        j[1][0] - j[0][1],
        0.0,
    );
    divergence[point] = j[0][0] + j[1][1] + j[2][2];
    var omega2 = 0.0;
    var strain2 = 0.0;
    for (var a: u32 = 0u; a < 3u; a = a + 1u) {
        for (var b: u32 = 0u; b < 3u; b = b + 1u) {
            let jab = j[a][b];
            let jba = j[b][a];
            let omega = 0.5 * (jab - jba);
            let strain = 0.5 * (jab + jba);
            omega2 = omega2 + omega * omega;
            strain2 = strain2 + strain * strain;
        }
    }
    q_values[point] = 0.5 * (omega2 - strain2);
}
";

/// A prepared GPU flow problem: geometry and adjacency uploaded once.
///
/// Per-frame updates replace only the velocity buffer and read back only
/// per-point results, which is the fast path for time-series movies on
/// large meshes.
#[derive(Debug)]
pub struct PreparedFlow {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    pipeline: wgpu::ComputePipeline,
    base: DataSet,
    velocity_field: usize,
    velocities_buffer: wgpu::Buffer,
    vorticity_buffer: wgpu::Buffer,
    q_buffer: wgpu::Buffer,
    divergence_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    point_count: usize,
}

impl PreparedFlow {
    /// Uploads a dataset's geometry, topology, and adjacency once.
    ///
    /// # Errors
    ///
    /// Returns an error when the field is missing, not a vector field, or
    /// the dataset is invalid, holds no volume cells, or the input name
    /// conflicts with a diagnostic output name.
    pub fn new(gpu: &GpuFlow, dataset: &DataSet, field_name: &str) -> Result<Self, TransformError> {
        dataset.validate()?;
        if matches!(field_name, "vorticity" | "q_criterion" | "divergence") {
            return Err(TransformError::WrongFieldShape {
                field: field_name.into(),
                reason: "input name conflicts with diagnostic output".into(),
            });
        }
        let field = dataset
            .field(Association::Point, field_name)
            .ok_or_else(|| TransformError::FieldNotFound(field_name.to_string()))?;
        let vectors = field
            .as_vector()
            .ok_or_else(|| TransformError::WrongFieldShape {
                field: field_name.to_string(),
                reason: "GPU flow diagnostics require a 3-vector field".into(),
            })?;
        let velocity_field = dataset
            .point_data
            .iter()
            .position(|field| field.name.as_ref() == field_name)
            .ok_or_else(|| TransformError::FieldNotFound(field_name.into()))?;
        let mut tets: Vec<[u32; 4]> = Vec::new();
        for cell in 0..dataset.cell_count() {
            let kind = dataset.cells.types[cell];
            if kind.is_volume() {
                let nodes = dataset.cells.cell(cell).unwrap_or(&[]);
                tets.extend(tetrahedralize(kind, nodes));
            }
        }
        if tets.is_empty() {
            return Err(TransformError::Degenerate(
                "dataset holds no volume cells".into(),
            ));
        }
        // Point -> adjacent tetrahedra CSR.
        let mut adjacency: Vec<Vec<u32>> = vec![Vec::new(); dataset.point_count()];
        for (index, tet) in tets.iter().enumerate() {
            for &node in tet {
                adjacency[node as usize].push(index as u32);
            }
        }
        let mut offsets: Vec<u32> = Vec::with_capacity(dataset.point_count() + 1);
        let mut flat_adjacency: Vec<u32> = Vec::new();
        offsets.push(0);
        for list in &adjacency {
            flat_adjacency.extend_from_slice(list);
            offsets.push(flat_adjacency.len() as u32);
        }
        let centre = dataset.bounds().map_or([0.0; 3], |b| b.center());
        let positions: Vec<[f32; 4]> = dataset
            .positions
            .iter()
            .map(|p| {
                [
                    (p[0] - centre[0]) as f32,
                    (p[1] - centre[1]) as f32,
                    (p[2] - centre[2]) as f32,
                    0.0,
                ]
            })
            .collect();
        let velocities: Vec<[f32; 4]> = vectors
            .iter()
            .map(|v| [v[0] as f32, v[1] as f32, v[2] as f32, 0.0])
            .collect();

        let device = gpu.device.clone();
        let queue = gpu.queue.clone();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("point diagnostics compute"),
            source: wgpu::ShaderSource::Wgsl(POINT_DIAGNOSTICS_WGSL.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("point diagnostics layout"),
            entries: &[
                uniform_binding(0),
                storage_binding(1),
                storage_binding(2),
                storage_binding(3),
                storage_binding_rw(4, true),
                storage_binding_rw(5, true),
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 8,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("point diagnostics pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("point diagnostics pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let storage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        let make = |label: &str, bytes: &[u8], usage: wgpu::BufferUsages| {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: bytes.len().max(4) as u64,
                usage,
                mapped_at_creation: false,
            });
            queue.write_buffer(&buffer, 0, bytes);
            buffer
        };
        let positions_buffer = make("flow positions", bytemuck::cast_slice(&positions), storage);
        let velocities_buffer = make(
            "flow velocities",
            bytemuck::cast_slice(&velocities),
            storage,
        );
        let tets_buffer = make("flow tets", bytemuck::cast_slice(&tets), storage);
        let offsets_buffer = make(
            "flow point tet offsets",
            bytemuck::cast_slice(&offsets),
            storage,
        );
        let adjacency_buffer = make(
            "flow point tets",
            bytemuck::cast_slice(&flat_adjacency),
            storage,
        );
        let point_count = dataset.point_count();
        let output_usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC;
        let vorticity_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("flow vorticity"),
            size: (point_count * 16) as u64,
            usage: output_usage,
            mapped_at_creation: false,
        });
        let q_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("flow q"),
            size: (point_count * 4) as u64,
            usage: output_usage,
            mapped_at_creation: false,
        });
        let divergence_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("flow divergence"),
            size: (point_count * 4) as u64,
            usage: output_usage,
            mapped_at_creation: false,
        });
        let uniforms_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("flow uniforms"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&uniforms_buffer, 0, &(point_count as u32).to_le_bytes());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("point diagnostics bind group"),
            layout: &layout,
            entries: &[
                bind(0, &uniforms_buffer),
                bind(1, &positions_buffer),
                bind(2, &velocities_buffer),
                bind(3, &tets_buffer),
                bind(4, &offsets_buffer),
                bind(5, &adjacency_buffer),
                bind(6, &vorticity_buffer),
                bind(7, &q_buffer),
                bind(8, &divergence_buffer),
            ],
        });
        Ok(Self {
            device,
            queue,
            pipeline,
            base: dataset.clone(),
            velocity_field,
            velocities_buffer,
            vorticity_buffer,
            q_buffer,
            divergence_buffer,
            bind_group,
            point_count,
        })
    }

    /// Recomputes diagnostics, optionally with a new velocity field.
    ///
    /// When `velocities` is provided it replaces the prepared field
    /// (same point count); the geometry upload is never repeated.
    ///
    /// # Errors
    ///
    /// Returns an error when GPU execution or readback fails.
    pub fn diagnostics(
        &mut self,
        velocities: Option<&[[f64; 3]]>,
    ) -> Result<DataSet, TransformError> {
        if let Some(values) = velocities {
            if values.len() != self.point_count {
                return Err(TransformError::Degenerate(format!(
                    "velocity update has {} points, expected {}",
                    values.len(),
                    self.point_count
                )));
            }
            let flat: Vec<[f32; 4]> = values
                .iter()
                .map(|v| [v[0] as f32, v[1] as f32, v[2] as f32, 0.0])
                .collect();
            self.queue
                .write_buffer(&self.velocities_buffer, 0, bytemuck::cast_slice(&flat));
            // Commit the same input revision used by subsequent GPU dispatches,
            // including a later diagnostics(None). Clones share this Arc.
            self.base.point_data[self.velocity_field].data =
                gpui_mesh::FieldData::Vector(values.into());
        }
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("point diagnostics encoder"),
            });
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("point diagnostics pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.dispatch_workgroups((self.point_count as u32).div_ceil(64), 1, 1);
        drop(pass);
        let vorticity_readback =
            self.readback_buffer((self.point_count * 16) as u64, "vorticity readback");
        let q_readback = self.readback_buffer((self.point_count * 4) as u64, "q readback");
        let divergence_readback =
            self.readback_buffer((self.point_count * 4) as u64, "divergence readback");
        encoder.copy_buffer_to_buffer(
            &self.vorticity_buffer,
            0,
            &vorticity_readback,
            0,
            (self.point_count * 16) as u64,
        );
        encoder.copy_buffer_to_buffer(
            &self.q_buffer,
            0,
            &q_readback,
            0,
            (self.point_count * 4) as u64,
        );
        encoder.copy_buffer_to_buffer(
            &self.divergence_buffer,
            0,
            &divergence_readback,
            0,
            (self.point_count * 4) as u64,
        );
        self.queue.submit([encoder.finish()]);
        let vorticity_flat =
            read_f32_buffer(&self.device, &vorticity_readback, self.point_count * 4)
                .map_err(|e| TransformError::Degenerate(e.to_string()))?;
        let q = read_f32_buffer(&self.device, &q_readback, self.point_count)
            .map_err(|e| TransformError::Degenerate(e.to_string()))?;
        let divergence = read_f32_buffer(&self.device, &divergence_readback, self.point_count)
            .map_err(|e| TransformError::Degenerate(e.to_string()))?;
        let vorticity: Vec<[f64; 3]> = vorticity_flat
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| [f64::from(c[0]), f64::from(c[1]), f64::from(c[2])])
            .collect();
        let mut result = self.base.clone();
        result
            .point_data
            .retain(|f| !matches!(f.name.as_ref(), "vorticity" | "q_criterion" | "divergence"));
        result
            .point_data
            .push(Field::vector("vorticity", vorticity));
        result.point_data.push(Field::scalar(
            "q_criterion",
            q.iter().map(|v| f64::from(*v)).collect::<Vec<_>>(),
        ));
        result.point_data.push(Field::scalar(
            "divergence",
            divergence.iter().map(|v| f64::from(*v)).collect::<Vec<_>>(),
        ));
        Ok(result)
    }

    fn readback_buffer(&self, size: u64, label: &str) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        })
    }
}

#[cfg(test)]
mod prepared_tests {
    use super::*;
    use gpui_mesh::Field;

    #[test]
    fn prepared_flow_matches_cpu_and_updates() {
        let Some(gpu) = test_gpu() else {
            return;
        };
        let mut ds = DataSet::structured_grid("g", [4, 4, 4], [-1.0; 3], [0.5; 3]).unwrap();
        let velocity: Vec<[f64; 3]> = ds
            .positions
            .iter()
            .map(|p| [-2.0 * p[1], 2.0 * p[0], 0.0])
            .collect();
        ds.point_data.push(Field::vector("velocity", velocity));
        let mut prepared = PreparedFlow::new(&gpu, &ds, "velocity").unwrap();
        let out = prepared.diagnostics(None).unwrap();
        let vorticity = out
            .field(Association::Point, "vorticity")
            .unwrap()
            .as_vector()
            .unwrap();
        assert!(
            vorticity.iter().all(|v| (v[2] - 4.0).abs() < 1e-2),
            "backend {} first {:?}",
            gpu.backend,
            vorticity[0]
        );
        // A zero velocity update must zero the diagnostics without re-upload.
        let zeros = vec![[0.0; 3]; ds.point_count()];
        let updated = prepared.diagnostics(Some(&zeros)).unwrap();
        let q = updated
            .field(Association::Point, "q_criterion")
            .unwrap()
            .as_scalar()
            .unwrap();
        assert!(q.iter().all(|v| v.abs() < 1e-6));
        assert_eq!(
            updated
                .field(Association::Point, "velocity")
                .unwrap()
                .as_vector()
                .unwrap(),
            zeros
        );
        let repeated = prepared.diagnostics(None).unwrap();
        assert_eq!(
            repeated
                .field(Association::Point, "velocity")
                .unwrap()
                .as_vector()
                .unwrap(),
            zeros
        );
        let cpu = crate::q_criterion(&repeated, "velocity", "q_cpu").unwrap();
        assert!(
            cpu.field(Association::Point, "q_cpu")
                .unwrap()
                .as_scalar()
                .unwrap()
                .iter()
                .all(|q| q.abs() < 1e-6)
        );
        assert!(prepared.diagnostics(Some(&zeros[..1])).is_err());
        let restored = prepared
            .diagnostics(Some(
                ds.field(Association::Point, "velocity")
                    .unwrap()
                    .as_vector()
                    .unwrap(),
            ))
            .unwrap();
        assert!(
            restored
                .field(Association::Point, "vorticity")
                .unwrap()
                .as_vector()
                .unwrap()
                .iter()
                .all(|v| (v[2] - 4.0).abs() < 1e-2)
        );
    }
}

#[cfg(test)]
fn test_gpu() -> Option<GpuFlow> {
    match GpuFlow::new() {
        Ok(gpu) => {
            eprintln!("flow test GPU backend: {}", gpu.backend);
            Some(gpu)
        }
        Err(GpuError::NoAdapter) if std::env::var_os("GPUI_REQUIRE_GPU").is_none() => {
            eprintln!("SKIP: flow GPU tests: no adapter");
            None
        }
        Err(error) => panic!("flow GPU initialization failed: {error}"),
    }
}
