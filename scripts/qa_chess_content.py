"""Independent chess screenshot checks: parity alone can accept shared tofu glyphs."""
from __future__ import annotations


def chess_content_errors(image) -> list[str]:
    """Require 32 visible pieces and distinct back-rank silhouettes at any scale.

    Locate the sixteen pale backing discs rather than hardcoding screen
    coordinates. The discs and scene colors are fixed across toolkit themes.
    Test the ink inside each disc, excluding its outline; then require the five
    different back-rank types. Both absent ink and identical missing-glyph boxes
    fail even when the native and browser images are pixel-identical.
    """
    from PIL import Image

    rgb = image.convert("RGB")
    width, height = rgb.size
    pixels = rgb.load()
    # Exact interior color from WHITE_PIECE, with tolerance for downsampling.
    pale = {(x, y) for y in range(height) for x in range(width)
            if all(abs(pixels[x, y][c] - (248, 250, 252)[c]) <= 2 for c in range(3))}
    discs = []
    while pale:
        seed = pale.pop()
        component = [seed]
        stack = [seed]
        while stack:
            x, y = stack.pop()
            for point in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)):
                if point in pale:
                    pale.remove(point)
                    stack.append(point)
                    component.append(point)
        xs, ys = zip(*component)
        left, right, top, bottom = min(xs), max(xs), min(ys), max(ys)
        w, h = right - left + 1, bottom - top + 1
        if len(component) >= 150 and 20 <= w <= width / 8 and 0.8 <= w / h <= 1.2:
            discs.append(((left + right) / 2, (top + bottom) / 2, w))
    if discs:
        from statistics import median
        diameter = median(disc[2] for disc in discs)
        # At native 2x scale, a closed outline glyph can itself enclose a
        # pale component large enough to be a candidate. Discs share a size;
        # these inner glyph islands are much smaller and must be excluded.
        discs = [disc for disc in discs if .8 * diameter <= disc[2] <= 1.2 * diameter]
    if len(discs) != 16:
        return [f"expected 16 pale chess discs, found {len(discs)}"]
    discs.sort(key=lambda disc: (disc[1], disc[0]))
    top = sorted(discs[:8])
    second = sorted(discs[8:])
    step = (top[-1][0] - top[0][0]) / 7
    if step <= 0 or any(abs(disc[1] - top[0][1]) > step * .1 for disc in top):
        return ["could not locate the chess starting ranks"]
    radius = step * .29  # Keep patches inside the disc, away from square/edge ink.
    errors = []
    masks = []
    for row in (0, 1, 6, 7):
        row_masks = []
        for column in range(8):
            x = top[column][0]
            y = top[column][1] + row * (second[column][1] - top[column][1])
            patch = rgb.crop((round(x-radius), round(y-radius), round(x+radius), round(y+radius)))
            samples = patch.convert("L").resize((12, 12), Image.Resampling.BOX).tobytes()
            mask = [value < 160 if row < 2 else value > 90 for value in samples]
            if sum(mask) < 3:
                errors.append(f"no piece ink at rank {8-row}, file {column+1}")
            row_masks.append(mask)
        if row in (0, 7):
            masks.append(row_masks)
    # Rook, knight, bishop, queen, king must have different shapes, regardless
    # of which rasterizer paints them. A font's .notdef boxes all look alike.
    for army, rank in zip(("black", "white"), masks):
        unique = []
        for mask in rank[:5]:
            if all(sum(a != b for a, b in zip(mask, other)) / len(mask) > .07 for other in unique):
                unique.append(mask)
        if len(unique) < 5:
            errors.append(f"{army} back rank has {len(unique)} distinct glyphs, expected 5 (missing font/tofu?)")
    return errors
