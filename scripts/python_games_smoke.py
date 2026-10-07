#!/usr/bin/env python3
"""Exercise the live native games host through its opt-in development API."""

import argparse
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

from python_dev_api import request


def find_node(value, node_id):
    if isinstance(value, dict):
        if value.get("id") == node_id:
            return value
        for child in value.values():
            found = find_node(child, node_id)
            if found is not None:
                return found
    elif isinstance(value, list):
        for child in value:
            found = find_node(child, node_id)
            if found is not None:
                return found
    return None


def exercise(api):
    def status():
        state = request(api, {"command": "status"})
        if state["error"] or state["surface_errors"]:
            raise AssertionError(f"Native session failed: {state['error']}\n{state['surface_errors']}\n{state['stderr']}")
        return state

    def wait(predicate):
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            state = status()
            if predicate(state):
                return state
            time.sleep(0.05)
        details = {key: state[key] for key in ("section", "revision", "surfaces", "bounds", "error", "stderr")}
        raise AssertionError(f"Native host did not publish the expected state change: {details}")

    def select(section, node_id):
        request(api, {"command": "select", "section": section})
        return wait(lambda state: state["section"] == section and node_id in state["bounds"])

    def changed(command):
        before = status()["revision"]
        request(api, command)
        return wait(lambda state: state["revision"] > before)

    def window_click(state, node_id, x=None, y=None):
        bounds = state["bounds"][node_id]
        if x is None:
            x, y = bounds["width"] / 2, bounds["height"] / 2
        else:
            view = find_node(state["app"], node_id)["scene"]["view_box"]
            x = (x - view["x"]) / view["width"] * bounds["width"]
            y = (y - view["y"]) / view["height"] * bounds["height"]
        return changed({"command": "window_click", "x": bounds["x"] + x, "y": bounds["y"] + y})

    def cell_click(state, surface, row, column):
        grid = find_node(state["app"], surface)["scene"]["grid"]
        x = grid["x"] + column * (grid["cell_width"] + grid["gap"]) + grid["cell_width"] / 2
        y = grid["y"] + row * (grid["cell_height"] + grid["gap"]) + grid["cell_height"] / 2
        return window_click(state, surface, x, y)

    # Repeat clicks after each Python patch, rather than testing initialization alone.
    state = select("queens", "queens-board")
    for row, column in ((0, 0), (0, 0), (1, 2)):
        state = cell_click(state, "queens-board", row, column)
    print("Queens: three GPUI window clicks and scene updates passed")

    state = select("zip", "zip-board")
    scene = find_node(state["app"], "zip-board")["scene"]
    start = next(node for node in scene["nodes"] if node["id"] == "zip-checkpoint-1")["kind"]["center"]
    state = window_click(state, "zip-board", start["x"], start["y"])
    changed({"command": "key", "surface": "zip-board", "phase": "down", "key": "Backspace"})
    request(api, {"command": "key", "surface": "zip-board", "phase": "up", "key": "Backspace"})
    print("Zip: GPUI checkpoint click and native keyboard undo passed")

    state = select("sudoku", "sudoku-board")
    for row, column in ((0, 2), (1, 1), (2, 3)):
        state = cell_click(state, "sudoku-board", row, column)
    changed({"command": "key", "surface": "sudoku-board", "phase": "down", "key": "5"})
    request(api, {"command": "key", "surface": "sudoku-board", "phase": "up", "key": "5"})
    print("Sudoku: three GPUI cell clicks and native digit input passed")

    state = select("chess", "chess-board")
    for row, column in ((6, 4), (4, 4)):
        state = cell_click(state, "chess-board", row, column)
    print("Chess: GPUI piece select, e2-e4, and AI reply passed")

    state = select("othello", "othello-board")
    state = cell_click(state, "othello-board", 5, 3)
    print("Othello: GPUI d3 placement and AI reply passed")

    state = select("tetris", "tetris-btn-start")
    state = window_click(state, "tetris-btn-start")
    state = wait(lambda current: current["revision"] > state["revision"])
    changed({"command": "pointer", "surface": "tetris-controls", "phase": "down",
             "device": "touch", "contact_id": 10, "x": 43, "y": 41})
    changed({"command": "pointer", "surface": "tetris-controls", "phase": "down",
             "device": "touch", "contact_id": 11, "x": 295, "y": 41})
    for contact, x in ((10, 43), (11, 295)):
        request(api, {"command": "pointer", "surface": "tetris-controls", "phase": "cancel",
                     "device": "touch", "contact_id": contact, "x": x, "y": 41})
    print("Tetris: GPUI Start click, live ticks, and two native contacts passed")

    state = select("overview", "palette-light")
    window_click(state, "palette-light")
    state = status()
    assert state["revision"] >= 15, state["revision"]
    assert not state["stderr"], state["stderr"]
    for bad_command in ({"command": "select", "section": "missing"},
                        {"command": "click", "surface": "missing", "x": 0, "y": 0}):
        try:
            request(api, bad_command)
        except RuntimeError:
            pass
        else:
            raise AssertionError("Invalid API command unexpectedly succeeded")
    status()
    print(f"All games passed: {state['revision']} native patch transactions; no session errors or Python stderr")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("host", type=Path, help="Built gpui-python-host executable")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    game = root / "crates/gpui-python-runtime/python/examples/games_demo.py"
    with tempfile.TemporaryDirectory(prefix="gpui-games-smoke-") as directory:
        api = Path(directory) / "api"
        environment = dict(os.environ, GPUI_TOOLKIT_DEV_API=str(api),
                           GPUI_TOOLKIT_DATA_DIR=str(Path(directory) / "state"),
                           GPUI_PYTHON=sys.executable)
        with (Path(directory) / "host.log").open("w+") as log:
            process = subprocess.Popen([str(args.host.resolve()), str(game)],
                                       env=environment, stdout=log, stderr=log)
            try:
                exercise(api)
            except Exception:
                log.seek(0)
                print(log.read(), file=sys.stderr)
                raise
            finally:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()


if __name__ == "__main__":
    main()
