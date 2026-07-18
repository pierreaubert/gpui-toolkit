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
EXCLUDED_CRATES = {"reqwest_client", "gpui_platform", "gpui_web", "zlog", "ztracing", "ztracing_macro"}
EXCLUDED_DIRS = {"examples", "benches"}
VENDOR_DIR = Path("crates/3rdparties")
GPUI_IMAGE_FEATURES = ["bmp", "gif", "ico", "jpeg", "png", "pnm", "tiff", "webp"]
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


# --- Task 2: closure, manifest rewrite, vendoring, CLI ----------------------

import argparse
import posixpath
import shutil
import sys
import tarfile
import tempfile
import urllib.request

DEP_SECTIONS = ("dependencies", "dev-dependencies", "build-dependencies")


def _iter_dep_sections(manifest: dict):
    for section in DEP_SECTIONS:
        yield manifest.get(section, {})
    for cfg in manifest.get("target", {}).values():
        for section in DEP_SECTIONS:
            yield cfg.get(section, {})


def compute_closure(zdir: Path, ctx: dict, roots: list[str]) -> list[str]:
    """BFS over upstream-internal (path) deps starting from roots.

    Fills ctx['versions'] and ctx['paths'] (repo-relative source dir per
    crate: internal crates may live outside crates/, e.g. tooling/perf, or
    nested, e.g. crates/refineable/derive_refineable).
    """
    seen: set[str] = set()
    order: list[str] = []
    stack = [(root, f"crates/{root}") for root in roots]
    while stack:
        name, path = stack.pop()
        if name in seen:
            continue
        seen.add(name)
        order.append(name)
        ctx.setdefault("paths", {})[name] = path
        manifest = read_toml(Path(zdir) / path / "Cargo.toml")
        version = _ws_value(manifest.get("package", {}).get("version"), ctx["ws_pkg"], "version")
        if version is None:
            raise SystemExit(f"error: {name}: cannot resolve version")
        ctx["versions"][name] = version
        for deps in _iter_dep_sections(manifest):
            for dep_name, spec in deps.items():
                # Name-based exclusion before workspace expansion: excluded
                # crates may be absent from upstream [workspace.dependencies].
                if dep_name in EXCLUDED_CRATES:
                    continue
                base = dep_base(dep_name, spec, ctx["ws_deps"])
                canonical = base.get("package", dep_name)
                if "path" in base and canonical not in seen and canonical not in EXCLUDED_CRATES:
                    # Workspace-dep paths are repo-relative; a direct path dep
                    # would be relative to the crate's own directory.
                    dep_path = base["path"]
                    if not (isinstance(spec, dict) and spec.get("workspace")):
                        dep_path = posixpath.normpath(posixpath.join(path, dep_path))
                    stack.append((canonical, dep_path))
    return order


def rewrite_manifest(manifest: dict, ctx: dict) -> dict:
    pkg_in = manifest.get("package", {})
    pkg: dict = {"name": pkg_in["name"]}
    for key in ("version", "edition", "rust-version", "description", "license", "authors"):
        value = _ws_value(pkg_in.get(key), ctx["ws_pkg"], key)
        if value is not None:
            pkg[key] = value
    pkg["publish"] = False
    for key in ("homepage", "repository", "keywords", "categories", "build", "links"):
        if key in pkg_in and not isinstance(pkg_in[key], dict):
            pkg[key] = pkg_in[key]
    doc: dict = {"package": pkg}
    if "lib" in manifest:
        doc["lib"] = manifest["lib"]
    if "features" in manifest:
        doc["features"] = manifest["features"]
    for section in DEP_SECTIONS:
        deps = manifest.get(section)
        if not deps:
            continue
        out: dict = {}
        for name, spec in deps.items():
            canonical, value = resolve_dep(name, spec, ctx)
            if canonical is not None:
                out[name] = value
        if out:
            doc[section] = out
    for cfg_expr, cfg_table in manifest.get("target", {}).items():
        target_out: dict = {}
        for section in DEP_SECTIONS:
            deps = cfg_table.get(section)
            if not deps:
                continue
            out = {}
            for name, spec in deps.items():
                canonical, value = resolve_dep(name, spec, ctx)
                if canonical is not None:
                    out[name] = value
            if out:
                target_out[section] = out
        if target_out:
            doc.setdefault("target", {})[cfg_expr] = target_out
    return doc


