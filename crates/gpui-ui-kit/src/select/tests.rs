use super::Select;
use super::select_option::SelectOption;

#[test]
fn test_select_open_with_backdrop_smoke() {
    let _select = Select::new("test-select")
        .placeholder("Choose")
        .options(vec![
            SelectOption::new("1", "Option 1"),
            SelectOption::new("2", "Option 2"),
        ])
        .is_open(true)
        .on_toggle(|_open, _window, _cx| {});
}

#[test]
fn dropdown_placement_flips_when_space_below_is_short() {
    // Trigger near the bottom of an 800px window, 200px dropdown:
    // 100px below, 700px above -> open upward with full height.
    let (up, cap) = super::dropdown_placement(800.0, 660.0, 700.0, 200.0);
    assert!(up);
    assert_eq!(cap, 200.0);
    // Plenty of room below: opens downward as before.
    let (up, cap) = super::dropdown_placement(800.0, 60.0, 100.0, 200.0);
    assert!(!up);
    assert_eq!(cap, 200.0);
    // Cramped both ways: picks the larger side and clamps height to
    // the available space (minus a small margin).
    let (up, cap) = super::dropdown_placement(300.0, 100.0, 140.0, 200.0);
    assert!(!up);
    assert_eq!(cap, 152.0);
    let (up, cap) = super::dropdown_placement(300.0, 200.0, 240.0, 200.0);
    assert!(up);
    assert_eq!(cap, 192.0);
}
