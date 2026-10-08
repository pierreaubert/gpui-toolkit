//! Positive `layout!` expansion tests against the real crates.

use gpui_ui_kit_macros::layout;

#[test]
fn layout_builds_nested_elements() {
    let _element = layout! {
        V {
            Tx "Dashboard",
            H {
                I #name [label = "Name"],
                B.primary("Go") #go,
            },
        }
    };
}

#[test]
fn layout_matches_tutorial_snippet() {
    let _hero = layout! {
        V {
            Tx "Hi",
            B.primary("Go"),
        }
    };
}

#[test]
fn layout_builds_divs_repeats_and_variants() {
    let _element = layout! {
        D {
            Tx "a",
            B.secondary("b") #b,
        }
    };
    let _element = layout! {
        V {
            B "x" * 2,
        }
    };
}
