//! Visual regression manifest helpers for layout stories.
//!
//! The builder crate does not capture pixels itself; this module produces a
//! stable manifest that screenshot runners can consume.

use crate::util::format_number;
use crate::{LayoutStoryCatalog, SolvedLayoutScenario};
use serde::{Serialize, Serializer, ser::SerializeStruct};
use std::collections::{BTreeMap, BTreeSet, HashSet};

/// Color scheme requested by a visual regression capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualColorScheme {
    Light,
    Dark,
    HighContrast,
}

impl VisualColorScheme {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::HighContrast => "high_contrast",
        }
    }
}

/// One screenshot capture case.
#[derive(Debug, Clone, PartialEq)]
pub struct VisualRegressionCase {
    pub id: String,
    pub story_id: String,
    pub scenario_id: String,
    pub width: f32,
    pub height: f32,
    pub color_scheme: VisualColorScheme,
    pub solved_text: String,
}

impl VisualRegressionCase {
    pub fn from_solved(
        solved: &SolvedLayoutScenario,
        color_scheme: VisualColorScheme,
    ) -> VisualRegressionCase {
        let id = capture_id(&solved.story_id, &solved.scenario_id, color_scheme);
        Self {
            id,
            story_id: solved.story_id.clone(),
            scenario_id: solved.scenario_id.clone(),
            width: solved.width,
            height: solved.height,
            color_scheme,
            solved_text: solved.to_text(),
        }
    }

    pub fn output_path(&self) -> String {
        output_path(&self.story_id, &self.scenario_id, self.color_scheme)
    }
}

impl Serialize for VisualRegressionCase {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("VisualRegressionCase", 8)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("story_id", &self.story_id)?;
        state.serialize_field("scenario_id", &self.scenario_id)?;
        state.serialize_field("width", &self.width)?;
        state.serialize_field("height", &self.height)?;
        state.serialize_field("color_scheme", &self.color_scheme)?;
        state.serialize_field("output_path", &self.output_path())?;
        state.serialize_field("solved_text", &self.solved_text)?;
        state.end()
    }
}

/// A stable list of screenshot captures.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct VisualRegressionManifest {
    pub cases: Vec<VisualRegressionCase>,
}

/// Visual regression manifest coverage finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VisualRegressionFinding {
    pub id: &'static str,
    pub message: String,
}

/// Result of validating a manifest before a screenshot runner consumes it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct VisualRegressionCoverageReport {
    pub findings: Vec<VisualRegressionFinding>,
}

impl VisualRegressionCoverageReport {
    pub fn passed(&self) -> bool {
        self.findings.is_empty()
    }
}

impl VisualRegressionManifest {
    pub fn from_catalog(
        catalog: &LayoutStoryCatalog<'_>,
        color_schemes: &[VisualColorScheme],
    ) -> Self {
        let schemes = if color_schemes.is_empty() {
            &[VisualColorScheme::Dark][..]
        } else {
            color_schemes
        };

        let mut cases = Vec::new();
        for solved in catalog.solve_all() {
            for &scheme in schemes {
                cases.push(VisualRegressionCase::from_solved(&solved, scheme));
            }
        }
        cases.sort_by(|a, b| a.id.cmp(&b.id));
        Self { cases }
    }

    pub fn validate_required_schemes(
        &self,
        required_schemes: &[VisualColorScheme],
    ) -> VisualRegressionCoverageReport {
        let required: BTreeSet<_> = if required_schemes.is_empty() {
            [VisualColorScheme::Dark].into_iter().collect()
        } else {
            required_schemes.iter().copied().collect()
        };
        let mut findings = Vec::new();

        if self.cases.is_empty() {
            findings.push(VisualRegressionFinding {
                id: "manifest.empty",
                message: "visual regression manifest has no capture cases".to_string(),
            });
            return VisualRegressionCoverageReport { findings };
        }

        let mut ids = HashSet::new();
        let mut by_scenario: BTreeMap<(&str, &str), BTreeSet<VisualColorScheme>> = BTreeMap::new();
        for case in &self.cases {
            if !ids.insert(case.id.as_str()) {
                findings.push(VisualRegressionFinding {
                    id: "manifest.duplicate_id",
                    message: format!("duplicate capture id {:?}", case.id),
                });
            }
            by_scenario
                .entry((&case.story_id, &case.scenario_id))
                .or_default()
                .insert(case.color_scheme);
        }

        for ((story_id, scenario_id), schemes) in by_scenario {
            for scheme in &required {
                if !schemes.contains(scheme) {
                    findings.push(VisualRegressionFinding {
                        id: "manifest.missing_scheme",
                        message: format!(
                            "{story_id}/{scenario_id} is missing {} capture",
                            scheme.as_str()
                        ),
                    });
                }
            }
        }

        VisualRegressionCoverageReport { findings }
    }

