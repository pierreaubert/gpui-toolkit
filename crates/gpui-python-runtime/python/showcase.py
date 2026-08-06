"""Python-authored GPUI runtime showcase."""

from __future__ import annotations

import math
import time

from gpui_toolkit import App, Event, SessionContext, charts, scene3d as s3, section, ui


class RuntimeShowcase(App):
    """A small live-session path exercised by the bundled native showcase."""

    def on_action(self, event: Event, context: SessionContext) -> None:
        if event.action == "set_simulation_step":
            context.acknowledge(event)
            context.patch([
                {"op": "set", "id": "simulation-workflow", "property": "active",
                 "value": int(event.payload.get("index", 0))},
            ], request_id=event.id)
            return
        if event.action in {"preview_drive_level", "set_frequency_start", "set_speaker_model"}:
            context.acknowledge(event)
            return
        if event.action != "run-showcase-simulation":
            context.reject(event, "unknown_action", "This showcase action is not available.")
            return

        context.acknowledge(event)

        def simulate(token) -> None:
            for completed in range(1, 6):
                if token.cancelled:
                    context.job_log("showcase-simulation", "Simulation cancelled by user.", "warn")
                    return
                context.job(
                    "showcase-simulation", "running", completed=completed, total=5,
                    message=f"Solving frequency band {completed} of 5",
                )
                context.job_log("showcase-simulation", f"Completed band {completed}/5")
                time.sleep(0.08)
            context.patch([
                {"op": "set", "id": "simulation-result", "property": "value", "value": "Ready"},
            ], request_id=event.id)

        context.spawn_job("showcase-simulation", simulate, resource_tags=("simulation",))


def build_app() -> App:
    surface = build_surface_spec()
    lines = build_lines_spec()
    scatter_x, scatter_y = generate_scatter_data()
    line_x, line_y = generate_frequency_response()
    heatmap_size = 24
    heatmap_z = generate_heatmap_data(heatmap_size)

    return RuntimeShowcase(
        title="GPUI Python Runtime Showcase",
        sidebar_title="Python GPUI",
        sidebar_subtitle="Python app, Rust renderers",
        sections=[
            section("overview", "Overview", overview_section()),
            section("ui-kit", "UI Kit", ui_kit_section()),
            section(
                "charts",
                "gpui-px Charts",
                ui.vstack(
                    [
                        ui.section_header(
                            "gpui-px Charts",
                            "Wheel zoom, drag pan, double-click reset; keyboard +/− and arrows also work.",
                        ),
                        ui.wrap(
                            [
                                charts.scatter(
                                    "latency",
                                    scatter_x,
                                    scatter_y,
                                    title="Callback Latency",
                                    color="#1f77b4",
                                    point_radius=4.0,
                                ),
                                charts.line(
                                    "response",
                                    line_x,
                                    line_y,
                                    title="Frequency Response",
                                    color="#ff7f0e",
                                    x_log=True,
                                    stroke_width=2.0,
                                    x_label="Frequency (Hz)",
                                    y_label="Level (dB)",
                                    series=[
                                        charts.Series("measured", line_x, line_y, label="Measured", color="#ff7f0e"),
                                        charts.Series(
                                            "target",
                                            line_x,
                                            [0.0 for _ in line_x],
                                            label="Target",
                                            color="#22c55e",
                                        ),
                                    ],
                                ),
                                charts.bar(
                                    "scene-nodes",
                                    ["Surface", "Lines", "Mesh", "Light", "Callback"],
                                    [42.0, 31.0, 18.0, 8.0, 5.0],
                                    title="Scene Nodes",
                                    color="#2ca02c",
                                ),
                                charts.heatmap(
                                    "uploads",
                                    heatmap_z,
                                    heatmap_size,
                                    heatmap_size,
                                    title="Upload Activity",
                                    color_scale="viridis",
                                    x=[float(index) for index in range(heatmap_size)],
                                    y=[float(index) for index in range(heatmap_size)],
                                    color_label="Upload intensity",
                                    color_unit="a.u.",
                                    color_range=(0.0, 1.0),
                                    aspect_ratio=1.0,
                                ),
                            ],
                            gap=20.0,
                        ),
                    ],
                    gap=20.0,
                ),
            ),
            section(
                "surface",
                "3D Surface",
                ui.vstack(
                    [
                        ui.section_header("3D Surface", "A log-frequency surface declared in Python"),
                        ui.scene3d(surface, width=760.0, height=480.0),
                        ui.card(
                            [
                                ui.table(
                                    ["field", "value"],
                                    [
                                        ["id", surface.to_spec()["id"]],
                                        ["grid", "10 x 7"],
                                        ["camera", "orbit distance 3.8"],
                                        ["resource path", "Surface3DElement"],
                                    ],
                                )
                            ]
                        ),
                    ],
                    gap=20.0,
                ),
            ),
            section(
                "lines",
                "3D Lines",
                ui.vstack(
                    [
                        ui.section_header("3D Lines", "Line strips use the same retained orbit model"),
                        ui.scene3d(lines, width=700.0, height=440.0),
                        ui.card(
                            [
                                ui.table(
                                    ["field", "value"],
                                    [
                                        ["id", lines.to_spec()["id"]],
                                        ["strips", "helix + xyz axes"],
                                        ["resource path", "Lines3DElement"],
                                    ],
                                )
                            ]
                        ),
                    ],
                    gap=20.0,
                ),
            ),
            section(
                "scene-specs",
                "Scene Specs",
                ui.vstack(
                    [
                        ui.section_header("Scene Specs", "Stable ids drive retained GPU resources"),
                        ui.wrap(
                            [
                                ui.metric("Surface samples", len(surface.to_spec()["z"]["values"])),
                                ui.metric("Line points", 86),
                                ui.metric("Cache entries", 2),
                                ui.metric("Python calls while idle", 0),
                            ],
                            gap=16.0,
                        ),
                        ui.hstack(
                            [
                                ui.scene3d(surface, id="surface-preview", width=420.0, height=280.0),
                                ui.scene3d(lines, id="lines-preview", width=420.0, height=280.0),
                            ],
                            gap=20.0,
                        ),
                        ui.scene3d(build_mesh_scene(), id="mesh-scene-preview", width=420.0, height=180.0),
                    ],
                    gap=20.0,
                ),
            ),
        ],
    )


