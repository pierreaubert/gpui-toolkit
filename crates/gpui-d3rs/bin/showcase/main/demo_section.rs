#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DemoSection {
    #[default]
    Overview,
    Scales,
    Axes,
    BarCharts,
    LineCharts,
    ScatterPlots,
    LodLargeData,
    SurfacePlots,
    Meshes,
    QuadTree,
    Contours,
    Transitions,
    Geo,
    Colors,
    Hierarchy,
    Force,
    Chord,
    // D3 Observable Examples
    D3VolcanoContours,
    D3KDE,
    D3Treemap,
    D3StackedBars,
    D3Versor,
    D3Histogram,
    D3Revenue,
    D3Horizon,
    D3Choropleth,
    // New D3 Examples
    D3Sankey,
    D3Calendar,
    D3RadialLine,
    D3ParallelCoordinates,
    // Observable Examples (using src/examples/ compute modules)
    D3Hexbin,
    D3PieChart,
    D3DonutChart,
    D3LineChart,
    D3Streamgraph,
    D3StackedBar,
    D3StackedArea,
    D3BoxPlot,
    D3ChordDiagram,
    D3ForceDirected,
    D3ParallelSets,
    D3DifferenceChart,
    D3Ridgeline,
    D3RealtimeHorizon,
    D3RadialTree,
    D3RadialCluster,
    D3CirclePacking,
    D3Sunburst,
    D3VoronoiAirports,
    D3TemperatureTrends,
    D3HertzsprungRussell,
    D3VoronoiLabels,
    D3ElectricUsage,
    D3StarMap,
    D3VoronoiStippling,
}

impl DemoSection {
    pub(super) fn all() -> Vec<Self> {
        vec![
            Self::Overview,
            Self::Scales,
            Self::Axes,
            Self::BarCharts,
            Self::LineCharts,
            Self::ScatterPlots,
            Self::LodLargeData,
            Self::SurfacePlots,
            Self::Meshes,
            Self::QuadTree,
            Self::Contours,
            Self::Transitions,
            Self::Geo,
            Self::Colors,
            Self::Hierarchy,
            Self::Force,
            Self::Chord,
            // D3 Observable Examples
            Self::D3VolcanoContours,
            Self::D3KDE,
            Self::D3Treemap,
            Self::D3StackedBars,
            Self::D3Versor,
            Self::D3Histogram,
            Self::D3Revenue,
            Self::D3Horizon,
            Self::D3Choropleth,
            // New D3 Examples
            Self::D3Sankey,
            Self::D3Calendar,
            Self::D3RadialLine,
            Self::D3ParallelCoordinates,
            // Observable Examples (golden-tested)
            Self::D3Hexbin,
            Self::D3PieChart,
            Self::D3DonutChart,
            Self::D3LineChart,
            Self::D3Streamgraph,
            Self::D3StackedBar,
            Self::D3StackedArea,
            Self::D3BoxPlot,
            Self::D3ChordDiagram,
            Self::D3ForceDirected,
            Self::D3ParallelSets,
            Self::D3DifferenceChart,
            Self::D3Ridgeline,
            Self::D3RealtimeHorizon,
            Self::D3RadialTree,
            Self::D3RadialCluster,
            Self::D3CirclePacking,
            Self::D3Sunburst,
            Self::D3VoronoiAirports,
            Self::D3TemperatureTrends,
            Self::D3HertzsprungRussell,
            Self::D3VoronoiLabels,
            Self::D3ElectricUsage,
            Self::D3StarMap,
            Self::D3VoronoiStippling,
        ]
    }

