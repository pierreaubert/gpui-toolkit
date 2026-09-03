"""Resource-backed gpui-px chart gallery for the Python v2 API."""

from __future__ import annotations

from array import array
import json
import math
import os

from gpui_toolkit import App, data, px, section, ui


def _frequency_samples(count: int = 48) -> list[float]:
    return [20.0 * (20_000.0 / 20.0) ** (index / (count - 1)) for index in range(count)]


def _heatmap_values(width: int, height: int) -> list[float]:
    return [
        0.5 + 0.5 * math.sin(column * 0.55) * math.cos(row * 0.45)
        for row in range(height)
        for column in range(width)
    ]


def build_line_charts() -> tuple[ui.Node, tuple[object, ...]]:
    frequencies = _frequency_samples()
    measured = [
        2.0 * math.sin(math.log10(frequency) * 3.0)
        + 0.7 * math.sin(math.log10(frequency) * 11.0)
        for frequency in frequencies
    ]
    line_data = data.Dataset.from_mapping(
        {
            "frequency": frequencies + frequencies,
            "level": measured + [0.0 for _ in frequencies],
            "series": ["Measured"] * len(frequencies) + ["Target"] * len(frequencies),
            "color": ["#f97316"] * len(frequencies) + ["#22c55e"] * len(frequencies),
            "dash": ["solid"] * len(frequencies) + ["dashed"] * len(frequencies),
        },
        id="gallery-response",
    )
    band_data = data.Dataset.from_mapping(
        {
            "band": list(range(1, 13)),
            "energy": [2.0 + math.sin(index * 0.7) for index in range(1, 13)],
            "baseline": [0.0] * 12,
        },
        id="gallery-band-energy",
    )
    scatter_data = data.Dataset.from_mapping(
        {"frequency": frequencies[::4], "level": measured[::4]},
        id="gallery-response-points",
    )
    charts = ui.wrap(
        [
            px.line("response").data(line_data).x("frequency").y("level")
            .series("series").color("color").dash("dash")
            .title("Measured response").x_log()
            .x_label("Frequency (Hz)").y_label("Level (dB)")
            .legend_position(px.LegendPosition.BOTTOM),
            px.area("band-energy").data(band_data).x("band").y("energy")
            .y0("baseline").title("Band energy").opacity(0.65),
            px.scatter("response-points").data(scatter_data)
            .x("frequency").y("level").title("Response samples")
            .x_log().point_radius(5.0),
        ],
        gap=20.0,
    )
    return charts, (line_data, band_data, scatter_data)


def build_grid_charts() -> tuple[ui.Node, tuple[object, ...]]:
    width, height = 18, 12
    grid = data.ArrayData.from_buffer(
        array("d", _heatmap_values(width, height)),
        shape=(height, width),
        dtype="f64",
        id="gallery-grid",
    )
    charts = ui.wrap(
        [
            px.heatmap("field-heatmap").data(grid)
            .title("Response field").aspect_ratio(1.35),
            px.contour("field-contour").data(grid)
            .title("Filled contours").thresholds([0.2, 0.4, 0.6, 0.8])
            .aspect_ratio(1.35),
            px.isoline("field-isoline").data(grid)
            .title("Isolines").levels([0.25, 0.5, 0.75])
            .stroke_width(1.5).aspect_ratio(1.35),
        ],
        gap=20.0,
    )
    return charts, (grid,)


def build_category_charts() -> tuple[ui.Node, tuple[object, ...]]:
    categories = data.Dataset.from_mapping(
        {
            "component": ["Woofer", "Midrange", "Tweeter", "Port"],
            "value": [42.0, 28.0, 18.0, 12.0],
        },
        id="gallery-components",
    )
    charts = ui.wrap(
        [
            px.bar("components-bar").data(categories)
            .x("component").y("value").title("Component mix"),
            px.pie("components-pie").data(categories)
            .label("component").y("value").title("Component mix"),
            px.donut("components-donut").data(categories)
            .label("component").y("value").title("Component mix").hole(0.55),
        ],
        gap=20.0,
    )
    return charts, (categories,)


def build_distribution_charts() -> tuple[ui.Node, tuple[object, ...]]:
    samples = [0.8, 1.1, 1.2, 1.4, 1.5, 1.7, 1.9, 2.0, 2.2, 2.5, 2.7, 3.0]
    latency = data.Dataset.from_mapping(
        {"sample": list(range(len(samples))), "latency": samples},
        id="gallery-latency",
    )
    hierarchy = data.Dataset.from_mapping(
        {
            "node": ["Response", "Low frequency", "Mid frequency", "High frequency"],
            "parent": ["", "Response", "Response", "Response"],
            "value": [88.0, 42.0, 28.0, 18.0],
        },
        key="node",
        id="gallery-response-tree",
    )
    charts = ui.wrap(
        [
            px.boxplot("latency-box").data(latency)
            .x("sample").y("latency").title("Callback latency"),
            px.treemap("speaker-tree").data(hierarchy)
            .row_id("node").parent("parent").size("value")
            .title("Response budget"),
        ],
        gap=20.0,
    )
    return charts, (latency, hierarchy)


def build_app() -> App:
    lines, line_resources = build_line_charts()
    grids, grid_resources = build_grid_charts()
    categories, category_resources = build_category_charts()
    distribution, distribution_resources = build_distribution_charts()
    app = App(
        title="Chart Gallery (Python v2)",
        sidebar_title="Python Chart Gallery",
        sidebar_subtitle="resource-backed gpui-px declarations",
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
                            "Strict builders bind revisioned datasets and dense arrays to native GPUI charts.",
                        ),
                        ui.wrap(
                            [
                                ui.metric("Chart families", 11),
                                ui.metric("Inline chart values", 0),
                                ui.metric("GPU handles in Python", 0),
                            ],
                            gap=16.0,
                        ),
                    ],
                    gap=20.0,
                ),
            ),
            section("lines", "Lines", ui.vstack([
                ui.section_header("Lines and areas", "Multi-series color and dash roles plus aligned area baselines."),
                lines,
            ], gap=20.0)),
            section("grids", "Grids", ui.vstack([
                ui.section_header("Heatmaps and contours", "One dense ArrayData grid drives three native renderers."),
                grids,
            ], gap=20.0)),
            section("categories", "Categories", ui.vstack([
                ui.section_header("Categorical charts", "Bars, pies, and donuts share one Dataset resource."),
                categories,
            ], gap=20.0)),
            section("distribution", "Distribution", ui.vstack([
                ui.section_header("Distribution and hierarchy", "Box plots and resource-backed treemaps."),
                distribution,
            ], gap=20.0)),
        ],
    )
    app.resources = (
        *line_resources,
        *grid_resources,
        *category_resources,
        *distribution_resources,
    )
    return app


def main() -> None:
    app = build_app()
    if (
        os.environ.get("GPUI_TOOLKIT_DUMP_IR") != "1"
        or os.environ.get("GPUI_TOOLKIT_SESSION") == "1"
        or bool(os.environ.get("GPUI_TOOLKIT_HOST"))
    ):
        app.run()
    else:
        print(json.dumps(app.to_spec(), indent=2))


if __name__ == "__main__":
    main()
