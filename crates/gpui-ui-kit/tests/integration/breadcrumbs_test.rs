//! Integration test for Breadcrumbs component

use gpui::{Context, TestAppContext, Window, div, prelude::*};
use gpui_ui_kit::breadcrumbs::{BreadcrumbItem, Breadcrumbs};

struct BreadcrumbsTestView;

impl Render for BreadcrumbsTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Breadcrumbs::new().items(vec![
            BreadcrumbItem::new("home", "Home"),
            BreadcrumbItem::new("docs", "Docs"),
            BreadcrumbItem::new("api", "API"),
        ]))
    }
}

#[gpui::test]
async fn test_breadcrumbs_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| BreadcrumbsTestView);
}
