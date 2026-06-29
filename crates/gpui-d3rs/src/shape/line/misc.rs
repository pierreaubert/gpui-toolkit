/// Clip a line segment to the unit rectangle [0,1] x [0,1] using Cohen-Sutherland algorithm
/// Returns Some((x0, y0, x1, y1)) if the clipped segment is visible, None if entirely outside
pub(super) fn clip_line_segment(
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
) -> Option<(f32, f32, f32, f32)> {
    const INSIDE: u8 = 0;
    const LEFT: u8 = 1;
    const RIGHT: u8 = 2;
    const BOTTOM: u8 = 4;
    const TOP: u8 = 8;

    fn compute_outcode(x: f32, y: f32) -> u8 {
        let mut code = INSIDE;
        if x < 0.0 {
            code |= LEFT;
        } else if x > 1.0 {
            code |= RIGHT;
        }
        if y < 0.0 {
            code |= TOP;
        } else if y > 1.0 {
            code |= BOTTOM;
        }
        code
    }

    let mut x0 = x0;
    let mut y0 = y0;
    let mut x1 = x1;
    let mut y1 = y1;
    let mut outcode0 = compute_outcode(x0, y0);
    let mut outcode1 = compute_outcode(x1, y1);

    loop {
        if (outcode0 | outcode1) == 0 {
            // Both points inside
            return Some((x0, y0, x1, y1));
        } else if (outcode0 & outcode1) != 0 {
            // Both points share an outside zone
            return None;
        } else {
            // Calculate intersection
            let outcode_out = if outcode0 != 0 { outcode0 } else { outcode1 };
            let (x, y);

            if (outcode_out & TOP) != 0 {
                x = x0 + (x1 - x0) * (0.0 - y0) / (y1 - y0);
                y = 0.0;
            } else if (outcode_out & BOTTOM) != 0 {
                x = x0 + (x1 - x0) * (1.0 - y0) / (y1 - y0);
                y = 1.0;
            } else if (outcode_out & RIGHT) != 0 {
                y = y0 + (y1 - y0) * (1.0 - x0) / (x1 - x0);
                x = 1.0;
            } else {
                // LEFT
                y = y0 + (y1 - y0) * (0.0 - x0) / (x1 - x0);
                x = 0.0;
            }

            if outcode_out == outcode0 {
                x0 = x;
                y0 = y;
                outcode0 = compute_outcode(x0, y0);
            } else {
                x1 = x;
                y1 = y;
                outcode1 = compute_outcode(x1, y1);
            }
        }
    }
}
