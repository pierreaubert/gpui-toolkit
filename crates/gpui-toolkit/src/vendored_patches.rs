//! Vendored dependency patch manifest for release QA.

/// Schema version for [`VendoredPatchManifest`].
pub const VENDORED_PATCH_SCHEMA_VERSION: u32 = 1;

/// Stable report type identifier for [`VendoredPatchManifest`].
pub const VENDORED_PATCH_REPORT_TYPE: &str = "gpui-toolkit-vendored-patches";

/// Whether a vendored directory is currently active in dependency resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VendoredPatchStatus {
    /// The root manifest actively patches dependency resolution to this path.
    ActivePatch,
    /// The directory is retained locally but the lockfile/root manifest does
    /// not currently resolve through it.
    InactiveSnapshot,
}

impl VendoredPatchStatus {
    /// Stable status label for release reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ActivePatch => "active-patch",
            Self::InactiveSnapshot => "inactive-snapshot",
        }
    }
}

/// How a vendored directory is maintained relative to its upstream source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VendoredPatchMaintenance {
    /// History-free snapshot imported by `scripts/import_gpui_upstream.py`;
    /// provenance and local patches live in the crate's `VENDORED.md`.
    ScriptVendored,
    /// Hand-maintained against upstream; upgrade notes live in the crate's
    /// `VENDORING.md`.
    HandMaintained,
}

impl VendoredPatchMaintenance {
    /// Stable maintenance label for release reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ScriptVendored => "script-vendored",
            Self::HandMaintained => "hand-maintained",
        }
    }
}

/// One vendored dependency entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VendoredPatch {
    /// Crate/package name.
    pub name: &'static str,
    /// Local vendored directory relative to the repository root.
    pub local_path: &'static str,
    /// Upstream project URL.
    pub upstream: &'static str,
    /// Upstream package path, crate name, tag, rev, or registry version.
    pub upstream_base: &'static str,
    /// Team responsible for reviewing and removing the patch.
    pub owner: &'static str,
    /// ISO-8601 date of the most recent upstream-delta review.
    pub last_reviewed: &'static str,
    /// Maximum number of days between upstream-delta reviews.
    pub review_cadence_days: u16,
    /// Observable condition under which this local copy must be removed.
    pub removal_condition: &'static str,
    /// Reproducible command used to inspect the local/upstream delta.
    pub delta_evidence_command: &'static str,
    /// Current local version/ref.
    pub local_ref: &'static str,
    /// Current dependency-resolution status.
    pub status: VendoredPatchStatus,
    /// Whether the copy is script-vendored or hand-maintained.
    pub maintenance: VendoredPatchMaintenance,
    /// Why the copy is retained locally.
    pub reason: &'static str,
    /// Local changes that must be re-evaluated during upgrades.
    pub retained_changes: &'static [&'static str],
    /// Required verification before this patch can move to a release.
    pub verification_gate: &'static str,
    /// Human- or script-maintained provenance file (`VENDORING.md` for
    /// hand-maintained crates, `VENDORED.md` for script-vendored crates).
    pub vendoring_doc: &'static str,
}

/// Versioned vendored patch manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VendoredPatchManifest {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub patches: &'static [VendoredPatch],
}

impl VendoredPatchManifest {
    /// Return active patches only.
    pub fn active_patches(self) -> impl Iterator<Item = &'static VendoredPatch> {
        self.patches
            .iter()
            .filter(|patch| patch.status == VendoredPatchStatus::ActivePatch)
    }

    /// Render the manifest as Markdown for release notes or QA reports.
    pub fn to_markdown_table(self) -> String {
        let mut markdown = format!(
            "# GPUI Toolkit Vendored Patch Manifest\n\n\
             - schema_version: {}\n\
             - report_type: `{}`\n\n\
             | Name | Status | Maintenance | Upstream base | Local path | Reason | Retained changes | Verification |\n\
             | --- | --- | --- | --- | --- | --- | --- | --- |\n",
            self.schema_version, self.report_type
        );

        for patch in self.patches {
            markdown.push_str(&format!(
                "| {} | {} | {} | {} | `{}` | {} | {} | {} |\n",
                patch.name,
                patch.status.as_str(),
                patch.maintenance.as_str(),
                patch.upstream_base,
                patch.local_path,
                patch.reason,
                patch.retained_changes.join("<br>"),
                patch.verification_gate
            ));
        }

        markdown
    }
}

