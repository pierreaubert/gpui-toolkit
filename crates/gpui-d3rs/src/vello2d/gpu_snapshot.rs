//! Blocking offscreen GPU snapshot of a [`ChartScene`].
//!
//! Lab/QA companion to the live `WgpuVelloDraw` custom draw: renders the
//! same scene through the same vello pipeline ([`to_vello_scene`] +
//! `render_to_texture`) on a private headless device, then reads the pixels
//! back. vello stores straight alpha, so the readback is converted to
//! premultiplied RGBA8, matching
//! [`CpuRasterizer`](crate::vello2d::CpuRasterizer) — [`compare_rgba`] is an
//! apples-to-apples CPU-vs-GPU check that works on any
//! host renderer — including Metal, where the live custom-draw path never
//! dispatches and a forced-GPU element would paint nothing.

use crate::vello2d::{ChartScene, to_vello_scene};
use std::cell::RefCell;
use std::fmt;
use std::mem::ManuallyDrop;
use std::sync::mpsc;
use std::time::Duration;
use vello::kurbo::Affine;
use vello::peniko::Color;
use vello::{AaConfig, AaSupport, RenderParams, Renderer, RendererOptions};

/// Why a GPU snapshot could not be produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotError {
    /// No wgpu adapter (headless CI without a GPU, primarily).
    NoAdapter,
    /// Adapter found, but device or vello renderer creation failed.
    Init(String),
    /// `render_to_texture` failed.
    Render(String),
    /// Texture-to-buffer copy, mapping, or callback wait failed.
    Readback(String),
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAdapter => write!(f, "no wgpu adapter available for GPU snapshot"),
            Self::Init(detail) => write!(f, "GPU snapshot device init failed: {detail}"),
            Self::Render(detail) => write!(f, "GPU snapshot render failed: {detail}"),
            Self::Readback(detail) => write!(f, "GPU snapshot readback failed: {detail}"),
        }
    }
}

impl std::error::Error for SnapshotError {}

struct SnapshotDevice {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: Renderer,
}

enum SnapshotState {
    Uninit,
    /// Intentionally never dropped: destroying a wgpu `Device`/`Renderer`
    /// at thread teardown touches wgpu-core thread-locals that may already
    /// be gone, aborting the process (observed as SIGTRAP on test-thread
    /// exit). One leaked device per snapshotting thread is the documented
    /// trade-off; per-call textures and staging buffers still drop inline.
    Ready(ManuallyDrop<Box<SnapshotDevice>>),
    Failed,
}

thread_local! {
    /// Headless device + vello renderer, created once per thread. Shader
    /// compilation makes first use slow; steady-state snapshots reuse it.
    static SNAPSHOT_STATE: RefCell<SnapshotState> = const { RefCell::new(SnapshotState::Uninit) };
}

fn create_device() -> Result<SnapshotDevice, SnapshotError> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .map_err(|_| SnapshotError::NoAdapter)?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("vello2d_snapshot device"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        trace: wgpu::Trace::Off,
        experimental_features: wgpu::ExperimentalFeatures::default(),
    }))
    .map_err(|err| SnapshotError::Init(err.to_string()))?;
    let renderer = Renderer::new(
        &device,
        RendererOptions {
            antialiasing_support: AaSupport::area_only(),
            ..Default::default()
        },
    )
    .map_err(|err| SnapshotError::Init(err.to_string()))?;
    Ok(SnapshotDevice {
        device,
        queue,
        renderer,
    })
}

