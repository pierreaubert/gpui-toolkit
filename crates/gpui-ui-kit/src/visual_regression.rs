//! Visual regression capture inventory for gpui-ui-kit release QA.

use std::collections::{BTreeSet, HashSet};

/// Schema version for [`UiKitVisualRegressionManifest`].
pub const UI_KIT_VISUAL_REGRESSION_SCHEMA_VERSION: u32 = 1;

/// Stable report type identifier for [`UiKitVisualRegressionManifest`].
pub const UI_KIT_VISUAL_REGRESSION_REPORT_TYPE: &str = "gpui-ui-kit-visual-regression-manifest";

/// Theme/color scheme expected for a UI-kit visual capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UiKitVisualColorScheme {
    Light,
    Dark,
    HighContrast,
}

impl UiKitVisualColorScheme {
    /// Stable label used in capture ids and artifact paths.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::HighContrast => "high_contrast",
        }
    }
}

/// Viewport preset expected for a UI-kit visual capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiKitVisualViewport {
    pub id: &'static str,
    pub label: &'static str,
    pub width: u32,
    pub height: u32,
    pub scale_factor: u32,
}

/// One UI-kit component story that should be captured by visual regression CI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiKitVisualStory {
    pub id: &'static str,
    pub label: &'static str,
    pub component: &'static str,
    pub component_family: &'static str,
    pub scenario: &'static str,
    pub release_focus: &'static str,
}

/// One deterministic screenshot capture expected by release QA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiKitVisualCapture {
    pub id: String,
    pub story_id: &'static str,
    pub story_label: &'static str,
    pub component: &'static str,
    pub component_family: &'static str,
    pub scenario: &'static str,
    pub viewport_id: &'static str,
    pub viewport_label: &'static str,
    pub width: u32,
    pub height: u32,
    pub scale_factor: u32,
    pub color_scheme: UiKitVisualColorScheme,
    pub release_focus: &'static str,
    pub baseline_path: String,
    pub actual_path: String,
    pub diff_path: String,
}

/// Versioned screenshot manifest for UI-kit visual regression tooling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiKitVisualRegressionManifest {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub crate_name: &'static str,
    pub crate_version: &'static str,
    pub stories: &'static [UiKitVisualStory],
    pub viewports: &'static [UiKitVisualViewport],
    pub color_schemes: &'static [UiKitVisualColorScheme],
    pub captures: Vec<UiKitVisualCapture>,
}

impl UiKitVisualRegressionManifest {
    /// Total number of generated captures.
    pub fn capture_count(&self) -> usize {
        self.captures.len()
    }

    /// Number of captures implied by stories, viewports, and color schemes.
    pub fn expected_capture_count(&self) -> usize {
        self.stories.len() * self.viewports.len() * self.color_schemes.len()
    }

    /// Return true when every generated capture id is unique.
    pub fn validate_unique_capture_ids(&self) -> bool {
        let mut ids = HashSet::new();
        self.captures
            .iter()
            .all(|capture| ids.insert(capture.id.as_str()))
    }

    /// Component names covered by the manifest.
    pub fn components(&self) -> BTreeSet<&'static str> {
        self.stories.iter().map(|story| story.component).collect()
    }

    /// Component families covered by the manifest.
    pub fn component_families(&self) -> BTreeSet<&'static str> {
        self.stories
            .iter()
            .map(|story| story.component_family)
            .collect()
    }

    /// Return generated captures for a component.
    pub fn captures_for_component(&self, component: &str) -> Vec<&UiKitVisualCapture> {
        self.captures
            .iter()
            .filter(|capture| capture.component == component)
            .collect()
    }

    /// Render the manifest as Markdown for release artifacts.
    pub fn to_markdown_table(&self) -> String {
        let mut output = String::from("# gpui-ui-kit Visual Regression Manifest\n\n");
        output.push_str(&format!(
            "- schema_version: {}\n- report_type: `{}`\n- crate: `{}` {}\n- stories: {}\n- viewports: {}\n- color_schemes: {}\n- captures: {}\n\n",
            self.schema_version,
            self.report_type,
            self.crate_name,
            self.crate_version,
            self.stories.len(),
            self.viewports.len(),
            self.color_schemes.len(),
            self.capture_count(),
        ));
        output.push_str("| capture | family | component | scenario | viewport | scheme | baseline | actual | diff | focus |\n");
        output.push_str("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n");
        for capture in &self.captures {
            output.push_str(&format!(
                "| `{}` | {} | {} | {} | {} {}x{}@{}x | {} | `{}` | `{}` | `{}` | {} |\n",
                capture.id,
                capture.component_family,
                capture.component,
                capture.scenario,
                capture.viewport_label,
                capture.width,
                capture.height,
                capture.scale_factor,
                capture.color_scheme.as_str(),
                capture.baseline_path,
                capture.actual_path,
                capture.diff_path,
                capture.release_focus,
            ));
        }
        output
    }
}

