//! Catalog of every graph published on www.spinorama.org.
//!
//! Each entry maps a measurement name (the API / payload stem) to the
//! renderer that can draw it. Specialised entries reuse the existing
//! CEA2034, contour, surface, and sphere views; everything else goes
//! through the generic trace renderers.

// Rust guideline compliant 2026-02-21

/// Renderer used for a catalog graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphRenderer {
    /// Existing CEA2034 view (also receives the full generic payload).
    Cea2034,
    /// Existing directivity (SPL vs frequency per angle) view.
    Directivity,
    /// Existing contour view (horizontal plane data).
    Contour,
    /// Existing 3D surface view fed by directivity data.
    Surface3d,
    /// Existing sphere view fed by directivity data.
    Sphere,
    /// Generic log/linear frequency chart (lines, fills, annotations).
    FrequencyChart,
    /// Generic polar chart from scatterpolar traces (radar / spider).
    PolarChart,
    /// Grouped categorical bar chart, optionally with scatter overlays.
    BarChart,
    /// Plotly `make_subplots` sheet: many mini charts in domain cells.
    Subplot,
}

/// One graph in the spinorama.org catalog.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraphDef {
    /// Measurement name sent to the API / payload file stem.
    pub measurement: &'static str,
    /// Human label for menus.
    pub label: &'static str,
    pub renderer: GraphRenderer,
    /// Y-axis title override (empty = take it from the payload layout).
    pub y_title: &'static str,
    /// Preferred y tick step (0 = derive from the payload / data).
    pub y_step: f64,
}

const fn freq(measurement: &'static str, label: &'static str) -> GraphDef {
    GraphDef {
        measurement,
        label,
        renderer: GraphRenderer::FrequencyChart,
        y_title: "",
        y_step: 0.0,
    }
}

const fn freq_y(
    measurement: &'static str,
    label: &'static str,
    y_title: &'static str,
    y_step: f64,
) -> GraphDef {
    GraphDef {
        measurement,
        label,
        renderer: GraphRenderer::FrequencyChart,
        y_title,
        y_step,
    }
}

