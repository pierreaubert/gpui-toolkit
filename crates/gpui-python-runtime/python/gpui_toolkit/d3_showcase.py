"""A Python-only implementation of the ``gpui-d3rs`` showcase.

Run this through the generic installed host:

``cargo run -p gpui-python-runtime --features showcase --bin gpui-python-showcase -- \
crates/gpui-python-runtime/python/d3_showcase.py``

There is deliberately no d3rs-showcase-specific Rust route here.  Python owns
the gallery declaration and interactions; the host only consumes public UI IR,
chart specs, and typed d3 commands.
"""

from __future__ import annotations

from array import array
from importlib.util import find_spec

import math

from gpui_toolkit import App, Event, SessionContext, d3rs as d3, data, px, section, ui
from gpui_toolkit.commands import CommandResult


# Keep this in the same order as gpui-d3rs/bin/showcase/main/demo_section.rs.
D3RS_SECTION_ORDER = (
    ("overview", "Overview"), ("scales", "Scales"), ("axes", "Axes"),
    ("bar-charts", "Bar Charts"), ("line-charts", "Line Charts"),
    ("scatter-plots", "Scatter Plots"), ("lod-large-data", "LOD Data"),
    ("surface-plots", "Surface Plots"), ("meshes", "Meshes"),
    ("quadtree", "QuadTree"), ("contours", "Contours"),
    ("transitions", "Transitions"), ("geo", "Geo"), ("colors", "Colors"),
    ("hierarchy", "Hierarchy"), ("force", "Force Graph"),
    ("chord", "Chord Diagram"), ("d3-volcano", "D3: Volcano"),
    ("d3-kde", "D3: KDE"), ("d3-treemap", "D3: Treemap"),
    ("d3-stacked-bars", "D3: Stacked Bars"),
    ("d3-versor", "D3: Versor Dragging"),
    ("d3-histogram", "D3: Histogram"),
    ("d3-revenue", "D3: Revenue Stream"),
    ("d3-horizon", "D3: Horizon Chart"),
    ("d3-choropleth", "D3: Choropleth"),
    ("d3-sankey", "D3: Sankey Diagram"),
    ("d3-calendar", "D3: Calendar Heatmap"),
    ("d3-radial-line", "D3: Radial Line"),
    ("d3-parallel-coordinates", "D3: Parallel Coord"),
    ("d3-hexbin", "D3: Hexbin"), ("d3-pie", "D3: Pie Chart"),
    ("d3-donut", "D3: Donut Chart"), ("d3-line", "D3: Line Chart"),
    ("d3-streamgraph", "D3: Streamgraph"),
    ("d3-stacked-bar", "D3: Stacked Bar"),
    ("d3-stacked-area", "D3: Stacked Area"),
    ("d3-box-plot", "D3: Box Plot"),
    ("d3-chord-diagram", "D3: Chord Diagram"),
    ("d3-force-directed", "D3: Force Directed"),
    ("d3-parallel-sets", "D3: Parallel Sets"),
    ("d3-difference-chart", "D3: Difference Chart"),
    ("d3-ridgeline", "D3: Ridgeline Plot"),
    ("d3-realtime-horizon", "D3: Realtime Horizon"),
    ("d3-radial-tree", "D3: Radial Tree"),
    ("d3-radial-cluster", "D3: Radial Cluster"),
    ("d3-circle-packing", "D3: Circle Packing"),
    ("d3-sunburst", "D3: Sunburst"),
    ("d3-voronoi-airports", "D3: Voronoi Airports"),
    ("d3-temperature-trends", "D3: Temperature Trends"),
    ("d3-hertzsprung-russell", "D3: H-R Diagram"),
    ("d3-voronoi-labels", "D3: Voronoi Labels"),
    ("d3-electric-usage", "D3: Electric Usage"),
    ("d3-star-map", "D3: Star Map"),
    ("d3-voronoi-stippling", "D3: Voronoi Stippling"),
)

# Deliberately small fixture: both v2 nodes bind this stable resource identity,
# while a production publisher may commit millions of rows independently.
D3_EVENTS = data.Dataset.from_mapping(
    {
        "event_id": [1, 2, 3, 4],
        "frequency": [63.0, 125.0, 250.0, 500.0],
        "spl": [72.0, 74.0, 71.0, 76.0],
        # The chart exercises the declarative, host-evaluated DatasetView
        # path; inactive probe readings remain in the table for inspection.
        "enabled": [True, False, True, True],
    },
    key="event_id",
    id="d3rs-events",
)


