#!/usr/bin/env python3
"""Vendor Zed's GPUI crates into crates/3rdparties as history-free snapshots.

Usage:
    python3 scripts/import_gpui_upstream.py --ref v1.9.0 --print-closure
    python3 scripts/import_gpui_upstream.py --ref v1.9.0           # vendor
    python3 scripts/import_gpui_upstream.py --ref v1.9.0 --check   # drift report
"""
from __future__ import annotations

import json
import tomllib
from pathlib import Path

ZED_GIT = "https://github.com/zed-industries/zed.git"
ZED_TARBALL = "https://github.com/zed-industries/zed/archive/refs/tags/{ref}.tar.gz"
DEFAULT_ROOTS = ["gpui", "gpui_macros", "gpui_macos", "gpui_linux", "collections", "util"]
EXCLUDED_CRATES = {"reqwest_client", "gpui_platform", "gpui_web"}
EXCLUDED_DIRS = {"examples", "benches"}
VENDOR_DIR = Path("crates/3rdparties")
CANON_KEY_ORDER = ["package", "version", "git", "tag", "rev", "branch",
                   "default-features", "features", "optional"]


class InlineTable(dict):
    """A dict rendered as a TOML inline table instead of a section."""


def read_toml(path: Path) -> dict:
    return tomllib.loads(Path(path).read_text())


def _toml_value(v) -> str:
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, str):
        return json.dumps(v)
    if isinstance(v, int):
        return str(v)
    if isinstance(v, list):
        return "[" + ", ".join(_toml_value(i) for i in v) + "]"
    if isinstance(v, InlineTable):
        inner = ", ".join(f"{k} = {_toml_value(x)}" for k, x in v.items())
        return "{ " + inner + " }" if inner else "{}"
    raise TypeError(f"unsupported TOML value: {v!r}")


def _quote_segment(seg: str) -> str:
    if seg and all(c.isalnum() or c in "-_" for c in seg):
        return seg
    if "'" in seg:
        raise ValueError(f"cannot quote TOML segment: {seg!r}")
    return f"'{seg}'"


def dumps_toml(doc: dict) -> str:
    lines: list[str] = []

    def emit(prefix: list[str], table: dict) -> None:
        scalars = {k: v for k, v in table.items()
                   if not isinstance(v, dict) or isinstance(v, InlineTable)}
        children = {k: v for k, v in table.items()
                    if isinstance(v, dict) and not isinstance(v, InlineTable)}
        # Header only for tables with own keys; pure-parent tables (e.g.
        # "target") must not emit intermediate headers.
        if prefix and scalars:
            lines.append("[" + ".".join(_quote_segment(s) for s in prefix) + "]")
        for k, v in scalars.items():
            lines.append(f"{k} = {_toml_value(v)}")
        if scalars:
            lines.append("")
        for name, child in children.items():
            emit(prefix + [name], child)

    emit([], doc)
    return "\n".join(lines).rstrip() + "\n"


def _ordered(d: dict) -> dict:
    keys = [k for k in CANON_KEY_ORDER if k in d]
    keys += [k for k in d if k not in CANON_KEY_ORDER]
    return {k: d[k] for k in keys}


def dep_base(name: str, spec, ws_deps: dict) -> dict:
    """Resolve a dependency entry to a plain spec dict (workspace refs expanded)."""
    if isinstance(spec, str):
        return {"version": spec}
    spec = dict(spec)
    if spec.pop("workspace", False):
        ws = ws_deps.get(name)
        if ws is None:
            raise SystemExit(f"error: {name}: not found in upstream workspace deps")
        return dict(ws) if isinstance(ws, dict) else {"version": ws}
    return spec


def _ws_value(value, ws_pkg: dict, key: str):
    if isinstance(value, dict) and value.get("workspace") is True:
        return ws_pkg.get(key)
    return value


def load_zed(zdir: Path, ref: str) -> dict:
    """Build the rewrite context from an upstream checkout."""
    root = read_toml(Path(zdir) / "Cargo.toml")
    ws = root.get("workspace", {})
    ws_deps = ws.get("dependencies", {})
    ws_pkg = ws.get("package", {})
    versions: dict = {}
    for dep_name, spec in ws_deps.items():
        if isinstance(spec, dict) and "path" in spec:
            canonical = spec.get("package", dep_name)
            manifest = read_toml(Path(zdir) / spec["path"] / "Cargo.toml")
            versions[canonical] = _ws_value(
                manifest.get("package", {}).get("version"), ws_pkg, "version"
            )
    return {"ref": ref, "ws_deps": ws_deps, "ws_pkg": ws_pkg, "versions": versions}


def resolve_dep(name: str, spec, ctx: dict):
    """Map one dependency entry to (canonical_name, toml_value) or (None, None) to drop."""
    if isinstance(spec, str):
        return (None, None) if name in EXCLUDED_CRATES else (name, spec)
    # Excluded crates are dropped before workspace expansion: they may be
    # absent from upstream [workspace.dependencies] (e.g. dev-deps).
    if name in EXCLUDED_CRATES:
        return None, None
    raw = dict(spec)
    extras = {k: raw.pop(k) for k in ("default-features", "features", "optional") if k in raw}
    base = dep_base(name, raw, ctx["ws_deps"])
    canonical = base.get("package", name)
    if canonical in EXCLUDED_CRATES:
        return None, None
    if "path" in base:  # upstream-internal path dep -> git+tag; root [patch] redirects
        version = ctx["versions"].get(canonical)
        if version is None:
            raise SystemExit(f"error: internal crate not in vendor set: {canonical}")
        merged = {"version": version, "git": ZED_GIT, "tag": ctx["ref"]}
        if canonical != name:
            merged = {"package": canonical, **merged}
    else:
        merged = dict(base)
        if canonical != name:
            merged = {"package": canonical, **{k: v for k, v in merged.items() if k != "package"}}
    merged.update(extras)
    if set(merged) == {"version"}:
        return canonical, merged["version"]
    return canonical, InlineTable(_ordered(merged))
