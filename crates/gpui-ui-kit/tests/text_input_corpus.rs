use gpui_ui_kit::DynamicTypePolicy;
use gpui_ui_kit::input::edit_state::EditState;
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Deserialize)]
struct Corpus {
    schema_version: u32,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    id: String,
    text: String,
    direction: String,
    ime_commit: String,
    dynamic_type_scale: f32,
}

#[test]
fn shared_corpus_preserves_ime_commits_selection_and_dynamic_type() {
    let corpus: Corpus =
        serde_json::from_str(include_str!("../../../qa/corpora/text-input-v1.json")).unwrap();
    assert_eq!(corpus.schema_version, 1);
    let mut ids = BTreeSet::new();

    for case in corpus.cases {
        assert!(ids.insert(case.id.clone()));
        assert!(["ltr", "rtl", "mixed"].contains(&case.direction.as_str()));

        let mut edit = EditState::new(&case.text);
        edit.insert_text(&case.ime_commit); // replaces the default select-all
        edit.select_all();
        assert_eq!(
            edit.get_selected_text().as_deref(),
            Some(case.ime_commit.as_str())
        );

        edit.move_to_end();
        edit.move_backward();
        edit.extend_forward();
        assert!(
            edit.has_selection(),
            "{} must select one Unicode scalar",
            case.id
        );

        let policy = DynamicTypePolicy {
            scale_factor: case.dynamic_type_scale,
            min_size: 12.0,
            max_size: 40.0,
        };
        let resolved = policy.resolve(16.0);
        assert!((12.0..=40.0).contains(&resolved));
    }
}
