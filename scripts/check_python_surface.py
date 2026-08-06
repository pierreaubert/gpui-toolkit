#!/usr/bin/env python3
"""Validate and report the checked-in Python surface registry.

The command is intentionally dependency-free so it can run in release and CI
environments before code generation is introduced.  ``--strict`` becomes the
parity release gate once every inventory entry has capability dispositions.
"""

from __future__ import annotations

import argparse
import importlib
import pathlib
import sys
import tomllib
from collections.abc import Iterable


ROOT = pathlib.Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "python-surface.toml"
PYTHON_SOURCE = ROOT / "crates/gpui-python-runtime/python"
DISPOSITIONS = {
    "direct", "declarative", "command", "event", "opaque", "host-owned",
    "platform-unavailable", "non-consumer",
}


def first_party_crates() -> set[str]:
    return {
        path.name
        for path in (ROOT / "crates").glob("gpui-*")
        if (path / "Cargo.toml").is_file() and path.name != "gpui-showcase"
    }


def load_manifest(path: pathlib.Path = MANIFEST) -> dict:
    with path.open("rb") as manifest:
        return tomllib.load(manifest)


def validate(data: dict, root: pathlib.Path = ROOT) -> list[str]:
    errors: list[str] = []
    if data.get("surface", {}).get("schema_version") != 1:
        errors.append("surface.schema_version must be 1")
    inventory = data.get("inventory", {}).get("crate", [])
    ids = [entry.get("id") for entry in inventory]
    if len(ids) != len(set(ids)):
        errors.append("inventory contains duplicate crate IDs")
    inventory_ids = {entry.get("id") for entry in inventory}
    missing = sorted(first_party_crates() - inventory_ids)
    extra = sorted(inventory_ids - first_party_crates())
    if missing:
        errors.append("inventory missing first-party crates: " + ", ".join(missing))
    if extra:
        errors.append("inventory lists unknown crates: " + ", ".join(extra))
    for entry in inventory:
        if not (root / entry.get("path", "")).is_dir():
            errors.append(f"inventory path missing for {entry.get('id')}: {entry.get('path')}")

    capabilities = data.get("capability", [])
    capability_ids = [entry.get("id") for entry in capabilities]
    if len(capability_ids) != len(set(capability_ids)):
        errors.append("capability registry contains duplicate IDs")
    for entry in capabilities:
        required = ("id", "rust_path", "python_path", "disposition", "tests")
        absent = [field for field in required if not entry.get(field)]
        if absent:
            errors.append(f"capability {entry.get('id', '<unknown>')} missing: {', '.join(absent)}")
        if entry.get("disposition") not in DISPOSITIONS:
            errors.append(f"capability {entry.get('id')} has invalid disposition")
        for test in entry.get("tests", []):
            if not (root / test).is_file():
                errors.append(f"capability {entry.get('id')} references missing test {test}")
        python_path = entry.get("python_path")
        if python_path:
            try:
                resolve_python_path(python_path)
            except (ImportError, AttributeError) as error:
                errors.append(f"capability {entry.get('id')} has unresolved Python path {python_path}: {error}")
    if {entry.get("id") for entry in capabilities} != capability_descriptor_ids():
        errors.append("gpui_toolkit.capabilities() does not match manifest capability IDs")
    return errors


def resolve_python_path(path: str) -> object:
    """Resolve ``package.module.Object`` without accepting an eval escape hatch."""
    if str(PYTHON_SOURCE) not in sys.path:
        sys.path.insert(0, str(PYTHON_SOURCE))
    components = path.split(".")
    if components[0] != "gpui_toolkit":
        raise ImportError("paths must begin with gpui_toolkit")
    module = None
    module_end = 1
    for end in range(len(components), 0, -1):
        try:
            module = importlib.import_module(".".join(components[:end]))
            module_end = end
            break
        except ModuleNotFoundError as error:
            candidate = ".".join(components[:end])
            # A missing suffix can be an attribute rather than a module; an
            # unrelated missing import inside a real module must still fail.
            if error.name != candidate and not candidate.startswith(f"{error.name}."):
                raise
    if module is None:
        raise ImportError(path)
    value: object = module
    for attribute in components[module_end:]:
        value = getattr(value, attribute)
    return value


def inventory_without_disposition(data: dict) -> set[str]:
    inventory = {entry["id"] for entry in data.get("inventory", {}).get("crate", [])}
    covered = {
        entry["rust_path"].split("::", 1)[0].replace("_", "-")
        for entry in data.get("capability", [])
        if entry.get("rust_path")
    }
    return inventory - covered


def requirements_without_capability(data: dict) -> set[str]:
    """Return design-level requirement IDs without an explicit capability link.

    Crate inventory prevents whole crates disappearing.  This separate matrix
    prevents one convenient class from being mistaken for every public family
    named by the full-surface design.
    """
    capabilities = {entry["id"] for entry in data.get("capability", [])}
    return {
        entry["id"]
        for entry in data.get("requirement", [])
        if entry.get("capability") not in capabilities
    }


def capability_descriptor_ids() -> set[str]:
    if str(PYTHON_SOURCE) not in sys.path:
        sys.path.insert(0, str(PYTHON_SOURCE))
    from gpui_toolkit.capabilities import capabilities
    return {item.id for item in capabilities()}


def main(argv: Iterable[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--strict", action="store_true", help="fail until every crate has a disposition")
    args = parser.parse_args(argv)
    data = load_manifest()
    errors = validate(data)
    unclassified = sorted(inventory_without_disposition(data))
    uncovered_requirements = sorted(requirements_without_capability(data))
    print(f"Python surface registry: {len(data.get('capability', []))} capabilities")
    print(f"First-party crate inventory: {len(data.get('inventory', {}).get('crate', []))} crates")
    if unclassified:
        print("Unclassified crates: " + ", ".join(unclassified))
    if uncovered_requirements:
        print("Uncovered design requirements: " + ", ".join(uncovered_requirements))
    if errors:
        print("Registry errors:", *errors, sep="\n- ", file=sys.stderr)
        return 1
    if args.strict and (unclassified or uncovered_requirements or not data.get("surface", {}).get("release_ready", False)):
        reason = (
            "unclassified crates" if unclassified
            else "uncovered design requirements" if uncovered_requirements
            else "surface.release_ready is false"
        )
        print(f"Strict parity gate blocked by {reason}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
