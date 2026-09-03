#!/usr/bin/env python3
"""Validate the strict, per-symbol v2 Python parity registry."""
from __future__ import annotations

import importlib
import inspect
import json
import pathlib
import sys
import tomllib


ROOT = pathlib.Path(__file__).resolve().parents[1]
PYTHON_SOURCE = ROOT / "crates/gpui-python-runtime/python"
VALID_DISPOSITIONS = {"direct", "translated", "host-owned", "intentionally-excluded"}
VALID_KINDS = {"type", "constructor", "builder_method", "event", "slot", "return_type"}


def rustdoc_classification_audit(
    data: dict[str, object], inventory: dict[str, object]
) -> tuple[list[str], int, int]:
    errors: list[str] = []
    symbols = {
        f"{symbol['package']}:{symbol['id']}": symbol
        for crate in inventory.get("crates", [])
        for symbol in crate.get("symbols", [])
    }
    classified: set[str] = set()
    for entry in data.get("rustdoc_classification", []):
        missing = [
            key
            for key in ("package", "rustdoc_id", "disposition", "rationale")
            if not entry.get(key)
        ]
        if missing:
            errors.append(
                f"rustdoc classification {entry.get('rustdoc_id', '<unknown>')}: "
                f"missing {', '.join(missing)}"
            )
            continue
        key = f"{entry['package']}:{entry['rustdoc_id']}"
        symbol = symbols.get(key)
        if symbol is None:
            errors.append(f"rustdoc classification references missing symbol {key}")
        elif key in classified:
            errors.append(f"rustdoc symbol classified more than once: {key}")
        else:
            classified.add(key)
        if entry["disposition"] not in VALID_DISPOSITIONS:
            errors.append(f"rustdoc classification {key}: invalid disposition")
        if entry["disposition"] != "intentionally-excluded":
            binding_missing = [
                name for name in ("python_path", "tests") if not entry.get(name)
            ]
            if binding_missing:
                errors.append(
                    f"rustdoc classification {key}: missing {', '.join(binding_missing)}"
                )
            else:
                try:
                    python_value = resolve(entry["python_path"])
                except (ImportError, AttributeError) as error:
                    errors.append(f"rustdoc classification {key}: unresolved Python path: {error}")
                else:
                    if entry.get("include_signature") and symbol is not None:
                        rust_parameters = symbol.get("signature", {}).get(
                            "parameters", []
                        )
                        parameter_map = entry.get("parameter_map", {})
                        try:
                            python_parameters = inspect.signature(python_value).parameters
                        except (TypeError, ValueError) as error:
                            errors.append(
                                f"rustdoc classification {key}: Python signature unavailable: {error}"
                            )
                        else:
                            for parameter in rust_parameters:
                                python_name = parameter_map.get(
                                    parameter["name"], parameter["name"]
                                )
                                if python_name not in python_parameters:
                                    errors.append(
                                        f"rustdoc classification {key}: Rust parameter "
                                        f"{parameter['name']} does not map to Python parameter {python_name}"
                                    )
                        children = {
                            child_key
                            for child_key, child in symbols.items()
                            if child.get("owner") == entry["rustdoc_id"]
                            and child.get("kind") in {"parameter", "return"}
                        }
                        duplicates = children & classified
                        if duplicates:
                            errors.append(
                                f"rustdoc classification {key}: signature overlaps {len(duplicates)} symbols"
                            )
                        classified.update(children)
                for test in entry["tests"]:
                    if not (ROOT / test).is_file():
                        errors.append(f"rustdoc classification {key}: missing test {test}")

    for rule in data.get("rustdoc_classification_rule", []):
        missing = [
            key
            for key in ("id", "package", "path_prefix", "disposition", "rationale")
            if not rule.get(key)
        ]
        if missing:
            errors.append(
                f"rustdoc classification rule {rule.get('id', '<unknown>')}: "
                f"missing {', '.join(missing)}"
            )
            continue
        if rule["disposition"] != "intentionally-excluded":
            errors.append(
                f"rustdoc classification rule {rule['id']}: only reviewed exclusions may use prefix rules"
            )
            continue
        prefix = f"{rule['package']}:{rule['path_prefix']}"
        matches = {
            key
            for key in symbols
            if key == prefix or key.startswith(prefix + "::") or key.startswith(prefix + "#")
        }
        if not matches:
            errors.append(f"rustdoc classification rule {rule['id']} matches no symbols")
            continue
        duplicates = matches & classified
        if duplicates:
            errors.append(
                f"rustdoc classification rule {rule['id']} overlaps {len(duplicates)} symbols"
            )
        classified.update(matches)

    for scope in data.get("rustdoc_completion_scope", []):
        missing = [
            key
            for key in ("id", "package", "path_prefix", "kinds")
            if not scope.get(key)
        ]
        if missing:
            errors.append(
                f"rustdoc completion scope {scope.get('id', '<unknown>')}: "
                f"missing {', '.join(missing)}"
            )
            continue
        prefix = f"{scope['package']}:{scope['path_prefix']}"
        matches = {
            key
            for key, symbol in symbols.items()
            if (key == prefix or key.startswith(prefix + "::") or key.startswith(prefix + "#"))
            and symbol.get("kind") in scope["kinds"]
        }
        if not matches:
            errors.append(f"rustdoc completion scope {scope['id']} matches no symbols")
            continue
        uncovered = matches - classified
        if uncovered:
            errors.append(
                f"rustdoc completion scope {scope['id']} has {len(uncovered)} unclassified symbols"
            )
    return errors, len(classified), len(symbols) - len(classified)


