//! Visual regression capture inventory for gpui-px release QA.

use std::collections::{BTreeSet, HashSet};

/// Schema version for [`ChartVisualRegressionManifest`].
pub const CHART_VISUAL_REGRESSION_SCHEMA_VERSION: u32 = 1;

/// Stable report type identifier for [`ChartVisualRegressionManifest`].
pub const CHART_VISUAL_REGRESSION_REPORT_TYPE: &str = "gpui-px-visual-regression-manifest";

/// Theme/color scheme expected for a chart visual capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChartVisualColorScheme {
    Light,
    Dark,
    HighContrast,
}

impl ChartVisualColorScheme {
    /// Stable label used in capture ids and artifact paths.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::HighContrast => "high_contrast",
        }
    }
}

/// Viewport preset expected for a chart visual capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChartVisualViewport {
    pub id: &'static str,
    pub label: &'static str,
    pub width: u32,
    pub height: u32,
    pub scale_factor: u32,
}

/// One chart story that should be captured by visual regression CI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChartVisualStory {
    pub id: &'static str,
    pub label: &'static str,
    pub chart_family: &'static str,
    pub scenario: &'static str,
    pub release_focus: &'static str,
}

/// One deterministic chart screenshot capture expected by release QA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartVisualCapture {
    pub id: String,
    pub story_id: &'static str,
    pub story_label: &'static str,
    pub chart_family: &'static str,
    pub scenario: &'static str,
    pub viewport_id: &'static str,
    pub viewport_label: &'static str,
    pub width: u32,
    pub height: u32,
    pub scale_factor: u32,
    pub color_scheme: ChartVisualColorScheme,
    pub release_focus: &'static str,
    pub baseline_path: String,
    pub actual_path: String,
    pub diff_path: String,
}

/// Versioned chart screenshot manifest for visual regression tooling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartVisualRegressionManifest {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub crate_name: &'static str,
    pub crate_version: &'static str,
    pub stories: &'static [ChartVisualStory],
    pub viewports: &'static [ChartVisualViewport],
    pub color_schemes: &'static [ChartVisualColorScheme],
    pub captures: Vec<ChartVisualCapture>,
}

impl ChartVisualRegressionManifest {
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

