use gpui::*;

/// SVG Path parser to GPUI Path
///
/// Supports: M, L, H, V, C, Z (absolute) and m, l, h, v, c, z (relative)
pub fn parse_svg_path(d: &str, bounds: Bounds<Pixels>) -> Option<Path<Pixels>> {
    let mut builder = PathBuilder::fill();
    let origin = bounds.origin;

    // Tokenize: insert spaces around command letters, then split
    let mut spaced = String::with_capacity(d.len() * 2);
    for ch in d.chars() {
        if ch.is_ascii_alphabetic() {
            spaced.push(' ');
            spaced.push(ch);
            spaced.push(' ');
        } else if ch == ',' {
            spaced.push(' ');
        } else if ch == '-'
            && !spaced.ends_with(' ')
            && !spaced.ends_with('e')
            && !spaced.ends_with('E')
        {
            // Negative number as implicit separator (e.g. "10-5" → "10 -5")
            spaced.push(' ');
            spaced.push(ch);
        } else {
            spaced.push(ch);
        }
    }

    let tokens: Vec<&str> = spaced.split_whitespace().collect();
    let mut idx = 0;
    let mut cx = 0.0f32;
    let mut cy = 0.0f32;

    while idx < tokens.len() {
        match tokens[idx] {
            "M" => {
                if let Some((x, y)) = parse_xy(&tokens, idx + 1) {
                    cx = x;
                    cy = y;
                    builder.move_to(origin + point(px(x), px(y)));
                    idx += 3;
                } else {
                    idx += 1;
                }
            }
            "m" => {
                if let Some((dx, dy)) = parse_xy(&tokens, idx + 1) {
                    cx += dx;
                    cy += dy;
                    builder.move_to(origin + point(px(cx), px(cy)));
                    idx += 3;
                } else {
                    idx += 1;
                }
            }
            "L" => {
                if let Some((x, y)) = parse_xy(&tokens, idx + 1) {
                    cx = x;
                    cy = y;
                    builder.line_to(origin + point(px(x), px(y)));
                    idx += 3;
                } else {
                    idx += 1;
                }
            }
            "l" => {
                if let Some((dx, dy)) = parse_xy(&tokens, idx + 1) {
                    cx += dx;
                    cy += dy;
                    builder.line_to(origin + point(px(cx), px(cy)));
                    idx += 3;
                } else {
                    idx += 1;
                }
            }
            "H" => {
                if let Some(x) = parse_f32(&tokens, idx + 1) {
                    cx = x;
                    builder.line_to(origin + point(px(cx), px(cy)));
                    idx += 2;
                } else {
                    idx += 1;
                }
            }
            "h" => {
                if let Some(dx) = parse_f32(&tokens, idx + 1) {
                    cx += dx;
                    builder.line_to(origin + point(px(cx), px(cy)));
                    idx += 2;
                } else {
                    idx += 1;
                }
            }
            "V" => {
                if let Some(y) = parse_f32(&tokens, idx + 1) {
                    cy = y;
                    builder.line_to(origin + point(px(cx), px(cy)));
                    idx += 2;
                } else {
                    idx += 1;
                }
            }
            "v" => {
                if let Some(dy) = parse_f32(&tokens, idx + 1) {
                    cy += dy;
                    builder.line_to(origin + point(px(cx), px(cy)));
                    idx += 2;
                } else {
                    idx += 1;
                }
            }
            "C" => {
                // Cubic bezier: C x1 y1 x2 y2 x y
                // Approximate with line segments
                if idx + 6 < tokens.len() {
                    if let (Some(x1), Some(y1), Some(x2), Some(y2), Some(x), Some(y)) = (
                        parse_f32(&tokens, idx + 1),
                        parse_f32(&tokens, idx + 2),
                        parse_f32(&tokens, idx + 3),
                        parse_f32(&tokens, idx + 4),
                        parse_f32(&tokens, idx + 5),
                        parse_f32(&tokens, idx + 6),
                    ) {
                        // Subdivide cubic bezier into line segments
                        let steps = 16;
                        for i in 1..=steps {
                            let t = i as f32 / steps as f32;
                            let u = 1.0 - t;
                            let px = u * u * u * cx
                                + 3.0 * u * u * t * x1
                                + 3.0 * u * t * t * x2
                                + t * t * t * x;
                            let py = u * u * u * cy
                                + 3.0 * u * u * t * y1
                                + 3.0 * u * t * t * y2
                                + t * t * t * y;
                            builder.line_to(origin + point(gpui::px(px), gpui::px(py)));
                        }
                        cx = x;
                        cy = y;
                        idx += 7;
                    } else {
                        idx += 1;
                    }
                } else {
                    idx += 1;
                }
            }
            "c" => {
                // Relative cubic bezier
                if idx + 6 < tokens.len() {
                    if let (Some(dx1), Some(dy1), Some(dx2), Some(dy2), Some(dx), Some(dy)) = (
                        parse_f32(&tokens, idx + 1),
                        parse_f32(&tokens, idx + 2),
                        parse_f32(&tokens, idx + 3),
                        parse_f32(&tokens, idx + 4),
                        parse_f32(&tokens, idx + 5),
                        parse_f32(&tokens, idx + 6),
                    ) {
                        let x1 = cx + dx1;
                        let y1 = cy + dy1;
                        let x2 = cx + dx2;
                        let y2 = cy + dy2;
                        let x = cx + dx;
                        let y = cy + dy;
                        let steps = 16;
                        for i in 1..=steps {
                            let t = i as f32 / steps as f32;
                            let u = 1.0 - t;
                            let px = u * u * u * cx
                                + 3.0 * u * u * t * x1
                                + 3.0 * u * t * t * x2
                                + t * t * t * x;
                            let py = u * u * u * cy
                                + 3.0 * u * u * t * y1
                                + 3.0 * u * t * t * y2
                                + t * t * t * y;
                            builder.line_to(origin + point(gpui::px(px), gpui::px(py)));
                        }
                        cx = x;
                        cy = y;
                        idx += 7;
                    } else {
                        idx += 1;
                    }
                } else {
                    idx += 1;
                }
            }
            "Z" | "z" => {
                builder.close();
                idx += 1;
            }
            _ => idx += 1,
        }
    }

    builder.build().ok()
}

