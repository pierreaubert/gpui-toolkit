//! Read-only health checks with an exit-code contract.
//!
//! [`run_doctor`] aggregates checks over a directory: `toolkit.toml`
//! (when present) parses strictly, the component catalog builds,
//! every exported story resolves, agent docs carry the managed catalog
//! block, and the checkout root is detectable. Each check reports
//! `[ok]`, `[warn]`, `[fail]`, or `[info]` with an actionable `fix`
//! when one exists. The exit code is the contract:
//! [`DoctorReport::failed`] is true when any check fails, and the CLI
//! exits 1 in that case, so `gpui-toolkit doctor` works as a raw CI
//! step. Warnings never fail.

// Rust guideline compliant 2026-02-21

use crate::config::{discover_config, read_config_file};
use gpui_component_lab::{
    UI_KIT_EXPORTED_COMPONENT_STORIES, UI_KIT_SHOWCASE_STORIES, builtin_story_registry,
};
use serde::Serialize;
use std::path::Path;

/// Discriminator for doctor payloads in JSON envelopes.
pub const DOCTOR_TYPE: &str = "doctor";

/// Health-check outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorStatus {
    /// Check passed.
    Ok,
    /// Non-blocking concern with a suggested fix.
    Warn,
    /// Blocking failure; fails the report.
    Fail,
    /// Informational; neither pass nor fail.
    Info,
}

impl DoctorStatus {
    /// Canonical wire name of this status.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use gpui_toolkit_cli::DoctorStatus;
    ///
    /// assert_eq!(DoctorStatus::Fail.as_str(), "fail");
    /// ```
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warn => "warn",
            Self::Fail => "fail",
            Self::Info => "info",
        }
    }
}

impl serde::Serialize for DoctorStatus {
    /// Serializes the status as its wire name.
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// One health-check result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorCheck {
    /// Stable check id, such as `catalog`.
    pub id: String,
    /// Outcome.
    pub status: DoctorStatus,
    /// Short label.
    pub label: String,
    /// Human-readable detail.
    pub message: String,
    /// Actionable fix, omitted when none exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
}

/// Outcome counts across a report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorSummary {
    /// Passed checks.
    pub pass: usize,
    /// Warnings.
    pub warn: usize,
    /// Failures.
    pub fail: usize,
    /// Informational results.
    pub info: usize,
}

/// Full doctor payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorReport {
    /// Checks in run order.
    pub checks: Vec<DoctorCheck>,
    /// Outcome counts.
    pub summary: DoctorSummary,
}

impl DoctorReport {
    /// Whether any check failed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use gpui_toolkit_cli::{DoctorReport, DoctorSummary};
    ///
    /// let report = DoctorReport {
    ///     checks: Vec::new(),
    ///     summary: DoctorSummary { pass: 1, warn: 0, fail: 0, info: 0 },
    /// };
    /// assert!(!report.failed());
    /// ```
    pub fn failed(&self) -> bool {
        self.summary.fail > 0
    }
}

