#!/usr/bin/env python3
"""Generate the dependency-free Python 3.10 capability descriptor table."""
from __future__ import annotations

import argparse
import pathlib
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "python-surface.toml"
OUTPUT = ROOT / "crates/gpui-python-runtime/python/gpui_toolkit/capabilities.py"


def render() -> str:
    with MANIFEST.open("rb") as stream:
        entries = tomllib.load(stream)["capability"]
    rows = "\n".join(
        f"    ({entry['id']!r}, {entry['disposition']!r}, {entry['python_path']!r}),"
        for entry in entries
    )
    return f'''\"\"\"Generated capability descriptors; do not edit by hand.\"\"\"
from __future__ import annotations
from dataclasses import dataclass

@dataclass(frozen=True)
class Capability:
    id: str
    disposition: str
    python_path: str

_ENTRIES = (
{rows}
)

def capabilities() -> tuple[Capability, ...]:
    return tuple(Capability(*entry) for entry in _ENTRIES)
'''


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    expected = render()
    if args.check:
        if not OUTPUT.is_file() or OUTPUT.read_text() != expected:
            print("generated capabilities.py is stale")
            return 1
        return 0
    OUTPUT.write_text(expected)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
