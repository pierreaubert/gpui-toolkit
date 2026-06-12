use super::ShowcaseApp;
use crate::DemoSection;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

pub fn render(_app: &mut ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .gap_6()
        .child(
            div()
                .text_2xl()
                .font_weight(FontWeight::BOLD)
                .child("Welcome to d3rs"),
        )
        .child(
            div()
                .text_base()
                .max_w(px(600.0))
                .child("d3rs is a D3.js-inspired plotting library for GPUI. It brings familiar D3 concepts like scales, axes, and shape generators to Rust applications built with GPUI."),
        )
        .child(section_header("Demos"))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .ml_4()
                .child(nav_item("Scales", "Linear, logarithmic, ordinal, and band scales", DemoSection::Scales, cx))
                .child(nav_item("Axes", "Customizable axes with tick formatting", DemoSection::Axes, cx))
                .child(nav_item("Bar Charts", "Simple and grouped bar charts", DemoSection::BarCharts, cx))
                .child(nav_item("Line Charts", "Line charts with points and curves", DemoSection::LineCharts, cx))
                .child(nav_item("Scatter Plots", "Scatter plots with symbols", DemoSection::ScatterPlots, cx))
                .child(nav_item("Surface Plots", "2D surface and heatmap visualizations", DemoSection::SurfacePlots, cx))
                .child(nav_item("QuadTree", "Spatial indexing and nearest neighbor search", DemoSection::QuadTree, cx))
                .child(nav_item("Contours", "Contour lines and density estimation", DemoSection::Contours, cx))
                .child(nav_item("Transitions", "Animated transitions and easing", DemoSection::Transitions, cx))
                .child(nav_item("Geo", "Geographic projections and maps", DemoSection::Geo, cx))
                .child(nav_item("Colors", "Color schemes and interpolation", DemoSection::Colors, cx))
                .child(nav_item("Hierarchy", "Tree and cluster layouts", DemoSection::Hierarchy, cx))
                .child(nav_item("Force Graph", "Force-directed graph layout", DemoSection::Force, cx))
                .child(nav_item("Chord Diagram", "Chord layout with ribbons", DemoSection::Chord, cx)),
        )
        .child(section_header("D3 Observable Examples"))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .ml_4()
                .child(nav_item("Volcano", "Volcano contour visualization", DemoSection::D3VolcanoContours, cx))
                .child(nav_item("KDE", "Kernel density estimation", DemoSection::D3KDE, cx))
                .child(nav_item("Treemap", "Hierarchical treemap layout", DemoSection::D3Treemap, cx))
                .child(nav_item("Stacked Bars", "Stacked and grouped bar charts", DemoSection::D3StackedBars, cx))
                .child(nav_item("Versor Dragging", "Orthographic globe with projection switching", DemoSection::D3Versor, cx))
                .child(nav_item("Histogram", "Binned value distribution", DemoSection::D3Histogram, cx))
                .child(nav_item("Revenue Stream", "Music industry stacked area (RIAA data)", DemoSection::D3Revenue, cx))
                .child(nav_item("Horizon Chart", "Multi-band realtime horizon chart", DemoSection::D3Horizon, cx))
                .child(nav_item("Choropleth", "World map with colored regions", DemoSection::D3Choropleth, cx))
                .child(nav_item("Sankey", "Energy flow Sankey diagram", DemoSection::D3Sankey, cx))
                .child(nav_item("Calendar", "Calendar heatmap (52x7 grid)", DemoSection::D3Calendar, cx))
                .child(nav_item("Radial Line", "Polar coordinate temperature chart", DemoSection::D3RadialLine, cx))
                .child(nav_item("Parallel Coord", "Multi-axis car dataset (405 cars)", DemoSection::D3ParallelCoordinates, cx)),
        )
        .child(section_header("Observable Examples (Golden-Tested)"))
        .child(
            div()
                .text_sm()
                .ml_4()
                .mb_1()
                .child("These use d3rs::examples::* compute modules — validated against D3.js golden files."),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .ml_4()
                .child(nav_item("Hexbin", "Log scales + hexagonal binning (diamonds.csv)", DemoSection::D3Hexbin, cx))
                .child(nav_item("Pie Chart", "Pie layout with arc path generation", DemoSection::D3PieChart, cx))
                .child(nav_item("Donut Chart", "Pie with inner radius and pad angle", DemoSection::D3DonutChart, cx))
                .child(nav_item("Line Chart", "7 curve types (linear, basis, cardinal, etc.)", DemoSection::D3LineChart, cx))
                .child(nav_item("Streamgraph", "Stack with Wiggle offset + InsideOut order", DemoSection::D3Streamgraph, cx))
                .child(nav_item("Stacked Bar", "BandScale + Stack with diverging offset", DemoSection::D3StackedBar, cx))
                .child(nav_item("Stacked Area", "TimeScale + Stack + area paths (unemployment.csv)", DemoSection::D3StackedArea, cx))
                .child(nav_item("Box Plot", "Quartiles, whiskers, outliers (diamonds.csv)", DemoSection::D3BoxPlot, cx))
                .child(nav_item("Chord Diagram", "Chord layout from adjacency matrix", DemoSection::D3ChordDiagram, cx))
                .child(nav_item("Force Directed", "Force simulation (miserables.json, 77 nodes)", DemoSection::D3ForceDirected, cx))
                .child(nav_item("Parallel Sets", "Categorical flow (titanic.csv, Sankey layout)", DemoSection::D3ParallelSets, cx))
                .child(nav_item("Difference Chart", "Two-series area comparison (sfo-temperature.csv)", DemoSection::D3DifferenceChart, cx))
                .child(nav_item("Ridgeline Plot", "Monthly temperature distributions (weather.csv)", DemoSection::D3Ridgeline, cx))
                .child(nav_item("Realtime Horizon", "Streaming multi-band horizon with pos/neg bands", DemoSection::D3RealtimeHorizon, cx))
                .child(nav_item("Radial Tree", "Radial tree layout (Flare hierarchy)", DemoSection::D3RadialTree, cx))
                .child(nav_item("Radial Cluster", "Radial cluster/dendrogram (Flare hierarchy)", DemoSection::D3RadialCluster, cx))
                .child(nav_item("Circle Packing", "Nested circles from hierarchy values", DemoSection::D3CirclePacking, cx))
                .child(nav_item("Sunburst", "Partition layout with arc rendering", DemoSection::D3Sunburst, cx))
                .child(nav_item("Voronoi Airports", "Delaunay/Voronoi of 3000 airports (airports.csv)", DemoSection::D3VoronoiAirports, cx))
                .child(nav_item("Temperature Trends", "Global warming scatter (136 years, diverging colors)", DemoSection::D3TemperatureTrends, cx))
                .child(nav_item("H-R Diagram", "29K stars: magnitude vs color (Hertzsprung-Russell)", DemoSection::D3HertzsprungRussell, cx))
                .child(nav_item("Voronoi Labels", "Label placement using Voronoi cell area", DemoSection::D3VoronoiLabels, cx))
                .child(nav_item("Electric Usage", "Hourly heatmap (8760 hours × usage intensity)", DemoSection::D3ElectricUsage, cx))
                .child(nav_item("Star Map", "Stereographic star chart with magnitude scaling", DemoSection::D3StarMap, cx))
                .child(nav_item("Voronoi Stippling", "Image stippling via Lloyd's relaxation (wood.jpeg)", DemoSection::D3VoronoiStippling, cx)),
        )
        .child(
            div()
                .mt_4()
                .p_4()
                .bg(ui_theme.surface)
                .rounded_md()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .child("Click any example above to navigate, or use the sidebar."),
                ),
        )
}

fn section_header(title: &str) -> Div {
    div()
        .mt_4()
        .text_lg()
        .font_weight(FontWeight::SEMIBOLD)
        .child(title.to_string())
}

fn nav_item(title: &str, desc: &str, section: DemoSection, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let title_str = title.to_string();
    let desc_str = desc.to_string();
    // Wrap in a plain Div so the return type stays Div
    // The clickable element is a child Stateful<Div>
    div().child(
        div()
            .id(ElementId::Name(format!("nav-{}", title_str).into()))
            .flex()
            .gap_2()
            .py_1()
            .px_2()
            .rounded_md()
            .cursor_pointer()
            .hover(|s| s.bg(ui_theme.surface_hover))
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(format!("{title_str}:")),
            )
            .child(div().child(desc_str))
            .on_click(cx.listener(move |this, _, _window, _cx| {
                this.current_section = section;
            })),
    )
}