def overview_section() -> ui.Node:
    return ui.vstack(
        [
            ui.section_header("Python-authored Showcase", "The app shell, sections, charts, and 3D specs are Python data"),
            ui.wrap(
                [
                    ui.metric("UI sections", 6),
                    ui.metric("Chart demos", 4),
                    ui.metric("3D specs", 3),
                    ui.metric("Raw wgpu exposed", 0),
                ],
                gap=16.0,
            ),
            ui.card(
                [
                    ui.heading("Runtime Boundary", level=2),
                    ui.text("Python declares stable ids, layout, chart data, and scene3d resources."),
                    ui.text("Rust owns GPUI, gpui-ui-kit rendering, gpui-px charts, retained 3D resources, and the native event loop."),
                    ui.hstack(
                        [
                            ui.badge("JSON UI IR", tone="accent"),
                            ui.badge("Retained 3D", tone="success"),
                            ui.badge("Native GPUI", tone="neutral"),
                        ],
                        gap=8.0,
                    ),
                ],
                width=760.0,
            ),
        ],
        gap=20.0,
    )


def ui_kit_section() -> ui.Node:
    return ui.vstack(
        [
            ui.section_header("UI Kit", "Python helpers cover the showcase component set"),
            ui.wrap(
                [
                    ui.card(
                        [
                            ui.heading("Actions", level=2),
                            ui.hstack(
                                [
                                    ui.button("Primary", selected=True),
                                    ui.button("Secondary"),
                                    ui.button("Disabled", disabled=True),
                                ],
                                gap=8.0,
                            ),
                        ],
                        width=360.0,
                    ),
                    ui.card(
                        [
                            ui.heading("Status", level=2),
                            ui.hstack(
                                [
                                    ui.badge("Ready", tone="success"),
                                    ui.badge("Preview", tone="accent"),
                                    ui.badge("Static", tone="neutral"),
                                ],
                                gap=8.0,
                            ),
                            ui.progress(0.68, label="Bridge coverage"),
                            ui.spinner("Renderer warm"),
                        ],
                        width=360.0,
                    ),
                    ui.card(
                        [
                            ui.heading("Navigation", level=2),
                            ui.tabs(["Layout", "Controls", "Data"], active=1),
                            ui.text("Tabs are represented in Python and styled by the host."),
                        ],
                        width=360.0,
                    ),
                    ui.card(
                        [
                            ui.heading("Simulation inputs", level=2),
                            ui.stepper(
                                id="simulation-workflow",
                                steps=["Model", "Solve", "Review"],
                                active=0,
                                action="set_simulation_step",
                            ),
                            ui.number_input(
                                id="frequency-start",
                                label="Start frequency",
                                value="20e",  # Deliberately intermediate text while editing.
                                unit="Hz",
                                commit_action="set_frequency_start",
                                validation={"severity": "error", "message": "Complete the exponent before running."},
                            ),
                            ui.slider(
                                id="drive-level",
                                label="Drive level",
                                value=0.65,
                                minimum=0.0,
                                maximum=1.0,
                                step=0.01,
                                show_value=True,
                                action="preview_drive_level",
                                commit_action="commit_drive_level",
                            ),
                            ui.accordion(
                                id="advanced-solver",
                                expanded=["tolerance"],
                                action="set_advanced_solver",
                                items=[(
                                    "tolerance", "Advanced solver settings", [
                                        ui.text("Changes are validated by the Python session.", tone="secondary"),
                                    ],
                                )],
                            ),
                            ui.list_editor(
                                id="evaluation-frequencies",
                                label="Evaluation frequencies",
                                rows=[
                                    {"id": "frequency-100", "label": "100 Hz", "value": 100.0},
                                    {"id": "frequency-1000", "label": "1 kHz", "value": 1000.0},
                                ],
                                add_action="add_evaluation_frequency",
                                remove_action="remove_evaluation_frequency",
                                reorder_action="reorder_evaluation_frequency",
                            ),
                            ui.path_input(
                                id="speaker-model",
                                label="Speaker model",
                                placeholder="Choose an .mlg or .json model",
                                value="",
                                filters=[("Speaker models", ["mlg", "json"])],
                                recent_values=["/models/reference.mlg", "/models/calibrated.json"],
                                must_exist=True,
                                commit_action="set_speaker_model",
                            ),
                            ui.checkbox(id="field-map", value=None, label="Generate field map"),
                            ui.button(
                                "Run showcase simulation", id="run-showcase-simulation",
                                action="run-showcase-simulation", selected=True,
                            ),
                            ui.metric("Latest result", "Not run", id="simulation-result"),
                        ],
                        width=420.0,
                    ),
                    ui.card(
                        [
                            ui.heading("Data", level=2),
                            ui.table(
                                id="runtime-capabilities",
                                columns=[
                                    {"id": "component", "label": "Component", "sortable": True, "width": 150},
                                    {"id": "state", "label": "State", "sortable": True, "width": 130},
                                ],
                                typed_rows=[
                                    {"id": "buttons", "cells": ["Buttons", "wrapped"]},
                                    {"id": "charts", "cells": ["Charts", "native"]},
                                    {"id": "scene3d", "cells": ["Scene3D", "retained"]},
                                ],
                                sort_action="sort_runtime_capabilities",
                            ),
                        ],
                        width=360.0,
                    ),
                ],
                gap=20.0,
            ),
        ],
        gap=20.0,
    )


