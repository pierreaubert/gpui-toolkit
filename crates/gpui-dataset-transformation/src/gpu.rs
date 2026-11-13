//! GPU compute for flow diagnostics through wgpu.
//!
//! The per-tetrahedron Jacobian kernel runs on the host GPU through
//! wgpu: Metal on Apple Silicon, Vulkan/DX12 elsewhere, WebGPU in the
//! browser. No CUDA or MLX backend is included. Point averaging
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
    grid_stride: u32,
};

@group(0) @binding(0) var<uniform> uniforms: GradientUniforms;
@group(0) @binding(1) var<storage, read> positions: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> velocities: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> tets: array<vec4<u32>>;
@group(0) @binding(4) var<storage, read_write> jacobians: array<f32>;
@group(0) @binding(5) var<storage, read_write> volumes: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let tet_index = id.x + id.y * uniforms.grid_stride;
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
    if (!(abs(det) >= 1e-20 && abs(det) <= 3.402823e38) ||
        velocities[nodes.x].w == 0.0 || velocities[nodes.y].w == 0.0 ||
        velocities[nodes.z].w == 0.0 || velocities[nodes.w].w == 0.0) {
        for (var i: u32 = 0u; i < 9u; i = i + 1u) {
            jacobians[base + i] = 0.0;
        }
        volumes[tet_index] = 0.0;
        return;
    }
    // (M^-1)^T via the adjugate: columns are pairwise cross products of
    // the edge columns. WGSL has no matrix inverse builtin.
    let inv_t = mat3x3<f32>(
        cross(m[1], m[2]) / det,
        cross(m[2], m[0]) / det,
        cross(m[0], m[1]) / det,
    );
    let v0 = velocities[nodes.x].xyz;
    var supported = true;
    for (var component: u32 = 0u; component < 3u; component = component + 1u) {
        let dv = vec3<f32>(
            velocities[nodes.y][component] - v0[component],
            velocities[nodes.z][component] - v0[component],
            velocities[nodes.w][component] - v0[component],
        );
        let g = inv_t * dv;
        supported = supported && all(abs(g) <= vec3<f32>(3.402823e38));
        jacobians[base + component * 3u + 0u] = g.x;
        jacobians[base + component * 3u + 1u] = g.y;
        jacobians[base + component * 3u + 2u] = g.z;
    }
    volumes[tet_index] = select(0.0, abs(det) / 6.0, supported);
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
    /// The problem exceeds a device limit (buffer size or dispatch grid).
    #[error("GPU problem too large: {0}")]
    TooLarge(String),
}

use gpui_mesh::tetrahedra as tetrahedralize;

/// Workgroup grid for `count` invocations of a 64-wide kernel,
/// partitioned across the x/y dimensions within device limits.
/// Returns `(workgroups_x, workgroups_y, grid_stride_invocations)`.
fn dispatch_plan(count: usize, device: &wgpu::Device) -> Result<(u32, u32, u32), GpuError> {
    dispatch_plan_for(count, device.limits().max_compute_workgroups_per_dimension)
}

/// Pure grid partitioning (unit-tested without a device).
fn dispatch_plan_for(count: usize, max_per_dimension: u32) -> Result<(u32, u32, u32), GpuError> {
    let max_x = max_per_dimension.max(1);
    let needed = u32::try_from(count.div_ceil(64)).map_err(|_| {
        GpuError::TooLarge(format!("{count} invocations overflow the dispatch grid"))
    })?;
    let workgroups_x = needed.min(max_x);
    let workgroups_y = needed.div_ceil(workgroups_x.max(1));
    if workgroups_y > max_x {
        return Err(GpuError::TooLarge(format!(
            "{count} invocations need a {needed}-workgroup grid; device allows {max_x} per dimension"
        )));
    }
    let stride = workgroups_x
        .checked_mul(64)
        .ok_or_else(|| GpuError::TooLarge("dispatch stride overflow".into()))?;
    Ok((workgroups_x.max(1), workgroups_y.max(1), stride))
}

/// Checked storage-buffer byte size against device limits, before any
/// allocation is attempted.
fn checked_storage_bytes(
    elements: usize,
    element_bytes: usize,
    device: &wgpu::Device,
    label: &str,
) -> Result<u64, GpuError> {
    let bytes = elements
        .checked_mul(element_bytes)
        .and_then(|bytes| u64::try_from(bytes).ok())
        .ok_or_else(|| GpuError::TooLarge(format!("{label}: size overflow")))?;
    let limits = device.limits();
    let limit = limits
        .max_buffer_size
        .min(u64::from(limits.max_storage_buffer_binding_size));
    if bytes > limit {
        return Err(GpuError::TooLarge(format!(
            "{label}: {bytes} bytes exceeds the device storage limit {limit}"
        )));
    }
    Ok(bytes.max(4))
}

