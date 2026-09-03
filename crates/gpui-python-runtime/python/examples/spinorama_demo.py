"""Offline Python version of the Rust ``gpui-px`` spinorama demo.

The Rust viewer loads measurements from spinorama.org.  This example keeps the
same plot story but generates a small deterministic speaker model locally, so
it can be run from a clean Python installation and used as a template for
real measurement data.
"""

from __future__ import annotations

from array import array
import json
import math
import os
from typing import Sequence

from gpui_toolkit import App, data, px, section, ui
from gpui_toolkit import scene3d as s3


FREQUENCIES = tuple(
    20.0 * (20_000.0 / 20.0) ** (index / 63.0) for index in range(64)
)
ANGLES = tuple(-180.0 + 10.0 * index for index in range(37))
_RESOURCES: list[object] = []


def _base_response(frequency: float) -> float:
    """Return a plausible on-axis response in dB for the local speaker model."""

    log_frequency = math.log10(frequency / 1_000.0)
    return (
        86.0
        + 1.8 * math.sin(log_frequency * 3.2)
        + 0.8 * math.sin(log_frequency * 10.0)
        - 1.5 * max(0.0, -log_frequency - 1.0)
    )


def _directivity_response(frequency: float, angle: float) -> float:
    """Approximate the narrowing directivity of a speaker at high frequency."""

    log_frequency = math.log10(frequency / 250.0)
    beam_width = max(0.32, 1.35 - 0.32 * max(0.0, log_frequency))
    angle_radians = math.radians(angle)
    directivity = math.exp(-(angle_radians**2) / (2.0 * beam_width**2))
    return _base_response(frequency) - 38.0 * (1.0 - directivity)


def frequency_response_curves() -> dict[str, list[float]]:
    """Build the five curves shown in the CEA2034-style plot."""

    on_axis = [_base_response(frequency) for frequency in FREQUENCIES]
    listening_window = [
        value - 0.7 + 0.35 * math.sin(math.log10(frequency) * 2.0)
        for frequency, value in zip(FREQUENCIES, on_axis)
    ]
    early_reflections = [
        value - 1.8 - 0.45 * max(0.0, math.log10(frequency / 1_000.0))
        for frequency, value in zip(FREQUENCIES, listening_window)
    ]
    sound_power = [
        value - 3.5 - 0.25 * max(0.0, math.log10(frequency / 1_000.0))
        for frequency, value in zip(FREQUENCIES, listening_window)
    ]
    predicted_in_room = [
        value - 0.4 + 0.2 * math.sin(math.log10(frequency) * 1.7)
        for frequency, value in zip(FREQUENCIES, listening_window)
    ]
    return {
        "on_axis": on_axis,
        "listening_window": listening_window,
        "early_reflections": early_reflections,
        "sound_power": sound_power,
        "predicted_in_room": predicted_in_room,
    }


def directivity_grid(angles: Sequence[float] = ANGLES) -> list[list[float]]:
    """Return a row-major angle x frequency SPL grid."""

    return [
        [_directivity_response(frequency, angle) for frequency in FREQUENCIES]
        for angle in angles
    ]


def _curve_dataset(curves: dict[str, list[float]], resource_id: str) -> data.Dataset:
    styles = {
        "on_axis": ("On-axis", "#f97316"),
        "listening_window": ("Listening window", "#22c55e"),
        "early_reflections": ("Early reflections", "#38bdf8"),
        "sound_power": ("Sound power", "#a78bfa"),
        "predicted_in_room": ("Predicted in-room", "#facc15"),
    }
    frequency: list[float] = []
    level: list[float] = []
    series: list[str] = []
    color: list[str] = []
    for curve_id, values in curves.items():
        label, curve_color = styles[curve_id]
        frequency.extend(FREQUENCIES)
        level.extend(values)
        series.extend([label] * len(values))
        color.extend([curve_color] * len(values))
    resource = data.Dataset.from_mapping(
        {"frequency": frequency, "level": level, "series": series, "color": color},
        id=resource_id,
    )
    _RESOURCES.append(resource)
    return resource