    pub fn to_markdown_table(&self) -> String {
        let mut output = String::from(
            "| capture | story | scenario | size | scheme | output |\n\
             | --- | --- | --- | ---: | --- | --- |\n",
        );
        for case in &self.cases {
            output.push_str(&format!(
                "| {} | {} | {} | {}x{} | {} | {} |\n",
                case.id,
                case.story_id,
                case.scenario_id,
                format_number(case.width),
                format_number(case.height),
                case.color_scheme.as_str(),
                case.output_path()
            ));
        }
        output
    }
}

fn capture_id(story_id: &str, scenario_id: &str, color_scheme: VisualColorScheme) -> String {
    format!("{story_id}__{scenario_id}__{}", color_scheme.as_str())
}

fn output_path(story_id: &str, scenario_id: &str, color_scheme: VisualColorScheme) -> String {
    format!("{story_id}/{scenario_id}/{}.png", color_scheme.as_str())
}

#[cfg(test)]
mod tests {
    use super::{LayoutStoryCatalog, VisualColorScheme, VisualRegressionManifest};
    use crate::{Axis, ContainerNode, LayoutNode, LayoutScenario, LayoutStory, Sizing, SlotNode};

    #[test]
    fn manifest_expands_story_scenarios_across_color_schemes() {
        let children = [LayoutNode::Slot(SlotNode {
            id: "content",
            sizing: Sizing::flex(120.0),
            priority: 1.0,
            collapsible: false,
            display_tiers: &[],
            collapse_label: None,
        })];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Vertical,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 0.0,
        });
        let scenarios = [LayoutScenario::new("phone", "Phone", 390.0, 844.0)];
        let story = LayoutStory::new("player", "Player", root, &scenarios);
        let stories = [story];
        let catalog = LayoutStoryCatalog::new(&stories);

        let manifest = VisualRegressionManifest::from_catalog(
            &catalog,
            &[VisualColorScheme::Light, VisualColorScheme::Dark],
        );

        assert_eq!(manifest.cases.len(), 2);
        assert_eq!(manifest.cases[0].id, "player__phone__dark");
        assert!(manifest.to_markdown_table().contains("390x844"));
    }

    #[test]
    fn manifest_reports_missing_required_color_schemes() {
        let children = [LayoutNode::Slot(SlotNode {
            id: "content",
            sizing: Sizing::flex(120.0),
            priority: 1.0,
            collapsible: false,
            display_tiers: &[],
            collapse_label: None,
        })];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Vertical,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 0.0,
        });
        let scenarios = [LayoutScenario::new("phone", "Phone", 390.0, 844.0)];
        let story = LayoutStory::new("player", "Player", root, &scenarios);
        let stories = [story];
        let catalog = LayoutStoryCatalog::new(&stories);

        let manifest = VisualRegressionManifest::from_catalog(&catalog, &[VisualColorScheme::Dark]);
        let report = manifest.validate_required_schemes(&[
            VisualColorScheme::Light,
            VisualColorScheme::Dark,
            VisualColorScheme::HighContrast,
        ]);

        assert!(!report.passed());
        assert_eq!(report.findings.len(), 2);
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.message.contains("light"))
        );
    }

    #[test]
    fn manifest_is_serializable_for_screenshot_runners() {
        let children = [LayoutNode::Slot(SlotNode {
            id: "content",
            sizing: Sizing::flex(120.0),
            priority: 1.0,
            collapsible: false,
            display_tiers: &[],
            collapse_label: None,
        })];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Vertical,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 0.0,
        });
        let scenarios = [LayoutScenario::new("phone", "Phone", 390.0, 844.0)];
        let story = LayoutStory::new("player", "Player", root, &scenarios);
        let stories = [story];
        let catalog = LayoutStoryCatalog::new(&stories);

        let manifest =
            VisualRegressionManifest::from_catalog(&catalog, &[VisualColorScheme::HighContrast]);
        let json = serde_json::to_string(&manifest).unwrap();

        assert!(json.contains("player__phone__high_contrast"));
        assert!(json.contains("\"color_scheme\":\"high_contrast\""));
        assert!(json.contains("\"output_path\""));
    }
}
