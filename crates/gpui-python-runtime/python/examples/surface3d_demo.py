"""Python counterpart to the Rust ``surface3d_demo`` example.

The three surfaces intentionally match the Rust demo's modes: a sinc-like
function, a speaker-style dispersion surface, and a saddle.  The data is
retained in Python as a serializable Scene3D spec; the native host owns the
interactive camera and GPU rendering.
"""

from __future__ import annotations

import json
import math
import os

from gpui_toolkit import App, section, ui
from gpui_toolkit import scene3d as s3


def _axis(start: float, stop: float, count: int) -> list[float]:
    return [start + (stop - start) * index / (count - 1) for index in range(count)]


def _sinc_surface() -> s3.Surface:
    axis = _axis(-3.0 * math.pi, 3.0 * math.pi, 48)
    values: list[list[float]] = []
    for x in axis:
        row = []
        for y in axis:
            radius = math.hypot(x, y)
            sinc = 1.0 if radius < 0.01 else math.sin(radius) / radius
            row.append(sinc * math.cos(x * 0.3) * math.cos(y * 0.3))
        values.append(row)
    return s3.surface(
        "sinc",
        z=values,
        x=axis,
        y=axis,
        colormap="viridis",
        labels={"x": "X", "y": "Y", "z": "Z"},
        camera=s3.orbit(distance=4.0, azimuth=45.0, elevation=30.0),
        interactions=["orbit", "pan", "zoom", "reset"],
        width=760.0,
        height=500.0,
    )


def _spinorama_surface() -> s3.Surface:
    frequencies = [
        20.0 * (20_000.0 / 20.0) ** (index / 39.0) for index in range(40)
    ]
    angles = _axis(-180.0, 180.0, 25)
    values: list[list[float]] = []
    for angle in angles:
        row = []
        for frequency in frequencies:
            log_frequency = math.log10(frequency)
            angle_radians = math.radians(angle)
            beaming = max(0.0, log_frequency - 2.0) * 0.5
            width = max(0.3, 1.5 - beaming)
            directivity = math.exp(-(angle_radians**2) / (2.0 * width**2))
            base = 85.0 + 2.0 * math.sin(log_frequency * 2.0)
            row.append(base * directivity - 40.0 * (1.0 - directivity))
        values.append(row)
    return s3.surface(
        "spinorama",
        z=values,
        x=frequencies,
        y=angles,
        colormap="turbo",
        x_log=True,
        z_range=(-40.0, 95.0),
        labels={"x": "Frequency (Hz)", "y": "Angle (deg)", "z": "SPL (dB)"},
        camera=s3.orbit(distance=3.5, azimuth=60.0, elevation=25.0),
        interactions=["orbit", "pan", "zoom", "reset"],
        width=760.0,
        height=500.0,
    )


def _saddle_surface() -> s3.Surface:
    axis = _axis(-2.0, 2.0, 42)
    values = [[x * x - y * y for y in axis] for x in axis]
    return s3.surface(
        "saddle",
        z=values,
        x=axis,
        y=axis,
        colormap="coolwarm",
        wireframe=True,
        z_range=(-4.0, 4.0),
        labels={"x": "X", "y": "Y", "z": "Z = X² - Y²"},
        camera=s3.orbit(distance=4.5, azimuth=35.0, elevation=25.0),
        interactions=["orbit", "pan", "zoom", "reset"],
        width=760.0,
        height=500.0,
    )


def build_surface(name: str) -> s3.Surface:
    surfaces = {
        "sinc": _sinc_surface,
        "spinorama": _spinorama_surface,
        "saddle": _saddle_surface,
    }
    try:
        return surfaces[name]()
    except KeyError as error:
        choices = ", ".join(surfaces)
        raise ValueError(f"unknown surface {name!r}; choose one of {choices}") from error


def build_spec(name: str = "sinc") -> dict[str, object]:
    """Build one JSON-ready surface spec for scripts and tests."""

    return build_surface(name).to_spec()


def build_app() -> App:
    return App(
        title="3D Surface Demo (Python)",
        sidebar_title="Python Surface 3D",
        sidebar_subtitle="Retained Scene3D examples",
        width=1180.0,
        height=820.0,
        sections=[
            section(
                "overview",
                "Overview",
                ui.vstack(
                    [
                        ui.section_header(
                            "Interactive 3D Surface Demo",
                            "The same three data modes as the Rust surface3d_demo.",
                        ),
                        ui.wrap(
                            [
                                ui.metric("Surface modes", 3),
                                ui.metric("Interactions", "orbit / pan / zoom"),
                                ui.metric("GPU objects exposed to Python", 0),
                            ],
                            gap=16.0,
                        ),
                        ui.text(
                            "Drag to orbit, use the middle button to pan, scroll to zoom, "
                            "and double-click to reset the camera.",
                        ),
                    ],
                    gap=20.0,
                ),
            ),
            section(
                "sinc",
                "Sinc",
                ui.vstack(
                    [
                        ui.section_header("Sinc surface", "A radial sinc function with a smooth viridis colormap."),
                        ui.scene3d(build_surface("sinc")),
                    ],
                    gap=20.0,
                ),
            ),
            section(
                "spinorama",
                "Spinorama",
                ui.vstack(
                    [
                        ui.section_header(
                            "Spinorama-style dispersion",
                            "Log-frequency directivity narrowing at higher frequencies.",
                        ),
                        ui.scene3d(build_surface("spinorama")),
                    ],
                    gap=20.0,
                ),
            ),
            section(
                "saddle",
                "Saddle",
                ui.vstack(
                    [
                        ui.section_header("Saddle surface", "A wireframe hyperbolic paraboloid."),
                        ui.scene3d(build_surface("saddle")),
                    ],
                    gap=20.0,
                ),
            ),
        ],
    )


def main() -> None:
    app = build_app()
    if (
        os.environ.get("GPUI_TOOLKIT_DUMP_IR") == "1"
        or os.environ.get("GPUI_TOOLKIT_SESSION") == "1"
        or bool(os.environ.get("GPUI_TOOLKIT_HOST"))
    ):
        app.run()
    else:
        print(json.dumps({name: build_spec(name) for name in ("sinc", "spinorama", "saddle")}, indent=2))


if __name__ == "__main__":
    main()
