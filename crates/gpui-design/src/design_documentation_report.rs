use super::DesignLanguage;
use super::design_conformance_matrix::DesignConformanceMatrix;
use super::design_system::all_design_presets;
use super::types::CornerRadiusStyle;
use serde::Serialize;

pub const DESIGN_DOCUMENTATION_REPORT_SCHEMA_VERSION: u32 = 1;
pub const DESIGN_DOCUMENTATION_REPORT_TYPE: &str = "gpui-design-documentation";

/// Stable docs-and-CI report for built-in design presets.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DesignDocumentationReport {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub presets: Vec<DesignPresetDocumentation>,
    pub conformance: DesignConformanceMatrix,
    pub markdown: String,
}

impl DesignDocumentationReport {
    pub fn for_all_presets() -> Self {
        let presets = all_design_presets()
            .into_iter()
            .map(|(preset_id, system)| DesignPresetDocumentation {
                preset_id,
                label: system.language.label(),
                language: system.language,
                token_count: system.style_dictionary_tokens_ref().len(),
                grid_unit: system.spacing.grid_unit,
                min_touch_target: system.interaction.min_touch_target,
                base_size: system.typography.base_size,
                corner_style: system.corners.style,
                motion_duration_ms: system.motion_spec(false).duration_ms,
                reduced_motion_duration_ms: system.motion_spec(true).duration_ms,
            })
            .collect::<Vec<_>>();
        let conformance = DesignConformanceMatrix::all_presets();
        let markdown = render_markdown(&presets, &conformance);

        Self {
            schema_version: DESIGN_DOCUMENTATION_REPORT_SCHEMA_VERSION,
            report_type: DESIGN_DOCUMENTATION_REPORT_TYPE,
            presets,
            conformance,
            markdown,
        }
    }

    pub fn passed(&self) -> bool {
        self.conformance.passed()
    }
}

/// Generated documentation summary for one built-in design preset.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DesignPresetDocumentation {
    pub preset_id: &'static str,
    pub label: &'static str,
    pub language: DesignLanguage,
    pub token_count: usize,
    pub grid_unit: f32,
    pub min_touch_target: f32,
    pub base_size: f32,
    pub corner_style: CornerRadiusStyle,
    pub motion_duration_ms: u32,
    pub reduced_motion_duration_ms: u32,
}

fn render_markdown(
    presets: &[DesignPresetDocumentation],
    conformance: &DesignConformanceMatrix,
) -> String {
    let mut output = String::from(
        "# GPUI Design Preset Report\n\n\
         ## Presets\n\n\
         | preset | language | tokens | grid | touch target | base type | corners | motion |\n\
         | --- | --- | ---: | ---: | ---: | ---: | --- | ---: |\n",
    );

    for preset in presets {
        output.push_str(&format!(
            "| {} | {} | {} | {:.1} | {:.1} | {:.1} | {:?} | {} ms |\n",
            preset.preset_id,
            preset.label,
            preset.token_count,
            preset.grid_unit,
            preset.min_touch_target,
            preset.base_size,
            preset.corner_style,
            preset.motion_duration_ms,
        ));
    }

    output.push_str("\n## Conformance\n\n");
    output.push_str(&conformance.to_markdown_table());
    output
}
