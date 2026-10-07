"""Tests for the classic-games demo (zip, queens, sudoku, tetris, chess, othello)."""
import contextlib
import importlib.util
import io
import json
import os
import random
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

EXAMPLES = Path(__file__).parents[1] / "examples"


def load_games_demo():
    path = EXAMPLES / "games_demo.py"
    module_spec = importlib.util.spec_from_file_location("games_demo", path)
    assert module_spec is not None
    assert module_spec.loader is not None
    module = importlib.util.module_from_spec(module_spec)
    # dataclasses with deferred annotations resolve via sys.modules.
    sys.modules["games_demo"] = module
    module_spec.loader.exec_module(module)
    return module


games = load_games_demo()

from gpui_toolkit import Event, GridCell, SessionContext  # noqa: E402
from gpui_toolkit.events import (  # noqa: E402
    Scene2DActivateInput, Scene2DEvent, Scene2DKeyInput, Scene2DLifecycleInput,
    Scene2DPointerInput,
    Scene2DTick,
)


def _walk(node, visit):
    if isinstance(node, dict):
        visit(node)
        for value in node.values():
            _walk(value, visit)
    elif isinstance(node, list):
        for value in node:
            _walk(value, visit)


def _collect_ids(spec):
    ids = []

    def visit(node):
        node_id = node.get("id")
        if isinstance(node_id, str):
            ids.append(node_id)

    _walk(spec, visit)
    return ids


def _flatten(node):
    values = []
    _walk(node, values.append)
    return values


def _button_labels(spec):
    labels = []

    def visit(node):
        if node.get("kind") == "button":
            labels.append(node.get("label"))

    _walk(spec, visit)
    return labels


def _run_action(app, action, node_id, payload=None):
    """Drive on_action with a capturing context; returns (messages, app)."""
    context = SessionContext()
    event = Event.from_message({
        "id": "evt-1",
        "sequence": 0,
        "node_id": node_id,
        "event": "click",
        "action": action,
        "payload": dict(payload or {}),
    })
    stream = io.StringIO()
    with contextlib.redirect_stdout(stream):
        app.on_action(event, context)
    messages = [json.loads(line) for line in stream.getvalue().splitlines() if line.strip()]
    return messages, app


class GamesDemoSpecTests(unittest.TestCase):
    def test_sections_serialize(self):
        spec = games.build_app().to_spec()
        self.assertEqual(
            [section["label"] for section in spec["sections"]],
            ["Overview", "Zip", "Queens", "Sudoku", "Tetris", "Chess", "Othello"],
        )
        json.dumps(spec)

    def test_controls_have_labels_and_boards_have_no_cell_buttons(self):
        spec = games.build_app().to_spec()
        labels = _button_labels(spec)
        self.assertGreater(len(labels), 20)
        for label in labels:
            self.assertTrue(isinstance(label, str) and label.strip(), repr(label))
        for section in spec["sections"]:
            found = []
            _walk(section["content"], lambda node: found.append(node)
                  if node.get("kind") == "scene2d" else None)
            for surface in found:
                scene = surface["scene"]
                self.assertGreater(len(scene["nodes"]), 0)
                self.assertTrue(all(node["kind"]["type"] != "button"
                                    for node in scene["nodes"]))

    def test_game_boards_precede_wrapped_hud_for_short_viewports(self):
        app = games.build_app()
        cases = (
            (app.zip_game, "zip-board", "zip-moves"),
            (app.queens_game, "queens-board", "queens-count"),
            (app.sudoku_game, "sudoku-board", "sudoku-filled"),
            (app.tetris_game, "tetris-board", "tetris-score"),
            (app.chess_game, "chess-board", "chess-to-move"),
            (app.othello_game, "othello-board", "othello-to-move"),
        )
        for game, surface_id, metric_id in cases:
            children = game.section_node().to_spec()["children"]
            board_index = next(
                index for index, child in enumerate(children)
                if any(node.get("kind") == "scene2d" and node.get("id") == surface_id
                       for node in _flatten(child))
            )
            metric_index = next(
                index for index, child in enumerate(children)
                if any(node.get("id") == metric_id for node in _flatten(child))
            )
            self.assertLess(board_index, metric_index, surface_id)

    def test_node_ids_are_unique(self):
        spec = games.build_app().to_spec()
        ids = _collect_ids(spec)
        self.assertGreater(len(ids), 200)
        self.assertEqual(len(ids), len(set(ids)))

    def test_overview_exposes_board_palette_without_implementation_metrics(self):
        app = games.build_app()
        overview = app.to_spec()["sections"][0]["content"]
        buttons = {}
        _walk(overview, lambda node: buttons.setdefault(node.get("id"), node)
              if node.get("id") in {"palette-light", "palette-dark"} else None)
        self.assertTrue(buttons["palette-dark"]["selected"])
        self.assertFalse(buttons["palette-light"]["selected"])
        self.assertNotIn("GPU handles in Python", json.dumps(overview))

    def test_patch_targets_exist_in_initial_spec(self):
        spec = games.build_app().to_spec()
        ids = set(_collect_ids(spec))
        for node_id in (
            "zip-board", "zip-moves", "zip-covered", "zip-next", "zip-badge",
            "zip-progress", "zip-status",
            "queens-board", "queens-count", "queens-conflicts", "queens-moves",
            "queens-badge", "queens-progress", "queens-status",
            "sudoku-board", "sudoku-filled", "sudoku-hints", "sudoku-moves",
            "sudoku-badge", "sudoku-progress", "sudoku-status",
            "tetris-board", "tetris-score", "tetris-lines", "tetris-level",
            "tetris-next", "tetris-badge", "tetris-progress", "tetris-status",
            "tetris-orb", "tetris-btn-start", "tetris-btn-auto",
            "tetris-preview", "tetris-controls", "tetris-control-hold-left",
            "tetris-control-hold-right", "tetris-control-hold-down",
            "tetris-control-hold-rotate",
            "chess-board", "chess-to-move", "chess-move", "chess-material",
            "chess-badge", "chess-progress", "chess-status", "chess-fen",
            "chess-moves", "chess-mode", "chess-difficulty", "chess-promotion",
            "chess-btn-new", "chess-btn-undo", "chess-btn-flip",
            "chess-btn-hint", "chess-btn-ai",
            "othello-board", "othello-to-move", "othello-black", "othello-white",
            "othello-badge", "othello-progress", "othello-status",
            "othello-moves", "othello-mode", "othello-difficulty",
            "othello-btn-new", "othello-btn-undo", "othello-btn-pass",
            "othello-btn-hint", "othello-btn-ai",
        ):
            self.assertIn(node_id, ids)

    def test_initial_scene_cache_matches_embedded_scenes(self):
        app = games.build_app()
        spec = app.to_spec()
        scenes = {}

        def visit(node):
            if node.get("kind") == "scene2d":
                scenes[node["id"]] = node["scene"]

        for section in spec["sections"]:
            _walk(section["content"], visit)
        self.assertEqual(set(app.scene_cache), set(scenes))
        for surface_id, scene in scenes.items():
            self.assertEqual(app.scene_cache[surface_id].scene_spec(), scene)


