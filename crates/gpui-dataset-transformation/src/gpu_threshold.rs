//! GPU threshold selection: classify → blocked exclusive scan → scatter.
//!
//! The portable-compute pipeline for cell selection (audit Phase C):
//! a classify kernel flags kept cells with exactly the CPU
//! [`threshold`](crate::threshold) semantics (invalid nodes never
//! match; `All` rejects any invalid node), a three-pass blocked scan
//! turns flags into output offsets, and a scatter kernel compacts the
//! kept cell indices. Dataset assembly reuses the CPU subset builder,
//! so GPU and CPU threshold outputs are identical by construction.
//!
//! Scalar comparison happens in `f32` (values and bounds are cast);
//! values that round across a bound in `f32` may classify differently
//! from the `f64` CPU path within one `f32` ULP of the bound.
//!
//! Cost profile (measured, M4 Pro, 48³ grid): one-shot GPU threshold
//! is ~2.6x slower than the CPU filter (4.4 ms vs 1.7 ms) because
//! connectivity upload and readback dominate. The pipeline pays off
//! for repeated selection over resident data and as the portable
//! classify/scan/compact substrate; one-shot callers should prefer
//! the CPU filter.

// Rust guideline compliant 2026-02-21

use crate::TransformError;
use crate::gpu::{GpuError, GpuFlow, checked_storage_bytes, dispatch_plan};
use crate::threshold::{ThresholdMode, subset};
use gpui_mesh::{Association, DataSet};

const CLASSIFY_WGSL: &str = r"
struct Uniforms {
    cell_count: u32,
    grid_stride: u32,
    lower: f32,
    upper: f32,
    mode: u32,
    cell_association: u32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> values: array<f32>;
@group(0) @binding(2) var<storage, read> offsets: array<u32>;
@group(0) @binding(3) var<storage, read> connectivity: array<u32>;
@group(0) @binding(4) var<storage, read_write> flags: array<u32>;

fn in_range(value: f32) -> bool {
    return value >= uniforms.lower && value <= uniforms.upper;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let cell = id.x + id.y * uniforms.grid_stride;
    if (cell >= uniforms.cell_count) {
        return;
    }
    var keep = false;
    if (uniforms.cell_association == 1u) {
        keep = in_range(values[cell]);
    } else if (uniforms.mode == 0u) {
        keep = true;
        for (var i: u32 = offsets[cell]; i < offsets[cell + 1u]; i = i + 1u) {
            keep = keep && in_range(values[connectivity[i]]);
        }
    } else {
        for (var i: u32 = offsets[cell]; i < offsets[cell + 1u]; i = i + 1u) {
            keep = keep || in_range(values[connectivity[i]]);
        }
    }
    flags[cell] = select(0u, 1u, keep);
}
";

const SCAN_WGSL: &str = r"
struct Uniforms {
    count: u32,
    grid_stride: u32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> flags: array<u32>;
@group(0) @binding(2) var<storage, read_write> scanned: array<u32>;
@group(0) @binding(3) var<storage, read_write> block_sums: array<u32>;

var<workgroup> thread_sums: array<u32, 256>;

// Block-local exclusive scan: thread-serial chunks of 16, then a serial
// scan of the 256 thread sums by thread 0. Block totals go to block_sums.
@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    let block = workgroup_id.x + workgroup_id.y * uniforms.grid_stride;
    let base = block * 4096u + local_id.x * 16u;
    var sum = 0u;
    for (var k: u32 = 0u; k < 16u; k = k + 1u) {
        let i = base + k;
        if (i < uniforms.count) {
            scanned[i] = sum;
            sum = sum + flags[i];
        }
    }
    thread_sums[local_id.x] = sum;
    workgroupBarrier();
    if (local_id.x == 0u) {
        var acc = 0u;
        for (var t: u32 = 0u; t < 256u; t = t + 1u) {
            let s = thread_sums[t];
            thread_sums[t] = acc;
            acc = acc + s;
        }
        block_sums[block] = acc;
    }
    workgroupBarrier();
    let offset = thread_sums[local_id.x];
    for (var k: u32 = 0u; k < 16u; k = k + 1u) {
        let i = base + k;
        if (i < uniforms.count) {
            scanned[i] = scanned[i] + offset;
        }
    }
}
";

const SCAN_TOTALS_WGSL: &str = r"
struct Uniforms {
    block_count: u32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> block_sums: array<u32>;
@group(0) @binding(2) var<storage, read_write> block_offsets: array<u32>;
@group(0) @binding(3) var<storage, read_write> total: array<u32>;

@compute @workgroup_size(1)
fn main() {
    var acc = 0u;
    for (var b: u32 = 0u; b < uniforms.block_count; b = b + 1u) {
        block_offsets[b] = acc;
        acc = acc + block_sums[b];
    }
    total[0] = acc;
}
";

const SCATTER_WGSL: &str = r"
struct Uniforms {
    count: u32,
    grid_stride: u32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> flags: array<u32>;
@group(0) @binding(2) var<storage, read> scanned: array<u32>;
@group(0) @binding(3) var<storage, read> block_offsets: array<u32>;
@group(0) @binding(4) var<storage, read_write> kept: array<u32>;

@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    let block = workgroup_id.x + workgroup_id.y * uniforms.grid_stride;
    let base = block * 4096u + local_id.x * 16u;
    for (var k: u32 = 0u; k < 16u; k = k + 1u) {
        let i = base + k;
        if (i < uniforms.count && flags[i] == 1u) {
            kept[scanned[i] + block_offsets[block]] = i;
        }
    }
}
";

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

fn storage_binding(index: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
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

fn read_u32_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    count: usize,
) -> Result<Vec<u32>, GpuError> {
    if count == 0 {
        return Ok(Vec::new());
    }
    let size = (count * 4) as u64;
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gpu threshold readback"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("gpu threshold readback"),
    });
    encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, size);
    queue.submit(std::iter::once(encoder.finish()));
    let slice = staging.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .map_err(|e| GpuError::Readback(e.to_string()))?;
    rx.recv()
        .map_err(|e| GpuError::Readback(e.to_string()))?
        .map_err(|e| GpuError::Readback(e.to_string()))?;
    let data = slice.get_mapped_range();
    let values: Vec<u32> = bytemuck::cast_slice(&data[..count * 4]).to_vec();
    drop(data);
    staging.unmap();
    Ok(values)
}