/// Every graph on spinorama.org, in site order.
///
/// Measurement names for the `filters_*` family follow the generated
/// payload stems (`filters_eq`, `filters_auto_eq`, `filters_eq_full`,
/// `filters_{spin,on,lw,pir}_{noeq,auto}`,
/// `filters_{on,lw,pir}_hist_{fullrange,midrange}` — the same stems the
/// Python `AutoEQ` export writes as hardened WebP files).
pub const GRAPH_CATALOG: &[GraphDef] = &[
    GraphDef {
        measurement: "CEA2034",
        label: "CEA2034 (Spinorama)",
        renderer: GraphRenderer::Cea2034,
        y_title: "SPL (dB)",
        y_step: 5.0,
    },
    freq("On Axis", "On Axis"),
    freq("Early Reflections", "Early Reflections"),
    freq(
        "Estimated In-Room Response",
        "Estimated In-Room Response",
    ),
    freq(
        "Estimated In-Room Response Normalized",
        "Estimated In-Room Response Normalized",
    ),
    freq("Horizontal Reflections", "Horizontal Reflections"),
    freq("Vertical Reflections", "Vertical Reflections"),
    freq_y(
        "Horizontal Directivity",
        "Horizontal Directivity",
        "Angle (°)",
        90.0,
    ),
    freq_y(
        "Vertical Directivity",
        "Vertical Directivity",
        "Angle (°)",
        90.0,
    ),
    freq_y("Group Delay", "Group Delay", "Group Delay (ms)", 5.0),
    // Existing specialised SPL views.
    GraphDef {
        measurement: "SPL Horizontal",
        label: "SPL Horizontal",
        renderer: GraphRenderer::Directivity,
        y_title: "SPL (dB)",
        y_step: 5.0,
    },
    GraphDef {
        measurement: "SPL Horizontal Normalized",
        label: "SPL Horizontal Normalized",
        renderer: GraphRenderer::Directivity,
        y_title: "SPL (dB)",
        y_step: 5.0,
    },
    GraphDef {
        measurement: "SPL Vertical",
        label: "SPL Vertical",
        renderer: GraphRenderer::Directivity,
        y_title: "SPL (dB)",
        y_step: 5.0,
    },
    GraphDef {
        measurement: "SPL Vertical Normalized",
        label: "SPL Vertical Normalized",
        renderer: GraphRenderer::Directivity,
        y_title: "SPL (dB)",
        y_step: 5.0,
    },
    GraphDef {
        measurement: "SPL Horizontal Contour",
        label: "SPL Horizontal Contour",
        renderer: GraphRenderer::Contour,
        y_title: "",
        y_step: 0.0,
    },
    GraphDef {
        measurement: "SPL Horizontal Contour Normalized",
        label: "SPL Horizontal Contour Normalized",
        renderer: GraphRenderer::Contour,
        y_title: "",
        y_step: 0.0,
    },
    GraphDef {
        measurement: "SPL Vertical Contour",
        label: "SPL Vertical Contour",
        renderer: GraphRenderer::Contour,
        y_title: "",
        y_step: 0.0,
    },
    GraphDef {
        measurement: "SPL Vertical Contour Normalized",
        label: "SPL Vertical Contour Normalized",
        renderer: GraphRenderer::Contour,
        y_title: "",
        y_step: 0.0,
    },
    // 3D contour / globe views reuse the surface and sphere renderers.
    GraphDef {
        measurement: "SPL Horizontal Contour",
        label: "SPL Horizontal Contour 3D",
        renderer: GraphRenderer::Surface3d,
        y_title: "",
        y_step: 0.0,
    },
    GraphDef {
        measurement: "SPL Horizontal Contour Normalized",
        label: "SPL Horizontal Contour Normalized 3D",
        renderer: GraphRenderer::Surface3d,
        y_title: "",
        y_step: 0.0,
    },
    GraphDef {
        measurement: "SPL Vertical Contour",
        label: "SPL Vertical Contour 3D",
        renderer: GraphRenderer::Surface3d,
        y_title: "",
        y_step: 0.0,
    },
    GraphDef {
        measurement: "SPL Vertical Contour Normalized",
        label: "SPL Vertical Contour Normalized 3D",
        renderer: GraphRenderer::Surface3d,
        y_title: "",
        y_step: 0.0,
    },
    GraphDef {
        measurement: "SPL Horizontal Globe",
        label: "SPL Horizontal Globe",
        renderer: GraphRenderer::Sphere,
        y_title: "",
        y_step: 0.0,
    },
    GraphDef {
        measurement: "SPL Horizontal Globe Normalized",
        label: "SPL Horizontal Globe Normalized",
        renderer: GraphRenderer::Sphere,
        y_title: "",
        y_step: 0.0,
    },
    GraphDef {
        measurement: "SPL Vertical Globe",
        label: "SPL Vertical Globe",
        renderer: GraphRenderer::Sphere,
        y_title: "",
        y_step: 0.0,
    },
    GraphDef {
        measurement: "SPL Vertical Globe Normalized",
        label: "SPL Vertical Globe Normalized",
        renderer: GraphRenderer::Sphere,
        y_title: "",
        y_step: 0.0,
    },
    // Radar (generic polar: all payload traces, no 5-frequency cap).
    GraphDef {
        measurement: "SPL Horizontal Radar",
        label: "SPL Horizontal Radar",
        renderer: GraphRenderer::PolarChart,
        y_title: "",
        y_step: 0.0,
    },
    GraphDef {
        measurement: "SPL Vertical Radar",
        label: "SPL Vertical Radar",
        renderer: GraphRenderer::PolarChart,
        y_title: "",
        y_step: 0.0,
    },
    // Headphones: ordinary log-frequency scatter graphs.
    freq("Frequency Response", "Frequency Response (Headphones)"),
    freq(
        "Frequency Response Compensated",
        "Frequency Response Compensated (Headphones)",
    ),
    freq("Target Deviation", "Target Deviation (Headphones)"),
    // Auxiliary graphs.
    freq("eq_compare", "EQ Compare"),
    GraphDef {
        measurement: "spider",
        label: "Spider",
        renderer: GraphRenderer::PolarChart,
        y_title: "",
        y_step: 0.0,
    },
    freq("filters_eq", "Filters EQ"),
    GraphDef {
        measurement: "filters_auto_eq",
        label: "Filters Auto EQ",
        renderer: GraphRenderer::Subplot,
        y_title: "",
        y_step: 0.0,
    },
    freq("filters_eq_full", "Filters EQ Full"),
    freq("filters_spin_noeq", "Filters Spinorama (no EQ)"),
    freq("filters_spin_auto", "Filters Spinorama (auto EQ)"),
    freq("filters_on_noeq", "Filters On Axis (no EQ)"),
    freq("filters_on_auto", "Filters On Axis (auto EQ)"),
    freq("filters_lw_noeq", "Filters Listening Window (no EQ)"),
    freq("filters_lw_auto", "Filters Listening Window (auto EQ)"),
    freq("filters_pir_noeq", "Filters PIR (no EQ)"),
    freq("filters_pir_auto", "Filters PIR (auto EQ)"),
    GraphDef {
        measurement: "filters_on_hist_fullrange",
        label: "Filters On-Axis Histogram (Fullrange)",
        renderer: GraphRenderer::BarChart,
        y_title: "",
        y_step: 0.0,
    },
    GraphDef {
        measurement: "filters_on_hist_midrange",
        label: "Filters On-Axis Histogram (Midrange)",
        renderer: GraphRenderer::BarChart,
        y_title: "",
        y_step: 0.0,
    },
    GraphDef {
        measurement: "filters_lw_hist_fullrange",
        label: "Filters LW Histogram (Fullrange)",
        renderer: GraphRenderer::BarChart,
        y_title: "",
        y_step: 0.0,
    },
    GraphDef {
        measurement: "filters_lw_hist_midrange",
        label: "Filters LW Histogram (Midrange)",
        renderer: GraphRenderer::BarChart,
        y_title: "",
        y_step: 0.0,
    },
    GraphDef {
        measurement: "filters_pir_hist_fullrange",
        label: "Filters PIR Histogram (Fullrange)",
        renderer: GraphRenderer::BarChart,
        y_title: "",
        y_step: 0.0,
    },
    GraphDef {
        measurement: "filters_pir_hist_midrange",
        label: "Filters PIR Histogram (Midrange)",
        renderer: GraphRenderer::BarChart,
        y_title: "",
        y_step: 0.0,
    },
];

