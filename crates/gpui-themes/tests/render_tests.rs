use gpui::TestAppContext;
use gpui_themes::{ComponentShowcase, EditorTheme, ThemeEditor};
use std::sync::Arc;

#[gpui::test]
async fn test_component_showcase_renders(cx: &mut TestAppContext) {
    let theme = Arc::new(EditorTheme::dark());
    let _window = cx.add_window(|_window, _cx| ComponentShowcase::new(theme));
}

#[gpui::test]
async fn test_theme_editor_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, cx| ThemeEditor::new(cx));
}