/// Compiled threshold pipelines, built once per [`GpuFlow`] instead of
/// per call (pipeline creation dominated small problems entirely).
#[derive(Debug)]
pub(crate) struct ThresholdPipelines {
    pub(crate) classify_layout: wgpu::BindGroupLayout,
    pub(crate) classify: wgpu::ComputePipeline,
    pub(crate) scan_layout: wgpu::BindGroupLayout,
    pub(crate) scan: wgpu::ComputePipeline,
    pub(crate) totals_layout: wgpu::BindGroupLayout,
    pub(crate) totals: wgpu::ComputePipeline,
    pub(crate) scatter_layout: wgpu::BindGroupLayout,
    pub(crate) scatter: wgpu::ComputePipeline,
}

impl ThresholdPipelines {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        fn make_pipeline(
            device: &wgpu::Device,
            label: &str,
            source: &str,
            entries: &[wgpu::BindGroupLayoutEntry],
        ) -> (wgpu::BindGroupLayout, wgpu::ComputePipeline) {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
                entries,
            });
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
            let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                module: &module,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
            (layout, pipeline)
        }
        let (classify_layout, classify) = make_pipeline(
            device,
            "threshold classify",
            CLASSIFY_WGSL,
            &[
                uniform_binding(0),
                storage_binding(1, true),
                storage_binding(2, true),
                storage_binding(3, true),
                storage_binding(4, false),
            ],
        );
        let (scan_layout, scan) = make_pipeline(
            device,
            "threshold scan",
            SCAN_WGSL,
            &[
                uniform_binding(0),
                storage_binding(1, true),
                storage_binding(2, false),
                storage_binding(3, false),
            ],
        );
        let (totals_layout, totals) = make_pipeline(
            device,
            "threshold scan totals",
            SCAN_TOTALS_WGSL,
            &[
                uniform_binding(0),
                storage_binding(1, true),
                storage_binding(2, false),
                storage_binding(3, false),
            ],
        );
        let (scatter_layout, scatter) = make_pipeline(
            device,
            "threshold scatter",
            SCATTER_WGSL,
            &[
                uniform_binding(0),
                storage_binding(1, true),
                storage_binding(2, true),
                storage_binding(3, true),
                storage_binding(4, false),
            ],
        );
        Self {
            classify_layout,
            classify,
            scan_layout,
            scan,
            totals_layout,
            totals,
            scatter_layout,
            scatter,
        }
    }
}

