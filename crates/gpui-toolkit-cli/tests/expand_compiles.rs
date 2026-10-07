//! Proves golden expansions compile against the real crates.
//!
//! `layout_expand_matches_fixture` pins `expand` output byte-for-byte to
//! the fixture; including the fixture here proves that exact output
//! type-checks and constructs.

include!("fixtures/expanded_fixture_dashboard.rs");

#[test]
fn expanded_fixture_constructs() {
    let _element = fixture_dashboard();
}
