#!/usr/bin/env python3
"""Build a gpui-toolkit wheel with a prebuilt native Python host."""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PACKAGE = ROOT / "crates" / "gpui-python-runtime" / "python" / "gpui_toolkit"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", type=Path, required=True, help="release gpui-python-host executable")
    parser.add_argument("--output", type=Path, default=ROOT / "dist")
    parser.add_argument("--platform-tag", help="wheel platform tag for the bundled native host")
    args = parser.parse_args()

    host = args.host if args.host.is_absolute() else ROOT / args.host
    if not host.is_file():
        parser.error(f"native host does not exist: {host}")

    executable_name = "gpui-python-host.exe" if os.name == "nt" else "gpui-python-host"
    bundled = PACKAGE / "bin" / executable_name
    bundled.parent.mkdir(parents=True, exist_ok=True)
    args.output.mkdir(parents=True, exist_ok=True)

    existing_bundled = bundled.read_bytes() if bundled.exists() else None
    existing_mode = bundled.stat().st_mode if bundled.exists() else None

    existing_wheels = set(args.output.glob("gpui_toolkit-*.whl"))
    shutil.copyfile(host, bundled)
    if os.name != "nt":
        bundled.chmod(bundled.stat().st_mode | 0o111)

    try:
        subprocess.run(
            [
                sys.executable,
                "-m",
                "pip",
                "wheel",
                "--no-deps",
                "--no-build-isolation",
                "--wheel-dir",
                str(args.output),
                str(ROOT),
            ],
            cwd=ROOT,
            check=True,
        )
        if args.platform_tag:
            new_wheels = sorted(set(args.output.glob("gpui_toolkit-*.whl")) - existing_wheels)
            if len(new_wheels) != 1:
                raise RuntimeError(f"expected one new wheel, found: {new_wheels}")
            subprocess.run(
                [
                    sys.executable,
                    "-m",
                    "wheel",
                    "tags",
                    "--remove",
                    "--platform-tag",
                    args.platform_tag,
                    str(new_wheels[0]),
                ],
                cwd=ROOT,
                check=True,
            )
    finally:
        bundled.unlink(missing_ok=True)
        if existing_bundled is not None:
            bundled.write_bytes(existing_bundled)
            bundled.chmod(existing_mode)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