impl GpuFlow {
    /// Classifies, scans, and compacts threshold-kept cell indices on
    /// the GPU, in source order.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid datasets/bounds, missing or
    /// non-scalar fields, device-limit overflows, or GPU readback
    /// failure.
    pub fn threshold_cells(
        &self,
        dataset: &DataSet,
        field_name: &str,
        association: Association,
        lower: f64,
        upper: f64,
        mode: ThresholdMode,
    ) -> Result<Vec<usize>, TransformError> {
        dataset.validate()?;
        if lower.is_nan() || upper.is_nan() || lower > upper {
            return Err(TransformError::Degenerate(
                "threshold bounds must be ordered and not NaN".into(),
            ));
        }
        let field = dataset
            .field(association, field_name)
            .ok_or_else(|| TransformError::FieldNotFound(field_name.into()))?;
        let values = field
            .as_scalar()
            .ok_or_else(|| TransformError::WrongFieldShape {
                field: field_name.into(),
                reason: "threshold requires a scalar field".into(),
            })?;
        let cell_count = dataset.cell_count();
        if cell_count == 0 {
            return Ok(Vec::new());
        }
        let packed: Vec<f32> = values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                if field.is_valid(index) {
                    *value as f32
                } else {
                    f32::NAN
                }
            })
            .collect();
        let device = &**self.device();
        let queue = &**self.queue();
        let too_large = |e: GpuError| TransformError::Degenerate(e.to_string());
        for (label, elements) in [
            ("threshold values", packed.len()),
            ("threshold offsets", dataset.cells.offsets.len()),
            ("threshold connectivity", dataset.cells.connectivity.len()),
            ("threshold flags", cell_count),
        ] {
            checked_storage_bytes(elements, 4, device, label).map_err(too_large)?;
        }
        let storage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        let make = |label: &str, bytes: &[u8], usage: wgpu::BufferUsages| {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: (bytes.len() as u64).max(4),
                usage,
                mapped_at_creation: false,
            });
            queue.write_buffer(&buffer, 0, bytes);
            buffer
        };
        let make_sized = |label: &str, size: u64, usage: wgpu::BufferUsages| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: size.max(4),
                usage,
                mapped_at_creation: false,
            })
        };
        let values_buffer = make("threshold values", bytemuck::cast_slice(&packed), storage);
        // Cell offsets are u64 on the CPU; the kernel indexes u32.
        let offsets32: Vec<u32> = dataset
            .cells
            .offsets
            .iter()
            .map(|&offset| {
                u32::try_from(offset).map_err(|_| {
                    TransformError::Degenerate("connectivity exceeds GPU u32 indexing".into())
                })
            })
            .collect::<Result<_, _>>()?;
        let offsets_buffer = make(
            "threshold offsets",
            bytemuck::cast_slice(&offsets32),
            storage,
        );
        let connectivity_buffer = make(
            "threshold connectivity",
            bytemuck::cast_slice(&dataset.cells.connectivity),
            storage,
        );
        let out_usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC;
        let flags_buffer = make_sized("threshold flags", (cell_count * 4) as u64, out_usage);
        let scanned_buffer = make_sized("threshold scan", (cell_count * 4) as u64, out_usage);
        let kept_buffer = make_sized("threshold kept", (cell_count * 4) as u64, out_usage);
        let blocks = cell_count.div_ceil(4096);
        let block_sums = make_sized("threshold block sums", (blocks * 4) as u64, out_usage);
        let block_offsets = make_sized("threshold block offsets", (blocks * 4) as u64, out_usage);
        let total_buffer = make_sized("threshold total", 4, out_usage);
        let pipelines = &self.threshold;
        let (classify_layout, classify_pipeline) =
            (&pipelines.classify_layout, &pipelines.classify);
        let (scan_layout, scan_pipeline) = (&pipelines.scan_layout, &pipelines.scan);
        let (totals_layout, totals_pipeline) = (&pipelines.totals_layout, &pipelines.totals);
        let (scatter_layout, scatter_pipeline) = (&pipelines.scatter_layout, &pipelines.scatter);
        let (classify_x, classify_y, classify_stride) =
            dispatch_plan(cell_count, device).map_err(too_large)?;
        let max_dim = device.limits().max_compute_workgroups_per_dimension;
        let scan_x = (blocks as u32).min(max_dim);
        let scan_y = (blocks as u32).div_ceil(scan_x.max(1));
        if scan_y > max_dim {
            return Err(TransformError::Degenerate(
                "threshold scan grid exceeded".into(),
            ));
        }

        // One uniform buffer per stage so all four dispatches can ride
        // a single encoder/submission (queue writes between passes of
        // one encoder would not be ordered per-pass).
        let make_uniforms = |words: &[u32]| {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("threshold uniforms"),
                size: 32,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut bytes = Vec::with_capacity(32);
            for word in words {
                bytes.extend_from_slice(&word.to_le_bytes());
            }
            queue.write_buffer(&buffer, 0, &bytes);
            buffer
        };
        let classify_uniforms = make_uniforms(&[
            cell_count as u32,
            classify_stride,
            (lower as f32).to_bits(),
            (upper as f32).to_bits(),
            u32::from(mode == ThresholdMode::Any),
            u32::from(association == Association::Cell),
        ]);
        let scan_uniforms = make_uniforms(&[cell_count as u32, scan_x]);
        let totals_uniforms = make_uniforms(&[blocks as u32]);
        let classify_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("threshold classify bind group"),
            layout: classify_layout,
            entries: &[
                bind(0, &classify_uniforms),
                bind(1, &values_buffer),
                bind(2, &offsets_buffer),
                bind(3, &connectivity_buffer),
                bind(4, &flags_buffer),
            ],
        });
        let scan_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("threshold scan bind group"),
            layout: scan_layout,
            entries: &[
                bind(0, &scan_uniforms),
                bind(1, &flags_buffer),
                bind(2, &scanned_buffer),
                bind(3, &block_sums),
            ],
        });
        let totals_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("threshold totals bind group"),
            layout: totals_layout,
            entries: &[
                bind(0, &totals_uniforms),
                bind(1, &block_sums),
                bind(2, &block_offsets),
                bind(3, &total_buffer),
            ],
        });
        let scatter_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("threshold scatter bind group"),
            layout: scatter_layout,
            entries: &[
                bind(0, &scan_uniforms),
                bind(1, &flags_buffer),
                bind(2, &scanned_buffer),
                bind(3, &block_offsets),
                bind(4, &kept_buffer),
            ],
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("gpu threshold"),
        });
        for (pipeline, group, x, y) in [
            (classify_pipeline, &classify_group, classify_x, classify_y),
            (scan_pipeline, &scan_group, scan_x, scan_y),
            (totals_pipeline, &totals_group, 1, 1),
            (scatter_pipeline, &scatter_group, scan_x, scan_y),
        ] {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("gpu threshold pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.dispatch_workgroups(x, y, 1);
        }
        queue.submit([encoder.finish()]);

        let total = read_u32_buffer(device, queue, &total_buffer, 1).map_err(too_large)?;
        let kept = total.first().copied().unwrap_or(0) as usize;
        if kept > cell_count {
            return Err(TransformError::Degenerate(
                "GPU threshold produced an out-of-range count".into(),
            ));
        }
        let indices = read_u32_buffer(device, queue, &kept_buffer, kept).map_err(too_large)?;
        Ok(indices.iter().map(|&cell| cell as usize).collect())
    }
}

