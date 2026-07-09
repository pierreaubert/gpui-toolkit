use super::design_documentation_report::{
    DESIGN_DOCUMENTATION_REPORT_TYPE, DesignDocumentationReport,
};
use super::design_system::all_design_presets;
use serde::Serialize;

pub const DESIGN_RELEASE_PRESENTATION_SCHEMA_VERSION: u32 = 1;
pub const DESIGN_RELEASE_PRESENTATION_REPORT_TYPE: &str = "gpui-design-release-presentation";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DesignReleaseAssetKind {
    DocumentationJson,
    DocumentationMarkdown,
    PresetScreenshot,
    ReleaseNotesMarkdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DesignReleaseAssetStatus {
    Generated,
    CaptureRequired,
}

impl DesignReleaseAssetStatus {
    pub fn label(self) -> &'static str {
        match self {
            DesignReleaseAssetStatus::Generated => "generated",
            DesignReleaseAssetStatus::CaptureRequired => "capture-required",
        }
    }

    pub fn is_release_blocking(self) -> bool {
        matches!(self, DesignReleaseAssetStatus::CaptureRequired)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DesignReleaseAsset {
    pub id: String,
    pub title: String,
    pub kind: DesignReleaseAssetKind,
    pub path: String,
    pub status: DesignReleaseAssetStatus,
    pub release_note_use: String,
}

impl DesignReleaseAsset {
    pub fn is_release_blocking(&self) -> bool {
        self.status.is_release_blocking()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DesignReleasePresentation {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub documentation_report_type: &'static str,
    pub documentation_report: DesignDocumentationReport,
    pub assets: Vec<DesignReleaseAsset>,
    pub release_notes_markdown: String,
}

impl DesignReleasePresentation {
    pub fn for_all_presets() -> Self {
        let documentation_report = DesignDocumentationReport::for_all_presets();
        let mut assets = vec![
            DesignReleaseAsset {
                id: "design-documentation-json".to_string(),
                title: "Design documentation JSON".to_string(),
                kind: DesignReleaseAssetKind::DocumentationJson,
                path: "release/gpui-design/design-documentation.json".to_string(),
                status: DesignReleaseAssetStatus::Generated,
                release_note_use:
                    "Machine-readable preset and conformance artifact for release verification."
                        .to_string(),
            },
            DesignReleaseAsset {
                id: "design-documentation-markdown".to_string(),
                title: "Design documentation Markdown".to_string(),
                kind: DesignReleaseAssetKind::DocumentationMarkdown,
                path: "release/gpui-design/design-documentation.md".to_string(),
                status: DesignReleaseAssetStatus::Generated,
                release_note_use: "Human-readable preset table and conformance summary."
                    .to_string(),
            },
            DesignReleaseAsset {
                id: "design-release-notes".to_string(),
                title: "Design release notes excerpt".to_string(),
                kind: DesignReleaseAssetKind::ReleaseNotesMarkdown,
                path: "release/gpui-design/release-notes.md".to_string(),
                status: DesignReleaseAssetStatus::Generated,
                release_note_use:
                    "Copy-ready release-note text that links the generated report and screenshots."
                        .to_string(),
            },
        ];

        for (preset_id, system) in all_design_presets() {
            assets.push(DesignReleaseAsset {
                id: format!("{preset_id}-screenshot"),
                title: format!("{} preset screenshot", system.language.label()),
                kind: DesignReleaseAssetKind::PresetScreenshot,
                path: format!("release/gpui-design/screenshots/{preset_id}.png"),
                status: DesignReleaseAssetStatus::CaptureRequired,
                release_note_use: format!(
                    "Visual proof for the {} design preset in release notes.",
                    system.language.label()
                ),
            });
        }

        let release_notes_markdown = render_release_notes_markdown(&documentation_report, &assets);

        Self {
            schema_version: DESIGN_RELEASE_PRESENTATION_SCHEMA_VERSION,
            report_type: DESIGN_RELEASE_PRESENTATION_REPORT_TYPE,
            documentation_report_type: DESIGN_DOCUMENTATION_REPORT_TYPE,
            documentation_report,
            assets,
            release_notes_markdown,
        }
    }

    pub fn blocking_assets(&self) -> Vec<&DesignReleaseAsset> {
        self.assets
            .iter()
            .filter(|asset| asset.is_release_blocking())
            .collect()
    }

    pub fn generated_assets(&self) -> Vec<&DesignReleaseAsset> {
        self.assets
            .iter()
            .filter(|asset| asset.status == DesignReleaseAssetStatus::Generated)
            .collect()
    }
}

fn render_release_notes_markdown(
    documentation_report: &DesignDocumentationReport,
    assets: &[DesignReleaseAsset],
) -> String {
    let mut output = String::from("# gpui-design Release Presentation\n\n");
    output.push_str("## Summary\n\n");
    output.push_str(&format!(
        "- Built-in presets documented: {}\n- Conformance: {}\n- Documentation report: `{}`\n\n",
        documentation_report.presets.len(),
        if documentation_report.passed() {
            "passed"
        } else {
            "failed"
        },
        DESIGN_DOCUMENTATION_REPORT_TYPE,
    ));
    output.push_str("## Attachments\n\n");
    output.push_str("| asset | status | path | release-note use |\n");
    output.push_str("| --- | --- | --- | --- |\n");
    for asset in assets {
        output.push_str(&format!(
            "| {} | `{}` | `{}` | {} |\n",
            asset.title,
            asset.status.label(),
            asset.path,
            asset.release_note_use
        ));
    }
    output
}