    /// Chart families covered by the manifest.
    pub fn chart_families(&self) -> BTreeSet<&'static str> {
        self.stories
            .iter()
            .map(|story| story.chart_family)
            .collect()
    }

    /// Return generated captures for a chart family.
    pub fn captures_for_family(&self, chart_family: &str) -> Vec<&ChartVisualCapture> {
        self.captures
            .iter()
            .filter(|capture| capture.chart_family == chart_family)
            .collect()
    }

    /// Render the manifest as Markdown for release artifacts.
    pub fn to_markdown_table(&self) -> String {
        let mut output = String::from("# gpui-px Visual Regression Manifest\n\n");
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
        output.push_str("| capture | chart family | scenario | viewport | scheme | baseline | actual | diff | focus |\n");
        output.push_str("| --- | --- | --- | --- | --- | --- | --- | --- | --- |\n");
        for capture in &self.captures {
            output.push_str(&format!(
                "| `{}` | {} | {} | {} {}x{}@{}x | {} | `{}` | `{}` | `{}` | {} |\n",
                capture.id,
                capture.chart_family,
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

/// Return the current chart visual regression capture manifest.
pub fn chart_visual_regression_manifest() -> ChartVisualRegressionManifest {
    let mut captures = Vec::with_capacity(
        CHART_VISUAL_STORIES.len()
            * CHART_VISUAL_VIEWPORTS.len()
            * CHART_VISUAL_COLOR_SCHEMES.len(),
    );

    for story in CHART_VISUAL_STORIES {
        for viewport in CHART_VISUAL_VIEWPORTS {
            for &scheme in CHART_VISUAL_COLOR_SCHEMES {
                let capture_id = format!("{}__{}__{}", story.id, viewport.id, scheme.as_str());
                captures.push(ChartVisualCapture {
                    id: capture_id,
                    story_id: story.id,
                    story_label: story.label,
                    chart_family: story.chart_family,
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

    ChartVisualRegressionManifest {
        schema_version: CHART_VISUAL_REGRESSION_SCHEMA_VERSION,
        report_type: CHART_VISUAL_REGRESSION_REPORT_TYPE,
        crate_name: env!("CARGO_PKG_NAME"),
        crate_version: env!("CARGO_PKG_VERSION"),
        stories: CHART_VISUAL_STORIES,
        viewports: CHART_VISUAL_VIEWPORTS,
        color_schemes: CHART_VISUAL_COLOR_SCHEMES,
        captures,
    }
}

/// Return the static chart visual regression stories.
pub const fn chart_visual_stories() -> &'static [ChartVisualStory] {
    CHART_VISUAL_STORIES
}

pub const CHART_VISUAL_VIEWPORTS: &[ChartVisualViewport] = &[
    ChartVisualViewport {
        id: "dashboard-wide",
        label: "Dashboard wide",
        width: 1280,
        height: 760,
        scale_factor: 2,
    },
    ChartVisualViewport {
        id: "panel-compact",
        label: "Panel compact",
        width: 720,
        height: 520,
        scale_factor: 2,
    },
    ChartVisualViewport {
        id: "mobile-card",
        label: "Mobile card",
        width: 390,
        height: 640,
        scale_factor: 3,
    },
];

pub const CHART_VISUAL_COLOR_SCHEMES: &[ChartVisualColorScheme] = &[
    ChartVisualColorScheme::Light,
    ChartVisualColorScheme::Dark,
    ChartVisualColorScheme::HighContrast,
];

pub const CHART_VISUAL_STORIES: &[ChartVisualStory] = &[
    ChartVisualStory {
        id: "px.scatter",
        label: "Scatter plot",
        chart_family: "scatter",
        scenario: "multi-point correlation with visible outliers",
        release_focus: "point radius, opacity, axis labels, hover target density, and legend spacing",
    },
    ChartVisualStory {
        id: "px.line",
        label: "Line chart",
        chart_family: "line",
        scenario: "multi-series time or frequency response",
        release_focus: "stroke joins, point markers, hidden series, secondary axis, and legend state",
    },
    ChartVisualStory {
        id: "px.bar",
        label: "Bar chart",
        chart_family: "bar",
        scenario: "grouped categorical comparison",
        release_focus: "bar grouping, category label fit, zero line, border radius, and legend colors",
    },
    ChartVisualStory {
        id: "px.area",
        label: "Area chart",
        chart_family: "area",
        scenario: "filled trend with explicit baseline",
        release_focus: "closed area path, baseline position, fill opacity, and line contrast",
    },
    ChartVisualStory {
        id: "px.pie-donut",
        label: "Pie and donut",
        chart_family: "pie/donut",
        scenario: "sorted slices with labels and donut hole",
        release_focus: "slice arcs, label collision risk, legend colors, padding, and center hole",
    },
    ChartVisualStory {
        id: "px.heatmap",
        label: "Heatmap",
        chart_family: "heatmap",
        scenario: "matrix color scale with explicit ranges",
        release_focus: "cell alignment, color-scale continuity, axis ticks, and colorbar legibility",
    },
    ChartVisualStory {
        id: "px.boxplot",
        label: "Box plot",
        chart_family: "boxplot",
        scenario: "distribution with whiskers and outliers",
        release_focus: "quartile box, median line, whisker caps, outlier dots, and category spacing",
    },
    ChartVisualStory {
        id: "px.treemap",
        label: "Treemap",
        chart_family: "treemap",
        scenario: "hierarchical rectangular tiling",
        release_focus: "tiling stability, category colors, label clipping, and nested boundaries",
    },
    ChartVisualStory {
        id: "px.isoline",
        label: "Isoline",
        chart_family: "isoline",
        scenario: "contour-line scalar field",
        release_focus: "path smoothing, level spacing, stroke contrast, and axis scaling",
    },
    ChartVisualStory {
        id: "px.contour",
        label: "Contour",
        chart_family: "contour",
        scenario: "filled contour bands",
        release_focus: "band ordering, fill opacity, threshold labels, and color-scale continuity",
    },
    ChartVisualStory {
        id: "px.surface3d",
        label: "Surface 3D",
        chart_family: "optional surface3d",
        scenario: "projected 3D surface mesh",
        release_focus: "projection framing, wireframe visibility, colorbar, z-range, and fallback messaging",
    },
];

fn artifact_path(
    kind: &str,
    story_id: &str,
    viewport_id: &str,
    color_scheme: ChartVisualColorScheme,
) -> String {
    format!(
        "artifacts/gpui-px/visual/{kind}/{story_id}/{viewport_id}/{}.png",
        color_scheme.as_str()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_visual_regression_manifest_has_stable_contract() {
        let manifest = chart_visual_regression_manifest();

        assert_eq!(
            manifest.schema_version,
            CHART_VISUAL_REGRESSION_SCHEMA_VERSION
        );
        assert_eq!(manifest.report_type, CHART_VISUAL_REGRESSION_REPORT_TYPE);
        assert_eq!(manifest.crate_name, "gpui-px");
        assert_eq!(manifest.capture_count(), manifest.expected_capture_count());
        assert_eq!(manifest.stories.len(), 11);
        assert_eq!(manifest.viewports.len(), 3);
        assert_eq!(manifest.color_schemes.len(), 3);
        assert!(manifest.validate_unique_capture_ids());
    }

    #[test]
    fn chart_visual_regression_manifest_covers_chart_families() {
        let manifest = chart_visual_regression_manifest();
        let families = manifest.chart_families();

        for family in [
            "scatter",
            "line",
            "bar",
            "area",
            "pie/donut",
            "heatmap",
            "boxplot",
            "treemap",
            "isoline",
            "contour",
            "optional surface3d",
        ] {
            assert!(families.contains(family), "missing {family}");
            assert_eq!(
                manifest.captures_for_family(family).len(),
                manifest.viewports.len() * manifest.color_schemes.len(),
                "missing captures for {family}",
            );
        }
    }

    #[test]
    fn chart_visual_regression_manifest_uses_stable_artifact_paths() {
        let manifest = chart_visual_regression_manifest();
        let capture = manifest
            .captures
            .iter()
            .find(|capture| capture.id == "px.heatmap__mobile-card__high_contrast")
            .expect("heatmap mobile high-contrast capture should exist");

        assert_eq!(
            capture.baseline_path,
            "artifacts/gpui-px/visual/baseline/px.heatmap/mobile-card/high_contrast.png"
        );
        assert_eq!(
            capture.actual_path,
            "artifacts/gpui-px/visual/actual/px.heatmap/mobile-card/high_contrast.png"
        );
        assert_eq!(
            capture.diff_path,
            "artifacts/gpui-px/visual/diff/px.heatmap/mobile-card/high_contrast.png"
        );
    }

    #[test]
    fn chart_visual_regression_manifest_markdown_is_release_attachable() {
        let markdown = chart_visual_regression_manifest().to_markdown_table();

        assert!(markdown.contains(CHART_VISUAL_REGRESSION_REPORT_TYPE));
        assert!(markdown.contains("px.scatter__dashboard-wide__dark"));
        assert!(markdown.contains("optional surface3d"));
        assert!(markdown.contains("artifacts/gpui-px/visual/diff"));
        assert!(markdown.contains("high_contrast"));
    }
}