/// GPU-accelerated [`threshold`](crate::threshold): classification,
/// scan, and compaction run on the GPU; output assembly reuses the CPU
/// subset builder, so results match the CPU filter exactly (modulo the
/// documented `f32` bound rounding).
///
/// # Errors
///
/// Returns the same errors as the CPU filter plus GPU failures.
pub fn threshold_gpu(
    gpu: &GpuFlow,
    dataset: &DataSet,
    field_name: &str,
    association: Association,
    lower: f64,
    upper: f64,
    mode: ThresholdMode,
) -> Result<DataSet, TransformError> {
    let cells = gpu.threshold_cells(dataset, field_name, association, lower, upper, mode)?;
    let mut keep = vec![false; dataset.cell_count()];
    for cell in cells {
        keep[cell] = true;
    }
    subset(dataset, &keep)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::threshold::threshold;
    use gpui_mesh::Field;

    fn gpu() -> Option<GpuFlow> {
        match GpuFlow::new() {
            Ok(gpu) => Some(gpu),
            Err(crate::gpu::GpuError::NoAdapter)
                if std::env::var_os("GPUI_REQUIRE_GPU").is_none() =>
            {
                None
            }
            Err(e) => panic!("GPU required: {e}"),
        }
    }

    fn grid() -> DataSet {
        let mut ds = DataSet::structured_grid("g", [6, 5, 4], [-1.0; 3], [0.25; 3]).unwrap();
        let values: Vec<f64> = ds
            .positions
            .iter()
            .map(|p| p[0] * 0.5 + p[1] - p[2] * 0.25)
            .collect();
        ds.point_data.push(Field::scalar("p", values));
        let cell_values: Vec<f64> = (0..ds.cell_count())
            .map(|cell| (cell % 7) as f64 / 7.0 - 0.4)
            .collect();
        ds.cell_data
            .push(Field::scalar("c", cell_values).on_cells());
        ds
    }

    fn assert_same(cpu: &DataSet, gpu: &DataSet) {
        assert_eq!(gpu.positions, cpu.positions, "positions differ");
        assert_eq!(gpu.cells.types, cpu.cells.types, "cell types differ");
        assert_eq!(gpu.cells.offsets, cpu.cells.offsets, "offsets differ");
        assert_eq!(
            gpu.cells.connectivity, cpu.cells.connectivity,
            "connectivity differs"
        );
        assert_eq!(gpu.point_data, cpu.point_data, "point data differs");
        assert_eq!(gpu.cell_data, cpu.cell_data, "cell data differs");
    }

    #[test]
    fn gpu_threshold_matches_cpu_point_and_cell() {
        let Some(gpu) = gpu() else { return };
        let ds = grid();
        for mode in [ThresholdMode::All, ThresholdMode::Any] {
            let cpu = threshold(&ds, "p", Association::Point, -0.2, 0.6, mode).unwrap();
            let gpu_ds =
                threshold_gpu(&gpu, &ds, "p", Association::Point, -0.2, 0.6, mode).unwrap();
            assert_same(&cpu, &gpu_ds);
        }
        let cpu = threshold(&ds, "c", Association::Cell, -0.3, 0.1, ThresholdMode::All).unwrap();
        let gpu_ds = threshold_gpu(
            &gpu,
            &ds,
            "c",
            Association::Cell,
            -0.3,
            0.1,
            ThresholdMode::All,
        )
        .unwrap();
        assert_same(&cpu, &gpu_ds);
        // Empty and full selections.
        for (lo, hi) in [(10.0, 20.0), (-100.0, 100.0)] {
            let cpu = threshold(&ds, "p", Association::Point, lo, hi, ThresholdMode::All).unwrap();
            let gpu_ds = threshold_gpu(
                &gpu,
                &ds,
                "p",
                Association::Point,
                lo,
                hi,
                ThresholdMode::All,
            )
            .unwrap();
            assert_same(&cpu, &gpu_ds);
        }
        // Masked points never match on either path.
        let mut masked = ds.clone();
        masked.point_data[0].valid = Some(
            (0..masked.point_count())
                .map(|point| point % 3 != 0)
                .collect::<Vec<_>>()
                .into(),
        );
        let cpu = threshold(
            &masked,
            "p",
            Association::Point,
            -10.0,
            10.0,
            ThresholdMode::Any,
        )
        .unwrap();
        let gpu_ds = threshold_gpu(
            &gpu,
            &masked,
            "p",
            Association::Point,
            -10.0,
            10.0,
            ThresholdMode::Any,
        )
        .unwrap();
        assert_same(&cpu, &gpu_ds);
    }
}