/// Render `scene` (logical coordinates) on the GPU into a `width`x`height`
/// physical pixmap, applying the same uniform `scale` as
/// [`CpuRasterizer::rasterize`](crate::vello2d::CpuRasterizer::rasterize).
/// Returns premultiplied RGBA8 (vello stores straight alpha; converted on
/// readback). Blocks the
/// calling thread on device creation (once), rendering, and readback.
pub fn snapshot_scene_gpu(
    scene: &ChartScene,
    width: u32,
    height: u32,
    scale: f32,
) -> Result<Vec<u8>, SnapshotError> {
    let width = width.max(1);
    let height = height.max(1);
    SNAPSHOT_STATE.with(|state| {
        let mut state = state.borrow_mut();
        if matches!(*state, SnapshotState::Uninit) {
            *state = match create_device() {
                Ok(device) => SnapshotState::Ready(ManuallyDrop::new(Box::new(device))),
                Err(SnapshotError::NoAdapter) => return Err(SnapshotError::NoAdapter),
                Err(err) => {
                    log::error!("vello2d: {err}");
                    SnapshotState::Failed
                }
            };
        }
        let SnapshotState::Ready(device) = &mut *state else {
            return Err(SnapshotError::Init(
                "cached GPU snapshot device failure".to_string(),
            ));
        };
        // Split borrows so the renderer (&mut) and device/queue (&) coexist.
        let SnapshotDevice {
            device: wgpu_device,
            queue,
            renderer,
        } = &mut ***device;

        let vello_scene = to_vello_scene(scene, Affine::scale(scale.max(0.01) as f64));
        let texture = wgpu_device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vello2d_snapshot"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        renderer
            .render_to_texture(
                wgpu_device,
                queue,
                &vello_scene,
                &texture.create_view(&Default::default()),
                &RenderParams {
                    base_color: Color::TRANSPARENT,
                    width,
                    height,
                    antialiasing_method: AaConfig::Area,
                },
            )
            .map_err(|err| SnapshotError::Render(err.to_string()))?;

        let mut rgba = read_texture(wgpu_device, queue, &texture, width, height)?;
        premultiply_rgba_in_place(&mut rgba);
        Ok(rgba)
    })
}

/// Convert straight-alpha RGBA8 to premultiplied in place. vello's fine
/// rasterizer unpremultiplies before storing (`fg.rgb / max(fg.a, eps)` in
/// `fine.wgsl`); the CPU oracle and GPUI atlases both work premultiplied.
fn premultiply_rgba_in_place(rgba: &mut [u8]) {
    for pixel in rgba.chunks_exact_mut(4) {
        let alpha = u32::from(pixel[3]);
        if alpha == 0 {
            pixel[0] = 0;
            pixel[1] = 0;
            pixel[2] = 0;
        } else if alpha < 255 {
            for channel in pixel.iter_mut().take(3) {
                *channel = ((u32::from(*channel) * alpha + 127) / 255) as u8;
            }
        }
    }
}

fn read_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, SnapshotError> {
    let row_bytes = width
        .checked_mul(4)
        .ok_or_else(|| SnapshotError::Readback("row size overflow".to_string()))?;
    let padded_row_bytes = row_bytes
        .div_ceil(256)
        .checked_mul(256)
        .ok_or_else(|| SnapshotError::Readback("row alignment overflow".to_string()))?;
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("vello2d_snapshot_readback"),
        size: u64::from(padded_row_bytes) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("vello2d_snapshot_copy"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row_bytes),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(std::iter::once(encoder.finish()));

    let slice = staging.slice(..);
    let (sender, receiver) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    let _ = device.poll(wgpu::PollType::Wait {
        submission_index: Default::default(),
        timeout: Some(Duration::from_secs(10)),
    });
    receiver
        .recv_timeout(Duration::from_secs(10))
        .map_err(|err| SnapshotError::Readback(format!("readback callback timed out: {err}")))?
        .map_err(|err| SnapshotError::Readback(format!("readback mapping failed: {err}")))?;

    let data = slice.get_mapped_range();
    let mut rgba = vec![0u8; width as usize * height as usize * 4];
    let row_bytes = row_bytes as usize;
    for (y, row) in rgba.chunks_exact_mut(row_bytes).enumerate() {
        let start = y * padded_row_bytes as usize;
        row.copy_from_slice(&data[start..start + row_bytes]);
    }
    drop(data);
    staging.unmap();
    Ok(rgba)
}

/// Summary of per-channel absolute differences between two premultiplied
/// RGBA8 buffers (all four channels included: alpha divergence matters).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PixelDiff {
    /// Pixels compared.
    pub pixels: usize,
    /// Mean |a-b| over all channels.
    pub mean_abs: f64,
    /// Worst single-channel |a-b|.
    pub max_abs: u8,
    /// Fraction of pixels with any channel differing by more than `tolerance`.
    pub frac_over_tol: f64,
}

