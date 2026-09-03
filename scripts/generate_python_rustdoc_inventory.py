#!/usr/bin/env python3
"""Generate the frozen Rust public-API inventory used by Python v2 parity.

The inventory is deliberately derived from rustdoc JSON rather than source-text
heuristics.  It records public items, inherent methods, fields, callable
parameters, and return types for the crates currently in the Python delivery
wave.  A checked-in snapshot makes a new Rust public API a review-visible
change even while manual Python classifications are still being completed.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import subprocess
from typing import Any, Iterable, Mapping


ROOT = pathlib.Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "python-rustdoc-inventory.json"
DEFAULT_PACKAGES = ("gpui-d3rs", "gpui-px")
DEFAULT_TOOLCHAIN = "nightly-2026-09-02"
RUSTDOC_FILE_NAMES = {"gpui-d3rs": "d3rs.json", "gpui-px": "gpui_px.json"}
PUBLIC_PATH_KINDS = {
    "constant",
    "enum",
    "function",
    "macro",
    "module",
    "proc_attribute",
    "proc_derive",
    "static",
    "struct",
    "trait",
    "trait_alias",
    "type_alias",
    "union",
    "variant",
}


def _item_inner(item: Mapping[str, Any]) -> tuple[str, Any]:
    inner = item.get("inner", {})
    if len(inner) != 1:
        raise ValueError(f"rustdoc item {item.get('id')} has ambiguous inner data")
    return next(iter(inner.items()))


def _canonical_path(path: Iterable[str]) -> str:
    return "::".join(path)


def _normalized(value: Any, paths: Mapping[str, Any]) -> Any:
    """Remove rustdoc-local IDs while preserving their canonical type paths."""

    if isinstance(value, list):
        return [_normalized(item, paths) for item in value]
    if not isinstance(value, dict):
        return value
    result: dict[str, Any] = {}
    for key, item in value.items():
        if key == "id":
            path = paths.get(str(item))
            if path is not None:
                result["path"] = _canonical_path(path["path"])
            continue
        result[key] = _normalized(item, paths)
    return result


def _source(item: Mapping[str, Any]) -> dict[str, Any] | None:
    span = item.get("span")
    if not span:
        return None
    # Line/column positions are intentionally omitted: moving an unchanged
    # item within its source file must not look like a public API change.
    return {"file": span["filename"]}


def _base_record(
    package: str,
    symbol_id: str,
    kind: str,
    item: Mapping[str, Any],
) -> dict[str, Any]:
    record: dict[str, Any] = {
        "id": symbol_id,
        "package": package,
        "kind": kind,
        "deprecated": item.get("deprecation") is not None,
    }
    attrs = sorted(attr for attr in item.get("attrs", []) if "cfg" in attr)
    if attrs:
        record["feature_attrs"] = attrs
    source = _source(item)
    if source is not None:
        record["source"] = source
    return record


def _callable_records(
    package: str,
    symbol_id: str,
    kind: str,
    item: Mapping[str, Any],
    function: Mapping[str, Any],
    paths: Mapping[str, Any],
) -> list[dict[str, Any]]:
    record = _base_record(package, symbol_id, kind, item)
    signature = function["sig"]
    inputs = signature.get("inputs", [])
    record["signature"] = {
        "parameters": [
            {"name": name, "type": _normalized(type_, paths)}
            for name, type_ in inputs
        ],
        "return": _normalized(signature.get("output"), paths),
        "is_async": bool(function.get("header", {}).get("is_async")),
        "is_unsafe": bool(function.get("header", {}).get("is_unsafe")),
        "generic_parameters": len(function.get("generics", {}).get("params", [])),
    }
    records = [record]
    for position, (name, type_) in enumerate(inputs):
        parameter = _base_record(
            package,
            f"{symbol_id}#parameter:{position}:{name}",
            "parameter",
            item,
        )
        parameter["owner"] = symbol_id
        parameter["position"] = position
        parameter["name"] = name
        parameter["type"] = _normalized(type_, paths)
        records.append(parameter)
    if signature.get("output") is not None:
        result = _base_record(package, f"{symbol_id}#return", "return", item)
        result["owner"] = symbol_id
        result["type"] = _normalized(signature["output"], paths)
        records.append(result)
    return records


def _public_associated_items(
    package: str,
    parent_id: str,
    parent: Mapping[str, Any],
    index: Mapping[str, Any],
    paths: Mapping[str, Any],
) -> list[dict[str, Any]]:
    parent_kind, parent_inner = _item_inner(parent)
    records: list[dict[str, Any]] = []

    if parent_kind == "struct":
        struct_kind = parent_inner["kind"]
        if "plain" in struct_kind:
            field_ids = struct_kind["plain"].get("fields", [])
        elif "tuple" in struct_kind:
            field_ids = [field_id for field_id in struct_kind["tuple"] if field_id is not None]
        else:
            field_ids = []
        impl_ids = parent_inner.get("impls", [])
    elif parent_kind == "union":
        field_ids = parent_inner.get("fields", [])
        impl_ids = parent_inner.get("impls", [])
    elif parent_kind == "enum":
        field_ids = []
        impl_ids = parent_inner.get("impls", [])
        for variant_id in parent_inner.get("variants", []):
            variant = index[str(variant_id)]
            variant_name = variant.get("name")
            if not variant_name:
                continue
            _, variant_inner = _item_inner(variant)
            variant_kind = variant_inner["kind"]
            if variant_kind == "plain":
                variant_field_ids = []
            elif "tuple" in variant_kind:
                variant_field_ids = [
                    field_id for field_id in variant_kind["tuple"] if field_id is not None
                ]
            elif "struct" in variant_kind:
                variant_field_ids = variant_kind["struct"].get("fields", [])
            else:
                variant_field_ids = []
            for position, field_id in enumerate(variant_field_ids):
                field = index[str(field_id)]
                field_kind, field_inner = _item_inner(field)
                field_name = field.get("name") or str(position)
                record = _base_record(
                    package,
                    f"{parent_id}::{variant_name}::{field_name}",
                    field_kind,
                    field,
                )
                record["owner"] = f"{parent_id}::{variant_name}"
                record["type"] = _normalized(field_inner, paths)
                records.append(record)
    elif parent_kind == "trait":
        field_ids = []
        impl_ids = []
        for associated_id in parent_inner.get("items", []):
            associated = index[str(associated_id)]
            name = associated.get("name")
            if name:
                records.extend(
                    _associated_record(package, parent_id, associated, paths, "trait_item")
                )
    else:
        return records

    for field_id in field_ids:
        field = index[str(field_id)]
        if field.get("visibility") != "public" or not field.get("name"):
            continue
        field_kind, field_inner = _item_inner(field)
        record = _base_record(
            package,
            f"{parent_id}::{field['name']}",
            field_kind,
            field,
        )
        record["owner"] = parent_id
        record["type"] = _normalized(field_inner, paths)
        records.append(record)

    for impl_id in impl_ids:
        impl_item = index[str(impl_id)]
        _, impl_inner = _item_inner(impl_item)
        if impl_inner.get("trait") is not None or impl_inner.get("blanket_impl") is not None:
            continue
        for associated_id in impl_inner.get("items", []):
            associated = index[str(associated_id)]
            if associated.get("visibility") != "public" or not associated.get("name"):
                continue
            records.extend(_associated_record(package, parent_id, associated, paths, "method"))
    return records


def _associated_record(
    package: str,
    parent_id: str,
    item: Mapping[str, Any],
    paths: Mapping[str, Any],
    default_kind: str,
) -> list[dict[str, Any]]:
    item_kind, inner = _item_inner(item)
    symbol_id = f"{parent_id}::{item['name']}"
    if item_kind == "function":
        kind = "constructor" if item["name"] == "new" else default_kind
        return _callable_records(package, symbol_id, kind, item, inner, paths)
    record = _base_record(package, symbol_id, item_kind, item)
    record["owner"] = parent_id
    record["type"] = _normalized(inner, paths)
    return [record]


def inventory_from_rustdoc(package: str, document: Mapping[str, Any]) -> dict[str, Any]:
    paths = document["paths"]
    index = document["index"]
    records: list[dict[str, Any]] = []
    public_parents: list[tuple[str, Mapping[str, Any]]] = []
    for rustdoc_id, path in paths.items():
        if path["crate_id"] != 0 or path["kind"] not in PUBLIC_PATH_KINDS:
            continue
        item = index[str(rustdoc_id)]
        symbol_id = _canonical_path(path["path"])
        item_kind, inner = _item_inner(item)
        if item_kind == "function":
            records.extend(
                _callable_records(package, symbol_id, "function", item, inner, paths)
            )
        else:
            records.append(_base_record(package, symbol_id, path["kind"], item))
        if item_kind in {"struct", "union", "enum", "trait"}:
            public_parents.append((symbol_id, item))

    for parent_id, parent in public_parents:
        records.extend(
            _public_associated_items(package, parent_id, parent, index, paths)
        )

    unique = {record["id"]: record for record in records}
    ordered = [unique[key] for key in sorted(unique)]
    target_family = document["target"]["triple"].split("-", 1)[1]
    digest = hashlib.sha256(
        json.dumps(
            {
                "crate_version": document.get("crate_version"),
                "rustdoc_format_version": document["format_version"],
                "target_family": target_family,
                "symbols": ordered,
            },
            sort_keys=True,
            separators=(",", ":"),
        ).encode("utf-8")
    ).hexdigest()
    return {
        "package": package,
        "crate_version": document.get("crate_version"),
        "rustdoc_format_version": document["format_version"],
        "target_family": target_family,
        "symbol_count": len(ordered),
        "digest": digest,
        "symbols": ordered,
    }


def build_rustdoc(package: str, toolchain: str) -> None:
    subprocess.run(
        [
            "cargo",
            f"+{toolchain}",
            "rustdoc",
            "-p",
            package,
            "--lib",
            "--all-features",
            "--",
            "-Z",
            "unstable-options",
            "--output-format",
            "json",
        ],
        cwd=ROOT,
        check=True,
    )


def generate(
    packages: Iterable[str], rustdoc_dir: pathlib.Path, toolchain: str
) -> dict[str, Any]:
    crates = []
    for package in packages:
        file_name = RUSTDOC_FILE_NAMES.get(package, package.replace("-", "_") + ".json")
        with (rustdoc_dir / file_name).open("rb") as stream:
            crates.append(inventory_from_rustdoc(package, json.load(stream)))
    digest = hashlib.sha256(
        "".join(crate["digest"] for crate in crates).encode("ascii")
    ).hexdigest()
    return {
        "schema_version": 1,
        "generator": "scripts/generate_python_rustdoc_inventory.py",
        "rustdoc_toolchain": toolchain,
        "packages": list(packages),
        "digest": digest,
        "crates": crates,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--build", action="store_true")
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--toolchain", default=DEFAULT_TOOLCHAIN)
    parser.add_argument("--rustdoc-dir", type=pathlib.Path, default=ROOT / "target/doc")
    parser.add_argument("--package", action="append", dest="packages")
    args = parser.parse_args()
    packages = tuple(args.packages or DEFAULT_PACKAGES)
    if args.build:
        for package in packages:
            build_rustdoc(package, args.toolchain)
    generated = generate(packages, args.rustdoc_dir, args.toolchain)
    rendered = json.dumps(generated, indent=2, sort_keys=True) + "\n"
    if args.check:
        if not OUTPUT.is_file() or OUTPUT.read_text() != rendered:
            print(
                "python rustdoc inventory is stale; run "
                "scripts/generate_python_rustdoc_inventory.py --build",
            )
            return 1
        print(
            "Python rustdoc inventory is fresh: "
            + ", ".join(
                f"{crate['package']}={crate['symbol_count']}" for crate in generated["crates"]
            )
        )
        return 0
    OUTPUT.write_text(rendered)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