fn packed_velocities(field: &Field, vectors: &[[f64; 3]]) -> Result<Vec<[f32; 4]>, TransformError> {
    vectors
        .iter()
        .enumerate()
        .map(|(index, vector)| {
            let valid = field.valid.as_ref().is_none_or(|mask| mask[index]);
            if vector
                .iter()
                .any(|v| v.is_infinite() || (valid && v.is_nan()))
            {
                return Err(TransformError::Degenerate(format!(
                    "velocity {index} is nonfinite without valid masking"
                )));
            }
            if !valid {
                return Ok([0.0; 4]);
            }
            let packed = [vector[0] as f32, vector[1] as f32, vector[2] as f32, 1.0];
            if packed.iter().any(|v| !v.is_finite()) {
                return Err(TransformError::Degenerate(format!(
                    "velocity {index} cannot be represented by GPU f32; use CPU diagnostics"
                )));
            }
            Ok(packed)
        })
        .collect()
}

fn append_diagnostics(
    dataset: &DataSet,
    fields: Vec<(Field, &str, &str)>,
    supported: impl Fn(usize) -> bool,
) -> DataSet {
    let mut result = dataset.clone();
    for (mut field, label, unit) in fields {
        field.label = label.into();
        field.unit = Some(unit.into());
        let field = crate::validity::finish(field, &supported);
        result
            .point_data
            .retain(|existing| existing.name != field.name);
        result.point_data.push(field);
    }
    result
}

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
    /// Creates a compute context with the system-default power policy.
    ///
    /// Hosts with their own device should prefer
    /// [`from_shared_device`](Self::from_shared_device) so compute and
    /// rendering share one device (and resident buffers can move
    /// GPU-to-GPU); hosts with a power policy use
    /// [`with_power_preference`](Self::with_power_preference).
    ///
    /// # Errors
    ///
    /// Returns [`GpuError::NoAdapter`] when no adapter exists; callers are
    /// expected to fall back to the CPU filters.
    pub fn new() -> Result<Self, GpuError> {
        Self::with_power_preference(wgpu::PowerPreference::None)
    }

    /// Creates a compute context on an adapter matching `preference`.
    ///
    /// # Errors
    ///
    /// Returns [`GpuError::NoAdapter`] when no adapter exists.
    pub fn with_power_preference(preference: wgpu::PowerPreference) -> Result<Self, GpuError> {
        pollster::block_on(Self::new_async(preference))
    }

    /// Asynchronously creates a compute context (no blocking poll, so
    /// wasm hosts with an async executor can use it too).
    ///
    /// # Errors
    ///
    /// Returns [`GpuError::NoAdapter`] when no adapter exists.
    pub async fn new_async(preference: wgpu::PowerPreference) -> Result<Self, GpuError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: preference,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|_| GpuError::NoAdapter)?;
        let backend = format!("{:?}", adapter.get_info().backend);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("gpui-dataset-transformation compute"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::default(),
            })
            .await
            .map_err(|e| GpuError::Device(e.to_string()))?;
        Ok(Self::with_device(
            Arc::new(device),
            Arc::new(queue),
            backend,
        ))
    }

    /// Builds the compute pipelines on a host-owned device/queue.
    ///
    /// The host keeps adapter/power policy ownership; buffers created
    /// by [`PreparedFlow`] on this device can be consumed by other
    /// passes on the same device without a CPU round trip.
    #[must_use]
    pub fn from_shared_device(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        Self::with_device(device, queue, "shared".into())
    }

    fn with_device(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>, backend: String) -> Self {
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
        Self {
            device,
            queue,
            pipeline,
            layout,
            backend,
        }
    }

    /// The device all compute (and resident buffers) live on.
    #[must_use]
    pub fn device(&self) -> &Arc<wgpu::Device> {
        &self.device
    }

    /// The queue submissions are made on.
    #[must_use]
    pub fn queue(&self) -> &Arc<wgpu::Queue> {
        &self.queue
    }

    /// Computes per-point velocity Jacobians on the GPU.
    /// Unsupported points return NaN components. Each contributing tetrahedron
    /// requires four valid velocity samples. Unrepresentable f32 inputs error;
    /// callers can use CPU diagnostics for those datasets.
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
        dataset.validate()?;
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
        if positions.iter().flatten().any(|v| !v.is_finite()) {
            return Err(TransformError::Degenerate(
                "local positions cannot be represented by GPU f32; use CPU diagnostics".into(),
            ));
        }
        let velocities = packed_velocities(field, vectors)?;
        let (jacobians, volumes) = self
            .run_kernel(&positions, &velocities, &tets)
            .map_err(|e| TransformError::Degenerate(e.to_string()))?;

        // Volume-weighted point averaging in f64 on the host.
        let mut acc = vec![([[0.0f64; 3]; 3], 0.0f64); dataset.point_count()];
        for (index, tet) in tets.iter().enumerate() {
            let volume = f64::from(volumes[index]);
            if !volume.is_finite() || volume <= 0.0 {
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
                if volume > 0.0 && volume.is_finite() {
                    for row in &mut jac {
                        for value in row.iter_mut() {
                            *value /= volume;
                        }
                    }
                } else {
                    jac = [[f64::NAN; 3]; 3];
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
        Ok(append_diagnostics(
            dataset,
            vec![
                (Field::vector("vorticity", vorticity), "Vorticity", "1/s"),
                (Field::scalar("q_criterion", q), "Q-criterion", "1/s^2"),
                (Field::scalar("divergence", divergence), "Divergence", "1/s"),
            ],
            |point| jacobian[point].iter().flatten().all(|v| v.is_finite()),
        ))
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
        for (label, elements) in [
            ("positions", positions.len() * 4),
            ("velocities", velocities.len() * 4),
            ("tets", tets.len() * 4),
        ] {
            checked_storage_bytes(elements, 4, device, label)?;
        }
        let (workgroups_x, workgroups_y, grid_stride) = dispatch_plan(tets.len(), device)?;
        let storage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        let positions_buffer = make_buffer("positions", bytemuck::cast_slice(positions), storage);
        let velocities_buffer =
            make_buffer("velocities", bytemuck::cast_slice(velocities), storage);
        let tets_buffer = make_buffer("tets", bytemuck::cast_slice(tets), storage);
        let jacobian_bytes = checked_storage_bytes(tets.len(), 9 * 4, device, "jacobians")?;
        let jacobians_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("jacobians"),
            size: jacobian_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let volume_bytes = checked_storage_bytes(tets.len(), 4, device, "volumes")?;
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
        let mut uniform_bytes = Vec::with_capacity(8);
        uniform_bytes.extend_from_slice(&(tets.len() as u32).to_le_bytes());
        uniform_bytes.extend_from_slice(&grid_stride.to_le_bytes());
        self.queue.write_buffer(&uniforms_buffer, 0, &uniform_bytes);

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
        pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
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

/// Asynchronous f32 readback: resolves through `map_async` without a
/// blocking `poll(Wait)`, so wasm hosts (which cannot block) can drive
/// it from their executor. Native hosts without an executor should use
/// the synchronous paths, which poll for completion.
pub async fn read_f32_buffer_async(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
    count: usize,
) -> Result<Vec<f32>, GpuError> {
    struct MapState {
        result: Option<Result<(), wgpu::BufferAsyncError>>,
        waker: Option<std::task::Waker>,
    }
    let state = std::sync::Arc::new(std::sync::Mutex::new(MapState {
        result: None,
        waker: None,
    }));
    let callback_state = state.clone();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let mut state = callback_state.lock().expect("map state lock");
            state.result = Some(result);
            if let Some(waker) = state.waker.take() {
                waker.wake();
            }
        });
    std::future::poll_fn(|cx| {
        // Native wgpu needs polling to deliver map callbacks; browsers
        // deliver them from the event loop.
        #[cfg(not(target_family = "wasm"))]
        let _ = device.poll(wgpu::PollType::Poll);
        let mut state = state.lock().expect("map state lock");
        if state.result.is_some() {
            std::task::Poll::Ready(())
        } else {
            state.waker = Some(cx.waker().clone());
            std::task::Poll::Pending
        }
    })
    .await;
    let result = state
        .lock()
        .expect("map state lock")
        .result
        .take()
        .expect("map completion recorded");
    result.map_err(|e| GpuError::Readback(e.to_string()))?;
    let slice = buffer.slice(..);
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
    fn dispatch_plan_partitions_beyond_one_dimension() {
        // 10M invocations at the common 65535-per-dimension limit.
        let (x, y, stride) = dispatch_plan_for(10_000_000, 65_535).unwrap();
        assert_eq!(x, 65_535);
        assert_eq!(stride, 65_535 * 64);
        assert!((x as u64 * y as u64 * 64) >= 10_000_000);
        assert!(y <= 65_535);
        // Small problems stay one-dimensional.
        assert_eq!(dispatch_plan_for(128, 65_535).unwrap(), (2, 1, 128));
        // Beyond a 2D grid: a typed error, never a truncated dispatch.
        assert!(matches!(
            dispatch_plan_for(usize::MAX / 2, 65_535),
            Err(GpuError::TooLarge(_))
        ));
    }

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
    grid_stride: u32,
};

@group(0) @binding(0) var<uniform> uniforms: PointUniforms;
@group(0) @binding(1) var<storage, read> velocities: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> tets: array<vec4<u32>>;
@group(0) @binding(3) var<storage, read> point_tet_offsets: array<u32>;
@group(0) @binding(4) var<storage, read> point_tets: array<u32>;
@group(0) @binding(5) var<storage, read_write> vorticity: array<vec4<f32>>;
@group(0) @binding(6) var<storage, read_write> q_values: array<f32>;
@group(0) @binding(7) var<storage, read_write> divergence: array<f32>;
// Per-tetrahedron geometry cached once at preparation (see
// TET_GEOMETRY_WGSL): three vec4s per tet holding the inverse-transpose
// columns, with the tet volume in the first w. Volume 0 = invalid tet.
@group(0) @binding(8) var<storage, read> tet_geo: array<vec4<f32>>;

fn tet_jacobian(tet_index: u32, nodes: vec4<u32>, volume_out: ptr<function, f32>) -> mat3x3<f32> {
    let base = tet_index * 3u;
    let volume = tet_geo[base].w;
    *volume_out = 0.0;
    if (volume == 0.0 ||
        velocities[nodes.x].w == 0.0 || velocities[nodes.y].w == 0.0 ||
        velocities[nodes.z].w == 0.0 || velocities[nodes.w].w == 0.0) {
        return mat3x3<f32>(vec3<f32>(0.0), vec3<f32>(0.0), vec3<f32>(0.0));
    }
    let inv_t = mat3x3<f32>(tet_geo[base].xyz, tet_geo[base + 1u].xyz, tet_geo[base + 2u].xyz);
    let v0 = velocities[nodes.x].xyz;
    var jac = mat3x3<f32>(vec3<f32>(0.0), vec3<f32>(0.0), vec3<f32>(0.0));
    for (var component: u32 = 0u; component < 3u; component = component + 1u) {
        let dv = vec3<f32>(
            velocities[nodes.y][component] - v0[component],
            velocities[nodes.z][component] - v0[component],
            velocities[nodes.w][component] - v0[component],
        );
        let g = inv_t * dv;
        if (!all(abs(g) <= vec3<f32>(3.402823e38))) {
            return mat3x3<f32>(vec3<f32>(0.0), vec3<f32>(0.0), vec3<f32>(0.0));
        }
        jac[component] = g;
    }
    *volume_out = volume;
    return jac;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let point = id.x + id.y * uniforms.grid_stride;
    if (point >= uniforms.point_count) {
        return;
    }
    var accumulated = mat3x3<f32>(vec3<f32>(0.0), vec3<f32>(0.0), vec3<f32>(0.0));
    var total_volume = 0.0;
    let start = point_tet_offsets[point];
    let end = point_tet_offsets[point + 1u];
    for (var i: u32 = start; i < end; i = i + 1u) {
        var volume = 0.0;
        let tet_index = point_tets[i];
        let jac = tet_jacobian(tet_index, tets[tet_index], &volume);
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
        select(0.0, 1.0, total_volume > 0.0 && total_volume <= 3.402823e38 &&
            all(abs(j[0]) <= vec3<f32>(3.402823e38)) &&
            all(abs(j[1]) <= vec3<f32>(3.402823e38)) &&
            all(abs(j[2]) <= vec3<f32>(3.402823e38))),
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

/// Per-tetrahedron geometry prepass: edge-matrix inverse-transpose and
/// volume, computed once per prepared problem instead of once per
/// (point, tet) adjacency entry in the point kernel.
pub const TET_GEOMETRY_WGSL: &str = r"
struct TetUniforms {
    tet_count: u32,
    grid_stride: u32,
};

@group(0) @binding(0) var<uniform> uniforms: TetUniforms;
@group(0) @binding(1) var<storage, read> positions: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> tets: array<vec4<u32>>;
@group(0) @binding(3) var<storage, read_write> tet_geo: array<vec4<f32>>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let tet_index = id.x + id.y * uniforms.grid_stride;
    if (tet_index >= uniforms.tet_count) {
        return;
    }
    let base = tet_index * 3u;
    let nodes = tets[tet_index];
    let p0 = positions[nodes.x].xyz;
    let p1 = positions[nodes.y].xyz;
    let p2 = positions[nodes.z].xyz;
    let p3 = positions[nodes.w].xyz;
    let m = mat3x3<f32>(p1 - p0, p2 - p0, p3 - p0);
    let det = determinant(m);
    if (!(abs(det) >= 1e-20 && abs(det) <= 3.402823e38)) {
        tet_geo[base] = vec4<f32>(0.0);
        tet_geo[base + 1u] = vec4<f32>(0.0);
        tet_geo[base + 2u] = vec4<f32>(0.0);
        return;
    }
    let inv_t = mat3x3<f32>(
        cross(m[1], m[2]) / det,
        cross(m[2], m[0]) / det,
        cross(m[0], m[1]) / det,
    );
    tet_geo[base] = vec4<f32>(inv_t[0], abs(det) / 6.0);
    tet_geo[base + 1u] = vec4<f32>(inv_t[1], 0.0);
    tet_geo[base + 2u] = vec4<f32>(inv_t[2], 0.0);
}
";

/// A prepared GPU flow problem: geometry and adjacency uploaded once.
///
/// Per-frame updates replace only the velocity buffer and read back only
/// per-point results, which is the fast path for time-series movies on
/// large meshes.
/// Which diagnostics a [`PreparedFlow`] run materializes on the CPU.
/// Unselected outputs are neither read back nor appended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlowOutputs {
    /// Curl of the averaged Jacobian (`vorticity` vector field).
    pub vorticity: bool,
    /// Q-criterion (`q_criterion` scalar field).
    pub q: bool,
    /// Divergence (`divergence` scalar field).
    pub divergence: bool,
}

impl FlowOutputs {
    /// All three diagnostics.
    #[must_use]
    pub fn all() -> Self {
        Self {
            vorticity: true,
            q: true,
            divergence: true,
        }
    }
}

impl Default for FlowOutputs {
    fn default() -> Self {
        Self::all()
    }
}

/// A prepared GPU flow problem: geometry and adjacency uploaded once.
///
/// Per-frame updates replace only the velocity buffer; outputs stay
/// resident in GPU buffers until read back, which is the fast path for
/// time-series movies on large meshes.
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
    vorticity_readback: wgpu::Buffer,
    q_readback: wgpu::Buffer,
    divergence_readback: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    point_count: usize,
    dispatch: (u32, u32),
    readback_bytes: u64,
    dispatches: u64,
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
        // Point -> adjacent tetrahedra CSR via counts + prefix sum +
        // cursor fill (no per-point growable vectors).
        let mut offsets: Vec<u32> = vec![0; dataset.point_count() + 1];
        for tet in &tets {
            for &node in tet {
                offsets[node as usize + 1] += 1;
            }
        }
        for point in 0..dataset.point_count() {
            offsets[point + 1] += offsets[point];
        }
        let mut flat_adjacency: Vec<u32> = vec![0; offsets[dataset.point_count()] as usize];
        let mut cursors = offsets[..dataset.point_count()].to_vec();
        for (index, tet) in tets.iter().enumerate() {
            for &node in tet {
                let slot = &mut cursors[node as usize];
                flat_adjacency[*slot as usize] = index as u32;
                *slot += 1;
            }
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
        if positions.iter().flatten().any(|v| !v.is_finite()) {
            return Err(TransformError::Degenerate(
                "local positions cannot be represented by GPU f32; use CPU diagnostics".into(),
            ));
        }
        let velocities = packed_velocities(field, vectors)?;

        let device = gpu.device.clone();
        let queue = gpu.queue.clone();
        let point_count = dataset.point_count();
        let too_large = |e: GpuError| TransformError::Degenerate(e.to_string());
        for (label, elements) in [
            ("flow positions", positions.len() * 4),
            ("flow velocities", velocities.len() * 4),
            ("flow tets", tets.len() * 4),
            ("flow tet geometry", tets.len() * 12),
            ("flow adjacency", flat_adjacency.len()),
            ("flow vorticity", point_count * 4),
            ("flow q", point_count),
        ] {
            checked_storage_bytes(elements, 4, &device, label).map_err(too_large)?;
        }
        let (dispatch_x, dispatch_y, grid_stride) =
            dispatch_plan(point_count, &device).map_err(too_large)?;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("point diagnostics compute"),
            source: wgpu::ShaderSource::Wgsl(POINT_DIAGNOSTICS_WGSL.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("point diagnostics layout"),
            entries: &[
                uniform_binding(0),
                storage_binding_rw(1, true),
                storage_binding_rw(2, true),
                storage_binding_rw(3, true),
                storage_binding_rw(4, true),
                storage_binding_rw(5, false),
                storage_binding_rw(6, false),
                storage_binding_rw(7, false),
                storage_binding_rw(8, true),
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
        // Per-tet geometry cache, filled once by the prepass below.
        let tet_geo_bytes = checked_storage_bytes(tets.len(), 12 * 4, &device, "flow tet geometry")
            .map_err(too_large)?;
        let tet_geo_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("flow tet geometry"),
            size: tet_geo_bytes,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        {
            let (geo_x, geo_y, geo_stride) =
                dispatch_plan(tets.len(), &device).map_err(too_large)?;
            let geo_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("tet geometry compute"),
                source: wgpu::ShaderSource::Wgsl(TET_GEOMETRY_WGSL.into()),
            });
            let geo_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("tet geometry layout"),
                entries: &[
                    uniform_binding(0),
                    storage_binding_rw(1, true),
                    storage_binding_rw(2, true),
                    storage_binding_rw(3, false),
                ],
            });
            let geo_pipeline_layout =
                device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("tet geometry pipeline layout"),
                    bind_group_layouts: &[Some(&geo_layout)],
                    immediate_size: 0,
                });
            let geo_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("tet geometry pipeline"),
                layout: Some(&geo_pipeline_layout),
                module: &geo_shader,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
            let geo_uniforms = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("tet geometry uniforms"),
                size: 16,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut uniform_bytes = Vec::with_capacity(8);
            uniform_bytes.extend_from_slice(&(tets.len() as u32).to_le_bytes());
            uniform_bytes.extend_from_slice(&geo_stride.to_le_bytes());
            queue.write_buffer(&geo_uniforms, 0, &uniform_bytes);
            let geo_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tet geometry bind group"),
                layout: &geo_layout,
                entries: &[
                    bind(0, &geo_uniforms),
                    bind(1, &positions_buffer),
                    bind(2, &tets_buffer),
                    bind(3, &tet_geo_buffer),
                ],
            });
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("tet geometry encoder"),
            });
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("tet geometry pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&geo_pipeline);
            pass.set_bind_group(0, &geo_bind_group, &[]);
            pass.dispatch_workgroups(geo_x, geo_y, 1);
            drop(pass);
            queue.submit([encoder.finish()]);
        }
        let output_usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC;
        let make_output = |label: &str, size: u64| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: size.max(4),
                usage: output_usage,
                mapped_at_creation: false,
            })
        };
        let vorticity_buffer = make_output("flow vorticity", (point_count * 16) as u64);
        let q_buffer = make_output("flow q", (point_count * 4) as u64);
        let divergence_buffer = make_output("flow divergence", (point_count * 4) as u64);
        let make_readback = |label: &str, size: u64| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: size.max(4),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            })
        };
        let vorticity_readback = make_readback("vorticity readback", (point_count * 16) as u64);
        let q_readback = make_readback("q readback", (point_count * 4) as u64);
        let divergence_readback = make_readback("divergence readback", (point_count * 4) as u64);
        let uniforms_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("flow uniforms"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut uniform_bytes = Vec::with_capacity(8);
        uniform_bytes.extend_from_slice(&(point_count as u32).to_le_bytes());
        uniform_bytes.extend_from_slice(&grid_stride.to_le_bytes());
        queue.write_buffer(&uniforms_buffer, 0, &uniform_bytes);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("point diagnostics bind group"),
            layout: &layout,
            entries: &[
                bind(0, &uniforms_buffer),
                bind(1, &velocities_buffer),
                bind(2, &tets_buffer),
                bind(3, &offsets_buffer),
                bind(4, &adjacency_buffer),
                bind(5, &vorticity_buffer),
                bind(6, &q_buffer),
                bind(7, &divergence_buffer),
                bind(8, &tet_geo_buffer),
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
            vorticity_readback,
            q_readback,
            divergence_readback,
            bind_group,
            point_count,
            dispatch: (dispatch_x, dispatch_y),
            readback_bytes: 0,
            dispatches: 0,
        })
    }

    /// The device resident buffers live on.
    #[must_use]
    pub fn device(&self) -> &Arc<wgpu::Device> {
        &self.device
    }

    /// The queue dispatches are submitted on.
    #[must_use]
    pub fn queue(&self) -> &Arc<wgpu::Queue> {
        &self.queue
    }

    /// Resident vorticity output (`vec4` per point; w is the validity flag).
    #[must_use]
    pub fn vorticity_buffer(&self) -> &wgpu::Buffer {
        &self.vorticity_buffer
    }

    /// Resident Q-criterion output (one `f32` per point).
    #[must_use]
    pub fn q_buffer(&self) -> &wgpu::Buffer {
        &self.q_buffer
    }

    /// Resident divergence output (one `f32` per point).
    #[must_use]
    pub fn divergence_buffer(&self) -> &wgpu::Buffer {
        &self.divergence_buffer
    }

    /// Cumulative CPU readback bytes through this prepared problem.
    #[must_use]
    pub fn readback_bytes(&self) -> u64 {
        self.readback_bytes
    }

    /// Number of resident dispatches issued.
    #[must_use]
    pub fn dispatches(&self) -> u64 {
        self.dispatches
    }

    /// Runs the point diagnostics kernel, leaving every output resident
    /// in GPU buffers (zero CPU readback). Hosts sharing this device can
    /// copy the buffers GPU-to-GPU, or call
    /// [`readback_diagnostics`](Self::readback_diagnostics) later.
    ///
    /// # Errors
    ///
    /// Returns an error for a wrong-length velocity update. GPU
    /// execution itself is submitted without waiting.
    pub fn run_resident(&mut self, velocities: Option<&[[f64; 3]]>) -> Result<(), TransformError> {
        if let Some(values) = velocities {
            if values.len() != self.point_count {
                return Err(TransformError::Degenerate(format!(
                    "velocity update has {} points, expected {}",
                    values.len(),
                    self.point_count
                )));
            }
            let flat = packed_velocities(&self.base.point_data[self.velocity_field], values)?;
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
        pass.dispatch_workgroups(self.dispatch.0, self.dispatch.1, 1);
        drop(pass);
        self.queue.submit([encoder.finish()]);
        self.dispatches = self.dispatches.saturating_add(1);
        Ok(())
    }

    /// Copies the selected resident outputs back and appends them as
    /// fields. Unselected outputs are neither copied nor appended.
    ///
    /// # Errors
    ///
    /// Returns an error when GPU readback fails.
    pub fn readback_diagnostics(
        &mut self,
        outputs: FlowOutputs,
    ) -> Result<DataSet, TransformError> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("point diagnostics readback encoder"),
            });
        if outputs.vorticity {
            encoder.copy_buffer_to_buffer(
                &self.vorticity_buffer,
                0,
                &self.vorticity_readback,
                0,
                (self.point_count * 16) as u64,
            );
        }
        if outputs.q {
            encoder.copy_buffer_to_buffer(
                &self.q_buffer,
                0,
                &self.q_readback,
                0,
                (self.point_count * 4) as u64,
            );
        }
        if outputs.divergence {
            encoder.copy_buffer_to_buffer(
                &self.divergence_buffer,
                0,
                &self.divergence_readback,
                0,
                (self.point_count * 4) as u64,
            );
        }
        self.queue.submit([encoder.finish()]);
        // The validity flag rides in vorticity.w; when vorticity is not
        // selected it is still needed to mask the other outputs.
        let vorticity_flat = if outputs.vorticity {
            Some({
                let buffer = self.vorticity_readback.clone();
                let count = self.point_count * 4;
                self.read_counted(&buffer, count)?
            })
        } else {
            None
        };
        let validity_flat;
        let validity = if let Some(flat) = &vorticity_flat {
            flat
        } else {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("point diagnostics validity encoder"),
                });
            encoder.copy_buffer_to_buffer(
                &self.vorticity_buffer,
                0,
                &self.vorticity_readback,
                0,
                (self.point_count * 16) as u64,
            );
            self.queue.submit([encoder.finish()]);
            {
                let buffer = self.vorticity_readback.clone();
                let count = self.point_count * 4;
                validity_flat = self.read_counted(&buffer, count)?;
            }
            &validity_flat
        };
        let mut fields: Vec<(Field, &str, &str)> = Vec::new();
        if let Some(flat) = &vorticity_flat {
            fields.push((
                Field::vector(
                    "vorticity",
                    flat.as_chunks::<4>()
                        .0
                        .iter()
                        .map(|c| [f64::from(c[0]), f64::from(c[1]), f64::from(c[2])])
                        .collect::<Vec<_>>(),
                ),
                "Vorticity",
                "1/s",
            ));
        }
        if outputs.q {
            let q = {
                let buffer = self.q_readback.clone();
                let count = self.point_count;
                self.read_counted(&buffer, count)?
            };
            fields.push((
                Field::scalar(
                    "q_criterion",
                    q.iter().map(|v| f64::from(*v)).collect::<Vec<_>>(),
                ),
                "Q-criterion",
                "1/s^2",
            ));
        }
        if outputs.divergence {
            let divergence = {
                let buffer = self.divergence_readback.clone();
                let count = self.point_count;
                self.read_counted(&buffer, count)?
            };
            fields.push((
                Field::scalar(
                    "divergence",
                    divergence.iter().map(|v| f64::from(*v)).collect::<Vec<_>>(),
                ),
                "Divergence",
                "1/s",
            ));
        }
        Ok(append_diagnostics(&self.base, fields, |point| {
            validity[point * 4 + 3] > 0.0
        }))
    }

    fn read_counted(
        &mut self,
        buffer: &wgpu::Buffer,
        count: usize,
    ) -> Result<Vec<f32>, TransformError> {
        let values = read_f32_buffer(&self.device, buffer, count)
            .map_err(|e| TransformError::Degenerate(e.to_string()))?;
        self.readback_bytes = self.readback_bytes.saturating_add((count * 4) as u64);
        Ok(values)
    }

    /// Recomputes diagnostics, optionally with a new velocity field.
    ///
    /// When `velocities` is provided it replaces the prepared field
    /// (same point count); the geometry upload is never repeated.
    /// The prepared validity mask is retained. Masked nodes stay missing until
    /// a new problem is prepared; updates may contain NaNs only at masked nodes.
    ///
    /// # Errors
    ///
    /// Returns an error when GPU execution or readback fails.
    pub fn diagnostics(
        &mut self,
        velocities: Option<&[[f64; 3]]>,
    ) -> Result<DataSet, TransformError> {
        self.diagnostics_selected(velocities, FlowOutputs::all())
    }

    /// Recomputes the selected diagnostics, optionally with a new
    /// velocity field. Unselected outputs are not read back.
    ///
    /// # Errors
    ///
    /// Returns an error when GPU execution or readback fails.
    pub fn diagnostics_selected(
        &mut self,
        velocities: Option<&[[f64; 3]]>,
        outputs: FlowOutputs,
    ) -> Result<DataSet, TransformError> {
        self.run_resident(velocities)?;
        self.readback_diagnostics(outputs)
    }
}