const VENDORED_PATCHES: &[VendoredPatch] = &[
    VendoredPatch {
        name: "block",
        local_path: "crates/3rdparties/block",
        upstream: "https://github.com/SSheldon/rust-block",
        upstream_base: "block 0.1.6",
        owner: "platform-maintainers",
        last_reviewed: "2026-07-12",
        review_cadence_days: 90,
        removal_condition: "Remove when upstream block supports the workspace Rust toolchain without the retained changes.",
        delta_evidence_command: "git diff block-0.1.6 -- crates/3rdparties/block",
        local_ref: "0.1.6 active local patch",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::HandMaintained,
        reason: "Objective-C block runtime binding patch point for current Rust compatibility.",
        retained_changes: &[
            "Represent _NSConcreteStackBlock as an opaque byte symbol and take its address with addr_of!.",
            "Avoid the Rust future-incompatibility warning for uninhabited extern statics.",
            "Use explicit extern \"C\" ABI spellings for block invoke function pointers.",
            "Remove the unusable packaged objc_test_utils dev-dependency so the vendored manifest checks directly.",
            "Declare edition 2015 explicitly to keep the upstream default edition warning-free.",
        ],
        verification_gate: "cargo check --manifest-path crates/3rdparties/block/Cargo.toml --lib and cargo report future-incompatibilities after a dependent build.",
        vendoring_doc: "crates/3rdparties/block/VENDORING.md",
    },
    VendoredPatch {
        name: "collections",
        local_path: "crates/3rdparties/collections",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/collections at Zed v1.9.0",
        owner: "platform-maintainers",
        last_reviewed: "2026-07-18",
        review_cadence_days: 90,
        removal_condition: "Remove when the GPUI closure builds without a local patch-table override for this crate.",
        delta_evidence_command: "git diff v1.9.0 -- crates/3rdparties/collections",
        local_ref: "0.1.0 script-vendored at Zed v1.9.0",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "Collection types used across the GPUI closure.",
        retained_changes: &[
            "No local patches; pristine v1.9.0 snapshot modulo the import exclusions recorded in VENDORED.md.",
        ],
        verification_gate: "cargo check -p collections and just lint-host.",
        vendoring_doc: "crates/3rdparties/collections/VENDORED.md",
    },
    VendoredPatch {
        name: "derive_refineable",
        local_path: "crates/3rdparties/derive_refineable",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/refineable/derive_refineable at Zed v1.9.0",
        owner: "platform-maintainers",
        last_reviewed: "2026-07-18",
        review_cadence_days: 90,
        removal_condition: "Remove when the GPUI closure builds without a local patch-table override for this crate.",
        delta_evidence_command: "git diff v1.9.0 -- crates/3rdparties/derive_refineable",
        local_ref: "0.1.0 script-vendored at Zed v1.9.0",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "Derive macro for refineable.",
        retained_changes: &[
            "No local patches; pristine v1.9.0 snapshot modulo the import exclusions recorded in VENDORED.md.",
        ],
        verification_gate: "cargo check -p derive_refineable and just lint-host.",
        vendoring_doc: "crates/3rdparties/derive_refineable/VENDORED.md",
    },
    VendoredPatch {
        name: "gpui",
        local_path: "crates/3rdparties/gpui",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/gpui at Zed v1.9.0",
        owner: "gpui-platform-maintainers",
        last_reviewed: "2026-07-18",
        review_cadence_days: 90,
        removal_condition: "Remove when the workspace tracks upstream gpui without local patch-table overrides.",
        delta_evidence_command: "git diff v1.9.0 -- crates/3rdparties/gpui",
        local_ref: "0.2.2 script-vendored at Zed v1.9.0",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "Core GPUI UI framework snapshot.",
        retained_changes: &[
            "Restored IBMPlexSans-Regular.ttf and Lilex-Regular.ttf under crates/assets/fonts/ so the svg_renderer include_bytes! paths resolve from the deeper vendored layout.",
            "Crate-root lint allows in src/gpui.rs for default clippy/rustc lints that fire on unmodified upstream code (inventory in VENDORED.md).",
            "Restricted image crate features to GPUI's advertised bitmap formats, excluding unrelated codecs and Rayon.",
        ],
        verification_gate: "cargo check -p gpui and just lint-host; run the gpui svg_renderer font tests when renderer or font code changes.",
        vendoring_doc: "crates/3rdparties/gpui/VENDORED.md",
    },
    VendoredPatch {
        name: "gpui_linux",
        local_path: "crates/3rdparties/gpui_linux",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/gpui_linux at Zed v1.9.0",
        owner: "linux-platform-maintainers",
        last_reviewed: "2026-07-18",
        review_cadence_days: 90,
        removal_condition: "Remove when the GPUI closure builds without a local patch-table override for this crate.",
        delta_evidence_command: "git diff v1.9.0 -- crates/3rdparties/gpui_linux",
        local_ref: "0.1.0 script-vendored at Zed v1.9.0",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "Linux platform backend for gpui.",
        retained_changes: &[
            "Standalone gpui dependency preserves Zed's default-features=false workspace policy; gpui-miniapp explicitly selects Wayland and X11.",
            "Restricted image crate features to GPUI's advertised bitmap formats.",
        ],
        verification_gate: "cargo check -p gpui_linux and just lint-host.",
        vendoring_doc: "crates/3rdparties/gpui_linux/VENDORED.md",
    },
    VendoredPatch {
        name: "gpui_macos",
        local_path: "crates/3rdparties/gpui_macos",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/gpui_macos at Zed v1.9.0",
        owner: "apple-platform-maintainers",
        last_reviewed: "2026-07-18",
        review_cadence_days: 90,
        removal_condition: "Remove when upstream gpui_macos drops the private CGS symbol references and the root patch table no longer needs this snapshot.",
        delta_evidence_command: "git diff v1.9.0 -- crates/3rdparties/gpui_macos",
        local_ref: "0.1.0 script-vendored at Zed v1.9.0 plus CGS patch",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "macOS platform backend for gpui; pristine re-vendor plus recorded CGS private-symbol removal (Mac App Store static-analysis rejection risk).",
        retained_changes: &[
            "Removed the private CGS symbols CGSMainConnectionID and CGSSetWindowBackgroundBlurRadius and the pre-Monterey blur branch from src/window.rs (exact regions in VENDORED.md).",
            "Crate-root lint allows in src/gpui_macos.rs for default clippy/rustc lints that fire on unmodified upstream code (inventory in VENDORED.md).",
            "Standalone gpui dependency preserves Zed's default-features=false workspace policy and image features are limited to GPUI's advertised formats.",
        ],
        verification_gate: "cargo check -p gpui_macos, just lint-host, and macOS platform smoke tests for window/appearance changes.",
        vendoring_doc: "crates/3rdparties/gpui_macos/VENDORED.md",
    },
    VendoredPatch {
        name: "gpui_macros",
        local_path: "crates/3rdparties/gpui_macros",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/gpui_macros at Zed v1.9.0",
        owner: "gpui-platform-maintainers",
        last_reviewed: "2026-07-18",
        review_cadence_days: 90,
        removal_condition: "Remove when the GPUI closure builds without a local patch-table override for this crate.",
        delta_evidence_command: "git diff v1.9.0 -- crates/3rdparties/gpui_macros",
        local_ref: "0.1.0 script-vendored at Zed v1.9.0",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "Proc macros for gpui.",
        retained_changes: &[
            "#![allow(unexpected_cfgs)] at the tests/derive_inspector_reflection.rs crate root for the rust_analyzer cfg Zed sets for editor tooling.",
        ],
        verification_gate: "cargo check -p gpui_macros and just lint-host.",
        vendoring_doc: "crates/3rdparties/gpui_macros/VENDORED.md",
    },
    VendoredPatch {
        name: "gpui_shared_string",
        local_path: "crates/3rdparties/gpui_shared_string",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/gpui_shared_string at Zed v1.9.0",
        owner: "platform-maintainers",
        last_reviewed: "2026-07-18",
        review_cadence_days: 90,
        removal_condition: "Remove when the GPUI closure builds without a local patch-table override for this crate.",
        delta_evidence_command: "git diff v1.9.0 -- crates/3rdparties/gpui_shared_string",
        local_ref: "0.1.0 script-vendored at Zed v1.9.0",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "Shared-string type used by gpui text.",
        retained_changes: &[
            "No local patches; pristine v1.9.0 snapshot modulo the import exclusions recorded in VENDORED.md.",
        ],
        verification_gate: "cargo check -p gpui_shared_string and just lint-host.",
        vendoring_doc: "crates/3rdparties/gpui_shared_string/VENDORED.md",
    },
    VendoredPatch {
        name: "gpui_util",
        local_path: "crates/3rdparties/gpui_util",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/gpui_util at Zed v1.9.0",
        owner: "platform-maintainers",
        last_reviewed: "2026-07-18",
        review_cadence_days: 90,
        removal_condition: "Remove when the GPUI closure builds without a local patch-table override for this crate.",
        delta_evidence_command: "git diff v1.9.0 -- crates/3rdparties/gpui_util",
        local_ref: "0.1.0 script-vendored at Zed v1.9.0",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "Utility helpers for gpui.",
        retained_changes: &[
            "No local patches; pristine v1.9.0 snapshot modulo the import exclusions recorded in VENDORED.md.",
        ],
        verification_gate: "cargo check -p gpui_util and just lint-host.",
        vendoring_doc: "crates/3rdparties/gpui_util/VENDORED.md",
    },
    VendoredPatch {
        name: "gpui_wgpu",
        local_path: "crates/3rdparties/gpui_wgpu",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/gpui_wgpu at Zed v1.9.0",
        owner: "rendering-maintainers",
        last_reviewed: "2026-07-12",
        review_cadence_days: 30,
        removal_condition: "Remove when the pinned Zed renderer satisfies workspace dependency and platform requirements unchanged.",
        delta_evidence_command: "git diff v1.9.0 -- crates/3rdparties/gpui_wgpu",
        local_ref: "0.1.0 active local patch",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::HandMaintained,
        reason: "Local GPUI WGPU renderer/backend patch point while tracking the Zed tag.",
        retained_changes: &[
            "Manifest tracks this workspace's Zed v1.9.0 dependency set.",
            "Standalone gpui dependency preserves Zed's default-features=false workspace policy.",
            "zed-font-kit is pinned to rev 110523127440aefb11ce0cf280ae7c5071337ec5 to match the root patch.",
            "Local Rust and WGSL sources differ from the available Zed v1.9.x checkout and require classification plus mobile/AU rendering gates before de-vendoring.",
        ],
        verification_gate: "cargo check -p gpui_wgpu and at least one renderer/showcase smoke pass for rendering changes.",
        vendoring_doc: "crates/3rdparties/gpui_wgpu/VENDORING.md",
    },
    VendoredPatch {
        name: "gpui_windows",
        local_path: "crates/3rdparties/gpui_windows",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/gpui_windows at Zed v1.9.0",
        owner: "windows-platform-maintainers",
        last_reviewed: "2026-07-12",
        review_cadence_days: 30,
        removal_condition: "Remove when the pinned Zed Windows backend contains the retained parity fixes.",
        delta_evidence_command: "git diff v1.9.0 -- crates/3rdparties/gpui_windows",
        local_ref: "0.1.0 active local patch",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::HandMaintained,
        reason: "Local GPUI Windows backend patch point for dependency features and platform parity fixes.",
        retained_changes: &[
            "Manifest uses workspace dependency pins and Windows feature choices.",
            "hide_other_apps and unhide_other_apps intentionally no-op on Windows instead of panicking.",
        ],
        verification_gate: "cargo check -p gpui_windows --target x86_64-pc-windows-msvc plus native Windows smoke tests.",
        vendoring_doc: "crates/3rdparties/gpui_windows/VENDORING.md",
    },
    VendoredPatch {
        name: "http_client",
        local_path: "crates/3rdparties/http_client",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/http_client at Zed v1.9.0",
        owner: "platform-maintainers",
        last_reviewed: "2026-07-18",
        review_cadence_days: 90,
        removal_condition: "Remove when the GPUI closure builds without a local patch-table override for this crate.",
        delta_evidence_command: "git diff v1.9.0 -- crates/3rdparties/http_client",
        local_ref: "0.1.0 script-vendored at Zed v1.9.0",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "HTTP client abstraction used by the GPUI closure.",
        retained_changes: &[
            "#![allow(clippy::new_without_default)] at the src/http_client.rs crate root (BlockedHttpClient::new).",
        ],
        verification_gate: "cargo check -p http_client and just lint-host.",
        vendoring_doc: "crates/3rdparties/http_client/VENDORED.md",
    },
    VendoredPatch {
        name: "mach2",
        local_path: "crates/3rdparties/mach2",
        upstream: "https://github.com/JohnTitor/mach2",
        upstream_base: "registry mach2 0.5.0 snapshot",
        owner: "apple-platform-maintainers",
        last_reviewed: "2026-07-12",
        review_cadence_days: 90,
        removal_condition: "Remove when no planned platform work requires the snapshot.",
        delta_evidence_command: "git diff mach2-0.5.0 -- crates/3rdparties/mach2",
        local_ref: "0.5.0 inactive local snapshot",
        status: VendoredPatchStatus::InactiveSnapshot,
        maintenance: VendoredPatchMaintenance::HandMaintained,
        reason: "Mach kernel bindings snapshot retained for possible platform work.",
        retained_changes: &[
            "Not active in current lockfile resolution.",
            "No release patch stack is currently claimed.",
        ],
        verification_gate: "Remove if unused, or diff and document patch reason before reactivation.",
        vendoring_doc: "crates/3rdparties/mach2/VENDORING.md",
    },
    VendoredPatch {
        name: "media",
        local_path: "crates/3rdparties/media",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/media at Zed v1.9.0",
        owner: "platform-maintainers",
        last_reviewed: "2026-07-18",
        review_cadence_days: 90,
        removal_condition: "Remove when the GPUI closure builds without a local patch-table override for this crate.",
        delta_evidence_command: "git diff v1.9.0 -- crates/3rdparties/media",
        local_ref: "0.1.0 script-vendored at Zed v1.9.0",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "Media and screen-capture types used by the GPUI closure.",
        retained_changes: &[
            "No local patches; pristine v1.9.0 snapshot modulo the import exclusions recorded in VENDORED.md.",
        ],
        verification_gate: "cargo check -p media and just lint-host.",
        vendoring_doc: "crates/3rdparties/media/VENDORED.md",
    },
    VendoredPatch {
        name: "objc",
        local_path: "crates/3rdparties/objc",
        upstream: "https://github.com/SSheldon/rust-objc",
        upstream_base: "objc 0.2.7",
        owner: "apple-platform-maintainers",
        last_reviewed: "2026-07-12",
        review_cadence_days: 90,
        removal_condition: "Remove when upstream objc supports modern Rust and Apple targets without retained changes.",
        delta_evidence_command: "git diff objc-0.2.7 -- crates/3rdparties/objc",
        local_ref: "0.2.7 active local patch",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::HandMaintained,
        reason: "Objective-C runtime binding patch point for modern Rust and Apple backend compatibility.",
        retained_changes: &[
            "Remove the legacy cargo-clippy cfg feature and stale replace_consts cfg_attr from macros.",
            "Use explicit extern \"C\" ABI spellings for runtime and method implementation signatures.",
            "Replace deprecated trim_left_matches and ONCE_INIT usage.",
            "Allow missing docs for the vendored upstream snapshot.",
            "Use addr_of! in msg_send! so nil raw pointers do not create null Rust references before ObjC dispatch.",
        ],
        verification_gate: "cargo check -p objc, cargo test -p objc, and representative Apple/backend dependent checks.",
        vendoring_doc: "crates/3rdparties/objc/VENDORING.md",
    },
    VendoredPatch {
        name: "perf",
        local_path: "crates/3rdparties/perf",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "tooling/perf at Zed v1.9.0",
        owner: "platform-maintainers",
        last_reviewed: "2026-07-18",
        review_cadence_days: 90,
        removal_condition: "Remove when the GPUI closure builds without a local patch-table override for this crate.",
        delta_evidence_command: "git diff v1.9.0 -- crates/3rdparties/perf",
        local_ref: "0.1.0 script-vendored at Zed v1.9.0",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "Profiling helpers used by the GPUI closure.",
        retained_changes: &[
            "No local patches; pristine v1.9.0 snapshot modulo the import exclusions recorded in VENDORED.md.",
        ],
        verification_gate: "cargo check -p perf and just lint-host.",
        vendoring_doc: "crates/3rdparties/perf/VENDORED.md",
    },
    VendoredPatch {
        name: "psm",
        local_path: "crates/3rdparties/psm",
        upstream: "https://github.com/rust-lang/stacker",
        upstream_base: "registry psm 0.1.30 snapshot",
        owner: "platform-maintainers",
        last_reviewed: "2026-07-12",
        review_cadence_days: 90,
        removal_condition: "Remove when no planned stacker integration requires the snapshot.",
        delta_evidence_command: "git diff psm-0.1.30 -- crates/3rdparties/psm",
        local_ref: "0.1.30 inactive local snapshot",
        status: VendoredPatchStatus::InactiveSnapshot,
        maintenance: VendoredPatchMaintenance::HandMaintained,
        reason: "Portable stack manipulation snapshot retained for possible stacker/platform work.",
        retained_changes: &[
            "Not active in current lockfile resolution.",
            "No release patch stack is currently claimed.",
        ],
        verification_gate: "Remove if unused, or diff and document patch reason before reactivation.",
        vendoring_doc: "crates/3rdparties/psm/VENDORING.md",
    },
    VendoredPatch {
        name: "refineable",
        local_path: "crates/3rdparties/refineable",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/refineable at Zed v1.9.0",
        owner: "platform-maintainers",
        last_reviewed: "2026-07-18",
        review_cadence_days: 90,
        removal_condition: "Remove when the GPUI closure builds without a local patch-table override for this crate.",
        delta_evidence_command: "git diff v1.9.0 -- crates/3rdparties/refineable",
        local_ref: "0.1.0 script-vendored at Zed v1.9.0",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "Refinement trait for GPUI style types.",
        retained_changes: &[
            "No local patches; pristine v1.9.0 snapshot modulo the import exclusions recorded in VENDORED.md.",
        ],
        verification_gate: "cargo check -p refineable and just lint-host.",
        vendoring_doc: "crates/3rdparties/refineable/VENDORED.md",
    },
    VendoredPatch {
        name: "scheduler",
        local_path: "crates/3rdparties/scheduler",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/scheduler at Zed v1.9.0",
        owner: "platform-maintainers",
        last_reviewed: "2026-07-18",
        review_cadence_days: 90,
        removal_condition: "Remove when the GPUI closure builds without a local patch-table override for this crate.",
        delta_evidence_command: "git diff v1.9.0 -- crates/3rdparties/scheduler",
        local_ref: "0.1.0 script-vendored at Zed v1.9.0",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "Async scheduler/executor used by gpui.",
        retained_changes: &[
            "Crate-root lint allows in src/scheduler.rs (new_without_default, type_complexity, nonminimal_bool, unnecessary_map_or, let_unit_value) for unmodified upstream code.",
        ],
        verification_gate: "cargo check -p scheduler and just lint-host.",
        vendoring_doc: "crates/3rdparties/scheduler/VENDORED.md",
    },
    VendoredPatch {
        name: "sum_tree",
        local_path: "crates/3rdparties/sum_tree",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/sum_tree at Zed v1.9.0",
        owner: "platform-maintainers",
        last_reviewed: "2026-07-18",
        review_cadence_days: 90,
        removal_condition: "Remove when upstream sum_tree drops the GPL-3.0 ztracing/zlog dependencies or the closure no longer needs a local copy.",
        delta_evidence_command: "git diff v1.9.0 -- crates/3rdparties/sum_tree",
        local_ref: "0.1.0 script-vendored at Zed v1.9.0 plus GPL patch",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "Sequence-tree storage for text; carries the GPL-3.0 ztracing/zlog removal patch.",
        retained_changes: &[
            "ztracing::instrument -> tracing::instrument in src/cursor.rs and src/sum_tree.rs (ztracing is GPL-3.0; tracing was already a dependency).",
            "Removed the zlog::init_test() test call and its empty init_logger wrapper (zlog is GPL-3.0).",
            "Dropped the GPL-3.0 ztracing/zlog dependencies from Cargo.toml.",
        ],
        verification_gate: "cargo check -p sum_tree, cargo test -p sum_tree, and just lint-host.",
        vendoring_doc: "crates/3rdparties/sum_tree/VENDORED.md",
    },
    VendoredPatch {
        name: "util",
        local_path: "crates/3rdparties/util",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/util at Zed v1.9.0",
        owner: "platform-maintainers",
        last_reviewed: "2026-07-18",
        review_cadence_days: 90,
        removal_condition: "Remove when the workspace no longer needs zed's async-process/async-task forks or a local util patch point.",
        delta_evidence_command: "git diff v1.9.0 -- crates/3rdparties/util",
        local_ref: "0.1.0 script-vendored at Zed v1.9.0",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "Shared platform/command utilities used across the closure.",
        retained_changes: &[
            "Root Cargo.toml (outside this crate) mirrors zed v1.9.0 [patch.crates-io] for async-process (rev 0b6d671) and async-task (rev b4486cd) because src/command/darwin.rs calls smol::process::Child::adopt_raw_pid.",
            "Crate-root lint allows in src/util.rs for default clippy lints that fire on unmodified upstream code (inventory in VENDORED.md).",
        ],
        verification_gate: "cargo check -p util and just lint-host; re-verify the adopt_raw_pid call sites when moving the async-process/async-task patches.",
        vendoring_doc: "crates/3rdparties/util/VENDORED.md",
    },
    VendoredPatch {
        name: "util_macros",
        local_path: "crates/3rdparties/util_macros",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/util_macros at Zed v1.9.0",
        owner: "platform-maintainers",
        last_reviewed: "2026-07-18",
        review_cadence_days: 90,
        removal_condition: "Remove when the GPUI closure builds without a local patch-table override for this crate.",
        delta_evidence_command: "git diff v1.9.0 -- crates/3rdparties/util_macros",
        local_ref: "0.1.0 script-vendored at Zed v1.9.0",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "Proc macros for util.",
        retained_changes: &[
            "#![allow(unexpected_cfgs)] at the src/util_macros.rs crate root for the perf_enabled cfg Zed sets via RUSTFLAGS.",
        ],
        verification_gate: "cargo check -p util_macros and just lint-host.",
        vendoring_doc: "crates/3rdparties/util_macros/VENDORED.md",
    },
    VendoredPatch {
        name: "zed-font-kit",
        local_path: "crates/3rdparties/zed-font-kit",
        upstream: "https://github.com/zed-industries/font-kit",
        upstream_base: "rev 110523127440aefb11ce0cf280ae7c5071337ec5",
        owner: "text-platform-maintainers",
        last_reviewed: "2026-07-12",
        review_cadence_days: 30,
        removal_condition: "Remove when upstream zed-font-kit supports Apple mobile cfgs and retained bitmap behavior.",
        delta_evidence_command: "git diff 110523127440aefb11ce0cf280ae7c5071337ec5 -- crates/3rdparties/zed-font-kit",
        local_ref: "0.14.1-zed active local patch",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::HandMaintained,
        reason: "Apple mobile target cfg and CoreText manifest fixes while staying close to Zed's font-kit fork.",
        retained_changes: &[
            "CoreText dependency cfgs include ios, tvos, watchos, and visionos.",
            "FreeType/fontconfig dependency cfgs exclude Apple mobile targets.",
            "Canvas bitmap conversion covers A8/RGBA and 1bpp bitmap expansion without panic stubs.",
            "Shared font source selection maps CSS-generic titled family aliases to platform defaults.",
            "CoreText unit-test helper paths are compatible with direct vendored crate test runs.",
            "Standalone [workspace] table keeps direct vendored crate tests runnable from this repository.",
            "Normalized vendored Cargo manifest is kept with Cargo.toml.orig for import comparison.",
        ],
        verification_gate: "Direct zed-font-kit canvas tests, Apple mobile target checks for font consumers, and Linux fontconfig/FreeType checks when non-Apple font discovery changes.",
        vendoring_doc: "crates/3rdparties/zed-font-kit/VENDORING.md",
    },
];

