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
    /// Current local version/ref.
    pub local_ref: &'static str,
    /// Current dependency-resolution status.
    pub status: VendoredPatchStatus,
    /// Why the copy is retained locally.
    pub reason: &'static str,
    /// Local changes that must be re-evaluated during upgrades.
    pub retained_changes: &'static [&'static str],
    /// Required verification before this patch can move to a release.
    pub verification_gate: &'static str,
    /// Human-maintained explanation file.
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
             | Name | Status | Upstream base | Local path | Reason | Retained changes | Verification |\n\
             | --- | --- | --- | --- | --- | --- | --- |\n",
            self.schema_version, self.report_type
        );

        for patch in self.patches {
            markdown.push_str(&format!(
                "| {} | {} | {} | `{}` | {} | {} | {} |\n",
                patch.name,
                patch.status.as_str(),
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
        local_ref: "0.1.6 active local patch",
        status: VendoredPatchStatus::ActivePatch,
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
        name: "gpui_macos",
        local_path: "crates/3rdparties/gpui_macos",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/gpui_macos at Zed v1.9.0",
        local_ref: "0.1.0 local snapshot",
        status: VendoredPatchStatus::InactiveSnapshot,
        reason: "Retained platform snapshot for App Store/private-symbol compatibility work.",
        retained_changes: &[
            "Private CGS symbol removal history is documented in VENDORING.md.",
            "Not active in root dependency resolution unless re-patched.",
        ],
        verification_gate: "Before reactivation, diff against the target Zed tag and run macOS platform smoke tests.",
        vendoring_doc: "crates/3rdparties/gpui_macos/VENDORING.md",
    },
    VendoredPatch {
        name: "gpui_wgpu",
        local_path: "crates/3rdparties/gpui_wgpu",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/gpui_wgpu at Zed v1.9.0",
        local_ref: "0.1.0 active local patch",
        status: VendoredPatchStatus::ActivePatch,
        reason: "Local GPUI WGPU renderer/backend patch point while tracking the Zed tag.",
        retained_changes: &[
            "Manifest tracks this workspace's Zed v1.9.0 dependency set.",
            "zed-font-kit is pinned to rev 110523127440aefb11ce0cf280ae7c5071337ec5 to match the root patch.",
        ],
        verification_gate: "cargo check -p gpui_wgpu and at least one renderer/showcase smoke pass for rendering changes.",
        vendoring_doc: "crates/3rdparties/gpui_wgpu/VENDORING.md",
    },
    VendoredPatch {
        name: "gpui_windows",
        local_path: "crates/3rdparties/gpui_windows",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/gpui_windows at Zed v1.9.0",
        local_ref: "0.1.0 active local patch",
        status: VendoredPatchStatus::ActivePatch,
        reason: "Local GPUI Windows backend patch point for dependency features and platform parity fixes.",
        retained_changes: &[
            "Manifest uses workspace dependency pins and Windows feature choices.",
            "hide_other_apps and unhide_other_apps intentionally no-op on Windows instead of panicking.",
        ],
        verification_gate: "cargo check -p gpui_windows --target x86_64-pc-windows-msvc plus native Windows smoke tests.",
        vendoring_doc: "crates/3rdparties/gpui_windows/VENDORING.md",
    },
    VendoredPatch {
        name: "mach2",
        local_path: "crates/3rdparties/mach2",
        upstream: "https://github.com/JohnTitor/mach2",
        upstream_base: "registry mach2 0.5.0 snapshot",
        local_ref: "0.5.0 inactive local snapshot",
        status: VendoredPatchStatus::InactiveSnapshot,
        reason: "Mach kernel bindings snapshot retained for possible platform work.",
        retained_changes: &[
            "Not active in current lockfile resolution.",
            "No release patch stack is currently claimed.",
        ],
        verification_gate: "Remove if unused, or diff and document patch reason before reactivation.",
        vendoring_doc: "crates/3rdparties/mach2/VENDORING.md",
    },
    VendoredPatch {
        name: "objc",
        local_path: "crates/3rdparties/objc",
        upstream: "https://github.com/SSheldon/rust-objc",
        upstream_base: "objc 0.2.7",
        local_ref: "0.2.7 active local patch",
        status: VendoredPatchStatus::ActivePatch,
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
        name: "psm",
        local_path: "crates/3rdparties/psm",
        upstream: "https://github.com/rust-lang/stacker",
        upstream_base: "registry psm 0.1.30 snapshot",
        local_ref: "0.1.30 inactive local snapshot",
        status: VendoredPatchStatus::InactiveSnapshot,
        reason: "Portable stack manipulation snapshot retained for possible stacker/platform work.",
        retained_changes: &[
            "Not active in current lockfile resolution.",
            "No release patch stack is currently claimed.",
        ],
        verification_gate: "Remove if unused, or diff and document patch reason before reactivation.",
        vendoring_doc: "crates/3rdparties/psm/VENDORING.md",
    },
    VendoredPatch {
        name: "zed-font-kit",
        local_path: "crates/3rdparties/zed-font-kit",
        upstream: "https://github.com/zed-industries/font-kit",
        upstream_base: "rev 110523127440aefb11ce0cf280ae7c5071337ec5",
        local_ref: "0.14.1-zed active local patch",
        status: VendoredPatchStatus::ActivePatch,
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
            assert!(!patch.local_ref.is_empty());
            assert!(!patch.status.as_str().is_empty());
            assert!(!patch.reason.is_empty());
            assert!(!patch.retained_changes.is_empty());
            assert!(!patch.verification_gate.is_empty());
            assert!(patch.vendoring_doc.ends_with("VENDORING.md"));
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
    fn vendored_patch_manifest_covers_known_active_patches() {
        let active = vendored_patch_manifest()
            .active_patches()
            .map(|patch| patch.name)
            .collect::<Vec<_>>();

        assert_eq!(
            active,
            ["block", "gpui_wgpu", "gpui_windows", "objc", "zed-font-kit"]
        );
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
        assert!(markdown.contains("gpui_wgpu"));
        assert!(markdown.contains("gpui_windows"));
        assert!(markdown.contains("objc"));
        assert!(markdown.contains("zed-font-kit"));
        assert!(markdown.contains("110523127440aefb11ce0cf280ae7c5071337ec5"));
    }
}
