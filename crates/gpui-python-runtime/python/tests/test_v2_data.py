import builtins
import unittest
from unittest.mock import patch
import contextlib
import io
import json
import os
import tempfile

from gpui_toolkit import (
    App,
    ResourceBackpressureError,
    ResourceFrameAcknowledgement,
    Section,
    SessionContext,
    data,
    native,
    px,
    ui,
)
from gpui_toolkit.app import PYTHON_SESSION_CAPABILITIES
from gpui_toolkit.commands import CommandResult


class ArrayBinaryStdout(io.StringIO):
    def __init__(self) -> None:
        super().__init__()
        self.buffer = io.BytesIO()


class V2DataTests(unittest.TestCase):
    def test_app_publishes_declared_resources_through_typed_bindings(self) -> None:
        dataset = data.Dataset.from_mapping({"value": [1.0]}, id="app-dataset")
        array_data = data.ArrayData.from_buffer(
            b"\x01\x02", shape=(2,), dtype="u8", id="app-array"
        )

        class RecordingContext:
            def __init__(self) -> None:
                self.datasets = []
                self.arrays = []

            def bind_dataset(self, resource):
                self.datasets.append(resource)

            def bind_array(self, resource):
                self.arrays.append(resource)

        context = RecordingContext()
        App(resources=(dataset, array_data)).on_session_ready(context)
        self.assertEqual(context.datasets, [dataset])
        self.assertEqual(context.arrays, [array_data])

        with self.assertRaisesRegex(TypeError, "Dataset and ArrayData"):
            App(resources=(object(),)).on_session_ready(context)

    def test_meshplot_compatibility_constructor_has_no_arbitrary_properties(self) -> None:
        import inspect
        from gpui_toolkit import meshplot

        self.assertFalse(
            any(
                parameter.kind is inspect.Parameter.VAR_KEYWORD
                for parameter in inspect.signature(meshplot.plot).parameters.values()
            )
        )

    def setUp(self) -> None:
        self.events = data.Dataset.from_mapping(
            {"event_id": [1, 2], "enabled": [True, False], "frequency": [20.0, 100.0], "spl": [1.0, 2.0], "channel": ["L", "R"]},
            key="event_id",
            id="events",
        )

    def test_dataset_view_and_chart_do_not_inline_rows(self) -> None:
        view = self.events.view().filter(data.col("enabled")).sort("frequency")
        chart = px.scatter("response").data(view).x("frequency").y("spl").color("channel").title("Response")
        spec = chart.to_spec()
        self.assertEqual(spec["data"]["source"]["dataset"]["id"], "events")
        self.assertEqual(spec["data"]["source"]["operations"][0]["op"], "filter")
        self.assertEqual(spec["data"]["source"]["operations"][1]["op"], "sort")
        self.assertNotIn("20.0", repr(spec))

        with self.assertRaises(data.DataError):
            self.events.view().sort("frequency").sort("spl")
        with self.assertRaises(data.DataError):
            self.events.view().range(0, 1).sort("frequency")
        with self.assertRaises(ValueError):
            px.scatter("bad-sort-role").data(
                self.events.view().sort("channel")
            ).x("frequency").y("spl").to_spec()
        with self.assertRaises(ValueError):
            px.bar("unsupported-sort").data(
                self.events.view().sort("frequency")
            ).label("channel").y("spl").to_spec()
        with self.assertRaises(ValueError):
            px.line("sort-range").data(
                self.events.view().sort("frequency").range(0, 1)
            ).x("frequency").y("spl").to_spec()

    def test_dataset_view_projection_is_consumed_by_bindings_and_tables(self) -> None:
        view = self.events.view().select("event_id", "frequency", "spl")
        chart = px.scatter("projected").data(view).x("frequency").y("spl")
        self.assertEqual(
            chart.to_spec()["data"]["source"]["operations"],
            [{"op": "select", "fields": ["event_id", "frequency", "spl"]}],
        )
        table = (
            ui.Table("projected-table")
            .data(view)
            .column(ui.Column("frequency").field("frequency"))
            .selection_mode(ui.SelectionMode.MULTIPLE)
        )
        self.assertEqual(table.to_spec()["data"]["operations"][0]["op"], "select")

        with self.assertRaises(data.DataError):
            chart.color("channel")
        with self.assertRaises(data.DataError):
            self.events.view().select()
        with self.assertRaises(data.DataError):
            self.events.view().select("spl", "spl")
        with self.assertRaises(data.DataError):
            view.select("spl")
        with self.assertRaises(data.DataError):
            view.sort("channel")
        with self.assertRaises(data.DataError):
            view.filter(data.col("enabled"))
        with self.assertRaises(ValueError):
            ui.Table("missing-projection").data(view).column(
                ui.Column("channel").field("channel")
            ).to_spec()

    def test_view_operations_are_immutable_serializable_ast(self) -> None:
        aggregated = (
            self.events.view()
            .group_by("channel")
            .aggregate(mean_spl="mean:spl", rows="count:*")
        )
        grouped = aggregated.sort("mean_spl").range(0, 12)
        operations = grouped.to_spec()["operations"]
        self.assertEqual([operation["op"] for operation in operations], ["group_by", "aggregate", "sort", "range"])
        self.assertEqual(grouped.available_fields, ("channel", "mean_spl", "rows"))

        windowed = self.events.view().bin("frequency", count=8).window(size=4, step=2)
        self.assertEqual(
            [operation["op"] for operation in windowed.to_spec()["operations"]],
            ["bin", "window"],
        )

        chart = px.bar("aggregated").data(grouped).x("channel").y("mean_spl")
        self.assertEqual(chart.to_spec()["data"]["roles"], {"x": "channel", "y": "mean_spl"})
        table = (
            ui.Table("aggregated")
            .data(aggregated)
            .column(ui.Column("channel").field("channel"))
            .column(ui.Column("mean").field("mean_spl"))
        )
        self.assertEqual(table.to_spec()["columns"][1]["field"], "mean_spl")

        with self.assertRaises(data.DataError):
            grouped.sort("spl")
        with self.assertRaises(data.DataError):
            self.events.view().group_by("channel").aggregate(channel="first:channel")
        with self.assertRaises(data.DataError):
            self.events.view().group_by("channel").group_by("enabled")
        with self.assertRaises(data.DataError):
            self.events.view().aggregate(rows="count:missing")
        with self.assertRaises(data.DataError):
            aggregated.filter(data.col("mean_spl"))
        with self.assertRaises(data.DataError):
            self.events.view().aggregate(bad="median:spl")

    def test_nullable_and_categorical_filter_ast_is_serializable(self) -> None:
        expression = data.col("channel").isin(["L", "R"]) & ~data.col("spl").is_null()
        spec = expression.to_spec()
        self.assertEqual(spec["op"], "and")
        self.assertEqual(spec["args"][0]["op"], "in")
        with self.assertRaises(data.DataError):
            data.col("channel").isin([])

    def test_dataset_descriptor_includes_logical_column_types(self) -> None:
        dataset = data.Dataset.from_mapping({"nullable": [None], "name": ["L"], "nested": [{"x": 1}], "items": [[1, 2]]})
        self.assertEqual(dataset.to_spec()["column_types"], {"nullable": "null", "name": "utf8", "nested": "struct", "items": "list"})

    def test_dataset_rejects_incompatible_column_values(self) -> None:
        with self.assertRaisesRegex(data.SchemaError, "incompatible"):
            data.Dataset.from_mapping({"mixed": ["text", 1]})

    def test_schema_fingerprint_includes_logical_types(self) -> None:
        integers = data.Dataset.from_mapping({"value": [1]})
        strings = data.Dataset.from_mapping({"value": ["1"]})
        self.assertNotEqual(integers.schema_fingerprint, strings.schema_fingerprint)

    def test_categorical_input_validates_dictionary_codes(self) -> None:
        dataset = data.Dataset.from_mapping({"channel": data.Categorical([0, 1, None], ["L", "R"])})
        self.assertEqual(dataset.row_count, 3)
        self.assertEqual(dataset.to_spec()["column_types"]["channel"], "dictionary")
        with self.assertRaises(data.DataError):
            data.Dataset.from_mapping({"channel": data.Categorical([2], ["L"])})

    def test_builders_are_immutable_and_validate_roles(self) -> None:
        original = px.scatter().data(self.events)
        configured = original.x("frequency")
        self.assertIsNone(original._binding)
        self.assertEqual(configured._binding.roles["x"], "frequency")
        with self.assertRaises(data.SchemaError):
            configured.y("missing")

    def test_resource_chart_builder_rejects_unknown_kind_and_empty_id(self) -> None:
        with self.assertRaisesRegex(ValueError, "unsupported resource chart kind"):
            px.ChartBuilder("unknown", "chart")
        with self.assertRaisesRegex(ValueError, "chart id"):
            px.ChartBuilder("line", "")

    def test_resource_area_and_boxplot_are_strict_data_builders(self) -> None:
        area = px.area("response-area").data(self.events).x("frequency").y("spl")
        box = px.boxplot("response-box").data(self.events).x("frequency").y("spl")
        self.assertEqual(area.to_spec()["chart"], "area")
        self.assertEqual(box.to_spec()["chart"], "box_plot")
        with self.assertRaisesRegex(ValueError, "requires .data"):
            px.area().x("frequency")

    def test_categorical_charts_preserve_label_and_value_roles(self) -> None:
        categories = data.Dataset.from_mapping(
            {"category": ["Low", "Mid", "High"], "value": [2.0, 5.0, 3.0]},
            id="categories",
        )
        for builder, kind in ((px.bar, "bar"), (px.pie, "pie"), (px.donut, "donut")):
            spec = builder().data(categories).label("category").y("value").to_spec()
            self.assertEqual(spec["chart"], kind)
            self.assertEqual(spec["data"]["roles"], {"label": "category", "y": "value"})
            self.assertNotIn("Low", repr(spec))

        # Bar charts retain x/y spelling while the host treats x as an Arrow
        # category rather than coercing it to a number.
        bar = px.bar().data(categories).x("category").y("value").to_spec()
        self.assertEqual(bar["data"]["roles"]["x"], "category")

    def test_categorical_chart_roles_fail_before_host_serialization(self) -> None:
        with self.assertRaisesRegex(ValueError, "label or x"):
            px.pie().data(self.events).y("spl").to_spec()
        with self.assertRaisesRegex(ValueError, "y field"):
            px.bar().data(self.events).x("channel").to_spec()

    def test_chart_legends_and_annotations_are_typed_and_immutable(self) -> None:
        annotation = (
            px.Annotation.point("peak", "Peak", 100.0, 2.0)
            .color("#FF0000")
            .series_index(0)
        )
        base = px.scatter().data(self.events).x("frequency").y("spl")
        configured = base.legend_position(px.LegendPosition.BOTTOM).annotation(annotation)
        self.assertNotIn("legend_position", base.to_spec())
        spec = configured.to_spec()
        self.assertEqual(spec["legend_position"], "bottom")
        self.assertEqual(
            spec["annotations"],
            [{
                "id": "peak", "label": "Peak", "target": "point",
                "x": 100.0, "y": 2.0, "color": "#ff0000", "series_index": 0,
            }],
        )

        category = px.Annotation.category("mid", "Mid band", "Mid")
        bar = (
            px.bar().data(self.events).x("channel").y("spl")
            .legend_position(px.LegendPosition.HIDDEN)
            .annotations([category])
        )
        self.assertEqual(bar.to_spec()["annotations"][0]["category"], "Mid")

    def test_chart_annotation_validation_is_strict(self) -> None:
        with self.assertRaisesRegex(ValueError, "finite"):
            px.Annotation.x_value("bad", "Bad", float("nan"))
        with self.assertRaisesRegex(ValueError, "#RRGGBB"):
            px.Annotation.y_value("bad", "Bad", 1.0).color("red")
        with self.assertRaisesRegex(ValueError, "does not support annotations"):
            px.pie().annotation(px.Annotation.category("a", "A", "A"))

    def test_scatter_line_and_bar_serialize_series_and_color_roles(self) -> None:
        for builder in (px.scatter, px.line):
            spec = (
                builder().data(self.events).x("frequency").y("spl")
                .series("channel").color("channel").to_spec()
            )
            self.assertEqual(spec["data"]["roles"]["series"], "channel")
            self.assertEqual(spec["data"]["roles"]["color"], "channel")
            self.assertNotIn("L", repr(spec))
        bar = (
            px.bar()
            .data(self.events)
            .x("channel")
            .y("spl")
            .series("channel")
            .color("channel")
            .to_spec()
        )
        self.assertEqual(bar["data"]["roles"]["series"], "channel")
        self.assertEqual(bar["data"]["roles"]["color"], "channel")
        for builder in (
            px.scatter().data(self.events).x("frequency").y("spl"),
            px.line().data(self.events).x("frequency").y("spl"),
            px.bar().data(self.events).x("channel").y("spl"),
        ):
            self.assertEqual(builder.graph_ratio(0.75).to_spec()["graph_ratio"], 0.75)
        with self.assertRaises(ValueError):
            px.area().graph_ratio(1.0)
        with self.assertRaises(ValueError):
            px.line().graph_ratio(0.0)
        with self.assertRaisesRegex(ValueError, "does not support series"):
            px.pie().data(self.events).label("channel").y("spl").series("channel").to_spec()

    def test_line_dash_role_is_typed_immutable_and_line_only(self) -> None:
        original = px.line("styled-series").data(self.events).x("frequency").y("spl")
        configured = original.series("channel").color("channel").dash("channel")

        self.assertNotIn("dash", original.to_spec()["data"]["roles"])
        self.assertEqual(configured.to_spec()["data"]["roles"]["dash"], "channel")
        binding = data.DataBinding(self.events).dash("channel")
        self.assertEqual(binding.to_spec()["roles"]["dash"], "channel")
        with self.assertRaisesRegex(ValueError, "only supported by line"):
            px.scatter().data(self.events).dash("channel")

    def test_line_secondary_axis_role_and_presentation_are_immutable(self) -> None:
        dual = data.Dataset.from_mapping(
            {
                "frequency": [100.0, 200.0, 100.0, 200.0],
                "level": [70.0, 72.0, 68.0, 71.0],
                "phase": [-30.0, -45.0, -25.0, -40.0],
                "channel": ["L", "L", "R", "R"],
            },
            id="dual-axis",
        )
        original = px.line("response").data(dual).x("frequency").y("level")
        configured = (
            original.y2("phase")
            .series("channel")
            .color("channel")
            .dash("channel")
            .y2_label("Phase (degrees)")
            .y2_range(-180.0, 180.0)
            .hidden_series([1])
            .on_legend_click("series-toggled")
        )
        self.assertNotIn("y2", original.to_spec()["data"]["roles"])
        spec = configured.to_spec()
        self.assertEqual(spec["data"]["roles"]["y2"], "phase")
        self.assertEqual(data.DataBinding(dual).y2("phase").to_spec()["roles"]["y2"], "phase")
        self.assertEqual(spec["y2_label"], "Phase (degrees)")
        self.assertEqual(spec["y2_range"], [-180.0, 180.0])
        self.assertEqual(spec["hidden_series"], [1])
        self.assertEqual(spec["legend_action"], "series-toggled")
        with self.assertRaises(ValueError):
            px.scatter().data(dual).y2("phase")
        with self.assertRaises(ValueError):
            px.bar().y2_label("Secondary")
        with self.assertRaises(ValueError):
            original.hidden_series([1, 1])
        with self.assertRaises(ValueError):
            px.scatter().on_legend_click("toggle")

    def test_area_y0_role_is_typed_immutable_and_area_only(self) -> None:
        original = px.area("band").data(self.events).x("frequency").y("spl")
        configured = original.y0("spl")

        self.assertNotIn("y0", original.to_spec()["data"]["roles"])
        self.assertEqual(configured.to_spec()["data"]["roles"]["y0"], "spl")
        binding = data.DataBinding(self.events).y0("spl")
        self.assertEqual(binding.to_spec()["roles"]["y0"], "spl")
        with self.assertRaisesRegex(ValueError, "only supported by area"):
            px.line().data(self.events).y0("spl")

    def test_scatter_point_radius_is_typed_immutable_and_serialized(self) -> None:
        original = px.scatter("points").data(self.events).x("frequency").y("spl")
        configured = original.point_radius(3.5)

        self.assertNotIn("point_radius", original.to_spec())
        self.assertEqual(configured.to_spec()["point_radius"], 3.5)
        self.assertRaisesRegex(ValueError, "positive", original.point_radius, 0.0)
        self.assertRaisesRegex(
            ValueError,
            "only supported",
            px.line().point_radius,
            3.0,
        )

    def test_cartesian_axis_configuration_is_typed_and_immutable(self) -> None:
        original = px.line("response").data(self.events).x("frequency").y("spl")
        configured = (
            original.x_log()
            .y_log(False)
            .x_label("Frequency (Hz)")
            .y_label(None)
            .x_range(20.0, 20_000.0)
            .y_range(40.0, 100.0)
        )

        self.assertNotIn("x_log", original.to_spec())
        self.assertEqual(
            {
                key: configured.to_spec()[key]
                for key in ("x_log", "y_log", "x_label", "y_label", "x_range", "y_range")
            },
            {
                "x_log": True,
                "y_log": False,
                "x_label": "Frequency (Hz)",
                "y_label": None,
                "x_range": [20.0, 20_000.0],
                "y_range": [40.0, 100.0],
            },
        )
        self.assertRaisesRegex(ValueError, "increasing", original.x_range, 2.0, 1.0)
        self.assertRaisesRegex(ValueError, "only supported", px.pie().x_log)
        self.assertRaises(TypeError, original.y_log, 1)

    def test_chart_style_and_level_configuration_is_typed_and_immutable(self) -> None:
        line = px.line().data(self.events).x("frequency").y("spl")
        styled = line.stroke_width(3.0).opacity(0.75).aspect_ratio(1.5)
        self.assertNotIn("stroke_width", line.to_spec())
        self.assertEqual(styled.to_spec()["stroke_width"], 3.0)
        self.assertEqual(styled.to_spec()["opacity"], 0.75)
        self.assertEqual(styled.to_spec()["aspect_ratio"], 1.5)

        grid = data.ArrayData.from_buffer(
            bytearray(range(12)), shape=(3, 4), dtype="u8", id="styled-grid"
        )
        contour = px.contour().data(grid).thresholds([1.0, 3.0, 7.0]).opacity(0.5)
        isoline = px.isoline().data(grid).levels([2.0, 5.0]).stroke_width(1.5)
        self.assertEqual(contour.to_spec()["thresholds"], [1.0, 3.0, 7.0])
        self.assertEqual(isoline.to_spec()["levels"], [2.0, 5.0])

        categories = data.Dataset.from_mapping(
            {"label": ["A", "B"], "value": [1.0, 2.0]}, id="pie-style"
        )
        donut = px.donut().data(categories).label("label").y("value").hole(0.6)
        self.assertEqual(donut.to_spec()["hole"], 0.6)
        self.assertRaisesRegex(ValueError, "increasing", px.contour().thresholds, [2, 1])
        self.assertRaisesRegex(ValueError, "unsupported", px.pie().opacity, 0.5)
        self.assertRaisesRegex(ValueError, "only supported", px.line().hole, 0.5)

    def test_line_curve_dash_and_point_visibility_are_typed_and_immutable(self) -> None:
        original = px.line("styled-line").data(self.events).x("frequency").y("spl")
        configured = (
            original.curve(px.CurveType.MONOTONE_X)
            .dash_style("dash_dot")
            .show_points()
        )

        self.assertNotIn("curve", original.to_spec())
        self.assertEqual(configured.to_spec()["curve"], "monotone_x")
        self.assertEqual(configured.to_spec()["dash_style"], "dash_dot")
        self.assertTrue(configured.to_spec()["show_points"])
        with self.assertRaisesRegex(ValueError, "dash_style"):
            original.dash_style("long_dash")
        with self.assertRaisesRegex(ValueError, "only supported by line"):
            px.scatter().curve(px.CurveType.LINEAR)
        with self.assertRaises(TypeError):
            original.show_points(1)

    def test_treemap_uses_keyed_hierarchy_resource_and_native_options(self) -> None:
        hierarchy = data.Dataset.from_mapping(
            {
                "id": ["root", "low", "high"],
                "parent": [None, "root", "root"],
                "value": [0.0, 2.0, 3.0],
            },
            key="id",
            id="hierarchy",
        )
        spec = (
            px.treemap("bands").data(hierarchy)
            .row_id("id").parent("parent").size("value")
            .tiling_method(px.TilingMethod.BINARY).padding(2.0)
            .colors(["#112233", "#abcdef"]).hover(False)
            .renderer_2d(px.Renderer2D.VELLO).vello_backend(px.VelloBackend.CPU)
            .fill().min_size(320.0, 240.0).aspect_ratio(1.5)
            .on_selection_change("band-selected")
            .to_spec()
        )
        self.assertEqual(spec["chart"], "treemap")
        self.assertEqual(spec["tiling_method"], "binary")
        self.assertEqual(spec["padding"], 2.0)
        self.assertEqual(spec["colors"], ["#112233", "#abcdef"])
        self.assertFalse(spec["hover"])
        self.assertEqual(spec["renderer_2d"], "vello")
        self.assertEqual(spec["vello_backend"], "cpu")
        self.assertTrue(spec["fill"])
        self.assertEqual(spec["min_width"], 320.0)
        self.assertEqual(spec["min_height"], 240.0)
        self.assertEqual(spec["aspect_ratio"], 1.5)
        self.assertNotIn("root", repr(spec["data"]["source"]))

        view_spec = (
            px.treemap("filtered-bands")
            .data(hierarchy.view().filter(data.col("value") >= 0.0))
            .row_id("id")
            .parent("parent")
            .size("value")
            .on_selection_change("filtered-band-selected")
            .to_spec()
        )
        self.assertEqual(view_spec["data"]["source"]["kind"], "dataset_view")
        self.assertNotIn("root", repr(view_spec["data"]["source"]))

        with self.assertRaisesRegex(ValueError, "missing roles"):
            px.treemap().data(hierarchy).row_id("id").size("value").to_spec()
        with self.assertRaisesRegex(ValueError, "primary key"):
            px.treemap().data(hierarchy).row_id("parent").parent("parent").size("value").on_selection_change("selected").to_spec()
        with self.assertRaises(TypeError):
            px.treemap().hover("yes")
        with self.assertRaises(ValueError):
            px.surface().renderer_2d(px.Renderer2D.LEGACY)

    def test_resource_grid_charts_bind_array_descriptors_without_values(self) -> None:
        grid = data.ArrayData.from_buffer(bytearray(range(12)), shape=(3, 4), dtype="u8", id="grid")
        for builder, kind in (
            (px.heatmap, "heatmap"),
            (px.contour, "contour"),
            (px.isoline, "isoline"),
            (px.surface, "surface"),
        ):
            spec = builder().data(grid).to_spec()
            self.assertEqual(spec["chart"], kind)
            self.assertEqual(spec["data"]["source"]["shape"], [3, 4])
            self.assertNotIn("values", repr(spec))
        surface_builder = (
            px.surface("camera-surface")
            .data(grid)
            .dimensions(720.0, 480.0)
            .on_viewport_change("camera")
        )
        surface = surface_builder.to_spec()
        self.assertEqual(surface["viewport_action"], "camera")
        self.assertEqual(surface["width"], 720.0)
        self.assertEqual(surface["height"], 480.0)
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            surface_builder.request_camera(SessionContext(), "query-camera")
            surface_builder.reset_camera(SessionContext(), "reset-camera")
        message = output.getvalue()
        self.assertIn('"command":"px.query_surface_camera"', message)
        self.assertIn('"command":"px.reset_surface_camera"', message)
        self.assertNotIn('"values"', message)
        camera = px.SurfaceCameraResult.from_command(
            CommandResult.from_wire(
                "query-camera",
                {
                    "ok": True,
                    "chart_id": "camera-surface",
                    "camera": {
                        "distance": 3.5,
                        "azimuth": 1.0,
                        "elevation": 0.5,
                        "target": [0.0, 0.25, -1.0],
                    },
                },
            )
        )
        self.assertEqual(camera.chart_id, "camera-surface")
        self.assertEqual(camera.target, (0.0, 0.25, -1.0))
        with self.assertRaises(ValueError):
            px.scatter().request_camera(SessionContext(), "query-camera")
        with self.assertRaises(ValueError):
            px.surface().dimensions(0.0, 480.0)
        scatter = (
            px.scatter("sized")
            .data(self.events)
            .x("frequency")
            .y("spl")
            .dimensions(800.0, 400.0)
            .min_size(320.0, 200.0)
            .to_spec()
        )
        self.assertEqual(scatter["width"], 800.0)
        self.assertEqual(scatter["height"], 400.0)
        self.assertEqual(scatter["min_width"], 320.0)
        self.assertEqual(scatter["min_height"], 200.0)

    def test_every_resource_chart_supports_responsive_sizing(self) -> None:
        grid = data.ArrayData.from_buffer(
            bytearray(range(12)), shape=(3, 4), dtype="u8", id="sizing-grid"
        )
        hierarchy = data.Dataset.from_mapping(
            {
                "id": ["root", "child"],
                "parent": [None, "root"],
                "value": [2.0, 2.0],
            },
            key="id",
            id="sizing-tree",
        )
        builders = [
            px.scatter().data(self.events).x("frequency").y("spl"),
            px.line().data(self.events).x("frequency").y("spl"),
            px.area().data(self.events).x("frequency").y("spl"),
            px.boxplot().data(self.events).x("frequency").y("spl"),
            px.bar().data(self.events).x("channel").y("spl"),
            px.pie().data(self.events).label("channel").y("spl"),
            px.donut().data(self.events).label("channel").y("spl"),
            px.heatmap().data(grid),
            px.contour().data(grid),
            px.isoline().data(grid),
            px.surface().data(grid),
            px.treemap().data(hierarchy).row_id("id").parent("parent").size("value"),
        ]
        for builder in builders:
            if builder.chart != "surface":
                builder = builder.renderer_2d(px.Renderer2D.LEGACY).vello_backend(
                    px.VelloBackend.CPU
                )
            fixed = builder.dimensions(700.0, 350.0).min_size(300.0, 180.0)
            spec = fixed.to_spec()
            self.assertEqual((spec["width"], spec["height"]), (700.0, 350.0))
            self.assertEqual((spec["min_width"], spec["min_height"]), (300.0, 180.0))
            if builder.chart != "surface":
                self.assertEqual(spec["renderer_2d"], "legacy")
                self.assertEqual(spec["vello_backend"], "cpu")
            filled = fixed.fill().to_spec()
            self.assertTrue(filled["fill"])
            self.assertNotIn("width", filled)
            self.assertNotIn("height", filled)

    def test_contour_and_isoline_sampling_controls_are_strict_and_immutable(self) -> None:
        grid = data.ArrayData.from_buffer(
            bytearray(range(12)), shape=(3, 4), dtype="u8", id="smooth-grid"
        )
        contour = px.contour("upsampled").data(grid)
        upsampled = contour.contour_upsample_factor(4)
        self.assertNotIn("contour_upsample_factor", contour.to_spec())
        self.assertEqual(upsampled.to_spec()["contour_upsample_factor"], 4)

        isoline = (
            px.isoline("smooth")
            .data(grid)
            .contour_upsample_factor(2)
            .smooth_strokes(True)
            .smoothing_iterations(3)
            .smoothing_max_deviation_px(1.25)
        ).to_spec()
        self.assertEqual(isoline["contour_upsample_factor"], 2)
        self.assertTrue(isoline["smooth_strokes"])
        self.assertEqual(isoline["smoothing_iterations"], 3)
        self.assertEqual(isoline["smoothing_max_deviation_px"], 1.25)

        with self.assertRaises(ValueError):
            contour.contour_upsample_factor(0)
        with self.assertRaises(ValueError):
            contour.smooth_strokes()
        with self.assertRaises(ValueError):
            px.isoline().smoothing_iterations(5)
        with self.assertRaises(ValueError):
            px.isoline().smoothing_max_deviation_px(float("nan"))

    def test_surface_axis_and_wireframe_controls_are_strict_and_immutable(self) -> None:
        grid = data.ArrayData.from_buffer(
            bytearray(range(12)), shape=(3, 4), dtype="u8", id="surface-style"
        )
        original = px.surface("terrain").data(grid)
        configured = (
            original.wireframe(True)
            .x_log(True)
            .y_log(True)
            .z_range(0.1, 12.0)
            .x_label("Longitude")
            .y_label("Latitude")
            .z_label("Elevation")
        )
        self.assertNotIn("wireframe", original.to_spec())
        self.assertTrue(configured.to_spec()["wireframe"])
        self.assertEqual(configured.to_spec()["z_range"], [0.1, 12.0])
        self.assertEqual(configured.to_spec()["z_label"], "Elevation")
        with self.assertRaises(ValueError):
            original.z_range(2.0, 1.0)
        with self.assertRaises(ValueError):
            px.line().wireframe()

    def test_dense_grid_axes_and_isoline_color_are_strict(self) -> None:
        grid = data.ArrayData.from_buffer(
            bytearray(range(12)), shape=(3, 4), dtype="u8", id="dense-axes"
        )
        original = px.isoline("levels").data(grid)
        configured = (
            original.x_log(True)
            .y_log(True)
            .x_range(1.0, 4.0)
            .y_range(1.0, 3.0)
            .stroke_color("#123456")
        )
        self.assertNotIn("stroke_color", original.to_spec())
        self.assertEqual(configured.to_spec()["stroke_color"], "#123456")
        self.assertEqual(configured.to_spec()["x_range"], [1.0, 4.0])
        self.assertTrue(configured.to_spec()["y_log"])
        with self.assertRaises(ValueError):
            px.heatmap().stroke_color("#123456")

    def test_bar_layout_controls_are_strict_and_immutable(self) -> None:
        categories = data.Dataset.from_mapping(
            {"category": ["A", "B"], "value": [1.0, 2.0]}, id="bar-layout"
        )
        original = px.bar("bars").data(categories).x("category").y("value")
        configured = original.bar_gap(7.5).border_radius(4.0)

        self.assertNotIn("bar_gap", original.to_spec())
        self.assertNotIn("border_radius", original.to_spec())
        self.assertEqual(configured.to_spec()["bar_gap"], 7.5)
        self.assertEqual(configured.to_spec()["border_radius"], 4.0)

        with self.assertRaises(ValueError):
            original.bar_gap(-1.0)
        with self.assertRaises(ValueError):
            original.border_radius(float("inf"))
        with self.assertRaises(ValueError):
            px.line().bar_gap(2.0)

    def test_box_plot_presentation_controls_are_strict_and_immutable(self) -> None:
        points = data.Dataset.from_mapping(
            {"x": [0.0, 1.0, 2.0], "y": [1.0, 3.0, 2.0]}, id="box-style"
        )
        original = px.boxplot("distribution").data(points).x("x").y("y")
        configured = (
            original
            .box_color("#abcdef")
            .median_color("#112233")
            .whisker_color("445566")
            .outlier_color("#778899")
            .box_opacity(0.75)
            .box_width(24.0)
            .outlier_radius(3.5)
            .bins(8)
        )

        self.assertNotIn("box_color", original.to_spec())
        self.assertEqual(configured.to_spec()["box_color"], "#abcdef")
        self.assertEqual(configured.to_spec()["median_color"], "#112233")
        self.assertEqual(configured.to_spec()["whisker_color"], "#445566")
        self.assertEqual(configured.to_spec()["outlier_color"], "#778899")
        self.assertEqual(configured.to_spec()["box_opacity"], 0.75)
        self.assertEqual(configured.to_spec()["box_width"], 24.0)
        self.assertEqual(configured.to_spec()["outlier_radius"], 3.5)
        self.assertEqual(configured.to_spec()["bins"], 8)

        with self.assertRaises(ValueError):
            original.median_color("red")
        with self.assertRaises(ValueError):
            original.box_opacity(1.1)
        with self.assertRaises(ValueError):
            original.box_width(0.0)
        with self.assertRaises(ValueError):
            original.bins(0)
        with self.assertRaises(ValueError):
            px.bar().box_color("#abcdef")

    def test_pie_presentation_controls_are_strict_and_immutable(self) -> None:
        categories = data.Dataset.from_mapping(
            {"label": ["A", "B"], "value": [2.0, 3.0]}, id="pie-style"
        )
        original = px.pie("share").data(categories).label("label").y("value")
        configured = (
            original.colors(["#112233", "445566"])
            .pad_angle(0.02)
            .corner_radius(3.0)
            .sort(False)
        )
        self.assertNotIn("colors", original.to_spec())
        self.assertEqual(configured.to_spec()["colors"], ["#112233", "#445566"])
        self.assertEqual(configured.to_spec()["pad_angle"], 0.02)
        self.assertEqual(configured.to_spec()["corner_radius"], 3.0)
        self.assertFalse(configured.to_spec()["sort"])
        with self.assertRaises(ValueError):
            original.colors([])
        with self.assertRaises(ValueError):
            original.pad_angle(-0.1)
        with self.assertRaises(TypeError):
            original.sort(1)
        with self.assertRaises(ValueError):
            px.bar().corner_radius(2.0)

    def test_area_presentation_controls_are_strict_and_immutable(self) -> None:
        points = data.Dataset.from_mapping(
            {"x": [1.0, 2.0], "y": [2.0, 4.0]}, id="area-style"
        )
        original = px.area("filled").data(points).x("x").y("y")
        configured = (
            original.fill_color("#336699")
            .curve(px.CurveType.NATURAL)
            .x_log(True)
            .y_log(True)
        )
        self.assertNotIn("fill_color", original.to_spec())
        self.assertEqual(configured.to_spec()["fill_color"], "#336699")
        self.assertEqual(configured.to_spec()["curve"], "natural")
        self.assertTrue(configured.to_spec()["x_log"])
        self.assertTrue(configured.to_spec()["y_log"])
        with self.assertRaises(ValueError):
            px.line().fill_color("#336699")
        with self.assertRaises(ValueError):
            px.scatter().curve(px.CurveType.LINEAR)

    def test_primary_series_color_is_distinct_from_color_role(self) -> None:
        points = data.Dataset.from_mapping(
            {"x": [1.0], "y": [2.0], "group": ["A"]}, id="primary-color"
        )
        original = px.scatter("points").data(points).x("x").y("y").color("group")
        configured = original.primary_color("#abcdef")
        self.assertEqual(original.to_spec()["data"]["roles"]["color"], "group")
        self.assertNotIn("primary_color", original.to_spec())
        self.assertEqual(configured.to_spec()["primary_color"], "#abcdef")
        self.assertEqual(
            px.bar().data(points).x("x").y("y").primary_color("#123456").to_spec()[
                "primary_color"
            ],
            "#123456",
        )
        with self.assertRaises(ValueError):
            px.area().primary_color("#abcdef")

    def test_resource_scalar_charts_use_typed_color_configuration(self) -> None:
        grid = data.ArrayData.from_buffer(
            bytearray(range(12)), shape=(3, 4), dtype="u8", id="color-grid"
        )
        original = px.surface("surface-colors").data(grid)
        configured = original.color_scale(px.ColorScale.INFERNO)

        self.assertNotIn("color_scale", original.to_spec())
        spec = configured.to_spec()
        self.assertEqual(spec["color_scale"], "inferno")
        self.assertNotIn("values", repr(spec))
        self.assertRaises(ValueError, px.scatter().color_scale, px.ColorScale.HEAT)
        self.assertRaises(ValueError, px.surface().color_scale, px.ColorScale.HEAT)
        self.assertRaises(px.ColorRangeError, px.ColorRange.fixed, 1.0, 1.0)

    def test_px_mesh_builder_reuses_revisioned_mesh_resources(self) -> None:
        from gpui_toolkit.resources import ResourceStore

        store = ResourceStore()
        positions = store.put_mesh_array(
            "positions", [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            shape=(3, 3), dtype="f64le",
        )
        triangles = store.put_mesh_array(
            "triangles", [[0, 1, 2]], shape=(1, 3), dtype="u32le",
        )
        values = store.put_mesh_array(
            "values", [1.0, 2.0, 3.0], shape=(3,), dtype="f64le",
        )
        geometry = px.mesh_geometry(positions, triangles, id="triangle")
        field = px.mesh_field(values, label="Pressure", unit="Pa")
        spec = px.mesh_plot(
            geometry, field, id="pressure", mode="scalar_fill", title="Pressure field",
        ).to_spec()
        self.assertEqual(spec["kind"], "mesh_plot")
        self.assertEqual(spec["geometry"]["positions"]["resource_id"], "positions")
        self.assertEqual(spec["field"]["resource_id"], "values")
        self.assertNotIn("1.0", repr(spec["geometry"]))

        base_builder = px.mesh("pressure").geometry(geometry).field(field)
        builder = (
            base_builder
            .mode("scalar_fill")
            .title("Pressure field")
            .on_selection_change("pressure-selected")
            .on_export("pressure-exported")
        )
        self.assertNotEqual(base_builder.to_spec()["mode"], builder.to_spec()["mode"])
        self.assertEqual(builder.to_spec(), spec)
        node = ui.mesh_plot(builder).to_spec()
        self.assertEqual(node["selection_action"], "pressure-selected")
        self.assertEqual(node["export_action"], "pressure-exported")
        with self.assertRaises(ValueError):
            builder.on_export("")

        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            builder.request_svg_export(
                SessionContext(), "export-pressure", width=480, height=320
            )
        message = output.getvalue()
        self.assertIn('"command":"px.export_mesh_svg"', message)
        self.assertIn('"resource_id":"positions"', message)
        self.assertIn('"options":{"width":480.0,"height":320.0}', message)
        self.assertNotIn("[[0.0", message)

        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            builder.request_accessibility_summary(SessionContext(), "summarize-pressure")
        message = output.getvalue()
        self.assertIn('"command":"px.mesh_accessibility_summary"', message)
        self.assertIn('"resource_id":"positions"', message)
        self.assertNotIn("[[0.0", message)

        summary = px.ChartAccessibilitySummary.from_command(
            CommandResult.from_wire(
                "summarize-pressure",
                {
                    "ok": True,
                    "chart_id": "pressure",
                    "summary": {
                        "chart_type": "mesh_plot",
                        "title": "Pressure field",
                        "series_count": 1,
                        "datum_count": 3,
                        "x_range": [0.0, 1.0],
                        "y_range": [0.0, 1.0],
                        "value_range": [1.0, 3.0],
                        "x_scale": "linear",
                        "y_scale": "linear",
                        "series_labels": ["Pressure"],
                        "description": "Mesh accessibility description",
                        "accessible_label": "Pressure field",
                        "accessible_value_text": "1 series; 3 data points",
                    },
                },
            )
        )
        self.assertEqual(summary.chart_type, "mesh_plot")
        self.assertEqual(summary.value_range, (1.0, 3.0))
        self.assertEqual(summary.series_labels, ("Pressure",))

    def test_px_mesh_builder_accepts_arraydata_without_inline_values(self) -> None:
        positions = data.ArrayData.from_buffer(
            bytearray(3 * 3 * 8), shape=(3, 3), dtype="f64", id="array-positions"
        )
        triangles = data.ArrayData.from_buffer(
            bytearray(3 * 4), shape=(1, 3), dtype="u32", id="array-triangles"
        )
        field_values = data.ArrayData.from_buffer(
            bytearray(3 * 8), shape=(3,), dtype="f64", id="array-field"
        )
        valid = data.ArrayData.from_buffer(
            bytearray([1, 1, 1]), shape=(3,), dtype="bool", id="array-valid"
        )
        default_colorbar = px.MeshColorbar("Amplitude")
        colorbar = (
            default_colorbar.unit("dB")
            .scale(px.ColorScale.COOLWARM)
            .range(px.ColorRange.fixed(-1.0, 1.0))
            .ticks([-1.0, 0.0, 1.0])
            .orientation(px.ColorbarOrientation.HORIZONTAL)
        )
        spec = (
            px.mesh("array-mesh")
            .geometry(px.mesh_geometry(positions, triangles))
            .field(px.mesh_field(field_values, valid=valid))
            .mode("scalar_fill")
            .toolbar(False)
            .toolbar_action_hidden("export")
            .toolbar_action_hidden("open_view_menu")
            .colorbar(colorbar)
            .renderer_backend(px.MeshPlotBackend.WGPU)
            .fill()
            .min_size(320.0, 240.0)
            .aspect_ratio(1.5)
            .to_spec()
        )
        self.assertEqual(spec["geometry"]["positions"]["kind"], "array_data")
        self.assertEqual(spec["geometry"]["positions"]["shape"], [3, 3])
        self.assertEqual(spec["geometry"]["triangles"]["dtype"], "u32")
        self.assertEqual(spec["field"]["kind"], "array_data")
        self.assertEqual(spec["field"]["valid"]["shape"], [3])
        self.assertNotIn("values", repr(spec))
        self.assertFalse(spec["toolbar"])
        self.assertEqual(spec["hidden_toolbar_actions"], ["export", "open_view_menu"])
        self.assertEqual(default_colorbar.to_spec()["scale"], "viridis")
        self.assertEqual(spec["colorbar"]["label"], "Amplitude")
        self.assertEqual(spec["colorbar"]["unit"], "dB")
        self.assertEqual(spec["colorbar"]["range"], [-1.0, 1.0])
        self.assertEqual(spec["colorbar"]["ticks"], [-1.0, 0.0, 1.0])
        self.assertEqual(spec["colorbar"]["orientation"], "horizontal")
        self.assertEqual(spec["renderer_backend"], "wgpu")
        self.assertTrue(spec["fill"])
        self.assertEqual(spec["min_width"], 320.0)
        self.assertEqual(spec["min_height"], 240.0)
        self.assertEqual(spec["aspect_ratio"], 1.5)

        with self.assertRaises(ValueError):
            px.mesh().size(320.0, None)
        with self.assertRaises(ValueError):
            px.mesh().min_size(0.0, 240.0)
        with self.assertRaises(ValueError):
            px.mesh().renderer_backend("metal")

        with self.assertRaises(ValueError):
            px.mesh().geometry(px.mesh_geometry(positions, triangles)).toolbar_action_hidden(
                "unsupported"
            ).to_spec()

    def test_svg_export_request_uses_resource_descriptor_and_typed_result(self) -> None:
        from gpui_toolkit.commands import CommandResult

        chart = px.scatter("response").data(self.events).x("frequency").y("spl")
        defaults = px.StaticSvgOptions.new(640, 320)
        options = (
            defaults
            .margins(left=20, right=10, top=12, bottom=18)
            .background(None)
            .show_axes(False)
        )
        self.assertEqual(defaults.to_spec(), {"width": 640.0, "height": 320.0})
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            chart.request_svg_export(
                SessionContext(), "export-response", options=options
            )
        message = output.getvalue()
        self.assertIn('"command":"px.export_svg"', message)
        self.assertIn('"id":"events"', message)
        self.assertIn('"margin_left":20.0', message)
        self.assertIn('"background":null', message)
        self.assertIn('"show_axes":false', message)
        self.assertNotIn('"values"', message)
        with self.assertRaises(ValueError):
            chart.request_svg_export(
                SessionContext(), "ambiguous-export", options=options, width=400
            )

        view_chart = (
            px.line("filtered-response")
            .data(self.events.view().filter(data.col("enabled")).range(0, 1))
            .x("frequency")
            .y("spl")
        )
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            view_chart.request_svg_export(SessionContext(), "export-filtered")
        view_message = output.getvalue()
        self.assertIn('"kind":"dataset_view"', view_message)
        self.assertIn('"op":"filter"', view_message)
        self.assertIn('"op":"range"', view_message)
        self.assertNotIn('"values"', view_message)

        result = px.SvgExportResult.from_command(CommandResult.from_wire(
            "export-response",
            {"ok": True, "chart_id": "response", "svg": "<svg></svg>"},
        ))
        self.assertEqual(result.chart_id, "response")
        self.assertEqual(result.svg, "<svg></svg>")

        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            chart.request_accessibility_summary(SessionContext(), "summarize-response")
        message = output.getvalue()
        self.assertIn('"command":"px.chart_accessibility_summary"', message)
        self.assertIn('"chart":"scatter"', message)
        self.assertNotIn('"values"', message)

        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            chart.request_metadata(SessionContext(), "metadata-response")
        message = output.getvalue()
        self.assertIn('"command":"px.chart_metadata"', message)
        self.assertNotIn('"values"', message)

        metadata = px.ChartMetadataResult.from_command(
            CommandResult.from_wire(
                "metadata-response",
                {
                    "ok": True,
                    "chart_id": "response",
                    "accessibility": {
                        "chart_type": "scatter",
                        "title": None,
                        "series_count": 1,
                        "datum_count": 4,
                        "x_range": [100.0, 400.0],
                        "y_range": [70.0, 73.0],
                        "value_range": None,
                        "x_scale": "linear",
                        "y_scale": "linear",
                        "series_labels": ["response"],
                        "description": "Scatter summary",
                        "accessible_label": "scatter chart",
                        "accessible_value_text": "1 series; 4 data points",
                    },
                    "legend": {
                        "chart_type": "scatter",
                        "visible": True,
                        "position": "right",
                        "position_explicit": False,
                        "item_count": 1,
                        "items": [{
                            "series_index": 0,
                            "label": "response",
                            "color": 0x1F77B4,
                            "marker": "circle",
                            "hidden": False,
                            "uses_secondary_axis": False,
                        }],
                        "description": "scatter legend is visible with 1 entry",
                    },
                    "annotations": {
                        "chart_type": "scatter",
                        "annotation_count": 1,
                        "annotations": [{
                            "id": "peak",
                            "label": "Peak",
                            "target": {"kind": "point", "x": 200.0, "y": 73.0},
                            "color": 0xFF0000,
                            "series_index": 0,
                        }],
                        "description": "scatter chart has 1 annotation",
                    },
                },
            )
        )
        self.assertEqual(metadata.legend.items[0].marker, "circle")
        self.assertEqual(metadata.annotations.annotations[0].target.x, 200.0)
        self.assertEqual(metadata.accessibility.datum_count, 4)
        with self.assertRaises(ValueError):
            px.area("area").request_metadata(SessionContext(), "metadata-area")

    def test_resource_chart_viewport_query_is_typed_and_requires_retained_state(self) -> None:
        chart = (
            px.line("response")
            .data(self.events)
            .x("frequency")
            .y("spl")
            .on_viewport_change("response-viewport")
        )
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            chart.request_viewport(SessionContext(), "query-response")
        self.assertIn('"command":"px.query_viewport"', output.getvalue())
        self.assertIn('"chart_id":"response"', output.getvalue())

        result = px.ChartViewportResult.from_command(
            CommandResult.from_wire(
                "query-response",
                {
                    "ok": True,
                    "chart_id": "response",
                    "x_domain": [20.0, 20_000.0],
                    "y_domain": [40.0, 120.0],
                    "zoom_level": 2,
                    "is_zoomed": True,
                },
            )
        )
        self.assertEqual(result.x_domain, (20.0, 20_000.0))
        self.assertEqual(result.zoom_level, 2)
        self.assertTrue(result.is_zoomed)

        with self.assertRaises(ValueError):
            px.line("not-interactive").request_viewport(SessionContext(), "query")
        with self.assertRaises(ValueError):
            px.bar("bar").request_viewport(SessionContext(), "query")

    def test_reusable_data_binding_covers_interaction_and_accessibility_roles(self) -> None:
        binding = (
            self.events.binding()
            .x("frequency").y("spl").color("channel").series("channel")
            .row_id("event_id").tooltip("spl").accessibility_description("channel")
        )
        self.assertEqual(binding.to_spec()["roles"]["row_id"], "event_id")
        self.assertEqual(binding.to_spec()["roles"]["accessibility_description"], "channel")

    def test_unset_is_omitted_but_explicit_none_is_preserved(self) -> None:
        base = px.scatter().data(self.events).x("frequency").y("spl")
        self.assertNotIn("title", base.to_spec())
        self.assertIn("title", base.title(None).to_spec())

    def test_table_uses_resource_descriptor_and_virtualization(self) -> None:
        table = (
            ui.Table("events")
            .data(self.events)
            .column(ui.Column("frequency").field("frequency").template(data.field("frequency")).sortable())
            .selection_mode(ui.SelectionMode.MULTIPLE)
            .virtualize(row_height=28, overscan=12)
            .on_selection_change("events-selected")
        )
        spec = table.to_spec()
        self.assertEqual(spec["kind"], "table_v2")
        self.assertEqual(spec["data"]["id"], "events")
        self.assertEqual(spec["virtualize"], {"row_height": 28.0, "overscan": 12})
        self.assertEqual(spec["columns"][0]["template"], {"kind": "field_ref", "field": "frequency"})

    def test_table_rejects_unknown_bound_column_field(self) -> None:
        table = ui.Table("events").data(self.events).column(ui.Column("bad").field("missing"))
        with self.assertRaises(data.SchemaError):
            table.to_spec()

    def test_table_has_a_bounded_virtual_window_by_default(self) -> None:
        spec = ui.Table("events").data(self.events).column(ui.Column("frequency").field("frequency")).to_spec()
        self.assertEqual(spec["virtualize"], {"row_height": 28.0, "overscan": 8})

    def test_table_selection_requires_stable_dataset_key(self) -> None:
        unkeyed = data.Dataset.from_mapping({"frequency": [20.0]}, id="unkeyed")
        table = ui.Table("unkeyed").data(unkeyed).column(ui.Column("frequency").field("frequency")).selection_mode(ui.SelectionMode.SINGLE)
        with self.assertRaisesRegex(ValueError, "primary key"):
            table.to_spec()

    def test_million_row_table_declaration_is_bounded_and_virtualized(self) -> None:
        million = data.Dataset.from_mapping({"id": range(1_000_000), "value": range(1_000_000)}, key="id", id="million")
        spec = ui.Table("million").data(million).column(ui.Column("value").field("value")).to_spec()
        encoded = repr(spec)
        self.assertEqual(spec["data"]["row_count"], 1_000_000)
        self.assertLess(len(encoded), 1_000)
        self.assertNotIn("999999", encoded)

    def test_multi_million_point_chart_declaration_is_lod_bound(self) -> None:
        points = data.Dataset.from_mapping({"x": range(2_000_000), "y": range(2_000_000)}, id="points")
        spec = px.scatter("points").data(points).x("x").y("y").lod(px.Lod.AUTO).to_spec()
        encoded = repr(spec)
        self.assertEqual(spec["lod"], "auto")
        self.assertEqual(spec["data"]["source"]["row_count"], 2_000_000)
        self.assertLess(len(encoded), 1_000)
        self.assertNotIn("1999999", encoded)
        aggressive = px.scatter("aggressive").data(points).x("x").y("y").lod(px.Lod.AGGRESSIVE).to_spec()
        self.assertEqual(aggressive["lod"], "aggressive")

    def test_chart_viewport_and_accessibility_are_named_declarations(self) -> None:
        spec = (px.scatter("response").data(self.events).x("frequency").y("spl")
                .on_viewport_change("viewport")
                .accessibility_description("Frequency response").to_spec())
        self.assertEqual(spec["viewport_action"], "viewport")
        self.assertEqual(spec["accessibility_description"], "Frequency response")

    def test_chart_selection_requires_keyed_tabular_data(self) -> None:
        unkeyed = data.Dataset.from_mapping(
            {"id": ["root"], "parent": [None], "value": [1.0]}, id="unkeyed-chart",
        )
        with self.assertRaisesRegex(ValueError, "primary key"):
            (px.treemap().data(unkeyed).row_id("id").parent("parent").size("value")
             .on_selection_change("selected").to_spec())

        scatter = (
            px.scatter("selectable-response")
            .data(self.events)
            .x("frequency")
            .y("spl")
            .row_id("event_id")
            .on_selection_change("point-selected")
            .to_spec()
        )
        self.assertEqual(scatter["selection_action"], "point-selected")
        self.assertEqual(scatter["data"]["roles"]["row_id"], "event_id")
        with self.assertRaisesRegex(ValueError, "row_id to match"):
            (
                px.line("bad-selection")
                .data(self.events)
                .x("frequency")
                .y("spl")
                .row_id("channel")
                .on_selection_change("point-selected")
                .to_spec()
            )

    def test_mutation_is_revisioned_and_keyed(self) -> None:
        self.events.append({"event_id": [3], "enabled": [True], "frequency": [1000.0], "spl": [3.0], "channel": ["L"]})
        self.assertEqual(self.events.generation, 2)
        self.events.upsert({"event_id": [2], "enabled": [True], "frequency": [200.0], "spl": [4.0], "channel": ["R"]})
        self.assertEqual(self.events.generation, 3)
        self.events.delete([1])
        self.assertEqual(self.events.row_count, 2)
        with self.assertRaises(data.SchemaError):
            self.events.append({"event_id": [2], "enabled": [True], "frequency": [1.0], "spl": [1.0], "channel": ["L"]})

    def test_schema_changes_require_explicit_migration(self) -> None:
        with self.assertRaises(data.SchemaError):
            self.events.replace({"event_id": [3], "value": [1.0]})
        self.events.migrate({"event_id": [3], "value": [1.0]}, key="event_id")
        self.assertEqual(self.events.schema, ("event_id", "value"))
        self.assertEqual(self.events.generation, 2)

    def test_transaction_commits_once_and_array_never_serializes_payload(self) -> None:
        with self.events.transaction() as transaction:
            transaction.append({"event_id": [3], "enabled": [True], "frequency": [1000.0], "spl": [3.0], "channel": ["L"]})
            transaction.upsert({"event_id": [2], "enabled": [True], "frequency": [200.0], "spl": [4.0], "channel": ["R"]})
            transaction.delete([1])
        self.assertEqual(self.events.generation, 2)
        self.assertEqual(self.events.row_count, 2)
        array = data.ArrayData.from_buffer(bytearray(b"abcd"), shape=(4,), dtype="u8", id="spectrum")
        spec = array.to_spec()
        self.assertEqual(
            {key: value for key, value in spec.items() if key != "schema_fingerprint"},
            {"kind": "array_data", "id": "spectrum", "generation": 1, "shape": [4], "dtype": "u8", "byte_length": 4},
        )
        self.assertEqual(len(spec["schema_fingerprint"]), 64)
        self.assertEqual(array.binary_chunks(max_bytes=2), (b"ab", b"cd"))

    def test_resource_descriptor_is_small_and_excludes_values(self) -> None:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            SessionContext().publish_resource(self.events)
        wire = output.getvalue()
        self.assertIn('"type":"resource_descriptor"', wire)
        self.assertIn('"resource_id":"events"', wire)
        self.assertIn('"schema_version":2', wire)
        self.assertIn('"column_types"', wire)
        self.assertNotIn("20.0", wire)

    def test_resource_capabilities_are_advertised(self) -> None:
        self.assertTrue({"datasets", "array_resources"}.issubset(PYTHON_SESSION_CAPABILITIES))

    def test_dataset_binary_publication_keeps_values_out_of_control_json(self) -> None:
        class BinaryStdout(io.StringIO):
            def __init__(self) -> None:
                super().__init__()
                self.buffer = io.BytesIO()

        class PublishedDataset:
            id = "events"
            generation = 7
            schema_fingerprint = "schema-v7"
            def to_spec(self):
                return {"kind": "dataset", "id": self.id, "generation": self.generation,
                        "schema_fingerprint": self.schema_fingerprint, "schema": ["event_id"],
                        "column_types": {"event_id": "int"}}
            def arrow_ipc_chunks(self, *, max_bytes: int):
                self.max_bytes = max_bytes
                return (b"arrow-", b"bytes")
            def subscribe(self, callback):
                self.callback = callback
                return lambda: None

        stream = ArrayBinaryStdout()
        dataset = PublishedDataset()
        with patch("gpui_toolkit.app.sys.stdout", stream):
            SessionContext().bind_dataset(dataset, max_frame_bytes=8)
            dataset.generation = 8
            dataset.callback(dataset)
        self.assertIn('"type":"resource_descriptor"', stream.getvalue())
        self.assertEqual(stream.getvalue().count('"type":"resource_descriptor"'), 2)
        frames = stream.buffer.getvalue()
        self.assertIn(b'"type":"dataset_frame"', frames)
        self.assertIn(b"arrow-", frames)
        self.assertIn(b"bytes", frames)
        self.assertNotIn(b"arrow-bytes", stream.getvalue().encode())
        self.assertEqual(dataset.max_bytes, 8)

    def test_live_dataset_binding_uses_native_or_requires_an_adapter(self) -> None:
        real_import = builtins.__import__

        def without_pyarrow(name, *args, **kwargs):
            if name == "pyarrow":
                raise ImportError("no pyarrow")
            return real_import(name, *args, **kwargs)

        with patch("builtins.__import__", side_effect=without_pyarrow):
            if native.AVAILABLE:
                stream = ArrayBinaryStdout()
                with patch("gpui_toolkit.app.sys.stdout", stream):
                    SessionContext().bind_dataset(self.events)
                self.assertIn(b'"type":"dataset_frame"', stream.buffer.getvalue())
            else:
                # A descriptor alone cannot claim a live binding because the
                # host would never receive batches.
                with contextlib.redirect_stdout(io.StringIO()):
                    with self.assertRaises(data.DataTransportError):
                        SessionContext().bind_dataset(self.events)

    def test_bound_resource_republishes_generation_without_ui_patch(self) -> None:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            unsubscribe = SessionContext().bind_resource(self.events)
            self.events.append({"event_id": [3], "enabled": [True], "frequency": [1.0], "spl": [1.0], "channel": ["L"]})
            unsubscribe()
            self.events.append({"event_id": [4], "enabled": [True], "frequency": [2.0], "spl": [2.0], "channel": ["R"]})
        wire = output.getvalue().splitlines()
        self.assertEqual(len(wire), 2)
        self.assertIn('"generation":2', wire[1])
        self.assertNotIn('"type":"patch"', output.getvalue())

    def test_array_binding_uses_binary_frames_not_json_values(self) -> None:
        stream = ArrayBinaryStdout()
        array = data.ArrayData.from_buffer(bytearray(b"abcd"), shape=(2, 2), dtype="u8", id="points")
        with patch("gpui_toolkit.app.sys.stdout", stream):
            SessionContext().bind_array(array, max_frame_bytes=2)
        self.assertIn('"resource_kind":"array_data"', stream.getvalue())
        self.assertIn(b'"type":"dataset_frame"', stream.buffer.getvalue())
        self.assertIn(b"ab", stream.buffer.getvalue())
        self.assertIn(b"cd", stream.buffer.getvalue())
        self.assertNotIn(b"abcd", stream.getvalue().encode())

    def test_array_binding_prefers_session_mmap_and_cleans_up_after_ack(self) -> None:
        output = io.StringIO()
        array = data.ArrayData.from_buffer(
            bytearray(b"abcd"), shape=(2, 2), dtype="u8", id="mmap-points"
        )
        with tempfile.TemporaryDirectory() as directory:
            os.chmod(directory, 0o700)
            context = SessionContext(
                capabilities=("resource_mmap_frames",),
                resource_directory=directory,
                resource_token="session-secret",
            )
            with contextlib.redirect_stdout(output):
                context.publish_array(array, max_frame_bytes=2)
            messages = [json.loads(line) for line in output.getvalue().splitlines()]
            self.assertEqual(messages[0]["type"], "resource_descriptor")
            frame = messages[1]
            self.assertEqual(frame["type"], "mapped_dataset_frame")
            self.assertEqual(frame["chunk_count"], 1)
            self.assertEqual(frame["byte_length"], 4)
            self.assertEqual(frame["session_token"], "session-secret")
            publication = os.path.join(directory, frame["filename"])
            with open(publication, "rb") as stream:
                self.assertEqual(stream.read(), b"abcd")
            self.assertEqual(os.stat(publication).st_mode & 0o777, 0o600)
            self.assertEqual(context.outstanding_resource_bytes, 4)
            context._acknowledge_resource_frame({
                "resource_id": "mmap-points",
                "generation": 1,
                "sequence": 0,
                "byte_length": 4,
                "complete": True,
                "accepted": True,
                "error": None,
            })
            self.assertFalse(os.path.exists(publication))
            self.assertEqual(context.outstanding_resource_bytes, 0)

    def test_mmap_backpressure_does_not_create_publication_file(self) -> None:
        array = data.ArrayData.from_buffer(b"abcd", shape=(4,), dtype="u8", id="limited")
        with tempfile.TemporaryDirectory() as directory:
            context = SessionContext(
                max_outstanding_resource_bytes=3,
                capabilities=("resource_mmap_frames",),
                resource_directory=directory,
                resource_token="session-secret",
            )
            with contextlib.redirect_stdout(io.StringIO()):
                with self.assertRaises(ResourceBackpressureError):
                    context.publish_array(array)
            self.assertEqual(os.listdir(directory), [])
            self.assertEqual(context.outstanding_resource_bytes, 0)

    def test_negotiated_mmap_transport_requires_host_credentials(self) -> None:
        with patch.dict(os.environ, {}, clear=False):
            os.environ.pop("GPUI_TOOLKIT_RESOURCE_DIR", None)
            os.environ.pop("GPUI_TOOLKIT_RESOURCE_TOKEN", None)
            with self.assertRaisesRegex(RuntimeError, "without session credentials"):
                SessionContext(capabilities=("resource_mmap_frames",))

    def test_resource_frame_backpressure_and_acknowledgement_release_bytes(self) -> None:
        stream = ArrayBinaryStdout()
        context = SessionContext(max_outstanding_resource_bytes=3)
        with patch("gpui_toolkit.app.sys.stdout", stream):
            context.dataset_frame(
                resource_id="events",
                generation=1,
                sequence=0,
                chunk_count=2,
                schema_fingerprint="schema-v1",
                payload=b"ab",
            )
            self.assertEqual(context.outstanding_resource_bytes, 2)
            with self.assertRaises(ResourceBackpressureError):
                context.dataset_frame(
                    resource_id="events",
                    generation=1,
                    sequence=1,
                    chunk_count=2,
                    schema_fingerprint="schema-v1",
                    payload=b"cd",
                )

            acknowledgement = context._acknowledge_resource_frame({
                "resource_id": "events",
                "generation": 1,
                "sequence": 0,
                "byte_length": 2,
                "complete": False,
                "accepted": True,
                "error": None,
            })
            self.assertIsInstance(acknowledgement, ResourceFrameAcknowledgement)
            self.assertEqual(context.outstanding_resource_bytes, 0)

            context.dataset_frame(
                resource_id="events",
                generation=1,
                sequence=1,
                chunk_count=2,
                schema_fingerprint="schema-v1",
                payload=b"cd",
            )
        rejected = context._acknowledge_resource_frame({
            "resource_id": "events",
            "generation": 1,
            "sequence": 1,
            "byte_length": 2,
            "complete": False,
            "accepted": False,
            "error": "checksum mismatch",
        })
        self.assertFalse(rejected.accepted)
        self.assertEqual(context.outstanding_resource_bytes, 0)
        self.assertEqual(context.resource_frame_rejections, (rejected,))
        self.assertNotIn(b"cd", stream.getvalue().encode())

    def test_resource_frame_acknowledgement_rejects_mismatch_and_duplicates(self) -> None:
        stream = ArrayBinaryStdout()
        context = SessionContext(max_outstanding_resource_bytes=8)
        with patch("gpui_toolkit.app.sys.stdout", stream):
            context.dataset_frame(
                resource_id="array",
                generation=2,
                sequence=0,
                chunk_count=1,
                schema_fingerprint="schema-v2",
                payload=b"1234",
            )
        wire = {
            "resource_id": "array",
            "generation": 2,
            "sequence": 0,
            "byte_length": 3,
            "complete": True,
            "accepted": True,
            "error": None,
        }
        with self.assertRaisesRegex(ValueError, "byte_length mismatch"):
            context._acknowledge_resource_frame(wire)
        self.assertEqual(context.outstanding_resource_bytes, 4)

        wire["byte_length"] = 4
        context._acknowledge_resource_frame(wire)
        with self.assertRaisesRegex(ValueError, "does not match an outstanding frame"):
            context._acknowledge_resource_frame(wire)
        self.assertEqual(context.outstanding_resource_bytes, 0)

    def test_resource_frame_acknowledgement_is_strictly_typed(self) -> None:
        with self.assertRaisesRegex(ValueError, "boolean complete"):
            ResourceFrameAcknowledgement.from_wire({
                "resource_id": "events",
                "generation": 1,
                "sequence": 0,
                "byte_length": 1,
                "complete": 1,
                "accepted": True,
                "error": None,
            })
        with self.assertRaisesRegex(ValueError, "requires an error"):
            ResourceFrameAcknowledgement.from_wire({
                "resource_id": "events",
                "generation": 1,
                "sequence": 0,
                "byte_length": 1,
                "complete": False,
                "accepted": False,
                "error": None,
            })

    def test_app_session_consumes_resource_frame_acknowledgements(self) -> None:
        class AcknowledgingApp(App):
            context: SessionContext | None = None
            outstanding_at_shutdown: int | None = None

            def on_session_ready(self, context: SessionContext) -> None:
                self.context = context
                context.dataset_frame(
                    resource_id="events",
                    generation=1,
                    sequence=0,
                    chunk_count=1,
                    schema_fingerprint="schema-v1",
                    payload=b"ab",
                )

            def on_session_shutdown(self, context: SessionContext) -> None:
                self.outstanding_at_shutdown = context.outstanding_resource_bytes

        app = AcknowledgingApp(sections=(Section("main", "Main", {}),))
        stream = ArrayBinaryStdout()
        acknowledgement = {
            "type": "resource_frame_result",
            "resource_id": "events",
            "generation": 1,
            "sequence": 0,
            "byte_length": 2,
            "complete": True,
            "accepted": True,
            "error": None,
        }
        initialize = {
            "type": "initialize",
            "session_version": 1,
            "capabilities": ["resource_frame_ack"],
        }
        with (
            patch("gpui_toolkit.app.sys.stdout", stream),
            patch("gpui_toolkit.app._read_message", return_value=initialize),
            patch(
                "gpui_toolkit.app._messages",
                return_value=iter((acknowledgement, {"type": "shutdown"})),
            ),
        ):
            app.serve()
        self.assertIsNotNone(app.context)
        self.assertEqual(app.outstanding_at_shutdown, 0)
        self.assertIn(b'"type":"dataset_frame"', stream.buffer.getvalue())

    def test_resources_support_deterministic_context_cleanup(self) -> None:
        with data.Dataset.from_mapping({"id": [1]}, id="temporary") as dataset:
            self.assertEqual(dataset.row_count, 1)
        with self.assertRaises(data.ClosedResourceError):
            dataset.to_spec()
        with data.ArrayData.from_buffer(bytearray(b"x"), shape=(1,), dtype="u8") as array:
            self.assertEqual(array.to_spec()["byte_length"], 1)
        with self.assertRaises(data.ClosedResourceError):
            array.to_spec()

    def test_optional_dataframe_and_array_adapters_do_not_require_dependencies(self) -> None:
        class ArrowLike:
            def to_pydict(self):
                return {"id": [1], "value": [2.0]}
        class NumpyLike(bytearray):
            shape = (2,)
            dtype = "uint8"
        self.assertEqual(data.Dataset.from_arrow(ArrowLike(), id="arrow").to_spec()["id"], "arrow")
        self.assertEqual(data.ArrayData.from_numpy(NumpyLike(b"ab"), id="numpy").to_spec()["dtype"], "uint8")

    def test_array_descriptor_rejects_mismatched_or_unknown_dtype_buffers(self) -> None:
        with self.assertRaises(data.DataError):
            data.ArrayData.from_buffer(b"abc", shape=(2,), dtype="f32")
        with self.assertRaises(data.DataError):
            data.ArrayData.from_buffer(b"a", shape=(1,), dtype="complex256")


    def test_dataframe_interchange_uses_optional_value_adapter(self) -> None:
        class Export:
            def column_names(self):
                return ("id", "label")

            def to_pydict(self):
                return {"id": [1, 2], "label": ["left", "right"]}

        class Frame:
            def __dataframe__(self):
                return Export()

        dataset = data.Dataset.from_dataframe(Frame(), key="id", id="interchange")
        self.assertEqual(dataset.schema, ("id", "label"))
        self.assertEqual(dataset.row_count, 2)


    def test_dlpack_uses_optional_numpy_adapter(self) -> None:
        import sys
        from types import SimpleNamespace
        from unittest.mock import patch

        class NumpyLike(bytearray):
            shape = (2,)
            dtype = "uint8"

        adapter = SimpleNamespace(from_dlpack=lambda producer: NumpyLike(b"ab"))
        with patch.dict(sys.modules, {"numpy": adapter}):
            array = data.ArrayData.from_dlpack(object(), id="dlpack")
        self.assertEqual(array.to_spec()["byte_length"], 2)


if __name__ == "__main__":
    unittest.main()
