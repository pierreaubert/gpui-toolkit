//! Focused safety audit report for the AU platform boundary.

pub const AU_SAFETY_REPORT_SCHEMA_VERSION: u32 = 1;
pub const AU_SAFETY_REPORT_TYPE: &str = "gpui-au-safety-audit";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuSafetyStatus {
    Audited,
    HostValidationRequired,
    FollowUpRequired,
}

impl AuSafetyStatus {
    pub const fn label(self) -> &'static str {
        match self {
            AuSafetyStatus::Audited => "audited",
            AuSafetyStatus::HostValidationRequired => "host-validation-required",
            AuSafetyStatus::FollowUpRequired => "follow-up-required",
        }
    }

    pub const fn is_release_blocking(self) -> bool {
        !matches!(self, AuSafetyStatus::Audited)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuSafetyBoundary {
    pub id: &'static str,
    pub area: &'static str,
    pub source: &'static str,
    pub invariant: &'static str,
    pub current_gate: &'static str,
    pub status: AuSafetyStatus,
}

impl AuSafetyBoundary {
    pub const fn is_release_blocking(&self) -> bool {
        self.status.is_release_blocking()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuSafetyReport {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub crate_name: &'static str,
    pub crate_version: &'static str,
    pub boundaries: &'static [AuSafetyBoundary],
}

impl AuSafetyReport {
    pub fn blocking_entries(&self) -> Vec<&'static AuSafetyBoundary> {
        self.boundaries
            .iter()
            .filter(|boundary| boundary.is_release_blocking())
            .collect()
    }

    pub fn to_markdown(&self) -> String {
        let mut markdown = String::new();
        markdown.push_str("# gpui-au Safety Audit\n\n");
        markdown.push_str(&format!(
            "- schema_version: {}\n- report_type: `{}`\n- crate: `{}` {}\n- boundaries: {}\n\n",
            self.schema_version,
            self.report_type,
            self.crate_name,
            self.crate_version,
            self.boundaries.len(),
        ));
        markdown.push_str("| id | status | area | source | invariant | current gate |\n");
        markdown.push_str("| --- | --- | --- | --- | --- | --- |\n");
        for boundary in self.boundaries {
            markdown.push_str(&format!(
                "| `{}` | `{}` | {} | `{}` | {} | {} |\n",
                boundary.id,
                boundary.status.label(),
                boundary.area,
                boundary.source,
                boundary.invariant,
                boundary.current_gate,
            ));
        }
        markdown
    }
}

pub const AU_SAFETY_BOUNDARIES: &[AuSafetyBoundary] = &[
    AuSafetyBoundary {
        id: "ffi-entrypoint-null-contract",
        area: "C ABI null and invalid-context handling",
        source: "ffi.rs",
        invariant: "Every exported lifecycle, rendering, input, and text entry point returns safely when the host passes a null context or optional pointer.",
        current_gate: "Covered by `exported_host_entry_points_are_null_safe`; keep the C header syntax check in CI.",
        status: AuSafetyStatus::Audited,
    },
    AuSafetyBoundary {
        id: "host-nsview-raw-window",
        area: "Host-provided NSView raw-window handle",
        source: "window/au_raw_window.rs",
        invariant: "The AUViewController owns the NSView for the duration of renderer surface creation and all callbacks run on the main thread.",
        current_gate: "Compiled by `cargo check -p gpui-au --all-targets`; still requires AU host smoke validation.",
        status: AuSafetyStatus::HostValidationRequired,
    },
    AuSafetyBoundary {
        id: "global-au-window-pointer",
        area: "Global AuWindow pointer used by render/input FFI",
        source: "window/au_window_ptr.rs",
        invariant: "The pointer is registered after boxing, cleared during destroy, and dereferenced only through `with_au_window()` after the main-thread assertion.",
        current_gate: "Covered by the explicit lifetime invariant and destroy path; host validation must still exercise destroy/recreate.",
        status: AuSafetyStatus::HostValidationRequired,
    },
    AuSafetyBoundary {
        id: "ffi-context-lifecycle",
        area: "C ABI create/destroy context ownership",
        source: "ffi.rs",
        invariant: "`gpui_au_create` returns a boxed `AuContext`, null-checks FFI inputs, keeps GPUI's AppCell alive, and `gpui_au_destroy` drops exactly that box.",
        current_gate: "Root-view and context helpers are unit-tested; AU host validation must cover repeated create/destroy.",
        status: AuSafetyStatus::HostValidationRequired,
    },
    AuSafetyBoundary {
        id: "dispatcher-trampoline",
        area: "GCD RunnableVariant trampoline",
        source: "dispatcher.rs",
        invariant: "RunnableVariant is converted to a raw pointer exactly once and reconstructed by the dispatch trampoline before running.",
        current_gate: "Dispatcher tests cover overflow handling and realtime spawn behavior.",
        status: AuSafetyStatus::Audited,
    },
    AuSafetyBoundary {
        id: "parameter-tree-realtime-safety",
        area: "AUParameterTree lock-free value path",
        source: "params.rs",
        invariant: "Parameter values are atomics, so audio-thread get/set never locks. Observer fan-out runs on the setter thread and must not re-enter the tree.",
        current_gate: "Covered by `params::tests` (clamp, NaN, duplicate/unknown ids, observer tokens); Swift must still validate observer threading.",
        status: AuSafetyStatus::Audited,
    },
    AuSafetyBoundary {
        id: "fullstate-roundtrip-persistence",
        area: "fullState save/load byte contract",
        source: "params.rs",
        invariant: "`gpui_au_save_state`/`gpui_au_load_state` round-trip every registered id; decode rejects bad magic/version/truncation and ignores unknown ids.",
        current_gate: "Covered by `params::tests` state round-trip and rejection cases plus the FFI null-safety test.",
        status: AuSafetyStatus::Audited,
    },
    AuSafetyBoundary {
        id: "renderer-lazy-init-realtime",
        area: "Deferred wgpu construction and lock-free draw/drop",
        source: "window/au_window.rs",
        invariant: "`AuWindow::new` never blocks on wgpu; `draw` clones the renderer handle under a short lock and drops contended frames via `try_lock` plus drop/coalesce counters.",
        current_gate: "Unit-tested counters, debounce, and throttle; AU host smoke validation must still cover first-frame init and resize-drag.",
        status: AuSafetyStatus::HostValidationRequired,
    },
    AuSafetyBoundary {
        id: "nslog-sandbox-hygiene",
        area: "NSLog gating in release builds",
        source: "helpers.rs",
        invariant: "Progress/tracing logs go through `nslog_verbose` (debug builds or the `verbose-logging` feature only); release builds emit genuine failures only.",
        current_gate: "Covered by `verbose_logging_helper_links`; host Console.app must stay quiet during normal playback.",
        status: AuSafetyStatus::Audited,
    },
    AuSafetyBoundary {
        id: "coretext-rasterization",
        area: "CoreText/CoreGraphics glyph rasterization",
        source: "text_system/",
        invariant: "Core Foundation references and glyph buffers stay inside the text-system ownership boundary and are exercised by the text-system test suite.",
        current_gate: "Covered by `cargo test -p gpui-au text_system --lib`; keep in AU host visual QA.",
        status: AuSafetyStatus::Audited,
    },
];

pub const fn au_safety_report() -> AuSafetyReport {
    AuSafetyReport {
        schema_version: AU_SAFETY_REPORT_SCHEMA_VERSION,
        report_type: AU_SAFETY_REPORT_TYPE,
        crate_name: env!("CARGO_PKG_NAME"),
        crate_version: env!("CARGO_PKG_VERSION"),
        boundaries: AU_SAFETY_BOUNDARIES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn au_safety_report_has_stable_contract() {
        let report = au_safety_report();

        assert_eq!(report.schema_version, AU_SAFETY_REPORT_SCHEMA_VERSION);
        assert_eq!(report.report_type, AU_SAFETY_REPORT_TYPE);
        assert_eq!(report.crate_name, "gpui-au");
        assert!(report.boundaries.len() >= 10);
    }

    #[test]
    fn au_safety_report_has_unique_boundary_ids() {
        let report = au_safety_report();
        let ids = report
            .boundaries
            .iter()
            .map(|boundary| boundary.id)
            .collect::<HashSet<_>>();

        assert_eq!(ids.len(), report.boundaries.len());
    }

    #[test]
    fn au_safety_report_covers_required_unsafe_boundaries() {
        let report = au_safety_report();
        let ids = report
            .boundaries
            .iter()
            .map(|boundary| boundary.id)
            .collect::<HashSet<_>>();

        assert!(ids.contains("host-nsview-raw-window"));
        assert!(ids.contains("ffi-entrypoint-null-contract"));
        assert!(ids.contains("global-au-window-pointer"));
        assert!(ids.contains("ffi-context-lifecycle"));
        assert!(ids.contains("dispatcher-trampoline"));
        assert!(ids.contains("coretext-rasterization"));
        assert!(ids.contains("parameter-tree-realtime-safety"));
        assert!(ids.contains("fullstate-roundtrip-persistence"));
        assert!(ids.contains("renderer-lazy-init-realtime"));
        assert!(ids.contains("nslog-sandbox-hygiene"));
    }

    #[test]
    fn au_safety_report_blocks_release_until_host_validation() {
        let blocking = au_safety_report().blocking_entries();

        assert!(
            blocking
                .iter()
                .any(|boundary| boundary.id == "host-nsview-raw-window")
        );
        assert!(
            blocking
                .iter()
                .any(|boundary| boundary.id == "global-au-window-pointer")
        );
        assert!(
            blocking
                .iter()
                .all(|boundary| boundary.status.is_release_blocking())
        );
    }

    #[test]
    fn au_safety_report_markdown_names_invariants_and_gates() {
        let markdown = au_safety_report().to_markdown();

        assert!(markdown.contains(AU_SAFETY_REPORT_TYPE));
        assert!(markdown.contains("host-nsview-raw-window"));
        assert!(markdown.contains("global-au-window-pointer"));
        assert!(markdown.contains("AUViewController"));
        assert!(markdown.contains("cargo check -p gpui-au --all-targets"));
    }
}
