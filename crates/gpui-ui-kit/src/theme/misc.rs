use gpui::{BoxShadow, Hsla, Rgba, point, px};

/// Create a glow shadow effect for hover states.
/// This is a shared helper to avoid duplicating shadow construction
/// across button, accordion, menu, tabs, and other components.
pub fn glow_shadow(color: Rgba) -> Vec<BoxShadow> {
    let glow_inner = Hsla::from(color).alpha(0.6);
    let glow_outer = Hsla::from(color).alpha(0.2);
    vec![
        BoxShadow {
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(4.0),
            spread_radius: px(0.0),
            color: glow_inner,
        },
        BoxShadow {
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(25.0),
            spread_radius: px(2.0),
            color: glow_outer,
        },
    ]
}