/// Return the current UI-kit visual regression capture manifest.
pub fn ui_kit_visual_regression_manifest() -> UiKitVisualRegressionManifest {
    let mut captures = Vec::with_capacity(
        UI_KIT_VISUAL_STORIES.len()
            * UI_KIT_VISUAL_VIEWPORTS.len()
            * UI_KIT_VISUAL_COLOR_SCHEMES.len(),
    );

    for story in UI_KIT_VISUAL_STORIES {
        for viewport in UI_KIT_VISUAL_VIEWPORTS {
            for &scheme in UI_KIT_VISUAL_COLOR_SCHEMES {
                let capture_id = format!("{}__{}__{}", story.id, viewport.id, scheme.as_str());
                captures.push(UiKitVisualCapture {
                    id: capture_id,
                    story_id: story.id,
                    story_label: story.label,
                    component: story.component,
                    component_family: story.component_family,
                    scenario: story.scenario,
                    viewport_id: viewport.id,
                    viewport_label: viewport.label,
                    width: viewport.width,
                    height: viewport.height,
                    scale_factor: viewport.scale_factor,
                    color_scheme: scheme,
                    release_focus: story.release_focus,
                    baseline_path: artifact_path("baseline", story.id, viewport.id, scheme),
                    actual_path: artifact_path("actual", story.id, viewport.id, scheme),
                    diff_path: artifact_path("diff", story.id, viewport.id, scheme),
                });
            }
        }
    }

    UiKitVisualRegressionManifest {
        schema_version: UI_KIT_VISUAL_REGRESSION_SCHEMA_VERSION,
        report_type: UI_KIT_VISUAL_REGRESSION_REPORT_TYPE,
        crate_name: env!("CARGO_PKG_NAME"),
        crate_version: env!("CARGO_PKG_VERSION"),
        stories: UI_KIT_VISUAL_STORIES,
        viewports: UI_KIT_VISUAL_VIEWPORTS,
        color_schemes: UI_KIT_VISUAL_COLOR_SCHEMES,
        captures,
    }
}

/// Return the static UI-kit visual regression stories.
pub const fn ui_kit_visual_stories() -> &'static [UiKitVisualStory] {
    UI_KIT_VISUAL_STORIES
}

pub const UI_KIT_VISUAL_VIEWPORTS: &[UiKitVisualViewport] = &[
    UiKitVisualViewport {
        id: "desktop-workbench",
        label: "Desktop workbench",
        width: 1200,
        height: 800,
        scale_factor: 2,
    },
    UiKitVisualViewport {
        id: "narrow-panel",
        label: "Narrow panel",
        width: 640,
        height: 900,
        scale_factor: 2,
    },
    UiKitVisualViewport {
        id: "mobile-preview",
        label: "Mobile preview",
        width: 390,
        height: 844,
        scale_factor: 3,
    },
];

pub const UI_KIT_VISUAL_COLOR_SCHEMES: &[UiKitVisualColorScheme] = &[
    UiKitVisualColorScheme::Light,
    UiKitVisualColorScheme::Dark,
    UiKitVisualColorScheme::HighContrast,
];