/// Convert a d3rs Path to a GPUI Path, applying an offset and scale.
///
/// This is the preferred way to render d3rs-computed geometry in the showcase.
/// It handles all path commands (arcs, curves, etc.) by linearizing them.
pub fn d3rs_path_to_gpui(
    path: &d3rs::shape::path::Path,
    bounds: Bounds<Pixels>,
    offset_x: f32,
    offset_y: f32,
    scale_x: f32,
    scale_y: f32,
) -> Option<Path<Pixels>> {
    use d3rs::shape::path::PathCommand;

    let mut builder = PathBuilder::fill();
    let origin = bounds.origin;
    let mut cx = 0.0_f32;
    let mut cy = 0.0_f32;

    let tx = |x: f64| -> f32 { x as f32 * scale_x + offset_x };
    let ty = |y: f64| -> f32 { y as f32 * scale_y + offset_y };

    for cmd in path.commands() {
        match cmd {
            PathCommand::MoveTo { x, y } => {
                cx = tx(*x);
                cy = ty(*y);
                builder.move_to(origin + point(px(cx), px(cy)));
            }
            PathCommand::LineTo { x, y } => {
                cx = tx(*x);
                cy = ty(*y);
                builder.line_to(origin + point(px(cx), px(cy)));
            }
            PathCommand::HorizontalLineTo { x } => {
                cx = tx(*x);
                builder.line_to(origin + point(px(cx), px(cy)));
            }
            PathCommand::VerticalLineTo { y } => {
                cy = ty(*y);
                builder.line_to(origin + point(px(cx), px(cy)));
            }
            PathCommand::CubicCurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                let (ax1, ay1) = (tx(*x1), ty(*y1));
                let (ax2, ay2) = (tx(*x2), ty(*y2));
                let (ax, ay) = (tx(*x), ty(*y));
                for i in 1..=16 {
                    let t = i as f32 / 16.0;
                    let u = 1.0 - t;
                    let px_val = u * u * u * cx
                        + 3.0 * u * u * t * ax1
                        + 3.0 * u * t * t * ax2
                        + t * t * t * ax;
                    let py_val = u * u * u * cy
                        + 3.0 * u * u * t * ay1
                        + 3.0 * u * t * t * ay2
                        + t * t * t * ay;
                    builder.line_to(origin + point(px(px_val), px(py_val)));
                }
                cx = ax;
                cy = ay;
            }
            PathCommand::QuadraticCurveTo { x1, y1, x, y } => {
                let (ax1, ay1) = (tx(*x1), ty(*y1));
                let (ax, ay) = (tx(*x), ty(*y));
                for i in 1..=12 {
                    let t = i as f32 / 12.0;
                    let u = 1.0 - t;
                    let px_val = u * u * cx + 2.0 * u * t * ax1 + t * t * ax;
                    let py_val = u * u * cy + 2.0 * u * t * ay1 + t * t * ay;
                    builder.line_to(origin + point(px(px_val), px(py_val)));
                }
                cx = ax;
                cy = ay;
            }
            PathCommand::Arc {
                x: acx,
                y: acy,
                radius,
                start_angle,
                end_angle,
                anticlockwise,
            } => {
                let center_x = tx(*acx);
                let center_y = ty(*acy);
                let r = *radius as f32 * scale_x; // assumes uniform scale for arcs
                let steps = 32;
                let (sa, ea) = if *anticlockwise && end_angle < start_angle {
                    (*start_angle as f32, *end_angle as f32)
                } else if *anticlockwise {
                    (
                        *start_angle as f32,
                        *end_angle as f32 - std::f32::consts::TAU,
                    )
                } else {
                    (*start_angle as f32, *end_angle as f32)
                };
                for i in 0..=steps {
                    let t = i as f32 / steps as f32;
                    let angle = sa + (ea - sa) * t;
                    let px_val = center_x + r * angle.cos();
                    let py_val = center_y + r * angle.sin();
                    if i == 0 {
                        // If this is the first point and we haven't moved yet, move_to
                        // otherwise line_to to connect from the current position
                        builder.line_to(origin + point(px(px_val), px(py_val)));
                    } else {
                        builder.line_to(origin + point(px(px_val), px(py_val)));
                    }
                }
                cx = center_x + r * ea.cos();
                cy = center_y + r * ea.sin();
            }
            PathCommand::ClosePath => {
                builder.close();
            }
            _ => {} // EllipticalArc etc. — skip for now
        }
    }

    builder.build().ok()
}

/// Convenience: convert d3rs Path to GPUI Path with just an offset (no scale).
pub fn d3rs_path_to_gpui_simple(
    path: &d3rs::shape::path::Path,
    bounds: Bounds<Pixels>,
    offset_x: f32,
    offset_y: f32,
) -> Option<Path<Pixels>> {
    d3rs_path_to_gpui(path, bounds, offset_x, offset_y, 1.0, 1.0)
}

fn parse_xy(tokens: &[&str], start: usize) -> Option<(f32, f32)> {
    if start + 1 < tokens.len()
        && let (Ok(x), Ok(y)) = (
            tokens[start].parse::<f32>(),
            tokens[start + 1].parse::<f32>(),
        )
        && x.is_finite()
        && y.is_finite()
    {
        return Some((x, y));
    }
    None
}

fn parse_f32(tokens: &[&str], idx: usize) -> Option<f32> {
    if idx < tokens.len()
        && let Ok(v) = tokens[idx].parse::<f32>()
        && v.is_finite()
    {
        return Some(v);
    }
    None
}
