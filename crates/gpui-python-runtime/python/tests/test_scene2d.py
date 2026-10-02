"""Wire, event, and reducer tests for the Python Scene2D API."""
import contextlib
import asyncio
import io
import json
import sys
from threading import Event as ThreadEvent
import unittest
from unittest.mock import patch

from gpui_toolkit import (
    App, Event, GridSpec, PathCommand, Scene2D,
    Scene2DClip, Scene2DGroup, Scene2DInputConfig, Scene2DLinearGradient,
    Scene2DSemantic, Scene2DShadow, Scene2DTransform, Scene2DTransition,
    Scene2DPatch, SceneCircle, SceneLine, ScenePath, SceneRect,
    SceneRoundedRect, SceneText, SessionContext, Stroke, patch_op,
    replace_op, section,
)
from gpui_toolkit.events import (
    Scene2DActivateInput, Scene2DEvent, Scene2DKeyInput, Scene2DLifecycleInput,
    Scene2DPointerInput, Scene2DTransitionCompleteInput, Scene2DTick, specialize,
)
import gpui_toolkit.app as app_module


class Scene2DSchemaTests(unittest.TestCase):
    def test_scene_uses_versioned_host_schema_and_native_node_tags(self):
        scene = Scene2D(
            "board", 120, 80,
            [
                SceneRect("field", 0, 0, 120, 80, fill="#123", stroke=Stroke("#fff", 2)),
                SceneRoundedRect("cell", 8, 8, 24, 24, fill="#8ad", radius=5,
                                 hit_id="cell-0", semantic=Scene2DSemantic("grid_cell", "Cell A1")),
                SceneCircle("dot", 50, 50, 6, fill="#ff0"),
                SceneLine("edge", (0, 0), (10, 10), Stroke("#fff")),
                ScenePath("path", [PathCommand("move_to", ((1, 1),)),
                                    PathCommand("line_to", ((4, 5),))], fill="#fff"),
                SceneText("label", 12, 20, "A1", 12, "#fff", align="center"),
            ],
            GridSpec(2, 3, 8, 8, 24, 24, 4),
            Scene2DInputConfig(pointer=True, continuous=True, capture=True, keyboard=True),
            semantic=Scene2DSemantic(
                "grid", "Example board", description="2 rows by 3 columns"),
        )
        spec = scene.to_spec()
        self.assertEqual(spec["kind"], "scene2d")
        wire = spec["scene"]
        self.assertEqual(wire["version"], 1)
        self.assertEqual(wire["revision"], 1)
        self.assertEqual(wire["grid"]["columns"], 3)
        self.assertIs(wire["input"]["keyboard"], True)
        self.assertEqual([node["kind"]["type"] for node in wire["nodes"]],
                         ["rect", "rounded_rect", "circle", "line", "path", "text"])
        self.assertEqual(wire["nodes"][0]["kind"]["fill"]["type"], "solid")
        self.assertEqual(wire["nodes"][1]["semantic"]["role"], "grid_cell")
        self.assertEqual(wire["nodes"][4]["kind"]["commands"][0],
                         {"type": "move_to", "point": {"x": 1.0, "y": 1.0}})
        json.dumps(spec)

    def test_linear_gradient_brush_serializes_for_shapes_and_scene_background(self):
        gradient = Scene2DLinearGradient(35, "#102030", "#f0e0d0")
        scene = Scene2D("gradient", 20, 20, [SceneRect("field", 0, 0, 20, 20,
                                                       fill=gradient)],
                        background=gradient)
        wire = scene.scene_spec()
        expected = {"type": "linear_gradient", "angle_degrees": 35.0,
                    "from": {"r": 16 / 255, "g": 32 / 255, "b": 48 / 255, "a": 1.0},
                    "to": {"r": 240 / 255, "g": 224 / 255, "b": 208 / 255, "a": 1.0}}
        self.assertEqual(wire["background"], expected)
        self.assertEqual(wire["nodes"][0]["kind"]["fill"], expected)
        replacement = Scene2D("gradient", 20, 20, revision=2)
        self.assertIsNone(patch_op(scene, replacement)["background"])

    def test_grid_pick_respects_gaps_and_returns_stable_cell_id(self):
        grid = GridSpec(3, 4, 10, 12, 20, 16, 3)
        self.assertEqual(grid.pick(10, 12).id, "r0c0")
        picked = grid.pick(36, 31)
        self.assertEqual((picked.row, picked.column, picked.index, picked.id),
                         (1, 1, 5, "r1c1"))
        self.assertIsNone(grid.pick(31, 14))
        self.assertIsNone(grid.pick(200, 200))

    def test_scene_patch_is_a_consecutive_stable_id_diff(self):
        old = Scene2D("board", 40, 20, [
            SceneRect("keep", 0, 0, 10, 10, fill="#111"),
            SceneRect("remove", 10, 0, 10, 10, fill="#222"),
        ])
        current = Scene2D("board", 40, 20, [
            SceneRect("keep", 0, 0, 10, 10, fill="#333"),
            SceneRect("add", 20, 0, 10, 10, fill="#444"),
        ], revision=2)
        op = patch_op(old, current)
        self.assertEqual(op["op"], "scene2d_patch")
        self.assertEqual((op["base_revision"], op["revision"]), (1, 2))
        self.assertEqual({node["id"] for node in op["upsert"]}, {"keep", "add"})
        self.assertEqual(op["remove"], ["remove"])
        self.assertEqual(replace_op(current)["scene"]["revision"], 2)
        with self.assertRaises(ValueError):
            Scene2DPatch.between(old, Scene2D("board", 40, 20, revision=3))

    def test_scene_patch_can_explicitly_remove_optional_grid_metadata(self):
        old = Scene2D("board", 40, 20, grid=GridSpec(2, 2, 0, 0, 10, 10))
        current = Scene2D("board", 40, 20, grid=None, revision=2)
        op = patch_op(old, current)
        self.assertIn("grid", op)
        self.assertIsNone(op["grid"])

    def test_scene_patch_can_replace_or_clear_surface_semantics(self):
        old = Scene2D("board", 40, 20,
                      semantic=Scene2DSemantic("grid", "Old board"))
        changed = Scene2D("board", 40, 20, revision=2,
                          semantic=Scene2DSemantic("grid", "New board"))
        self.assertEqual(patch_op(old, changed)["semantic"],
                         {"role": "grid", "label": "New board"})
        cleared = Scene2D("board", 40, 20, revision=2)
        self.assertIsNone(patch_op(old, cleared)["semantic"])

    def test_invalid_scene_values_fail_before_serialization(self):
        with self.assertRaises(ValueError):
            Scene2D("", 20, 20)
        with self.assertRaises(ValueError):
            SceneText("text", 0, 0, "bad", 10, "#fff", align="middle").to_spec()
        with self.assertRaisesRegex(ValueError, "single line"):
            SceneText("text", 0, 0, "two\nlines", 10, "#fff").to_spec()
        with self.assertRaises(ValueError):
            Scene2DTransition(10, "springy").to_spec()
        with self.assertRaises(ValueError):
            Scene2DTransition(10.5).to_spec()
        with self.assertRaises(ValueError):
            Scene2DTransition(60_001).to_spec()
        with self.assertRaises(ValueError):
            Scene2DLinearGradient(361, "#000", "#fff").to_spec()
        with self.assertRaises(ValueError):
            PathCommand("close", ((0, 0),)).to_spec()

    def test_recursive_groups_clip_shadow_and_transition_completion_wire(self):
        transition = Scene2DTransition(100, completion_id="fall-4",
                                       animate_color=True, reveal_path=True)
        group = Scene2DGroup(
            "piece", [SceneRect(
                "tile", 2, 3, 10, 10, fill="#f00", hit_id="board-cell",
                clip=Scene2DClip(0, 0, 8, 8),
                shadow=Scene2DShadow(1, 2, 4, "#0008"),
                transition=transition,
            )],
            transform=None, clip=Scene2DClip(0, 0, 12, 12),
        )
        scene = Scene2D("board", 20, 20, [group])
        node = scene.scene_spec()["nodes"][0]
        self.assertEqual(node["kind"]["children"][0]["shadow"]["blur_radius"], 4.0)
        self.assertEqual(node["kind"]["children"][0]["transition"], {
            "duration_ms": 100, "easing": "ease_out_cubic", "completion_id": "fall-4",
            "animate_color": True, "reveal_path": True,
        })
        with self.assertRaisesRegex(ValueError, "unique"):
            Scene2D("board", 20, 20, [Scene2DGroup("same", [SceneRect("same", 0, 0, 1, 1, fill="#fff")])])
        with self.assertRaisesRegex(ValueError, "nesting"):
            nested = Scene2DGroup("g32", [SceneRect("leaf", 0, 0, 1, 1, fill="#fff")])
            for index in range(65):
                nested = Scene2DGroup(f"g{index}", [nested])
            Scene2D("board", 20, 20, [nested])

    def test_nested_transforms_reject_f32_overflow_in_scenes_and_patches(self):
        nested = SceneRect("leaf", 0, 0, 1, 1, fill="#fff")
        for index in range(14):
            nested = Scene2DGroup(
                f"scale-{index}", [nested],
                transform=Scene2DTransform(scale_x=1000.0, scale_y=1000.0),
            )

        with self.assertRaisesRegex(ValueError, "composed transform"):
            Scene2D("overflow", 20, 20, [nested])
        with self.assertRaisesRegex(ValueError, "composed transform"):
            Scene2DPatch("overflow", 1, 2, upsert=[nested.to_spec()])

    def test_python_transform_values_must_fit_f32_and_group_depth_matches_native(self):
        with self.assertRaisesRegex(ValueError, "f32"):
            Scene2DTransform(translate_x=1e300).to_spec()
        with self.assertRaisesRegex(ValueError, "f32"):
            Scene2DTransform(scale_x=1e300).to_spec()

        raw_node = SceneRect("raw", 0, 0, 1, 1, fill="#fff").to_spec()
        raw_node["transform"] = {
            "translate_x": 1e300, "translate_y": 0,
            "scale_x": 1, "scale_y": 1,
            "rotation_degrees": 0, "shear_x": 0,
        }
        with self.assertRaisesRegex(ValueError, "f32"):
            Scene2DPatch("raw", 1, 2, upsert=[raw_node])

        nested = SceneRect("leaf", 0, 0, 1, 1, fill="#fff")
        for index in range(64):
            nested = Scene2DGroup(f"valid-{index}", [nested])
        Scene2D("depth-64", 20, 20, [nested])
        with self.assertRaisesRegex(ValueError, "nesting"):
            Scene2D("depth-65", 20, 20, [Scene2DGroup("extra", [nested])])


