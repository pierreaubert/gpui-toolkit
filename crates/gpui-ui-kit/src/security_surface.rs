//! Security-sensitive dependency and task boundaries for release QA.

/// Schema version for [`SecuritySurfaceReport`].
pub const SECURITY_SURFACE_SCHEMA_VERSION: u32 = 1;

/// Stable report type identifier for [`SecuritySurfaceReport`].
pub const SECURITY_SURFACE_REPORT_TYPE: &str = "gpui-ui-kit-security-surface";

/// Release-readiness status for a security-relevant UI-kit surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecuritySurfaceStatus {
    /// The surface is always compiled and has a documented in-process boundary.
    InProcessDocumented,
    /// The surface is opt-in through a Cargo feature or example gate.
    FeatureGated,
    /// The surface uses a weak-entity scoped async loop with a documented stop condition.
    ScopedAsyncTask,
    /// The surface is owned by the host app or operating-system permission layer.
    HostPermissionRequired,
}

impl SecuritySurfaceStatus {
    /// Stable status label for release reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InProcessDocumented => "in-process-documented",
            Self::FeatureGated => "feature-gated",
            Self::ScopedAsyncTask => "scoped-async-task",
            Self::HostPermissionRequired => "host-permission-required",
        }
    }

    /// Whether this status is enough for a UI-kit security-surface claim.
    pub const fn is_release_ready(self) -> bool {
        matches!(
            self,
            Self::InProcessDocumented
                | Self::FeatureGated
                | Self::ScopedAsyncTask
                | Self::HostPermissionRequired
        )
    }
}

/// One dependency or task surface tracked by release QA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecuritySurfaceEntry {
    /// Stable surface id.
    pub id: &'static str,
    /// Human-readable surface name.
    pub surface: &'static str,
    /// Current release status.
    pub status: SecuritySurfaceStatus,
    /// Relevant dependency, feature, or module.
    pub dependency_or_module: &'static str,
    /// Boundary evidence recorded for release notes.
    pub evidence: &'static str,
    /// Requirement before claiming this surface is release-ready.
    pub release_requirement: &'static str,
}

/// Versioned security-surface report for release QA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecuritySurfaceReport {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub reviewed_on: &'static str,
    pub entries: &'static [SecuritySurfaceEntry],
}

impl SecuritySurfaceReport {
    /// Return true only when every security surface has a documented boundary.
    pub fn all_release_ready(self) -> bool {
        self.entries
            .iter()
            .all(|entry| entry.status.is_release_ready())
    }

    /// Return entries that still block a documented security-surface claim.
    pub fn blocking_entries(self) -> impl Iterator<Item = &'static SecuritySurfaceEntry> {
        self.entries
            .iter()
            .filter(|entry| !entry.status.is_release_ready())
    }

    /// Render the report as Markdown for release notes.
    pub fn to_markdown_table(self) -> String {
        let mut markdown = format!(
            "# GPUI UI Kit Security Surface\n\n\
             - schema_version: {}\n\
             - report_type: `{}`\n\
             - reviewed_on: {}\n\n\
             | Surface | Status | Dependency/module | Evidence | Release requirement |\n\
             | --- | --- | --- | --- | --- |\n",
            self.schema_version, self.report_type, self.reviewed_on
        );

        for entry in self.entries {
            markdown.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                entry.surface,
                entry.status.as_str(),
                entry.dependency_or_module,
                entry.evidence,
                entry.release_requirement
            ));
        }

        markdown
    }
}

