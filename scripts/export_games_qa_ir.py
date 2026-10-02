#!/usr/bin/env python3
"""Export deterministic, populated game scenes for native framebuffer QA."""

from __future__ import annotations

import argparse
from dataclasses import replace
import importlib.util
import json
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
PYTHON_ROOT = ROOT / "crates/gpui-python-runtime/python"


def export_games(output: Path) -> None:
    sys.path.insert(0, str(PYTHON_ROOT))
    source = PYTHON_ROOT / "examples/games_demo.py"
    spec = importlib.util.spec_from_file_location("games_qa_demo", source)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    output.mkdir(parents=True, exist_ok=True)

    for palette in ("light", "dark"):
        app = module.build_app()
        app.rng.seed(7)
        app.palette = palette
        games = (app.zip_game, app.queens_game, app.sudoku_game, app.tetris_game)
        for game in games:
            game.palette = palette
        # Include the path, crown, selection/candidates, locked blocks, ghost,
        # and next piece in addition to the static board geometry.
        for cell in module._snake_path(app.zip_game.level.rows, app.zip_game.level.cols)[:4]:
            app.zip_game.drag_to(*cell)
        app.queens_game.click(*min(app.queens_game.solution))
        app.sudoku_game.selected = next(
            (row, column)
            for row in range(9)
            for column in range(9)
            if (row, column) not in app.sudoku_game.givens
        )
        app.tetris_game.start(app.rng)
        app.tetris_game.hard_drop(app.rng)
        app.tetris_game.running = False
        app.miniapp = replace(app.miniapp, initial_theme=palette)
        app.sections = [
            module.section(name, name.title(), game.section_node())
            for name, game in zip(("zip", "queens", "sudoku", "tetris"), games)
        ]
        document = app.to_spec()
        nodes: dict[str, dict] = {}
        pending = [document]
        while pending:
            value = pending.pop()
            if isinstance(value, dict):
                if isinstance(value.get("kind"), str) and isinstance(value.get("id"), str):
                    nodes[value["id"]] = value
                pending.extend(value.values())
            elif isinstance(value, list):
                pending.extend(value)
        # Populate the HUD through the same operations used after live play.
        for game in games:
            for operation in game.status_ops():
                assert operation["op"] == "set", operation
                nodes[operation["id"]][operation["property"]] = operation["value"]
        destination = output / f"{palette}.json"
        destination.write_text(json.dumps(document, indent=2) + "\n")
        print(destination)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "target/qa/games/fixtures")
    args = parser.parse_args()
    export_games(args.output)


if __name__ == "__main__":
    main()
