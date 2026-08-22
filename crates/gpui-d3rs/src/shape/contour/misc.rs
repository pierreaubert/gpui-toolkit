use crate::color::D3Color;
use crate::shape::contour_smoothing::StrokePoint;
use gpui::*;

/// Split screen-space stroke points at discontinuities. Both the legacy GPUI
/// painter and Vello scene builder use this so contour topology cannot drift
/// between raster backends.
pub(crate) fn split_stroke_points(
    points: &[StrokePoint],
    x_jump_threshold: f32,
    y_jump_threshold: f32,
) -> Vec<Vec<StrokePoint>> {
    let mut segments = Vec::new();
    let mut current: Vec<StrokePoint> = Vec::new();
    let mut distances: Vec<_> = points
        .windows(2)
        .map(|pair| (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y))
        .filter(|distance| distance.is_finite() && *distance > 0.0)
        .collect();
    distances.sort_by(|left, right| left.total_cmp(right));
    let typical_distance = distances.get(distances.len() / 2).copied().unwrap_or(0.0);
    let outlier_distance = typical_distance * 4.0;

    for point in points {
        if let Some(previous) = current.last()
            && (((point.x - previous.x).abs() > x_jump_threshold
                || (point.y - previous.y).abs() > y_jump_threshold)
                && (point.x - previous.x).hypot(point.y - previous.y) > outlier_distance)
        {
            if current.len() >= 2 {
                segments.push(current);
            }
            current = Vec::new();
        }
        current.push(*point);
    }

    if current.len() >= 2 {
        segments.push(current);
    }
    segments
}

pub(super) fn split_stroke_segments(
    points: &[Point<Pixels>],
    x_jump_threshold: f32,
    y_jump_threshold: f32,
) -> Vec<Vec<Point<Pixels>>> {
    let points: Vec<_> = points
        .iter()
        .map(|point| StrokePoint::new(point.x.into(), point.y.into()))
        .collect();
    split_stroke_points(&points, x_jump_threshold, y_jump_threshold)
        .into_iter()
        .map(|segment| {
            segment
                .into_iter()
                .map(|point| gpui::point(px(point.x), px(point.y)))
                .collect()
        })
        .collect()
}

/// Create a viridis-like color scale for contours
pub fn viridis_color_scale() -> impl Fn(f64) -> D3Color + Send + Sync {
    move |t: f64| {
        let t = t.clamp(0.0, 1.0);
        // Simplified viridis approximation
        let colors = [
            D3Color::from_hex(0x440154), // Dark purple
            D3Color::from_hex(0x482878), // Purple
            D3Color::from_hex(0x3e4a89), // Blue-purple
            D3Color::from_hex(0x31688e), // Blue
            D3Color::from_hex(0x26838f), // Teal
            D3Color::from_hex(0x1f9e89), // Green-teal
            D3Color::from_hex(0x35b779), // Green
            D3Color::from_hex(0x6ece58), // Light green
            D3Color::from_hex(0xb5de2b), // Yellow-green
            D3Color::from_hex(0xfde725), // Yellow
        ];

        let idx = (t * (colors.len() - 1) as f64) as usize;
        let idx = idx.min(colors.len() - 2);
        let local_t = (t * (colors.len() - 1) as f64) - idx as f64;

        colors[idx].interpolate(&colors[idx + 1], local_t as f32)
    }
}

/// Create a heat color scale (blue -> white -> red)
pub fn heat_color_scale() -> impl Fn(f64) -> D3Color + Send + Sync {
    move |t: f64| {
        let t = t.clamp(0.0, 1.0);
        if t < 0.5 {
            // Blue to white
            let local_t = t * 2.0;
            D3Color::from_hex(0x0571b0).interpolate(&D3Color::from_hex(0xf7f7f7), local_t as f32)
        } else {
            // White to red
            let local_t = (t - 0.5) * 2.0;
            D3Color::from_hex(0xf7f7f7).interpolate(&D3Color::from_hex(0xca0020), local_t as f32)
        }
    }
}