def apply_local_manifest_policy(doc: dict) -> dict:
    """Apply small, documented dependency policies to regenerated snapshots."""
    package_name = doc["package"]["name"]
    for deps in _iter_dep_sections(doc):
        if package_name in {"gpui", "gpui_linux", "gpui_macos"} and "image" in deps:
            image = deps["image"]
            image = {"version": image} if isinstance(image, str) else dict(image)
            image["default-features"] = False
            image["features"] = GPUI_IMAGE_FEATURES
            deps["image"] = InlineTable(_ordered(image))

        if package_name in {"gpui_linux", "gpui_macos"} and "gpui" in deps:
            gpui = dict(deps["gpui"])
            gpui["default-features"] = False
            deps["gpui"] = InlineTable(_ordered(gpui))
    return doc


LOCAL_PATCHES_HEADER = "## Local patches"


def _vendored_md(name: str, path: str, ref: str, prior: str | None) -> str:
    if prior and LOCAL_PATCHES_HEADER in prior:
        tail = prior[prior.index(LOCAL_PATCHES_HEADER):].rstrip() + "\n"
    else:
        tail = LOCAL_PATCHES_HEADER + "\n\nnone\n"
    return (
        f"# Vendored: {name}\n\n"
        f"- Upstream: https://github.com/zed-industries/zed/tree/{ref}/{path}\n"
        f"- Base ref: {ref}\n"
        f"- Import: scripts/import_gpui_upstream.py (history-free snapshot)\n"
        f"- Excluded on import: examples/, benches/, deps on {', '.join(sorted(EXCLUDED_CRATES))}\n\n"
        + tail
    )


def vendor_crate(name: str, zdir: Path, ctx: dict, dest_root: Path) -> None:
    path = ctx.get("paths", {}).get(name, f"crates/{name}")
    src = Path(zdir) / path
    dst = Path(dest_root) / name
    prior_md = (dst / "VENDORED.md").read_text() if (dst / "VENDORED.md").exists() else None
    shutil.rmtree(dst, ignore_errors=True)
    shutil.copytree(src, dst, ignore=shutil.ignore_patterns(*EXCLUDED_DIRS))
    if not any(p.name.startswith("LICENSE") for p in dst.iterdir()):
        copied = False
        for candidate in ("LICENSE-APACHE", "LICENSE.apache", "LICENSE"):
            if (Path(zdir) / candidate).exists():
                shutil.copy(Path(zdir) / candidate, dst / "LICENSE-APACHE")
                copied = True
                break
        if not copied:
            raise SystemExit(f"error: no LICENSE file found upstream for {name}")
    manifest = read_toml(dst / "Cargo.toml")
    rewritten = apply_local_manifest_policy(rewrite_manifest(manifest, ctx))
    (dst / "Cargo.toml").write_text(dumps_toml(rewritten))
    (dst / "VENDORED.md").write_text(_vendored_md(name, path, ctx["ref"], prior_md))


def vendor_closure(closure: list[str], zdir: Path, ctx: dict, dest_root: Path,
                   skip: set[str] | None = None) -> None:
    """Vendor every crate in the closure, minus the skip set.

    Skipped crates are left untouched on disk (hand-maintained crates such as
    gpui_wgpu) but stay in the closure and the printed [patch] block.
    """
    skip = skip or set()
    for name in closure:
        if name in skip:
            print(f"skipped {name}")
            continue
        vendor_crate(name, zdir, ctx, dest_root)
        print(f"vendored {name}")


