//! MiniApp - A minimal application template for GPUI examples and showcases
//!
//! Provides a reusable application shell with:
//! - Standard menu bar with Quit option (Cmd+Q on macOS)
//! - Theme variant switching with Cmd+T
//! - Language switching menu
//! - Configurable window title and size
//! - Builder-backed content layout and design-system defaults
//! - Extensible for additional default features
//!
//! # Example
//!
//! ```ignore
//! use gpui::*;
//! use gpui_miniapp::{MiniApp, MiniAppConfig};
//!
//! struct MyDemo;
//!
//! impl Render for MyDemo {
//!     fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
//!         div().child("Hello from MiniApp!")
//!     }
//! }
//!
//! fn main() {
//!     MiniApp::run(MiniAppConfig::new("My Demo"), |cx| cx.new(|_| MyDemo));
//! }
//! ```

use gpui::*;

actions!(
    miniapp,
    [
        Quit,
        ToggleTheme,
        SetLanguageEnglish,
        SetLanguageFrench,
        SetLanguageGerman,
        SetLanguageSpanish,
        SetLanguageJapanese,
    ]
);

mod mini_app;
mod mini_app_config;
mod mini_app_shell;
mod mini_app_state;
mod misc;
#[cfg(test)]
mod tests;

pub use mini_app::*;
pub use mini_app_config::*;
pub use mini_app_state::{
    MiniAppState, language_from_code, load_miniapp_state, save_miniapp_state, theme_from_name,
};
pub use misc::*;