class Scene2DGameInputTests(unittest.TestCase):
    def test_accessibility_activation_routes_to_each_game_without_sticking_controls(self):
        app = games.build_app()
        context = SessionContext()
        output = io.StringIO()

        def activate(surface_id, target_id, cell=None):
            return Scene2DEvent(
                f"activate-{surface_id}-{target_id}", 1, surface_id, "scene2d.event",
                payload={}, surface_id=surface_id,
                input=Scene2DActivateInput(target_id, 1, target_id, cell),
            )

        zip_start = app.zip_game.level.start
        zip_next = (zip_start[0], zip_start[1] + 1)
        if zip_next[1] >= app.zip_game.level.cols:
            zip_next = (zip_start[0] + 1, zip_start[1])
        with contextlib.redirect_stdout(output):
            app.on_scene2d_event(activate(
                "zip-board", f"zip-cell-{zip_start[0]}-{zip_start[1]}",
                GridCell(zip_start[0], zip_start[1],
                         zip_start[0] * app.zip_game.level.cols + zip_start[1],
                         f"r{zip_start[0]}c{zip_start[1]}")), context)
            app.on_scene2d_event(activate(
                "zip-board", f"zip-cell-{zip_next[0]}-{zip_next[1]}",
                GridCell(zip_next[0], zip_next[1],
                         zip_next[0] * app.zip_game.level.cols + zip_next[1],
                         f"r{zip_next[0]}c{zip_next[1]}")), context)
            app.on_scene2d_event(activate(
                "queens-board", "queens-cell-0-0", GridCell(0, 0, 0, "r0c0")), context)
            app.on_scene2d_event(activate(
                "sudoku-board", "sudoku-cell-0-2", GridCell(0, 2, 2, "r0c2")), context)

            app.tetris_game.start(random.Random(7))
            start_column = app.tetris_game.piece_col
            app.on_scene2d_event(activate("tetris-controls", "hold-left"), context)

        self.assertEqual(app.zip_game.path[:2], [zip_start, zip_next])
        self.assertEqual(app.queens_game.marks[0][0], 1)
        self.assertEqual(app.sudoku_game.selected, (0, 2))
        self.assertEqual(app.tetris_game.piece_col, start_column - 1)
        self.assertFalse(app.held_tetris_controls)
        self.assertFalse(app.held_tetris_keys)

    def test_zip_drag_traverses_continuous_cells_and_publishes_scene_and_hud_together(self):
        app = games.build_app()
        context = SessionContext()
        start = app.zip_game.level.start
        next_cell = (start[0], start[1] + 1) if start[1] + 1 < app.zip_game.level.cols else (start[0] + 1, start[1])
        events = [
            Scene2DEvent("down-1", 1, "zip-board", "scene2d.event", payload={},
                         surface_id="zip-board", input=Scene2DPointerInput(
                             "down", "touch", 10, 1, 0, 0, ("left",), (),
                             f"zip-cell-{start[0]}-{start[1]}",
                             GridCell(start[0], start[1], start[0] * app.zip_game.level.cols + start[1],
                                      f"r{start[0]}c{start[1]}"))),
            Scene2DEvent("move-1", 2, "zip-board", "scene2d.event", payload={},
                         surface_id="zip-board", input=Scene2DPointerInput(
                             "move", "touch", 10, 2, 0, 0, ("left",), (),
                             f"zip-cell-{next_cell[0]}-{next_cell[1]}",
                             GridCell(next_cell[0], next_cell[1], next_cell[0] * app.zip_game.level.cols + next_cell[1],
                                      f"r{next_cell[0]}c{next_cell[1]}"))),
        ]
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            for event in events:
                app.on_scene2d_event(event, context)
        self.assertEqual(app.zip_game.path, [start, next_cell])
        self.assertEqual(app.scene_cache["zip-board"].revision, 3)
        messages = [json.loads(line) for line in output.getvalue().splitlines()]
        patches = [message for message in messages if message.get("type") == "patch"]
        self.assertEqual(len(patches), 2)
        self.assertTrue(all(any(op["op"] == "scene2d_patch" for op in message["ops"])
                            for message in patches))

    def test_focused_board_keys_change_selection_without_global_actions(self):
        app = games.build_app()
        context = SessionContext()
        app.queens_game.selected = (0, 0)
        event = Scene2DEvent(
            "key-1", 1, "queens-board", "scene2d.event", payload={},
            surface_id="queens-board",
            input=Scene2DKeyInput("down", "ArrowRight", False, (), 10),
        )
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            app.on_scene2d_event(event, context)
        self.assertEqual(app.queens_game.selected, (0, 1))
        self.assertEqual(app.scene_cache["queens-board"].revision, 2)
        self.assertIn("scene2d_patch", output.getvalue())

    def test_tetris_two_touch_holds_clear_on_lifecycle_cancel(self):
        app = games.build_app()
        app.tetris_game.start(random.Random(19))
        context = SessionContext()
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            for contact, hit_id in ((1, "hold-left"), (2, "hold-rotate")):
                app.on_scene2d_event(Scene2DEvent(
                    f"touch-{contact}", contact, "tetris-controls", "scene2d.event",
                    payload={}, surface_id="tetris-controls",
                    input=Scene2DPointerInput("down", "touch", contact, contact,
                                              0, 0, ("left",), (), hit_id, None),
                ), context)
        self.assertEqual(app.held_tetris_controls, {1: "left", 2: "rotate"})
        cancel = Scene2DEvent(
            "cancel-1", 3, "tetris-board", "scene2d.event", payload={},
            surface_id="tetris-board", input=Scene2DLifecycleInput("focus_lost", 3),
        )
        with contextlib.redirect_stdout(io.StringIO()):
            app.on_scene2d_event(cancel, context)
        self.assertFalse(app.held_tetris_controls)
        self.assertFalse(app.held_tetris_keys)

    def test_section_lifecycle_keeps_requested_simulation_and_resets_elapsed_time(self):
        app = games.build_app()
        app.tetris_game.running = True
        app.tetris_elapsed = 0.19
        app.sudoku_game.solving = True
        app.sudoku_elapsed = 0.17
        app.scheduled_interval = 0.05
        app.ticks_enabled = True
        app.held_tetris_controls[1] = "left"
        context = SessionContext()

        def lifecycle(event_id, reason):
            return Scene2DEvent(
                event_id, 1, "tetris-board", "scene2d.event", payload={},
                surface_id="tetris-board",
                input=Scene2DLifecycleInput(reason, 10),
            )

        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            app.on_scene2d_event(lifecycle("inactive", "section_changed"), context)
            app.on_scene2d_event(lifecycle("active", "resumed"), context)

        self.assertTrue(app.tetris_game.running)
        self.assertTrue(app.sudoku_game.solving)
        self.assertEqual(app.tetris_elapsed, 0.0)
        self.assertEqual(app.sudoku_elapsed, 0.0)
        self.assertFalse(app.held_tetris_controls)
        self.assertAlmostEqual(app.scheduled_interval,
                               min(0.08, app.tetris_game.tick_interval), places=6)
        self.assertNotIn('"interval":null', output.getvalue())

    def test_tick_surface_id_only_advances_that_games_simulation(self):
        app = games.build_app()
        app.tetris_game.running = True
        app.tetris_game.auto = True
        app.sudoku_game.solving = True
        app.scheduled_interval = 0.05
        context = SessionContext()
        with patch.object(app.tetris_game, "auto_step", return_value=[]) as auto_step:
            with contextlib.redirect_stdout(io.StringIO()):
                app.on_tick(Scene2DTick(100_000_000, 1, "sudoku-board"), context)
        auto_step.assert_not_called()
        self.assertEqual(app.tetris_elapsed, 0.0)

    def test_tetris_tick_caps_elapsed_catchup_and_skips_idle_scene_patches(self):
        app = games.build_app()
        game = app.tetris_game
        game.running = True
        game.auto = True
        app.ticks_enabled = True
        context = SessionContext()
        with patch.object(game, "auto_step", return_value=[]) as step:
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                app.on_tick(Scene2DTick(10_000_000_000, 1), context)
        self.assertLessEqual(step.call_count, 4)
        self.assertAlmostEqual(app.tetris_elapsed, 0.01, places=6)
        self.assertFalse(any(json.loads(line).get("type") == "patch"
                             for line in output.getvalue().splitlines()))
        self.assertAlmostEqual(app.scheduled_interval, 0.07, places=6)

    def test_scene_and_hud_ops_share_one_patch_message(self):
        app = games.build_app()
        event = Event.from_message({
            "id": "start-1", "sequence": 1, "node_id": "tetris-btn-start",
            "event": "click", "action": games.TETRIS_CONTROL_ACTION, "payload": {},
        })
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            app.on_action(event, SessionContext())
        messages = [json.loads(line) for line in output.getvalue().splitlines()]
        patch_messages = [message for message in messages if message.get("type") == "patch"]
        self.assertEqual(len(patch_messages), 1)
        ops = patch_messages[0]["ops"]
        self.assertTrue(any(op["op"] == "scene2d_patch" and op.get("id") == "tetris-board"
                            for op in ops))
        self.assertFalse(any(op["op"] == "scene2d_replace" for op in ops))
        self.assertTrue(any(op["op"] == "set" and op.get("id") == "tetris-score" for op in ops))

    def test_tetris_clear_flash_is_one_native_transition_then_one_removal_patch(self):
        app = games.build_app()
        game = app.tetris_game
        game.grid[-1] = ["L"] * games.TETRIS_COLS
        for column in range(3, 7):
            game.grid[-1][column] = None
        game.current = "I"
        game.cells = list(games._TETROMINOES["I"][0])
        game.piece_row, game.piece_col = 0, 3
        game.pieces, game.running = 1, True
        context = SessionContext()
        drop = Scene2DEvent(
            "drop-1", 1, "tetris-board", "scene2d.event", payload={},
            surface_id="tetris-board",
            input=Scene2DKeyInput("down", "Space", False, (), 1),
        )
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            app.on_scene2d_event(drop, context)
        self.assertEqual(game.lines, 1)
        self.assertEqual(game.last_clear_rows, [19])
        self.assertGreater(app.tetris_effect_remaining, 0.0)
        add_message = next(json.loads(line) for line in output.getvalue().splitlines()
                           if json.loads(line).get("type") == "patch")
        scene_patch = next(op for op in add_message["ops"] if op["op"] == "scene2d_patch")
        clear_node = next(node for node in scene_patch["patch"]["upsert"]
                          if node["id"] == "tetris-clear-19")
        self.assertEqual(clear_node["transition"]["duration_ms"], 320.0)

        output = io.StringIO()
        with contextlib.redirect_stdout(io.StringIO()):
            # Elapsed catch-up is intentionally capped at 250 ms per tick.
            app.on_tick(Scene2DTick(250_000_000, 2), context)
        self.assertGreater(app.tetris_effect_remaining, 0.0)
        with contextlib.redirect_stdout(output):
            app.on_tick(Scene2DTick(80_000_000, 3), context)
        remove_message = next(json.loads(line) for line in output.getvalue().splitlines()
                              if json.loads(line).get("type") == "patch")
        removed = next(op for op in remove_message["ops"] if op["op"] == "scene2d_patch")
        self.assertIn("tetris-clear-19", removed["patch"]["remove"])
        self.assertFalse(game.last_lock_cells)

    def test_main_launches_native_host_by_default(self):
        with patch.dict(os.environ, {k: v for k, v in os.environ.items()
                                     if not k.startswith("GPUI_TOOLKIT_")}, clear=True):
            with patch.object(games.App, "run") as run_mock:
                games.main()
        run_mock.assert_called_once_with()

    def test_main_dumps_spec_on_request(self):
        env = {k: v for k, v in os.environ.items() if not k.startswith("GPUI_TOOLKIT_")}
        env["GPUI_TOOLKIT_DUMP_IR"] = "1"
        with patch.dict(os.environ, env, clear=True):
            with patch.object(games.App, "run") as run_mock:
                stream = io.StringIO()
                with contextlib.redirect_stdout(stream):
                    games.main()
        run_mock.assert_not_called()
        json.loads(stream.getvalue())


