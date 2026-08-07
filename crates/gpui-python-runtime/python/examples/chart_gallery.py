"""High-level chart gallery for the Python GPUI Toolkit declarations."""

from __future__ import annotations

import json
import math
import os

from gpui_toolkit import App, charts, section, ui


def _frequency_samples(count: int = 48) -> list[float]:
    return [20.0 * (20_000.0 / 20.0) ** (index / (count - 1)) for index in range(count)]


def _heatmap_values(width: int, height: int) -> list[float]:
    return [
        0.5 + 0.5 * math.sin(column * 0.55) * math.cos(row * 0.45)
        for row in range(height)
        for column in range(width)
    ]


def build_line_charts() -> ui.Node:
    frequencies = _frequency_samples()
    measured = [
        2.0 * math.sin(math.log10(frequency) * 3.0)
        + 0.7 * math.sin(math.log10(frequency) * 11.0)
        for frequency in frequencies
    ]
    target = [0.0 for _ in frequencies]
    return ui.wrap(
        [
            charts.line(
                "response",
                frequencies,
                measured,
                title="Measured response",
                x_log=True,
                x_label="Frequency (Hz)",
                y_label="Level (dB)",
                series=(
                    charts.Series("measured", frequencies, measured, label="Measured", color="#f97316"),
                    charts.Series("target", frequencies, target, label="Target", color="#22c55e", dash=charts.StrokeDash.DASHED),
                ),
                legend_position=charts.LegendPosition.BOTTOM,
            ),
            charts.area(
                "band-energy",
                list(range(1, 13)),
                [2.0 + math.sin(index * 0.7) for index in range(1, 13)],
                y0=[0.0] * 12,
                title="Band energy",
                color="#38bdf8",
                opacity=0.65,
            ),
            charts.scatter(
                "response-points",
                frequencies[::4],
                measured[::4],
                title="Response samples",
                x_log=True,
                color="#facc15",
                point_radius=5.0,
            ),
        ],
        gap=20.0,
    )


def build_grid_charts() -> ui.Node:
    width = 18
    height = 12
    values = _heatmap_values(width, height)
    axes_x = [float(index) for index in range(width)]
    axes_y = [float(index) for index in range(height)]
    return ui.wrap(
        [
            charts.heatmap(
                "heatmap",
                values,
                width,
                height,
                title="Heatmap",
                x=axes_x,
                y=axes_y,
                color_label="Intensity",
                color_unit="a.u.",
                color_range=(0.0, 1.0),
                aspect_ratio=1.35,
            ),
            charts.contour(
                "contour",
                values,
                width,
                height,
                title="Contour",
                x=axes_x,
                y=axes_y,
                color_scale="turbo",
                levels=(0.2, 0.4, 0.6, 0.8),
                aspect_ratio=1.35,
            ),
            charts.isoline(
                "isoline",
                values,
                width,
                height,
                title="Isolines",
                x=axes_x,
                y=axes_y,
                levels=(0.25, 0.5, 0.75),
            ),
        ],
        gap=20.0,
    )


def build_category_charts() -> ui.Node:
    labels = ["Woofer", "Midrange", "Tweeter", "Port"]
    values = [42.0, 28.0, 18.0, 12.0]
    return ui.wrap(
        [
            charts.bar(
                "components",
                labels,
                values,
                title="Component contribution",
                color="#a78bfa",
                y_label="Percent",
            ),
            charts.pie("components-pie", labels, values, title="Component mix"),
            charts.donut("components-donut", labels, values, title="Component mix", inner_radius=0.55),
        ],
        gap=20.0,
    )


def build_distribution_charts() -> ui.Node:
    samples = [
        0.8,
        1.1,
        1.2,
        1.4,
        1.5,
        1.7,
        1.9,
        2.0,
        2.2,
        2.5,
        2.7,
        3.0,
    ]
    tree = charts.TreemapNode(
        "Speaker",
        children=(
            charts.TreemapNode("Low frequency", value=42.0),
            charts.TreemapNode("Mid frequency", value=28.0),
            charts.TreemapNode("High frequency", value=18.0),
        ),
    )
    return ui.wrap(
        [
            charts.boxplot("latency-box", list(range(len(samples))), samples, title="Callback latency"),
            charts.treemap("speaker-tree", tree, title="Response budget"),
        ],
        gap=20.0,
    )


def build_app() -> App:
    return App(
        title="Chart Gallery (Python)",
        sidebar_title="Python Chart Gallery",
        sidebar_subtitle="gpui-px declarations",
        width=1240.0,
        height=820.0,
        sections=[
            section(
                "overview",
                "Overview",
                ui.vstack(
                    [
                        ui.section_header(
                            "Python Chart Gallery",
                            "Chart specifications are plain Python values and render natively in GPUI.",
                        ),
                        ui.wrap(
                            [
                                ui.metric("Chart families", 11),
                                ui.metric("External dependencies", 0),
                                ui.metric("GPU handles in Python", 0),
                            ],
                            gap=16.0,
                        ),
                    ],
                    gap=20.0,
                ),
            ),
            section(
                "lines",
                "Lines",
                ui.vstack(
                    [
                        ui.section_header("Lines and areas", "Named series, log axes, styles, and filled ranges."),
                        build_line_charts(),
                    ],
                    gap=20.0,
                ),
            ),
            section(
                "grids",
                "Grids",
                ui.vstack(
                    [
                        ui.section_header("Heatmaps and contours", "The same row-major grid can drive several renderers."),
                        build_grid_charts(),
                    ],
                    gap=20.0,
                ),
            ),
            section(
                "categories",
                "Categories",
                ui.vstack(
                    [
                        ui.section_header("Categorical charts", "Bars, pies, and donuts share typed labels and values."),
                        build_category_charts(),
                    ],
                    gap=20.0,
                ),
            ),
            section(
                "distribution",
                "Distribution",
                ui.vstack(
                    [
                        ui.section_header("Distribution and hierarchy", "Box plots and treemaps cover result-summary layouts."),
                        build_distribution_charts(),
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
        print(json.dumps(app.to_spec(), indent=2))


if __name__ == "__main__":
    main()
