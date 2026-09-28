#!/usr/bin/env python3
"""Resolve a built cargo binary path from ``--message-format=json`` output.

Build wrappers such as mbx relocate the target directory, so recipes must
not assume ``target/release/<bin>``. Usage::

    cargo build --release --bin my-bin --message-format=json > artifacts.json
    python3 scripts/cargo_bin_path.py --bin my-bin artifacts.json
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bin", required=True, help="cargo binary target name")
    parser.add_argument(
        "artifacts", type=Path, help="file holding cargo json message lines"
    )
    args = parser.parse_args()
    try:
        lines = args.artifacts.read_text().splitlines()
    except OSError as error:
        parser.error(f"cannot read build output {args.artifacts}: {error}")
    executables = []
    for line in lines:
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            message = json.loads(line)
        except ValueError:
            continue
        if (
            message.get("reason") == "compiler-artifact"
            and message.get("target", {}).get("name") == args.bin
            and message.get("executable")
        ):
            executables.append(message["executable"])
    if not executables:
        parser.error(f"no executable for bin target {args.bin!r} in {args.artifacts}")
    print(executables[-1])
    return 0


if __name__ == "__main__":
    sys.exit(main())
