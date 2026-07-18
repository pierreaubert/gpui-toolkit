#!/usr/bin/env python3
"""Fail if any package in the cargo graph still comes from zed-industries/zed.git."""
import json
import subprocess
import sys

ZED_MARKER = "zed-industries/zed"


def find_zed_sources(metadata: dict) -> list[str]:
    return sorted({
        pkg["name"]
        for pkg in metadata.get("packages", [])
        if pkg.get("source") and ZED_MARKER in pkg["source"]
    })


def main() -> int:
    out = subprocess.run(
        ["cargo", "metadata", "--format-version", "1"],
        capture_output=True, text=True, check=True,
    ).stdout
    bad = find_zed_sources(json.loads(out))
    if bad:
        print("error: zed.git sources still in graph: " + ", ".join(bad))
        return 1
    print("ok: no zed-industries/zed.git sources in dependency graph")
    return 0


if __name__ == "__main__":
    sys.exit(main())
