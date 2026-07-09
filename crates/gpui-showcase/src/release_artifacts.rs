use crate::showcase::{ShowcaseGroup, ShowcaseSection};

pub const SHOWCASE_RELEASE_ARTIFACT_SCHEMA_VERSION: u32 = 1;
pub const SHOWCASE_RELEASE_ARTIFACT_REPORT_TYPE: &str = "gpui-showcase-release-artifacts";
pub const SHOWCASE_VISUAL_CAPTURE_SCHEMA_VERSION: u32 = 1;
pub const SHOWCASE_VISUAL_CAPTURE_REPORT_TYPE: &str = "gpui-showcase-visual-capture-manifest";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShowcaseReleaseArtifactStatus {
    Ready,
    ManualGate,
    ExternalGate,
}

impl ShowcaseReleaseArtifactStatus {
    pub fn label(self) -> &'static str {
        match self {
            ShowcaseReleaseArtifactStatus::Ready => "ready",
            ShowcaseReleaseArtifactStatus::ManualGate => "manual-gate",
            ShowcaseReleaseArtifactStatus::ExternalGate => "external-gate",
        }
    }

    pub fn is_release_blocking(self) -> bool {
        !matches!(self, ShowcaseReleaseArtifactStatus::Ready)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowcaseReleaseArtifact {
    pub id: &'static str,
    pub title: &'static str,
    pub artifact_type: &'static str,
    pub path_or_command: &'static str,
    pub status: ShowcaseReleaseArtifactStatus,
    pub release_requirement: &'static str,
}

impl ShowcaseReleaseArtifact {
    pub fn is_release_blocking(&self) -> bool {
        self.status.is_release_blocking()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowcaseStoryInventory {
    pub group_count: usize,
    pub section_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowcaseVisualCaptureViewport {
    pub id: &'static str,
    pub label: &'static str,
    pub width: u32,
    pub height: u32,
    pub scale_factor: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowcaseVisualCapture {
    pub id: String,
    pub group: &'static str,
    pub section: String,
    pub section_label: &'static str,
    pub viewport_id: &'static str,
    pub viewport_label: &'static str,
    pub width: u32,
    pub height: u32,
    pub scale_factor: u32,
    pub baseline_path: String,
    pub actual_path: String,
    pub diff_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowcaseVisualCaptureManifest {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub crate_name: &'static str,
    pub crate_version: &'static str,
    pub viewports: &'static [ShowcaseVisualCaptureViewport],
    pub captures: Vec<ShowcaseVisualCapture>,
}

impl ShowcaseVisualCaptureManifest {
    pub fn capture_count(&self) -> usize {
        self.captures.len()
    }

    pub fn expected_capture_count(&self) -> usize {
        ShowcaseSection::all().len() * self.viewports.len()
    }

    pub fn to_markdown_table(&self) -> String {
        let mut markdown = String::new();
        markdown.push_str("# gpui-showcase Visual Capture Manifest\n\n");
        markdown.push_str(&format!(
            "- schema_version: {}\n- report_type: `{}`\n- crate: `{}` {}\n- viewports: {}\n- captures: {}\n\n",
            self.schema_version,
            self.report_type,
            self.crate_name,
            self.crate_version,
            self.viewports.len(),
            self.capture_count(),
        ));
        markdown.push_str("| id | section | viewport | size | baseline | actual | diff |\n");
        markdown.push_str("| --- | --- | --- | --- | --- | --- | --- |\n");
        for capture in &self.captures {
            markdown.push_str(&format!(
                "| `{}` | {} / {} | {} | {}x{}@{}x | `{}` | `{}` | `{}` |\n",
                capture.id,
                capture.group,
                capture.section_label,
                capture.viewport_label,
                capture.width,
                capture.height,
                capture.scale_factor,
                capture.baseline_path,
                capture.actual_path,
                capture.diff_path,
            ));
        }
        markdown
    }

    pub fn to_json(&self) -> String {
        let mut json = String::new();
        json.push_str("{\n");
        json.push_str(&format!(
            "  \"schema_version\": {},\n  \"report_type\": \"{}\",\n  \"crate_name\": \"{}\",\n  \"crate_version\": \"{}\",\n",
            self.schema_version,
            escape_json(self.report_type),
            escape_json(self.crate_name),
            escape_json(self.crate_version),
        ));
        json.push_str("  \"viewports\": [\n");
        for (index, viewport) in self.viewports.iter().enumerate() {
            let comma = if index + 1 == self.viewports.len() {
                ""
            } else {
                ","
            };
            json.push_str(&format!(
                "    {{ \"id\": \"{}\", \"label\": \"{}\", \"width\": {}, \"height\": {}, \"scale_factor\": {} }}{}\n",
                escape_json(viewport.id),
                escape_json(viewport.label),
                viewport.width,
                viewport.height,
                viewport.scale_factor,
                comma,
            ));
        }
        json.push_str("  ],\n  \"captures\": [\n");
        for (index, capture) in self.captures.iter().enumerate() {
            let comma = if index + 1 == self.captures.len() {
                ""
            } else {
                ","
            };
            json.push_str(&format!(
                "    {{ \"id\": \"{}\", \"group\": \"{}\", \"section\": \"{}\", \"section_label\": \"{}\", \"viewport_id\": \"{}\", \"viewport_label\": \"{}\", \"width\": {}, \"height\": {}, \"scale_factor\": {}, \"baseline_path\": \"{}\", \"actual_path\": \"{}\", \"diff_path\": \"{}\" }}{}\n",
                escape_json(&capture.id),
                escape_json(capture.group),
                escape_json(&capture.section),
                escape_json(capture.section_label),
                escape_json(capture.viewport_id),
                escape_json(capture.viewport_label),
                capture.width,
                capture.height,
                capture.scale_factor,
                escape_json(&capture.baseline_path),
                escape_json(&capture.actual_path),
                escape_json(&capture.diff_path),
                comma,
            ));
        }
        json.push_str("  ]\n}\n");
        json
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowcaseReleaseArtifactReport {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub crate_name: &'static str,
    pub crate_version: &'static str,
    pub story_inventory: ShowcaseStoryInventory,
    pub artifacts: &'static [ShowcaseReleaseArtifact],
}

impl ShowcaseReleaseArtifactReport {
    pub fn blocking_entries(&self) -> Vec<&'static ShowcaseReleaseArtifact> {
        self.artifacts
            .iter()
            .filter(|artifact| artifact.is_release_blocking())
            .collect()
    }

    pub fn to_markdown(&self) -> String {
        let mut markdown = String::new();
        markdown.push_str("# gpui-showcase Release Artifacts\n\n");
        markdown.push_str(&format!(
            "- schema_version: {}\n- report_type: `{}`\n- crate: `{}` {}\n- story_groups: {}\n- story_sections: {}\n\n",
            self.schema_version,
            self.report_type,
            self.crate_name,
            self.crate_version,
            self.story_inventory.group_count,
            self.story_inventory.section_count,
        ));
        markdown.push_str("| id | status | artifact | path or command | release requirement |\n");
        markdown.push_str("| --- | --- | --- | --- | --- |\n");
        for artifact in self.artifacts {
            markdown.push_str(&format!(
                "| `{}` | `{}` | {} | `{}` | {} |\n",
                artifact.id,
                artifact.status.label(),
                artifact.title,
                artifact.path_or_command,
                artifact.release_requirement
            ));
        }
        markdown
    }
}

pub fn showcase_release_artifact_report() -> ShowcaseReleaseArtifactReport {
    ShowcaseReleaseArtifactReport {
        schema_version: SHOWCASE_RELEASE_ARTIFACT_SCHEMA_VERSION,
        report_type: SHOWCASE_RELEASE_ARTIFACT_REPORT_TYPE,
        crate_name: env!("CARGO_PKG_NAME"),
        crate_version: env!("CARGO_PKG_VERSION"),
        story_inventory: ShowcaseStoryInventory {
            group_count: ShowcaseGroup::all().len(),
            section_count: ShowcaseSection::all().len(),
        },
        artifacts: SHOWCASE_RELEASE_ARTIFACTS,
    }
}

pub fn showcase_visual_capture_manifest() -> ShowcaseVisualCaptureManifest {
    let mut captures = Vec::with_capacity(ShowcaseSection::all().len() * SHOWCASE_VIEWPORTS.len());
    for section in ShowcaseSection::all() {
        let group = section.group();
        let section_slug = slug(section.label());
        for viewport in SHOWCASE_VIEWPORTS {
            let capture_id = format!("{section_slug}-{}", viewport.id);
            captures.push(ShowcaseVisualCapture {
                id: capture_id.clone(),
                group: group.label(),
                section: section_slug.clone(),
                section_label: section.label(),
                viewport_id: viewport.id,
                viewport_label: viewport.label,
                width: viewport.width,
                height: viewport.height,
                scale_factor: viewport.scale_factor,
                baseline_path: format!(
                    "artifacts/showcase/visual/baseline/{}/{}.png",
                    viewport.id, section_slug
                ),
                actual_path: format!(
                    "artifacts/showcase/visual/actual/{}/{}.png",
                    viewport.id, section_slug
                ),
                diff_path: format!(
                    "artifacts/showcase/visual/diff/{}/{}.png",
                    viewport.id, section_slug
                ),
            });
        }
    }

    ShowcaseVisualCaptureManifest {
        schema_version: SHOWCASE_VISUAL_CAPTURE_SCHEMA_VERSION,
        report_type: SHOWCASE_VISUAL_CAPTURE_REPORT_TYPE,
        crate_name: env!("CARGO_PKG_NAME"),
        crate_version: env!("CARGO_PKG_VERSION"),
        viewports: SHOWCASE_VIEWPORTS,
        captures,
    }
}

pub const SHOWCASE_VIEWPORTS: &[ShowcaseVisualCaptureViewport] = &[
    ShowcaseVisualCaptureViewport {
        id: "desktop",
        label: "Desktop release viewport",
        width: 1200,
        height: 900,
        scale_factor: 1,
    },
    ShowcaseVisualCaptureViewport {
        id: "narrow",
        label: "Narrow responsive viewport",
        width: 390,
        height: 844,
        scale_factor: 2,
    },
];

pub const SHOWCASE_RELEASE_ARTIFACTS: &[ShowcaseReleaseArtifact] = &[
    ShowcaseReleaseArtifact {
        id: "desktop-showcase-binary",
        title: "Desktop showcase app",
        artifact_type: "runtime",
        path_or_command: "cargo run -p gpui-showcase --bin gpui-showcase",
        status: ShowcaseReleaseArtifactStatus::Ready,
        release_requirement: "Launches the public component showcase shell used for release walkthroughs.",
    },
    ShowcaseReleaseArtifact {
        id: "embeddable-showcase-library",
        title: "Embeddable Showcase component",
        artifact_type: "api",
        path_or_command: "gpui_showcase::Showcase",
        status: ShowcaseReleaseArtifactStatus::Ready,
        release_requirement: "Lets downstream tools embed the same component coverage without shell-specific code.",
    },
    ShowcaseReleaseArtifact {
        id: "story-inventory",
        title: "Grouped component story inventory",
        artifact_type: "metadata",
        path_or_command: "ShowcaseGroup::all() / ShowcaseSection::all()",
        status: ShowcaseReleaseArtifactStatus::Ready,
        release_requirement: "Provides a stable source of truth for the release walkthrough and capture matrix.",
    },
    ShowcaseReleaseArtifact {
        id: "release-qa-note",
        title: "Release QA readiness note",
        artifact_type: "documentation",
        path_or_command: "docs/qa-20260707.md",
        status: ShowcaseReleaseArtifactStatus::Ready,
        release_requirement: "Records what the showcase proves and what still needs external release validation.",
    },
    ShowcaseReleaseArtifact {
        id: "visual-capture-manifest",
        title: "CI-ready visual capture manifest",
        artifact_type: "metadata",
        path_or_command: "cargo run -p gpui-showcase --bin gpui-showcase -- --visual-manifest",
        status: ShowcaseReleaseArtifactStatus::Ready,
        release_requirement: "Enumerates every showcase section across desktop and narrow viewports with stable baseline/actual/diff artifact paths.",
    },
    ShowcaseReleaseArtifact {
        id: "manual-visual-walkthrough",
        title: "Manual desktop and narrow-width visual walkthrough",
        artifact_type: "qa-evidence",
        path_or_command: "manual release QA",
        status: ShowcaseReleaseArtifactStatus::ManualGate,
        release_requirement: "Required before release until automated screenshot capture is wired into CI.",
    },
    ShowcaseReleaseArtifact {
        id: "visual-regression-ci-artifacts",
        title: "Screenshot baseline, actual, and diff artifacts",
        artifact_type: "ci-evidence",
        path_or_command: "future visual regression CI job",
        status: ShowcaseReleaseArtifactStatus::ExternalGate,
        release_requirement: "Blocks calling the showcase fully automated; tracked separately from this artifact story.",
    },
];

fn slug(label: &str) -> String {
    let mut output = String::new();
    let mut pending_dash = false;
    for ch in label.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_dash && !output.is_empty() {
                output.push('-');
            }
            output.push(ch.to_ascii_lowercase());
            pending_dash = false;
        } else {
            pending_dash = true;
        }
    }
    output
}

fn escape_json(input: &str) -> String {
    let mut output = String::new();
    for ch in input.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            _ => output.push(ch),
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn release_artifact_report_has_stable_contract() {
        let report = showcase_release_artifact_report();

        assert_eq!(
            report.schema_version,
            SHOWCASE_RELEASE_ARTIFACT_SCHEMA_VERSION
        );
        assert_eq!(report.report_type, SHOWCASE_RELEASE_ARTIFACT_REPORT_TYPE);
        assert_eq!(report.crate_name, "gpui-showcase");
        assert_eq!(
            report.story_inventory.group_count,
            ShowcaseGroup::all().len()
        );
        assert_eq!(
            report.story_inventory.section_count,
            ShowcaseSection::all().len()
        );
        assert!(report.artifacts.len() >= 5);
    }

    #[test]
    fn release_artifact_report_names_required_release_outputs() {
        let report = showcase_release_artifact_report();
        let ids = report
            .artifacts
            .iter()
            .map(|artifact| artifact.id)
            .collect::<HashSet<_>>();

        assert!(ids.contains("desktop-showcase-binary"));
        assert!(ids.contains("embeddable-showcase-library"));
        assert!(ids.contains("story-inventory"));
        assert!(ids.contains("visual-capture-manifest"));
        assert!(ids.contains("manual-visual-walkthrough"));
        assert!(ids.contains("visual-regression-ci-artifacts"));
        assert!(
            report
                .artifacts
                .iter()
                .any(|artifact| artifact.path_or_command.contains("gpui-showcase"))
        );
    }

    #[test]
    fn release_artifact_blocking_entries_exclude_ready_outputs() {
        let report = showcase_release_artifact_report();
        let blocking = report.blocking_entries();

        assert_eq!(blocking.len(), 2);
        assert!(
            blocking
                .iter()
                .all(|artifact| artifact.status != ShowcaseReleaseArtifactStatus::Ready)
        );
        assert!(
            blocking
                .iter()
                .any(|artifact| artifact.id == "manual-visual-walkthrough")
        );
    }

    #[test]
    fn release_artifact_markdown_is_release_note_ready() {
        let markdown = showcase_release_artifact_report().to_markdown();

        assert!(markdown.contains(SHOWCASE_RELEASE_ARTIFACT_REPORT_TYPE));
        assert!(markdown.contains("story_sections"));
        assert!(markdown.contains("desktop-showcase-binary"));
        assert!(markdown.contains("visual-regression-ci-artifacts"));
        assert!(markdown.contains("visual-capture-manifest"));
        assert!(markdown.contains("cargo run -p gpui-showcase --bin gpui-showcase"));
    }

    #[test]
    fn visual_capture_manifest_covers_every_section_and_viewport() {
        let manifest = showcase_visual_capture_manifest();

        assert_eq!(
            manifest.schema_version,
            SHOWCASE_VISUAL_CAPTURE_SCHEMA_VERSION
        );
        assert_eq!(manifest.report_type, SHOWCASE_VISUAL_CAPTURE_REPORT_TYPE);
        assert_eq!(manifest.crate_name, "gpui-showcase");
        assert_eq!(manifest.viewports, SHOWCASE_VIEWPORTS);
        assert_eq!(manifest.capture_count(), manifest.expected_capture_count());
        assert_eq!(
            manifest.capture_count(),
            ShowcaseSection::all().len() * SHOWCASE_VIEWPORTS.len()
        );
    }

    #[test]
    fn visual_capture_manifest_uses_stable_capture_ids_and_paths() {
        let manifest = showcase_visual_capture_manifest();
        let ids = manifest
            .captures
            .iter()
            .map(|capture| capture.id.as_str())
            .collect::<HashSet<_>>();

        assert_eq!(ids.len(), manifest.captures.len());
        assert!(ids.contains("buttons-desktop"));
        assert!(ids.contains("command-palette-narrow"));

        let buttons = manifest
            .captures
            .iter()
            .find(|capture| capture.id == "buttons-desktop")
            .expect("buttons desktop capture should exist");
        assert_eq!(buttons.group, "Actions");
        assert_eq!(buttons.section, "buttons");
        assert_eq!(buttons.viewport_id, "desktop");
        assert_eq!(
            buttons.baseline_path,
            "artifacts/showcase/visual/baseline/desktop/buttons.png"
        );
        assert_eq!(
            buttons.actual_path,
            "artifacts/showcase/visual/actual/desktop/buttons.png"
        );
        assert_eq!(
            buttons.diff_path,
            "artifacts/showcase/visual/diff/desktop/buttons.png"
        );
    }

    #[test]
    fn visual_capture_manifest_outputs_markdown_and_json() {
        let manifest = showcase_visual_capture_manifest();
        let markdown = manifest.to_markdown_table();
        let json = manifest.to_json();

        assert!(markdown.contains(SHOWCASE_VISUAL_CAPTURE_REPORT_TYPE));
        assert!(markdown.contains("buttons-desktop"));
        assert!(markdown.contains("artifacts/showcase/visual/diff/desktop/buttons.png"));

        assert!(json.contains("\"report_type\": \"gpui-showcase-visual-capture-manifest\""));
        assert!(json.contains("\"id\": \"buttons-desktop\""));
        assert!(json.contains("\"viewport_id\": \"narrow\""));
    }

    #[test]
    fn showcase_section_inventory_matches_group_inventory_once() {
        let mut grouped_sections = Vec::new();
        for group in ShowcaseGroup::all() {
            grouped_sections.extend_from_slice(group.sections());
        }

        assert_eq!(grouped_sections.len(), ShowcaseSection::all().len());
        for section in ShowcaseSection::all() {
            assert_eq!(
                grouped_sections
                    .iter()
                    .filter(|grouped| *grouped == section)
                    .count(),
                1,
                "{section:?} should appear in exactly one showcase group"
            );
        }
    }
}
