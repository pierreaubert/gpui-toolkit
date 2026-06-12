"""Build a 3D line-strip scene spec with orbit controls."""

from __future__ import annotations

import json
import math

from gpui_toolkit import scene3d as s3


def build_spec() -> dict:
    turns = 2.5
    samples = 80
    helix = []
    for index in range(samples):
        t = index / (samples - 1)
        angle = t * turns * math.tau
        radius = 0.7 + 0.2 * math.sin(t * math.tau)
        helix.append(
            (
                radius * math.cos(angle),
                (t - 0.5) * 1.8,
                radius * math.sin(angle),
            )
        )

    strips = [
        s3.line_strip("helix", helix, color="#7dd3fc", width=2.5),
        s3.line_strip("x_axis", [(-1.2, 0.0, 0.0), (1.2, 0.0, 0.0)], color="#ef4444"),
        s3.line_strip("y_axis", [(0.0, -1.0, 0.0), (0.0, 1.0, 0.0)], color="#22c55e"),
        s3.line_strip("z_axis", [(0.0, 0.0, -1.2), (0.0, 0.0, 1.2)], color="#3b82f6"),
    ]

    return s3.lines(
        "orbit-lines",
        strips,
        background="#0b1020",
        camera=s3.orbit(distance=4.2, azimuth=42.0, elevation=24.0),
        interactions=["orbit", "pan", "zoom", "reset"],
        width=640,
        height=420,
    ).to_spec()


if __name__ == "__main__":
    print(json.dumps(build_spec(), indent=2))