#[cfg(test)]
mod prepared_tests {
    use super::*;
    use gpui_mesh::Field;

    #[test]
    fn resident_run_defers_readback_and_selection_limits_it() {
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
        // Resident outputs are consumable on a host-shared device.
        let shared = GpuFlow::from_shared_device(gpu.device().clone(), gpu.queue().clone());
        let mut prepared = PreparedFlow::new(&shared, &ds, "velocity").unwrap();
        prepared.run_resident(None).unwrap();
        assert_eq!(prepared.dispatches(), 1);
        assert_eq!(
            prepared.readback_bytes(),
            0,
            "resident run must not read back"
        );
        assert_eq!(
            prepared.q_buffer().size(),
            (prepared.point_count * 4) as u64
        );
        // GPU-to-GPU copy of the resident Q buffer, still zero CPU readback.
        let copy = prepared.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("q copy"),
            size: prepared.q_buffer().size(),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let mut encoder = prepared
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_buffer_to_buffer(prepared.q_buffer(), 0, &copy, 0, copy.size());
        prepared.queue().submit([encoder.finish()]);
        assert_eq!(prepared.readback_bytes(), 0);
        // Selected readback materializes only Q.
        let out = prepared
            .readback_diagnostics(FlowOutputs {
                vorticity: false,
                q: true,
                divergence: false,
            })
            .unwrap();
        assert!(out.field(Association::Point, "q_criterion").is_some());
        assert!(out.field(Association::Point, "vorticity").is_none());
        assert!(out.field(Association::Point, "divergence").is_none());
        let q_only_bytes = prepared.readback_bytes();
        assert!(q_only_bytes > 0);
        // Full diagnostics read back strictly more than Q alone.
        let out = prepared.diagnostics(None).unwrap();
        assert!(out.field(Association::Point, "vorticity").is_some());
        assert!(prepared.readback_bytes() > q_only_bytes);
    }

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
        prepared.diagnostics(Some(&zeros[..1])).unwrap_err();
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