class ZipGameTests(unittest.TestCase):
    def test_bundled_levels_are_solvable(self):
        for level in games.ZIP_LEVELS:
            with self.subTest(level=level.id):
                path = games.solve_zip(level)
                self.assertIsNotNone(path)
                assert path is not None
                self.assertEqual(len(path), level.cell_count)
                self.assertEqual(path[0], level.start)
                self.assertEqual(path[-1], level.finish)
                self.assertEqual(len(set(path)), level.cell_count)
                for first, second in zip(path, path[1:]):
                    self.assertEqual(abs(first[0] - second[0]) + abs(first[1] - second[1]), 1)
                order = [path.index(level.checkpoints[n]) for n in sorted(level.checkpoints)]
                self.assertEqual(order, sorted(order))

    def test_click_flow_wins_on_solver_path(self):
        game = games.ZipGame()
        level = game.level
        wrong = (level.rows - 1, level.cols - 1) if level.start != (level.rows - 1, level.cols - 1) else (0, 1)
        game.click(*wrong)
        self.assertEqual(game.path, [])
        path = games.solve_zip(level)
        assert path is not None
        for cell in path:
            game.click(*cell)
        self.assertTrue(game.won)
        self.assertEqual(len(game.path), level.cell_count)

    def test_out_of_order_checkpoint_is_rejected(self):
        game = games.ZipGame()
        level = game.level
        game.click(*level.start)
        numbers = sorted(level.checkpoints)
        if len(numbers) < 2:
            self.skipTest("level needs two checkpoints")
        second = numbers[1]
        ops = game.click(*level.checkpoints[second])
        # Either non-adjacent or out of order: the path must not advance.
        texts = [op.get("value", "") for op in ops if op.get("property") == "text"]
        self.assertTrue(any("Start" in str(t) or "order" in str(t) or "free cell" in str(t)
                            for t in texts) or len(game.path) == 1)

    def test_undo_and_reset(self):
        game = games.ZipGame()
        level = game.level
        path = games.solve_zip(level)
        assert path is not None
        game.click(*path[0])
        game.click(*path[1])
        self.assertEqual(len(game.path), 2)
        game.click(*path[1])
        self.assertEqual(game.path, [path[0]])
        game.control("undo")
        self.assertEqual(game.path, [])
        game.click(*path[0])
        ops = game.control("reset")
        self.assertEqual(game.path, [])
        self.assertTrue(any(op.get("op") == "replace" for op in ops))

    def test_select_level_rebuilds_board(self):
        game = games.ZipGame()
        second = games.ZIP_LEVELS[1]
        ops = game.select_level(second.id)
        self.assertEqual(game.level_index, 1)
        self.assertTrue(any(op.get("op") == "replace" for op in ops))
        with self.assertRaises(ValueError):
            game.select_level("nope")

    def test_next_advances_through_levels_and_wraps(self):
        game = games.ZipGame()
        path = games.solve_zip(game.level)
        assert path is not None
        for cell in path:
            game.click(*cell)
        self.assertTrue(game.won)
        ops = game.control("next")
        self.assertEqual(game.level_index, 1)
        self.assertFalse(game.won)
        self.assertEqual(game.path, [])
        self.assertTrue(any(op.get("op") == "replace" for op in ops))
        game.control("next")
        self.assertEqual(game.level_index, 2)
        game.control("next")
        self.assertEqual(game.level_index, 0)