def _wave(points: int = 96) -> tuple[list[float], list[float]]:
    x = [index / (points - 1) * 12.0 for index in range(points)]
    return x, [math.sin(value) + 0.25 * math.sin(value * 3.0) for value in x]


def _heatmap(size: int = 24) -> list[float]:
    return [
        math.exp(-(((x / (size - 1)) - 0.32) ** 2 + ((y / (size - 1)) - 0.65) ** 2) * 22)
        + 0.65 * math.exp(-(((x / (size - 1)) - 0.72) ** 2 + ((y / (size - 1)) - 0.30) ** 2) * 32)
        for y in range(size)
        for x in range(size)
    ]


def _chart_for(
    section_id: str,
    resources: list[data.Dataset | data.ArrayData],
):
    x, y = _wave()
    chart_id = f"{section_id}-chart"
    if section_id in {"d3-pie", "d3-donut", "chord", "d3-chord-diagram"}:
        categories = data.Dataset.from_mapping(
            {"label": ["A", "B", "C", "D"], "value": [24, 18, 31, 27]},
            id=f"{chart_id}-data",
        )
        resources.append(categories)
        factory = px.donut if section_id == "d3-donut" else px.pie
        return factory(chart_id).data(categories).label("label").y("value").title("Category flow")
    if section_id in {"contours", "d3-volcano", "d3-calendar", "surface-plots"}:
        grid = data.ArrayData.from_buffer(
            array("d", _heatmap()),
            shape=(24, 24),
            dtype="f64",
            id=f"{chart_id}-data",
        )
        resources.append(grid)
        return px.heatmap(chart_id).data(grid).color_scale(px.ColorScale.VIRIDIS).title("Density field")
    if section_id in {"bar-charts", "d3-stacked-bars", "d3-stacked-bar", "d3-histogram", "d3-revenue", "d3-electric-usage"}:
        totals = data.Dataset.from_mapping(
            {
                "category": ["Mon", "Tue", "Wed", "Thu", "Fri"],
                "value": [18, 26, 21, 33, 29],
            },
            id=f"{chart_id}-data",
        )
        resources.append(totals)
        return px.bar(chart_id).data(totals).x("category").y("value").title("Daily totals")
    if section_id in {"scatter-plots", "d3-hexbin", "d3-voronoi-airports", "d3-hertzsprung-russell", "d3-star-map"}:
        points = data.Dataset.from_mapping(
            {"x": x, "y": y}, id=f"{chart_id}-data"
        )
        resources.append(points)
        return (
            px.scatter(chart_id)
            .data(points)
            .x("x")
            .y("y")
            .point_radius(3.0)
            .title("Spatial sample")
        )
    if section_id in {"d3-treemap", "hierarchy", "d3-circle-packing", "d3-sunburst"}:
        hierarchy = data.Dataset.from_mapping(
            {
                "id": ["root", "audio", "gpu", "ui"],
                "parent": [None, "root", "root", "root"],
                "label": ["Root", "Audio", "GPU", "UI"],
                "value": [0.0, 28.0, 42.0, 30.0],
            },
            key="id",
            id=f"{chart_id}-data",
        )
        resources.append(hierarchy)
        return (
            px.treemap(chart_id)
            .data(hierarchy)
            .row_id("id")
            .parent("parent")
            .label("label")
            .size("value")
            .title("Hierarchy")
        )
    samples = data.Dataset.from_mapping({"x": x, "y": y}, id=f"{chart_id}-data")
    resources.append(samples)
    return px.line(chart_id).data(samples).x("x").y("y").title("Computed sample")


def _gallery_section(
    section_id: str,
    label: str,
    resources: list[data.Dataset | data.ArrayData],
):
    return section(
        section_id,
        label,
        ui.vstack(
            [
                ui.section_header(label, "Python declaration rendered by gpui-d3rs through the generic host."),
                _chart_for(section_id, resources),
                ui.hstack(
                    [
                        ui.badge("Python authored", tone="success"),
                        ui.badge("gpui-d3rs renderer", tone="info"),
                        ui.button("Recompute", id=f"{section_id}-recompute", action="recompute-d3"),
                    ],
                    gap=10.0,
                ),
            ],
            gap=16.0,
        ),
    )


