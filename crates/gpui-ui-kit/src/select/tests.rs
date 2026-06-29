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
