use super::treemap_node::TreemapNode;

/// Slice tiling - horizontal strips.
pub(super) fn tile_slice(
    children: &[TreemapNode],
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    total: f64,
) -> Vec<(f64, f64, f64, f64)> {
    let height = y1 - y0;
    let mut rects = Vec::with_capacity(children.len());
    let mut y = y0;

    for child in children {
        let value = child.total_value();
        let h = (value / total) * height;
        rects.push((x0, y, x1, y + h));
        y += h;
    }

    rects
}

/// Dice tiling - vertical strips.
pub(super) fn tile_dice(
    children: &[TreemapNode],
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    total: f64,
) -> Vec<(f64, f64, f64, f64)> {
    let width = x1 - x0;
    let mut rects = Vec::with_capacity(children.len());
    let mut x = x0;

    for child in children {
        let value = child.total_value();
        let w = (value / total) * width;
        rects.push((x, y0, x + w, y1));
        x += w;
    }

    rects
}

/// Slice-Dice tiling - alternates between slice and dice based on depth.
pub(super) fn tile_slice_dice(
    children: &[TreemapNode],
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

/// Binary tiling - recursively subdivides into two halves.
pub(super) fn tile_binary(
    children: &[TreemapNode],
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

    // Find partition point that balances the two halves
    let total: f64 = children.iter().map(|c| c.total_value()).sum();
    let mut cumsum = 0.0;
    let mut split_idx = 0;
    let half = total / 2.0;

    for (i, child) in children.iter().enumerate() {
        cumsum += child.total_value();
        if cumsum >= half {
            split_idx = i + 1;
            break;
        }
    }

    split_idx = split_idx.max(1).min(children.len() - 1);

    let left: f64 = children[..split_idx].iter().map(|c| c.total_value()).sum();
    let right: f64 = children[split_idx..].iter().map(|c| c.total_value()).sum();
    let left_ratio = left / (left + right);

    let width = x1 - x0;
    let height = y1 - y0;

    let mut rects = Vec::with_capacity(children.len());

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

/// Squarify tiling - creates rectangles with aspect ratios close to 1.
pub(super) fn tile_squarify(
    children: &[TreemapNode],
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    total: f64,
) -> Vec<(f64, f64, f64, f64)> {
    // Work with indices to avoid allocating temporary (node, value) vectors.
    // Filter out zero-value children to avoid division by zero.
    let mut indices: Vec<usize> = (0..children.len())
        .filter(|&i| children[i].total_value() > 0.0)
        .collect();
    if indices.is_empty() {
        return vec![(x0, y0, x0, y0); children.len()];
    }

    let width = x1 - x0;
    let height = y1 - y0;

    // Sort by value descending for better packing
    indices.sort_by(|&a, &b| {
        children[b]
            .total_value()
            .total_cmp(&children[a].total_value())
    });

    // Keep output in declaration order. Squarify must sort by value to pack
    // well, but callers associate each returned rectangle with the child at
    // the matching declaration index.
    let mut rects = vec![(x0, y0, x0, y0); children.len()];
    let mut remaining_start = 0;
    let mut x = x0;
    let mut y = y0;
    let mut w = width;
    let mut h = height;
    let mut remaining_value = total;

    while remaining_start < indices.len() {
        let remaining = &indices[remaining_start..];

        // Try to find best row
        let mut best_row_len = 1;
        let mut best_worst_ratio = f64::INFINITY;

        for row_len in 1..=remaining.len() {
            let row = &remaining[..row_len];
            let row_sum: f64 = row.iter().map(|&i| children[i].total_value()).sum();

            let short_side = w.min(h);
            let long_side = w.max(h);

            // Calculate worst aspect ratio in this row
            let mut worst_ratio: f64 = 0.0;
            for &i in row {
                let value = children[i].total_value();
                let rect_area =
                    (value / row_sum) * (short_side * long_side / (w.max(h) / short_side));
                let rect_short = rect_area / long_side;
                let ratio = rect_short.max(long_side / rect_short);
                worst_ratio = worst_ratio.max(ratio);
            }

            if worst_ratio < best_worst_ratio {
                best_worst_ratio = worst_ratio;
                best_row_len = row_len;
            } else {
                break; // Aspect ratios getting worse
            }
        }

        // Layout the best row
        let row = &remaining[..best_row_len];
        let row_sum: f64 = row.iter().map(|&i| children[i].total_value()).sum();

        let use_width = w <= h;
        if use_width {
            // Layout horizontally
            let row_height = (row_sum / remaining_value) * h;
            let mut rx = x;
            for &i in row {
                let value = children[i].total_value();
                let rw = (value / row_sum) * w;
                rects[i] = (rx, y, rx + rw, y + row_height);
                rx += rw;
            }
            y += row_height;
            h -= row_height;
        } else {
            // Layout vertically
            let row_width = (row_sum / remaining_value) * w;
            let mut ry = y;
            for &i in row {
                let value = children[i].total_value();
                let rh = (value / row_sum) * h;
                rects[i] = (x, ry, x + row_width, ry + rh);
                ry += rh;
            }
            x += row_width;
            w -= row_width;
        }

        remaining_value -= row_sum;
        remaining_start += best_row_len;
    }

    rects
}