/// Create a turbo color scale (Google's improved rainbow colormap)
///
/// Turbo is designed to be perceptually uniform while covering a wide
/// range of hues. It's an improvement over jet/rainbow colormaps.
/// Reference: <https://ai.googleblog.com/2019/08/turbo-improved-rainbow-colormap-for.html>
pub fn turbo_color_scale() -> impl Fn(f64) -> D3Color + Send + Sync {
    move |t: f64| {
        let t = t.clamp(0.0, 1.0);
        // Turbo colormap keypoints (simplified 16-color approximation)
        let colors = [
            D3Color::from_hex(0x30123b), // Dark blue
            D3Color::from_hex(0x4145ab), // Blue
            D3Color::from_hex(0x4675ed), // Light blue
            D3Color::from_hex(0x39a2fc), // Cyan-blue
            D3Color::from_hex(0x1bd0d5), // Cyan
            D3Color::from_hex(0x24f0a0), // Cyan-green
            D3Color::from_hex(0x5dfc69), // Green
            D3Color::from_hex(0xa4fc3c), // Yellow-green
            D3Color::from_hex(0xd9ec37), // Yellow
            D3Color::from_hex(0xfece2f), // Yellow-orange
            D3Color::from_hex(0xfda629), // Orange
            D3Color::from_hex(0xf67828), // Red-orange
            D3Color::from_hex(0xe74a24), // Red
            D3Color::from_hex(0xcd2220), // Dark red
            D3Color::from_hex(0xa91c1c), // Darker red
            D3Color::from_hex(0x7a0403), // Very dark red
        ];

        let idx = (t * (colors.len() - 1) as f64) as usize;
        let idx = idx.min(colors.len() - 2);
        let local_t = (t * (colors.len() - 1) as f64) - idx as f64;

        colors[idx].interpolate(&colors[idx + 1], local_t as f32)
    }
}

/// Create a magma color scale (perceptually uniform, dark to bright)
pub fn magma_color_scale() -> impl Fn(f64) -> D3Color + Send + Sync {
    move |t: f64| {
        let t = t.clamp(0.0, 1.0);
        let colors = [
            D3Color::from_hex(0x000004), // Black
            D3Color::from_hex(0x1c1044), // Dark purple
            D3Color::from_hex(0x4f127b), // Purple
            D3Color::from_hex(0x812581), // Magenta-purple
            D3Color::from_hex(0xb5367a), // Magenta
            D3Color::from_hex(0xe55964), // Pink-red
            D3Color::from_hex(0xfb8761), // Orange
            D3Color::from_hex(0xfec287), // Light orange
            D3Color::from_hex(0xfcfdbf), // Light yellow
        ];

        let idx = (t * (colors.len() - 1) as f64) as usize;
        let idx = idx.min(colors.len() - 2);
        let local_t = (t * (colors.len() - 1) as f64) - idx as f64;

        colors[idx].interpolate(&colors[idx + 1], local_t as f32)
    }
}

/// Create a plasma color scale (perceptually uniform, purple to yellow)
pub fn plasma_color_scale() -> impl Fn(f64) -> D3Color + Send + Sync {
    move |t: f64| {
        let t = t.clamp(0.0, 1.0);
        let colors = [
            D3Color::from_hex(0x0d0887), // Dark blue
            D3Color::from_hex(0x46039f), // Purple
            D3Color::from_hex(0x7201a8), // Violet
            D3Color::from_hex(0x9c179e), // Magenta
            D3Color::from_hex(0xbd3786), // Pink
            D3Color::from_hex(0xd8576b), // Red-pink
            D3Color::from_hex(0xed7953), // Orange
            D3Color::from_hex(0xfb9f3a), // Yellow-orange
            D3Color::from_hex(0xfdca26), // Yellow
            D3Color::from_hex(0xf0f921), // Bright yellow
        ];

        let idx = (t * (colors.len() - 1) as f64) as usize;
        let idx = idx.min(colors.len() - 2);
        let local_t = (t * (colors.len() - 1) as f64) - idx as f64;

        colors[idx].interpolate(&colors[idx + 1], local_t as f32)
    }
}

/// Create an inferno color scale (perceptually uniform, black to yellow)
pub fn inferno_color_scale() -> impl Fn(f64) -> D3Color + Send + Sync {
    move |t: f64| {
        let t = t.clamp(0.0, 1.0);
        let colors = [
            D3Color::from_hex(0x000004), // Black
            D3Color::from_hex(0x1f0c48), // Dark purple
            D3Color::from_hex(0x550f6d), // Purple
            D3Color::from_hex(0x88226a), // Magenta
            D3Color::from_hex(0xba3655), // Red
            D3Color::from_hex(0xe35933), // Orange-red
            D3Color::from_hex(0xf98c09), // Orange
            D3Color::from_hex(0xf9c932), // Yellow-orange
            D3Color::from_hex(0xfcffa4), // Light yellow
        ];

        let idx = (t * (colors.len() - 1) as f64) as usize;
        let idx = idx.min(colors.len() - 2);
        let local_t = (t * (colors.len() - 1) as f64) - idx as f64;

        colors[idx].interpolate(&colors[idx + 1], local_t as f32)
    }
}
