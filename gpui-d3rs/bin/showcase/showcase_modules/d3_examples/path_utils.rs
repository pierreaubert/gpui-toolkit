use gpui::*;

/// Simple SVG Path parser to GPUI Path
pub fn parse_svg_path(d: &str, bounds: Bounds<Pixels>) -> Option<Path<Pixels>> {
    let mut builder = PathBuilder::fill();
    let clean_d = d
        .replace("M", " M ")
        .replace("L", " L ")
        .replace("Z", " Z ")
        .replace(",", " ");
    let tokens: Vec<&str> = clean_d.split_whitespace().collect();
    let mut idx = 0;

    while idx < tokens.len() {
        match tokens[idx] {
            "M" => {
                if idx + 2 < tokens.len() {
                    if let (Ok(x), Ok(y)) = (
                        tokens[idx + 1].parse::<f32>(),
                        tokens[idx + 2].parse::<f32>(),
                    ) {
                        if x.is_finite() && y.is_finite() {
                            builder.move_to(bounds.origin + point(px(x), px(y)));
                        }
                    }
                    idx += 3;
                } else {
                    idx += 1;
                }
            }
            "L" => {
                if idx + 2 < tokens.len() {
                    if let (Ok(x), Ok(y)) = (
                        tokens[idx + 1].parse::<f32>(),
                        tokens[idx + 2].parse::<f32>(),
                    ) {
                        if x.is_finite() && y.is_finite() {
                            builder.line_to(bounds.origin + point(px(x), px(y)));
                        }
                    }
                    idx += 3;
                } else {
                    idx += 1;
                }
            }
            "Z" => {
                builder.close();
                idx += 1;
            }
            _ => idx += 1,
        }
    }

    builder.build().ok()
}