class D3rsShowcase(App):
    """Python-owned d3rs gallery and typed native-command demonstration."""

    gallery_resources: tuple[data.Dataset | data.ArrayData, ...] = ()

    def on_session_ready(self, context: SessionContext) -> None:
        # Prefer the public binary Dataset binding whenever its optional Arrow
        # adapter is installed. Source-only validation intentionally has no
        # PyArrow dependency, so it remains descriptor-only in that case.
        if find_spec("pyarrow") is not None:
            context.bind_dataset(D3_EVENTS)
        else:
            context.bind_resource(D3_EVENTS)
        for resource in self.gallery_resources:
            if isinstance(resource, data.ArrayData):
                context.bind_array(resource)
            elif find_spec("pyarrow") is not None:
                context.bind_dataset(resource)
            else:
                context.bind_resource(resource)
        d3.ScaleRequest(d3.ScaleKind.LINEAR, [0, 100], [0, 960], [0, 25, 50, 100]).send(context, "d3rs-scale")
        d3.AlgorithmRequest.hexbin([(0, 0), (4, 2), (7, 5), (11, 3)], radius=2.0).send(context, "d3rs-hexbin")
        d3.ZoomRequest((0, 100), (-1, 1), [d3.ZoomOperation.zoom_to((20, 80), (-0.5, 0.5))]).send(context, "d3rs-zoom")
        d3.request_module_catalog(context, "d3rs-modules")
        d3.request_reports(context, "d3rs-reports")

    def on_action(self, event: Event, context: SessionContext) -> None:
        if event.action == "d3rs-row-selected":
            context.acknowledge(event)
            keys = event.payload.get("keys", [])
            selected = ", ".join(str(key) for key in keys) or "none"
            context.patch(
                [
                    {
                        "op": "set",
                        "id": "d3rs-command-status",
                        "property": "value",
                        "value": f"selected: {selected}",
                    }
                ]
            )
            return
        if event.action != "recompute-d3":
            context.reject(event, "unknown_action", "This d3rs showcase action is not available.")
            return
        context.acknowledge(event)
        d3.AlgorithmRequest.lod_m4([0, 1, 2, 3], [0.0, 1.0, -0.5, 0.75], x0=0, x1=3, columns=2).send(context, event.id)

    def on_command_result(self, request_id: str, result: CommandResult, context: SessionContext) -> None:
        value = "ready" if result.ok else result.status.value
        context.patch([{"op": "set", "id": "d3rs-command-status", "property": "value", "value": value}], request_id=request_id)


def build_app() -> App:
    """Build the complete d3rs navigation catalog using Python declarations only."""
    resources: list[data.Dataset | data.ArrayData] = []
    overview = section(
        "overview",
        "Overview",
        ui.vstack(
            [
                ui.section_header("gpui-d3rs Showcase", "The d3rs gallery authored entirely in the installed Python API."),
                ui.wrap([
                    ui.metric("Gallery sections", len(D3RS_SECTION_ORDER)),
                    ui.metric("Typed d3 commands", 5),
                    ui.metric("Rust showcase render branches", 0),
                    ui.metric("Command status", "pending", id="d3rs-command-status"),
                ], gap=16.0),
                (ui.Table("d3rs-events-table")
                    .data(D3_EVENTS)
                    .column(ui.Column("frequency").field("frequency").sortable())
                    .column(ui.Column("spl").field("spl"))
                    .selection_mode(ui.SelectionMode.SINGLE)
                    .virtualize(row_height=28, overscan=12)
                    .on_selection_change("d3rs-row-selected")),
                (px.scatter("d3rs-events-chart")
                    .data(
                        D3_EVENTS.view()
                        .filter(data.col("enabled") & (data.col("spl") > 0.0))
                        .range(0, 2)
                    )
                    .x("frequency")
                    .y("spl")
                    .lod(px.Lod.AUTO)
                    .title("Resource-backed response")),
                ui.text("Scale, zoom, algorithms, module inventory, and parity reports use typed gpui_toolkit.d3 requests."),
            ],
            gap=20.0,
        ),
    )
    app = D3rsShowcase(
        title="gpui-d3rs Python Showcase",
        sidebar_title="gpui-d3rs",
        sidebar_subtitle="Python declarations · native renderers",
        sections=[
            overview,
            *(
                _gallery_section(section_id, label, resources)
                for section_id, label in D3RS_SECTION_ORDER[1:]
            ),
        ],
    )
    app.gallery_resources = tuple(resources)
    return app


def main() -> None:
    """Launch the gallery through the generic installed GPUI host."""
    build_app().run()


__all__ = ["D3RS_SECTION_ORDER", "D3rsShowcase", "build_app", "main"]


if __name__ == "__main__":
    main()
