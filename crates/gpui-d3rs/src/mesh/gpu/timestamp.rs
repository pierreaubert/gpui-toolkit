use gpui_wgpu::WgpuContext;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const QUERY_COUNT: u32 = 2;
const QUERY_BYTES: u64 = QUERY_COUNT as u64 * std::mem::size_of::<u64>() as u64;

/// Optional asynchronous GPU duration recorder for one retained custom draw.
///
/// The recorder is deliberately best-effort. Adapters without encoder
/// timestamp support keep the normal render path unchanged, while supported
/// adapters resolve the previous frame's timestamps without blocking the
/// current frame. The result is therefore a GPU execution duration, not a CPU
/// command-recording duration.
pub(crate) struct GpuTimestampRecorder {
    query_set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    readback_buffer: wgpu::Buffer,
    callback_result: Arc<Mutex<Option<Result<(), wgpu::BufferAsyncError>>>>,
    pending: bool,
    map_requested: bool,
    active: bool,
}

impl GpuTimestampRecorder {
    pub(crate) fn new(ctx: &WgpuContext, label: &str) -> Option<Self> {
        // Timestamp queries are a diagnostic instrument. Keep them opt-in so
        // normal interactive rendering does not inherit backend-specific
        // query/teardown costs or known flaky timestamp behavior.
        if !ctx.supports_timestamp_queries()
            || std::env::var_os("SOTF_GPU_TIMESTAMPS").as_deref() != Some(std::ffi::OsStr::new("1"))
        {
            return None;
        }

        Some(Self {
            query_set: ctx.device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some(label),
                count: QUERY_COUNT,
                ty: wgpu::QueryType::Timestamp,
            }),
            resolve_buffer: ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("mesh_gpu_timestamp_resolve"),
                size: QUERY_BYTES,
                usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::QUERY_RESOLVE,
                mapped_at_creation: false,
            }),
            readback_buffer: ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("mesh_gpu_timestamp_readback"),
                size: QUERY_BYTES,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            callback_result: Arc::new(Mutex::new(None)),
            pending: false,
            map_requested: false,
            active: false,
        })
    }

    /// Poll a previously submitted query without waiting for the GPU.
    pub(crate) fn poll(&mut self, ctx: &WgpuContext) -> Option<Duration> {
        if !self.pending {
            return None;
        }

        if !self.map_requested {
            let callback_result = Arc::clone(&self.callback_result);
            self.readback_buffer
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    if let Ok(mut slot) = callback_result.lock() {
                        *slot = Some(result);
                    }
                });
            self.map_requested = true;
        }
        let _ = ctx.device.poll(wgpu::PollType::Poll);
        let result = self.callback_result.lock().ok()?.take()?;
        self.pending = false;
        self.map_requested = false;
        if result.is_err() {
            return None;
        }

        let timestamps = {
            let view = self.readback_buffer.slice(..).get_mapped_range();
            bytemuck::cast_slice::<u8, u64>(&view).to_vec()
        };
        self.readback_buffer.unmap();

        let [start, end] = timestamps.as_slice() else {
            return None;
        };
        let ticks = end.wrapping_sub(*start);
        let nanos = (ticks as f64 * f64::from(ctx.timestamp_period_ns())).round();
        if !nanos.is_finite() || nanos <= 0.0 {
            return None;
        }
        Some(Duration::from_nanos(nanos.min(u64::MAX as f64) as u64))
    }

    /// Reserve the next timestamp pair when the previous sample has
    /// completed.
    pub(crate) fn begin(&mut self) -> bool {
        if self.pending || self.active {
            return false;
        }
        self.active = true;
        true
    }

    /// Return render-pass boundary timestamp writes for the active sample.
    pub(crate) fn render_pass_writes(
        &self,
        active: bool,
    ) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        active.then_some(wgpu::RenderPassTimestampWrites {
            query_set: &self.query_set,
            beginning_of_pass_write_index: Some(0),
            end_of_pass_write_index: Some(1),
        })
    }

    /// Resolve and asynchronously map the current timestamp pair.
    pub(crate) fn finish(&mut self, encoder: &mut wgpu::CommandEncoder, active: bool) {
        if !active || !self.active {
            return;
        }
        encoder.resolve_query_set(&self.query_set, 0..QUERY_COUNT, &self.resolve_buffer, 0);
        encoder.copy_buffer_to_buffer(
            &self.resolve_buffer,
            0,
            &self.readback_buffer,
            0,
            QUERY_BYTES,
        );

        self.pending = true;
        self.active = false;
    }
}
