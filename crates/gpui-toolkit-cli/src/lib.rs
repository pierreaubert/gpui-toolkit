#![forbid(unsafe_code)]

//! Unified agent-ready CLI for gpui-toolkit.
//!
//! This crate is the primary interface for working with the design
//! system, for humans and agents alike. Every command honors `--json`
//! with a typed [`Envelope`] (or [`ErrorEnvelope`] on failure), and
//! `manifest --json` self-describes the whole surface so agents never
//! scrape `--help`. The binary in `src/main.rs` is a thin wrapper: it
//! parses arguments, calls these library functions, and formats output,
//! so spawning `gpui-toolkit --json …` and calling the crate agree.

// Rust guideline compliant 2026-02-21

pub mod cli;
pub mod component;
pub mod doctor;
pub mod eject;
pub mod envelope;
pub mod error_codes;
pub mod gap;
pub mod init;
pub mod layout;
pub mod manifest;
pub mod template;
pub mod theme;
pub mod upgrade;

#[doc(inline)]
pub use cli::{Cli, CommandOutput, Commands, DetailArg, LayoutAction, ThemeAction, run_command};
#[doc(inline)]
pub use component::{
    COMPONENT_DETAIL_TYPE, COMPONENT_LIST_TYPE, COMPONENT_PROPS_TYPE, ComponentDetail,
    ComponentEntry, ComponentList, ComponentProps, DEFAULT_SEARCH_LIMIT, DetailLevel, SEARCH_TYPE,
    SearchHit, SearchResults, component_detail, component_list, component_props,
    parse_detail_level, render_detail_text, render_list_text, render_props_text,
    render_search_text, search,
};
#[doc(inline)]
pub use doctor::{
    DOCTOR_TYPE, DoctorCheck, DoctorReport, DoctorStatus, DoctorSummary, render_doctor_text,
    run_doctor,
};
#[doc(inline)]
pub use eject::{
    EJECT_TYPE, EjectReceipt, TOOLKIT_ROOT_ENV, eject_component, eject_component_from_root,
    find_toolkit_root, render_eject_text,
};
#[doc(inline)]
pub use envelope::{
    API_VERSION, Envelope, ErrorEnvelope, error_envelope, render_json, render_json_compact,
    success_envelope,
};
#[doc(inline)]
pub use error_codes::{ErrorCode, Suggestion, ToolkitError};
#[doc(inline)]
pub use gap::{GAP_AREAS, GAP_REPORT_TYPE, GapReport, gap_report, render_gap_text};
#[doc(inline)]
pub use init::{CATALOG_BEGIN, CATALOG_END, INIT_TYPE, InitOptions, InitReceipt, run_init};
#[doc(inline)]
pub use layout::{
    DEFAULT_LAYOUT_FN, LAYOUT_CHECK_TYPE, LAYOUT_EXPAND_TYPE, LAYOUT_GRAMMAR_TYPE, LayoutCheck,
    LayoutExpand, LayoutGrammar, layout_check, layout_expand, layout_expand_to_file,
    layout_grammar, render_layout_check_text, render_layout_expand_text,
};
#[doc(inline)]
pub use manifest::{
    COMMANDS, MANIFEST_TYPE, Manifest, ManifestArgument, ManifestCommand, ManifestOption,
    build_manifest, command_static,
};
#[doc(inline)]
pub use template::{
    TEMPLATE_COPY_TYPE, TEMPLATE_LIST_TYPE, TEMPLATE_SHOW_TYPE, TEMPLATE_SKELETON_TYPE, TEMPLATES,
    TemplateCopy, TemplateEntry, TemplateKind, TemplateList, TemplateShow, TemplateSkeleton,
    render_template_copy_text, render_template_list_text, template_copy, template_list,
    template_show, template_skeleton,
};
#[doc(inline)]
pub use theme::{
    StaleOutput, THEME_BUILD_TYPE, THEME_CHECK_TYPE, THEME_LIST_TYPE, THEME_TARGETS_TYPE,
    ThemeBuild, ThemeCheck, ThemeEntry, ThemeList, ThemeTarget, ThemeTargets,
    render_theme_build_text, render_theme_check_text, render_theme_list_text,
    render_theme_targets_text, theme_build, theme_check, theme_list, theme_targets,
};
#[doc(inline)]
pub use upgrade::{
    MIGRATIONS, MigrationNote, UPGRADE_LIST_TYPE, UpgradeList, render_upgrade_list_text,
    upgrade_list,
};
