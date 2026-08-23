//! Design-system helpers shared by UI-kit components.

use std::sync::{Arc, OnceLock};

use gpui::App;
use gpui_design::{DesignExt, DesignSystem};

/// Resolve an explicit design override or fall back to the app-global design.
pub fn resolve_design(explicit: Option<Arc<DesignSystem>>, cx: &mut App) -> Arc<DesignSystem> {
    explicit.unwrap_or_else(|| cx.design())
}

/// Neutral design fallback for direct `build()` calls outside a GPUI context.
pub fn neutral_design() -> Arc<DesignSystem> {
    static NEUTRAL_DESIGN: OnceLock<Arc<DesignSystem>> = OnceLock::new();
    Arc::clone(NEUTRAL_DESIGN.get_or_init(|| Arc::new(DesignSystem::neutral())))
}

/// Platform-default design fallback for helpers that are not tied to an app.
pub fn platform_design() -> Arc<DesignSystem> {
    static PLATFORM_DESIGN: OnceLock<Arc<DesignSystem>> = OnceLock::new();
    Arc::clone(PLATFORM_DESIGN.get_or_init(|| Arc::new(DesignSystem::platform_default())))
}
