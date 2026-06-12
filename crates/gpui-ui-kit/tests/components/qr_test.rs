//! QrCode component tests

use gpui_ui_kit::qr::QrCode;

#[test]
fn test_qr_code_creation() {
    let qr = QrCode::new("https://example.com");
    let _ = qr;
}

#[test]
fn test_qr_code_custom_size() {
    let qr = QrCode::new("hello").size(gpui::px(300.0));
    let _ = qr;
}

#[test]
fn test_qr_code_custom_colors() {
    let qr = QrCode::new("test")
        .fg(gpui::rgba(0x000000ff))
        .bg(gpui::rgba(0xffffffff));
    let _ = qr;
}

#[test]
fn test_qr_code_empty_string() {
    let qr = QrCode::new("");
    let _ = qr;
}

#[test]
fn test_qr_code_full_configuration() {
    let qr = QrCode::new("https://example.com/long/url/path")
        .size(gpui::px(400.0))
        .fg(gpui::rgba(0x333333ff))
        .bg(gpui::rgba(0xeeeeeeff));
    let _ = qr;
}
