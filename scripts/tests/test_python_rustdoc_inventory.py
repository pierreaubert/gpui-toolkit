from __future__ import annotations

import importlib.util
import pathlib
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "generate_python_rustdoc_inventory",
    ROOT / "scripts/generate_python_rustdoc_inventory.py",
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)
CHECK_SPEC = importlib.util.spec_from_file_location(
    "check_python_v2_surface",
    ROOT / "scripts/check_python_v2_surface.py",
)
assert CHECK_SPEC is not None and CHECK_SPEC.loader is not None
CHECK = importlib.util.module_from_spec(CHECK_SPEC)
CHECK_SPEC.loader.exec_module(CHECK)


def item(id_, name, visibility, inner, *, line=1):
    return {
        "id": id_,
        "crate_id": 0,
        "name": name,
        "span": {
            "filename": "src/lib.rs",
            "begin": [line, 1],
            "end": [line, 2],
        },
        "visibility": visibility,
        "docs": None,
        "links": {},
        "attrs": [],
        "deprecation": None,
        "stability": None,
        "const_stability": None,
        "inner": inner,
    }


def function(inputs, output):
    return {
        "function": {
            "sig": {"inputs": inputs, "output": output, "is_c_variadic": False},
            "generics": {"params": [], "where_predicates": []},
            "header": {"is_async": False, "is_unsafe": False},
            "has_body": True,
        }
    }


class RustdocInventoryTests(unittest.TestCase):
    def test_extracts_public_items_fields_methods_parameters_and_returns(self):
        document = {
            "crate_version": "1.2.3",
            "format_version": 61,
            "target": {"triple": "aarch64-apple-darwin"},
            "paths": {
                "1": {"crate_id": 0, "path": ["demo", "Thing"], "kind": "struct"},
                "7": {"crate_id": 0, "path": ["demo", "make"], "kind": "function"},
                "99": {"crate_id": 1, "path": ["core", "primitive", "u32"], "kind": "primitive"},
            },
            "index": {
                "1": item(
                    1,
                    "Thing",
                    "public",
                    {
                        "struct": {
                            "kind": {"plain": {"fields": ["2"], "has_stripped_fields": False}},
                            "generics": {"params": [], "where_predicates": []},
                            "impls": ["3"],
                        }
                    },
                ),
                "2": item(2, "value", "public", {"struct_field": {"primitive": "u32"}}, line=2),
                "3": item(
                    3,
                    None,
                    "default",
                    {
                        "impl": {
                            "trait": None,
                            "blanket_impl": None,
                            "items": ["4", "6"],
                        }
                    },
                ),
                "4": item(
                    4,
                    "new",
                    "public",
                    function([["value", {"primitive": "u32"}]], {"generic": "Self"}),
                    line=4,
                ),
                "6": item(6, "hidden", "private", function([], None), line=6),
                "7": item(
                    7,
                    "make",
                    "public",
                    function([["count", {"primitive": "u32"}]], {"resolved_path": {"id": 1, "args": None}}),
                    line=7,
                ),
            },
        }

        inventory = MODULE.inventory_from_rustdoc("demo-package", document)
        symbols = {symbol["id"]: symbol for symbol in inventory["symbols"]}
        self.assertIn("demo::Thing", symbols)
        self.assertIn("demo::Thing::value", symbols)
        self.assertEqual(symbols["demo::Thing::new"]["kind"], "constructor")
        self.assertIn("demo::Thing::new#parameter:0:value", symbols)
        self.assertIn("demo::Thing::new#return", symbols)
        self.assertNotIn("demo::Thing::hidden", symbols)
        self.assertEqual(
            symbols["demo::make"]["signature"]["return"],
            {"resolved_path": {"path": "demo::Thing", "args": None}},
        )
        self.assertEqual(inventory["symbol_count"], len(symbols))

    def test_output_is_deterministic_when_rustdoc_maps_are_reordered(self):
        document = {
            "crate_version": None,
            "format_version": 61,
            "target": {"triple": "aarch64-apple-darwin"},
            "paths": {
                "2": {"crate_id": 0, "path": ["demo", "z"], "kind": "function"},
                "1": {"crate_id": 0, "path": ["demo", "a"], "kind": "function"},
            },
            "index": {
                "1": item(1, "a", "public", function([], None)),
                "2": item(2, "z", "public", function([], None)),
            },
        }
        first = MODULE.inventory_from_rustdoc("demo", document)
        document["paths"] = dict(reversed(list(document["paths"].items())))
        second = MODULE.inventory_from_rustdoc("demo", document)
        self.assertEqual(first, second)
        document["target"]["triple"] = "x86_64-unknown-linux-gnu"
        linux = MODULE.inventory_from_rustdoc("demo", document)
        self.assertNotEqual(first["digest"], linux["digest"])

    def test_only_reviewed_exclusions_can_classify_by_prefix(self):
        inventory = {
            "crates": [
                {
                    "symbols": [
                        {"package": "demo", "id": "demo::examples", "kind": "module"},
                        {"package": "demo", "id": "demo::examples::run", "kind": "function"},
                        {"package": "demo", "id": "demo::scale", "kind": "function"},
                    ]
                }
            ]
        }
        data = {
            "rustdoc_classification_rule": [
                {
                    "id": "examples",
                    "package": "demo",
                    "path_prefix": "demo::examples",
                    "disposition": "intentionally-excluded",
                    "rationale": "Example application API",
                }
            ]
        }
        errors, classified, unclassified = CHECK.rustdoc_classification_audit(
            data, inventory
        )
        self.assertEqual(errors, [])
        self.assertEqual(classified, 2)
        self.assertEqual(unclassified, 1)

        data["rustdoc_classification_rule"][0]["disposition"] = "direct"
        errors, _, _ = CHECK.rustdoc_classification_audit(data, inventory)
        self.assertTrue(any("only reviewed exclusions" in error for error in errors))

        data["rustdoc_classification_rule"][0]["disposition"] = "intentionally-excluded"
        data["rustdoc_completion_scope"] = [
            {
                "id": "all-functions",
                "package": "demo",
                "path_prefix": "demo",
                "kinds": ["function"],
            }
        ]
        errors, _, _ = CHECK.rustdoc_classification_audit(data, inventory)
        self.assertTrue(any("has 1 unclassified symbols" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