def build_surface_spec() -> s3.Surface:
    freqs = [20.0, 40.0, 80.0, 160.0, 315.0, 630.0, 1250.0, 2500.0, 5000.0, 10000.0]
    angles = [-90.0, -60.0, -30.0, 0.0, 30.0, 60.0, 90.0]
    z: list[list[float]] = []

    for angle in angles:
        angle_weight = abs(angle) / 90.0
        row: list[float] = []
        for freq in freqs:
            octave = math.log2(freq / 1000.0)
            on_axis_ripple = 2.0 * math.sin(octave * 2.4)
            off_axis_rolloff = -9.0 * angle_weight * max(0.0, math.log10(freq / 1000.0))
            row.append(on_axis_ripple + off_axis_rolloff)
        z.append(row)

    return s3.surface(
        "dispersion",
        z=z,
        x=freqs,
        y=angles,
        colormap="turbo",
        x_log=True,
        z_range=(-12.0, 4.0),
        labels={"x": "Frequency (Hz)", "y": "Angle (deg)", "z": "Level (dB)"},
        camera=s3.orbit(distance=3.8, azimuth=58.0, elevation=28.0),
        interactions=["orbit", "pan", "zoom", "reset"],
    )


def build_lines_spec() -> s3.Lines:
    helix = []
    for index in range(80):
        t = index / 79.0
        angle = t * 2.5 * math.tau
        radius = 0.7 + 0.2 * math.sin(t * math.tau)
        helix.append((radius * math.cos(angle), (t - 0.5) * 1.8, radius * math.sin(angle)))

    return s3.lines(
        "orbit-lines",
        strips=[
            s3.line_strip("helix", helix, color="#7dd3fc", width=2.5),
            s3.line_strip("x-axis", [(-1.2, 0.0, 0.0), (1.2, 0.0, 0.0)], color="#ef4444"),
            s3.line_strip("y-axis", [(0.0, -1.0, 0.0), (0.0, 1.0, 0.0)], color="#22c55e"),
            s3.line_strip("z-axis", [(0.0, 0.0, -1.2), (0.0, 0.0, 1.2)], color="#3b82f6"),
        ],
        background="#0b1020",
        camera=s3.orbit(distance=4.2, azimuth=42.0, elevation=24.0),
        interactions=["orbit", "pan", "zoom", "reset"],
    )