class Scene2DEventTests(unittest.TestCase):
    def test_action_failure_reports_context_and_exception_with_stderr_traceback(self):
        class BrokenApp(App):
            def on_action(self, event, context):
                raise TypeError("duplicate surface_id")

        app = BrokenApp(sections=[section("home", "Home", {})])
        output, diagnostics = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(output), contextlib.redirect_stderr(diagnostics):
            app._handle_action(Event("failed-1", 1, "zip-board", "click", "zip_control"),
                               SessionContext())
        error = json.loads(output.getvalue())
        self.assertEqual((error["type"], error["code"], error["request_id"]),
                         ("error", "action_failed", "failed-1"))
        for detail in ("click", "zip-board", "zip_control", "TypeError", "duplicate surface_id"):
            self.assertIn(detail, error["message"])
        self.assertIn("Traceback (most recent call last)", diagnostics.getvalue())
        self.assertIn("on_action", diagnostics.getvalue())

    def test_session_dispatch_preserves_specialized_surface_events(self):
        class Recorder(App):
            def __init__(self):
                super().__init__(sections=[section("home", "Home", {})], serial_reducer=True)
                self.inputs = []

            def on_scene2d_event(self, event, context):
                self.inputs.append((event.id, event.surface_id, event.input))
                context.acknowledge(event)

        inputs = [
            {"type": "pointer", "phase": "down", "device": "mouse",
             "contact_id": 1, "timestamp_ns": 1, "position": {"x": 4, "y": 8},
             "buttons": ["left"], "modifiers": []},
            {"type": "key", "phase": "down", "key": "5", "repeat": False,
             "modifiers": [], "timestamp_ns": 2},
            {"type": "lifecycle", "reason": "focus_lost", "timestamp_ns": 3},
            {"type": "activate", "id": "cell-1", "timestamp_ns": 4},
            {"type": "transition_complete", "id": "cell-1",
             "completion_id": "move-1", "timestamp_ns": 5},
        ]
        messages = [{"type": "initialize", "session_version": 1, "capabilities": []}]
        for index, value in enumerate(inputs):
            messages.append({"type": "event", "id": f"input-{index}", "sequence": index,
                             "node_id": "board", "event": "scene2d.event",
                             "payload": {"event": value}})
        messages.extend([
            {"type": "event", "id": "cancel", "sequence": 5, "node_id": "board",
             "event": "scene2d.cancel_all", "payload": {"reason": "outbound_overflow"}},
            {"type": "shutdown"},
        ])
        app = Recorder()
        output = io.StringIO()
        with patch.object(sys, "stdin", io.StringIO("".join(
                json.dumps(message) + "\n" for message in messages))), \
                contextlib.redirect_stdout(output):
            app.serve()

        self.assertEqual([item[0] for item in app.inputs],
                         [f"input-{index}" for index in range(5)] + ["cancel"])
        self.assertTrue(all(item[1] == "board" for item in app.inputs))
        self.assertEqual([type(item[2]) for item in app.inputs], [
            Scene2DPointerInput, Scene2DKeyInput, Scene2DLifecycleInput,
            Scene2DActivateInput, Scene2DTransitionCompleteInput, Scene2DLifecycleInput,
        ])
        wire = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertFalse([message for message in wire if message["type"] == "error"])
        self.assertEqual([message["request_id"] for message in wire
                          if message["type"] == "acknowledged"],
                         [item[0] for item in app.inputs])

        # Queue-overflow recovery also creates an unspecialized cancellation.
        with contextlib.redirect_stdout(io.StringIO()):
            app._handle_action(Event("raw-cancel", 6, "board", "scene2d.cancel_all",
                                     payload={"reason": "reducer_overflow"}), SessionContext())
        self.assertEqual(app.inputs[-1][0], "raw-cancel")
        self.assertEqual(app.inputs[-1][2].reason, "reducer_overflow")

    def test_pointer_key_lifecycle_and_cancel_all_specialize(self):
        pointer = specialize({
            "id": "p1", "sequence": 1, "node_id": "zip-board",
            "event": "scene2d.event", "payload": {"type": "scene2d.event", "event": {
                "type": "pointer", "phase": "move", "device": "touch",
                "contact_id": 7, "timestamp_ns": 99, "position": {"x": 24, "y": 18},
                "buttons": ["left"], "modifiers": ["shift"],
                "hit_id": "zip-cell-0-1",
                "cell": {"row": 0, "column": 1, "index": 1, "id": "r0c1"},
            }},
        })
        self.assertIsInstance(pointer, Scene2DEvent)
        self.assertIsInstance(pointer.input, Scene2DPointerInput)
        self.assertEqual((pointer.surface_id, pointer.input.contact_id), ("zip-board", 7))
        self.assertEqual(pointer.input.cell.id, "r0c1")

        key = Scene2DEvent.from_event(Event.from_message({
            "id": "k1", "sequence": 2, "node_id": "sudoku-board",
            "event": "scene2d.event", "payload": {"event": {
                "type": "key", "phase": "down", "key": "5", "repeat": False,
                "modifiers": [], "timestamp_ns": 101,
            }},
        }))
        self.assertIsInstance(key.input, Scene2DKeyInput)
        self.assertEqual(key.input.key, "5")

        cancel = specialize({
            "id": "c1", "sequence": 3, "node_id": "tetris-board",
            "event": "scene2d.cancel_all", "payload": {"reason": "outbound_overflow"},
        })
        self.assertIsInstance(cancel.input, Scene2DLifecycleInput)
        self.assertEqual(cancel.input.reason, "outbound_overflow")

        completed = specialize({
            "id": "tr1", "sequence": 4, "node_id": "tetris-board",
            "event": "scene2d.event", "payload": {"event": {
                "type": "transition_complete", "id": "piece-4",
                "completion_id": "fall-4", "timestamp_ns": 130,
            }},
        })
        self.assertIsInstance(completed.input, Scene2DTransitionCompleteInput)
        self.assertEqual((completed.input.id, completed.input.completion_id),
                         ("piece-4", "fall-4"))

        activate = specialize({
            "id": "a1", "sequence": 5, "node_id": "sudoku-board",
            "event": "scene2d.event", "payload": {"event": {
                "type": "activate", "id": "cell-4-4", "hit_id": "r4c4",
                "cell": {"row": 4, "column": 4, "index": 40, "id": "r4c4"},
                "timestamp_ns": 150,
            }},
        })
        self.assertIsInstance(activate.input, Scene2DActivateInput)
        self.assertEqual(activate.input.cell.id, "r4c4")

    def test_tick_is_typed_elapsed_time(self):
        event = Event.from_message({
            "id": "t1", "sequence": 5, "node_id": "app",
            "event": "scene2d.tick", "payload": {
                "type": "scene2d.tick", "elapsed_ns": 25_000_000, "frame": 12,
            },
        })
        tick = Scene2DTick.from_event(event)
        self.assertEqual(tick.elapsed_ns, 25_000_000)
        self.assertEqual(tick.frame, 12)
        self.assertEqual(tick.elapsed_seconds, 0.025)

    def test_tick_acknowledges_after_async_callback_completes(self):
        class Recorder(App):
            serial_reducer = True

            def __init__(self):
                super().__init__(sections=[section("home", "Home", {})])
                self.calls = []
                self.completed = False

            async def on_tick(self, tick, context):
                self.calls.append((tick.frame, context._revision))
                await asyncio.sleep(0)
                self.completed = True

        app = Recorder()

        class TrackingContext(SessionContext):
            def __init__(self):
                super().__init__()
                self.completed_when_acknowledged = False

            def acknowledge(self, event):
                self.completed_when_acknowledged = app.completed
                super().acknowledge(event)

        context = TrackingContext()
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            app._handle_action(Event.from_message({
                "id": "tick-12", "sequence": 12, "node_id": "app",
                "event": "scene2d.tick", "payload": {"elapsed_ns": 5_000_000, "frame": 12},
            }), context)
        self.assertEqual(app.calls, [(12, 0)])
        self.assertTrue(context.completed_when_acknowledged)
        self.assertEqual(json.loads(output.getvalue())["type"], "acknowledged")