class QueensGameTests(unittest.TestCase):
    def test_generated_puzzles_are_valid(self):
        for size in games.QUEENS_SIZES:
            for seed in (1, 2, 3):
                with self.subTest(size=size, seed=seed):
                    rng = random.Random(seed)
                    solution, regions = games.generate_queens_puzzle(size, rng)
                    self.assertEqual(len(solution), size)
                    rows = [row for row, _ in solution]
                    cols = [col for _, col in solution]
                    self.assertEqual(len(set(rows)), size)
                    self.assertEqual(len(set(cols)), size)
                    for row, col in solution:
                        for dr in (-1, 0, 1):
                            for dc in (-1, 0, 1):
                                if dr or dc:
                                    self.assertNotIn((row + dr, col + dc), solution)
                    # Regions cover the board and each holds one solution crown.
                    flat = [cell for row in regions for cell in row]
                    self.assertEqual(sorted(set(flat)), list(range(size)))
                    counts = [0] * size
                    for row, col in solution:
                        counts[regions[row][col]] += 1
                    self.assertEqual(counts, [1] * size)
                    self.assertEqual(
                        games.queens_conflicts(size, solution, regions)["total"], 0)
                    # Regions are connected.
                    for region in range(size):
                        cells = {(r, c) for r in range(size) for c in range(size)
                                 if regions[r][c] == region}
                        stack = [next(iter(cells))]
                        seen = {stack[0]}
                        while stack:
                            r, c = stack.pop()
                            for dr, dc in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                                nxt = (r + dr, c + dc)
                                if nxt in cells and nxt not in seen:
                                    seen.add(nxt)
                                    stack.append(nxt)
                        self.assertEqual(seen, cells)

    def test_click_cycles_and_wins(self):
        game = games.QueensGame()
        game.new_puzzle(6, random.Random(4))
        row, col = next(iter(game.solution))
        game.click(row, col)
        self.assertEqual(game.marks[row][col], games.QUEENS_QUEEN)
        game.click(row, col)
        self.assertEqual(game.marks[row][col], games.QUEENS_MARK)
        game.click(row, col)
        self.assertEqual(game.marks[row][col], games.QUEENS_EMPTY)
        for crowns in game.solution:
            game.click(*crowns)
        self.assertTrue(game.won)

    def test_conflicts_are_reported(self):
        game = games.QueensGame()
        game.new_puzzle(6, random.Random(4))
        game.click(0, 0)
        ops = game.click(0, 1)
        texts = [op.get("value", "") for op in ops if op.get("property") == "text"]
        self.assertTrue(any("Conflict" in str(text) for text in texts))

    def test_controls_and_size_select(self):
        game = games.QueensGame()
        game.new_puzzle(6, random.Random(4))
        game.click(0, 0)
        ops = game.control("clear", random.Random())
        self.assertTrue(any(op.get("op") == "replace" for op in ops))
        self.assertEqual(game.queens, set())
        ops = game.select_size("8", random.Random(9))
        self.assertEqual(game.size, 8)
        self.assertTrue(any(op.get("op") == "replace" for op in ops))
        with self.assertRaises(ValueError):
            game.select_size("9", random.Random())