def build_mesh_scene() -> s3.Scene:
    return s3.scene(
        "speaker-model",
        camera=s3.orbit(distance=3.5, azimuth=45.0, elevation=30.0),
        children=[
            s3.mesh(
                "speaker",
                vertices=[(-0.6, -0.5, 0.0), (0.6, -0.5, 0.0), (0.0, 0.7, 0.0)],
                indices=[0, 1, 2],
                material=s3.material("#88ccff", opacity=0.82),
                scalar_values=[0.1, 0.65, 1.0],
                scalar_location="vertex",
                colormap="turbo",
                scalar_range=(0.0, 1.0),
                scalar_label="Normalized displacement",
            ),
            s3.light("key", direction=(1.0, -2.0, -1.0), intensity=1.3),
        ],
        background="#111827",
    )


def generate_scatter_data() -> tuple[list[float], list[float]]:
    x: list[float] = []
    y: list[float] = []
    for index in range(80):
        t = index / 79.0
        x.append(t * 100.0)
        y.append(20.0 + 28.0 * t + 8.0 * math.sin(t * math.tau * 3.0))
    return x, y


def generate_frequency_response() -> tuple[list[float], list[float]]:
    x: list[float] = []
    y: list[float] = []
    for index in range(72):
        freq = 20.0 * 10 ** (index / 23.0)
        bass_shelf = -5.0 * (120.0 - freq) / 100.0 if freq < 120.0 else 0.0
        treble = -4.0 * (freq - 6000.0) / 14000.0 if freq > 6000.0 else 0.0
        x.append(freq)
        y.append(bass_shelf + treble + 1.2 * math.sin(math.log2(freq / 1000.0) * 3.0))
    return x, y


def generate_heatmap_data(size: int) -> list[float]:
    values: list[float] = []
    for y in range(size):
        for x in range(size):
            nx = x / (size - 1) * 2.0 - 1.0
            ny = y / (size - 1) * 2.0 - 1.0
            left = math.exp(-((nx + 0.35) ** 2 + (ny - 0.2) ** 2) * 8.0)
            right = 0.7 * math.exp(-((nx - 0.4) ** 2 + (ny + 0.25) ** 2) * 18.0)
            values.append(left + right)
    return values


if __name__ == "__main__":
    build_app().run()
