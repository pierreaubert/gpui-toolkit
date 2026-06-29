use super::super::flare_data::HierarchyNode;
use super::misc::find_best_row;

/// Slice tiling - horizontal strips
pub(super) fn tile_slice(
    children: &[(&HierarchyNode, f64)],
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    total: f64,
) -> Vec<(f64, f64, f64, f64)> {
    let height = y1 - y0;
    let mut rects = Vec::new();
    let mut y = y0;

    for (_, value) in children {
        let h = (value / total) * height;
        rects.push((x0, y, x1, y + h));
        y += h;
    }

    rects
}

/// Dice tiling - vertical strips
pub(super) fn tile_dice(
    children: &[(&HierarchyNode, f64)],
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    total: f64,
) -> Vec<(f64, f64, f64, f64)> {
    let width = x1 - x0;
    let mut rects = Vec::new();
    let mut x = x0;

    for (_, value) in children {
        let w = (value / total) * width;
        rects.push((x, y0, x + w, y1));
        x += w;
    }

    rects
}

/// Slice-Dice tiling - alternates between slice and dice based on depth
pub(super) fn tile_slice_dice(
    children: &[(&HierarchyNode, f64)],
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    total: f64,
    depth: usize,
) -> Vec<(f64, f64, f64, f64)> {
    if depth.is_multiple_of(2) {
        tile_slice(children, x0, y0, x1, y1, total)
    } else {
        tile_dice(children, x0, y0, x1, y1, total)
    }
}

/// Binary tiling - recursively subdivides into two halves
pub(super) fn tile_binary(
    children: &[(&HierarchyNode, f64)],
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    _total: f64,
) -> Vec<(f64, f64, f64, f64)> {
    if children.is_empty() {
        return Vec::new();
    }
    if children.len() == 1 {
        return vec![(x0, y0, x1, y1)];
    }

    // Find the partition point that balances the two halves
    let total: f64 = children.iter().map(|(_, v)| *v).sum();
    let mut cumsum = 0.0;
    let mut split_idx = 0;
    let half = total / 2.0;

    for (i, (_, value)) in children.iter().enumerate() {
        cumsum += *value;
        if cumsum >= half {
            split_idx = i + 1;
            break;
        }
    }

    // Ensure we don't get empty splits
    split_idx = split_idx.max(1).min(children.len() - 1);

    let left: f64 = children[..split_idx].iter().map(|(_, v)| *v).sum();
    let right: f64 = children[split_idx..].iter().map(|(_, v)| *v).sum();
    let left_ratio = left / (left + right);

    let width = x1 - x0;
    let height = y1 - y0;

    let mut rects = Vec::new();

    if width >= height {
        // Split horizontally
        let mid_x = x0 + width * left_ratio;
        let left_rects = tile_binary(&children[..split_idx], x0, y0, mid_x, y1, left);
        let right_rects = tile_binary(&children[split_idx..], mid_x, y0, x1, y1, right);
        rects.extend(left_rects);
        rects.extend(right_rects);
    } else {
        // Split vertically
        let mid_y = y0 + height * left_ratio;
        let top_rects = tile_binary(&children[..split_idx], x0, y0, x1, mid_y, left);
        let bottom_rects = tile_binary(&children[split_idx..], x0, mid_y, x1, y1, right);
        rects.extend(top_rects);
        rects.extend(bottom_rects);
    }

    rects
}

/// Squarify tiling - creates rectangles with aspect ratios close to 1
pub(super) fn tile_squarify(
    children: &[(&HierarchyNode, f64)],
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    _total: f64,
) -> Vec<(f64, f64, f64, f64)> {
    if children.is_empty() {
        return Vec::new();
    }

    let width = x1 - x0;
    let height = y1 - y0;
    let area = width * height;
    let total: f64 = children.iter().map(|(_, v)| *v).sum();

    if total == 0.0 || area == 0.0 {
        return children.iter().map(|_| (x0, y0, x0, y0)).collect();
    }

    // Scale factor to convert values to areas
    let scale = area / total;

    let mut rects = vec![(0.0, 0.0, 0.0, 0.0); children.len()];
    let mut remaining_x0 = x0;
    let mut remaining_y0 = y0;
    let remaining_x1 = x1;
    let remaining_y1 = y1;

    let mut i = 0;
    while i < children.len() {
        let remaining_width = remaining_x1 - remaining_x0;
        let remaining_height = remaining_y1 - remaining_y0;

        if remaining_width <= 0.0 || remaining_height <= 0.0 {
            break;
        }

        // Determine if we're laying out horizontally or vertically
        let horizontal = remaining_width >= remaining_height;
        let side = if horizontal {
            remaining_height
        } else {
            remaining_width
        };

        // Find the best row
        let (row_end, row_sum) =
            find_best_row(&children[i..], side, scale, children.len() - i, horizontal);

        // Layout this row
        let row_length = row_sum * scale / side;

        if horizontal {
            // Layout vertically within a horizontal strip
            let mut y = remaining_y0;
            for j in i..(i + row_end) {
                let h = (children[j].1 * scale) / row_length;
                rects[j] = (remaining_x0, y, remaining_x0 + row_length, y + h);
                y += h;
            }
            remaining_x0 += row_length;
        } else {
            // Layout horizontally within a vertical strip
            let mut x = remaining_x0;
            for j in i..(i + row_end) {
                let w = (children[j].1 * scale) / row_length;
                rects[j] = (x, remaining_y0, x + w, remaining_y0 + row_length);
                x += w;
            }
            remaining_y0 += row_length;
        }

        i += row_end;
    }

    rects
}