/// Compare two premultiplied RGBA8 buffers. Returns `None` on length
/// mismatch, non-RGBA length, or empty input.
pub fn compare_rgba(a: &[u8], b: &[u8], tolerance: u8) -> Option<PixelDiff> {
    if a.len() != b.len() || a.is_empty() || !a.len().is_multiple_of(4) {
        return None;
    }
    let pixels = a.len() / 4;
    let mut sum = 0u64;
    let mut max = 0u8;
    let mut over = 0usize;
    for (index, pixel) in a.chunks_exact(4).enumerate() {
        let other = &b[index * 4..index * 4 + 4];
        let mut pixel_over = false;
        for channel in 0..4 {
            let delta = pixel[channel].abs_diff(other[channel]);
            sum += u64::from(delta);
            max = max.max(delta);
            pixel_over |= delta > tolerance;
        }
        over += usize::from(pixel_over);
    }
    Some(PixelDiff {
        pixels,
        mean_abs: sum as f64 / a.len() as f64,
        max_abs: max,
        frac_over_tol: over as f64 / pixels as f64,
    })
}

/// Compare 2x2-downsampled block averages instead of raw pixels. Subpixel
/// coverage rounding can differ between engines while block averages of the
/// same picture agree: this metric stays quiet on that noise while
/// structural errors (wrong scale, offset, missing shapes) still read hot.
/// `width` and `height` must both be even. Returns `None` on the same shape
/// errors as [`compare_rgba`], plus odd dimensions.
pub fn compare_rgba_downsampled2(
    a: &[u8],
    b: &[u8],
    width: usize,
    height: usize,
    tolerance: f32,
) -> Option<PixelDiff> {
    if a.len() != b.len()
        || a.is_empty()
        || !a.len().is_multiple_of(4)
        || a.len() != width * height * 4
        || !width.is_multiple_of(2)
        || !height.is_multiple_of(2)
    {
        return None;
    }
    let blocks_wide = width / 2;
    let blocks_high = height / 2;
    let pixels = blocks_wide * blocks_high;
    let block_avg = |buf: &[u8], bx: usize, by: usize, channel: usize| {
        let mut sum = 0u32;
        for dy in 0..2 {
            for dx in 0..2 {
                sum += u32::from(buf[((by * 2 + dy) * width + bx * 2 + dx) * 4 + channel]);
            }
        }
        sum as f32 / 4.0
    };
    let mut sum = 0.0f64;
    let mut max = 0.0f32;
    let mut over = 0usize;
    for by in 0..blocks_high {
        for bx in 0..blocks_wide {
            let mut block_over = false;
            for channel in 0..4 {
                let delta = (block_avg(a, bx, by, channel) - block_avg(b, bx, by, channel)).abs();
                sum += f64::from(delta);
                max = max.max(delta);
                block_over |= delta > tolerance;
            }
            over += usize::from(block_over);
        }
    }
    Some(PixelDiff {
        pixels,
        mean_abs: sum / a.len() as f64,
        max_abs: max.round().clamp(0.0, 255.0) as u8,
        frac_over_tol: over as f64 / pixels as f64,
    })
}

/// Absolute-difference visualization: each channel is `|a-b| * amplify`
/// (clamped), alpha is opaque wherever any channel differs. Returns `None`
/// on the same shape errors as [`compare_rgba`].
pub fn diff_image_rgba(a: &[u8], b: &[u8], amplify: f32) -> Option<Vec<u8>> {
    if a.len() != b.len() || a.is_empty() || !a.len().is_multiple_of(4) {
        return None;
    }
    let mut out = Vec::with_capacity(a.len());
    for pixel in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        let (pa, pb) = pixel;
        let mut different = false;
        for channel in 0..3 {
            let delta = pa[channel].abs_diff(pb[channel]);
            different |= delta > 0;
            out.push((f32::from(delta) * amplify).clamp(0.0, 255.0) as u8);
        }
        out.push(u8::from(different) * 255);
    }
    Some(out)
}

#[cfg(test)]
mod compare_tests {
    use super::{compare_rgba, compare_rgba_downsampled2, diff_image_rgba};

    #[test]
    fn identical_buffers_compare_clean() {
        let buf = vec![10u8, 200, 30, 255, 0, 0, 0, 0];
        let diff = compare_rgba(&buf, &buf, 0).expect("same shape compares");
        assert_eq!(diff.pixels, 2);
        assert_eq!(diff.mean_abs, 0.0);
        assert_eq!(diff.max_abs, 0);
        assert_eq!(diff.frac_over_tol, 0.0);
    }

