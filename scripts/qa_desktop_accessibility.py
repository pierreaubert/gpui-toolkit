#!/usr/bin/env python3
"""Emit deterministic desktop interaction/accessibility release evidence."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


SCHEMA_VERSION = 1
REPORT_TYPE = "gpui-toolkit-desktop-accessibility-evidence"
REVIEWED_ON = "2026-08-07"


CHECKS = (
    {
        "id": "pointer-activation",
        "dimension": "Pointer activation",
        "status": "component-tested",
        "evidence": [
            ["crates/gpui-ui-kit/tests/integration/button_test.rs", "test_button_click"],
            ["crates/gpui-ui-kit/tests/integration/slider_test/test.rs", "test_slider_disabled_ignores_clicks"],
        ],
    },
    {
        "id": "keyboard-activation-navigation",
        "dimension": "Keyboard activation and navigation",
        "status": "component-tested",
        "evidence": [
            ["crates/gpui-ui-kit/tests/components/button_test.rs", "test_button_keyboard_accessible"],
            ["crates/gpui-ui-kit/tests/components/select_test.rs", "test_select_complete_keyboard_support"],
            ["crates/gpui-ui-kit/src/focus.rs", "focus_group_maps_keys_by_direction"],
        ],
    },
    {
        "id": "focus-order-restoration",
        "dimension": "Focus order and restoration",
        "status": "component-tested",
        "evidence": [
            ["crates/gpui-ui-kit/src/focus.rs", "focus_group_computes_navigation_targets"],
            ["crates/gpui-ui-kit/src/dialog.rs", "dialog_builder_records_keyboard_focus_contract"],
            ["crates/gpui-ui-kit/src/popover.rs", "popover_builder_records_keyboard_dismiss_contract"],
        ],
    },
    {
        "id": "disabled-state",
        "dimension": "Disabled-state input suppression",
        "status": "component-tested",
        "evidence": [
            ["crates/gpui-ui-kit/tests/integration/button_test.rs", "test_button_disabled_ignores_click"],
            ["crates/gpui-ui-kit/tests/integration/input_test/test.rs", "test_input_disabled_no_input"],
            ["crates/gpui-ui-kit/tests/integration/toggle_test.rs", "test_toggle_disabled_ignores_click"],
        ],
    },
    {
        "id": "accessible-names-actions",
        "dimension": "Accessible names, roles, values, and actions",
        "status": "component-tested",
        "evidence": [
            ["crates/gpui-ui-kit/src/accessibility.rs", "bridge_snapshot_reports_missing_accessible_names"],
            ["crates/gpui-ui-kit/src/accessibility.rs", "bridge_snapshot_exports_native_adapter_payload"],
            ["crates/gpui-ui-kit/src/accessibility.rs", "ui_kit_roles_map_to_native_accesskit_roles"],
        ],
    },
    {
        "id": "native-adapter-parity",
        "dimension": "Native adapter payload parity",
        "status": "component-tested",
        "evidence": [
            ["crates/gpui-ui-kit/src/accessibility.rs", "native_targets_preserve_tree_order_states_values_and_action_parity"],
            ["crates/gpui-ui-kit/src/accessibility.rs", "native_adapter_payload_rejects_bad_release_inputs"],
        ],
    },
    {
        "id": "reduced-motion",
        "dimension": "Reduced-motion conformance",
        "status": "renderer-conformance-tested",
        "evidence": [
            ["crates/gpui-component-lab/src/lib/validate.rs", "stories must include a reduced-motion preset"],
            ["crates/gpui-component-lab/src/lib/default.rs", "reduced-motion"],
        ],
    },
    {
        "id": "high-contrast",
        "dimension": "High-contrast rendered coverage",
        "status": "renderer-conformance-tested",
        "evidence": [
            ["crates/gpui-ui-kit/src/visual_regression.rs", "high_contrast"],
            ["qa/visual/baselines/README.md", "Metal"],
        ],
    },
)


def validate_contracts(root: Path) -> None:
    errors: list[str] = []
    for check in CHECKS:
        for relative, marker in check["evidence"]:
            path = root / relative
            if not path.is_file():
                errors.append(f"{check['id']}: missing {relative}")
                continue
            if marker not in path.read_text(encoding="utf-8"):
                errors.append(f"{check['id']}: {relative} lacks marker {marker!r}")
    if errors:
        raise SystemExit("desktop accessibility evidence is stale:\n- " + "\n- ".join(errors))


def report() -> dict[str, object]:
    return {
        "schema_version": SCHEMA_VERSION,
        "report_type": REPORT_TYPE,
        "reviewed_on": REVIEWED_ON,
        "scope": "portable desktop component and renderer contracts",
        "automated_release_ready": True,
        "native_screen_reader_qa": "manual-required",
        "commands": [
            "cargo test -p gpui-ui-kit",
            "cargo test -p gpui-component-lab",
            "just qa-visual",
        ],
        "checks": CHECKS,
        "limitations": [
            "This report does not claim VoiceOver, Narrator, Orca/AT-SPI, or TalkBack execution.",
            "Workflow-canvas traversal and mobile gesture accessibility remain platform QA gates.",
        ],
    }


def markdown(body: dict[str, object]) -> str:
    lines = [
        "# Desktop interaction and accessibility evidence",
        "",
        f"- schema_version: {body['schema_version']}",
        f"- report_type: `{body['report_type']}`",
        f"- reviewed_on: {body['reviewed_on']}",
        f"- native_screen_reader_qa: `{body['native_screen_reader_qa']}`",
        "",
        "| Dimension | Status | Evidence contracts |",
        "| --- | --- | --- |",
    ]
    for check in CHECKS:
        evidence = ", ".join(f"`{path}` (`{marker}`)" for path, marker in check["evidence"])
        lines.append(f"| {check['dimension']} | {check['status']} | {evidence} |")
    lines.extend(["", "## Limitations", ""])
    lines.extend(f"- {item}" for item in body["limitations"])
    return "\n".join(lines) + "\n"


def write(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(content, encoding="utf-8")
    temporary.replace(path)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-json", type=Path, required=True)
    parser.add_argument("--output-markdown", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    validate_contracts(root)
    body = report()
    write(args.output_json, json.dumps(body, indent=2, sort_keys=True) + "\n")
    write(args.output_markdown, markdown(body))


if __name__ == "__main__":
    main()
