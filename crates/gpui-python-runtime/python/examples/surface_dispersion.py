"""Build a 3D dispersion surface spec."""

from __future__ import annotations

import json
import math

from gpui_toolkit import scene3d as s3


def build_spec() -> dict:
    freqs = [20.0, 40.0, 80.0, 160.0, 315.0, 630.0, 1250.0, 2500.0, 5000.0, 10000.0]
    angles = [-90.0, -60.0, -30.0, 0.0, 30.0, 60.0, 90.0]

    z_grid = []
    for angle in angles:
        row = []
        angle_weight = abs(angle) / 90.0
        for freq in freqs:
            octave = math.log2(freq / 1000.0)
            on_axis_ripple = 2.0 * math.sin(octave * 2.4)
            off_axis_rolloff = -9.0 * angle_weight * max(0.0, math.log10(freq / 1000.0))
            row.append(on_axis_ripple + off_axis_rolloff)
        z_grid.append(row)

    return s3.surface(
        "dispersion",
        z=z_grid,
        x=freqs,
        y=angles,
        colormap="turbo",
        x_log=True,
        wireframe=False,
        z_range=(-12.0, 4.0),
        labels={"x": "Frequency (Hz)", "y": "Angle (deg)", "z": "Level (dB)"},
        camera=s3.orbit(distance=3.8, azimuth=58.0, elevation=28.0),
        interactions=["orbit", "pan", "zoom", "reset"],
        width=720,
        height=460,
    ).to_spec()


if __name__ == "__main__":
    print(json.dumps(build_spec(), indent=2))