class SudokuGameTests(unittest.TestCase):
    def test_masks_are_uniquely_solved_by_reference_solution(self):
        reference = games._sudoku_grid(games.SUDOKU_SOLUTION)
        self.assertTrue(games.is_valid_sudoku_solution(reference))
        givens_counts = {}
        for difficulty, mask in games.SUDOKU_MASKS.items():
            with self.subTest(difficulty=difficulty):
                grid = games._sudoku_grid(mask)
                givens_counts[difficulty] = sum(cell != 0 for row in grid for cell in row)
                self.assertEqual(games.solve_sudoku(grid), reference)
                self.assertEqual(games.count_sudoku_solutions(grid, 2), 1)
        self.assertGreater(givens_counts["easy"], givens_counts["medium"])
        self.assertGreater(givens_counts["medium"], givens_counts["hard"])

    def test_transform_preserves_solution_and_uniqueness(self):
        game = games.SudokuGame()
        game.new_puzzle("medium", random.Random(12))
        self.assertTrue(games.is_valid_sudoku_solution(game.solution))
        grid = [[game.solution[row][col] if (row, col) in game.givens else 0
                 for col in range(9)] for row in range(9)]
        self.assertEqual(games.count_sudoku_solutions(grid, 2), 1)
        # And a second sample at hard.
        game.new_puzzle("hard", random.Random(13))
        grid = [[game.solution[row][col] if (row, col) in game.givens else 0
                 for col in range(9)] for row in range(9)]
        self.assertEqual(games.count_sudoku_solutions(grid, 2), 1)

    def test_play_flow_detects_mistakes_and_wins(self):
        game = games.SudokuGame()
        game.new_puzzle("easy", random.Random(14))
        hole = next((row, col) for row in range(9) for col in range(9)
                    if (row, col) not in game.givens)
        game.click(*hole)
        self.assertEqual(game.selected, hole)
        wrong = game.solution[hole[0]][hole[1]] % 9 + 1
        game.enter_digit(wrong)
        self.assertEqual(game.mistakes(), [hole])
        game.enter_digit(game.solution[hole[0]][hole[1]])
        self.assertEqual(game.mistakes(), [])
        before = len(game.entries)
        game.hint()
        self.assertGreaterEqual(len(game.entries), before)
        # Solve every remaining cell through the public animation steps.
        for row, col, digit in game.solve_step_cells():
            game.apply_solve_step(row, col, digit)
        closing = game.finish_solving()
        self.assertTrue(game.won)
        self.assertTrue(any(op.get("property") == "label" and op.get("value") == "Solved!"
                            for op in closing))

    def test_erase_hint_check_and_new(self):
        game = games.SudokuGame()
        game.new_puzzle("easy", random.Random(15))
        hole = next((row, col) for row in range(9) for col in range(9)
                    if (row, col) not in game.givens)
        game.click(*hole)
        game.enter_digit(game.solution[hole[0]][hole[1]])
        game.erase()
        self.assertNotIn(hole, game.entries)
        ops = game.check()
        self.assertTrue(any("No mistakes" in str(op.get("value", "")) for op in ops))
        ops = game.control("new", random.Random(16))
        self.assertTrue(any(op.get("op") == "replace" for op in ops))
        ops = game.select_difficulty("hard", random.Random(17))
        self.assertEqual(game.difficulty, "hard")
        self.assertTrue(any(op.get("op") == "replace" for op in ops))
        with self.assertRaises(ValueError):
            game.select_difficulty("extreme", random.Random())


class TetrisGameTests(unittest.TestCase):
    def test_bag_spawn_and_bounds(self):
        game = games.TetrisGame()
        rng = random.Random(21)
        game.start(rng)
        self.assertTrue(game.running)
        self.assertIsNotNone(game.current)
        self.assertIsNotNone(game.next_shape)
        for _ in range(30):
            game.move(-1)
        self.assertGreaterEqual(game.piece_col, -min(c for _, c in game.cells))
        rows = game.render_rows()
        self.assertEqual(len(rows), 20)
        self.assertTrue(all(len(line) == 10 for line in rows))

    def test_rotation_kicks_and_hard_drop_lock(self):
        game = games.TetrisGame()
        rng = random.Random(22)
        game.start(rng)
        pieces = game.pieces
        game.rotate()
        game.hard_drop(rng)
        self.assertGreater(game.pieces, pieces)
        locked = sum(cell is not None for row in game.grid for cell in row)
        self.assertEqual(locked, 4)

    def test_line_clear_scores_and_levels(self):
        game = games.TetrisGame()
        game.grid[-1] = ["🟦"] * 9 + [None]
        game.grid[-2] = ["🟦"] * 9 + [None]
        cleared = game._clear_lines()
        self.assertEqual(cleared, 0)
        game.grid[-1][9] = "🟦"
        self.assertEqual(game._clear_lines(), 1)
        self.assertEqual(len(game.grid), 20)
        self.assertEqual(game.grid[0], [None] * 10)
        self.assertEqual(game.grid[-1], ["🟦"] * 9 + [None])
        game.lines = 9
        self.assertEqual(game.level, 1)
        game.lines = 10
        self.assertEqual(game.level, 2)

    def test_tick_locks_piece(self):
        game = games.TetrisGame()
        rng = random.Random(23)
        game.start(rng)
        pieces = game.pieces
        for _ in range(25):
            game.tick(rng)
        self.assertGreater(game.pieces, pieces)
        locked = sum(cell is not None for row in game.grid for cell in row)
        self.assertGreaterEqual(locked, 4)

    def test_blocked_spawn_tops_out(self):
        game = games.TetrisGame()
        rng = random.Random(23)
        full: list[list[str | None]] = [["🟥"] * 10 for _ in range(20)]
        gap: list[str | None] = [None] * 10
        full[19] = gap  # one incomplete row: no line clear rescues spawn
        game.grid = full
        game.current = None
        self.assertFalse(game._spawn(rng))
        self.assertTrue(game.over)
        self.assertFalse(game.running)

    def test_delta_patches_touch_only_changed_rows(self):
        game = games.TetrisGame()
        rng = random.Random(24)
        game.start(rng)
        game.last_rows = game.render_rows()
        ops = game.move(1)
        row_ops = [op for op in ops if op.get("id", "").startswith("tetris-row-")]
        # A horizontal step repaints at most the rows the piece occupies.
        self.assertLessEqual(len(row_ops), 4)
        self.assertGreater(len(row_ops), 0)

    def test_autopilot_clears_lines(self):
        game = games.TetrisGame()
        game.auto = True
        rng = random.Random(25)
        game.start(rng)
        for _ in range(600):
            if game.over:
                break
            game.auto_step(rng)
            if game.lines >= 4:
                break
        self.assertGreater(game.pieces, 5)
        self.assertGreaterEqual(game.lines, 1)

    def test_controls_toggle_start_and_auto(self):
        game = games.TetrisGame()
        rng = random.Random(26)
        ops = game.control("start", rng)
        self.assertTrue(game.running)
        self.assertTrue(any(op.get("id") == "tetris-btn-start" for op in ops))
        game.control("start", rng)
        self.assertFalse(game.running)
        game.control("auto", rng)
        self.assertTrue(game.auto)
        self.assertTrue(game.running)
        with self.assertRaises(ValueError):
            game.control("warp", rng)