class SerialReducerTests(unittest.TestCase):
    def test_serial_admission_is_bounded_and_overflow_is_reported(self):
        first_started = ThreadEvent()
        release_first = ThreadEvent()

        class SlowReducer(App):
            def __init__(self):
                super().__init__(sections=[section("home", "Home", {})], serial_reducer=True)
                self.seen = []

            def on_action(self, event, context):
                self.seen.append(event.id)
                if event.id == "event-0":
                    first_started.set()
                    release_first.wait(timeout=3.0)

        app = SlowReducer()

        def messages(_context):
            yield {"type": "event", "id": "event-0", "sequence": 0,
                   "node_id": "button", "event": "click"}
            self.assertTrue(first_started.wait(timeout=1.0))
            for index in range(1, 241):
                yield {"type": "event", "id": f"event-{index}", "sequence": index,
                       "node_id": "button", "event": "click"}
            # This runs only after serve() handles event-240 and schedules
            # cancel_all because the ordinary admission limit is 240.
            release_first.set()
            yield {"type": "shutdown"}

        initialize = json.dumps({"type": "initialize", "session_version": 1,
                                 "capabilities": []}) + "\n"
        output = io.StringIO()
        with patch.object(sys, "stdin", io.StringIO(initialize)), \
                patch.object(app_module, "_messages", side_effect=lambda context: messages(context)), \
                contextlib.redirect_stdout(output):
            app.serve()

        self.assertEqual(app.seen, [f"event-{index}" for index in range(240)])
        wire = [json.loads(line) for line in output.getvalue().splitlines()]
        rejected = [message for message in wire if message.get("type") == "rejected"]
        self.assertEqual(len(rejected), 1)
        self.assertEqual(rejected[0]["request_id"], "event-240")
        self.assertEqual(rejected[0]["code"], "reducer_overflow")


if __name__ == "__main__":
    unittest.main()
