//! Vendored dependency patch manifest for release QA.

// Rust guideline compliant 2026-02-21

use std::path::PathBuf;

/// Schema version for [`VendoredPatchManifest`].
pub const VENDORED_PATCH_SCHEMA_VERSION: u32 = 1;

/// Stable report type identifier for [`VendoredPatchManifest`].
pub const VENDORED_PATCH_REPORT_TYPE: &str = "gpui-toolkit-vendored-patches";

/// GitHub source of the third-party dependency snapshots tracked below.
pub const SOTF_THIRD_PARTIES_GIT_URL: &str = "https://github.com/pierreaubert/sotf-3rdparties";

/// Environment override pointing at an explicit sotf-3rdparties checkout.
///
/// When set, [`sotf_third_parties_dir`] returns it verbatim. This covers
/// developers working from a local clone, e.g. inside the `all_of_sotf`
/// monorepo; standalone checkouts fall back to the cargo git checkout at
/// the `Cargo.lock`-pinned revision.
pub const SOTF_THIRD_PARTIES_DIR_ENV: &str = "SOTF_3RDPARTIES_DIR";

/// Directory prefix cargo uses for the sotf-3rdparties git checkouts.
///
/// Cargo names git checkout roots `<url-last-segment>-<db-hash>`; the hash
/// suffix is cargo-version specific, so only the prefix is matched.
const SOTF_THIRD_PARTIES_CHECKOUT_PREFIX: &str = "sotf-3rdparties-";

/// Provenance file every usable sotf-3rdparties checkout must contain.
///
/// Used as a sanity probe so a partial or unexpected checkout is never
/// mistaken for the dependency source.
const SOTF_THIRD_PARTIES_PROBE_PATH: &str = "gpui/crates/gpui/VENDORED.md";