class GamesAppDispatchTests(unittest.TestCase):
    def test_light_sudoku_fixed_clues_use_dark_text_on_light_cells(self):
        game = games.build_app().sudoku_game
        game.palette = "light"
        row, column = next(iter(game.givens))
        scene = game.board_node().scene_spec()
        node = next(node for node in scene["nodes"]
                    if node["id"] == f"sudoku-value-{row}-{column}")
        color = node["kind"]["color"]
        self.assertLess(max(color["r"], color["g"], color["b"]), 0.3)

    def test_zip_click_round_trip(self):
        app = games.build_app()
        start = app.zip_game.level.start
        messages, _ = _run_action(app, "zip_cell", f"zip-cell-{start[0]}-{start[1]}")
        kinds = [message.get("type") for message in messages]
        self.assertIn("acknowledged", kinds)
        self.assertIn("patch", kinds)
        self.assertEqual(app.zip_game.path, [start])

    def test_unknown_action_is_rejected(self):
        app = games.build_app()
        messages, _ = _run_action(app, "nope", "nope")
        self.assertTrue(any(message.get("type") == "rejected" for message in messages))

    def test_othello_dropdown_picks_apply_and_patch(self):
        app = games.build_app()
        messages, _ = _run_action(
            app, games.OTHELLO_MODE_ACTION, "othello-mode", {"value": "two"})
        kinds = [message.get("type") for message in messages]
        self.assertIn("acknowledged", kinds)
        self.assertIn("patch", kinds)
        self.assertEqual(app.othello_game.mode, "two")
        messages, _ = _run_action(
            app, games.OTHELLO_DIFFICULTY_ACTION, "othello-difficulty",
            {"value": "hard"})
        self.assertEqual(app.othello_game.difficulty, "hard")
        self.assertTrue(
            any(message.get("type") == "patch" for message in messages))
        messages, _ = _run_action(
            app, games.OTHELLO_MODE_ACTION, "othello-mode",
            {"value": "correspondence"})
        self.assertTrue(
            any(message.get("type") == "rejected" for message in messages))
        self.assertEqual(app.othello_game.mode, "two")

    def test_zip_level_dropdown_changes_board_size(self):
        app = games.build_app()
        messages, _ = _run_action(
            app, games.ZIP_LEVEL_ACTION, "zip-level", {"value": "zip-6"})
        self.assertIn("acknowledged",
                      [message.get("type") for message in messages])
        self.assertEqual(app.zip_game.level_index, 2)
        self.assertEqual((app.zip_game.level.rows, app.zip_game.level.cols),
                         (6, 6))

    def test_board_palette_updates_every_surface_with_one_revisioned_diff(self):
        app = games.build_app()
        other_app = games.build_app()
        previous_revision = app.scene_cache["zip-board"].revision
        messages, _ = _run_action(app, "game_palette", "palette-light")
        patch = next(message for message in messages if message.get("type") == "patch")
        scene_ops = [op for op in patch["ops"] if op.get("op") == "scene2d_patch"]
        self.assertEqual(len(scene_ops), len(app.scene_cache))
        self.assertTrue(all(op.get("op") != "scene2d_replace" for op in patch["ops"]))
        self.assertEqual(app.scene_cache["zip-board"].revision, previous_revision + 1)
        self.assertEqual(app.palette, "light")
        self.assertEqual(app.zip_game.palette, "light")
        self.assertEqual(other_app.palette, "dark")
        zip_patch = next(op for op in scene_ops if op["id"] == "zip-board")
        well = next(node for node in zip_patch["patch"]["upsert"] if node["id"] == "zip-well")
        color = well["kind"]["fill"]["color"]
        self.assertAlmostEqual(color["r"], 232 / 255)
        self.assertAlmostEqual(color["g"], 238 / 255)
        self.assertAlmostEqual(color["b"], 244 / 255)

    def test_scene_status_changes_use_compact_stable_id_diffs(self):
        app = games.build_app()
        previous_count = len(app.scene_cache["queens-board"].nodes)
        messages, _ = _run_action(app, "queens_cell", "queens-cell-0-0")
        patch = next(message for message in messages if message.get("type") == "patch")
        board = next(op for op in patch["ops"]
                     if op.get("op") == "scene2d_patch" and op.get("id") == "queens-board")
        self.assertNotIn("grid", board["patch"])
        self.assertLess(len(board["patch"]["upsert"]), previous_count)
        self.assertGreater(len(board["patch"]["upsert"]), 0)

    def test_malformed_cell_is_rejected(self):
        app = games.build_app()
        messages, _ = _run_action(app, "zip_cell", "bogus")
        self.assertTrue(any(message.get("type") == "rejected" for message in messages))

    def test_sudoku_pad_and_select_flow(self):
        app = games.build_app()
        hole = next((row, col) for row in range(9) for col in range(9)
                    if (row, col) not in app.sudoku_game.givens)
        _run_action(app, "sudoku_cell", f"sudoku-cell-{hole[0]}-{hole[1]}")
        digit = app.sudoku_game.solution[hole[0]][hole[1]]
        _run_action(app, "sudoku_pad", f"sudoku-pad-{digit}")
        self.assertEqual(app.sudoku_game.entries.get(hole), digit)
        messages, _ = _run_action(app, "sudoku_difficulty", "sudoku-difficulty",
                                  {"value": "hard"})
        self.assertIn("patch", [message.get("type") for message in messages])
        self.assertEqual(app.sudoku_game.difficulty, "hard")

    def test_chess_cell_round_trip_plays_human_move_and_ai_reply(self):
        app = games.build_app()
        _run_action(app, "chess_cell", "chess-cell-6-4")
        self.assertEqual(app.chess_game.selected, (6, 4))
        messages, _ = _run_action(app, "chess_cell", "chess-cell-4-4")
        self.assertIn("patch", [message.get("type") for message in messages])
        self.assertEqual(len(app.chess_game.match.history), 2)
        self.assertEqual(app.chess_game.match.position.side, "w")

    def test_othello_cell_round_trip_places_and_ai_replies(self):
        app = games.build_app()
        messages, _ = _run_action(app, "othello_cell", "othello-cell-5-3")
        self.assertIn("patch", [message.get("type") for message in messages])
        self.assertGreaterEqual(len(app.othello_game.match.history), 2)
        self.assertEqual(app.othello_game.match.position.side, "B")


