"""Mobile project generation and validation for Python-authored apps.

This is the desktop-side entry point for the mobile hosts: it scaffolds a
`python-mobile` project through the `gpui-scaffolder` binary and validates
an app's declarations (`build_app().to_spec()`) before any mobile build
starts. Device builds themselves run from the generated Justfile.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import os
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

MANIFEST_NAME = "gpui-mobile.toml"


@dataclass(frozen=True)
class MobileManifest:
    """Identity section of a generated project's `gpui-mobile.toml`."""

    name: str
    title: str
    version: str
    bundle_identifier: str
    entry: str
    python_version: str
    toolkit_root: str

    @classmethod
    def from_toml(cls, text: str) -> "MobileManifest":
        try:
            import tomllib
        except ModuleNotFoundError as error:
            raise RuntimeError(
                "reading gpui-mobile.toml requires Python 3.11+ (tomllib)"
            ) from error
        data = tomllib.loads(text)
        app = data.get("app", {})
        python = data.get("python", {})
        toolkit = data.get("toolkit", {})
        fields = {
            "name": app.get("name"),
            "title": app.get("title"),
            "version": app.get("version"),
            "bundle_identifier": app.get("bundle_identifier"),
            "entry": app.get("entry"),
            "python_version": python.get("version"),
            "toolkit_root": toolkit.get("root"),
        }
        missing = [key for key, value in fields.items() if not isinstance(value, str) or not value]
        if missing:
            raise ValueError(
                f"gpui-mobile.toml is missing required string fields: {', '.join(missing)}"
            )
        return cls(**fields)  # type: ignore[arg-type]

    @classmethod
    def load(cls, project_dir: str | Path) -> "MobileManifest":
        path = Path(project_dir) / MANIFEST_NAME
        return cls.from_toml(path.read_text(encoding="utf-8"))


def find_scaffolder() -> str | None:
    """Locate the `gpui-scaffolder` binary (env override, then PATH)."""
    configured = os.environ.get("GPUI_SCAFFOLDER")
    if configured:
        return configured
    return shutil.which("gpui-scaffolder")


def scaffold_mobile_app(
    name: str,
    output_dir: str | Path,
    *,
    no_ios: bool = False,
    no_android: bool = False,
    dry_run: bool = False,
    scaffolder: str | None = None,
) -> subprocess.CompletedProcess[str]:
    """Run `gpui-scaffolder --template python-mobile` for `name`."""
    binary = scaffolder or find_scaffolder()
    if binary is None:
        raise RuntimeError(
            "gpui-scaffolder binary not found; install the toolkit CLI or set GPUI_SCAFFOLDER"
        )
    command = [
        binary,
        name,
        "--output-dir",
        str(output_dir),
        "--template",
        "python-mobile",
    ]
    if no_ios:
        command.append("--no-ios")
    if no_android:
        command.append("--no-android")
    if dry_run:
        command.append("--dry-run")
    return subprocess.run(command, check=True, capture_output=True, text=True)


def load_app(entry: str | Path) -> Any:
    """Import an app module by path and return its `App` instance."""
    path = Path(entry)
    spec = importlib.util.spec_from_file_location("gpui_mobile_user_app", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load Python app module from {path}")
    module = importlib.util.module_from_spec(spec)
    # Dataclasses resolve their defining module through sys.modules.
    sys.modules[spec.name] = module
    if str(path.parent) not in sys.path:
        sys.path.insert(0, str(path.parent))
    spec.loader.exec_module(module)
    build_app = getattr(module, "build_app", None)
    if callable(build_app):
        return build_app()
    app = getattr(module, "app", None)
    if app is not None:
        return app
    raise RuntimeError(f"{path} defines neither build_app() nor app")


def validate_app(entry: str | Path) -> dict[str, Any]:
    """Return the app's validated IR spec (the mobile preflight check)."""
    spec = load_app(entry).to_spec()
    json.dumps(spec)  # the session boundary is JSON; fail here if it is not
    return spec


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="python -m gpui_toolkit.mobile")
    subparsers = parser.add_subparsers(dest="command", required=True)

    scaffold = subparsers.add_parser("scaffold", help="generate a python-mobile project")
    scaffold.add_argument("name")
    scaffold.add_argument("--output-dir", default=".")
    scaffold.add_argument("--no-ios", action="store_true")
    scaffold.add_argument("--no-android", action="store_true")
    scaffold.add_argument("--dry-run", action="store_true")

    preflight = subparsers.add_parser("preflight", help="validate an app's declarations")
    preflight.add_argument("entry", nargs="?", default="app/main_app.py")

    args = parser.parse_args(argv)
    if args.command == "scaffold":
        result = scaffold_mobile_app(
            args.name,
            args.output_dir,
            no_ios=args.no_ios,
            no_android=args.no_android,
            dry_run=args.dry_run,
        )
        print(result.stdout, end="")
        return 0
    spec = validate_app(args.entry)
    print(
        f"validated Python GPUI app {spec['title']!r} "
        f"with {len(spec['sections'])} sections"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
