//! Thinking Orb Debug Example
//!
//! Shows all nine `OrbState`s animating in a 3×3 grid at 96 px each, with the
//! per-state accessibility label under each orb.

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::Text;
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct ThinkingOrbDebug {
    orbs: Vec<(OrbState, Entity<ThinkingOrb>)>,
}

impl ThinkingOrbDebug {
    fn new(cx: &mut Context<Self>) -> Self {
        let orbs = OrbState::ALL
            .iter()
            .map(|&state| (state, cx.new(|cx| ThinkingOrb::new(state, px(96.0), cx))))
            .collect();
        Self { orbs }
    }
}

impl Render for ThinkingOrbDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("thinking-orb-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("Thinking Orb Debug"))
            .child(
                div()
                    .grid()
                    .grid_cols(3)
                    .gap_6()
                    .children(self.orbs.iter().map(|(state, orb)| {
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_2()
                            .child(orb.clone())
                            .child(Text::new(state.label()))
                    })),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Thinking Orb Debug")
            .size(560.0, 560.0)
            .with_theme(true),
        |cx| cx.new(ThinkingOrbDebug::new),
    );
}