/// Locate the local sotf-3rdparties checkout backing this workspace.
///
/// Resolution order:
/// 1. [`SOTF_THIRD_PARTIES_DIR_ENV`] when set.
/// 2. The cargo git checkout (`$CARGO_HOME/git/checkouts/sotf-3rdparties-*/…`)
///    matching the revision pinned in the workspace `Cargo.lock`.
///
/// Returns `None` when no usable checkout exists. In practice any build that
/// compiled the git dependencies already materialized the checkout, so
/// release-gate tests treat `None` as a hard failure rather than a skip.
///
/// # Examples
///
/// ```no_run
/// let dir = gpui_release_gates::sotf_third_parties_dir();
/// assert!(dir.is_some());
/// ```
pub fn sotf_third_parties_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os(SOTF_THIRD_PARTIES_DIR_ENV) {
        return Some(PathBuf::from(dir));
    }
    let rev = cargo_lock_third_parties_rev()?;
    for cargo_home in cargo_home_candidates() {
        let Ok(entries) = std::fs::read_dir(cargo_home.join("git/checkouts")) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if !name.starts_with(SOTF_THIRD_PARTIES_CHECKOUT_PREFIX) {
                continue;
            }
            let Ok(revisions) = std::fs::read_dir(entry.path()) else {
                continue;
            };
            for revision in revisions.flatten() {
                let dir_name = revision.file_name();
                let Some(dir_name) = dir_name.to_str() else {
                    continue;
                };
                if !rev.starts_with(dir_name) {
                    continue;
                }
                let candidate = revision.path();
                if candidate.join(SOTF_THIRD_PARTIES_PROBE_PATH).is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

/// Cargo home candidates in lookup order.
fn cargo_home_candidates() -> Vec<PathBuf> {
    let mut homes = Vec::new();
    if let Some(home) = std::env::var_os("CARGO_HOME") {
        homes.push(PathBuf::from(home));
    }
    if let Some(home) = std::env::var_os("HOME") {
        homes.push(PathBuf::from(home).join(".cargo"));
    }
    homes
}

/// Revision of `sotf-3rdparties` pinned in the workspace `Cargo.lock`.
fn cargo_lock_third_parties_rev() -> Option<String> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lock = manifest_dir
        .ancestors()
        .map(|dir| dir.join("Cargo.lock"))
        .find(|lock| lock.is_file())?;
    let content = std::fs::read_to_string(lock).ok()?;
    content.lines().find_map(lock_source_rev)
}

/// Extract the `sotf-3rdparties` revision from one lockfile `source` line,
/// if that line names the repository. Accepts an optional `?query` fragment
/// (e.g. `?branch=...`) between the URL and the `#rev` fragment.
fn lock_source_rev(line: &str) -> Option<String> {
    let base = format!("git+{SOTF_THIRD_PARTIES_GIT_URL}");
    let source = line.trim().strip_prefix("source = \"")?.strip_suffix('"')?;
    let rest = source.strip_prefix(base.as_str())?;
    if !rest.starts_with('#') && !rest.starts_with('?') {
        return None;
    }
    // `rest` is `#<rev>` or `?<query>#<rev>`; the rev itself never
    // contains `#`, so the last fragment is always the revision.
    match rest.rsplit('#').next() {
        Some(rev) if !rev.is_empty() => Some(rev.to_string()),
        _ => None,
    }
}

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
    /// Vendored directory relative to the sotf-3rdparties checkout
    /// (see [`sotf_third_parties_dir`]).
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
    /// Run from the sotf-3rdparties checkout root.
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
    /// hand-maintained crates, `VENDORED.md` for script-vendored crates),
    /// relative to the sotf-3rdparties checkout.
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
        local_path: "block",
        upstream: "https://github.com/SSheldon/rust-block",
        upstream_base: "block 0.1.6",
        owner: "platform-maintainers",
        last_reviewed: "2026-08-26",
        review_cadence_days: 90,
        removal_condition: "Remove when upstream block supports the workspace Rust toolchain without the retained changes.",
        delta_evidence_command: "git diff block-0.1.6 -- block",
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
        verification_gate: "cargo check --manifest-path block/Cargo.toml --lib and cargo report future-incompatibilities after a dependent build.",
        vendoring_doc: "block/VENDORING.md",
    },
    VendoredPatch {
        name: "gpui",
        local_path: "gpui/crates/gpui",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/gpui at Zed v1.9.0",
        owner: "gpui-platform-maintainers",
        last_reviewed: "2026-08-07",
        review_cadence_days: 90,
        removal_condition: "Remove when the workspace tracks upstream gpui without local patch-table overrides.",
        delta_evidence_command: "git diff v1.9.0 -- gpui/crates/gpui",
        local_ref: "0.2.2 script-vendored at Zed v1.9.0",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "Core GPUI UI framework snapshot.",
        retained_changes: &[
            "src/app.rs adds ApplicationHandle and Application::run_embedded, ported verbatim from upstream zed PR #60574 so wasm/embedded platforms can keep the app alive when Platform::run returns immediately.",
            "Restored IBMPlexSans-Regular.ttf and Lilex-Regular.ttf under crates/assets/fonts/ so the svg_renderer include_bytes! paths resolve from the deeper vendored layout.",
            "Crate-root lint allows in src/gpui.rs for default clippy/rustc lints that fire on unmodified upstream code (inventory in VENDORED.md).",
            "Restricted image crate features to GPUI's advertised bitmap formats, excluding unrelated codecs and Rayon.",
            "Profiler locks use the already-present parking_lot crate instead of yanked spin 0.10.",
            "stacksafe 1.0 replaces the future-incompatible proc-macro-error2 dependency chain without changing GPUI's StackSafe API usage.",
        ],
        verification_gate: "cargo check -p gpui and just lint-host; run the gpui svg_renderer font tests when renderer or font code changes.",
        vendoring_doc: "gpui/crates/gpui/VENDORED.md",
    },
    VendoredPatch {
        name: "gpui_linux",
        local_path: "gpui/crates/gpui_linux",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/gpui_linux at Zed v1.9.0",
        owner: "linux-platform-maintainers",
        last_reviewed: "2026-10-21",
        review_cadence_days: 90,
        removal_condition: "Remove when the GPUI closure builds without a local patch-table override for this crate.",
        delta_evidence_command: "git diff v1.9.0 -- gpui/crates/gpui_linux",
        local_ref: "0.1.0 script-vendored at Zed v1.9.0",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "Linux platform backend for gpui.",
        retained_changes: &[
            "Standalone gpui dependency preserves Zed's default-features=false workspace policy; gpui-miniapp explicitly selects Wayland and X11.",
            "Restricted image crate features to GPUI's advertised bitmap formats.",
        ],
        verification_gate: "cargo check -p gpui_linux and just lint-host.",
        vendoring_doc: "gpui/crates/gpui_linux/VENDORED.md",
    },
    VendoredPatch {
        name: "gpui_macos",
        local_path: "gpui/crates/gpui_macos",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/gpui_macos at Zed v1.9.0",
        owner: "apple-platform-maintainers",
        last_reviewed: "2026-10-21",
        review_cadence_days: 90,
        removal_condition: "Remove when upstream gpui_macos drops the private CGS symbol references and the root patch table no longer needs this snapshot.",
        delta_evidence_command: "git diff v1.9.0 -- gpui/crates/gpui_macos",
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
        vendoring_doc: "gpui/crates/gpui_macos/VENDORED.md",
    },
    VendoredPatch {
        name: "gpui_macros",
        local_path: "gpui/crates/gpui_macros",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/gpui_macros at Zed v1.9.0",
        owner: "gpui-platform-maintainers",
        last_reviewed: "2026-10-21",
        review_cadence_days: 90,
        removal_condition: "Remove when the GPUI closure builds without a local patch-table override for this crate.",
        delta_evidence_command: "git diff v1.9.0 -- gpui/crates/gpui_macros",
        local_ref: "0.1.0 script-vendored at Zed v1.9.0",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "Proc macros for gpui.",
        retained_changes: &[
            "#![allow(unexpected_cfgs)] at the tests/derive_inspector_reflection.rs crate root for the rust_analyzer cfg Zed sets for editor tooling.",
        ],
        verification_gate: "cargo check -p gpui_macros and just lint-host.",
        vendoring_doc: "gpui/crates/gpui_macros/VENDORED.md",
    },
    VendoredPatch {
        name: "gpui_web",
        local_path: "gpui/crates/gpui_web",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/gpui_web at Zed v1.9.0",
        owner: "gpui-platform-maintainers",
        last_reviewed: "2026-08-15",
        review_cadence_days: 90,
        removal_condition: "Remove when the GPUI closure builds without a local patch-table override for this crate.",
        delta_evidence_command: "git diff v1.9.0 -- gpui/crates/gpui_web",
        local_ref: "0.1.0 script-vendored at Zed v1.9.0 plus PR #61707 port",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "Web/wasm platform backend for gpui (canvas plus WebGPU).",
        retained_changes: &[
            "src/window.rs ports the reentrancy-safe callback invocation from upstream zed PR #61707: WebWindowInner::with_callback take/call/restore, raf_id tracking, and a Drop for WebWindow that cancels the pending rAF and disconnects the ResizeObserver (details in VENDORED.md).",
            "src/dispatcher.rs ports the run_waker_loop fixes from the same PR #61707 commit: await waitAsync only when is_async, and drain the mailbox again after re-arming the signal so no notifications are lost (details in VENDORED.md).",
            "The remaining PR #61707 hunks (platform.rs DOM failure message, events.rs EventListenerHandle rework, window.rs fullscreen/clamp/handle hunks) and the on-demand frame_waker rework (upstream zed PR #62327) are deliberately not ported; documented as non-ported follow-ups in VENDORED.md.",
        ],
        verification_gate: "just wasm-check (nightly wasm32-unknown-unknown closure) and just lint-host; run just wasm-test for web platform changes.",
        vendoring_doc: "gpui/crates/gpui_web/VENDORED.md",
    },
    VendoredPatch {
        name: "gpui_wgpu",
        local_path: "gpui/crates/gpui_wgpu",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/gpui_wgpu at Zed v1.9.0",
        owner: "rendering-maintainers",
        last_reviewed: "2026-10-04",
        review_cadence_days: 30,
        removal_condition: "Classify remaining source deltas, retain custom draws, then pass native/wasm/AU render gates with the replacement.",
        delta_evidence_command: "git diff --no-index <Zed ced90fc/crates/gpui_wgpu> gpui/crates/gpui_wgpu",
        local_ref: "0.1.2 active local patch; tree 80f354219ed1f9cb5123546da4d4d4a8ff694f02 vs Zed 830b79b0110a44303a6f2c29bdf1c3c60fe59613",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::HandMaintained,
        reason: "Local GPUI WGPU renderer/backend patch point while tracking the Zed tag.",
        retained_changes: &[
            "Manifest tracks this workspace's Zed v1.9.0 dependency set.",
            "Standalone gpui dependency preserves Zed's default-features=false workspace policy.",
            "zed-font-kit is pinned to rev 94b0f28166665e8fd2f53ff6d268a14955c82269 to match the root patch.",
            "Custom draw dispatch carries frame format/full bounds and capability probing; premultiplied sprite shaders differ from pinned Zed.",
            "Seven common Rust/WGSL source files and split text/renderer modules need classification before de-vendoring.",
        ],
        verification_gate: "cargo check -p gpui_wgpu and at least one renderer/showcase smoke pass for rendering changes.",
        vendoring_doc: "gpui/crates/gpui_wgpu/VENDORING.md",
    },
    VendoredPatch {
        name: "gpui_windows",
        local_path: "gpui/crates/gpui_windows",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/gpui_windows at Zed v1.9.0",
        owner: "windows-platform-maintainers",
        last_reviewed: "2026-10-04",
        review_cadence_days: 30,
        removal_condition: "Classify source deltas, retain scene/platform behavior, and pass native Windows compile and UI gates.",
        delta_evidence_command: "git diff --no-index <Zed ced90fc/crates/gpui_windows> gpui/crates/gpui_windows",
        local_ref: "0.1.2 active local patch; tree e5bfb74d26de9ffa09b245c3d962dbbbe71668d7 vs Zed 356dd80ca8b943a04d67898c1ee6a1d792eb0d6f",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::HandMaintained,
        reason: "Local GPUI Windows backend patch point for dependency features and platform parity fixes.",
        retained_changes: &[
            "Manifest uses workspace dependency pins and Windows feature choices.",
            "hide_other_apps and unhide_other_apps intentionally no-op on Windows instead of panicking.",
            "DirectX renderer handles PrimitiveBatch::Custom as a no-op; fourteen common source files plus build.rs differ from pinned Zed and need classification.",
        ],
        verification_gate: "cargo check -p gpui_windows --target x86_64-pc-windows-msvc plus native Windows smoke tests.",
        vendoring_doc: "gpui/crates/gpui_windows/VENDORING.md",
    },
    VendoredPatch {
        name: "http_client",
        local_path: "gpui/crates/http_client",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/http_client at Zed v1.9.0",
        owner: "platform-maintainers",
        last_reviewed: "2026-10-21",
        review_cadence_days: 90,
        removal_condition: "Remove when the GPUI closure builds without a local patch-table override for this crate.",
        delta_evidence_command: "git diff v1.9.0 -- gpui/crates/http_client",
        local_ref: "0.1.0 script-vendored at Zed v1.9.0",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "HTTP client abstraction used by the GPUI closure.",
        retained_changes: &[
            "#![allow(clippy::new_without_default)] at the src/http_client.rs crate root (BlockedHttpClient::new).",
        ],
        verification_gate: "cargo check -p http_client and just lint-host.",
        vendoring_doc: "gpui/crates/http_client/VENDORED.md",
    },
    VendoredPatch {
        name: "objc",
        local_path: "objc",
        upstream: "https://github.com/SSheldon/rust-objc",
        upstream_base: "objc 0.2.7",
        owner: "apple-platform-maintainers",
        last_reviewed: "2026-10-08",
        review_cadence_days: 90,
        removal_condition: "Remove when upstream objc supports modern Rust and Apple targets without retained changes.",
        delta_evidence_command: "git diff objc-0.2.7 -- objc",
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
            "Rename the Cargo package to sotf-objc (lib target stays objc) with sotf-3rdparties repository metadata so the fork is publishable.",
        ],
        verification_gate: "cargo check -p sotf-objc, cargo test -p sotf-objc, and representative Apple/backend dependent checks.",
        vendoring_doc: "objc/VENDORING.md",
    },
    VendoredPatch {
        name: "perf",
        local_path: "gpui/crates/perf",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "tooling/perf at Zed v1.9.0",
        owner: "platform-maintainers",
        last_reviewed: "2026-10-21",
        review_cadence_days: 90,
        removal_condition: "Remove when the GPUI closure builds without a local patch-table override for this crate.",
        delta_evidence_command: "git diff v1.9.0 -- gpui/crates/perf",
        local_ref: "0.1.0 script-vendored at Zed v1.9.0",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "Profiling helpers used by the GPUI closure.",
        retained_changes: &[
            "No local patches; pristine v1.9.0 snapshot modulo the import exclusions recorded in VENDORED.md.",
        ],
        verification_gate: "cargo check -p perf and just lint-host.",
        vendoring_doc: "gpui/crates/perf/VENDORED.md",
    },
    VendoredPatch {
        name: "scheduler",
        local_path: "gpui/crates/scheduler",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/scheduler at Zed v1.9.0",
        owner: "platform-maintainers",
        last_reviewed: "2026-10-08",
        review_cadence_days: 90,
        removal_condition: "Remove when the GPUI closure builds without a local patch-table override for this crate.",
        delta_evidence_command: "git diff v1.9.0 -- gpui/crates/scheduler",
        local_ref: "0.1.0 script-vendored at Zed v1.9.0",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "Async scheduler/executor used by gpui.",
        retained_changes: &[
            "Crate-root lint allows in src/scheduler.rs (new_without_default, type_complexity, nonminimal_bool, unnecessary_map_or, let_unit_value) for unmodified upstream code.",
            "Three blocking flume channels use equivalent standard-library MPSC channels, removing the yanked spin transitive dependency while preserving capacities and shutdown behavior.",
            "The backtrace dependency is target-gated off Apple mobile targets (ios, tvos, watchos, visionos), where backtrace 0.3 no longer compiles with libc >= 0.2.190; a cfg-gated module in src/test_scheduler.rs provides capture-less stand-ins so TestScheduler keeps its API.",
        ],
        verification_gate: "cargo test -p scheduler --all-targets, cargo check -p gpui --all-targets, and just lint-host.",
        vendoring_doc: "gpui/crates/scheduler/VENDORED.md",
    },
    VendoredPatch {
        name: "sum_tree",
        local_path: "gpui/crates/sum_tree",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/sum_tree at Zed v1.9.0",
        owner: "platform-maintainers",
        last_reviewed: "2026-10-21",
        review_cadence_days: 90,
        removal_condition: "Remove when upstream sum_tree drops the GPL-3.0 ztracing/zlog dependencies or the closure no longer needs a local copy.",
        delta_evidence_command: "git diff v1.9.0 -- gpui/crates/sum_tree",
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
        vendoring_doc: "gpui/crates/sum_tree/VENDORED.md",
    },
    VendoredPatch {
        name: "util",
        local_path: "gpui/crates/util",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/util at Zed v1.9.0",
        owner: "platform-maintainers",
        last_reviewed: "2026-10-21",
        review_cadence_days: 90,
        removal_condition: "Remove when the workspace no longer needs zed's async-process/async-task forks or a local util patch point.",
        delta_evidence_command: "git diff v1.9.0 -- gpui/crates/util",
        local_ref: "0.1.0 script-vendored at Zed v1.9.0",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "Shared platform/command utilities used across the closure.",
        retained_changes: &[
            "Root Cargo.toml (outside this crate) mirrors zed v1.9.0 [patch.crates-io] for async-process (rev 0b6d671) and async-task (rev b4486cd) because src/command/darwin.rs calls smol::process::Child::adopt_raw_pid.",
            "Crate-root lint allows in src/util.rs for default clippy lints that fire on unmodified upstream code (inventory in VENDORED.md).",
        ],
        verification_gate: "cargo check -p util and just lint-host; re-verify the adopt_raw_pid call sites when moving the async-process/async-task patches.",
        vendoring_doc: "gpui/crates/util/VENDORED.md",
    },
    VendoredPatch {
        name: "util_macros",
        local_path: "gpui/crates/util_macros",
        upstream: "https://github.com/zed-industries/zed",
        upstream_base: "crates/util_macros at Zed v1.9.0",
        owner: "platform-maintainers",
        last_reviewed: "2026-10-21",
        review_cadence_days: 90,
        removal_condition: "Remove when the GPUI closure builds without a local patch-table override for this crate.",
        delta_evidence_command: "git diff v1.9.0 -- gpui/crates/util_macros",
        local_ref: "0.1.0 script-vendored at Zed v1.9.0",
        status: VendoredPatchStatus::ActivePatch,
        maintenance: VendoredPatchMaintenance::ScriptVendored,
        reason: "Proc macros for util.",
        retained_changes: &[
            "#![allow(unexpected_cfgs)] at the src/util_macros.rs crate root for the perf_enabled cfg Zed sets via RUSTFLAGS.",
        ],
        verification_gate: "cargo check -p util_macros and just lint-host.",
        vendoring_doc: "gpui/crates/util_macros/VENDORED.md",
    },
    VendoredPatch {
        name: "zed-font-kit",
        local_path: "zed-font-kit",
        upstream: "https://github.com/zed-industries/font-kit",
        upstream_base: "rev 94b0f28166665e8fd2f53ff6d268a14955c82269",
        owner: "text-platform-maintainers",
        last_reviewed: "2026-10-04",
        review_cadence_days: 30,
        removal_condition: "Remove after Apple mobile cfg, bitmap/CSS behavior, and other source deltas are matched and platform gates pass.",
        delta_evidence_command: "git diff --no-index <font-kit 94b0f28> zed-font-kit",
        local_ref: "0.14.1-zed active local patch; tree fd3baa7d0e1fc56f35dd1448e728fcee426d7842 vs upstream a9dd26e1b5f651d474b3ffe11a16f0c7471b9440",
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
            "Eight common source files and platform loader module layout differ from pinned font-kit and need classification.",
        ],
        verification_gate: "Direct zed-font-kit canvas tests, Apple mobile target checks for font consumers, and Linux fontconfig/FreeType checks when non-Apple font discovery changes.",
        vendoring_doc: "zed-font-kit/VENDORING.md",
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
    fn lock_source_rev_accepts_plain_and_branch_sources() {
        assert_eq!(
            lock_source_rev(
                r#"source = "git+https://github.com/pierreaubert/sotf-3rdparties#972d8b28adfdd12ed31307a49c58be2a650c2233""#
            )
            .as_deref(),
            Some("972d8b28adfdd12ed31307a49c58be2a650c2233")
        );
        assert_eq!(
            lock_source_rev(
                r#"source = "git+https://github.com/pierreaubert/sotf-3rdparties?branch=foreign-parent-win-linux#d7b22228899e7f6504e6c28f4e7c316b8bfdc420""#
            )
            .as_deref(),
            Some("d7b22228899e7f6504e6c28f4e7c316b8bfdc420")
        );
    }

    #[test]
    fn lock_source_rev_rejects_other_repositories() {
        assert_eq!(
            lock_source_rev(r#"source = "registry+https://github.com/rust-lang/crates.io-index""#),
            None
        );
        assert_eq!(
            lock_source_rev(
                r#"source = "git+https://github.com/pierreaubert/sotf-3rdparties-fork#972d8b28adfdd12ed31307a49c58be2a650c2233""#
            ),
            None
        );
        assert_eq!(lock_source_rev(r#"name = "gpui""#), None);
    }

    #[test]
    fn sotf_third_parties_dir_resolves_the_locked_checkout() {
        let dir = sotf_third_parties_dir().expect(
            "locate the sotf-3rdparties checkout (set SOTF_3RDPARTIES_DIR or build the git dependencies first)",
        );
        assert!(
            dir.join(SOTF_THIRD_PARTIES_PROBE_PATH).is_file(),
            "sotf-3rdparties checkout lacks gpui provenance: {}",
            dir.display()
        );
    }

    #[test]
    fn vendored_patch_manifest_has_stable_contract() {
        let manifest = vendored_patch_manifest();

        assert_eq!(manifest.schema_version, VENDORED_PATCH_SCHEMA_VERSION);
        assert_eq!(manifest.report_type, VENDORED_PATCH_REPORT_TYPE);
        assert!(!manifest.patches.is_empty());

        for patch in manifest.patches {
            assert!(!patch.name.is_empty());
            assert!(
                !patch.local_path.starts_with('/') && !patch.local_path.contains(".."),
                "patch {} escapes the third-party checkout: {}",
                patch.name,
                patch.local_path
            );
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
        let third_parties = sotf_third_parties_dir().expect(
            "locate the sotf-3rdparties checkout (set SOTF_3RDPARTIES_DIR or build the git dependencies first)",
        );

        for patch in vendored_patches() {
            assert!(
                third_parties.join(patch.local_path).is_dir(),
                "vendored path missing for {}: {} (third-party root: {})",
                patch.name,
                patch.local_path,
                third_parties.display()
            );
            assert!(
                third_parties.join(patch.vendoring_doc).is_file(),
                "provenance doc missing for {}: {} (third-party root: {})",
                patch.name,
                patch.vendoring_doc,
                third_parties.display()
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
        let third_parties = sotf_third_parties_dir().expect(
            "locate the sotf-3rdparties checkout (set SOTF_3RDPARTIES_DIR or build the git dependencies first)",
        );
        // Vendored crates live at the third-party root (block, objc,
        // zed-font-kit) and under gpui/crates; both roots hold
        // provenance-bearing directories the manifest must cover.
        let scan_roots = [third_parties.clone(), third_parties.join("gpui/crates")];
        let mut covered = std::collections::BTreeSet::new();
        let mut missing = Vec::new();

        for scan_root in &scan_roots {
            let entries = std::fs::read_dir(scan_root).unwrap_or_else(|error| {
                panic!(
                    "read vendored third-party dir {}: {error}",
                    scan_root.display()
                )
            });
            for entry in entries {
                let directory = entry.expect("3rdparties dir entry").path();
                if !directory.is_dir() {
                    continue;
                }
                let has_provenance = directory.join("VENDORING.md").is_file()
                    || directory.join("VENDORED.md").is_file();
                if !has_provenance {
                    continue;
                }
                let name = directory
                    .file_name()
                    .and_then(|name| name.to_str())
                    .expect("utf-8 crate directory name")
                    .to_string();
                let known = vendored_patches().iter().any(|patch| patch.name == name);
                if !known {
                    missing.push(name.clone());
                }
                covered.insert(name);
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
                "gpui",
                "gpui_linux",
                "gpui_macos",
                "gpui_macros",
                "gpui_web",
                "gpui_wgpu",
                "gpui_windows",
                "http_client",
                "objc",
                "perf",
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
        assert!(markdown.contains("94b0f28166665e8fd2f53ff6d268a14955c82269"));
    }
}