/// Runs every health check over `dir`.
///
/// Infallible: failures record as [`DoctorStatus::Fail`] checks rather
/// than errors, so callers always get the full report.
///
/// # Examples
///
/// ```rust
/// use gpui_toolkit_cli::run_doctor;
///
/// let report = run_doctor(std::path::Path::new("."));
/// assert!(!report.checks.is_empty());
/// ```
pub fn run_doctor(dir: &Path) -> DoctorReport {
    let mut checks = Vec::with_capacity(5);
    checks.push(config_check(dir));
    let registry = builtin_story_registry();
    match &registry {
        Ok(registry) if registry.is_empty() => checks.push(DoctorCheck {
            id: String::from("catalog"),
            status: DoctorStatus::Fail,
            label: String::from("Component catalog"),
            message: String::from("the story registry built but is empty"),
            fix: Some(String::from(
                "register stories in gpui-component-lab/src/lib/register.rs",
            )),
        }),
        Ok(registry) => checks.push(DoctorCheck {
            id: String::from("catalog"),
            status: DoctorStatus::Ok,
            label: String::from("Component catalog"),
            message: format!("{} stories registered", registry.len()),
            fix: None,
        }),
        Err(error) => checks.push(DoctorCheck {
            id: String::from("catalog"),
            status: DoctorStatus::Fail,
            label: String::from("Component catalog"),
            message: format!("the story registry failed to build: {error}"),
            fix: None,
        }),
    }
    match &registry {
        Ok(registry) => {
            let missing: Vec<&str> = UI_KIT_EXPORTED_COMPONENT_STORIES
                .iter()
                .chain(UI_KIT_SHOWCASE_STORIES.iter())
                .map(|(id, _, _)| *id)
                .filter(|id| registry.story(id).is_none())
                .collect();
            if missing.is_empty() {
                let total = UI_KIT_EXPORTED_COMPONENT_STORIES.len() + UI_KIT_SHOWCASE_STORIES.len();
                checks.push(DoctorCheck {
                    id: String::from("story-coverage"),
                    status: DoctorStatus::Ok,
                    label: String::from("Exported story coverage"),
                    message: format!("all {total} exported stories resolve"),
                    fix: None,
                });
            } else {
                checks.push(DoctorCheck {
                    id: String::from("story-coverage"),
                    status: DoctorStatus::Fail,
                    label: String::from("Exported story coverage"),
                    message: format!(
                        "{} exported stories are missing: {}",
                        missing.len(),
                        missing.join(", ")
                    ),
                    fix: Some(String::from(
                        "register stories in gpui-component-lab/src/lib/register.rs",
                    )),
                });
            }
        }
        Err(_) => checks.push(DoctorCheck {
            id: String::from("story-coverage"),
            status: DoctorStatus::Fail,
            label: String::from("Exported story coverage"),
            message: String::from("cannot check coverage: the catalog failed to build"),
            fix: None,
        }),
    }
    let agents = dir.join("AGENTS.md");
    match std::fs::read_to_string(&agents) {
        Ok(content) if content.contains(crate::CATALOG_BEGIN) => checks.push(DoctorCheck {
            id: String::from("agent-docs"),
            status: DoctorStatus::Ok,
            label: String::from("Agent docs"),
            message: format!("managed catalog block present in {}", agents.display()),
            fix: None,
        }),
        Ok(_) => checks.push(DoctorCheck {
            id: String::from("agent-docs"),
            status: DoctorStatus::Warn,
            label: String::from("Agent docs"),
            message: format!("{} has no managed catalog block", agents.display()),
            fix: Some(format!("run `gpui-toolkit init --dir {}`", dir.display())),
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            checks.push(DoctorCheck {
                id: String::from("agent-docs"),
                status: DoctorStatus::Info,
                label: String::from("Agent docs"),
                message: format!("no {} in {}", agents.display(), dir.display()),
                fix: None,
            });
        }
        Err(error) => checks.push(DoctorCheck {
            id: String::from("agent-docs"),
            status: DoctorStatus::Warn,
            label: String::from("Agent docs"),
            message: format!("cannot read {}: {error}", agents.display()),
            fix: None,
        }),
    }
    if dir.join("crates/gpui-ui-kit").is_dir() {
        checks.push(DoctorCheck {
            id: String::from("checkout-root"),
            status: DoctorStatus::Ok,
            label: String::from("Toolkit checkout"),
            message: format!("toolkit checkout at {}", dir.display()),
            fix: None,
        });
    } else {
        checks.push(DoctorCheck {
            id: String::from("checkout-root"),
            status: DoctorStatus::Info,
            label: String::from("Toolkit checkout"),
            message: String::from(
                "not a toolkit checkout; eject resolves sources via GPUI_TOOLKIT_ROOT",
            ),
            fix: None,
        });
    }
    let mut summary = DoctorSummary {
        pass: 0,
        warn: 0,
        fail: 0,
        info: 0,
    };
    for check in &checks {
        match check.status {
            DoctorStatus::Ok => summary.pass += 1,
            DoctorStatus::Warn => summary.warn += 1,
            DoctorStatus::Fail => summary.fail += 1,
            DoctorStatus::Info => summary.info += 1,
        }
    }
    DoctorReport { checks, summary }
}

/// Checks `toolkit.toml` discovery and strict parsing.
///
/// Missing config is informational (built-ins apply); an unparsable or
/// invalid config fails, since silently ignoring project settings is
/// worse than stopping.
fn config_check(dir: &Path) -> DoctorCheck {
    let id = String::from("config");
    let label = String::from("Project config");
    match discover_config(dir) {
        None => DoctorCheck {
            id,
            status: DoctorStatus::Info,
            label,
            message: format!(
                "no toolkit.toml above {}; built-in defaults apply",
                dir.display()
            ),
            fix: None,
        },
        Some(path) => match read_config_file(&path) {
            Ok(loaded) => match loaded.customs() {
                Ok(customs) => DoctorCheck {
                    id,
                    status: DoctorStatus::Ok,
                    label,
                    message: format!(
                        "{} valid ({} custom layout components)",
                        path.display(),
                        customs.len()
                    ),
                    fix: None,
                },
                Err(error) => DoctorCheck {
                    id,
                    status: DoctorStatus::Fail,
                    label,
                    message: format!("{}: {}", path.display(), error.message),
                    fix: Some(format!("fix the reported error in {}", path.display())),
                },
            },
            Err(error) => DoctorCheck {
                id,
                status: DoctorStatus::Fail,
                label,
                message: format!("{}: {}", path.display(), error.message),
                fix: Some(format!("fix the reported error in {}", path.display())),
            },
        },
    }
}

/// Renders a report as human-readable text.
///
/// Dense output keeps one line per check plus the counts; the default
/// rendering adds fixes and a closing verdict.
pub fn render_doctor_text(report: &DoctorReport, dense: bool) -> String {
    let mut text = String::new();
    for check in &report.checks {
        if dense {
            text.push_str(&format!(
                "{} {} {}\n",
                check.status.as_str(),
                check.id,
                check.message
            ));
        } else {
            text.push_str(&format!(
                "[{}] {} — {}: {}\n",
                check.status.as_str(),
                check.id,
                check.label,
                check.message
            ));
            if let Some(fix) = &check.fix {
                text.push_str(&format!("  fix: {fix}\n"));
            }
        }
    }
    if dense {
        text.push_str(&format!(
            "pass={} warn={} fail={} info={}\n",
            report.summary.pass, report.summary.warn, report.summary.fail, report.summary.info
        ));
    } else {
        text.push_str(&format!(
            "\npass: {}\nwarn: {}\nfail: {}\ninfo: {}\n",
            report.summary.pass, report.summary.warn, report.summary.fail, report.summary.info
        ));
        if report.failed() {
            text.push_str("\nFailures need attention before this tree is trusted.\n");
        } else {
            text.push_str("\nNo failures.\n");
        }
    }
    text
}