    pub(super) fn label(&self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Scales => "Scales",
            Self::Axes => "Axes",
            Self::BarCharts => "Bar Charts",
            Self::LineCharts => "Line Charts",
            Self::ScatterPlots => "Scatter Plots",
            Self::LodLargeData => "LOD / Large Data",
            Self::SurfacePlots => "Surface Plots",
            Self::Meshes => "Meshes",
            Self::QuadTree => "QuadTree",
            Self::Contours => "Contours",
            Self::Transitions => "Transitions",
            Self::Geo => "Geo",
            Self::Colors => "Colors",
            Self::Hierarchy => "Hierarchy",
            Self::Force => "Force Graph",
            Self::Chord => "Chord Diagram",
            // D3 Observable Examples
            Self::D3VolcanoContours => "D3: Volcano",
            Self::D3KDE => "D3: KDE",
            Self::D3Treemap => "D3: Treemap",
            Self::D3StackedBars => "D3: Stacked Bars",
            Self::D3Versor => "D3: Versor Dragging",
            Self::D3Histogram => "D3: Histogram",
            Self::D3Revenue => "D3: Revenue Stream",
            Self::D3Horizon => "D3: Horizon Chart",
            Self::D3Choropleth => "D3: Choropleth",
            // New D3 Examples
            Self::D3Sankey => "D3: Sankey Diagram",
            Self::D3Calendar => "D3: Calendar Heatmap",
            Self::D3RadialLine => "D3: Radial Line",
            Self::D3ParallelCoordinates => "D3: Parallel Coord",
            // Observable Examples (golden-tested)
            Self::D3Hexbin => "D3: Hexbin",
            Self::D3PieChart => "D3: Pie Chart",
            Self::D3DonutChart => "D3: Donut Chart",
            Self::D3LineChart => "D3: Line Chart",
            Self::D3Streamgraph => "D3: Streamgraph",
            Self::D3StackedBar => "D3: Stacked Bar",
            Self::D3StackedArea => "D3: Stacked Area",
            Self::D3BoxPlot => "D3: Box Plot",
            Self::D3ChordDiagram => "D3: Chord Diagram",
            Self::D3ForceDirected => "D3: Force Directed",
            Self::D3ParallelSets => "D3: Parallel Sets",
            Self::D3DifferenceChart => "D3: Difference Chart",
            Self::D3Ridgeline => "D3: Ridgeline Plot",
            Self::D3RealtimeHorizon => "D3: Realtime Horizon",
            Self::D3RadialTree => "D3: Radial Tree",
            Self::D3RadialCluster => "D3: Radial Cluster",
            Self::D3CirclePacking => "D3: Circle Packing",
            Self::D3Sunburst => "D3: Sunburst",
            Self::D3VoronoiAirports => "D3: Voronoi Airports",
            Self::D3TemperatureTrends => "D3: Temperature Trends",
            Self::D3HertzsprungRussell => "D3: H-R Diagram",
            Self::D3VoronoiLabels => "D3: Voronoi Labels",
            Self::D3ElectricUsage => "D3: Electric Usage",
            Self::D3StarMap => "D3: Star Map",
            Self::D3VoronoiStippling => "D3: Voronoi Stippling",
        }
    }
}

/// Emit the capture inventory consumed by the shared browser gallery tool.
/// Keep this beside the enum so adding a demo section automatically adds it
/// to the generated snapshot matrix.
#[cfg(not(target_family = "wasm"))]
pub(super) fn visual_manifest_json() -> String {
    use std::fmt::Write;

    const VIEWPORTS: &[(&str, &str, u32, u32, u32)] = &[
        ("desktop", "Desktop release viewport", 1200, 900, 1),
        ("narrow", "Narrow responsive viewport", 390, 844, 2),
    ];

    let mut json = String::from(
        "{\n  \"schema_version\": 1,\n  \"report_type\": \"gpui-d3rs-visual-capture-manifest\",\n  \"crate_name\": \"gpui-d3rs\",\n  \"crate_version\": \"",
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

    let sections = DemoSection::all();
    let capture_count = sections.len() * VIEWPORTS.len();
    let mut capture_index = 0;
    for section in &sections {
        let section_slug = slug(section.label());
        for (viewport_id, viewport_label, width, height, scale_factor) in VIEWPORTS {
            capture_index += 1;
            let comma = if capture_index == capture_count {
                ""
            } else {
                ","
            };
            let capture_id = format!("{section_slug}-{viewport_id}");
            // The geo capture uses the full 50m world dataset so gallery
            // thumbnails show detailed coastlines, not the simplified default.
            let extra_query = if matches!(*section, DemoSection::Geo) {
                ", \"extra_query\": {\"dataset\": \"large\"}"
            } else {
                ""
            };
            let _ = writeln!(
                json,
                "    {{ \"id\": \"{capture_id}\", \"group\": \"D3\", \"section\": \"{section_slug}\", \"section_label\": \"{}\", \"viewport_id\": \"{viewport_id}\", \"viewport_label\": \"{viewport_label}\", \"width\": {width}, \"height\": {height}, \"scale_factor\": {scale_factor}, \"renderer\": \"vello-auto\", \"renderer_query\": \"auto\", \"renderer_qa_queries\": [\"auto\", \"cpu\", \"legacy\"]{extra_query} }}{comma}",
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
    fn visual_manifest_covers_every_demo_section_and_viewport() {
        let manifest = visual_manifest_json();
        assert_eq!(
            manifest.matches("\"viewport_id\"").count(),
            DemoSection::all().len() * 2
        );
        assert!(manifest.contains("\"id\": \"scales-desktop\""));
        assert!(manifest.contains("\"id\": \"d3-volcano-narrow\""));
        assert!(manifest.contains("\"renderer\": \"vello-auto\""));
        assert!(manifest.contains("\"renderer_query\": \"auto\""));
        assert!(
            manifest.contains("\"id\": \"geo-desktop\", \"group\": \"D3\", \"section\": \"geo\"")
        );
        assert!(manifest.contains("\"extra_query\": {\"dataset\": \"large\"}"));
        assert_eq!(manifest.matches("\"extra_query\"").count(), 2);
    }
}
