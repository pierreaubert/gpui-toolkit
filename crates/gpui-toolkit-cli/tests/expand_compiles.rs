//! Proves golden expansions compile against the real crates.
//!
//! `layout_expand_matches_fixture` pins `expand` output byte-for-byte to
//! the fixtures; including each fixture in its own module here proves
//! that exact output type-checks and constructs.

mod dashboard {
    include!("fixtures/expanded_fixture_dashboard.rs");
}

mod styled {
    include!("fixtures/expanded_fixture_styled.rs");
}

#[test]
fn expanded_fixture_constructs() {
    let _element = dashboard::fixture_dashboard();
}

#[test]
fn expanded_styled_fixture_constructs() {
    let _element = styled::fixture_styled();
}