def _play_uci(match, moves):
    for uci in moves:
        match.make_move(games.chess_move_from_uci(match, uci))
    return match


class ChessEngineTests(unittest.TestCase):
    def test_start_position_has_twenty_moves_and_round_trips_fen(self):
        match = games.ChessMatch()
        self.assertEqual(len(match.legal_moves()), 20)
        self.assertEqual(match.position.to_fen(), games.CHESS_START_FEN)
        rebuilt = games.ChessPosition.from_fen(games.CHESS_START_FEN)
        self.assertEqual(rebuilt.to_fen(), games.CHESS_START_FEN)
        with self.assertRaises(ValueError):
            games.ChessPosition.from_fen("not a fen")

    def test_perft_depth_two_counts_400_nodes(self):
        def perft(pos, depth):
            if depth == 0:
                return 1
            total = 0
            for move in pos.legal_moves():
                undo = pos.make_move(move)
                total += perft(pos, depth - 1)
                pos.undo_move(move, undo)
            return total

        self.assertEqual(perft(games.ChessPosition.starting(), 2), 400)

    def test_make_and_undo_restores_fen(self):
        pos = games.ChessPosition.starting()
        before = pos.to_fen()
        for move in pos.legal_moves()[:8]:
            undo = pos.make_move(move)
            pos.undo_move(move, undo)
            self.assertEqual(pos.to_fen(), before)

    def test_fools_mate_is_checkmate_with_san(self):
        match = _play_uci(games.ChessMatch(), ("f2f3", "e7e5", "g2g4", "d8h4"))
        self.assertEqual(match.status, "checkmate")
        self.assertEqual([san for _, san in match.history],
                         ["f3", "e5", "g4", "Qh4#"])
        self.assertEqual(match.result(), "0-1")
        with self.assertRaises(ValueError):
            match.make_move(games.ChessMove(0, 1))

    def test_castling_generation_and_rights_update(self):
        match = games.ChessMatch("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1")
        castles = {move.uci() for move in match.legal_moves() if move.castling}
        self.assertEqual(castles, {"e1g1", "e1c1"})
        match.make_move(games.chess_move_from_uci(match, "e1g1"))
        self.assertEqual(match.position.to_fen().split()[2], "kq")
        self.assertEqual(
            [san for _, san in match.history], ["O-O"])

    def test_en_passant_capture_removes_pawn(self):
        match = games.ChessMatch("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1")
        captures = [move for move in match.legal_moves() if move.en_passant]
        self.assertEqual([move.uci() for move in captures], ["e5d6"])
        san = match.make_move(captures[0])
        self.assertEqual(san, "exd6")
        self.assertIsNone(match.position.board[games._chess_from_algebraic("d5")])

    def test_promotion_generates_four_moves_with_san(self):
        match = games.ChessMatch("4k3/1P6/8/8/8/8/8/4K3 w - - 0 1")
        promos = sorted(move.uci() for move in match.legal_moves()
                        if move.promotion)
        self.assertEqual(promos, ["b7b8b", "b7b8n", "b7b8q", "b7b8r"])
        san = match.make_move(games.chess_move_from_uci(match, "b7b8q"))
        self.assertEqual(san, "b8=Q+")

    def test_stalemate_and_draw_detection(self):
        stalemate = games.ChessMatch("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1")
        self.assertEqual(stalemate.status, "stalemate")
        bare_kings = games.ChessMatch("4k3/8/8/8/8/8/8/4K3 w - - 0 1")
        self.assertEqual(bare_kings.status, "draw")
        self.assertEqual(bare_kings.draw_reason, "insufficient material")
        fifty = games.ChessMatch("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 100 1")
        self.assertEqual(fifty.status, "draw")
        self.assertEqual(fifty.draw_reason, "fifty-move rule")
        repetition = _play_uci(games.ChessMatch(),
                             ("g1f3", "g8f6", "f3g1", "f6g8",
                              "g1f3", "g8f6", "f3g1", "f6g8"))
        self.assertEqual(repetition.status, "draw")
        self.assertEqual(repetition.draw_reason, "threefold repetition")

    def test_illegal_moves_and_uci_errors(self):
        match = games.ChessMatch()
        with self.assertRaises(ValueError):
            match.make_move(games.ChessMove(12, 28))  # e2e5 jumps a pawn
        with self.assertRaises(ValueError):
            games.chess_move_from_uci(match, "e2e5")
        with self.assertRaises(ValueError):
            games.chess_move_from_uci(match, "bogus")
        with self.assertRaises(ValueError):
            games.chess_move_from_uci(match, "e7e8x")
        with self.assertRaises(ValueError):
            games.ChessMatch().undo_move()

    def test_best_move_returns_legal_moves(self):
        match = games.ChessMatch()
        for difficulty in ("harmless", "easy"):
            with self.subTest(difficulty=difficulty):
                move, _score, _nodes = games.chess_best_move(
                    match, difficulty, random.Random(3))
                self.assertIn(move, match.legal_moves())
        with self.assertRaises(ValueError):
            games.chess_best_move(match, "grandmaster", random.Random())