    #[test]
    fn single_channel_delta_counts_once_per_pixel() {
        let a = vec![100u8, 100, 100, 255];
        let b = vec![110u8, 100, 100, 255];
        let diff = compare_rgba(&a, &b, 4).expect("same shape compares");
        assert_eq!(diff.pixels, 1);
        assert!((diff.mean_abs - 2.5).abs() < 1e-9, "mean {}", diff.mean_abs);
        assert_eq!(diff.max_abs, 10);
        assert_eq!(diff.frac_over_tol, 1.0);
        // Within tolerance: still reported in mean/max, not in the fraction.
        let loose = compare_rgba(&a, &b, 10).expect("same shape compares");
        assert_eq!(loose.frac_over_tol, 0.0);
        assert_eq!(loose.max_abs, 10);
    }

    #[test]
    fn shape_errors_return_none() {
        assert!(compare_rgba(&[], &[], 0).is_none());
        assert!(compare_rgba(&[1, 2, 3], &[1, 2, 3], 0).is_none());
        assert!(compare_rgba(&[0u8; 4], &[0u8; 8], 0).is_none());
        assert!(diff_image_rgba(&[0u8; 4], &[0u8; 8], 4.0).is_none());
    }

    #[test]
    fn downsampled_compare_quiet_on_fringe_hot_on_shift() {
        // Opaque 4x4, one bright pixel at (0,0). Nudging it moves one block
        // average a little; relocating it to another block moves two a lot.
        let mut a = vec![0u8; 4 * 4 * 4];
        for pixel in a.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
        a[0] = 255;
        a[1] = 255;
        a[2] = 255;
        let mut fringe = a.clone();
        fringe[0] = 200;
        let fringe = compare_rgba_downsampled2(&a, &fringe, 4, 4, 8.0).expect("even dims compare");
        assert!(fringe.mean_abs < 1.0, "fringe {}", fringe.mean_abs);
        assert_eq!(fringe.frac_over_tol, 0.25);
        let mut shifted = a.clone();
        shifted[0] = 0;
        shifted[1] = 0;
        shifted[2] = 0;
        shifted[(2 * 4) * 4] = 255;
        shifted[(2 * 4) * 4 + 1] = 255;
        shifted[(2 * 4) * 4 + 2] = 255;
        let moved = compare_rgba_downsampled2(&a, &shifted, 4, 4, 8.0).expect("even dims compare");
        assert!(moved.mean_abs > 3.0, "shift {}", moved.mean_abs);
        assert_eq!(moved.frac_over_tol, 0.5);
    }

    #[test]
    fn downsampled_compare_rejects_bad_shapes() {
        let buf = vec![0u8; 4 * 4 * 4];
        assert!(compare_rgba_downsampled2(&buf, &buf, 4, 3, 8.0).is_none());
        assert!(compare_rgba_downsampled2(&buf, &buf, 8, 8, 8.0).is_none());
        assert!(compare_rgba_downsampled2(&buf, &[0u8; 8], 4, 4, 8.0).is_none());
    }

    #[test]
    fn premultiply_round_trips_straight_alpha() {
        use super::premultiply_rgba_in_place;
        // Straight (240,180,40,150) premultiplies to the CPU oracle values.
        let mut band = vec![240u8, 180, 40, 150];
        premultiply_rgba_in_place(&mut band);
        assert_eq!(band, vec![141u8, 106, 24, 150]);
        // Opaque is identity; transparent zeroes rgb.
        let mut opaque = vec![10u8, 20, 30, 255];
        premultiply_rgba_in_place(&mut opaque);
        assert_eq!(opaque, vec![10u8, 20, 30, 255]);
        let mut clear = vec![255u8, 255, 255, 0];
        premultiply_rgba_in_place(&mut clear);
        assert_eq!(clear, vec![0u8, 0, 0, 0]);
    }

    #[test]
    fn diff_image_amplifies_and_clamps() {
        let a = vec![100u8, 100, 100, 255, 50, 50, 50, 255];
        let b = vec![110u8, 100, 100, 255, 50, 50, 50, 255];
        let out = diff_image_rgba(&a, &b, 4.0).expect("same shape diffs");
        assert_eq!(out, vec![40u8, 0, 0, 255, 0, 0, 0, 0]);
        // Clamp: 200 * 4 saturates instead of wrapping.
        let hot = diff_image_rgba(&[0u8, 0, 0, 255], &[200u8, 0, 0, 255], 4.0)
            .expect("same shape diffs");
        assert_eq!(hot[0], 255);
    }
}
