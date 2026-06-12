//! ImageView component tests

use gpui_ui_kit::image_view::{ImageFit, ImageView};

#[test]
fn test_image_view_creation() {
    let view = ImageView::new("test");
    drop(view);
}

#[test]
fn test_image_view_src() {
    let view = ImageView::new("test").src("path/to/image.png");
    drop(view);
}

#[test]
fn test_image_view_alt() {
    let view = ImageView::new("test").alt("Album artwork");
    drop(view);
}

#[test]
fn test_image_view_width() {
    let view = ImageView::new("test").width(gpui::px(200.0));
    drop(view);
}

#[test]
fn test_image_view_height() {
    let view = ImageView::new("test").height(gpui::px(150.0));
    drop(view);
}

#[test]
fn test_image_view_size() {
    let view = ImageView::new("test").size(gpui::px(100.0));
    drop(view);
}

#[test]
fn test_image_view_fit_variants() {
    for fit in [ImageFit::Cover, ImageFit::Contain, ImageFit::Fill] {
        let view = ImageView::new("test").fit(fit);
        drop(view);
    }
}

#[test]
fn test_image_view_rounded() {
    let view = ImageView::new("test").rounded(gpui::px(8.0));
    drop(view);
}

#[test]
fn test_image_view_show_border() {
    let view = ImageView::new("test").show_border(true);
    drop(view);

    let view = ImageView::new("test").show_border(false);
    drop(view);
}

#[test]
fn test_image_view_placeholder_icon() {
    let view = ImageView::new("test").placeholder_icon("📷");
    drop(view);
}

#[test]
fn test_image_view_on_click() {
    let view = ImageView::new("test").on_click(|_window, _cx| {});
    drop(view);
}

#[test]
fn test_image_view_full_configuration() {
    let view = ImageView::new("album-art")
        .src("path/to/cover.jpg")
        .alt("Album cover")
        .width(gpui::px(300.0))
        .height(gpui::px(300.0))
        .fit(ImageFit::Cover)
        .rounded(gpui::px(8.0))
        .show_border(true)
        .placeholder_icon("🎵")
        .on_click(|_window, _cx| {});
    drop(view);
}