class ChessGameTests(unittest.TestCase):
    def test_click_selects_and_moves_with_ai_reply(self):
        game = games.ChessGame()
        game.click(6, 4)
        self.assertEqual(game.selected, (6, 4))
        game.click(4, 4, random.Random(9))
        self.assertIsNone(game.selected)
        self.assertEqual(len(game.match.history), 2)
        self.assertEqual(game.match.position.side, "w")

    def test_two_player_mode_has_no_ai_reply(self):
        game = games.ChessGame()
        game.select_mode("two")
        game.click(6, 4)
        game.click(4, 4)
        self.assertEqual(len(game.match.history), 1)
        self.assertEqual(game.match.position.side, "b")

    def test_illegal_clicks_are_rejected_without_state_change(self):
        game = games.ChessGame()
        game.click(3, 3)
        self.assertIsNone(game.selected)
        self.assertEqual(game.match.history, [])
        game.click(6, 4)
        game.click(6, 4)
        self.assertIsNone(game.selected)

    def test_promotion_picker_is_respected(self):
        game = games.ChessGame()
        game.match = games.ChessMatch("4k3/1P6/8/8/8/8/8/4K3 w - - 0 1")
        game.select_mode("two")
        game.select_promotion("N")
        game.click(1, 1)
        game.click(0, 1)
        self.assertEqual(game.match.history[-1][1], "b8=N")

    def test_checkmate_badge(self):
        game = games.ChessGame()
        game.select_mode("two")
        _play_uci(game.match, ("f2f3", "e7e5", "g2g4", "d8h4"))
        ops = game.status_ops()
        badge = next(op for op in ops if op.get("id") == "chess-badge"
                     and op.get("property") == "label")
        self.assertIn("Checkmate", badge["value"])

    def test_controls_and_selects(self):
        game = games.ChessGame()
        game.select_mode("two")
        game.click(6, 4)
        game.click(4, 4)
        ops = game.control("undo", random.Random())
        self.assertEqual(game.match.history, [])
        self.assertTrue(any("Undid" in str(op.get("value", "")) for op in ops))
        ops = game.control("flip", random.Random())
        self.assertTrue(game.flipped)
        self.assertTrue(any(op.get("op") == "replace" for op in ops))
        ops = game.control("hint", random.Random(2))
        self.assertIsNotNone(game.hint_move)
        self.assertTrue(any("Hint" in str(op.get("value", "")) for op in ops))
        game.control("ai", random.Random(2))
        self.assertEqual(len(game.match.history), 1)
        game.control("new", random.Random())
        self.assertEqual(game.match.history, [])
        with self.assertRaises(ValueError):
            game.control("warp", random.Random())
        with self.assertRaises(ValueError):
            game.select_mode("correspondence")
        with self.assertRaises(ValueError):
            game.select_difficulty("grandmaster")
        with self.assertRaises(ValueError):
            game.select_promotion("K")


def _othello_must_pass_match():
    """White to move with no placement while Black can play a1."""
    board = ["W"] * 64
    board[0] = None
    board[63] = "B"
    match = games.OthelloMatch()
    match.position = games.OthelloPosition(board, "W")
    match._refresh_status()
    return match


class OthelloEngineTests(unittest.TestCase):
    def test_start_position_has_four_placements(self):
        match = games.OthelloMatch()
        names = sorted(games._othello_name(sq)
                       for sq in match.position.placements())
        self.assertEqual(names, ["c4", "d3", "e6", "f5"])
        self.assertEqual(match.counts(), (2, 2))
        self.assertEqual(match.position.side, "B")

    def test_d3_flips_one_disc(self):
        match = games.OthelloMatch()
        flips = match.place(19)
        self.assertEqual(flips, 1)
        self.assertEqual(match.counts(), (4, 1))
        self.assertEqual(match.position.side, "W")

    def test_illegal_placement_and_early_pass_raise(self):
        match = games.OthelloMatch()
        with self.assertRaises(ValueError):
            match.place(0)  # a1 brackets nothing
        with self.assertRaises(ValueError):
            match.do_pass()

    def test_must_pass_position(self):
        match = _othello_must_pass_match()
        self.assertEqual(match.status, "active")
        self.assertEqual(match.position.placements(), [])
        self.assertEqual(match.position.placements_for("B"), [0])
        side = match.do_pass()
        self.assertEqual(side, "W")
        self.assertEqual(match.position.side, "B")
        match.undo_move()
        self.assertEqual(match.position.side, "W")

    def test_full_selfplay_game_finishes(self):
        match = games.OthelloMatch()
        rng = random.Random(5)
        plies = 0
        while not match.is_game_over() and plies < 70:
            move, _score, _nodes = games.othello_best_move(match, "easy", rng)
            if move is None:
                match.do_pass()
            else:
                match.place(move)
            plies += 1
        self.assertTrue(match.is_game_over())
        black, white = match.counts()
        if match.winner == "B":
            self.assertGreater(black, white)
        elif match.winner == "W":
            self.assertGreater(white, black)
        else:
            self.assertEqual(black, white)

    def test_best_move_is_legal_and_start_evaluates_symmetric(self):
        match = games.OthelloMatch()
        move, _score, _nodes = games.othello_best_move(
            match, "easy", random.Random(1))
        self.assertIn(move, match.position.placements())
        self.assertEqual(games.othello_evaluate(games.OthelloPosition()), 0)
        with self.assertRaises(ValueError):
            games.othello_best_move(match, "grandmaster", random.Random())


class OthelloGameTests(unittest.TestCase):
    def test_click_places_and_ai_replies(self):
        game = games.OthelloGame()
        game.click(5, 3, random.Random(7))
        self.assertGreaterEqual(len(game.match.history), 2)
        self.assertEqual(game.match.position.side, "B")

    def test_illegal_clicks_are_rejected_without_state_change(self):
        game = games.OthelloGame()
        game.click(0, 0)
        self.assertEqual(game.match.history, [])
        game.click(3, 3)  # occupied center
        self.assertEqual(game.match.history, [])

    def test_pass_control_flow(self):
        game = games.OthelloGame()
        ops = game.control("pass", random.Random())
        self.assertTrue(any("not allowed" in str(op.get("value", ""))
                            for op in ops))
        game.match = _othello_must_pass_match()
        game.select_mode("two")
        game.control("pass", random.Random())
        self.assertEqual(game.match.position.side, "B")

    def test_controls_and_selects(self):
        game = games.OthelloGame()
        game.select_mode("two")
        game.click(5, 3)
        self.assertEqual(len(game.match.history), 1)
        ops = game.control("undo", random.Random())
        self.assertEqual(game.match.history, [])
        self.assertTrue(any("Undid" in str(op.get("value", "")) for op in ops))
        game.click(5, 3)
        ops = game.control("hint", random.Random(4))
        self.assertIsNotNone(game.hint_move)
        self.assertTrue(any("Hint" in str(op.get("value", "")) for op in ops))
        game.control("ai", random.Random(4))
        self.assertEqual(len(game.match.history), 2)
        game.control("new", random.Random())
        self.assertEqual(game.match.history, [])
        with self.assertRaises(ValueError):
            game.control("warp", random.Random())
        with self.assertRaises(ValueError):
            game.select_mode("correspondence")
        with self.assertRaises(ValueError):
            game.select_difficulty("grandmaster")


if __name__ == "__main__":
    unittest.main()