def fetch(ref: str, cache: Path) -> Path:
    """Download and partially extract the upstream tarball; return the checkout dir."""
    cache = Path(cache)
    cache.mkdir(parents=True, exist_ok=True)
    tarball = cache / f"{ref}.tar.gz"
    if not tarball.exists():
        print(f"downloading {ZED_TARBALL.format(ref=ref)}")
        urllib.request.urlretrieve(ZED_TARBALL.format(ref=ref), tarball)
    with tarfile.open(tarball) as tf:
        top = tf.getmembers()[0].name.split("/")[0]
        zdir = cache / top
        # Workspace path deps may live outside crates/ (e.g. tooling/): extract
        # every top-level dir a path dep points into, not just crates/.
        root_toml = tomllib.loads(tf.extractfile(f"{top}/Cargo.toml").read().decode())
        ws_deps = root_toml.get("workspace", {}).get("dependencies", {})
        dep_dirs = sorted({spec["path"].split("/")[0] for spec in ws_deps.values()
                           if isinstance(spec, dict) and "path" in spec})
        if not (zdir / "Cargo.toml").exists() or not all((zdir / d).exists() for d in dep_dirs):
            wanted = tuple(f"{top}/{p}" for p in ["Cargo.toml", "LICENSE", *dep_dirs])
            members = [m for m in tf.getmembers() if m.name.startswith(wanted)]
            tf.extractall(cache, members=members, filter="data")
    return zdir


def check(dest_root: Path, zdir: Path, ctx: dict, closure: list[str],
          skip: set[str] | None = None) -> list[str]:
    """Regenerate into a temp dir and byte-compare. Differences = drift or local patches.

    Skipped crates (hand-maintained, e.g. gpui_wgpu, gpui_macos) are excluded
    from checking entirely, same semantics as vendor_closure.
    """
    skip = skip or set()
    closure = [name for name in closure if name not in skip]
    diffs: list[str] = []
    with tempfile.TemporaryDirectory() as tmp:
        for name in closure:
            vendor_crate(name, zdir, ctx, Path(tmp))
        for name in closure:
            regen = {p.relative_to(Path(tmp) / name): p for p in (Path(tmp) / name).rglob("*") if p.is_file()}
            real_root = Path(dest_root) / name
            real = {p.relative_to(real_root): p for p in real_root.rglob("*") if p.is_file()} if real_root.exists() else {}
            for rel in sorted(set(regen) | set(real)):
                if rel not in regen:
                    diffs.append(f"{name}/{rel}: not regenerated (local addition)")
                elif rel not in real:
                    diffs.append(f"{name}/{rel}: missing in tree")
                elif regen[rel].read_bytes() != real[rel].read_bytes():
                    diffs.append(f"{name}/{rel}: differs from regeneration")
    return diffs


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ref", default="v1.9.0")
    parser.add_argument("--cache", default="target/zed-upstream")
    parser.add_argument("--root", action="append", default=[])
    parser.add_argument("--skip", action="append", default=[], metavar="NAME",
                        help="do not vendor NAME (kept in closure and [patch] block); repeatable")
    parser.add_argument("--print-closure", action="store_true")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    roots = DEFAULT_ROOTS + [r for r in args.root if r not in DEFAULT_ROOTS]
    zdir = fetch(args.ref, Path(args.cache))
    ctx = load_zed(zdir, args.ref)
    closure = compute_closure(zdir, ctx, roots)
    if args.print_closure:
        print("closure:", " ".join(closure))
        for name in closure:
            print(f"  {name} {ctx['versions'][name]}")
    if args.check:
        diffs = check(VENDOR_DIR, zdir, ctx, closure, skip=set(args.skip))
        if diffs:
            print("drift / local patch surface:")
            print("\n".join(diffs))
            return 1
        print("ok: vendored tree matches regeneration")
        return 0
    if not args.print_closure:
        vendor_closure(closure, zdir, ctx, VENDOR_DIR, skip=set(args.skip))
        print("\n[patch.\"https://github.com/zed-industries/zed.git\"]")
        for name in closure:
            print(f"{name} = {{ path = \"crates/3rdparties/{name}\" }}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