pub const UI_KIT_VISUAL_STORIES: &[UiKitVisualStory] = &[
    UiKitVisualStory {
        id: "ui-kit.button-set",
        label: "Buttons and icon buttons",
        component: "Button",
        component_family: "core",
        scenario: "primary, secondary, destructive, ghost, outline, disabled, icon-only",
        release_focus: "variant colors, focus rings, icon alignment, text fit, and disabled contrast",
    },
    UiKitVisualStory {
        id: "ui-kit.forms",
        label: "Form controls",
        component: "Input",
        component_family: "forms",
        scenario: "input, number input, checkbox, toggle, select, slider, validation",
        release_focus: "labels, validation states, keyboard focus, compact spacing, and long values",
    },
    UiKitVisualStory {
        id: "ui-kit.overlays",
        label: "Menus, popovers, and dialogs",
        component: "Dialog",
        component_family: "overlays",
        scenario: "menu bar, context menu, popover, confirm dialog, modal dialog",
        release_focus: "z-order, backdrop, Escape-dismiss affordance, restore-focus hint, and clipped content",
    },
    UiKitVisualStory {
        id: "ui-kit.data-display",
        label: "Data display",
        component: "Table",
        component_family: "data-display",
        scenario: "table, tree view, badges, progress, empty state, keyboard shortcut labels",
        release_focus: "dense row alignment, virtual-window spacers, hierarchy indentation, and status color legibility",
    },
    UiKitVisualStory {
        id: "ui-kit.navigation",
        label: "Navigation",
        component: "Tabs",
        component_family: "navigation",
        scenario: "tabs, breadcrumbs, accordion, wizard",
        release_focus: "selected states, step status, separators, collapsed panels, and narrow wrapping",
    },
    UiKitVisualStory {
        id: "ui-kit.mobile-surfaces",
        label: "Mobile surfaces",
        component: "SwipePanel",
        component_family: "mobile",
        scenario: "bottom and top anchored swipe panels with touch targets",
        release_focus: "snap positions, handle affordance, keyboard focus ring, and mobile viewport fit",
    },
    UiKitVisualStory {
        id: "ui-kit.workflow",
        label: "Workflow canvas",
        component: "WorkflowCanvas",
        component_family: "workflow",
        scenario: "nodes, ports, connections, selection, minimap-style dense canvas",
        release_focus: "connection paths, node labels, port hit targets, selection state, and high-DPI strokes",
    },
    UiKitVisualStory {
        id: "ui-kit.feedback",
        label: "Feedback and status",
        component: "Toast",
        component_family: "feedback",
        scenario: "alerts, inline alerts, toasts, loading overlay, spinner, tooltip",
        release_focus: "severity colors, motion-reduced loading states, overlay readability, and tooltip placement",
    },
];

fn artifact_path(
    kind: &str,
    story_id: &str,
    viewport_id: &str,
    color_scheme: UiKitVisualColorScheme,
) -> String {
    format!(
        "artifacts/gpui-ui-kit/visual/{kind}/{story_id}/{viewport_id}/{}.png",
        color_scheme.as_str()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visual_regression_manifest_has_stable_contract() {
        let manifest = ui_kit_visual_regression_manifest();

        assert_eq!(
            manifest.schema_version,
            UI_KIT_VISUAL_REGRESSION_SCHEMA_VERSION
        );
        assert_eq!(manifest.report_type, UI_KIT_VISUAL_REGRESSION_REPORT_TYPE);
        assert_eq!(manifest.crate_name, "gpui-ui-kit");
        assert_eq!(manifest.capture_count(), manifest.expected_capture_count());
        assert_eq!(manifest.stories.len(), 8);
        assert_eq!(manifest.viewports.len(), 3);
        assert_eq!(manifest.color_schemes.len(), 3);
        assert!(manifest.validate_unique_capture_ids());
    }

    #[test]
    fn visual_regression_manifest_covers_ui_component_families() {
        let manifest = ui_kit_visual_regression_manifest();
        let families = manifest.component_families();

        for family in [
            "core",
            "forms",
            "overlays",
            "data-display",
            "navigation",
            "mobile",
            "workflow",
            "feedback",
        ] {
            assert!(families.contains(family), "missing {family}");
        }

        for component in [
            "Button",
            "Input",
            "Dialog",
            "Table",
            "Tabs",
            "SwipePanel",
            "WorkflowCanvas",
            "Toast",
        ] {
            assert_eq!(
                manifest.captures_for_component(component).len(),
                manifest.viewports.len() * manifest.color_schemes.len(),
                "missing captures for {component}",
            );
        }
    }

    #[test]
    fn visual_regression_manifest_uses_stable_artifact_paths() {
        let manifest = ui_kit_visual_regression_manifest();
        let capture = manifest
            .captures
            .iter()
            .find(|capture| capture.id == "ui-kit.forms__mobile-preview__high_contrast")
            .expect("forms mobile high-contrast capture should exist");

        assert_eq!(
            capture.baseline_path,
            "artifacts/gpui-ui-kit/visual/baseline/ui-kit.forms/mobile-preview/high_contrast.png"
        );
        assert_eq!(
            capture.actual_path,
            "artifacts/gpui-ui-kit/visual/actual/ui-kit.forms/mobile-preview/high_contrast.png"
        );
        assert_eq!(
            capture.diff_path,
            "artifacts/gpui-ui-kit/visual/diff/ui-kit.forms/mobile-preview/high_contrast.png"
        );
    }

    #[test]
    fn visual_regression_manifest_markdown_is_release_attachable() {
        let markdown = ui_kit_visual_regression_manifest().to_markdown_table();

        assert!(markdown.contains(UI_KIT_VISUAL_REGRESSION_REPORT_TYPE));
        assert!(markdown.contains("ui-kit.data-display__desktop-workbench__dark"));
        assert!(markdown.contains("WorkflowCanvas"));
        assert!(markdown.contains("artifacts/gpui-ui-kit/visual/diff"));
        assert!(markdown.contains("high_contrast"));
    }
}
