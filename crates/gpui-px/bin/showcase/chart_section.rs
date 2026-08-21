#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum ChartSection {
    #[default]
    Overview,
    Scatter,
    Line,
    Bar,
    BoxPlot,
    LogScales,
    Heatmap,
    Contour,
    Isoline,
    Treemap,
    Gallery,
    // Appended last so existing wasm-visual nav click coordinates for the
    // sections above stay valid.
    #[cfg(feature = "vello")]
    ScatterVello,
}

impl ChartSection {
    pub(super) fn all() -> &'static [ChartSection] {
        &[
            ChartSection::Overview,
            ChartSection::Scatter,
            ChartSection::Line,
            ChartSection::Bar,
            ChartSection::BoxPlot,
            ChartSection::LogScales,
            ChartSection::Heatmap,
            ChartSection::Contour,
            ChartSection::Isoline,
            ChartSection::Treemap,
            ChartSection::Gallery,
            #[cfg(feature = "vello")]
            ChartSection::ScatterVello,
        ]
    }

    pub(super) fn label(&self) -> &'static str {
        match self {
            ChartSection::Overview => "Overview",
            ChartSection::Scatter => "Scatter",
            ChartSection::Line => "Line",
            ChartSection::Bar => "Bar",
            ChartSection::BoxPlot => "Box Plot",
            ChartSection::LogScales => "Log Scales",
            ChartSection::Heatmap => "Heatmap",
            ChartSection::Contour => "Contour",
            ChartSection::Isoline => "Isoline",
            ChartSection::Treemap => "Treemap",
            ChartSection::Gallery => "Gallery",
            #[cfg(feature = "vello")]
            ChartSection::ScatterVello => "Scatter (vello)",
        }
    }
}

/// Emit the capture inventory consumed by the shared browser gallery tool.
/// Keep this beside the enum so adding a chart section automatically adds it
/// to the generated snapshot matrix.
#[cfg(not(target_family = "wasm"))]
pub(super) fn visual_manifest_json() -> String {
    use std::fmt::Write;

    const VIEWPORTS: &[(&str, &str, u32, u32, u32)] = &[
        ("desktop", "Desktop release viewport", 1200, 900, 1),
        ("narrow", "Narrow responsive viewport", 390, 844, 2),
    ];

    let mut json = String::from(
        "{\n  \"schema_version\": 1,\n  \"report_type\": \"gpui-px-visual-capture-manifest\",\n  \"crate_name\": \"gpui-px\",\n  \"crate_version\": \"",
    );
    let _ = write!(json, "{}", env!("CARGO_PKG_VERSION"));
    json.push_str("\",\n  \"viewports\": [\n");
    for (index, (id, label, width, height, scale_factor)) in VIEWPORTS.iter().enumerate() {
        let comma = if index + 1 == VIEWPORTS.len() {
            ""
        } else {
            ","
        };
        let _ = writeln!(
            json,
            "    {{ \"id\": \"{id}\", \"label\": \"{label}\", \"width\": {width}, \"height\": {height}, \"scale_factor\": {scale_factor} }}{comma}"
        );
    }
    json.push_str("  ],\n  \"captures\": [\n");

    let sections = ChartSection::all();
    let capture_count = sections.len() * VIEWPORTS.len();
    let mut capture_index = 0;
    for section in sections {
        let section_slug = slug(section.label());
        for (viewport_id, viewport_label, width, height, scale_factor) in VIEWPORTS {
            capture_index += 1;
            let comma = if capture_index == capture_count {
                ""
            } else {
                ","
            };
            let capture_id = format!("{section_slug}-{viewport_id}");
            let _ = writeln!(
                json,
                "    {{ \"id\": \"{capture_id}\", \"group\": \"Charts\", \"section\": \"{section_slug}\", \"section_label\": \"{}\", \"viewport_id\": \"{viewport_id}\", \"viewport_label\": \"{viewport_label}\", \"width\": {width}, \"height\": {height}, \"scale_factor\": {scale_factor}, \"renderer\": \"vello-auto\", \"renderer_query\": \"auto\", \"renderer_qa_queries\": [\"auto\", \"cpu\", \"legacy\"] }}{comma}",
                escape_json(section.label())
            );
        }
    }
    json.push_str("  ]\n}\n");
    json
}

#[cfg(not(target_family = "wasm"))]
fn slug(label: &str) -> String {
    let mut output = String::new();
    let mut pending_dash = false;
    for character in label.chars() {
        if character.is_ascii_alphanumeric() {
            if pending_dash && !output.is_empty() {
                output.push('-');
            }
            output.push(character.to_ascii_lowercase());
            pending_dash = false;
        } else {
            pending_dash = true;
        }
    }
    output
}

#[cfg(not(target_family = "wasm"))]
fn escape_json(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visual_manifest_covers_every_chart_section_and_viewport() {
        let manifest = visual_manifest_json();
        assert_eq!(
            manifest.matches("\"viewport_id\"").count(),
            ChartSection::all().len() * 2
        );
        assert!(manifest.contains("\"id\": \"scatter-desktop\""));
        assert!(manifest.contains("\"id\": \"heatmap-narrow\""));
        assert!(manifest.contains("\"renderer\": \"vello-auto\""));
        assert!(manifest.contains("\"renderer_query\": \"auto\""));
        assert!(manifest.contains("\"renderer_qa_queries\": [\"auto\", \"cpu\", \"legacy\"]"));
        assert!(manifest.contains("\"id\": \"scatter-vello-desktop\"") || !cfg!(feature = "vello"));
    }
}