def build_cea2034_chart() -> px.ChartBuilder:
    curves = frequency_response_curves()
    resource = _curve_dataset(curves, "spinorama-cea2034")
    return (
        px.line("cea2034").data(resource).x("frequency").y("level")
        .series("series").color("color").title("CEA2034-style response")
        .x_log().x_label("Frequency (Hz)").y_label("SPL (dB)")
        .x_range(20.0, 20_000.0).y_range(35.0, 95.0)
        .legend_position(px.LegendPosition.BOTTOM)
        .annotation(px.Annotation.x_value("reference-frequency", "1 kHz", 1_000.0).color("#94a3b8"))
    )


def _angle_dataset(angles: Sequence[float], resource_id: str) -> data.Dataset:
    colors = {
        -90.0: "#38bdf8",
        -60.0: "#60a5fa",
        -30.0: "#a78bfa",
        0.0: "#f97316",
        30.0: "#a78bfa",
        60.0: "#60a5fa",
        90.0: "#38bdf8",
    }
    frequency: list[float] = []
    level: list[float] = []
    series: list[str] = []
    color: list[str] = []
    for angle in angles:
        frequency.extend(FREQUENCIES)
        level.extend(_directivity_response(value, angle) for value in FREQUENCIES)
        series.extend([f"{angle:+.0f}°"] * len(FREQUENCIES))
        color.extend([colors.get(angle, "#94a3b8")] * len(FREQUENCIES))
    resource = data.Dataset.from_mapping(
        {"frequency": frequency, "level": level, "series": series, "color": color},
        id=resource_id,
    )
    _RESOURCES.append(resource)
    return resource


def build_horizontal_spl_chart() -> px.ChartBuilder:
    angles = (-90.0, -60.0, -30.0, 0.0, 30.0, 60.0, 90.0)
    resource = _angle_dataset(angles, "spinorama-horizontal")
    return (
        px.line("horizontal-spl").data(resource).x("frequency").y("level")
        .series("series").color("color").title("Horizontal SPL")
        .x_log().x_label("Frequency (Hz)").y_label("SPL (dB)")
        .x_range(20.0, 20_000.0).y_range(35.0, 95.0)
        .legend_position(px.LegendPosition.BOTTOM)
    )


def build_vertical_spl_chart() -> px.ChartBuilder:
    angles = (-90.0, -45.0, 0.0, 45.0, 90.0)
    resource = _angle_dataset(angles, "spinorama-vertical")
    return (
        px.line("vertical-spl").data(resource).x("frequency").y("level")
        .series("series").color("color").title("Vertical SPL")
        .x_log().x_label("Frequency (Hz)").y_label("SPL (dB)")
        .x_range(20.0, 20_000.0).y_range(35.0, 95.0)
        .legend_position(px.LegendPosition.BOTTOM)
    )


def build_contour_chart() -> px.ChartBuilder:
    grid = directivity_grid()
    resource = data.ArrayData.from_buffer(
        array("d", (value for row in grid for value in row)),
        shape=(len(ANGLES), len(FREQUENCIES)),
        dtype="f64",
        id="spinorama-directivity-grid",
    )
    _RESOURCES.append(resource)
    return (
        px.contour("directivity-contour").data(resource)
        .title("Horizontal directivity contour")
        .color_scale(px.ColorScale.VIRIDIS)
        .thresholds([40.0, 50.0, 60.0, 70.0, 80.0, 90.0])
        .aspect_ratio(1.45)
    )


def build_surface_spec() -> s3.Surface:
    grid = directivity_grid()
    resource = data.ArrayData.from_buffer(
        array("d", (value for row in grid for value in row)),
        shape=(len(ANGLES), len(FREQUENCIES)),
        dtype="f64",
        id="spinorama-directivity-surface",
    )
    _RESOURCES.append(resource)
    return s3.surface(
        "directivity-surface",
        z=resource,
        x=FREQUENCIES,
        y=ANGLES,
        colormap="turbo",
        x_log=True,
        z_range=(-40.0, 95.0),
        labels={"x": "Frequency (Hz)", "y": "Angle (deg)", "z": "SPL (dB)"},
        camera=s3.orbit(distance=3.6, azimuth=58.0, elevation=25.0),
        interactions=["orbit", "pan", "zoom", "reset"],
        width=820.0,
        height=520.0,
    )


