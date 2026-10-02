#!/usr/bin/env python3
"""Capture installed native games and their Android accessibility hierarchy."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import struct
import subprocess
import time
import xml.etree.ElementTree as ET


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    sdk = os.environ.get("ANDROID_HOME", "/opt/homebrew/share/android-commandlinetools")
    parser.add_argument("--adb", type=Path, default=Path(sdk) / "platform-tools/adb")
    parser.add_argument("--serial", default="emulator-5554")
    parser.add_argument("--output", type=Path, default=Path("target/qa/games/android"))
    parser.add_argument("--game", choices=("zip", "queens", "sudoku", "tetris"), action="append")
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    adb = [str(args.adb), "-s", args.serial]

    def run(*command: str) -> bytes:
        return subprocess.check_output([*adb, *command], stderr=subprocess.STDOUT)

    package = "org.spinorama.gpui.showcase"
    device = run("shell", "getprop", "ro.product.model").decode().strip()
    api = run("shell", "getprop", "ro.build.version.sdk").decode().strip()
    abi = run("shell", "getprop", "ro.product.cpu.abi").decode().strip()
    reports = []
    for game in args.game or ("zip", "queens", "sudoku", "tetris"):
        run("shell", "am", "force-stop", package)
        launch = run("shell", "am", "start", "-W", "-n",
                     f"{package}/dev.gpui.mobile.GpuiActivity",
                     "--es", "section", "Games", "--es", "game", game).decode()
        pid = run("shell", "pidof", package).decode().strip()
        assert pid.isdigit(), f"{game} process did not start: {launch}"
        time.sleep(2)
        pixels = run("exec-out", "screencap", "-p")
        assert pixels.startswith(b"\x89PNG\r\n\x1a\n"), f"{game} capture is not PNG"
        width, height = struct.unpack(">II", pixels[16:24])
        screenshot = args.output / f"android-{game}.png"
        screenshot.write_bytes(pixels)
        for _ in range(2):
            run("shell", "uiautomator", "dump", "/sdcard/games-qa-window.xml")
        hierarchy = run("exec-out", "cat", "/sdcard/games-qa-window.xml")
        xml_path = args.output / f"android-{game}-accessibility.xml"
        xml_path.write_bytes(hierarchy)
        nodes = list(ET.fromstring(hierarchy).iter("node"))
        cells = [node for node in nodes if
                 node.attrib.get("content-desc", "").startswith("Row ")]
        expected = {"zip": 25, "queens": 64, "sudoku": 81, "tetris": 200}.get(game)
        if expected is not None:
            assert len(cells) == expected, f"{game}: {len(cells)}/{expected} accessible cells"
            board_label = "Tetris playfield" if game == "tetris" else f"{game.title()} board"
            boards = [node for node in nodes if
                      node.attrib.get("content-desc", "") == board_label]
            assert len(boards) == 1, f"{game}: missing board accessibility container"
            board = [int(value) for value in re.findall(r"\d+", boards[0].attrib["bounds"])]
            assert 0 <= board[0] < board[2] <= width, boards[0].attrib
            assert 0 <= board[1] < board[3] <= height, boards[0].attrib
            for cell in cells:
                bounds = [int(value) for value in re.findall(r"\d+", cell.attrib["bounds"])]
                assert board[0] <= bounds[0] < bounds[2] <= board[2], cell.attrib
                assert board[1] <= bounds[1] < bounds[3] <= board[3], cell.attrib
                if cell.attrib.get("enabled") == "true":
                    assert cell.attrib.get("clickable") == "true", cell.attrib
        report = {
            "schema_version": 1, "game": game, "device": device,
            "serial": args.serial, "api_level": api, "abi": abi,
            "process_id": int(pid), "pixel_width": width, "pixel_height": height,
            "screenshot": screenshot.name, "pixel_sha256": hashlib.sha256(pixels).hexdigest(),
            "accessibility": xml_path.name, "accessible_cells": len(cells),
            "accessible_nodes": len(nodes), "checks_passed": True,
            "interaction_scope": ["launch", "pixel-capture", "accessibility-hierarchy"],
            "finger_input_measured": False, "input_to_present_measured": False,
        }
        (args.output / f"android-{game}.json").write_text(json.dumps(report, indent=2) + "\n")
        reports.append(report)
        print(f"{game}: {width}x{height}, {len(cells)} accessible cells")
    (args.output / "android-manifest.json").write_text(json.dumps(reports, indent=2) + "\n")
    run("shell", "am", "force-stop", package)


if __name__ == "__main__":
    main()
