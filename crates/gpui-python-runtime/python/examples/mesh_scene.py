"""Build a lower-level scene spec with mesh, path, and light nodes."""

from __future__ import annotations

import json

from gpui_toolkit import scene3d as s3


def build_spec() -> dict:
    vertices = [
        (0.0, 0.9, 0.0),
        (-0.8, -0.5, 0.6),
        (0.8, -0.5, 0.6),
        (0.0, -0.5, -0.9),
    ]
    indices = [
        0,
        1,
        2,
        0,
        2,
        3,
        0,
        3,
        1,
        1,
        3,
        2,
    ]

    return s3.scene(
        "model",
        camera=s3.orbit(distance=3.4, azimuth=35.0, elevation=24.0),
        interactions=["orbit", "pan", "zoom", "reset", "hit_test"],
        background="#101820",
        width=720,
        height=460,
        children=[
            s3.mesh(
                "tetrahedron",
                vertices=vertices,
                indices=indices,
                material=s3.material("#88ccff", opacity=0.86),
            ),
            s3.lines(
                "reference-path",
                [
                    s3.line_strip(
                        "orbit-path",
                        [(-1.1, -0.7, 0.0), (-0.3, -0.1, 0.4), (0.4, 0.1, -0.3), (1.1, 0.7, 0.0)],
                        color="#ffffff",
                        width=1.8,
                    )
                ],
            ),
            s3.light("key", direction=(1.0, -2.0, -1.0), intensity=1.25),
        ],
    ).to_spec()


if __name__ == "__main__":
    print(json.dumps(build_spec(), indent=2))