def resolve(path: str) -> object:
    parts = path.split(".")
    if parts[0] != "gpui_toolkit":
        raise ImportError("v2 paths must start with gpui_toolkit")
    # V2 symbols live in explicit first-level package modules; resolving this
    # way avoids mistaking a class name for a missing nested module.
    module = importlib.import_module(".".join(parts[:2]))
    value: object = module
    for part in parts[2:]:
        value = getattr(value, part)
    return value


def validate() -> list[str]:
    sys.path.insert(0, str(PYTHON_SOURCE))
    data = tomllib.loads((ROOT / "python-surface.toml").read_text())
    errors: list[str] = []
    surface = data.get("surface", {})
    inventory_name = surface.get("rustdoc_inventory")
    expected_packages = surface.get("rustdoc_packages", [])
    expected_inventory_digest = surface.get("rustdoc_inventory_digest")
    if not inventory_name:
        errors.append("surface: missing rustdoc_inventory")
    else:
        inventory_path = ROOT / inventory_name
        if not inventory_path.is_file():
            errors.append(f"surface: missing rustdoc inventory {inventory_name}")
        else:
            inventory = json.loads(inventory_path.read_text())
            if inventory.get("schema_version") != 1:
                errors.append("surface: unsupported rustdoc inventory schema")
            if inventory.get("packages") != expected_packages:
                errors.append("surface: rustdoc package list does not match inventory")
            if inventory.get("digest") != expected_inventory_digest:
                errors.append("surface: rustdoc inventory digest has not been reviewed")
            rustdoc_ids: list[str] = []
            for crate in inventory.get("crates", []):
                symbols = crate.get("symbols", [])
                if crate.get("symbol_count") != len(symbols):
                    errors.append(
                        f"surface: rustdoc count mismatch for {crate.get('package', '<unknown>')}"
                    )
                for symbol in symbols:
                    symbol_id = symbol.get("id")
                    if not symbol_id or not symbol.get("kind") or not symbol.get("package"):
                        errors.append("surface: malformed rustdoc symbol entry")
                        continue
                    rustdoc_ids.append(f"{symbol['package']}:{symbol_id}")
            if len(rustdoc_ids) != len(set(rustdoc_ids)):
                errors.append("surface: duplicate rustdoc symbol IDs")
            classification_errors, _, unclassified = rustdoc_classification_audit(
                data, inventory
            )
            errors.extend(classification_errors)
            if surface.get("release_ready") and unclassified:
                errors.append(
                    f"surface: release_ready requires classifying {unclassified} rustdoc symbols"
                )
    capabilities = data.get("capability", [])
    capability_ids = [entry.get("id") for entry in capabilities]
    if len(capability_ids) != len(set(capability_ids)):
        errors.append("duplicate v2 capability IDs")
    available_capabilities = set(capability_ids)
    for requirement in data.get("requirement", []):
        capability = requirement.get("capability")
        if not requirement.get("id") or not capability:
            errors.append("v2 requirement requires id and capability")
        elif capability not in available_capabilities:
            errors.append(f"{requirement['id']}: missing capability {capability}")
    for capability in capabilities:
        missing = [key for key in ("id", "rust_path", "python_path", "disposition", "tests") if not capability.get(key)]
        if missing:
            errors.append(f"{capability.get('id', '<unknown>')}: missing {', '.join(missing)}")
        for test in capability.get("tests", []):
            if not (ROOT / test).is_file():
                errors.append(f"{capability.get('id', '<unknown>')}: missing test {test}")
    symbols = data.get("symbol", [])
    ids = [symbol.get("id") for symbol in symbols]
    if len(ids) != len(set(ids)):
        errors.append("duplicate v2 symbol IDs")
    for symbol in symbols:
        missing = [key for key in ("id", "rust_path", "python_path", "disposition", "kind", "tests") if not symbol.get(key)]
        if missing:
            errors.append(f"{symbol.get('id', '<unknown>')}: missing {', '.join(missing)}")
            continue
        if symbol["disposition"] not in VALID_DISPOSITIONS:
            errors.append(f"{symbol['id']}: invalid disposition")
        if symbol["kind"] not in VALID_KINDS:
            errors.append(f"{symbol['id']}: invalid kind")
        try:
            resolve(symbol["python_path"])
        except (ImportError, AttributeError) as error:
            errors.append(f"{symbol['id']}: unresolved Python path: {error}")
        for test in symbol["tests"]:
            if not (ROOT / test).is_file():
                errors.append(f"{symbol['id']}: missing test {test}")
    return errors


if __name__ == "__main__":
    failures = validate()
    if failures:
        print("\n".join(failures), file=sys.stderr)
        raise SystemExit(1)
    manifest = tomllib.loads((ROOT / "python-surface.toml").read_text())
    inventory = json.loads(
        (ROOT / manifest["surface"]["rustdoc_inventory"]).read_text()
    )
    _, classified, unclassified = rustdoc_classification_audit(manifest, inventory)
    print(
        "Python v2 symbol registry is valid; rustdoc classifications: "
        f"{classified} classified, {unclassified} open"
    )