const SECURITY_SURFACE_ENTRIES: &[SecuritySurfaceEntry] = &[
    SecuritySurfaceEntry {
        id: "qr-encoding",
        surface: "QR code generation",
        status: SecuritySurfaceStatus::InProcessDocumented,
        dependency_or_module: "qrcode; gpui_ui_kit::qr::{QrCode, AnimatedQrCode}",
        evidence: "QR components encode caller-provided bytes in memory, render from the generated matrix, and perform no camera, file-system, network, or clipboard I/O.",
        release_requirement: "Keep QR rendering tests green and document app-level size/rate limits for untrusted payloads.",
    },
    SecuritySurfaceEntry {
        id: "animated-qr-task",
        surface: "Animated QR repaint loop",
        status: SecuritySurfaceStatus::ScopedAsyncTask,
        dependency_or_module: "AnimatedQrCode::new; smol::Timer (native) / BackgroundExecutor::timer (wasm); Context::spawn",
        evidence: "The detached loop holds only a WeakEntity, wakes every 33 ms while the entity is alive, calls notify, and exits when the entity update fails.",
        release_requirement: "Keep the loop weak-entity scoped and avoid background I/O or unbounded work inside the animation task.",
    },
    SecuritySurfaceEntry {
        id: "swipe-panel-task",
        surface: "SwipePanel spring animation loop",
        status: SecuritySurfaceStatus::ScopedAsyncTask,
        dependency_or_module: "SwipePanel::ensure_animation; smol::Timer (native) / BackgroundExecutor::timer (wasm); Context::spawn",
        evidence: "The detached loop holds only a WeakEntity, steps spring state every 16 ms, exits when the entity is gone, and stops when the animating flag becomes false.",
        release_requirement: "Keep animation work bounded to UI state updates and preserve the stop condition when changing SwipePanel animation.",
    },
];

/// Return the current security-surface report.
pub const fn security_surface_report() -> SecuritySurfaceReport {
    SecuritySurfaceReport {
        schema_version: SECURITY_SURFACE_SCHEMA_VERSION,
        report_type: SECURITY_SURFACE_REPORT_TYPE,
        reviewed_on: "2026-07-08",
        entries: SECURITY_SURFACE_ENTRIES,
    }
}

/// Return all security-surface entries.
pub const fn security_surface_entries() -> &'static [SecuritySurfaceEntry] {
    SECURITY_SURFACE_ENTRIES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_surface_report_has_stable_contract() {
        let report = security_surface_report();

        assert_eq!(report.schema_version, SECURITY_SURFACE_SCHEMA_VERSION);
        assert_eq!(report.report_type, SECURITY_SURFACE_REPORT_TYPE);
        assert_eq!(report.reviewed_on, "2026-07-08");
        assert!(!report.entries.is_empty());
        assert!(report.all_release_ready());
        assert_eq!(report.blocking_entries().count(), 0);
    }

    #[test]
    fn security_surface_report_has_unique_ids() {
        let mut ids = std::collections::BTreeSet::new();

        for entry in security_surface_entries() {
            assert!(
                ids.insert(entry.id),
                "duplicate security surface {}",
                entry.id
            );
            assert!(!entry.surface.is_empty());
            assert!(!entry.status.as_str().is_empty());
            assert!(!entry.dependency_or_module.is_empty());
            assert!(!entry.evidence.is_empty());
            assert!(!entry.release_requirement.is_empty());
        }
    }

    #[test]
    fn security_surface_report_names_qr_and_async_boundaries() {
        let ids = security_surface_entries()
            .iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>();

        assert!(ids.contains(&"qr-encoding"));
        assert!(ids.contains(&"animated-qr-task"));
        assert!(ids.contains(&"swipe-panel-task"));
    }

    #[test]
    fn security_surface_report_keeps_camera_capture_out_of_ui_kit() {
        let markdown = security_surface_report().to_markdown_table();
        assert!(!markdown.contains("nokhwa"));
        assert!(!markdown.contains("Camera operating-system permission flow"));
    }

    #[test]
    fn security_surface_markdown_names_statuses() {
        let markdown = security_surface_report().to_markdown_table();

        assert!(markdown.contains(SECURITY_SURFACE_REPORT_TYPE));
        assert!(markdown.contains("QR code generation"));
        assert!(markdown.contains("scoped-async-task"));
    }
}
