use gpui::*;

pub(super) const PREVIEW_WIDTH: usize = 320;

pub(super) const PREVIEW_HEIGHT: usize = 240;

pub(super) fn tab_button(
    id: &'static str,
    label: &'static str,
    active: bool,
    theme: &gpui_ui_kit::theme::Theme,
) -> Stateful<Div> {
    let mut btn = div()
        .id(id)
        .px_4()
        .py_2()
        .rounded_md()
        .cursor_pointer()
        .text_sm()
        .font_weight(FontWeight::MEDIUM);

    if active {
        btn = btn.bg(theme.accent).text_color(rgba(0xffffffff));
    } else {
        let hover_bg = theme.surface_hover;
        btn = btn
            .text_color(theme.text_secondary)
            .hover(move |s| s.bg(hover_bg));
    }

    btn.child(label)
}

#[allow(
    clippy::too_many_arguments,
    reason = "camera preview painter takes pixel data, overlay state, colors, and target window"
)]
pub(super) fn paint_camera_preview(
    bounds: Bounds<Pixels>,
    rgb: &[u8],
    pw: usize,
    ph: usize,
    qr_bounds: Option<[(f32, f32); 4]>,
    accent: Rgba,
    success: Rgba,
    has_decode: bool,
    window: &mut Window,
) {
    let bw: f32 = bounds.size.width.into();
    let bh: f32 = bounds.size.height.into();
    let ox: f32 = bounds.origin.x.into();
    let oy: f32 = bounds.origin.y.into();

    let scale_x = bw / pw as f32;
    let scale_y = bh / ph as f32;

    // Paint each pixel as a scaled quad
    // For performance at 320x240, each source pixel maps to a ~2x2 block
    for y in 0..ph {
        for x in 0..pw {
            let idx = (y * pw + x) * 3;
            if idx + 2 >= rgb.len() {
                break;
            }

            let r = rgb[idx] as f32 / 255.0;
            let g = rgb[idx + 1] as f32 / 255.0;
            let b = rgb[idx + 2] as f32 / 255.0;
            let color = Rgba { r, g, b, a: 1.0 };

            let px_x = ox + x as f32 * scale_x;
            let px_y = oy + y as f32 * scale_y;

            window.paint_quad(PaintQuad {
                bounds: Bounds {
                    origin: point(px(px_x), px(px_y)),
                    size: size(px(scale_x.ceil()), px(scale_y.ceil())),
                },
                corner_radii: Corners::default(),
                background: color.into(),
                border_widths: Edges::default(),
                border_color: color.into(),
                border_style: BorderStyle::default(),
            });
        }
    }

    // Draw QR bounding box overlay
    if let Some(corners) = qr_bounds {
        let highlight = if has_decode { success } else { accent };
        let line_w = 3.0_f32;

        // Draw lines between consecutive corners
        for i in 0..4 {
            let (x1, y1) = corners[i];
            let (x2, y2) = corners[(i + 1) % 4];

            let px1 = ox + x1 * bw;
            let py1 = oy + y1 * bh;
            let px2 = ox + x2 * bw;
            let py2 = oy + y2 * bh;

            // Approximate line as a thin quad
            let dx = px2 - px1;
            let dy = py2 - py1;
            let len = (dx * dx + dy * dy).sqrt();
            if len < 1.0 {
                continue;
            }

            // Draw as a series of small dots along the line for simplicity
            let steps = (len / 2.0).ceil() as usize;
            for s in 0..=steps {
                let t = s as f32 / steps as f32;
                let lx = px1 + dx * t;
                let ly = py1 + dy * t;
                window.paint_quad(PaintQuad {
                    bounds: Bounds {
                        origin: point(px(lx - line_w / 2.0), px(ly - line_w / 2.0)),
                        size: size(px(line_w), px(line_w)),
                    },
                    corner_radii: Corners::default(),
                    background: highlight.into(),
                    border_widths: Edges::default(),
                    border_color: highlight.into(),
                    border_style: BorderStyle::default(),
                });
            }
        }
    }
}

#[cfg(target_os = "macos")]
#[link(name = "AVFoundation", kind = "framework")]
unsafe extern "C" {}

#[cfg(target_os = "macos")]
#[allow(unexpected_cfgs)]
pub(super) fn request_camera_permission_macos() -> bool {
    use block2::RcBlock;
    use objc::runtime::Class;
    use objc::{msg_send, sel, sel_impl};

    unsafe {
        let cls = Class::get("AVCaptureDevice").expect("AVCaptureDevice class not found");

        // AVMediaTypeVideo = "vide"
        let media_type: *mut objc::runtime::Object =
            msg_send![Class::get("NSString").unwrap(), stringWithUTF8String: c"vide".as_ptr()];

        // Check current status first (0=notDetermined, 1=restricted, 2=denied, 3=authorized)
        let status: i64 = msg_send![cls, authorizationStatusForMediaType: media_type];
        if status == 3 {
            return true; // Already authorized
        }
        if status == 1 || status == 2 {
            return false; // Restricted or denied
        }

        // Status is notDetermined — trigger the system permission dialog.
        // BOOL has no Encode impl in block2, so the closure takes the C ABI
        // representation (i8) and we coerce to bool inside.
        let (tx, rx) = std::sync::mpsc::channel();
        let callback = RcBlock::new(move |granted: i8| {
            let _ = tx.send(granted != 0);
        });

        let _: () = msg_send![cls, requestAccessForMediaType: media_type
                                        completionHandler: &*callback];

        rx.recv().unwrap_or(false)
    }
}

pub(super) fn downsample_rgb(
    src: &[u8],
    src_width: usize,
    src_height: usize,
    dst_width: usize,
    dst_height: usize,
) -> Vec<u8> {
    let mut dst = Vec::with_capacity(dst_width * dst_height * 3);
    let x_ratio = src_width as f32 / dst_width as f32;
    let y_ratio = src_height as f32 / dst_height as f32;

    for dst_y in 0..dst_height {
        for dst_x in 0..dst_width {
            let src_x = (dst_x as f32 * x_ratio) as usize;
            let src_y = (dst_y as f32 * y_ratio) as usize;
            let idx = (src_y * src_width + src_x) * 3;

            if idx + 2 < src.len() {
                dst.push(src[idx]);
                dst.push(src[idx + 1]);
                dst.push(src[idx + 2]);
            } else {
                dst.extend_from_slice(&[128, 128, 128]);
            }
        }
    }

    dst
}

pub(super) fn extract_qr_bounds<G>(
    grid: &rqrr::Grid<G>,
    img_width: usize,
    img_height: usize,
) -> [(f32, f32); 4] {
    let bounds = &grid.bounds;
    let normalize = |p: rqrr::Point| {
        (
            p.x as f32 / img_width as f32,
            p.y as f32 / img_height as f32,
        )
    };
    [
        normalize(bounds[0]),
        normalize(bounds[1]),
        normalize(bounds[2]),
        normalize(bounds[3]),
    ]
}