def _plot_section(title: str, description: str, content: object) -> ui.Node:
    return ui.vstack(
        [
            ui.section_header(title, description),
            ui.card([content], width=900.0),
        ],
        gap=20.0,
    )


def build_app() -> App:
    _RESOURCES.clear()
    surface = build_surface_spec()
    app = App(
        title="Spinorama Viewer (Python)",
        sidebar_title="Python Spinorama",
        sidebar_subtitle="Python declarations, Rust renderers",
        width=1240.0,
        height=820.0,
        sections=[
            section(
                "overview",
                "Overview",
                ui.vstack(
                    [
                        ui.section_header(
                            "Spinorama Viewer",
                            "An offline Python-authored counterpart to the Rust gpui-px demo.",
                        ),
                        ui.wrap(
                            [
                                ui.metric("Plot modes", 5),
                                ui.metric("Frequency samples", len(FREQUENCIES)),
                                ui.metric("Angle samples", len(ANGLES)),
                                ui.metric("Network requests", 0),
                            ],
                            gap=16.0,
                        ),
                        ui.card(
                            [
                                ui.heading("Plot sections", level=2),
                                ui.text(
                                    "The curves are deterministic stand-ins for measurement data. "
                                    "Replace frequency_response_curves() and directivity_grid() "
                                    "with values from a measurement pipeline.",
                                ),
                                ui.tabs(
                                    ["CEA2034", "Horizontal SPL", "Vertical SPL", "Contour", "Surface 3D"],
                                    active=0,
                                ),
                            ],
                            width=900.0,
                        ),
                    ],
                    gap=20.0,
                ),
            ),
            section(
                "cea2034",
                "CEA2034",
                _plot_section(
                    "CEA2034-style response",
                    "Multiple named response curves with a logarithmic frequency axis.",
                    build_cea2034_chart(),
                ),
            ),
            section(
                "horizontal-spl",
                "Horizontal SPL",
                _plot_section(
                    "Horizontal SPL",
                    "Off-axis response at several horizontal listening angles.",
                    build_horizontal_spl_chart(),
                ),
            ),
            section(
                "vertical-spl",
                "Vertical SPL",
                _plot_section(
                    "Vertical SPL",
                    "Off-axis response at several vertical listening angles.",
                    build_vertical_spl_chart(),
                ),
            ),
            section(
                "contour",
                "Contour",
                _plot_section(
                    "Directivity contour",
                    "The same retained grid rendered as a 2D color contour.",
                    build_contour_chart(),
                ),
            ),
            section(
                "surface-3d",
                "Surface 3D",
                ui.vstack(
                    [
                        ui.section_header(
                            "Directivity surface",
                            "The contour data rendered through the retained Scene3D API.",
                        ),
                        ui.scene3d(surface),
                        ui.table(
                            ["field", "value"],
                            [
                                ["grid", f"{len(ANGLES)} x {len(FREQUENCIES)}"],
                                ["colormap", "turbo"],
                                ["camera", "orbit"],
                                ["interactions", "orbit, pan, zoom, reset"],
                            ],
                        ),
                    ],
                    gap=20.0,
                ),
            ),
        ],
    )
    app.resources = tuple(_RESOURCES)
    return app


def main() -> None:
    app = build_app()
    if (
        os.environ.get("GPUI_TOOLKIT_DUMP_IR") == "1"
        or os.environ.get("GPUI_TOOLKIT_SESSION") == "1"
        or bool(os.environ.get("GPUI_TOOLKIT_HOST"))
    ):
        app.run()
    else:
        print(json.dumps(app.to_spec(), indent=2))


if __name__ == "__main__":
    main()
