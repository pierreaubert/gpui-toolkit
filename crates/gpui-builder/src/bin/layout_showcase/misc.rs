use gpui::prelude::FluentBuilder as _;
use gpui::{FontWeight, IntoElement, ParentElement, Rgba, SharedString, Styled, div, px};
use gpui_builder::{Axis, DisplayTier, SolvedNodeRef};
use std::rc::Rc;

pub(super) static INSPECTOR_TIERS: &[DisplayTier<'_>] = &[
    DisplayTier {
        name: "Full",
        min_size: 200.0,
    },
    DisplayTier {
        name: "Mini",
        min_size: 80.0,
    },
];

pub(super) fn muted(color: Rgba, alpha: f32) -> Rgba {
    Rgba {
        r: color.r,
        g: color.g,
        b: color.b,
        a: alpha,
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "showcase rendering helper keeps visual styling inputs explicit at call sites"
)]
pub(super) fn panel_box(
    label: &str,
    size_info: &str,
    bg: Rgba,
    fg: Rgba,
    selected: bool,
    accent: Rgba,
    base_size: f32,
    small_size: f32,
    gap: f32,
) -> impl IntoElement {
    div()
        .size_full()
        .min_w_0()
        .min_h_0()
        .bg(bg)
        .when(selected, |d| d.border_1().border_color(muted(accent, 0.8)))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(gap))
        .child(
            div()
                .text_size(px(base_size))
                .font_weight(FontWeight::BOLD)
                .text_color(fg)
                .child(SharedString::from(label.to_string())),
        )
        .child(
            div()
                .text_size(px(small_size))
                .text_color(muted(fg, 0.5))
                .child(SharedString::from(size_info.to_string())),
        )
}

pub(super) fn size_label(node: &SolvedNodeRef<'_, '_>) -> String {
    let tier = node
        .active_tier()
        .map(|t| format!(" [{t}]"))
        .unwrap_or_default();
    format!("{:.0} x {:.0}{tier}", node.width(), node.height())
}

pub(super) fn axis_text(axis: Option<Axis>) -> &'static str {
    match axis {
        Some(Axis::Horizontal) => "H",
        Some(Axis::Vertical) => "V",
        None => "-",
    }
}

pub(super) fn current_platform() -> Result<Rc<dyn gpui::Platform>, String> {
    #[cfg(target_os = "macos")]
    {
        Ok(Rc::new(gpui_macos::MacPlatform::new(false)))
    }
    #[cfg(target_os = "linux")]
    {
        Ok(gpui_linux::current_platform(false))
    }
    #[cfg(target_os = "windows")]
    {
        gpui_windows::WindowsPlatform::new(false)
            .map(|p| Rc::new(p) as Rc<dyn gpui::Platform>)
            .map_err(|e| format!("failed to create Windows platform: {e:?}"))
    }
    #[cfg(any(target_os = "ios", target_os = "tvos"))]
    {
        Ok(gpui_ios::current_platform(false))
    }
    #[cfg(not(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "windows",
        target_os = "ios",
        target_os = "tvos"
    )))]
    {
        compile_error!("unsupported platform for layout showcase")
    }
}