/// Looks up a catalog entry by measurement name.
#[allow(dead_code, reason = "used by the capture-bench batch renderer")]
pub fn graph_for_measurement(measurement: &str) -> Option<&'static GraphDef> {
    GRAPH_CATALOG.iter().find(|g| g.measurement == measurement)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_covers_every_site_family() {
        let names: Vec<&str> = GRAPH_CATALOG.iter().map(|g| g.measurement).collect();
        for required in [
            "CEA2034",
            "On Axis",
            "Group Delay",
            "SPL Horizontal Radar",
            "SPL Vertical Globe",
            "Frequency Response Compensated",
            "Target Deviation",
            "eq_compare",
            "spider",
            "filters_eq",
            "filters_auto_eq",
            "filters_spin_noeq",
            "filters_spin_auto",
            "filters_on_noeq",
            "filters_on_auto",
            "filters_lw_auto",
            "filters_pir_auto",
            "filters_pir_hist_midrange",
        ] {
            assert!(names.contains(&required), "missing {required}");
        }
        // filters_eq is a scatter of filter responses, not bars; the
        // combined sheet needs the subplot renderer.
        assert_eq!(
            graph_for_measurement("filters_eq").map(|def| def.renderer),
            Some(GraphRenderer::FrequencyChart)
        );
        assert_eq!(
            graph_for_measurement("filters_auto_eq").map(|def| def.renderer),
            Some(GraphRenderer::Subplot)
        );
        assert!(GRAPH_CATALOG.len() >= 40);
    }
}
