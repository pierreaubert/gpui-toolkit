//! Tests for the Python host's committed application state.

// Rust guideline compliant 2026-10-09

use super::python_table_rows::TableRows;
use super::{PresentationStore, PythonAppIr, PythonIrShowcase};
use gpui_python_runtime::session::{Patch, PatchOp};
use gpui_python_runtime::ui_ir::UiNode;
use serde_json::json;
use std::rc::Rc;

fn first_table_rows(showcase: &mut PythonIrShowcase) -> Rc<TableRows> {
    let app = showcase.app.take().unwrap();
    let UiNode::Table(node) = &app.sections[0].content else {
        panic!("fixture must contain a table");
    };
    let rows = showcase.retained_table_rows(node, 0, node.rows.len().max(node.typed_rows.len()));
    showcase.app = Some(app);
    rows
}

#[::core::prelude::v1::test]
fn json_snapshot_tracks_committed_apps_and_rejected_patches() {
    let app: PythonAppIr = serde_json::from_value(json!({
        "title": "Test",
        "sections": [{
            "id": "main",
            "label": "Main",
            "content": {"kind": "table", "id": "table", "rows": [["initial"]]}
        }]
    }))
    .unwrap();
    let mut showcase = PythonIrShowcase::new_empty(PresentationStore::open());
    showcase.current_section = "main".into();
    showcase.apply_snapshot_message(app);
    assert_eq!(
        showcase.app_value,
        showcase
            .app
            .as_ref()
            .map(|app| serde_json::to_value(app).unwrap())
    );

    showcase.apply_patch_message(Patch {
        revision: 1,
        request_id: None,
        ops: vec![PatchOp::Replace {
            id: "table".into(),
            node: json!({"kind": "table", "id": "table", "rows": [["updated"]]}),
        }],
    });
    assert_eq!(showcase.session_state.revision(), 1);
    let committed = showcase.app_value.clone();
    assert_eq!(
        committed,
        showcase
            .app
            .as_ref()
            .map(|app| serde_json::to_value(app).unwrap())
    );
    assert_eq!(
        committed.as_ref().unwrap()["sections"][0]["content"]["rows"][0][0],
        "updated"
    );

    showcase.apply_patch_message(Patch {
        revision: 2,
        request_id: None,
        ops: vec![PatchOp::Set {
            id: "table".into(),
            property: "unknown_property".into(),
            value: json!(true),
        }],
    });
    assert_eq!(showcase.session_state.revision(), 1);
    assert_eq!(showcase.app_value, committed);
    assert_eq!(
        showcase.app_value,
        showcase
            .app
            .as_ref()
            .map(|app| serde_json::to_value(app).unwrap())
    );
}

#[::core::prelude::v1::test]
fn table_rows_follow_successful_patches_and_snapshots() {
    let app: PythonAppIr = serde_json::from_value(json!({
        "title": "Test",
        "sections": [{
            "id": "main",
            "label": "Main",
            "content": {"kind": "table", "id": "table", "rows": [["initial"]]}
        }]
    }))
    .unwrap();
    let mut showcase = PythonIrShowcase::new_empty(PresentationStore::open());
    showcase.current_section = "main".into();
    showcase.apply_snapshot_message(app.clone());
    let first = first_table_rows(&mut showcase);
    let next_frame = first_table_rows(&mut showcase);
    assert!(Rc::ptr_eq(&first, &next_frame));
    assert_eq!(
        first.row(0),
        Some(("row-0".into(), vec![json!("initial")], false))
    );

    showcase.apply_patch_message(Patch {
        revision: 1,
        request_id: None,
        ops: vec![PatchOp::Set {
            id: "table".into(),
            property: "rows".into(),
            value: json!([["patched"]]),
        }],
    });
    let patched = first_table_rows(&mut showcase);
    assert_eq!(
        patched.row(0),
        Some(("row-0".into(), vec![json!("patched")], false))
    );
    assert_eq!(
        first.row(0),
        Some(("row-0".into(), vec![json!("initial")], false))
    );

    showcase.apply_snapshot_message(app);
    let replaced = first_table_rows(&mut showcase);
    assert_eq!(
        replaced.row(0),
        Some(("row-0".into(), vec![json!("initial")], false))
    );
}

#[::core::prelude::v1::test]
fn selected_typed_row_changes_after_a_patch() {
    let app: PythonAppIr = serde_json::from_value(json!({
        "title": "Test",
        "sections": [{
            "id": "main",
            "label": "Main",
            "content": {
                "kind": "table", "id": "table", "selected_row": "a",
                "typed_rows": [
                    {"id": "a", "cells": ["first"]},
                    {"id": "b", "cells": ["second"]}
                ]
            }
        }]
    }))
    .unwrap();
    let mut showcase = PythonIrShowcase::new_empty(PresentationStore::open());
    showcase.current_section = "main".into();
    showcase.apply_snapshot_message(app);
    let initial = first_table_rows(&mut showcase);
    assert_eq!(initial.selected_index(), Some(0));
    assert!(initial.row(0).unwrap().2);

    showcase.apply_patch_message(Patch {
        revision: 1,
        request_id: None,
        ops: vec![PatchOp::Set {
            id: "table".into(),
            property: "selected_row".into(),
            value: json!("b"),
        }],
    });
    let updated = first_table_rows(&mut showcase);
    assert_eq!(updated.selected_index(), Some(1));
    assert!(!updated.row(0).unwrap().2);
    assert!(updated.row(1).unwrap().2);
}
