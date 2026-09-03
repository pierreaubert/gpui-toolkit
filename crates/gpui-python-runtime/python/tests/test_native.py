import io
import math
import unittest
from unittest.mock import patch
import builtins
from array import array

from gpui_toolkit import data, native, px
from gpui_toolkit.resources import StaleResourceError


class NativeWrapperTests(unittest.TestCase):
    def test_installed_extension_encodes_dataset_arrow_ipc_without_pyarrow(self):
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")

        dataset = data.Dataset.from_mapping(
            {
                "enabled": [True, None, False],
                "count": [1, 2, None],
                "value": [1, 2.5, None],
                "label": ["left", None, "right"],
            },
            id="native-arrow",
        )
        self.assertEqual(dataset.column_types["value"], "float64")
        real_import = builtins.__import__

        def without_pyarrow(name, *args, **kwargs):
            if name == "pyarrow":
                raise ImportError("no pyarrow")
            return real_import(name, *args, **kwargs)

        with patch("builtins.__import__", side_effect=without_pyarrow):
            payload = dataset.to_arrow_ipc()
        self.assertGreater(len(payload), 128)
        self.assertEqual(payload[:4], b"\xff\xff\xff\xff")

    def test_installed_extension_runs_gpui_px_color_computations(self):
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")

        self.assertEqual(px.ColorScale.VIRIDIS.to_colormap_index(), 0)
        self.assertEqual(px.ColorScale.GREYS.to_colormap_index(), 6)
        self.assertEqual(px.ColorScale.VIRIDIS.map(0.0), "#440154")
        self.assertEqual(px.ColorScale.VIRIDIS.map(1.0), "#fde725")
        self.assertEqual(px.ColorScale.VIRIDIS.map(-1.0), "#440154")
        self.assertRaises(ValueError, px.ColorScale.PLASMA.map, math.nan)

        self.assertEqual(px.ColorRange.auto().resolve(-2.0, 8.0), (-2.0, 8.0))
        self.assertEqual(
            px.ColorRange.symmetric().resolve(-3.0, 8.0), (-8.0, 8.0)
        )
        self.assertEqual(
            px.ColorRange.symmetric(
                2.0, px.AutoOrFixed.fixed(4.0)
            ).resolve(-100.0, 100.0),
            (-2.0, 6.0),
        )
        with self.assertRaises(px.ColorRangeError) as error:
            px.ColorRange.auto().resolve(2.0, -2.0)
        self.assertEqual(error.exception.path, "color_range")

        custom = px.ColorScale.custom(
            lambda value: "ffffff" if value > 0.5 else "#000000"
        )
        self.assertEqual(custom.map(0.75), "#ffffff")
        self.assertEqual(custom.to_colormap_index(), 0)

    def test_installed_extension_runs_complete_d3_rgb_lab_hcl_values(self):
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")
        from gpui_toolkit import d3rs

        red = d3rs.D3Color.rgb(255, 0, 0)
        blue = d3rs.D3Color.from_hex(0x0000FF)
        self.assertEqual(red.to_hex(), "#ff0000")
        self.assertEqual(d3rs.D3Color.rgba(255, 0, 0, 128).to_hex_alpha(), "#ff000080")
        self.assertEqual(red.interpolate(blue, 0.5).to_hex(), "#800080")
        self.assertEqual(red.with_alpha(2.0).opacity(), 1.0)
        self.assertEqual(red.with_opacity(-1.0).opacity(), 0.0)
        self.assertGreater(red.lighten(0.5).g, red.g)
        self.assertLess(red.darken(0.5).r, red.r)
        self.assertGreater(red.brighter(1.0).r, 0.99)
        self.assertLess(red.darker(1.0).r, red.r)
        self.assertAlmostEqual(d3rs.D3Color.from_hsl(0.0, 1.0, 0.5).r, 1.0)
        self.assertEqual(d3rs.D3Color.from_rgba((1.0, 0.0, 0.0, 0.5)).a, 0.5)
        self.assertEqual(red.to_rgba(), (1.0, 0.0, 0.0, 1.0))

        lab = red.to_lab()
        self.assertIsInstance(lab, d3rs.Lab)
        self.assertAlmostEqual(lab.to_rgb().r, red.r, delta=0.02)
        self.assertAlmostEqual(lab.delta_e(lab), 0.0)
        self.assertGreater(lab.chroma(), 0.0)
        self.assertEqual(d3rs.Lab.new(200.0, 20.0, -10.0).l, 100.0)
        self.assertEqual(d3rs.Lab.with_alpha(50.0, 0.0, 0.0, 2.0).alpha, 1.0)
        self.assertAlmostEqual(
            d3rs.D3Color.from_lab(lab.l, lab.a, lab.b).r, 1.0, delta=0.02
        )

        hcl = red.to_hcl()
        self.assertIsInstance(hcl, d3rs.Hcl)
        self.assertAlmostEqual(hcl.to_rgb().r, red.r, delta=0.02)
        self.assertAlmostEqual(hcl.to_lab().l, lab.l, delta=0.02)
        self.assertEqual(
            d3rs.Hcl.new(370.0, -1.0, 200.0),
            d3rs.Hcl(10.0, 0.0, 100.0, 1.0),
        )
        self.assertEqual(d3rs.Hcl.from_lab(lab).alpha, lab.alpha)
        self.assertEqual(d3rs.Hcl.from_rgb(red).alpha, red.a)
        other = d3rs.Hcl.new(240.0, 50.0, 50.0)
        self.assertNotEqual(
            hcl.interpolate(other, 0.5).h,
            hcl.interpolate_long(other, 0.5).h,
        )
        self.assertAlmostEqual(
            d3rs.D3Color.from_hcl(hcl.h, hcl.c, hcl.l).r, 1.0, delta=0.02
        )

        with self.assertRaises(ValueError):
            d3rs.D3Color.rgb(256, 0, 0)
        with self.assertRaises(ValueError):
            d3rs.D3Color.from_hex(0x1000000)
        with self.assertRaisesRegex(ValueError, "finite"):
            d3rs.Lab.new(math.nan, 0.0, 0.0)

        hsl = d3rs.Hsl.from_rgb(red)
        self.assertAlmostEqual(hsl.h, 0.0)
        self.assertAlmostEqual(hsl.s, 1.0)
        self.assertAlmostEqual(hsl.l, 0.5)
        self.assertAlmostEqual(hsl.to_rgb().r, 1.0)
        self.assertEqual(d3rs.Hsl.new(120.0, 1.0, 0.5).a, 1.0)
        cubehelix = d3rs.Cubehelix.from_rgb(red)
        self.assertAlmostEqual(cubehelix.h, 86.95188852163756)
        self.assertAlmostEqual(cubehelix.to_rgb().r, 0.19760046899318695)
        self.assertEqual(d3rs.Cubehelix.new(300.0, 0.5, 0.5).alpha, 1.0)
        self.assertEqual(d3rs.cubehelix_default(-1.0).to_hex(), "#000000")
        self.assertEqual(d3rs.cubehelix_default(2.0).to_hex(), "#ffffff")
        self.assertIsInstance(
            d3rs.cubehelix_custom(300.0, -1.5, 1.0, 1.0, 0.5),
            d3rs.D3Color,
        )

        category10 = d3rs.ColorScheme.category10()
        self.assertEqual(category10.len(), 10)
        self.assertEqual(len(category10), 10)
        self.assertFalse(category10.is_empty())
        self.assertEqual(category10.color(0).to_hex(), "#1f77b4")
        self.assertEqual(category10.color(10), category10.color(0))
        self.assertEqual(d3rs.ColorScheme.tableau10().len(), 10)
        self.assertEqual(d3rs.ColorScheme.pastel().len(), 8)
        custom = d3rs.ColorScheme.new([red, blue])
        self.assertEqual(custom.colors(), (red, blue))
        self.assertEqual(custom.color(3), blue)
        self.assertEqual(d3rs.ColorScheme.new([]).color(0).to_hex(), "#000000")
        with self.assertRaises(TypeError):
            d3rs.ColorScheme.new(["#ff0000"])

        self.assertEqual(d3rs.interpolate_colors([red, blue], 0.5).to_hex(), "#800080")
        self.assertEqual(d3rs.interpolate_colors([], 0.5).to_hex(), "#000000")
        self.assertGreater(d3rs.sequential_color(0.0).b, d3rs.sequential_color(0.0).r)
        self.assertGreater(d3rs.sequential_color(1.0).r, d3rs.sequential_color(1.0).b)

        custom_sequential = d3rs.SequentialScale.new(
            [red.to_hcl(), blue.to_hcl()], "RedBlue"
        )
        self.assertEqual(custom_sequential.name(), "RedBlue")
        self.assertEqual(custom_sequential.get(0.0).to_hex(), "#ff0000")
        self.assertEqual(custom_sequential.get(1.0).to_hex(), "#0000ff")
        self.assertEqual(len(custom_sequential.sample(5)), 5)
        sequential_schemes = [
            d3rs.SequentialScheme.blues(),
            d3rs.SequentialScheme.greens(),
            d3rs.SequentialScheme.reds(),
            d3rs.SequentialScheme.purples(),
            d3rs.SequentialScheme.oranges(),
            d3rs.SequentialScheme.viridis(),
            d3rs.SequentialScheme.magma(),
            d3rs.SequentialScheme.inferno(),
            d3rs.SequentialScheme.plasma(),
            d3rs.SequentialScheme.turbo(),
            d3rs.SequentialScheme.bu_pu(),
            d3rs.SequentialScheme.cubehelix(),
        ]
        self.assertEqual(len(sequential_schemes), 12)
        self.assertTrue(all(len(scale.sample(3)) == 3 for scale in sequential_schemes))
        self.assertEqual(d3rs.SequentialScheme.get("Viridis").name(), "Viridis")
        self.assertIsNone(d3rs.SequentialScheme.get("unknown"))

        custom_diverging = d3rs.DivergingScale.new(
            [red.to_hcl()], d3rs.Hcl.new(0.0, 0.0, 97.0), [blue.to_hcl()], "Custom"
        )
        self.assertEqual(custom_diverging.name(), "Custom")
        self.assertEqual(custom_diverging.get(0.0).to_hex(), "#ff0000")
        self.assertEqual(custom_diverging.get(1.0).to_hex(), "#0000ff")
        self.assertEqual(len(custom_diverging.sample(5)), 5)
        diverging_schemes = [
            d3rs.DivergingScheme.rd_bu(),
            d3rs.DivergingScheme.rd_yl_bu(),
            d3rs.DivergingScheme.rd_yl_gn(),
            d3rs.DivergingScheme.pi_yg(),
            d3rs.DivergingScheme.br_bg(),
            d3rs.DivergingScheme.pu_or(),
            d3rs.DivergingScheme.spectral(),
        ]
        self.assertEqual(len(diverging_schemes), 7)
        self.assertTrue(all(len(scale.sample(3)) == 3 for scale in diverging_schemes))
        self.assertEqual(d3rs.DivergingScheme.get("RdBu").name(), "RdBu")
        self.assertIsNone(d3rs.DivergingScheme.get("unknown"))

    def test_installed_extension_runs_gpui_px_chart_interactions(self):
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")

        original = px.ChartInteraction(0.0, 10.0, 0.0, 20.0)
        interaction = original.with_size(100.0, 100.0).with_mode(
            px.InteractionMode.BRUSH
        )
        self.assertNotEqual(original.point_to_domain(50.0, 50.0), (5.0, 10.0))
        self.assertEqual(interaction.point_to_domain(50.0, 50.0), (5.0, 10.0))

        interaction.start_brush(10.0, 10.0)
        interaction.update_brush(60.0, 60.0)
        self.assertTrue(interaction.is_brushing())
        selection = interaction.end_brush(apply_zoom=True)
        self.assertIsNotNone(selection)
        self.assertAlmostEqual(selection.x0, 1.0)
        self.assertAlmostEqual(selection.x1, 6.0)
        self.assertAlmostEqual(selection.y0, 8.0)
        self.assertAlmostEqual(selection.y1, 18.0)
        self.assertEqual(interaction.zoom_level(), 1)
        self.assertEqual(interaction.x_domain(), (1.0, 6.0))

        hover = interaction.update_hover_pixel(50.0, 50.0)
        self.assertEqual(hover, interaction.hover_domain())
        interaction.clear_hover()
        self.assertIsNone(interaction.hover_domain())
        interaction.pan_by_pixels(10.0, 0.0)
        self.assertNotEqual(interaction.x_domain(), (1.0, 6.0))
        interaction.zoom_around_pixel(50.0, 50.0, 1.25)
        self.assertTrue(interaction.zoom_back())
        history_level = interaction.zoom_level()
        interaction.set_viewport_without_history(2.0, 4.0, 5.0, 15.0)
        self.assertEqual(interaction.zoom_level(), history_level)
        interaction.reset_zoom()
        self.assertFalse(interaction.is_zoomed())

        self.assertEqual(
            px.keyboard_action_for_key("ArrowLeft"), px.ChartKeyboardAction.PAN_LEFT
        )
        self.assertEqual(
            px.keyboard_action_for_key("+"), px.ChartKeyboardAction.ZOOM_IN
        )
        self.assertIsNone(px.keyboard_action_for_key("escape"))
        self.assertRaises(ValueError, px.ChartInteraction, 1.0, 1.0, 0.0, 1.0)
        self.assertRaises(ValueError, interaction.with_size, 0.0, 100.0)
        self.assertRaises(ValueError, interaction.zoom_around_domain, 1.0, 2.0, 0.0)

    def test_installed_extension_reports_authoritative_gpui_px_capabilities(self):
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")

        report = px.chart_capability_report()
        self.assertEqual(report.schema_version, 1)
        self.assertEqual(report.report_type, "gpui-px-chart-capabilities")
        self.assertFalse(report.all_release_ready())
        self.assertTrue(report.entries)
        self.assertTrue(report.to_markdown_table().startswith("# GPUI PX"))
        by_id = {entry.id: entry for entry in report.entries}
        self.assertEqual(
            by_id["static-export"].status,
            px.ChartCapabilityStatus.IMPLEMENTED,
        )
        self.assertEqual(
            by_id["interaction-state"].status,
            px.ChartCapabilityStatus.PARTIAL,
        )
        self.assertIn("line", by_id["chart-builders"].chart_families)
        blocking = {entry.id for entry in report.blocking_entries()}
        self.assertIn("interaction-state", blocking)
        self.assertNotIn("static-export", blocking)

    def test_installed_extension_runs_gpui_px_treemap_layout(self):
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")

        group = px.TreemapNode.with_children(
            "Group",
            [px.TreemapNode.new("A", 20.0), px.TreemapNode.new("B", 30.0)],
        )
        root = px.TreemapNode.with_children(
            "Root", [group, px.TreemapNode.new("C", 50.0)]
        )
        self.assertEqual(root.total_value(), 100.0)
        self.assertFalse(root.is_leaf())
        self.assertEqual(len(group.children), 2)

        for method in px.TilingMethod:
            rects = px.treemap_layout(
                root, 400.0, 200.0, method=method, padding=0.0
            )
            self.assertEqual([rect.name for rect in rects], ["A", "B", "C"])
            self.assertEqual(
                [(rect.depth, rect.category_index) for rect in rects],
                [(2, 0), (2, 0), (1, 1)],
            )
            self.assertTrue(
                all(
                    0.0 <= rect.x0 <= rect.x1 <= 400.0
                    and 0.0 <= rect.y0 <= rect.y1 <= 200.0
                    for rect in rects
                )
            )

        self.assertRaises(px.TreemapLayoutError, px.treemap_layout, root, 0.0, 200.0)
        self.assertRaises(
            px.TreemapLayoutError,
            px.treemap_layout,
            px.TreemapNode.new("bad", -1.0),
            400.0,
            200.0,
        )

    def test_installed_extension_retains_gpui_px_mesh_pick_index(self):
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")

        positions = data.ArrayData.from_buffer(
            array("d", [0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0]),
            shape=(4, 3),
            dtype="f64",
            id="pick-positions",
        )
        triangles = data.ArrayData.from_buffer(
            array("I", [0, 1, 2, 0, 2, 3]),
            shape=(2, 3),
            dtype="u32",
            id="pick-triangles",
        )
        values = data.ArrayData.from_buffer(
            array("d", [0.0, 1.0, 1.0, 0.0]),
            shape=(4,),
            dtype="f64",
            id="pick-values",
        )
        vertex_ids = data.ArrayData.from_buffer(
            array("Q", [10, 11, 12, 13]),
            shape=(4,),
            dtype="u64",
            id="pick-vertex-ids",
        )
        cell_ids = data.ArrayData.from_buffer(
            array("Q", [20, 21]),
            shape=(2,),
            dtype="u64",
            id="pick-cell-ids",
        )
        index = px.MeshPickIndex(
            positions,
            triangles,
            mesh_id="square",
            plot_id="pressure",
            field=values,
            field_id="spl",
            vertex_ids=vertex_ids,
            cell_ids=cell_ids,
        )
        self.assertEqual(index.vertex_count, 4)
        self.assertEqual(index.triangle_count, 2)
        pick = index.pick(0.4, 0.3)
        self.assertIsNotNone(pick)
        self.assertEqual(pick.plot_id, "pressure")
        self.assertEqual(pick.mesh_id, "square")
        self.assertEqual(pick.cell_index, 0)
        self.assertEqual(pick.cell_id, 20)
        self.assertIn(pick.vertex_id, {10, 11, 12})
        self.assertAlmostEqual(pick.displayed_value, 0.4)
        self.assertEqual(pick.field_id, "spl")
        self.assertAlmostEqual(pick.world_position[0], 0.4)
        self.assertAlmostEqual(pick.world_position[1], 0.3)
        self.assertEqual(pick.world_position[2], 0.0)
        self.assertIsNone(index.pick(2.0, 2.0))

        positions32 = data.ArrayData.from_buffer(
            array("f", [0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0]),
            shape=(4, 3),
            dtype="f32",
        )
        triangles16 = data.ArrayData.from_buffer(
            array("H", [0, 1, 2, 0, 2, 3]), shape=(2, 3), dtype="u16"
        )
        cell_values = data.ArrayData.from_buffer(
            array("f", [5.0, 6.0]), shape=(2,), dtype="f32"
        )
        validity = data.ArrayData.from_buffer(
            bytearray([1, 0]), shape=(2,), dtype="bool"
        )
        with px.MeshPickIndex(
            positions32,
            triangles16,
            field=cell_values,
            field_association="cell",
            valid=validity,
        ) as masked:
            self.assertIsNone(masked.pick(0.1, 0.9))

        positions.replace(
            array("d", [0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0])
        )
        self.assertRaises(StaleResourceError, index.pick, 0.4, 0.3)
        index.close()
        self.assertEqual(index.vertex_count, 0)
        self.assertRaises(RuntimeError, index.pick, 0.4, 0.3)

    def test_abi3_contract_is_importable_without_a_built_wheel(self):
        self.assertEqual(native.abi3_minimum_python(), "3.10")

    def test_source_tree_reports_a_missing_extension_without_fallback_math(self):
        if native.AVAILABLE:
            self.skipTest("installed abi3 wheel supplies the extension")
        with self.assertRaisesRegex(RuntimeError, "native extension is not installed"):
            native.linear_scale(0.5, domain=(0.0, 1.0), range=(0.0, 1.0))
        with self.assertRaisesRegex(RuntimeError, "native extension is not installed"):
            native.mean([1.0, 2.0])
        with self.assertRaisesRegex(RuntimeError, "native extension is not installed"):
            native.histogram([1.0, 2.0])

    def test_installed_extension_uses_d3rs_scale_and_validates_input(self):
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")
        from gpui_toolkit import d3rs

        self.assertEqual(
            native.linear_scale(5.0, domain=(0.0, 10.0), range=(0.0, 100.0)),
            50.0,
        )
        self.assertEqual(
            native.linear_scale(-2.0, domain=(0.0, 10.0), range=(0.0, 100.0), clamp=True),
            0.0,
        )
        with self.assertRaisesRegex(ValueError, "value must be finite"):
            native.linear_scale(math.nan, domain=(0.0, 1.0), range=(0.0, 1.0))
        self.assertEqual(
            native.linear_scale(1.0, domain=(1.0, 1.0), range=(0.0, 1.0)),
            0.5,
        )
        base = d3rs.LinearScale()
        configured = base.domain(0.0, 100.0).range(0.0, 500.0).clamp(True)
        self.assertIsNot(base, configured)
        self.assertIsInstance(configured, d3rs.Scale)
        self.assertEqual(base.domain_values, (0.0, 1.0))
        self.assertEqual(configured.scale(50.0), 250.0)
        self.assertEqual(configured.invert(250.0), 50.0)
        self.assertEqual(configured.scale(-1.0), 0.0)
        self.assertEqual(configured.domain_min(), 0.0)
        self.assertEqual(configured.domain_max(), 100.0)
        self.assertTrue(configured.is_clamped())
        self.assertTrue(configured.ticks(5))
        self.assertEqual(base.range_normalized(320.0).range_values, (0.0, 320.0))
        self.assertEqual(
            d3rs.LinearScale().domain(0.123, 0.987).nice().domain_values,
            (0.1, 1.0),
        )
        self.assertIs(configured.copy(), configured)
        with self.assertRaises(TypeError):
            d3rs.LinearScale(_domain=(0.0, 10.0))

        log_base = d3rs.LogScale()
        log_scale = log_base.domain(1.0, 16.0).range(0.0, 1.0).base(2.0)
        self.assertIsNot(log_base, log_scale)
        self.assertIsInstance(log_scale, d3rs.Scale)
        self.assertEqual(log_base.domain_values, (1.0, 10.0))
        self.assertAlmostEqual(log_scale.scale(4.0), 0.5)
        self.assertAlmostEqual(log_scale.invert(0.5), 4.0)
        self.assertEqual(log_scale.ticks(10), [1.0, 2.0, 4.0, 8.0, 16.0])
        self.assertTrue(log_scale.is_clamped())
        self.assertEqual(log_scale.range_normalized(500.0).range_values, (0.0, 500.0))
        self.assertLess(log_scale.clamp(False).scale(0.5), 0.0)
        with self.assertRaises(TypeError):
            d3rs.LogScale(_base=2.0)
        with self.assertRaisesRegex(ValueError, "positive"):
            log_base.domain(0.0, 10.0)
        with self.assertRaisesRegex(ValueError, "different from 1"):
            log_base.base(1.0)

        pow_base = d3rs.PowScale()
        pow_scale = (
            pow_base.domain(-100.0, 100.0)
            .range(-10.0, 10.0)
            .exponent(0.5)
        )
        self.assertIsNot(pow_base, pow_scale)
        self.assertIsInstance(pow_scale, d3rs.Scale)
        self.assertEqual(pow_scale.scale(-25.0), -5.0)
        self.assertEqual(pow_scale.scale(25.0), 5.0)
        self.assertAlmostEqual(pow_scale.invert(5.0), 25.0)
        self.assertEqual(pow_scale.exponent_value(), 0.5)
        self.assertFalse(pow_scale.is_clamped())
        self.assertTrue(pow_scale.ticks(5))
        self.assertEqual(
            d3rs.PowScale().domain(0.123, 0.987).nice().domain_values,
            (0.1, 1.0),
        )
        self.assertEqual(d3rs.sqrt_scale().exponent_value(), 0.5)
        self.assertIs(d3rs.SqrtScale, d3rs.PowScale)
        with self.assertRaises(TypeError):
            d3rs.PowScale(_exponent=2.0)
        with self.assertRaisesRegex(ValueError, "positive"):
            pow_base.exponent(0.0)

        symlog_base = d3rs.SymlogScale()
        symlog_scale = (
            symlog_base.domain(-100.0, 100.0)
            .range(0.0, 1.0)
            .constant(1.0)
        )
        self.assertIsNot(symlog_base, symlog_scale)
        self.assertIsInstance(symlog_scale, d3rs.Scale)
        self.assertAlmostEqual(symlog_scale.scale(0.0), 0.5)
        self.assertAlmostEqual(symlog_scale.invert(0.5), 0.0)
        self.assertEqual(symlog_scale.constant_value(), 1.0)
        self.assertFalse(symlog_scale.is_clamped())
        self.assertTrue(symlog_scale.ticks(5))
        self.assertEqual(
            d3rs.SymlogScale().domain(-0.987, 0.987).nice().domain_values,
            (-1.0, 1.0),
        )
        with self.assertRaises(TypeError):
            d3rs.SymlogScale(_constant=2.0)
        with self.assertRaisesRegex(ValueError, "positive"):
            symlog_base.constant(0.0)

        threshold_base = d3rs.ThresholdScale[str]()
        threshold = threshold_base.domain([0.0, 50.0, 100.0]).range(
            ["very low", "low", "medium", "high"]
        )
        self.assertIsNot(threshold_base, threshold)
        self.assertEqual(threshold.scale(-10.0), "very low")
        self.assertEqual(threshold.scale(50.0), "medium")
        self.assertEqual(threshold.scale(150.0), "high")
        self.assertEqual(threshold.scale(math.nan), "very low")
        self.assertEqual(threshold.thresholds(), (0.0, 50.0, 100.0))
        self.assertEqual(threshold.range_values(), ("very low", "low", "medium", "high"))
        self.assertEqual(threshold.invert_extent(1), (0.0, 50.0))
        self.assertEqual(threshold.ticks(), [0.0, 50.0, 100.0])
        self.assertIs(threshold.copy(), threshold)
        self.assertIsNone(threshold.invert("low"))
        self.assertEqual(
            d3rs.ThresholdScale.with_range(["negative", "positive"])
            .domain([0.0])
            .scale(1.0),
            "positive",
        )
        with self.assertRaisesRegex(ValueError, "strictly increasing"):
            threshold_base.domain([1.0, 1.0])
        with self.assertRaisesRegex(ValueError, "at least one range"):
            threshold_base.scale(1.0)

        quantize_base = d3rs.QuantizeScale[str]()
        quantize = quantize_base.domain(0.0, 100.0).range(["low", "mid", "high"])
        self.assertEqual(quantize.scale(10.0), "low")
        self.assertEqual(quantize.scale(50.0), "mid")
        self.assertEqual(quantize.scale(90.0), "high")
        self.assertEqual(quantize.thresholds(), [100.0 / 3.0, 200.0 / 3.0])
        self.assertEqual(quantize.invert_extent(1), (100.0 / 3.0, 200.0 / 3.0))
        self.assertEqual(quantize.domain_values, (0.0, 100.0))
        self.assertEqual(quantize.domain_min(), 0.0)
        self.assertEqual(quantize.domain_max(), 100.0)
        self.assertEqual(quantize.range_values(), ("low", "mid", "high"))
        self.assertIs(quantize.copy(), quantize)
        self.assertIsNone(quantize.invert("mid"))
        self.assertEqual(
            d3rs.QuantizeScale.with_range(["only"]).domain(0.0, 100.0).scale(50.0),
            "only",
        )

        quantile_base = d3rs.QuantileScale[str]()
        quantile = quantile_base.domain([4.0, 1.0, math.nan, 3.0, 2.0]).range(
            ["low", "high"]
        )
        self.assertEqual(quantile.domain_samples(), (1.0, 2.0, 3.0, 4.0))
        self.assertEqual(quantile.quantiles(), [2.5])
        self.assertEqual(quantile.scale(2.0), "low")
        self.assertEqual(quantile.scale(3.0), "high")
        self.assertEqual(quantile.scale(math.nan), "low")
        self.assertEqual(quantile.invert_extent(0), (1.0, 2.5))
        self.assertEqual(quantile.range_values(), ("low", "high"))
        self.assertIs(quantile.copy(), quantile)
        self.assertIsNone(quantile.invert("low"))
        self.assertEqual(
            d3rs.QuantileScale.with_range(["only"])
            .domain([1.0, 2.0, 3.0])
            .scale(2.0),
            "only",
        )
        with self.assertRaisesRegex(ValueError, "infinite"):
            quantile_base.domain([float("inf")])

        ordinal_base = d3rs.OrdinalScale[str, str]()
        ordinal = (
            ordinal_base.domain(["alpha", "beta", "gamma"])
            .range(["red", "blue"])
            .unknown("gray")
        )
        self.assertIsNot(ordinal_base, ordinal)
        self.assertEqual(ordinal.scale("alpha"), "red")
        self.assertEqual(ordinal.scale("beta"), "blue")
        self.assertEqual(ordinal.scale("gamma"), "red")
        self.assertEqual(ordinal.scale("missing"), "gray")
        self.assertEqual(ordinal.get_domain(), ("alpha", "beta", "gamma"))
        self.assertEqual(ordinal.get_range(), ("red", "blue"))
        self.assertIsNone(ordinal_base.scale("missing"))
        with self.assertRaises(TypeError):
            d3rs.OrdinalScale(_domain=("bad",))
        with self.assertRaisesRegex(TypeError, "hashable"):
            ordinal_base.domain([["not-hashable"]])

        band_base = d3rs.BandScale[str]()
        band = band_base.domain(["a", "b", "c", "d"]).range(0.0, 400.0)
        self.assertEqual(band.scale("a"), 0.0)
        self.assertEqual(band.scale("b"), 100.0)
        self.assertEqual(band.bandwidth(), 100.0)
        self.assertEqual(band.step(), 100.0)
        self.assertIsNone(band.scale("missing"))
        self.assertEqual(band.get_domain(), ("a", "b", "c", "d"))
        self.assertEqual(band.get_range(), (0.0, 400.0))
        padded_band = band.padding(0.2).align(0.0).round(True)
        self.assertGreater(padded_band.step(), padded_band.bandwidth())
        self.assertEqual(padded_band.padding_inner(2.0)._padding_inner, 1.0)
        with self.assertRaises(TypeError):
            band.round(1)

        point_base = d3rs.PointScale[str]()
        point = point_base.domain(["a", "b", "c"]).range(0.0, 100.0)
        self.assertEqual(point.scale("a"), 0.0)
        self.assertEqual(point.scale("b"), 50.0)
        self.assertEqual(point.scale("c"), 100.0)
        self.assertEqual(point.step(), 50.0)
        self.assertIsNone(point.scale("missing"))
        self.assertEqual(point.get_domain(), ("a", "b", "c"))
        centered = d3rs.PointScale[str]().domain(["only"]).range(0.0, 100.0)
        self.assertEqual(centered.scale("only"), 50.0)
        self.assertEqual(point.padding(2.0)._padding, 1.0)
        with self.assertRaises(TypeError):
            point.round(1)

    def test_installed_extension_runs_d3_array_statistics_and_ticks(self):
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")
        from gpui_toolkit import d3rs
        values = [1.0, 2.0, 2.0, 4.0]
        # The audited rustdoc mappings target the installed public d3rs module,
        # so exercise every mapped parameter through that path as well.
        self.assertEqual(d3rs.binary_search(values, 4.0), 3)
        self.assertEqual(d3rs.bisect_left(values, 2.0), 1)
        self.assertEqual(d3rs.bisect_right(values, 2.0), 3)
        self.assertEqual(d3rs.cumsum(values), [1.0, 3.0, 5.0, 9.0])
        self.assertAlmostEqual(d3rs.deviation(values), 1.2583057392117916)
        self.assertEqual(d3rs.extent(values), (1.0, 4.0))
        self.assertEqual(d3rs.max_index([4.0, 4.0, 1.0]), 0)
        self.assertEqual(d3rs.mean(values), 2.25)
        self.assertEqual(d3rs.median(values), 2.0)
        self.assertEqual(d3rs.quantile(values, 0.75), 2.5)
        self.assertEqual(d3rs.nice_bin_edges(0.0, 1.0, 2), [0.0, 0.5, 1.0])
        self.assertEqual(d3rs.threshold_sturges(100), 8)
        self.assertEqual(d3rs.least_index(values, 3.25), 3)
        self.assertEqual(d3rs.difference(values, [2.0]), [1.0, 4.0])
        self.assertEqual(d3rs.intersection(values, [2.0, 3.0]), [2.0, 2.0])
        self.assertTrue(d3rs.is_disjoint([1.0], [2.0]))
        self.assertTrue(d3rs.is_subset([1.0, 2.0], values))
        self.assertTrue(d3rs.is_superset(values, [1.0, 2.0]))
        self.assertEqual(
            d3rs.symmetric_difference([1.0, 2.0], [2.0, 3.0]),
            [1.0, 3.0],
        )
        self.assertEqual(d3rs.union([1.0, 2.0], [2.0, 3.0]), [1.0, 2.0, 3.0])
        self.assertEqual(d3rs.min(values), 1.0)
        self.assertEqual(d3rs.max(values), 4.0)
        self.assertEqual(d3rs.min_index(values), 0)
        self.assertEqual(d3rs.sum(values), 9.0)
        self.assertEqual(d3rs.nice(0.134, 0.867, 5), (0.1, 0.9))
        self.assertEqual(d3rs.ticks(0.0, 1.0, 5)[-1], 1.0)
        self.assertEqual(d3rs.bisect_left_f64(values, 2.0), 1)
        self.assertEqual(d3rs.bisect_right_f64(values, 2.0), 3)
        bins = d3rs.bin(values, 2)
        self.assertEqual(
            [(item.x0, item.x1, item.values) for item in bins],
            [(1.0, 2.5, (1.0, 2.0, 2.0)), (2.5, 4.0, (4.0,))],
        )
        self.assertEqual(native.bisect_left(values, 2.0), 1)
        self.assertEqual(native.bisect_right(values, 2.0), 3)
        self.assertEqual(native.quantile(values, 0.5), 2.0)
        self.assertEqual(native.quantile_sorted(values, 0.5), 2.0)
        self.assertEqual(d3rs.quantile_sorted(values, 0.5), 2.0)
        self.assertEqual(native.min(values), 1.0)
        self.assertEqual(native.max(values), 4.0)
        self.assertEqual(native.min_index(values), 0)
        self.assertEqual(native.max_index([4.0, 4.0, 1.0]), 0)
        self.assertEqual(native.least_index(values, 3.25), 3)
        self.assertEqual(native.sum(values), 9.0)
        self.assertEqual(native.mean(values), 2.25)
        self.assertEqual(native.median(values), 2.0)
        self.assertAlmostEqual(native.variance(values), 1.5833333333333333)
        self.assertAlmostEqual(d3rs.variance(values), 1.5833333333333333)
        self.assertAlmostEqual(native.deviation(values), 1.2583057392117916)
        self.assertEqual(native.extent(values), (1.0, 4.0))
        self.assertEqual(native.cumsum(values), [1.0, 3.0, 5.0, 9.0])
        generated_ticks = native.ticks(0.0, 1.0, 5)
        self.assertEqual(len(generated_ticks), 11)
        self.assertEqual(generated_ticks[0], 0.0)
        self.assertEqual(generated_ticks[-1], 1.0)
        self.assertAlmostEqual(generated_ticks[1], 0.1)
        self.assertEqual(native.tick_step(0.0, 1.0, 5), 0.1)
        self.assertEqual(d3rs.tick_increment(0.0, 1.0, 5), 0.1)
        self.assertEqual(d3rs.ticks_interval(0.0, 1.0, 0.25)[2], 0.5)
        self.assertTrue(d3rs.time_ticks(0.0, 10_000.0, 3))
        self.assertEqual(d3rs.tick_step(0.0, 1.0, 5), 0.1)
        self.assertEqual(native.nice_number(15.0, False), 20.0)
        self.assertEqual(d3rs.nice_number(15.0, True), 20.0)
        self.assertEqual(d3rs.scale_nice_number(15.0, True), 20.0)
        self.assertEqual(
            d3rs.generate_linear_ticks(min=0.0, max=100.0, count=5),
            [0.0, 20.0, 40.0, 60.0, 80.0, 100.0],
        )
        self.assertEqual(d3rs.generate_linear_ticks(2.0, 4.0, 0), [2.0])
        self.assertEqual(
            d3rs.generate_log_ticks(1.0, 100.0, 10.0, False),
            [1.0, 10.0, 100.0],
        )
        self.assertEqual(native.nice_number(-15.0, False), -20.0)
        nice_start, nice_stop = native.nice(0.134, 0.867, 5)
        self.assertLessEqual(nice_start, 0.134)
        self.assertGreaterEqual(nice_stop, 0.867)
        self.assertEqual(native.ticks_interval(0.0, 1.0, 0.25), [0.0, 0.25, 0.5, 0.75, 1.0])
        self.assertTrue(native.log_ticks(1.0, 100.0))
        self.assertEqual(
            d3rs.log_ticks(1.0, 8.0, base=2.0, subdivisions=False),
            [1.0, 2.0, 4.0, 8.0],
        )
        self.assertTrue(native.time_ticks(0.0, 1000.0, 4))

        with self.assertRaisesRegex(ValueError, r"percentile must be in \[0, 1\]"):
            native.quantile(values, 2.0)
        with self.assertRaisesRegex(ValueError, r"data\[1\] must be finite"):
            native.mean([1.0, math.nan])
        with self.assertRaisesRegex(ValueError, "sorted in ascending order"):
            native.quantile_sorted([2.0, 1.0], 0.5)
        with self.assertRaisesRegex(ValueError, "sorted in ascending order"):
            native.bisect_left([2.0, 1.0], 1.0)
        with self.assertRaisesRegex(ValueError, "interval must be positive"):
            native.ticks_interval(0.0, 1.0, 0.0)
        with self.assertRaisesRegex(ValueError, "base must be finite and greater than 1"):
            native.log_ticks(1.0, 10.0, base=1.0)
        with self.assertRaisesRegex(ValueError, "count must be non-negative"):
            native.ticks(0.0, 1.0, -1)
        with self.assertRaisesRegex(ValueError, "min must be finite"):
            d3rs.generate_linear_ticks(math.nan, 1.0, 5)

    def test_translated_d3_array_callbacks_preserve_generic_values(self):
        from gpui_toolkit import d3rs

        records = [
            {"name": "alpha", "score": 3},
            {"name": "beta", "score": 1},
            {"name": "gamma", "score": 2},
            {"name": "beta", "score": 4},
        ]
        compare = lambda left, right: (left["score"] > right["score"]) - (
            left["score"] < right["score"]
        )
        self.assertEqual(d3rs.bisect([1, 3, 3, 5], 3), 3)
        bisector = d3rs.Bisector(lambda item: item["score"])
        ordered = sorted(records, key=lambda item: item["score"])
        self.assertEqual(bisector.left(ordered, 2.0), 1)
        self.assertEqual(bisector.right(ordered, 2.0), 2)
        self.assertIs(bisector.center(ordered, 2.6), ordered[2])
        self.assertIsNone(bisector.center([], 2.0))
        with self.assertRaisesRegex(TypeError, "accessor must be callable"):
            d3rs.Bisector(None)
        self.assertEqual(d3rs.count(records, lambda item: item["score"] >= 3), 2)
        self.assertEqual(d3rs.min_by(records, compare)["score"], 1)
        self.assertEqual(d3rs.max_by(records, compare)["score"], 4)
        extent = d3rs.extent_by(records, compare)
        self.assertIsNotNone(extent)
        self.assertEqual((extent[0]["score"], extent[1]["score"]), (1, 4))
        self.assertEqual(d3rs.mean_by(records, lambda item: item["score"]), 2.5)
        self.assertEqual(
            d3rs.filter(records, lambda item: item["score"] % 2 == 0),
            [records[2], records[3]],
        )
        self.assertEqual(
            d3rs.map(records, lambda item: item["name"]),
            ["alpha", "beta", "gamma", "beta"],
        )
        self.assertEqual(
            d3rs.reduce(records, 0, lambda total, item: total + item["score"]),
            10,
        )
        grouped = d3rs.group(records, lambda item: item["name"])
        self.assertEqual(grouped["beta"], [records[1], records[3]])
        self.assertEqual(
            d3rs.rollup(
                records,
                lambda item: item["name"],
                lambda group: sum(item["score"] for item in group),
            )["beta"],
            5,
        )
        self.assertIs(
            d3rs.index(records, lambda item: item["name"])["beta"], records[3]
        )
        self.assertEqual(
            [item["score"] for item in d3rs.sort_by(records, lambda item: item["score"])],
            [1, 2, 3, 4],
        )
        self.assertEqual(
            [
                item["score"]
                for item in d3rs.sort_by_desc(records, lambda item: item["score"])
            ],
            [4, 3, 2, 1],
        )
        with self.assertRaisesRegex(TypeError, "predicate must be callable"):
            d3rs.filter([], None)
        with self.assertRaisesRegex(RuntimeError, "callback failed"):
            d3rs.map(records, lambda _item: (_ for _ in ()).throw(RuntimeError("callback failed")))

    def test_installed_extension_runs_histogram_threshold_strategies(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")
        from gpui_toolkit import d3rs

        values = [0.0, 1.0, 2.0, 3.0]
        bins = native.histogram(
            values, strategy=native.HistogramThreshold.COUNT, count=2
        )
        self.assertEqual(
            [(item.x0, item.x1) for item in bins], [(0.0, 1.5), (1.5, 3.0)]
        )
        self.assertEqual([item.count for item in bins], [2, 2])
        self.assertEqual(len(bins[0]), 2)
        self.assertFalse(bins[0].is_empty)
        self.assertEqual(bins[0].values, (0.0, 1.0))

        explicit = native.histogram(
            values,
            strategy=native.HistogramThreshold.VALUES,
            thresholds=[0.0, 2.0, 4.0],
        )
        self.assertEqual(
            [item.values for item in explicit], [(0.0, 1.0), (2.0, 3.0)]
        )
        self.assertEqual(d3rs.histogram(values)[0].count, native.histogram(values)[0].count)
        records = [
            {"name": "low", "value": 0.0},
            {"name": "middle", "value": 1.0},
            {"name": "high", "value": 3.0},
        ]
        base = d3rs.BinGenerator()
        configured = (
            base.value(lambda item: item["value"])
            .domain(0.0, 3.0)
            .thresholds_count(2)
        )
        self.assertIsNot(base, configured)
        generic_bins = configured.generate(records)
        self.assertEqual(generic_bins[0].values, (records[0], records[1]))
        self.assertEqual(generic_bins[1].values, (records[2],))
        self.assertEqual(len(generic_bins[0]), 2)
        self.assertFalse(generic_bins[0].is_empty())
        explicit_generic = (
            base.value(lambda item: item["value"])
            .thresholds([0.0, 2.0, 4.0])
            .generate(records)
        )
        self.assertEqual([len(item) for item in explicit_generic], [2, 1])
        self.assertNotEqual(base.thresholds_sturges(), base.thresholds_freedman_diaconis())
        with self.assertRaises(TypeError):
            d3rs.BinGenerator(_count=2)
        with self.assertRaisesRegex(TypeError, "accessor must be callable"):
            base.value(None)
        with self.assertRaisesRegex(ValueError, "finite increasing"):
            base.domain(2.0, 1.0)
        for invalid_count in (0, True):
            with self.subTest(invalid_count=invalid_count):
                with self.assertRaisesRegex(ValueError, "positive integer"):
                    base.thresholds_count(invalid_count)
        with self.assertRaisesRegex(ValueError, "strictly increasing"):
            base.thresholds([0.0, 0.0])
        self.assertEqual(native.threshold_sturges(100), 8)
        self.assertEqual(native.nice_bin_edges(0.0, 1.0, 2), [0.0, 0.5, 1.0])

        for strategy in (
            native.HistogramThreshold.STURGES,
            native.HistogramThreshold.FREEDMAN_DIACONIS,
            native.HistogramThreshold.SCOTT,
        ):
            generated = native.histogram(values, strategy=strategy)
            self.assertEqual(sum(item.count for item in generated), 4)

        with self.assertRaisesRegex(ValueError, "requires count"):
            native.histogram(values, strategy=native.HistogramThreshold.COUNT)
        with self.assertRaisesRegex(ValueError, "increasing"):
            native.histogram(
                values,
                strategy=native.HistogramThreshold.VALUES,
                thresholds=[0.0, 0.0],
            )
        with self.assertRaisesRegex(ValueError, r"data\[1\]"):
            native.histogram([0.0, math.nan])

    def test_installed_extension_runs_array_transforms(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")
        from gpui_toolkit import d3rs

        values = [1.0, 2.0, 3.0, 4.0]
        self.assertEqual(native.reverse(values), [4.0, 3.0, 2.0, 1.0])
        self.assertEqual(values, [1.0, 2.0, 3.0, 4.0])
        shuffled = native.shuffle_seeded(values, 42)
        self.assertEqual(shuffled, native.shuffle_seeded(values, 42))
        self.assertEqual(sorted(shuffled), values)
        self.assertEqual(sorted(native.shuffle(values)), values)
        left_rng = native.LcgRng.new(42)
        right_rng = native.LcgRng.new(42)
        labels = ["alpha", {"id": 2}, ("gamma",)]
        self.assertEqual(
            native.shuffle(left_rng, labels),
            native.shuffle(right_rng, labels),
        )
        self.assertNotEqual(left_rng.next_f64(), native.LcgRng.new(42).next_f64())
        mutable = ["alpha", "beta", "gamma", "delta"]
        self.assertIsNone(d3rs.shuffle_in_place(native.LcgRng.new(7), mutable))
        self.assertEqual(sorted(mutable), ["alpha", "beta", "delta", "gamma"])
        self.assertEqual(
            native.RandomUniform.new(2.0, 3.0).sample() >= 2.0,
            True,
        )
        self.assertIsInstance(native.RandomNormal.new(0.0, 1.0).sample(), float)
        self.assertGreater(native.RandomLogNormal.new(0.0, 1.0).sample(), 0.0)
        self.assertGreaterEqual(native.RandomExponential.new(1.0).sample(), 0.0)
        self.assertIsInstance(native.RandomBernoulli.new(0.5).sample(), bool)
        self.assertIsInstance(native.RandomPoisson.new(1.0).sample(), int)
        self.assertIsInstance(native.RandomIrwinHall.new(2).sample(), float)
        self.assertIsInstance(native.RandomBates.new(2).sample(), float)
        self.assertEqual(native.pairs(values), [(1.0, 2.0), (2.0, 3.0), (3.0, 4.0)])
        self.assertEqual(
            d3rs.cross([1.0, 2.0], [10.0, 20.0]),
            [(1.0, 10.0), (1.0, 20.0), (2.0, 10.0), (2.0, 20.0)],
        )
        self.assertEqual(native.unique([1.0, 2.0, 1.0, 3.0]), [1.0, 2.0, 3.0])
        self.assertEqual(native.sort([3.0, 1.0, 2.0]), [1.0, 2.0, 3.0])
        self.assertEqual(native.sort_descending([3.0, 1.0, 2.0]), [3.0, 2.0, 1.0])
        self.assertEqual(
            native.merge_sorted([[1.0, 3.0], [2.0, 4.0]]),
            [1.0, 2.0, 3.0, 4.0],
        )
        self.assertEqual(native.binary_search([1.0, 2.0, 3.0], 2.0), 1)
        self.assertIsNone(native.binary_search([1.0, 2.0, 3.0], 4.0))
        self.assertEqual(native.difference([1.0, 2.0, 3.0], [2.0]), [1.0, 3.0])
        self.assertEqual(native.intersection([1.0, 2.0, 3.0], [2.0, 4.0]), [2.0])
        self.assertEqual(native.union([1.0, 2.0], [2.0, 3.0]), [1.0, 2.0, 3.0])
        self.assertEqual(len(native.union([-0.0], [0.0])), 1)
        self.assertEqual(
            native.symmetric_difference([1.0, 2.0], [2.0, 3.0]), [1.0, 3.0]
        )
        self.assertTrue(native.is_subset([1.0, 2.0], [1.0, 2.0, 3.0]))
        self.assertTrue(native.is_superset([1.0, 2.0, 3.0], [1.0, 2.0]))
        self.assertTrue(native.is_disjoint([1.0, 2.0], [3.0, 4.0]))
        self.assertFalse(native.is_disjoint([1.0, 2.0], [2.0, 3.0]))
        with self.assertRaisesRegex(ValueError, "non-negative"):
            native.shuffle_seeded(values, -1)
        with self.assertRaisesRegex(ValueError, r"data\[1\]"):
            native.reverse([1.0, math.inf])
        with self.assertRaisesRegex(ValueError, "range must be finite"):
            native.nice_number(math.nan, False)
        with self.assertRaisesRegex(ValueError, "sorted in ascending order"):
            native.merge_sorted([[2.0, 1.0]])

    def test_installed_extension_runs_numeric_and_color_interpolation(self):
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")
        from gpui_toolkit import d3rs

        self.assertEqual(native.interpolate_number(10.0, 20.0, 0.25), 12.5)
        self.assertEqual(d3rs.interpolate_number(10.0, 20.0, 0.25), 12.5)
        self.assertEqual(d3rs.interpolate_f32(10.0, 20.0, 0.25), 12.5)
        self.assertEqual(d3rs.lerp(10.0, 20.0, 0.25), 12.5)
        self.assertEqual(
            d3rs.piecewise_with(
                [0.0, 10.0, 30.0], lambda left, right, t: left + (right - left) * t, 0.75
            ),
            20.0,
        )
        self.assertEqual(d3rs.interpolate_array([0, 10, 20], [10, 30], 0.5), [5, 20])
        interpolators = d3rs.ArrayInterpolator(
            [lambda t: 10.0 * t, lambda t: 20.0 + 10.0 * t]
        )
        self.assertEqual(interpolators.interpolate(0.5), [5.0, 25.0])
        self.assertEqual(native.interpolate_round(0, 10, 0.25), 3)
        self.assertEqual(
            native.interpolate_number_array([0.0, 10.0], [10.0, 30.0], 0.5),
            [5.0, 20.0],
        )
        for interpolator in (
            native.interpolate_rgb,
            native.interpolate_hsl,
            native.interpolate_hsl_long,
            native.interpolate_lab,
            native.interpolate_hcl,
            native.interpolate_hcl_long,
        ):
            self.assertEqual(interpolator("#ff0000", "#0000ff", 0.0), "#ff0000")
            self.assertEqual(interpolator("#ff0000", "#0000ff", 1.0), "#0000ff")
        for interpolator in (
            native.interpolate_cubehelix,
            native.interpolate_cubehelix_long,
        ):
            midpoint = interpolator("#ff0000", "#0000ff", 0.5)
            self.assertTrue(midpoint.startswith("#"))
            self.assertEqual(len(midpoint), 7)
        self.assertEqual(native.interpolate_rgb("#000000", "#ffffff", 0.5), "#808080")
        self.assertEqual(native.color_luminance("#000000"), 0.0)
        self.assertEqual(native.color_luminance("#ffffff"), 1.0)
        self.assertNotEqual(native.color_lighten("#202020", 0.25), "#202020")
        self.assertNotEqual(native.color_darken("#e0e0e0", 0.25), "#e0e0e0")
        with self.assertRaisesRegex(ValueError, "equal lengths"):
            native.interpolate_number_array([0.0], [1.0, 2.0], 0.5)
        with self.assertRaisesRegex(ValueError, "#RRGGBB"):
            native.interpolate_rgb("red", "#0000ff", 0.5)

    def test_installed_extension_runs_complete_d3_ease_family(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")
        from gpui_toolkit import d3rs

        defaults = (
            "ease_linear",
            "ease_quad_in",
            "ease_quad_out",
            "ease_quad_in_out",
            "ease_cubic_in",
            "ease_cubic_out",
            "ease_cubic_in_out",
            "ease_sin_in",
            "ease_sin_out",
            "ease_sin_in_out",
            "ease_exp_in",
            "ease_exp_out",
            "ease_exp_in_out",
            "ease_circle_in",
            "ease_circle_out",
            "ease_circle_in_out",
            "ease_elastic_in",
            "ease_elastic_out",
            "ease_elastic_in_out",
            "ease_back_in",
            "ease_back_out",
            "ease_back_in_out",
            "ease_bounce_in",
            "ease_bounce_out",
            "ease_bounce_in_out",
        )
        for name in defaults:
            easing = getattr(native, name)
            self.assertAlmostEqual(easing(0.0), 0.0, msg=name)
            self.assertAlmostEqual(easing(1.0), 1.0, msg=name)

        self.assertEqual(native.ease_poly_in(4.0, 0.5), 0.0625)
        self.assertEqual(native.ease_poly_out(4.0, 0.5), 0.9375)
        self.assertEqual(native.ease_poly_in_out(4.0, 0.5), 0.5)
        self.assertAlmostEqual(native.ease_back_in_with(0.0, 0.5), 0.125)
        self.assertAlmostEqual(native.ease_back_out_with(0.0, 0.5), 0.875)
        self.assertAlmostEqual(native.ease_back_in_out_with(0.0, 0.5), 0.5)
        self.assertEqual(native.ease_elastic_in_with(1.0, 0.3, 0.0), 0.0)
        self.assertEqual(native.ease_elastic_out_with(1.0, 0.3, 1.0), 1.0)
        self.assertEqual(d3rs.ease(d3rs.EaseKind.CUBIC_IN_OUT, 0.5), 0.5)
        self.assertEqual(d3rs.EaseKind.CUBIC_IN_OUT.apply(0.5), 0.5)
        self.assertEqual(d3rs.ease_quad_in(0.5), 0.25)

        with self.assertRaisesRegex(ValueError, "period must be positive"):
            native.ease_elastic_in_with(1.0, 0.0, 0.5)
        with self.assertRaisesRegex(ValueError, "t must be finite"):
            native.ease_linear(math.nan)

    def test_installed_extension_runs_complete_d3_format_surface(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")
        from gpui_toolkit import d3rs

        specifier = d3rs.parse_format_specifier(specifier=".2f")
        self.assertEqual(specifier.fill, " ")
        self.assertEqual(specifier.align, d3rs.FormatAlign.RIGHT)
        self.assertEqual(specifier.sign, d3rs.FormatSign.MINUS)
        self.assertEqual(specifier.precision, 2)
        self.assertEqual(specifier.format_type, d3rs.FormatType.FIXED)
        self.assertEqual(d3rs.FormatAlign.AFTER_SIGN.value, "after_sign")
        self.assertEqual(d3rs.FormatType.CHARACTER.value, "character")

        self.assertEqual(d3rs.format_value(".2f", 3.14159), "3.14")
        self.assertEqual(d3rs.format("+.1%")(.255), "+25.5%")
        self.assertEqual(d3rs.format_value("f", math.inf), "Infinity")
        self.assertEqual(d3rs.format_value("f", math.nan), "NaN")
        self.assertEqual(d3rs.prefix_exponent(1_000_000.0), 6)
        self.assertEqual(
            d3rs.format_prefix(specifier=".2", reference=1_000.0)(2_500.0),
            "2.50k",
        )

        locale = d3rs.Locale(
            decimal=",",
            thousands=" ",
            currency_prefix="€",
            currency_suffix=None,
            grouping=(3,),
            minus="−",
            percent=" pct",
        )
        self.assertEqual(locale.decimal, ",")
        self.assertEqual(locale.thousands, " ")
        self.assertEqual(locale.currency_prefix, "€")
        self.assertIsNone(locale.currency_suffix)
        self.assertEqual(locale.grouping, (3,))
        self.assertIsNone(locale.numerals)
        self.assertEqual(locale.minus, "−")
        self.assertEqual(locale.percent, " pct")
        self.assertEqual(locale.format(",.2f", 1234.5), "1 234,50")
        self.assertEqual(d3rs.format_locale(locale, "+.0%")(0.5), "+50 pct")
        self.assertEqual(d3rs.format_locale_value(locale, "$,.2f", 1234.5), "€1 234,50")
        self.assertEqual(d3rs.format("d")(42.0), "42")
        self.assertEqual(d3rs.DEFAULT_LOCALE.format(".1f", 1.25), "1.2")

        with self.assertRaisesRegex(ValueError, "exactly ten"):
            d3rs.Locale(numerals=("0", "1"))
        with self.assertRaisesRegex(ValueError, "reference must be finite"):
            d3rs.format_prefix(".2", math.inf)(1.0)

    def test_installed_extension_runs_d3_time_intervals_formats_and_scale(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")
        from gpui_toolkit import d3rs

        timestamp = 1_701_432_645  # 2023-12-01 12:30:45 UTC
        self.assertEqual(d3rs.TimeInterval.DAY.floor(timestamp), 1_701_388_800)
        self.assertEqual(d3rs.TimeInterval.HOUR.ceil(timestamp), 1_701_435_600)
        self.assertEqual(d3rs.TimeInterval.MINUTE.round(timestamp), 1_701_432_660)
        self.assertEqual(d3rs.TimeInterval.MONTH.offset(1_706_659_200), 1_709_164_800)
        self.assertEqual(d3rs.TimeInterval.MONTH.count(1_704_067_200, 1_711_929_600), 3)
        self.assertEqual(
            d3rs.TimeInterval.MONTH.range(1_704_067_200, 1_711_929_600),
            [1_704_067_200, 1_706_745_600, 1_709_251_200],
        )
        self.assertEqual(d3rs.TimeInterval.DAY.duration, 86_400)
        self.assertEqual(d3rs.TimeInterval.MONTH.format_pattern, "%B")
        self.assertEqual(d3rs.TimeInterval.for_span(86_400), d3rs.TimeInterval.DAY)
        self.assertEqual(d3rs.time_monday(), d3rs.TimeInterval.MONDAY)

        parts = d3rs.TimeFormatParts.from_unix_seconds(951_782_400)
        self.assertEqual(
            (parts.year, parts.month, parts.day, parts.weekday, parts.day_of_year),
            (2000, 2, 29, 2, 60),
        )
        formatter = d3rs.TimeFormat("%Y-%m-%d %H:%M:%S")
        self.assertEqual(formatter.format(0), "1970-01-01 00:00:00")
        self.assertEqual(d3rs.time_format("%a %b %d")(0), "Thu Jan 01")
        self.assertEqual(d3rs.time_format_value("%Y", 0), "1970")

        base = d3rs.TimeScale()
        with self.assertRaises(TypeError):
            d3rs.TimeScale(_domain=(0, 10))
        scale = base.domain(0, 86_400).range(0.0, 100.0).clamp(True)
        self.assertEqual(base.domain_values, (0, 1))
        self.assertEqual(scale.scale(43_200), 50.0)
        self.assertEqual(scale.scale(-1), 0.0)
        self.assertEqual(scale.invert(75.0), 64_800)
        self.assertEqual(scale.interval(), d3rs.TimeInterval.DAY)
        self.assertTrue(scale.ticks(4))
        self.assertEqual(scale.time_ticks(4), scale.ticks(4))
        self.assertEqual(scale.domain_min(), 0)
        self.assertEqual(scale.domain_max(), 86_400)
        self.assertEqual(
            d3rs.TimeScale().domain(5, 86_500).nice().domain_values,
            (0, 172_800),
        )
        self.assertIs(scale.copy(), scale)
        self.assertEqual(d3rs.TimeScale().domain(0, 60).scale(30), 0.5)
        self.assertEqual(d3rs.timestamp_from_millis(millis=2_500), 2)
        self.assertEqual(d3rs.millis_from_timestamp(timestamp=2), 2_000)

        with self.assertRaisesRegex(ValueError, "step must be positive"):
            d3rs.TimeInterval.DAY.range(0, 10, 0)
        with self.assertRaisesRegex(ValueError, "domain endpoints must differ"):
            d3rs.TimeScale().domain(1, 1)

    def test_installed_extension_runs_remaining_numeric_and_string_interpolators(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")
        from gpui_toolkit import d3rs

        self.assertEqual(native.clamp01(-1.0), 0.0)
        self.assertEqual(native.clamp01(2.0), 1.0)
        self.assertEqual(native.interpolate_clamped(0.0, 100.0, 2.0), 100.0)
        self.assertTrue(math.isfinite(native.interpolate_basis([0.0, 10.0, 0.0], 0.5)))
        self.assertAlmostEqual(
            native.interpolate_basis_closed([0.0, 10.0, 20.0, 10.0], 0.0),
            native.interpolate_basis_closed([0.0, 10.0, 20.0, 10.0], 1.0),
        )
        self.assertAlmostEqual(native.interpolate_exp(1.0, 100.0, 0.5), 10.0)
        self.assertEqual(native.interpolate_discrete([0.0, 10.0, 20.0, 30.0], 0.5), 20.0)
        self.assertEqual(native.interpolate_quantize(0.0, 100.0, 5, 0.25), 25.0)
        self.assertEqual(native.piecewise([0.0, 50.0, 100.0], 0.25), 25.0)
        self.assertEqual(
            native.piecewise_domain([0.0, 0.3, 1.0], [0.0, 100.0, 200.0], 0.3),
            100.0,
        )
        self.assertEqual(native.quantize([0.0, 10.0, 20.0, 30.0], 0.75), 30.0)
        self.assertEqual(native.interpolate_ease(0.0, 100.0, "quad_in", 0.5), 25.0)
        self.assertEqual(
            native.interpolate_matrix(
                [[0.0, 1.0], [2.0, 3.0]],
                [[10.0, 11.0], [12.0, 13.0]],
                0.5,
            ),
            [[5.0, 6.0], [7.0, 8.0]],
        )
        x, y, width = native.interpolate_zoom_vector(
            (0.0, 0.0, 100.0), (0.0, 0.0, 10.0), 0.5
        )
        self.assertEqual((x, y), (0.0, 0.0))
        self.assertAlmostEqual(width, math.sqrt(1_000.0))
        self.assertEqual(native.interpolate_string("10px", "20px", 0.5), "15px")
        css = native.interpolate_transform_css(
            "translate(0px, 0px)", "translate(100px, 50px)", 0.5
        )
        self.assertIn("50", css)
        self.assertIn("25", css)
        self.assertEqual(native.interpolate_date(0.0, 100.0, 0.25), 25.0)
        self.assertEqual(
            d3rs.interpolate_ease(0.0, 1.0, d3rs.EaseKind.CUBIC_IN, 0.5),
            0.125,
        )

        with self.assertRaisesRegex(ValueError, "at least one value"):
            native.interpolate_discrete([], 0.5)
        with self.assertRaisesRegex(ValueError, "same non-zero length"):
            native.piecewise_domain([0.0], [0.0, 1.0], 0.5)
        with self.assertRaisesRegex(ValueError, "zoom widths must be positive"):
            native.interpolate_zoom_vector((0.0, 0.0, 0.0), (1.0, 1.0, 1.0), 0.5)

    def test_installed_extension_runs_typed_transform_and_zoom_interpolation(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")
        from gpui_toolkit import d3rs

        identity = native.Transform2D.identity()
        self.assertEqual(identity.apply(10.0, 20.0), (10.0, 20.0))
        translated = native.Transform2D.translate(100.0, 50.0)
        self.assertEqual(translated.apply(10.0, 20.0), (110.0, 70.0))
        rotated = native.Transform2D.rotate_deg(90.0)
        x, y = rotated.apply(1.0, 0.0)
        self.assertAlmostEqual(x, 0.0, places=12)
        self.assertAlmostEqual(y, 1.0, places=12)
        self.assertEqual(native.Transform2D.scale_uniform(2.0).apply(2.0, 3.0), (4.0, 6.0))
        self.assertEqual(
            native.Transform2D.from_matrix(translated.to_matrix()).as_tuple(),
            translated.as_tuple(),
        )
        midpoint = native.interpolate_transform(identity, translated, 0.5)
        self.assertEqual((midpoint.translate_x, midpoint.translate_y), (50.0, 25.0))
        self.assertIn("translate(50.000px, 25.000px)", midpoint.to_css())
        self.assertTrue(midpoint.to_svg().startswith("matrix("))
        matrix_midpoint = native.interpolate_transform_svg(
            identity.to_matrix(), translated.to_matrix(), 0.5
        )
        self.assertEqual(matrix_midpoint[4:], (50.0, 25.0))
        self.assertEqual(
            d3rs.Transform2D.skew_x_deg(0.0).to_matrix(), identity.to_matrix()
        )

        start = native.ZoomView(0.0, 0.0, 100.0)
        end = native.ZoomView(50.0, 50.0, 10.0)
        zoom_midpoint = native.interpolate_zoom_view(start, end, 0.5)
        self.assertTrue(0.0 < zoom_midpoint.cx < 50.0)
        self.assertTrue(0.0 < zoom_midpoint.cy < 50.0)
        self.assertGreater(zoom_midpoint.size, 0.0)
        zoom_start = start.interpolate(end, 0.0)
        self.assertAlmostEqual(zoom_start.cx, start.cx)
        self.assertAlmostEqual(zoom_start.cy, start.cy)
        self.assertAlmostEqual(zoom_start.size, start.size)
        self.assertAlmostEqual(start.interpolate(end, 1.0).cx, end.cx)
        self.assertGreater(start.duration(end), 0.0)
        custom = native.ZoomParams(rho=1.5)
        self.assertGreater(native.zoom_duration(start, end, params=custom), 0.0)
        self.assertGreater(d3rs.zoom_duration_with_rho(start, end, 1.5), 0.0)
        self.assertEqual(
            d3rs.interpolate_zoom_with_params(start, end, custom, 0.0), start
        )

        with self.assertRaisesRegex(ValueError, "exactly six"):
            native.Transform2D.from_matrix([1.0, 0.0])
        with self.assertRaisesRegex(ValueError, "finite and positive"):
            native.ZoomView(0.0, 0.0, 0.0)
        with self.assertRaisesRegex(ValueError, "rho must be finite and positive"):
            native.ZoomParams(rho=0.0)

    def test_installed_extension_runs_hierarchy_layouts(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")

        from gpui_toolkit import d3rs

        leaf_a = native.HierarchyNode("a", 1.0)
        leaf_c = native.HierarchyNode("c", 3.0)
        branch = native.HierarchyNode("b", 2.0, (leaf_c,))
        root = native.HierarchyNode("root", children=(leaf_a, branch))

        summed = root.sum()
        self.assertEqual(summed.values, (6.0, 1.0, 5.0, 3.0))
        self.assertEqual(root.count().values, (4.0, 1.0, 2.0, 1.0))
        callback_sum = root.sum(
            lambda item: {"root": 0, "a": 4, "b": 0, "c": 6}[item]
        )
        self.assertEqual(callback_sum.values, (10.0, 4.0, 6.0, 6.0))
        self.assertEqual(root.try_sum(lambda _: 1.0).values, (4.0, 1.0, 2.0, 1.0))

        metadata = summed.snapshot()
        self.assertEqual(
            [(item.parent, item.children, item.depth, item.height) for item in metadata],
            [
                (None, (1, 2), 0, 2),
                (0, (), 1, 0),
                (0, (3,), 1, 1),
                (2, (), 2, 0),
            ],
        )
        self.assertEqual(metadata[2].data, "b")

        resized = native.TreemapLayout().size(640.0, 480.0).padding(2.0)
        self.assertEqual(native.TreemapLayout()._width, 1.0)
        rectangles = resized.layout(summed)
        self.assertEqual(
            [item.node.data for item in rectangles], ["root", "a", "b", "c"]
        )
        self.assertEqual(rectangles[0].value, 6.0)
        self.assertEqual(len(native.PartitionLayout().layout(root)), 4)

        circles = native.PackLayout().size(300.0, 200.0).layout(root)
        self.assertEqual(len(circles), 4)
        self.assertGreater(circles[0].radius, 0.0)
        tree_points = native.TreeLayout().size(100.0, 80.0).layout(root)
        self.assertEqual(len(tree_points), 4)
        positioned = summed.snapshot(tree_points)
        self.assertEqual(
            [(item.x, item.y) for item in positioned],
            [(item.x, item.y) for item in tree_points],
        )
        self.assertEqual(
            len(native.ClusterLayout().node_size(12.0, 8.0).try_layout(root)), 4
        )
        separation_calls: list[tuple[str, str]] = []

        def separate(left: native.HierarchyNode, right: native.HierarchyNode) -> float:
            separation_calls.append((str(left.data), str(right.data)))
            return 4.0

        separated = native.TreeLayout().node_size(10.0, 10.0).separation(separate)
        separated_points = separated.layout(root)
        self.assertTrue(separation_calls)
        self.assertGreater(
            max(point.y for point in separated_points),
            max(
                point.y
                for point in native.TreeLayout().node_size(10.0, 10.0).layout(root)
            ),
        )

        def fail_separation(
            _left: native.HierarchyNode, _right: native.HierarchyNode
        ) -> float:
            raise RuntimeError("separation callback failed")

        with self.assertRaisesRegex(RuntimeError, "separation callback failed"):
            native.ClusterLayout().separation(fail_separation).layout(root)
        self.assertIs(d3rs.HierarchyNode, native.HierarchyNode)

        visited: list[str] = []
        self.assertIs(root.each(lambda node: visited.append(str(node.data))), root)
        self.assertEqual(visited, ["root", "a", "b", "c"])
        self.assertEqual(
            [
                node.data
                for node in root.sort(
                    lambda node: str(node.data), reverse=True
                ).children
            ],
            ["b", "a"],
        )
        self.assertEqual(
            [node.data for node in root.sort(lambda left, right: len(str(left.data)) - len(str(right.data))).children],
            ["a", "b"],
        )
        with self.assertRaisesRegex(ValueError, "shared"):
            native.HierarchyNode("bad", children=(leaf_a, leaf_a)).sum()
        with self.assertRaises(native.HierarchyError) as error:
            root.try_sum(lambda item: -1.0 if item == "a" else 0.0)
        self.assertEqual(error.exception.kind, native.HierarchyErrorKind.NEGATIVE_VALUE)
        self.assertEqual(error.exception.node_index, 1)
        with self.assertRaisesRegex(ValueError, "non-negative"):
            native.TreemapLayout().size(-1.0, 1.0).layout(root)
        with self.assertRaises(native.HierarchyError) as layout_error:
            native.PackLayout().padding(float("nan")).layout(root)
        self.assertEqual(
            layout_error.exception.kind,
            native.HierarchyErrorKind.NON_FINITE_LAYOUT_PADDING,
        )

    def test_installed_extension_runs_lod_pyramid_and_m4(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")
        from gpui_toolkit import d3rs

        x = [0.0, 0.1, 0.2, 0.3, 0.4, 0.5]
        y = [0.0, 1.0, 99.0, 2.0, -5.0, 0.0]
        indices = d3rs.m4_indices(x, y, 0.0, 1.0, 2)
        self.assertIn(2, indices)
        self.assertIn(4, indices)
        self.assertLessEqual(len(indices), 8)
        self.assertEqual(
            d3rs.m4_point_indices([(0.0, 0.0), (0.5, 4.0), (1.0, 1.0)], 2),
            [0, 1, 2],
        )

        bounds = d3rs.LodBounds.new(0.0, 1.0, 0.0, 1.0)
        pyramid = d3rs.DensityPyramid.build(
            [0.125, 0.375, 0.625, 0.875],
            [0.125, 0.375, 0.625, 0.875],
            bounds,
            4,
        )
        self.assertEqual(pyramid.bounds(), bounds)
        self.assertEqual(pyramid.level_count(), 3)
        grid = pyramid.compose(bounds, 4, 4, 1)
        self.assertIsNotNone(grid)
        assert grid is not None
        self.assertEqual((grid.width, grid.height, grid.level), (4, 4, 0))
        self.assertAlmostEqual(sum(grid.values), 4.0)
        with self.assertRaisesRegex(d3rs.LodError, "base dimension"):
            d3rs.DensityPyramid.build([0.0], [0.0], bounds, 3)

    def test_installed_extension_runs_contour_and_density(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")

        from gpui_toolkit import d3rs

        values = [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]
        generator = (
            native.ContourGenerator(3, 3)
            .x_values((10.0, 100.0, 1000.0))
            .y(0.0, 2.0)
            .x_log_interpolation()
            .upsample_factor(2)
        )
        self.assertEqual(native.ContourGenerator(3, 3)._upsample_factor, 1)

        generated = generator.contour(values, 0.5)
        self.assertEqual(generated.value, 0.5)
        self.assertTrue(generated.coordinates)
        self.assertTrue(generated.coordinates[0].is_closed())
        self.assertTrue(math.isfinite(generated.coordinates[0].area()))
        self.assertEqual(len(generator.contours(values, (0.25, 0.5, 0.75))), 3)
        self.assertTrue(generator.contour_segments(values, (0.5,)))
        bands = generator.contour_bands(values, (0.25, 0.75))
        self.assertEqual(len(bands), 1)
        self.assertEqual(bands[0].mid_value(), 0.5)
        reused_bands = [native.ContourBand.new(-1.0, -0.5)]
        self.assertIsNone(
            generator.contour_bands_into(values, (0.25, 0.75), reused_bands)
        )
        self.assertEqual(reused_bands, bands)
        reused_contour = generator.contour_into(
            values, 0.5, native.Contour.new(-1.0)
        )
        self.assertEqual(reused_contour, generated)
        self.assertIs(d3rs.ContourGenerator, native.ContourGenerator)

        ring = native.ContourRing.new(((0.0, 0.0), (1.0, 0.0), (0.0, 0.0)))
        self.assertTrue(ring.is_closed())
        self.assertEqual(
            native.Contour(0.5).add_ring(ring).coordinates,
            (ring,),
        )
        invalid_ring = native.ContourRing.new(
            ((0.0, 0.0), (float("nan"), 1.0), (0.0, 0.0))
        )
        self.assertTrue(math.isnan(invalid_ring.area()))
        with self.assertRaises(native.ContourRingError) as ring_error:
            invalid_ring.try_area()
        self.assertEqual((ring_error.exception.index, ring_error.exception.coordinate), (1, "x"))

        estimator = (
            native.DensityEstimator()
            .size(8, 6)
            .x(0.0, 1.0)
            .y(0.0, 1.0)
            .bandwidth(0.2)
            .kernel(native.DensityKernel.EPANECHNIKOV)
        )
        self.assertEqual(native.DensityEstimator()._width, 100)
        density = estimator.estimate(((0.25, 0.25), (0.75, 0.75)))
        self.assertEqual((density.width, density.height, len(density)), (8, 6, 48))
        self.assertGreaterEqual(density.at(2, 2), 0.0)
        weighted = estimator.estimate_weighted(
            ((0.25, 0.25, 1.0), (0.75, 0.75, 2.0))
        )
        self.assertEqual(len(weighted), 48)
        automatic = native.density_2d(((0.0, 0.0), (1.0, 1.0)), 5, 4, 0.2)
        self.assertEqual((automatic.width, automatic.height), (5, 4))
        self.assertAlmostEqual(
            native.gaussian_kernel(0.0, 1.0), 0.3989422804014327
        )
        self.assertEqual(native.epanechnikov_kernel(2.0, 1.0), 0.0)

        self.assertTrue(native.contour_threshold_sturges(0.0, 10.0, 100))
        sample = [float(value) for value in range(100)]
        self.assertTrue(native.contour_threshold_scott(sample, 0.0, 100.0))
        self.assertTrue(
            native.contour_threshold_freedman_diaconis(sample, 0.0, 100.0)
        )

        with self.assertRaisesRegex(ValueError, "does not match"):
            native.contour([0.0], 3, 3, 0.5)
        with self.assertRaisesRegex(ValueError, "strictly increasing"):
            generator.contour_bands(values, (0.75, 0.25))
        invalid_weighted = estimator.estimate_weighted(((0.5, 0.5, -1.0),))
        self.assertEqual(invalid_weighted.values, (0.0,) * 48)
        with self.assertRaises(native.DensityError) as density_error:
            estimator.try_estimate_weighted(((0.5, 0.5, -1.0),))
        self.assertIn("non-negative", density_error.exception.message)
        self.assertEqual(density_error.exception.path, "density.estimator.weighted_points")
        invalid_estimator = native.DensityEstimator().size(0, 4)
        self.assertEqual(invalid_estimator.estimate(((0.5, 0.5),)).values, ())
        with self.assertRaises(native.DensityError):
            invalid_estimator.try_estimate(((0.5, 0.5),))
        self.assertEqual(
            native.density_2d(((float("nan"), 0.0),), 3, 2, 0.2).values,
            (0.0,) * 6,
        )
        with self.assertRaises(native.DensityError):
            native.try_density_2d(((float("nan"), 0.0),), 3, 2, 0.2)
        with self.assertRaisesRegex(ValueError, "positive"):
            native.gaussian_kernel(0.0, 0.0)

    def test_installed_extension_runs_polygon_delaunay_and_voronoi(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")

        from gpui_toolkit import d3rs

        self.assertTrue(d3rs.Renderer2D.VELLO.is_vello())
        self.assertFalse(d3rs.Renderer2D.LEGACY.is_vello())
        self.assertIs(px.Renderer2D, d3rs.Renderer2D)
        self.assertIs(px.VelloBackend, d3rs.VelloBackend)
        self.assertEqual(d3rs.VelloBackend.AUTO.value, "auto")
        with self.assertRaises(ValueError):
            d3rs.VelloBackend("metal")

        square = ((0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0))
        self.assertEqual(d3rs.polygon_area(polygon=square), 100.0)
        self.assertEqual(abs(d3rs.polygon_area_signed(square)), 100.0)
        self.assertEqual(d3rs.polygon_centroid(square), (5.0, 5.0))
        self.assertTrue(d3rs.polygon_contains(polygon=square, point=(5.0, 5.0)))
        self.assertFalse(d3rs.polygon_contains(square, (20.0, 5.0)))
        self.assertEqual(d3rs.polygon_length(square), 40.0)
        self.assertEqual(len(d3rs.polygon_hull((*square, (5.0, 5.0)))), 4)
        with self.assertRaisesRegex(ValueError, "must be finite"):
            d3rs.polygon_area(((0.0, 0.0), (float("nan"), 1.0)))

        points = (*square, (5.0, 5.0))
        triangulation = native.Delaunay(points)
        self.assertEqual(len(triangulation), 5)
        self.assertFalse(triangulation.is_empty())
        self.assertEqual(triangulation.points(), points)
        self.assertEqual(triangulation.point(4), (5.0, 5.0))
        self.assertEqual(triangulation.find(5.1, 5.1), 4)
        self.assertEqual(triangulation.find_within_radius(5.1, 5.1, 1.0), 4)
        self.assertIsNone(triangulation.find_within_radius(50.0, 50.0, 1.0))
        self.assertEqual(triangulation.triangle_count(), len(triangulation.triangles()))
        self.assertTrue(triangulation.edges())
        self.assertTrue(triangulation.hull())
        self.assertEqual(triangulation.hull_polygon()[0], triangulation.hull_polygon()[-1])
        self.assertIn("M", triangulation.render_to_path())
        self.assertIn("M", triangulation.render_hull_to_path())

        voronoi = triangulation.voronoi((0.0, 0.0, 10.0, 10.0))
        self.assertEqual(
            native.Voronoi.new(triangulation, (0.0, 0.0, 10.0, 10.0)).bounds(),
            voronoi.bounds(),
        )
        self.assertEqual(voronoi.bounds(), (0.0, 0.0, 10.0, 10.0))
        self.assertEqual(voronoi.cell_count(), 5)
        self.assertEqual(len(voronoi.bounds_polygon()), 5)
        self.assertEqual(len(voronoi.indexed_cell_polygons()), 5)
        self.assertTrue(voronoi.contains(4, 5.0, 5.0))
        self.assertTrue(voronoi.neighbors(4))
        self.assertIn("M", voronoi.render_to_path())
        self.assertIn("M", voronoi.render_bounds_to_path())
        self.assertIsNotNone(voronoi.render_cell_to_path(4))
        triangulation_buffer = io.StringIO("prefix:")
        triangulation_buffer.seek(0, io.SEEK_END)
        self.assertIsNone(triangulation.render_to_path_into(triangulation_buffer))
        self.assertTrue(triangulation_buffer.getvalue().startswith("prefix:M"))
        voronoi_buffer = io.StringIO()
        self.assertIsNone(voronoi.render_to_path_into(voronoi_buffer))
        self.assertEqual(voronoi_buffer.getvalue(), voronoi.render_to_path())
        cell_buffer = io.StringIO()
        self.assertTrue(voronoi.render_cell_to_path_into(4, cell_buffer))
        self.assertEqual(cell_buffer.getvalue(), voronoi.render_cell_to_path(4))
        self.assertFalse(voronoi.render_cell_to_path_into(99, io.StringIO()))
        self.assertEqual(triangulation.len(), len(triangulation))
        self.assertIs(triangulation.inner(), triangulation)
        self.assertIs(d3rs.Delaunay, native.Delaunay)

        with self.assertRaises(native.DelaunayError) as point_error:
            native.Delaunay(((math.nan, 0.0),))
        self.assertEqual(
            point_error.exception.kind,
            native.DelaunayErrorKind.NON_FINITE_POINT_COORDINATE,
        )
        self.assertEqual(
            (point_error.exception.index, point_error.exception.coordinate), (0, "x")
        )
        self.assertIsNone(triangulation.find(math.nan, 0.0))
        with self.assertRaises(native.DelaunayError) as query_error:
            triangulation.try_find(math.nan, 0.0)
        self.assertEqual(
            query_error.exception.kind,
            native.DelaunayErrorKind.NON_FINITE_QUERY_COORDINATE,
        )
        self.assertIsNone(triangulation.find_within_radius(0.0, 0.0, -1.0))
        with self.assertRaises(native.DelaunayError) as radius_error:
            triangulation.try_find_within_radius(0.0, 0.0, -1.0)
        self.assertEqual(
            radius_error.exception.kind, native.DelaunayErrorKind.NEGATIVE_RADIUS
        )
        with self.assertRaises(native.DelaunayError) as bounds_error:
            triangulation.voronoi((10.0, 0.0, 0.0, 10.0))
        self.assertEqual(
            bounds_error.exception.kind,
            native.DelaunayErrorKind.REVERSED_VORONOI_BOUNDS,
        )

    def test_installed_extension_runs_immutable_force_simulation(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")

        from gpui_toolkit import d3rs

        nodes = (
            native.SimulationNode(0, -10.0, -10.0).fix_x(-10.0).fix_y(-10.0),
            native.SimulationNode(1, 10.0, -10.0),
            native.SimulationNode(2, 10.0, 10.0),
            native.SimulationNode(3, -10.0, 10.0),
        )
        x_force = native.ForceX(0.0).strength(0.05)
        self.assertEqual(native.ForceX(0.0)._strength, 0.1)
        radial = native.ForceRadial.with_center(12.0, 0.0, 0.0).strength(0.02)
        collide = (
            native.ForceCollide.with_radius(2.0)
            .radii((2.0, 2.5, 2.0, 2.5))
            .strength(0.8)
            .iterations(2)
        )
        many_body = (
            native.ForceManyBody()
            .strength(-5.0)
            .distance_min(1.0)
            .distance_max(100.0)
        )
        link = native.ForceLink(((0, 1), (1, 2), (2, 3), (3, 0))).distance(20.0)
        simulation = (
            native.Simulation(nodes)
            .alpha(0.8)
            .alpha_target(0.1)
            .velocity_decay(0.7)
            .force(native.ForceCenter(0.0, 0.0))
            .force(x_force)
            .force(native.ForceY(0.0).strength(0.05))
            .force(radial)
            .force(collide)
            .force(many_body)
            .force(link)
        )
        result = simulation.tick(5)

        self.assertEqual(simulation.nodes(), nodes)
        self.assertEqual(result.nodes()[0].x, -10.0)
        self.assertEqual(result.nodes()[0].y, -10.0)
        self.assertTrue(
            all(
                math.isfinite(value)
                for node in result.nodes()
                for value in (node.x, node.y, node.vx, node.vy)
            )
        )
        self.assertLess(result.current_alpha, simulation.current_alpha)
        self.assertIs(d3rs.Simulation, native.Simulation)

        with self.assertRaisesRegex(ValueError, "node count"):
            native.Simulation(nodes).force(
                native.ForceCollide().radii((1.0,))
            ).tick()
        with self.assertRaisesRegex(ValueError, "link"):
            native.Simulation(nodes).force(native.ForceLink(((0, 99),))).tick()
        with self.assertRaisesRegex(ValueError, "radius"):
            native.Simulation(nodes).force(native.ForceCollide.with_radius(-1.0)).tick()
        with self.assertRaisesRegex(ValueError, "distance"):
            native.Simulation(nodes).force(
                native.ForceManyBody().distance_min(10.0).distance_max(1.0)
            ).tick()

    def test_installed_extension_runs_arc_symbol_link_and_pie_shapes(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")

        from gpui_toolkit import d3rs

        datum = (
            native.ArcDatum.new()
            .inner_radius(20.0)
            .outer_radius(50.0)
            .start_angle(0.0)
            .end_angle(math.pi)
            .pad_angle(0.02)
            .corner_radius(2.0)
        )
        self.assertEqual(native.ArcDatum()._inner_radius, 0.0)
        arc = native.Arc.new().center(100.0, 80.0)
        path = arc.generate(datum, segments=24)
        self.assertTrue(path.svg.startswith("M"))
        self.assertGreater(len(path.points), 20)
        self.assertEqual(path.to_svg_string(), arc.path_string(datum))
        cx, cy = datum.centroid()
        self.assertTrue(math.isfinite(cx) and math.isfinite(cy))
        self.assertEqual(len(native.arc_points(datum, 8)), 19)

        for symbol_type in native.SymbolType:
            symbol = native.Symbol.new(symbol_type, 64.0)
            symbol_path = symbol.generate_at(10.0, 20.0)
            self.assertTrue(symbol_path.svg.startswith("M"))
            self.assertTrue(symbol_path.points)
            self.assertGreater(symbol.radius(), 0.0)
        self.assertEqual(native.Symbol.circle(64.0)._symbol_type, native.SymbolType.CIRCLE)
        self.assertIs(d3rs.Symbol, native.Symbol)

        link = native.Link.from_points((0.0, 10.0), (100.0, 80.0))
        self.assertEqual(link, native.Link.new(0.0, 10.0, 100.0, 80.0))
        link.validate()
        self.assertTrue(native.link_horizontal(link).startswith("M"))
        self.assertTrue(native.link_vertical(link).startswith("M"))
        self.assertTrue(
            native.link_step(link, native.LinkDirection.HORIZONTAL).startswith("M")
        )
        radial_link = native.RadialLink.new(0.0, 10.0, math.pi / 2.0, 20.0)
        cartesian = radial_link.to_cartesian(50.0, 50.0)
        self.assertAlmostEqual(cartesian.source_x, 60.0)
        self.assertAlmostEqual(cartesian.target_y, 70.0)
        self.assertTrue(native.link_radial(radial_link, 50.0, 50.0).startswith("M"))

        data = ({"name": "a", "value": 1.0}, {"name": "b", "value": 3.0})
        pie = (
            native.Pie.new()
            .inner_radius(20.0)
            .outer_radius(80.0)
            .pad_angle(0.01)
            .sort(True)
            .sort_descending(True)
        )
        slices = pie.generate(data, lambda item: item["value"])
        self.assertEqual([item.data["name"] for item in slices], ["b", "a"])
        self.assertEqual([item.index for item in slices], [1, 0])
        self.assertAlmostEqual(
            sum(item.arc._end_angle - item.arc._start_angle for item in slices),
            math.tau - 0.02,
        )
        self.assertEqual(len(native.pie((1.0, 2.0), 100.0)), 2)
        self.assertEqual(len(native.donut((1.0, 2.0), 50.0, 100.0)), 2)
        self.assertEqual(len(native.half_pie((1.0, 2.0), 100.0)), 2)

        permissive = native.Arc.new().generate(native.ArcDatum.new().inner_radius(-1.0))
        self.assertTrue(permissive.svg)
        with self.assertRaises(native.ArcGenerationError) as negative_radius:
            native.Arc.new().try_generate(native.ArcDatum.new().inner_radius(-1.0))
        self.assertEqual(
            negative_radius.exception.kind,
            native.ArcGenerationErrorKind.NEGATIVE_PARAMETER,
        )
        self.assertEqual(negative_radius.exception.parameter, "inner_radius")
        self.assertEqual(negative_radius.exception.value, -1.0)
        with self.assertRaises(native.ArcGenerationError) as reversed_radii:
            native.Arc.new().try_generate(
                native.ArcDatum.new().inner_radius(20.0).outer_radius(10.0)
            )
        self.assertEqual(
            reversed_radii.exception.kind,
            native.ArcGenerationErrorKind.INNER_RADIUS_EXCEEDS_OUTER_RADIUS,
        )
        self.assertEqual(reversed_radii.exception.inner_radius, 20.0)
        self.assertEqual(reversed_radii.exception.outer_radius, 10.0)
        self.assertGreater(len(native.arc_points(datum, 0)), 0)
        with self.assertRaises(native.ArcGenerationError) as zero_segments:
            native.try_arc_points(datum, 0)
        self.assertEqual(
            zero_segments.exception.kind,
            native.ArcGenerationErrorKind.ZERO_SEGMENTS,
        )
        self.assertTrue(native.Symbol.circle(-1.0).generate().svg)
        with self.assertRaises(native.SymbolGenerationError) as negative_size:
            native.Symbol.circle(-1.0).try_generate()
        self.assertEqual(
            negative_size.exception.kind,
            native.SymbolGenerationErrorKind.NEGATIVE_SIZE,
        )
        self.assertEqual(negative_size.exception.size, -1.0)
        with self.assertRaises(native.SymbolGenerationError) as non_finite_coordinate:
            native.Symbol.circle(64.0).try_generate_at(math.nan, 0.0)
        self.assertEqual(
            non_finite_coordinate.exception.kind,
            native.SymbolGenerationErrorKind.NON_FINITE_COORDINATE,
        )
        self.assertEqual(non_finite_coordinate.exception.coordinate, "x")
        with self.assertRaises(native.SymbolGenerationError):
            native.try_symbol_radius(native.SymbolType.CIRCLE, math.inf)
        invalid_link = native.Link.new(math.nan, 0.0, 1.0, 1.0)
        self.assertTrue(native.link_horizontal(invalid_link).startswith("M"))
        with self.assertRaises(native.LinkGenerationError) as non_finite_link:
            native.try_link_horizontal(invalid_link)
        self.assertEqual(
            non_finite_link.exception.kind,
            native.LinkGenerationErrorKind.NON_FINITE_PARAMETER,
        )
        self.assertEqual(non_finite_link.exception.parameter, "source_x")
        negative_radial = native.RadialLink.new(0.0, -1.0, 1.0, 2.0)
        self.assertIsInstance(negative_radial.to_cartesian(), native.Link)
        with self.assertRaises(native.LinkGenerationError) as negative_radius:
            negative_radial.try_to_cartesian()
        self.assertEqual(
            negative_radius.exception.kind,
            native.LinkGenerationErrorKind.NEGATIVE_RADIUS,
        )
        self.assertEqual(negative_radius.exception.parameter, "source_radius")
        self.assertEqual(len(native.pie((1.0, -1.0), 100.0)), 2)
        with self.assertRaises(native.PieLayoutError) as negative_value:
            native.try_pie((1.0, -1.0), 100.0)
        self.assertEqual(
            negative_value.exception.kind,
            native.PieLayoutErrorKind.NEGATIVE_VALUE,
        )
        self.assertEqual(negative_value.exception.index, 1)
        self.assertEqual(negative_value.exception.value, -1.0)
        with self.assertRaises(native.PieLayoutError) as invalid_radius:
            native.Pie.new().outer_radius(-1.0).try_generate((1.0,))
        self.assertEqual(
            invalid_radius.exception.kind,
            native.PieLayoutErrorKind.NEGATIVE_LAYOUT_PARAMETER,
        )
        self.assertEqual(invalid_radius.exception.parameter, "outer_radius")
        with self.assertRaises(native.PieLayoutError) as non_finite:
            native.Pie.new().try_generate((math.nan,))
        self.assertEqual(
            non_finite.exception.kind,
            native.PieLayoutErrorKind.NON_FINITE_VALUE,
        )
        with self.assertRaisesRegex(TypeError, "sort must be bool"):
            native.Pie.new().sort(1)

    def test_installed_extension_runs_stack_curve_and_radial_shapes(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")

        from gpui_toolkit import d3rs

        data = ((1.0, 2.0), (3.0, 4.0), (2.0, 1.0))
        layout = (
            native.Stack()
            .keys(("left", "right"))
            .order(native.StackOrder.REVERSE)
            .offset(native.StackOffset.EXPAND)
        )
        series = layout.generate(data)
        self.assertEqual([item.key for item in series], ["right", "left"])
        self.assertEqual(layout._order, native.StackOrder.REVERSE)
        self.assertEqual(native.Stack()._order, native.StackOrder.NONE)
        self.assertEqual(series[0].get(0), (0.0, 2.0 / 3.0))
        self.assertIsNone(series[0].get(99))
        self.assertEqual(len(native.stack(data)), 2)
        self.assertEqual(len(native.stack_expand(data)), 2)
        self.assertEqual(len(native.streamgraph(data)), 2)
        checked = native.Stack.new().keys(("left", "right"))
        ragged = ((1.0, 2.0), (3.0,))
        self.assertEqual(len(checked.generate(ragged)), 2)
        with self.assertRaises(native.StackLayoutError) as mismatch:
            checked.try_generate(ragged)
        self.assertEqual(
            mismatch.exception.kind,
            native.StackLayoutErrorKind.ROW_LENGTH_MISMATCH,
        )
        self.assertEqual(mismatch.exception.row_index, 1)
        self.assertEqual(mismatch.exception.expected, 2)
        self.assertEqual(mismatch.exception.actual, 1)
        with self.assertRaises(native.StackLayoutError) as non_finite:
            checked.try_generate(((1.0, math.nan),))
        self.assertEqual(
            non_finite.exception.kind,
            native.StackLayoutErrorKind.NON_FINITE_VALUE,
        )
        self.assertEqual(non_finite.exception.row_index, 0)
        self.assertEqual(non_finite.exception.series_index, 1)
        self.assertEqual(len(native.stack(ragged)), 2)
        with self.assertRaises(native.StackLayoutError):
            native.try_stack(ragged)
        self.assertIs(d3rs.Stack, native.Stack)

        points = ((0.0, 0.0), (10.0, 20.0), (20.0, 5.0))
        self.assertEqual(native.Curve.linear().interpolate(points), points)
        self.assertGreater(len(native.Curve.basis().interpolate(points)), len(points))
        self.assertGreater(
            len(native.Curve.catmull_rom(0.5).interpolate(points)), len(points)
        )
        self.assertEqual(native.Curve.cardinal(-1.0).parameter, 0.0)
        self.assertEqual(native.Curve.cardinal(2.0).parameter, 1.0)
        self.assertEqual(native.Curve.catmull_rom(-1.0).parameter, 0.0)
        self.assertEqual(native.Curve.catmull_rom(2.0).parameter, 1.0)
        interpolated: list[tuple[float, float]] = [(99.0, 99.0)]
        native.Curve.step().interpolate_into(points, interpolated)
        self.assertEqual(native.Curve.step().subdivisions(), 2)
        self.assertGreater(len(interpolated), len(points))
        for curve in (
            native.Curve.step_before(),
            native.Curve.step_after(),
            native.Curve.basis_closed(),
            native.Curve.basis_open(),
            native.Curve.bundle(0.5),
            native.Curve.cardinal_closed(0.5),
            native.Curve.cardinal_open(0.5),
            native.Curve.catmull_rom_closed(0.5),
            native.Curve.catmull_rom_open(0.5),
            native.Curve.monotone_x(),
            native.Curve.monotone_y(),
            native.Curve.natural(),
        ):
            self.assertTrue(curve.interpolate(points))

        radial_points = (
            native.RadialPoint(0.0, 10.0),
            native.RadialPoint(math.pi / 2.0, 20.0),
            native.RadialPoint(math.pi, 10.0),
        )
        cartesian = radial_points[0].to_cartesian(5.0, 7.0)
        self.assertEqual(cartesian, (15.0, 7.0))
        restored = native.RadialPoint.from_cartesian(*cartesian, 5.0, 7.0)
        self.assertAlmostEqual(restored.angle, 0.0)
        self.assertAlmostEqual(restored.radius, 10.0)

        line_config = (
            native.RadialLineConfig(5.0, 7.0)
            .curve(native.CurveKind.STEP)
            .closed(True)
        )
        self.assertTrue(native.radial_line(radial_points, line_config).endswith("Z"))
        area_config = (
            native.RadialAreaConfig(5.0, 7.0)
            .inner_radius(2.0)
            .curve(native.Curve.natural())
        )
        self.assertTrue(native.radial_area(radial_points, area_config).endswith("Z"))
        self.assertEqual(len(native.polar_grid_circles(0.0, 0.0, (10.0, 20.0))), 2)
        self.assertEqual(
            len(native.polar_grid_rays(0.0, 0.0, 20.0, (0.0, math.pi))), 2
        )
        self.assertEqual(native.RadialPoint.new(0.0, 10.0), radial_points[0])
        self.assertEqual(
            native.RadialLineConfig.new(5.0, 7.0), native.RadialLineConfig(5.0, 7.0)
        )
        self.assertEqual(
            native.RadialAreaConfig.new(5.0, 7.0), native.RadialAreaConfig(5.0, 7.0)
        )
        self.assertEqual(
            native.try_radial_line(radial_points, line_config),
            native.radial_line(radial_points, line_config),
        )
        self.assertEqual(
            native.try_radial_area(radial_points, area_config),
            native.radial_area(radial_points, area_config),
        )

        with self.assertRaisesRegex(native.StackLayoutError, "expected 2"):
            layout.try_generate(((1.0,),))
        self.assertEqual(native.RadialPoint(0.0, -1.0).to_cartesian(), (-1.0, 0.0))
        with self.assertRaises(native.RadialGenerationError) as negative_point:
            native.RadialPoint(0.0, -1.0).try_to_cartesian()
        self.assertEqual(
            negative_point.exception.kind,
            native.RadialGenerationErrorKind.NEGATIVE_POINT_RADIUS,
        )
        with self.assertRaises(native.RadialGenerationError) as negative_grid:
            native.try_polar_grid_circles(0.0, 0.0, (-1.0,))
        self.assertEqual(
            negative_grid.exception.kind,
            native.RadialGenerationErrorKind.NEGATIVE_GRID_RADIUS,
        )
        with self.assertRaises(native.RadialGenerationError) as nonfinite_point:
            native.try_radial_line(
                (native.RadialPoint(math.nan, 1.0),),
                line_config,
            )
        self.assertEqual(
            nonfinite_point.exception.kind,
            native.RadialGenerationErrorKind.NON_FINITE_POINT,
        )
        self.assertEqual(nonfinite_point.exception.index, 0)
        self.assertEqual(
            nonfinite_point.exception.field, native.RadialPointField.ANGLE
        )
        with self.assertRaises(native.RadialGenerationError) as negative_inner:
            native.RadialAreaConfig.new(0.0, 0.0).inner_radius(-1.0).try_generate(
                radial_points
            )
        self.assertEqual(
            negative_inner.exception.kind,
            native.RadialGenerationErrorKind.NEGATIVE_RADIUS,
        )
        self.assertEqual(negative_inner.exception.parameter, "inner_radius")
        with self.assertRaisesRegex(ValueError, "does not accept"):
            native.Curve(native.CurveKind.LINEAR, 1.0).interpolate(points)
        with self.assertRaisesRegex(ValueError, r"points\[0\]\.x"):
            native.Curve.linear().interpolate(((math.nan, 0.0),))

    def test_installed_extension_runs_path_and_area_shapes(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")

        from gpui_toolkit import d3rs

        point = native.Point(0.0, 0.0)
        self.assertEqual(point.distance(native.Point(3.0, 4.0)), 5.0)
        self.assertEqual(point.lerp(native.Point(10.0, 20.0), 0.25), native.Point(2.5, 5.0))

        empty_builder = native.PathBuilder()
        builder = (
            empty_builder.move_to(0.0, 0.0)
            .line_to(10.0, 0.0)
            .horizontal_line_to(20.0)
            .vertical_line_to(10.0)
            .quadratic_curve_to(25.0, 15.0, 30.0, 10.0)
            .cubic_curve_to(35.0, 5.0, 40.0, 15.0, 45.0, 10.0)
            .arc(45.0, 15.0, 5.0, -math.pi / 2.0, math.pi / 2.0)
            .elliptical_arc(5.0, 3.0, 0.0, False, True, 55.0, 10.0)
            .rect(60.0, 0.0, 10.0, 10.0)
            .close_path()
        )
        self.assertEqual(empty_builder.current_point(), native.Point())
        self.assertEqual(builder.current_point(), native.Point(0.0, 0.0))
        path = builder.build()
        self.assertFalse(path.is_empty())
        self.assertEqual(len(path.commands()), 10)
        self.assertTrue(path.to_svg_string().startswith("M"))
        self.assertTrue(path.flatten(0.25))
        self.assertIsNotNone(path.bounds())
        svg_parts: list[str] = []
        path.write_svg_string(svg_parts)
        self.assertEqual(svg_parts, [path.to_svg_string()])
        self.assertIs(d3rs.PathBuilder, native.PathBuilder)

        rows = (
            {"x": 0.0, "low": 1.0, "high": 4.0, "ok": True},
            {"x": 1.0, "low": math.nan, "high": math.nan, "ok": False},
            {"x": 2.0, "low": 2.0, "high": 6.0, "ok": True},
        )
        area = (
            native.Area()
            .x(lambda item: item["x"])
            .y0(lambda item: item["low"])
            .y1(lambda item: item["high"])
            .defined(lambda item: item["ok"])
            .curve(native.Curve.step())
        )
        area_path = area.generate(rows)
        self.assertFalse(area_path.is_empty())
        self.assertGreaterEqual(
            sum(command.kind is native.PathCommandKind.MOVE_TO for command in area_path.commands()),
            2,
        )
        combined = area.generate_into(rows, native.PathBuilder().move_to(-1.0, -1.0))
        self.assertGreater(len(combined.build().commands()), len(area_path.commands()))
        self.assertEqual(native.Area()._x, native.Area()._x)

        simple = native.SimpleArea((0.0, 1.0), (0.0, 0.0), (2.0, 3.0))
        self.assertEqual(len(simple.points()), 5)
        self.assertFalse(simple.path().is_empty())
        self.assertEqual(
            native.area_points(
                rows[::2],
                lambda item: item["x"],
                lambda item: item["low"],
                lambda item: item["high"],
            ),
            (
                native.Point(0.0, 4.0),
                native.Point(2.0, 6.0),
                native.Point(2.0, 2.0),
                native.Point(0.0, 1.0),
                native.Point(0.0, 4.0),
            ),
        )

        with self.assertRaisesRegex(ValueError, "mismatched lengths"):
            native.SimpleArea((0.0,), (), (1.0,)).path()
        with self.assertRaisesRegex(ValueError, "radius"):
            native.PathBuilder().arc(0.0, 0.0, -1.0, 0.0, 1.0).build().bounds()
        with self.assertRaisesRegex(ValueError, "tolerance"):
            path.flatten(0.0)

    def test_installed_extension_runs_chord_layout_and_ribbons(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")

        from gpui_toolkit import d3rs

        matrix = (
            (0.0, 4.0, 2.0),
            (3.0, 0.0, 5.0),
            (1.0, 6.0, 0.0),
        )
        original = native.ChordLayout.new()
        layout = (
            original.pad_angle(0.04)
            .sort_groups(native.ChordSort.DESCENDING)
            .sort_subgroups(native.ChordSort.ASCENDING)
            .sort_chords(native.ChordSort.DESCENDING)
        )
        result = layout.compute(matrix)
        self.assertEqual(original._pad_angle, 0.0)
        self.assertEqual(layout.pad_angle_value, 0.04)
        self.assertEqual(layout.sort_groups_value, native.ChordSort.DESCENDING)
        self.assertEqual(layout.sort_subgroups_value, native.ChordSort.ASCENDING)
        self.assertEqual(layout.sort_chords_value, native.ChordSort.DESCENDING)
        self.assertEqual(len(result.groups), 3)
        self.assertTrue(result.chords)
        self.assertEqual([group.index for group in result.groups], [0, 1, 2])
        self.assertEqual(sum(group.value for group in result.groups), 21.0)
        self.assertIs(d3rs.ChordLayout, native.ChordLayout)

        ribbon = native.RibbonGenerator.new(80.0).center(100.0, 120.0)
        ribbon_path = ribbon.generate_path(result.chords[0])
        self.assertFalse(ribbon_path.is_empty())
        self.assertTrue(ribbon.generate(result.chords[0]).startswith("M"))
        self.assertIsNotNone(ribbon_path.bounds())

        with self.assertRaises(native.ChordLayoutError) as square_error:
            layout.compute(((1.0, 2.0), (3.0,)))
        self.assertEqual(
            square_error.exception.kind,
            native.ChordLayoutErrorKind.NON_SQUARE_MATRIX,
        )
        self.assertEqual(
            (square_error.exception.row, square_error.exception.expected, square_error.exception.actual),
            (1, 2, 1),
        )
        with self.assertRaises(native.ChordLayoutError) as negative_error:
            layout.compute(((0.0, -1.0), (1.0, 0.0)))
        self.assertEqual(
            negative_error.exception.kind,
            native.ChordLayoutErrorKind.NEGATIVE_VALUE,
        )
        with self.assertRaisesRegex(ValueError, "pad_angle"):
            native.ChordLayout().pad_angle(math.nan).compute(((0.0,),))
        with self.assertRaisesRegex(ValueError, "radius"):
            native.RibbonGenerator(-1.0).generate_path(result.chords[0])

    def test_installed_extension_runs_stateful_random_generators(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")

        from gpui_toolkit import d3rs

        rng_a = native.LcgRng(42)
        rng_b = native.LcgRng(42)
        sequence_a = [rng_a.next_f64() for _ in range(4)]
        sequence_b = [rng_b.next_f64() for _ in range(4)]
        self.assertEqual(sequence_a, sequence_b)
        self.assertEqual(len(set(sequence_a)), 4)
        self.assertTrue(all(0 <= rng_a.next_u64(7) < 7 for _ in range(4)))

        constructors = (
            native.RandomUniform.with_seed(-2.0, 3.0, 7),
            native.RandomNormal.with_seed(10.0, 2.0, 7),
            native.RandomLogNormal.with_seed(0.0, 0.5, 7),
            native.RandomExponential.with_seed(2.0, 7),
            native.RandomPoisson.with_seed(3.0, 7),
            native.RandomIrwinHall.with_seed(4, 7),
            native.RandomBates.with_seed(4, 7),
        )
        values = [generator.sample() for generator in constructors]
        self.assertTrue(all(math.isfinite(float(value)) for value in values))
        self.assertNotEqual(
            native.RandomUniform.with_seed(0.0, 1.0, 9).sample(),
            native.RandomUniform.with_seed(0.0, 1.0, 10).sample(),
        )
        self.assertTrue(0.0 <= native.RandomUniform.unit().sample() < 1.0)
        self.assertTrue(math.isfinite(native.RandomNormal.standard().sample()))
        bernoulli = native.RandomBernoulli.with_seed(0.25, 7)
        self.assertIsInstance(bernoulli.sample(), bool)
        self.assertIn(bernoulli.sample_int(), (0, 1))
        self.assertIs(d3rs.RandomUniform, native.RandomUniform)

        with self.assertRaisesRegex(ValueError, "max"):
            native.LcgRng(1).next_u64(0)
        with self.assertRaisesRegex(ValueError, "lambda"):
            native.RandomExponential(0.0)
        with self.assertRaisesRegex(ValueError, "std_dev"):
            native.RandomNormal(0.0, -1.0)
        with self.assertRaisesRegex(ValueError, "n"):
            native.RandomBates(0)

    def test_installed_extension_runs_geo_math_graticules_and_versors(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")

        from gpui_toolkit import d3rs

        self.assertAlmostEqual(native.radians(180.0), math.pi)
        self.assertAlmostEqual(native.degrees(math.pi / 2.0), 90.0)
        self.assertAlmostEqual(native.geo_distance(0.0, 0.0, 90.0, 0.0), math.pi / 2.0)
        line = ((0.0, 0.0), (45.0, 0.0), (90.0, 0.0))
        self.assertAlmostEqual(native.geo_length(line), math.pi / 2.0)
        midpoint = native.geo_interpolate(0.0, 0.0, 90.0, 0.0, 0.5)
        self.assertAlmostEqual(midpoint[0], 45.0)
        self.assertAlmostEqual(midpoint[1], 0.0)
        polygon = ((0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0))
        self.assertGreater(native.geo_area(polygon), 0.0)
        self.assertEqual(native.geo_bounds(polygon), ((0.0, 0.0), (10.0, 10.0)))
        centroid = native.geo_centroid(polygon)
        self.assertAlmostEqual(centroid[0], 5.0, places=2)
        self.assertTrue(native.geo_contains(polygon, 5.0, 5.0))
        self.assertFalse(native.geo_contains(polygon, 20.0, 5.0))
        self.assertIs(d3rs.geo_distance, native.geo_distance)

        original = native.Graticule()
        graticule = (
            original.extent(((-30.0, -20.0), (30.0, 20.0)))
            .step((10.0, 10.0))
            .precision(1.0)
        )
        self.assertNotEqual(original._config.extent_major, graticule._config.extent_major)
        self.assertTrue(graticule.lines())
        outline = graticule.outline()
        self.assertGreater(len(outline), 4)
        self.assertEqual(outline[0], outline[-1])
        self.assertTrue(native.graticule10())

        rotation = native.Rotation().angles(20.0, -10.0, 5.0)
        rotated = rotation.rotate(12.0, 34.0)
        restored = rotation.invert(*rotated)
        self.assertAlmostEqual(restored[0], 12.0, places=9)
        self.assertAlmostEqual(restored[1], 34.0, places=9)

        identity = native.Versor()
        self.assertEqual(identity.to_array(), (1.0, 0.0, 0.0, 0.0))
        self.assertEqual(native.Versor.from_array(identity.to_array()), identity)
        self.assertAlmostEqual(identity.norm(), 1.0)
        angles = native.Versor.from_angles(20.0, 10.0, 5.0)
        self.assertAlmostEqual(angles.normalize().norm(), 1.0)
        self.assertEqual(identity.multiply(angles), angles)
        self.assertAlmostEqual(angles.dot(angles), angles.norm() ** 2)
        self.assertEqual(angles.conjugate().conjugate(), angles)
        cartesian = native.Versor.spherical_to_cartesian(0.0, 0.0)
        self.assertEqual(cartesian, (1.0, 0.0, 0.0))
        delta = native.Versor.delta(cartesian, (0.0, 1.0, 0.0))
        self.assertAlmostEqual(delta.norm(), 1.0)
        self.assertAlmostEqual(identity.slerp(angles, 0.0).w, 1.0)
        self.assertEqual(identity.rotate_spherical(0.25, 0.5), (0.25, 0.5))
        self.assertEqual(
            native.Versor.rotate_degrees((0.0, 0.0, 0.0), 12.0, 34.0),
            (12.0, 34.0),
        )

        with self.assertRaisesRegex(ValueError, r"coordinates\[0\]"):
            native.geo_length(((math.nan, 0.0),))
        with self.assertRaisesRegex(ValueError, "step_major"):
            native.Graticule().step_major((0.0, 10.0)).lines()
        with self.assertRaisesRegex(ValueError, "four"):
            native.Versor.from_array((1.0, 0.0))

    def test_installed_extension_runs_geo_projections(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")

        from gpui_toolkit import d3rs

        projections = (
            native.Mercator(),
            native.Equirectangular(),
            native.Orthographic(),
            native.Stereographic(),
            native.TransverseMercator(),
            native.ConicEqualArea(),
            native.Albers(),
        )
        for projection in projections:
            projected = projection.project(-73.9857, 40.7484)
            self.assertTrue(all(math.isfinite(value) for value in projected))
            inverted = projection.invert(*projected)
            self.assertIsNotNone(inverted)
            assert inverted is not None
            self.assertAlmostEqual(inverted[0], -73.9857, places=6)
            self.assertAlmostEqual(inverted[1], 40.7484, places=6)
            self.assertGreater(projection.scale_value, 0.0)
            self.assertEqual(len(projection.translate_value), 2)
            self.assertEqual(len(projection.center_value), 2)
            self.assertEqual(len(projection.rotate_value), 3)
            self.assertTrue(
                all(
                    math.isfinite(value)
                    for value in projection.project_rotated(0.1, 0.2)
                )
            )

        original = native.Mercator()
        configured = (
            original.scale(250.0)
            .translate(400.0, 300.0)
            .center(5.0, 10.0)
            .rotate(15.0, -5.0, 2.0)
        )
        self.assertNotEqual(original.scale_value, configured.scale_value)
        self.assertEqual(configured.scale_value, 250.0)
        self.assertEqual(configured.translate_value, (400.0, 300.0))
        self.assertEqual(configured.center_value, (5.0, 10.0))
        self.assertEqual(configured.rotate_value, (15.0, -5.0, 2.0))
        self.assertIsNotNone(configured.clip_extent())
        self.assertIsNotNone(configured.longitude_unwrap_center())
        self.assertIs(d3rs.Mercator, native.Mercator)

        orthographic = native.Orthographic()
        self.assertIsNotNone(orthographic.clip_angle())
        self.assertTrue(orthographic.is_visible(0.0, 0.0))
        self.assertFalse(orthographic.is_visible(180.0, 0.0))
        conic = native.ConicEqualArea.with_parallels(20.0, 50.0).scale(200.0)
        self.assertTrue(all(math.isfinite(value) for value in conic.project(10.0, 30.0)))

        with self.assertRaisesRegex(ValueError, "scale"):
            native.Mercator().scale(math.nan).project(0.0, 0.0)
        with self.assertRaisesRegex(ValueError, "zero cone"):
            native.ConicEqualArea.with_parallels(-30.0, 30.0).project(0.0, 0.0)

    def test_installed_extension_runs_geojson_paths_and_measurements(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")

        from gpui_toolkit import d3rs

        geometries = (
            native.GeoJsonGeometry.point(7.0, 46.0),
            native.GeoJsonGeometry.multi_point(((7.0, 46.0), (8.0, 47.0))),
            native.GeoJsonGeometry.line_string(((7.0, 46.0), (8.0, 47.0))),
            native.GeoJsonGeometry.multi_line_string(
                (((7.0, 46.0), (8.0, 47.0)), ((9.0, 46.0), (10.0, 47.0)))
            ),
            native.GeoJsonGeometry.polygon(
                (((7.0, 46.0), (8.0, 46.0), (8.0, 47.0), (7.0, 46.0)),)
            ),
            native.GeoJsonGeometry.multi_polygon(
                ((((7.0, 46.0), (8.0, 46.0), (8.0, 47.0), (7.0, 46.0)),),)
            ),
        )
        original = native.GeoPath(native.Mercator())
        path = original.digits(5).point_radius(3.0)
        self.assertEqual(original.digits_value, 3)
        self.assertEqual(original.point_radius_value, 4.5)
        self.assertEqual(path.digits_value, 5)
        self.assertEqual(path.point_radius_value, 3.0)
        self.assertIs(path.projection().__class__, native.Mercator)
        self.assertIs(d3rs.GeoPath, native.GeoPath)

        rendered = tuple(path.render(geometry) for geometry in geometries)
        self.assertTrue(all(value.startswith("M") for value in rendered))
        self.assertEqual(path.render_cow(geometries[2]), rendered[2])
        buffer = io.StringIO()
        path.render_into(geometries[2], buffer)
        self.assertEqual(buffer.getvalue(), rendered[2])

        projected = path.project_coords(((7.0, 46.0), (8.0, 47.0)))
        self.assertEqual(len(projected), 2)
        self.assertTrue(all(math.isfinite(value) for point in projected for value in point))
        bounds = path.bounds(geometries[4])
        centroid = path.centroid(geometries[4])
        self.assertTrue(all(math.isfinite(value) for point in bounds for value in point))
        self.assertTrue(all(math.isfinite(value) for value in centroid))

        events = native.geo_stream_events(geometries[4])
        self.assertEqual(events[0].kind, native.GeoStreamEventKind.POLYGON_START)
        self.assertEqual(events[1].kind, native.GeoStreamEventKind.LINE_START)
        self.assertEqual(events[-2].kind, native.GeoStreamEventKind.LINE_END)
        self.assertEqual(events[-1].kind, native.GeoStreamEventKind.POLYGON_END)

        calls: list[tuple[object, ...]] = []

        class Recorder:
            def point(self, x: float, y: float, marker: int) -> None:
                calls.append(("point", x, y, marker))

            def line_start(self) -> None:
                calls.append(("line_start",))

            def line_end(self) -> None:
                calls.append(("line_end",))

            def polygon_start(self) -> None:
                calls.append(("polygon_start",))

            def polygon_end(self) -> None:
                calls.append(("polygon_end",))

            def sphere(self) -> None:
                calls.append(("sphere",))

        native.stream_geojson(geometries[2], Recorder())
        self.assertEqual(calls[0], ("line_start",))
        self.assertEqual(calls[-1], ("line_end",))
        self.assertEqual(len([call for call in calls if call[0] == "point"]), 2)

        topology = """{
            "objects":{"land":{"type":"Polygon","arcs":[[0]]}},
            "arcs":[[[0,0],[1,0],[0,1],[-1,-1]]],
            "transform":{"scale":[1,1],"translate":[7,46]}
        }"""
        land = native.parse_land_with_budget(topology)
        self.assertEqual(land.kind, native.GeoJsonKind.MULTI_POLYGON)
        self.assertTrue(native.GeoPath(native.Mercator()).render(land).startswith("M"))
        self.assertEqual(native.parse_land("not json"), None)
        self.assertIs(d3rs.parse_land_with_budget, native.parse_land_with_budget)
        with self.assertRaises(native.TopoJsonInvalidError):
            native.parse_land_with_budget("not json")
        with self.assertRaisesRegex(native.TopoJsonBudgetError, "input bytes"):
            native.parse_land_with_budget(
                topology, native.TopoJsonBudget(max_input_bytes=1)
            )
        with self.assertRaisesRegex(ValueError, "max_arcs"):
            native.TopoJsonBudget(max_arcs=-1)

        hidden = native.GeoPath(native.Orthographic()).render(
            native.GeoJsonGeometry.point(180.0, 0.0)
        )
        self.assertEqual(hidden, "")
        with self.assertRaisesRegex(ValueError, r"coordinates\[1\]"):
            native.GeoJsonGeometry.line_string(((0.0, 0.0), (math.nan, 1.0)))
        with self.assertRaisesRegex(ValueError, "digits"):
            path.digits(16)
        with self.assertRaisesRegex(ValueError, "point_radius"):
            path.point_radius(-1.0)

    def test_installed_extension_runs_fetch_auto_type_batches(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")

        from gpui_toolkit import d3rs

        values = tuple(
            native.auto_type(value)
            for value in ("", "true", "42", "3.5", "hello", "2026-09-02", "NaN")
        )
        self.assertEqual(
            tuple(value.kind for value in values),
            (
                native.AutoTypeKind.NULL,
                native.AutoTypeKind.BOOL,
                native.AutoTypeKind.INTEGER,
                native.AutoTypeKind.FLOAT,
                native.AutoTypeKind.STRING,
                native.AutoTypeKind.DATE,
                native.AutoTypeKind.FLOAT,
            ),
        )
        self.assertTrue(values[0].is_null())
        self.assertIs(values[1].as_bool(), True)
        self.assertEqual(values[2].as_f64(), 42.0)
        self.assertEqual(values[3].as_i64(), 3)
        self.assertEqual(values[4].as_str(), "hello")
        self.assertEqual(values[5].as_str(), "2026-09-02")
        self.assertEqual(values[6].as_i64(), 0)

        row = native.auto_type_row({"name": "Ada", "age": "37"})
        self.assertEqual(row["name"].kind, native.AutoTypeKind.STRING)
        self.assertEqual(row["age"].value, 37)
        rows = native.auto_type_rows(({"x": "1"}, {}, {"x": "2.5"}))
        self.assertEqual(rows[0]["x"].value, 1)
        self.assertEqual(rows[1], {})
        self.assertEqual(rows[2]["x"].value, 2.5)
        self.assertIs(d3rs.auto_type_rows, native.auto_type_rows)
        with self.assertRaisesRegex(TypeError, r"row\['x'\]"):
            native.auto_type_row({"x": 1})

    def test_installed_extension_runs_budgeted_dsv_parsing_and_formatting(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")

        from gpui_toolkit import d3rs

        text = 'name,value,note\n Ada ,10,"hello, world"\nBob,20,plain\n'
        rows = native.parse_csv(text)
        self.assertEqual(
            rows,
            [
                {"name": "Ada", "value": "10", "note": "hello, world"},
                {"name": "Bob", "value": "20", "note": "plain"},
            ],
        )
        self.assertIs(d3rs.parse_csv, native.parse_csv)
        self.assertEqual(native.try_parse_csv(text), rows)
        self.assertEqual(native.parse_tsv("x\ty\n1\t2"), [{"x": "1", "y": "2"}])

        original = native.DsvParser(",")
        strict = original.column_policy(native.ColumnPolicy.STRICT)
        self.assertEqual(original.parse("a,b\n1"), [{"a": "1", "b": ""}])
        with self.assertRaises(native.DsvParseError) as mismatch:
            strict.parse("a,b\n1")
        self.assertEqual(
            mismatch.exception.kind, native.DsvParseErrorKind.HEADER_COLUMN_MISMATCH
        )
        self.assertEqual(mismatch.exception.expected, 2)
        self.assertEqual(mismatch.exception.actual, 1)
        self.assertGreaterEqual(mismatch.exception.line, 2)

        with self.assertRaises(native.DsvParseError) as duplicate:
            strict.parse("a,a\n1,2")
        self.assertEqual(duplicate.exception.header_name, "a")
        self.assertEqual(
            duplicate.exception.kind, native.DsvParseErrorKind.DUPLICATE_HEADER
        )

        with self.assertRaises(native.DsvBudgetError) as limited:
            native.parse_csv_with_budget(text, native.DsvBudget(max_records=1))
        self.assertEqual(limited.exception.resource, native.DsvBudgetResource.RECORDS)
        self.assertEqual(limited.exception.limit, 1)
        self.assertGreater(limited.exception.actual, 1)

        cancellation = native.DsvCancellationToken()
        cancellation.cancel()
        self.assertTrue(cancellation.is_cancelled())
        with self.assertRaises(native.DsvCancelledError):
            native.parse_csv_with_budget_and_cancel(
                text, native.DsvBudget(), cancellation
            )
        cancellation.reset()
        self.assertFalse(cancellation.is_cancelled())

        raw = original.parse_rows('a,"b,b"\n1,2')
        self.assertEqual(raw, [["a", "b,b"], ["1", "2"]])
        self.assertEqual(native.DsvParser('"').parse_lossy("a,b\n1,2"), [])

        options = native.CsvOptions().trim_values(False)
        self.assertTrue(native.CsvOptions().trim_values_value)
        self.assertFalse(options.trim_values_value)
        self.assertEqual(
            native.parse_csv_with_options("x\n value ", options),
            [{"x": " value "}],
        )
        formatted = native.format_csv(rows, ("name", "value", "note"))
        self.assertIn('Ada,10,"hello, world"', formatted)
        self.assertEqual(
            native.parse_csv(formatted),
            rows,
        )
        self.assertIn("1\t2", native.format_tsv([{"x": "1", "y": "2"}], ("x", "y")))

    def test_installed_extension_runs_stateful_quadtree_surface(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")

        from gpui_toolkit import d3rs

        points = (
            {"id": "a", "x": 0.0, "y": 0.0},
            {"id": "b", "x": 2.0, "y": 2.0},
            {"id": "c", "x": 4.0, "y": 0.0},
        )
        tree = native.QuadTree.try_from_data(
            points, lambda value: value["x"], lambda value: value["y"]
        )
        self.assertIs(d3rs.QuadTree, native.QuadTree)
        self.assertEqual(tree.size(), 3)
        self.assertFalse(tree.is_empty())
        self.assertEqual(tree.find(2.1, 2.1)["id"], "b")
        self.assertIsNone(tree.find(20.0, 20.0, radius=1.0))
        self.assertEqual(
            {value["id"] for value in tree.find_all(2.0, 1.0, 3.0)},
            {"a", "b", "c"},
        )

        extent = tree.extent()
        self.assertIsNotNone(extent)
        self.assertGreaterEqual(extent.width(), 4.0)
        self.assertTrue(extent.contains(2.0, 2.0))
        self.assertTrue(native.Extent.new(0.0, 0.0, 1.0, 1.0).contains(1.0, 1.0))
        union = extent.union(native.Extent(-10.0, -10.0, -9.0, -9.0))
        self.assertEqual(union.x0, -10.0)
        aggregate = native.Aggregate(1.0, 0.0, 0.0).merge(
            native.Aggregate(3.0, 4.0, 0.0)
        )
        self.assertEqual(aggregate.mass, 4.0)
        self.assertEqual(aggregate.x, 3.0)
        self.assertEqual(native.Aggregate.new(1.0, 2.0, 3.0).y, 3.0)
        point = native.QuadPoint.new(1.0, 2.0, {"id": "point"})
        self.assertEqual((point.x, point.y, point.data["id"]), (1.0, 2.0, "point"))
        internal = native.QuadNode.new_internal()
        internal.set_aggregate(aggregate)
        self.assertEqual(internal.aggregate, aggregate)
        with self.assertRaisesRegex(ValueError, "leaf"):
            native.QuadNode(native.QuadNodeKind.LEAF, point, None).set_aggregate(
                aggregate
            )

        duplicate = {"id": "duplicate", "x": 2.0, "y": 2.0}
        self.assertIs(tree.add(2.0, 2.0, duplicate), tree)
        self.assertEqual(tree.size(), 4)
        leaf = next(
            node for _, node in tree.snapshots() if node.kind is native.QuadNodeKind.LEAF
            and node.point is not None and node.point.x == 2.0
        )
        self.assertIsNotNone(leaf.point.next)
        self.assertTrue(tree.remove(2.0, 2.0))
        self.assertEqual(tree.size(), 3)

        copied = tree.copy()
        copied.add(8.0, 8.0, {"id": "copy", "x": 8.0, "y": 8.0})
        self.assertEqual(tree.size(), 3)
        self.assertEqual(copied.size(), 4)

        pre_order: list[native.QuadNodeKind] = []
        tree.visit(
            lambda _x0, _y0, _x1, _y1, node: pre_order.append(node.kind) or True
        )
        post_order: list[native.QuadNodeKind] = []
        tree.visit_after(
            lambda _x0, _y0, _x1, _y1, node: post_order.append(node.kind)
        )
        self.assertEqual(pre_order[0], native.QuadNodeKind.INTERNAL)
        self.assertEqual(post_order[-1], native.QuadNodeKind.INTERNAL)

        pruned: list[native.QuadNodeKind] = []
        tree.visit(
            lambda _x0, _y0, _x1, _y1, node: pruned.append(node.kind) or False
        )
        self.assertEqual(len(pruned), 1)

        tree.compute_aggregates()
        masses: list[float] = []
        tree.visit_aggregate(
            lambda _x0, _y0, _x1, _y1, _node, value: (
                masses.append(0.0 if value is None else value.mass) or True
            )
        )
        self.assertEqual(masses[0], 3.0)

        before = tree.size()
        with self.assertRaises(native.QuadTreeError) as invalid:
            tree.try_add_all(
                ({"x": 10.0, "y": 10.0}, {"x": math.nan, "y": 0.0}),
                lambda value: value["x"],
                lambda value: value["y"],
            )
        self.assertEqual(invalid.exception.index, 1)
        self.assertEqual(tree.size(), before)
        self.assertEqual(
            tree.remove_all(lambda value, _x, _y: value["id"] in {"a", "c"}),
            2,
        )
        self.assertEqual(tree.size(), 1)

        lossy = native.QuadTree.from_data(
            ((0.0, 0.0), (math.nan, 1.0)), lambda value: value[0], lambda value: value[1]
        )
        self.assertEqual(lossy.size(), 1)

    def test_installed_extension_runs_immutable_tile_layout(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")

        from gpui_toolkit import d3rs

        original = native.TileLayout.new()
        layout = original.size(512.0, 512.0).scale(512.0).translate((256.0, 256.0))
        tile_set = layout.try_tiles()
        self.assertIs(d3rs.TileLayout, native.TileLayout)
        self.assertGreater(len(tile_set), 0)
        self.assertEqual(tile_set.len(), len(tile_set.tiles))
        self.assertFalse(tile_set.is_empty())
        self.assertTrue(all(tile.z == tile_set.zoom for tile in tile_set.tiles))
        bounds = tile_set.tile_bounds(tile_set.tiles[0])
        self.assertAlmostEqual(bounds[1][0] - bounds[0][0], tile_set.tile_screen_size)
        self.assertAlmostEqual(bounds[1][1] - bounds[0][1], tile_set.tile_screen_size)
        self.assertEqual(layout.tiles(), tile_set)
        self.assertEqual(
            native.tiles_for_viewport(512.0, 512.0, 512.0, (256.0, 256.0)),
            tile_set,
        )
        self.assertNotEqual(original.try_tiles(), tile_set)
        self.assertEqual(native.MAX_TILE_ZOOM, 30)
        self.assertEqual(native.MAX_VISIBLE_TILES, 1_000_000)

        unclamped = layout.translate((1024.0, 1024.0)).clamp(False, False).tiles()
        self.assertTrue(any(tile.x < 0 or tile.y < 0 for tile in unclamped.tiles))
        with self.assertRaises(native.TileError) as invalid_scale:
            layout.scale(0.0).try_tiles()
        self.assertEqual(
            invalid_scale.exception.kind, native.TileErrorKind.NON_POSITIVE_SCALE
        )
        with self.assertRaises(native.TileError) as invalid_extent:
            layout.extent(((10.0, 10.0), (0.0, 0.0))).try_tiles()
        self.assertEqual(invalid_extent.exception.kind, native.TileErrorKind.INVALID_EXTENT)

    def test_installed_extension_runs_immutable_hexbin_surface(self) -> None:
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")

        from gpui_toolkit import d3rs

        points = (
            {"id": "a", "x": 10.0, "y": 10.0},
            {"id": "b", "x": 11.0, "y": 10.5},
            {"id": "c", "x": 80.0, "y": 70.0},
        )
        original = native.Hexbin.with_accessors(
            lambda value: value["x"], lambda value: value["y"]
        )
        hexbin = original.radius(12.0).extent(0.0, 0.0, 100.0, 100.0)
        bins = hexbin.try_bin(points)
        self.assertIs(d3rs.Hexbin, native.Hexbin)
        self.assertEqual(sum(len(bin_) for bin_ in bins), 3)
        self.assertEqual(sorted(bin_.len() for bin_ in bins), [1, 2])
        self.assertTrue(all(not bin_.is_empty() for bin_ in bins))
        self.assertTrue(any(bin_.points[0] is points[0] for bin_ in bins))
        self.assertNotEqual(original.hexagon(), hexbin.hexagon())

        path = hexbin.hexagon()
        self.assertIn("m", path)
        self.assertTrue(path.endswith("z"))
        self.assertNotEqual(path, hexbin.hexagon_with_radius(6.0))
        centers = hexbin.centers()
        self.assertGreater(len(centers), 0)
        self.assertTrue(all(math.isfinite(value) for point in centers for value in point))

        invalid_data = ({"x": 1.0, "y": 1.0}, {"x": math.nan, "y": 2.0})
        permissive = hexbin.bin(invalid_data)
        self.assertEqual(sum(len(bin_) for bin_ in permissive), 1)
        with self.assertRaises(native.HexbinError) as invalid_point:
            hexbin.try_bin(invalid_data)
        self.assertEqual(
            invalid_point.exception.kind,
            native.HexbinErrorKind.NON_FINITE_POINT_COORDINATE,
        )
        self.assertEqual(invalid_point.exception.index, 1)
        with self.assertRaises(native.HexbinError) as invalid_radius:
            hexbin.radius(0.0).try_bin(points)
        self.assertEqual(
            invalid_radius.exception.kind, native.HexbinErrorKind.NON_POSITIVE_RADIUS
        )
        invalid_extent = hexbin.extent(10.0, 0.0, 0.0, 10.0)
        self.assertEqual(invalid_extent.centers(), ())
        with self.assertRaises(native.HexbinError) as reversed_extent:
            invalid_extent.try_centers()
        self.assertEqual(
            reversed_extent.exception.kind, native.HexbinErrorKind.REVERSED_EXTENT
        )
        self.assertEqual(reversed_extent.exception.axis, "x")
        invalid_hexagon = hexbin.radius(0.0)
        self.assertEqual(invalid_hexagon.hexagon(), "")
        with self.assertRaises(native.HexbinError):
            invalid_hexagon.try_hexagon()

    def test_installed_extension_runs_sankey_layout_and_reports_typed_errors(self):
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")
        from gpui_toolkit import d3rs

        links = (
            native.SankeyLinkInput("source", "left", 2.0),
            native.SankeyLinkInput("source", "right", 1.0),
            native.SankeyLinkInput("left", "sink", 2.0),
            native.SankeyLinkInput("right", "sink", 1.0),
        )
        original = native.SankeyLayout.new()
        layout = (
            original.width(720.0)
            .height(360.0)
            .margins(8.0, 12.0, 8.0, 12.0)
            .node_width(18.0)
            .node_padding(12.0)
            .iterations(8)
            .node_align(native.SankeyNodeAlign.JUSTIFY)
        )
        result = layout.try_compute(("source", "left", "right", "sink"), links)

        self.assertIs(d3rs.SankeyLayout, native.SankeyLayout)
        self.assertNotEqual(original, layout)
        self.assertEqual(tuple(node.id for node in result.nodes), ("source", "left", "right", "sink"))
        self.assertEqual(len(result.links), len(links))
        for node in result.nodes:
            self.assertTrue(
                all(
                    math.isfinite(value)
                    for value in (node.x0, node.x1, node.y0, node.y1, node.value)
                )
            )
            self.assertLessEqual(node.x0, node.x1)
            self.assertLessEqual(node.y0, node.y1)
        for link, input_link in zip(result.links, links):
            self.assertEqual(link.value, input_link.value)
            self.assertTrue(all(math.isfinite(value) for value in (link.y0, link.y1, link.width)))
            self.assertTrue(link.path.startswith("M"))
            self.assertIn("C", link.path)

        for align in native.SankeyNodeAlign:
            aligned = layout.node_align(align).compute(
                ("source", "left", "right", "sink"), links
            )
            self.assertEqual(len(aligned.nodes), 4)

        input_order = layout.link_sort_input_order().compute(
            ("source", "left", "right", "sink"), links
        )
        self.assertEqual(tuple(link.value for link in input_order.links), (2.0, 1.0, 2.0, 1.0))
        self.assertNotEqual(layout, layout.link_sort(native.SankeyLinkSort.INPUT_ORDER))
        unchecked = layout.compute(
            ("source", "sink"),
            (native.SankeyLinkInput("source", "missing", 1.0),),
        )
        self.assertEqual(tuple(node.id for node in unchecked.nodes), ("source", "sink"))
        self.assertEqual(unchecked.links, ())

        error_cases = (
            (
                ("source", "source"),
                (),
                native.SankeyLayoutErrorKind.DUPLICATE_NODE_NAME,
            ),
            (
                ("source", "sink"),
                (native.SankeyLinkInput("source", "missing", 1.0),),
                native.SankeyLayoutErrorKind.UNKNOWN_LINK_ENDPOINT,
            ),
            (
                ("source", "sink"),
                (native.SankeyLinkInput("source", "sink", -1.0),),
                native.SankeyLayoutErrorKind.NEGATIVE_LINK_VALUE,
            ),
            (
                ("source", "sink"),
                (native.SankeyLinkInput("source", "sink", math.inf),),
                native.SankeyLayoutErrorKind.NON_FINITE_LINK_VALUE,
            ),
        )
        for names, invalid_links, expected_kind in error_cases:
            with self.subTest(kind=expected_kind):
                with self.assertRaises(native.SankeyLayoutError) as raised:
                    layout.try_compute(names, invalid_links)
                self.assertEqual(raised.exception.kind, expected_kind)
        with self.assertRaises(native.SankeyLayoutError) as endpoint_error:
            layout.try_compute(
                ("source", "sink"),
                (native.SankeyLinkInput("source", "missing", 1.0),),
            )
        self.assertEqual(endpoint_error.exception.name, "missing")
        self.assertEqual(endpoint_error.exception.link_index, 0)

        with self.assertRaises(native.SankeyLayoutError) as invalid_width:
            layout.width(0.0).try_compute(("source", "sink"), links[:1])
        self.assertEqual(
            invalid_width.exception.kind,
            native.SankeyLayoutErrorKind.NON_POSITIVE_CONFIG_FIELD,
        )
        self.assertEqual(invalid_width.exception.field, "width")

        with self.assertRaises(native.SankeyLayoutError) as invalid_area:
            layout.extent(10.0, 0.0, 10.0, 100.0).try_compute(
                ("source", "sink"), links[:1]
            )
        self.assertEqual(
            invalid_area.exception.kind,
            native.SankeyLayoutErrorKind.INVALID_DRAWABLE_AREA,
        )
        self.assertEqual(invalid_area.exception.axis, "x")
        self.assertEqual(invalid_area.exception.available, -18.0)

    def test_installed_extension_runs_renderer_independent_legend_layout(self):
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")
        from gpui_toolkit import d3rs

        items = tuple(
            native.LegendItem.with_symbol(
                f"Series {index}",
                f"#{index + 1:02x}4488",
                symbol,
            ).data(f"series-{index}")
            for index, symbol in enumerate(native.LegendSymbol)
        )
        original = native.LegendConfig.new()
        config = (
            original.position(native.LegendPosition.BOTTOM_RIGHT)
            .orientation(native.LegendOrientation.HORIZONTAL)
            .title("Measurements")
            .items(items)
            .symbol_size(14.0)
            .item_spacing(9.0)
            .padding(10.0)
            .background(False)
            .background_color("#fefefe")
            .border_width(2.0)
            .border_color("#202020")
            .font_size(13.0)
            .max_width(260.0)
        )
        layout = config.layout_with_char_width(800.0, 7.0)

        self.assertIs(d3rs.LegendConfig, native.LegendConfig)
        self.assertNotEqual(original, config)
        color_item = native.LegendItem.color(
            "Native color", d3rs.D3Color.rgba(31, 119, 180, 128)
        )
        self.assertTrue(color_item.color_value.startswith("#"))
        self.assertEqual(color_item.label_value, "Native color")
        self.assertEqual(color_item.symbol_value, native.LegendSymbol.CIRCLE)
        self.assertIsNone(color_item.data_value)
        self.assertEqual(len(layout.items), len(items))
        self.assertEqual(tuple(item.symbol for item in layout.items), tuple(native.LegendSymbol))
        self.assertEqual(layout.title.text, "Measurements")
        self.assertLessEqual(layout.width, 260.0)
        self.assertGreater(layout.columns, 0)
        self.assertGreater(layout.rows, 0)
        self.assertFalse(layout.is_empty())
        self.assertEqual(
            native.LegendLayout.try_from_config_with_char_width(
                config, 800.0, 7.0
            ),
            layout,
        )
        self.assertEqual(
            native.LegendLayout.from_config(config, 800.0),
            config.layout(800.0),
        )
        self.assertEqual(
            native.LegendLayout.try_from_config(config, 800.0),
            config.try_layout(800.0),
        )
        point = native.LegendPoint.new(1, 2)
        self.assertEqual(point, native.LegendPoint(1.0, 2.0))
        self.assertEqual(
            native.LegendRect.new(1, 2, 3, 4),
            native.LegendRect(point, 3.0, 4.0),
        )
        for item in layout.items:
            for bounds in (item.item_bounds, item.symbol_bounds, item.label_bounds):
                self.assertTrue(
                    all(
                        math.isfinite(value)
                        for value in (
                            bounds.origin.x,
                            bounds.origin.y,
                            bounds.width,
                            bounds.height,
                        )
                    )
                )

        self.assertEqual(native.legend_layout(config, 800.0), config.layout(800.0))
        self.assertEqual(
            config.offset_from_corner(800.0, 600.0, 200.0, 100.0, 12.0),
            (588.0, 488.0),
        )
        estimated = config.estimate_dimensions(7.0)
        self.assertTrue(all(value > 0.0 for value in estimated))

        scaled_items = native.legend_from_scale(
            lambda value: "#ff0000" if value < 0.5 else "#0000ff",
            (0.0, 1.0),
            format=lambda value: f"{value:.1f}",
        )
        scaled = original.items(scaled_items).layout(400.0)
        self.assertEqual(tuple(item.symbol for item in scaled.items), (
            native.LegendSymbol.SQUARE,
            native.LegendSymbol.SQUARE,
        ))

        empty = original.layout(400.0)
        self.assertTrue(empty.is_empty())
        title_only = original.title("Title only").layout(400.0)
        self.assertIsNotNone(title_only.title)
        self.assertFalse(title_only.is_empty())

        with self.assertRaises(native.LegendLayoutError) as negative_width:
            config.try_layout(-1.0)
        self.assertEqual(
            negative_width.exception.kind,
            native.LegendLayoutErrorKind.NEGATIVE_SIZE,
        )
        self.assertEqual(negative_width.exception.field, "available_width")

        with self.assertRaises(native.LegendLayoutError) as invalid_config:
            config.padding(math.nan).try_layout(400.0)
        self.assertEqual(
            invalid_config.exception.kind,
            native.LegendLayoutErrorKind.NON_FINITE_CONFIG,
        )
        self.assertEqual(invalid_config.exception.field, "padding")

        with self.assertRaises(native.LegendLayoutError) as invalid_char_width:
            config.layout_with_char_width(400.0, 0.0)
        self.assertEqual(
            invalid_char_width.exception.kind,
            native.LegendLayoutErrorKind.NON_POSITIVE_AVERAGE_CHAR_WIDTH,
        )
        self.assertEqual(invalid_char_width.exception.value, 0.0)
        with self.assertRaisesRegex(RuntimeError, "owned by the GPUI host"):
            native.LegendConfig.from_design(object())
        with self.assertRaisesRegex(RuntimeError, "owned by the GPUI host"):
            original.with_design(object())
        with self.assertRaisesRegex(RuntimeError, "owned by the GPUI host"):
            native.render_legend(config, 800.0, "#000000", None)

    def test_installed_extension_runs_renderer_independent_axis_layout(self):
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")
        from gpui_toolkit import d3rs

        scale = native.AxisScale.linear().with_domain(0.0, 100.0).with_range(0.0, 500.0)
        original = native.AxisConfig.bottom()
        config = (
            original.with_ticks(5)
            .with_tick_values((0.0, 50.0, 100.0))
            .with_minor_tick_values((25.0, 75.0))
            .with_minor_tick_size(2.0)
            .with_tick_size(8.0)
            .with_tick_padding(6.0)
            .with_label_font_size(11.0)
            .with_formatter(lambda value: f"{value:.0f} Hz")
            .with_domain_line_width(2.0)
            .with_title("Frequency")
            .with_title_font_size(14.0)
            .with_title_padding(9.0)
            .with_label_angle(-45.0)
        )
        layout = native.axis_layout(scale, config, 80.0)
        self.assertIs(d3rs.AxisConfig, native.AxisConfig)
        self.assertIs(d3rs.AxisTheme, native.AxisTheme)
        self.assertNotEqual(original, config)
        self.assertEqual(layout.orientation, native.AxisOrientation.BOTTOM)
        self.assertEqual(tuple(tick.value for tick in layout.major_ticks), (0.0, 50.0, 100.0))
        self.assertEqual(tuple(tick.label for tick in layout.major_ticks), ("0 Hz", "50 Hz", "100 Hz"))
        self.assertEqual(tuple(tick.value for tick in layout.minor_ticks), (25.0, 75.0))
        self.assertTrue(all(tick.is_minor for tick in layout.minor_ticks))
        self.assertEqual(len(layout.ticks()), 5)
        self.assertEqual(layout.all_ticks(), layout.ticks())
        self.assertEqual(
            native.AxisLayout.from_scale(scale, config, 80.0), layout
        )
        self.assertEqual(
            native.AxisLayout.try_from_scale(scale, config, 80.0), layout
        )
        self.assertIsNotNone(layout.domain_line)
        self.assertEqual(layout.title.text, "Frequency")
        self.assertGreater(config.total_size(), original.total_size())

        for orientation_factory in (
            native.AxisConfig.top,
            native.AxisConfig.right,
            native.AxisConfig.bottom,
            native.AxisConfig.left,
        ):
            oriented = orientation_factory().with_ticks(3).layout(scale, 60.0)
            self.assertGreater(len(oriented.major_ticks), 0)

        scales = (
            native.AxisScale.linear().with_domain(-10.0, 10.0),
            native.AxisScale.log().with_domain(1.0, 1000.0).base(10.0),
            native.AxisScale.pow().with_domain(-10.0, 10.0).exponent(2.0),
            native.AxisScale.symlog().with_domain(-10.0, 10.0).constant(2.0),
        )
        for candidate in scales:
            candidate = candidate.with_range(0.0, 400.0)
            result = original.with_ticks(4).try_layout(candidate, 50.0)
            self.assertTrue(all(math.isfinite(tick.position) for tick in result.major_ticks))

        hidden = original.hide_domain_line().layout(scale, 40.0)
        self.assertIsNone(hidden.domain_line)
        self.assertTrue(native.AxisOrientation.TOP.is_horizontal())
        self.assertTrue(native.AxisOrientation.LEFT.is_vertical())
        point = native.AxisPoint.new(1, 2)
        self.assertEqual(point, native.AxisPoint(1.0, 2.0))
        self.assertEqual(native.AxisLine.new(point, point), native.AxisLine(point, point))
        theme = native.DefaultAxisTheme()
        self.assertEqual(theme.axis_line_color(), native.AxisRgba(0.5, 0.5, 0.5, 1.0))
        self.assertEqual(theme.axis_label_color(), native.AxisRgba(0.3, 0.3, 0.3, 1.0))
        self.assertIsNone(theme.background_color())
        with self.assertRaisesRegex(RuntimeError, "owned by the GPUI host"):
            native.AxisConfig.from_design(object())
        with self.assertRaisesRegex(RuntimeError, "owned by the GPUI host"):
            original.with_design(object())
        with self.assertRaisesRegex(RuntimeError, "owned by the GPUI host"):
            native.render_axis(scale, config, 80.0, theme)

        with self.assertRaises(native.AxisLayoutError) as invalid_size:
            config.try_layout(scale, -1.0)
        self.assertEqual(invalid_size.exception.kind, native.AxisLayoutErrorKind.NEGATIVE_CONFIG)
        self.assertEqual(invalid_size.exception.field, "size")

        with self.assertRaises(native.AxisLayoutError) as invalid_tick:
            config.with_tick_values((math.nan,)).try_layout(scale, 40.0)
        self.assertEqual(invalid_tick.exception.kind, native.AxisLayoutErrorKind.NON_FINITE_TICK)

        with self.assertRaisesRegex(ValueError, "positive"):
            original.layout(native.AxisScale.log().with_domain(-1.0, 10.0), 40.0)

    def test_installed_extension_runs_renderer_independent_grid_layout(self):
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")
        from gpui_toolkit import d3rs

        x_scale = native.AxisScale.log().with_domain(1.0, 1000.0).with_range(0.0, 600.0)
        y_scale = native.AxisScale.symlog().with_domain(-10.0, 10.0).with_range(400.0, 0.0)
        original = native.GridConfig.new()
        config = (
            native.GridConfig.with_lines()
            .with_vertical_values((1.0, 10.0, 100.0, 1000.0))
            .with_horizontal_values((-10.0, 0.0, 10.0))
            .with_line_width(2.0)
            .with_dot_radius(3.0)
            .with_line_opacity(0.5)
            .with_dot_opacity(0.75)
        )
        layout = native.grid_layout(x_scale, y_scale, config, 600.0, 400.0)

        self.assertIs(d3rs.GridConfig, native.GridConfig)
        self.assertNotEqual(original, config)
        self.assertEqual(len(layout.vertical_lines), 4)
        self.assertEqual(len(layout.horizontal_lines), 3)
        self.assertEqual(len(layout.dots), 12)
        self.assertFalse(layout.is_empty())
        self.assertEqual(
            native.GridLayout.from_scales(x_scale, y_scale, config, 600.0, 400.0),
            layout,
        )
        self.assertEqual(
            native.GridLayout.try_from_scales(
                x_scale, y_scale, config, 600.0, 400.0
            ),
            layout,
        )
        self.assertAlmostEqual(layout.vertical_lines[0].start.x, 0.0)
        self.assertAlmostEqual(layout.vertical_lines[-1].start.x, 600.0)
        self.assertAlmostEqual(layout.horizontal_lines[0].start.y, 400.0)
        self.assertAlmostEqual(layout.horizontal_lines[-1].start.y, 0.0)
        self.assertTrue(
            all(
                math.isfinite(value)
                for dot in layout.dots
                for value in (dot.x_value, dot.y_value, dot.center.x, dot.center.y)
            )
        )
        point = native.GridPoint.new(1, 2)
        self.assertEqual(point, native.GridPoint(1.0, 2.0))
        self.assertEqual(
            native.GridLine.new(3, point, point),
            native.GridLine(3.0, point, point),
        )
        self.assertEqual(
            native.GridDot.new(4, 5, point),
            native.GridDot(4.0, 5.0, point),
        )
        with self.assertRaisesRegex(RuntimeError, "owned by the GPUI host"):
            native.GridConfig.from_design(object())
        with self.assertRaisesRegex(RuntimeError, "owned by the GPUI host"):
            original.with_design(object())
        with self.assertRaisesRegex(RuntimeError, "owned by the GPUI host"):
            native.render_grid(
                x_scale,
                y_scale,
                config,
                600.0,
                400.0,
                native.DefaultAxisTheme(),
            )

        dots = native.GridConfig.dots_only().with_vertical_values((1.0,)).with_horizontal_values((0.0,)).layout(
            x_scale, y_scale, 600.0, 400.0
        )
        self.assertEqual(len(dots.dots), 1)
        self.assertFalse(dots.vertical_lines)
        self.assertFalse(dots.horizontal_lines)
        lines = native.GridConfig.lines_only().with_vertical_values((1.0,)).with_horizontal_values((0.0,)).layout(
            x_scale, y_scale, 600.0, 400.0
        )
        self.assertFalse(lines.dots)

        clamped = original.with_line_opacity(-2.0).with_dot_opacity(3.0)
        self.assertFalse(clamped.layout(x_scale, y_scale, 600.0, 400.0).is_empty())

        with self.assertRaises(native.GridLayoutError) as negative_size:
            config.try_layout(x_scale, y_scale, -1.0, 400.0)
        self.assertEqual(negative_size.exception.kind, native.GridLayoutErrorKind.NEGATIVE_SIZE)
        self.assertEqual(negative_size.exception.field, "width")

        with self.assertRaises(native.GridLayoutError) as degenerate:
            config.try_layout(x_scale.with_range(1.0, 1.0), y_scale, 600.0, 400.0)
        self.assertEqual(degenerate.exception.kind, native.GridLayoutErrorKind.DEGENERATE_RANGE)
        self.assertEqual(degenerate.exception.axis, "x")

        with self.assertRaises(native.GridLayoutError) as invalid_tick:
            config.with_horizontal_values((math.nan,)).try_layout(
                x_scale, y_scale, 600.0, 400.0
            )
        self.assertEqual(invalid_tick.exception.kind, native.GridLayoutErrorKind.NON_FINITE_TICK)
        self.assertEqual(invalid_tick.exception.axis, "y")

        with self.assertRaises(native.GridLayoutError) as invalid_config:
            config.with_dot_radius(math.inf).try_layout(x_scale, y_scale, 600.0, 400.0)
        self.assertEqual(invalid_config.exception.kind, native.GridLayoutErrorKind.NON_FINITE_CONFIG)
        self.assertEqual(invalid_config.exception.field, "dot_radius")

    def test_installed_extension_runs_native_transition_state_and_lifecycle(self):
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")
        from gpui_toolkit import d3rs

        lifecycle: list[str] = []
        original = native.Transition.new()
        defaults = native.TransitionConfig()
        self.assertEqual(defaults.duration, 250.0)
        self.assertEqual(defaults.delay, 0.0)
        self.assertEqual(defaults.ease, native.TransitionEase.LINEAR)
        self.assertIsNone(defaults.name)
        transition = (
            original.duration(100.0)
            .delay(20.0)
            .ease(native.TransitionEase.CUBIC_IN_OUT)
            .name("gain")
            .from_to(-12.0, 0.0)
            .on_start(lambda: lifecycle.append("start"))
            .on_end(lambda: lifecycle.append("end"))
            .on_interrupt(lambda: lifecycle.append("interrupt"))
        )
        self.assertIs(d3rs.Transition, native.Transition)
        self.assertNotEqual(original, transition)
        with transition.start() as handle:
            self.assertEqual(handle.state(), native.TransitionState.PENDING)
            self.assertEqual(handle.tick(10.0), -12.0)
            self.assertEqual(handle.state(), native.TransitionState.PENDING)
            self.assertEqual(handle.tick(10.0), -12.0)
            self.assertEqual(handle.state(), native.TransitionState.ACTIVE)
            middle = handle.tick(50.0)
            self.assertGreater(middle, -12.0)
            self.assertLess(middle, 0.0)
            self.assertEqual(handle.tick(50.0), 0.0)
            self.assertTrue(handle.is_complete())
            self.assertEqual(handle.state(), native.TransitionState.ENDED)
        self.assertEqual(lifecycle, ["start", "end"])

        for ease in native.TransitionEase:
            handle = native.Transition.new().duration(10.0).ease(ease).start()
            self.assertTrue(math.isfinite(handle.tick(5.0)))
            handle.close()

        interrupted: list[str] = []
        handle = (
            native.Transition.new()
            .duration(100.0)
            .on_interrupt(lambda: interrupted.append("interrupt"))
            .start()
        )
        handle.tick(1.0)
        handle.interrupt()
        self.assertEqual(handle.state(), native.TransitionState.INTERRUPTED)
        self.assertEqual(interrupted, ["interrupt"])
        handle.reset()
        self.assertEqual(handle.state(), native.TransitionState.PENDING)
        handle.close()

        manager = native.TransitionManager.new()
        manager.add("x", native.Transition.new().duration(20.0).from_to(0.0, 2.0))
        self.assertEqual(manager.get("x"), 0.0)
        first = manager.tick(10.0)
        self.assertEqual(first[0][0], "x")
        self.assertTrue(manager.is_animating())
        manager.tick(10.0)
        self.assertIsNone(manager.get("x"))
        manager.add("a", native.Transition.new().duration(100.0))
        manager.add("b", native.Transition.new().duration(100.0))
        manager.interrupt("a")
        self.assertIsNone(manager.get("a"))
        manager.interrupt_all()
        self.assertIsNone(manager.get("b"))

        with self.assertRaisesRegex(ValueError, "non-negative"):
            native.Transition.new().duration(-1.0).start()
        with self.assertRaisesRegex(ValueError, "dt"):
            native.Transition.new().start().tick(-1.0)

    def test_installed_extension_runs_shared_native_timer_resources(self):
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")
        from gpui_toolkit import d3rs

        before = native.now()
        native.timer_flush()
        native.set_now(before)
        self.assertGreaterEqual(native.now(), before)
        with self.assertRaisesRegex(RuntimeError, "owned by the GPUI host"):
            native.set_ui_dispatcher(lambda callback: callback())
        with self.assertRaisesRegex(RuntimeError, "owned by the GPUI host"):
            native.clear_ui_dispatcher()
        self.assertIs(d3rs.Timer, native.Timer)

        timer_elapsed: list[float] = []
        timer = native.timer(lambda elapsed: not timer_elapsed.append(elapsed) and False)
        timer_id = timer.id()
        self.assertGreaterEqual(timer_id, 0)
        self.assertGreaterEqual(timer.delay(), 0.0)
        self.assertTrue(math.isfinite(timer.start_time()))
        timer.join()
        self.assertEqual(len(timer_elapsed), 1)
        self.assertTrue(timer.is_stopped())

        restarted: list[float] = []
        timer.restart(lambda elapsed: not restarted.append(elapsed) and False, 1.0)
        timer.join()
        self.assertEqual(len(restarted), 1)
        self.assertNotEqual(timer.id(), timer_id)
        timer.close()

        interval_elapsed: list[float] = []
        interval = native.interval(
            lambda elapsed: not interval_elapsed.append(elapsed)
            and len(interval_elapsed) < 3,
            2.0,
        )
        interval.join()
        self.assertEqual(len(interval_elapsed), 3)
        self.assertEqual(interval.state(), native.TimerState.STOPPED)
        interval.close()

        timeout_elapsed: list[float] = []
        with native.timeout(lambda elapsed: timeout_elapsed.append(elapsed), 2.0) as timeout:
            self.assertTrue(timeout.try_join(500.0))
            self.assertTrue(timeout.is_stopped())
        self.assertEqual(len(timeout_elapsed), 1)

        stopped = native.Interval.new(lambda _elapsed: True, 100.0)
        stopped.stop()
        stopped.join()
        self.assertTrue(stopped.is_stopped())
        stopped.close()

        def fail(_elapsed: float) -> bool:
            raise ValueError("timer callback failed")

        failed = native.Timer.new(fail)
        with self.assertRaisesRegex(native.TimerCallbackError, "timer callback failed"):
            failed.join()
        failed.close()

        with self.assertRaisesRegex(ValueError, "intervals must be positive"):
            native.Interval.new(lambda _elapsed: False, 0.0)
        with self.assertRaisesRegex(ValueError, "non-negative"):
            native.Timeout.new(lambda _elapsed: None, -1.0)

    def test_installed_extension_runs_native_drag_state_machine(self):
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")
        from gpui_toolkit import d3rs

        extent = native.DragExtent.try_new(0.0, 0.0, 100.0, 50.0)
        config = native.DragConfig().with_click_distance(5.0).with_extent(extent)
        self.assertIs(config.validate(), config)
        point = native.DragPoint.try_new(8.0, 9.0)
        self.assertEqual(point.delta_from(native.DragPoint(3.0, 5.0)), native.DragDelta(5.0, 4.0))
        self.assertEqual(extent.clamp(native.DragPoint(120.0, -2.0)), native.DragPoint(100.0, 0.0))
        drag = native.DragState.with_config(config)
        self.assertIs(d3rs.DragState, native.DragState)
        self.assertEqual(drag.config(), config)

        start = drag.start(7, -10.0, 20.0)
        self.assertEqual(start.phase, native.DragPhase.START)
        self.assertEqual(start.current, native.DragPoint(0.0, 20.0))
        self.assertFalse(start.exceeds_click_distance)
        self.assertTrue(drag.is_active())
        self.assertEqual(drag.active_pointer_id(), 7)

        update = drag.drag(7, 3.0, 24.0)
        self.assertEqual(update.delta, native.DragDelta(3.0, 4.0))
        self.assertEqual(update.delta.length(), 5.0)
        self.assertEqual(update.delta.length_squared(), 25.0)
        self.assertTrue(update.exceeds_click_distance)
        self.assertEqual(drag.current_update().current, update.current)

        end = drag.end(7, 120.0, -5.0)
        self.assertEqual(end.phase, native.DragPhase.END)
        self.assertEqual(end.current, native.DragPoint(100.0, 0.0))
        self.assertFalse(drag.is_active())
        self.assertIsNone(drag.active_pointer_id())
        self.assertIsNone(drag.current_update())

        drag.start(9, 10.0, 10.0)
        with self.assertRaises(native.DragError) as already_active:
            drag.start(10, 0.0, 0.0)
        self.assertEqual(already_active.exception.kind, native.DragErrorKind.ALREADY_ACTIVE)
        self.assertEqual(already_active.exception.active, 9)
        with self.assertRaises(native.DragError) as mismatch:
            drag.drag(10, 1.0, 1.0)
        self.assertEqual(mismatch.exception.kind, native.DragErrorKind.POINTER_MISMATCH)
        self.assertEqual(mismatch.exception.active, 9)
        self.assertEqual(mismatch.exception.received, 10)
        cancelled = drag.cancel(9)
        self.assertEqual(cancelled.phase, native.DragPhase.CANCEL)

        with self.assertRaises(native.DragError) as inactive:
            drag.end(9, 0.0, 0.0)
        self.assertEqual(inactive.exception.kind, native.DragErrorKind.INACTIVE)
        with self.assertRaises(native.DragError) as coordinate:
            native.DragState.new().start(1, math.nan, 0.0)
        self.assertEqual(coordinate.exception.kind, native.DragErrorKind.NON_FINITE_COORDINATE)
        self.assertEqual(coordinate.exception.axis, "x")
        with self.assertRaises(native.DragError) as invalid_extent:
            native.DragExtent.try_new(10.0, 0.0, 0.0, 10.0)
        self.assertEqual(invalid_extent.exception.kind, native.DragErrorKind.INVALID_EXTENT)
        with self.assertRaises(native.DragError) as click_distance:
            native.DragConfig().with_click_distance(-1.0)
        self.assertEqual(click_distance.exception.kind, native.DragErrorKind.INVALID_CLICK_DISTANCE)

    def test_installed_extension_runs_native_brush_and_zoom_state(self):
        if not native.AVAILABLE:
            self.skipTest("requires an installed abi3 wheel")
        from gpui_toolkit import d3rs

        brush = native.BrushState.new()
        self.assertIs(d3rs.BrushState, native.BrushState)
        self.assertFalse(brush.is_active())
        self.assertIsNone(brush.end())
        brush.start(375.0, 150.0)
        brush.update(125.0, 50.0)
        current = brush.current_selection()
        self.assertEqual(current, native.BrushSelection(125.0, 50.0, 375.0, 150.0))
        self.assertEqual(current.width(), 250.0)
        self.assertEqual(current.height(), 100.0)
        self.assertFalse(current.is_trivial(5.0))

        x_scale = native.AxisScale.linear().with_domain(0.0, 100.0).with_range(0.0, 500.0)
        y_scale = native.AxisScale.linear().with_domain(-10.0, 10.0).with_range(200.0, 0.0)
        domain = current.to_domain(x_scale, y_scale)
        self.assertAlmostEqual(domain.x0, 25.0)
        self.assertAlmostEqual(domain.x1, 75.0)
        self.assertAlmostEqual(domain.y0, -5.0)
        self.assertAlmostEqual(domain.y1, 5.0)
        self.assertEqual(brush.end(), current)
        self.assertFalse(brush.is_active())
        brush.start(1.0, 2.0)
        brush.update(3.0, 4.0)
        brush.reset()
        self.assertFalse(brush.is_active())
        self.assertIsNone(brush.current_selection())
        self.assertEqual(
            native.DomainSelection.new(8.0, 9.0, 2.0, 1.0),
            native.DomainSelection(2.0, 1.0, 8.0, 9.0),
        )

        log_domain = native.BrushSelection.new(0.0, 0.0, 250.0, 100.0).to_domain(
            native.AxisScale.log().with_domain(20.0, 20000.0).with_range(0.0, 500.0),
            native.AxisScale.linear().with_domain(-40.0, 10.0).with_range(0.0, 200.0),
        )
        self.assertAlmostEqual(log_domain.x0, 20.0)
        self.assertAlmostEqual(log_domain.x1, 632.4555, delta=1.0)
        brush_config = native.BrushConfig()
        self.assertEqual(brush_config.fill_color, (100, 150, 200, 80))
        self.assertEqual(brush_config.stroke_color, (70, 130, 180))
        self.assertEqual(brush_config.stroke_width, 1.0)
        self.assertEqual(brush_config.min_size, 5.0)

        zoom = native.ZoomState.new(0.0, 100.0, -10.0, 10.0)
        self.assertIs(d3rs.ZoomState, native.ZoomState)
        zoom.zoom_to(25.0, 75.0, -5.0, 5.0)
        self.assertTrue(zoom.is_zoomed())
        self.assertEqual(zoom.x_domain(), (25.0, 75.0))
        self.assertEqual(zoom.zoom_level(), 1)
        zoom.set_viewport(30.0, 70.0, -4.0, 4.0)
        self.assertEqual(zoom.zoom_level(), 1)
        self.assertEqual(zoom.x_domain(), (30.0, 70.0))
        self.assertTrue(zoom.zoom_back())
        self.assertEqual(zoom.x_domain(), (0.0, 100.0))
        self.assertFalse(zoom.zoom_back())

        positive = native.ZoomState.new(20.0, 20000.0, 1.0, 100.0)
        log_zoom = positive.with_log_x(True)
        self.assertIsNot(log_zoom, positive)
        log_zoom.zoom_to(-10.0, 1000.0, 1.0, 50.0)
        self.assertEqual(log_zoom.x_domain(), (20.0, 1000.0))
        self.assertEqual(positive.x_domain(), (20.0, 20000.0))
        log_zoom.set_original(10.0, 10000.0, 1.0, 200.0)
        self.assertEqual(log_zoom.original_x_domain(), (10.0, 10000.0))
        self.assertEqual(log_zoom.original_y_domain(), (1.0, 200.0))
        self.assertFalse(log_zoom.is_zoomed())
        log_zoom.reset()
        log_y = positive.with_log_y(True)
        log_y.zoom_to(20.0, 1000.0, -10.0, 50.0)
        self.assertEqual(log_y.y_domain(), (1.0, 50.0))
        zoom_config = native.ZoomConfig()
        self.assertTrue(zoom_config.zoom_x)
        self.assertTrue(zoom_config.zoom_y)
        self.assertEqual(zoom_config.min_extent, 0.001)
        self.assertEqual(zoom_config.max_extent, 100.0)

        with self.assertRaisesRegex(ValueError, "strictly increasing"):
            native.ZoomState.new(1.0, 1.0, 0.0, 1.0)
        with self.assertRaisesRegex(ValueError, "positive"):
            native.ZoomState.new(-1.0, 10.0, 0.0, 1.0).with_log_x(True)
        with self.assertRaisesRegex(ValueError, "finite"):
            native.BrushSelection.new(math.nan, 0.0, 1.0, 1.0)

    def test_dispatcher_preserves_payload_identity_once_and_removal(self):
        from gpui_toolkit import d3rs

        dispatcher = native.dispatcher()
        payload = {"revision": 7}
        received: list[tuple[str, object]] = []
        persistent = dispatcher.on(
            "update", lambda event: received.append((event.type_, event.payload))
        )
        dispatcher.once(
            "update", lambda event: received.append(("once", event.payload))
        )
        dispatcher.on("close", lambda _event: None)

        self.assertIs(d3rs.Dispatcher, native.Dispatcher)
        self.assertTrue(dispatcher.has_listeners("update"))
        self.assertEqual(dispatcher.listener_count("update"), 2)
        self.assertEqual(dispatcher.total_listeners(), 3)
        self.assertEqual(dispatcher.event_types(), ("close", "update"))
        dispatcher.dispatch_typed("update", payload)
        dispatcher.dispatch("update", payload)
        self.assertEqual(len(received), 3)
        self.assertTrue(all(value is payload for _, value in received))
        self.assertEqual(dispatcher.listener_count("update"), 1)

        dispatcher.off(persistent)
        self.assertFalse(dispatcher.has_listeners("update"))
        dispatcher.off_all("close")
        self.assertEqual(dispatcher.total_listeners(), 0)
        dispatcher.on("event", lambda _event: None)
        dispatcher.clear()
        self.assertEqual(dispatcher.event_types(), ())

        event = native.Event.with_data("data", payload)
        self.assertEqual(event.type_, "data")
        self.assertIs(event.payload, payload)
        self.assertIs(event.data(dict), payload)
        self.assertIsNone(event.data(list))
        self.assertIs(event.data(), payload)
        with self.assertRaisesRegex(TypeError, "must be a type"):
            event.data("dict")