/// Return the current vendored patch manifest.
pub const fn vendored_patch_manifest() -> VendoredPatchManifest {
    VendoredPatchManifest {
        schema_version: VENDORED_PATCH_SCHEMA_VERSION,
        report_type: VENDORED_PATCH_REPORT_TYPE,
        patches: VENDORED_PATCHES,
    }
}

/// Return vendored patch entries without allocating.
pub const fn vendored_patches() -> &'static [VendoredPatch] {
    VENDORED_PATCHES
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn repository_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn vendored_patch_manifest_has_stable_contract() {
        let manifest = vendored_patch_manifest();

        assert_eq!(manifest.schema_version, VENDORED_PATCH_SCHEMA_VERSION);
        assert_eq!(manifest.report_type, VENDORED_PATCH_REPORT_TYPE);
        assert!(!manifest.patches.is_empty());

        for patch in manifest.patches {
            assert!(!patch.name.is_empty());
            assert!(patch.local_path.starts_with("crates/3rdparties/"));
            assert!(!patch.upstream.is_empty());
            assert!(!patch.upstream_base.is_empty());
            assert!(!patch.owner.is_empty());
            assert_eq!(patch.last_reviewed.len(), 10);
            assert!((1..=365).contains(&patch.review_cadence_days));
            assert!(!patch.removal_condition.is_empty());
            assert!(patch.delta_evidence_command.starts_with("git diff "));
            assert!(!patch.local_ref.is_empty());
            assert!(!patch.status.as_str().is_empty());
            assert!(!patch.maintenance.as_str().is_empty());
            assert!(!patch.reason.is_empty());
            assert!(!patch.retained_changes.is_empty());
            assert!(!patch.verification_gate.is_empty());
            assert!(
                patch.vendoring_doc.ends_with("VENDORING.md")
                    || patch.vendoring_doc.ends_with("VENDORED.md"),
                "patch {} has an unexpected provenance doc {}",
                patch.name,
                patch.vendoring_doc
            );
        }
    }

    #[test]
    fn vendored_patch_manifest_has_unique_names() {
        let patches = vendored_patches();
        for (index, patch) in patches.iter().enumerate() {
            assert!(
                !patches[..index]
                    .iter()
                    .any(|previous| previous.name == patch.name),
                "duplicate vendored patch {}",
                patch.name
            );
        }
    }

    #[test]
    fn vendored_patch_manifest_matches_filesystem() {
        let root = repository_root();

        for patch in vendored_patches() {
            assert!(
                root.join(patch.local_path).is_dir(),
                "vendored path missing for {}: {}",
                patch.name,
                patch.local_path
            );
            assert!(
                root.join(patch.vendoring_doc).is_file(),
                "provenance doc missing for {}: {}",
                patch.name,
                patch.vendoring_doc
            );
            match patch.maintenance {
                VendoredPatchMaintenance::ScriptVendored => assert!(
                    patch.vendoring_doc.ends_with("VENDORED.md"),
                    "script-vendored patch {} must point at VENDORED.md",
                    patch.name
                ),
                VendoredPatchMaintenance::HandMaintained => assert!(
                    patch.vendoring_doc.ends_with("VENDORING.md"),
                    "hand-maintained patch {} must point at VENDORING.md",
                    patch.name
                ),
            }
        }
    }

    #[test]
    fn every_vendored_crate_dir_has_manifest_entry() {
        let third_parties = repository_root().join("crates/3rdparties");
        let mut covered = std::collections::BTreeSet::new();
        let mut missing = Vec::new();

        for entry in std::fs::read_dir(&third_parties).expect("read crates/3rdparties") {
            let directory = entry.expect("3rdparties dir entry").path();
            if !directory.is_dir() {
                continue;
            }
            let has_provenance =
                directory.join("VENDORING.md").is_file() || directory.join("VENDORED.md").is_file();
            if !has_provenance {
                continue;
            }
            let name = directory
                .file_name()
                .and_then(|name| name.to_str())
                .expect("utf-8 crate directory name")
                .to_string();
            covered.insert(name.clone());
            if !vendored_patches().iter().any(|patch| patch.name == name) {
                missing.push(name);
            }
        }

        assert!(
            missing.is_empty(),
            "vendored crate directories missing from the manifest: {missing:?}"
        );
        assert_eq!(covered.len(), vendored_patches().len());
    }

    #[test]
    fn vendored_patch_manifest_covers_known_active_patches() {
        let active = vendored_patch_manifest()
            .active_patches()
            .map(|patch| patch.name)
            .collect::<Vec<_>>();

        assert_eq!(
            active,
            [
                "block",
                "collections",
                "derive_refineable",
                "gpui",
                "gpui_linux",
                "gpui_macos",
                "gpui_macros",
                "gpui_shared_string",
                "gpui_util",
                "gpui_wgpu",
                "gpui_windows",
                "http_client",
                "media",
                "objc",
                "perf",
                "refineable",
                "scheduler",
                "sum_tree",
                "util",
                "util_macros",
                "zed-font-kit",
            ]
        );
    }

    #[test]
    fn script_vendored_patches_track_the_zed_tag() {
        for patch in vendored_patches()
            .iter()
            .filter(|patch| patch.maintenance == VendoredPatchMaintenance::ScriptVendored)
        {
            assert!(
                patch.upstream_base.contains("v1.9.0"),
                "script-vendored patch {} lacks the v1.9.0 base ref",
                patch.name
            );
            assert_eq!(patch.status, VendoredPatchStatus::ActivePatch);
        }
    }

    #[test]
    fn active_patches_have_upgrade_evidence() {
        for patch in vendored_patch_manifest().active_patches() {
            assert!(
                patch.upstream_base.contains("v1.9.0")
                    || patch.upstream_base.contains("rev ")
                    || patch.upstream_base.contains("0.2.7")
                    || patch.upstream_base.contains("0.1.6"),
                "active patch {} lacks an exact upstream base",
                patch.name
            );
            assert!(
                patch
                    .retained_changes
                    .iter()
                    .any(|change| !change.contains("not yet")),
                "active patch {} lacks retained-change notes",
                patch.name
            );
        }
    }

    #[test]
    fn vendored_patch_markdown_names_active_patches() {
        let markdown = vendored_patch_manifest().to_markdown_table();

        assert!(markdown.contains(VENDORED_PATCH_REPORT_TYPE));
        assert!(markdown.contains("block"));
        assert!(markdown.contains("gpui"));
        assert!(markdown.contains("gpui_macos"));
        assert!(markdown.contains("gpui_wgpu"));
        assert!(markdown.contains("gpui_windows"));
        assert!(markdown.contains("objc"));
        assert!(markdown.contains("sum_tree"));
        assert!(markdown.contains("zed-font-kit"));
        assert!(markdown.contains("script-vendored"));
        assert!(markdown.contains("hand-maintained"));
        assert!(markdown.contains("110523127440aefb11ce0cf280ae7c5071337ec5"));
    }
}
