//! D3.js-inspired feature parity metadata for release QA.

/// Schema version for [`FeatureParityReport`].
pub const FEATURE_PARITY_SCHEMA_VERSION: u32 = 1;

/// Stable report type identifier for [`FeatureParityReport`].
pub const FEATURE_PARITY_REPORT_TYPE: &str = "gpui-d3rs-feature-parity";

/// Schema version for [`D3OptionParityReport`].
pub const D3_OPTION_PARITY_SCHEMA_VERSION: u32 = 1;

/// Stable report type identifier for [`D3OptionParityReport`].
pub const D3_OPTION_PARITY_REPORT_TYPE: &str = "gpui-d3rs-d3-option-parity";

/// Schema version for [`D3BenchmarkCoverageReport`].
pub const D3_BENCHMARK_COVERAGE_SCHEMA_VERSION: u32 = 1;

/// Stable report type identifier for [`D3BenchmarkCoverageReport`].
pub const D3_BENCHMARK_COVERAGE_REPORT_TYPE: &str = "gpui-d3rs-large-dataset-benchmarks";

/// Current implementation status for one D3-inspired area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureParityStatus {
    /// The module has a usable core implementation and tests/examples.
    CoreImplemented,
    /// The module has checked APIs for invalid user data.
    CheckedInputs,
    /// The module exists but important D3 parity or release hardening remains.
    Partial,
    /// The D3 area is not implemented as a first-class module.
    Missing,
}

impl FeatureParityStatus {
    /// Stable status label for release reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CoreImplemented => "core-implemented",
            Self::CheckedInputs => "checked-inputs",
            Self::Partial => "partial",
            Self::Missing => "missing",
        }
    }

    /// Whether this status is strong enough for a release candidate claim.
    pub const fn is_release_ready(self) -> bool {
        matches!(self, Self::CoreImplemented | Self::CheckedInputs)
    }
}

/// One D3-inspired module or feature family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureParityEntry {
    /// Stable area id.
    pub id: &'static str,
    /// D3.js module or conceptual feature family.
    pub d3_area: &'static str,
    /// Matching gpui-d3rs module(s), if present.
    pub gpui_d3rs_modules: &'static str,
    /// Current implementation status.
    pub status: FeatureParityStatus,
    /// Evidence recorded for release notes.
    pub evidence: &'static str,
    /// Remaining work before claiming stronger parity.
    pub release_requirement: &'static str,
}

/// Current support status for one D3 option or feature surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum D3OptionParityStatus {
    /// The equivalent option is available and covered by release tests.
    Supported,
    /// A related surface exists, but important D3 behavior remains incomplete.
    Partial,
    /// The D3 option or submodule does not have a first-class equivalent.
    Missing,
    /// The D3 option does not map directly to GPUI-native retained rendering.
    NotApplicable,
}

impl D3OptionParityStatus {
    /// Stable status label for release reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::Partial => "partial",
            Self::Missing => "missing",
            Self::NotApplicable => "not-applicable",
        }
    }

    /// Whether this option still blocks a broad D3 parity claim.
    pub const fn is_blocking(self) -> bool {
        matches!(self, Self::Partial | Self::Missing)
    }
}

/// One D3 option or option family mapped to gpui-d3rs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct D3OptionParityEntry {
    /// Stable module id.
    pub module: &'static str,
    /// D3 option, method, or submodule.
    pub d3_option: &'static str,
    /// Matching gpui-d3rs public surface, if any.
    pub gpui_surface: &'static str,
    /// Current option parity status.
    pub status: D3OptionParityStatus,
    /// Release-note-ready notes.
    pub notes: &'static str,
}

/// Current coverage status for one release benchmark case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum D3BenchmarkCoverageStatus {
    /// The case is implemented by a Criterion benchmark target.
    CriterionBench,
}

impl D3BenchmarkCoverageStatus {
    /// Stable status label for release reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CriterionBench => "criterion-bench",
        }
    }
}

/// One large-dataset benchmark case used by release QA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct D3BenchmarkCoverageCase {
    /// Stable benchmark case id.
    pub id: &'static str,
    /// gpui-d3rs module or feature family exercised by the case.
    pub module: &'static str,
    /// Criterion benchmark target name.
    pub bench_target: &'static str,
    /// Criterion benchmark group within the target.
    pub benchmark_group: &'static str,
    /// Criterion benchmark id within the group.
    pub benchmark_id: &'static str,
    /// Synthetic or fixture dataset scale.
    pub dataset_scale: &'static str,
    /// Current coverage status.
    pub status: D3BenchmarkCoverageStatus,
    /// Evidence recorded for release notes.
    pub evidence: &'static str,
}

/// Versioned large-dataset benchmark coverage table for release notes and CI artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct D3BenchmarkCoverageReport {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub reviewed_on: &'static str,
    pub command: &'static str,
    pub baseline_policy: &'static str,
    pub cases: &'static [D3BenchmarkCoverageCase],
}

impl D3BenchmarkCoverageReport {
    /// Return cases for one stable module id.
    pub fn cases_for_module(
        self,
        module: &'static str,
    ) -> impl Iterator<Item = &'static D3BenchmarkCoverageCase> {
        self.cases.iter().filter(move |case| case.module == module)
    }

    /// Return the number of benchmark cases in the report.
    pub const fn case_count(self) -> usize {
        self.cases.len()
    }

    /// Render the benchmark report as Markdown for release notes.
    pub fn to_markdown_table(self) -> String {
        let mut markdown = format!(
            "# GPUI D3RS Large-Dataset Benchmarks\n\n\
             - schema_version: {}\n\
             - report_type: `{}`\n\
             - reviewed_on: {}\n\
             - command: `{}`\n\
             - baseline_policy: {}\n\n\
             | Module | Bench target | Group | ID | Dataset scale | Status | Evidence |\n\
             | --- | --- | --- | --- | --- | --- | --- |\n",
            self.schema_version,
            self.report_type,
            self.reviewed_on,
            self.command,
            self.baseline_policy
        );

        for case in self.cases {
            markdown.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} |\n",
                case.module,
                case.bench_target,
                case.benchmark_group,
                case.benchmark_id,
                case.dataset_scale,
                case.status.as_str(),
                case.evidence
            ));
        }

        markdown
    }
}

/// Versioned D3 option parity table for release notes and CI artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct D3OptionParityReport {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub reviewed_on: &'static str,
    pub entries: &'static [D3OptionParityEntry],
}

impl D3OptionParityReport {
    /// Return entries for one stable module id.
    pub fn entries_for_module(
        self,
        module: &'static str,
    ) -> impl Iterator<Item = &'static D3OptionParityEntry> {
        self.entries
            .iter()
            .filter(move |entry| entry.module == module)
    }

    /// Return options that still block a full D3 option-parity claim.
    pub fn blocking_entries(self) -> impl Iterator<Item = &'static D3OptionParityEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.status.is_blocking())
    }

    /// Render the option report as Markdown for release notes.
    pub fn to_markdown_table(self) -> String {
        let mut markdown = format!(
            "# GPUI D3RS D3 Option Parity\n\n\
             - schema_version: {}\n\
             - report_type: `{}`\n\
             - reviewed_on: {}\n\n\
             | Module | D3 option | gpui-d3rs surface | Status | Notes |\n\
             | --- | --- | --- | --- | --- |\n",
            self.schema_version, self.report_type, self.reviewed_on
        );

        for entry in self.entries {
            markdown.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                entry.module,
                entry.d3_option,
                entry.gpui_surface,
                entry.status.as_str(),
                entry.notes
            ));
        }

        markdown
    }
}

/// Versioned feature parity report for release notes and CI artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureParityReport {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub reviewed_on: &'static str,
    pub entries: &'static [FeatureParityEntry],
}

impl FeatureParityReport {
    /// Return true only when every parity entry is release-ready.
    pub fn all_release_ready(self) -> bool {
        self.entries
            .iter()
            .all(|entry| entry.status.is_release_ready())
    }

    /// Return entries that still block a broad D3 parity claim.
    pub fn blocking_entries(self) -> impl Iterator<Item = &'static FeatureParityEntry> {
        self.entries
            .iter()
            .filter(|entry| !entry.status.is_release_ready())
    }

    /// Render the report as Markdown for release notes.
    pub fn to_markdown_table(self) -> String {
        let mut markdown = format!(
            "# GPUI D3RS Feature Parity\n\n\
             - schema_version: {}\n\
             - report_type: `{}`\n\
             - reviewed_on: {}\n\n\
             | D3 area | gpui-d3rs modules | Status | Evidence | Release requirement |\n\
             | --- | --- | --- | --- | --- |\n",
            self.schema_version, self.report_type, self.reviewed_on
        );

        for entry in self.entries {
            markdown.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                entry.d3_area,
                entry.gpui_d3rs_modules,
                entry.status.as_str(),
                entry.evidence,
                entry.release_requirement
            ));
        }

        markdown
    }
}

const FEATURE_PARITY_ENTRIES: &[FeatureParityEntry] = &[
    FeatureParityEntry {
        id: "array-scale-format-time-color-interpolate",
        d3_area: "d3-array, d3-scale, d3-format, d3-time, d3-time-format, d3-color, d3-interpolate",
        gpui_d3rs_modules: "array, scale, format, time, time::format, color, interpolate",
        status: FeatureParityStatus::CoreImplemented,
        evidence: "Core modules exist with unit, golden, or example coverage for statistics, ticks, scales, numeric formatting, UTC time intervals, D3-style UTC time formatting, colors, and interpolation.",
        release_requirement: "Attach benchmark and example coverage if these modules are highlighted as production-stable.",
    },
    FeatureParityEntry {
        id: "shape-contour-geo-fetch",
        d3_area: "d3-shape, d3-contour, d3-geo, d3-fetch",
        gpui_d3rs_modules: "shape, contour, geo, fetch",
        status: FeatureParityStatus::CoreImplemented,
        evidence: "Shape, contour, geo, and fetch modules exist with examples and focused tests, including path/golden coverage; ContourRing::try_area validates finite ring points before area computation.",
        release_requirement: "Document input-size and malformed-data limits for untrusted datasets.",
    },
    FeatureParityEntry {
        id: "shape-pie",
        d3_area: "d3-shape pie/donut layouts",
        gpui_d3rs_modules: "shape::pie",
        status: FeatureParityStatus::CheckedInputs,
        evidence: "Pie::try_generate plus try_pie, try_donut, and try_half_pie validate non-finite and negative values plus non-finite/negative layout parameters while permissive APIs remain backward-compatible.",
        release_requirement: "Keep checked pie layout tests green and document permissive versus checked APIs.",
    },
    FeatureParityEntry {
        id: "shape-arc",
        d3_area: "d3-shape arc generators",
        gpui_d3rs_modules: "shape::arc",
        status: FeatureParityStatus::CheckedInputs,
        evidence: "Arc::try_generate, try_path_string, and try_arc_points validate finite centers/angles, non-negative radii/padding, radius ordering, and sampling segment counts while permissive APIs remain backward-compatible.",
        release_requirement: "Keep checked arc generator tests green and document permissive versus checked APIs.",
    },
    FeatureParityEntry {
        id: "shape-area",
        d3_area: "d3-shape area generators",
        gpui_d3rs_modules: "shape::area",
        status: FeatureParityStatus::CheckedInputs,
        evidence: "Area::try_generate, try_generate_into, try_area_points, SimpleArea::try_points, and SimpleArea::try_path validate finite rendered coordinates and simple-area lengths before emitting paths or outline points; permissive APIs remain backward-compatible.",
        release_requirement: "Keep checked area generator tests green and document permissive versus checked APIs.",
    },
    FeatureParityEntry {
        id: "shape-line",
        d3_area: "d3-shape line renderers",
        gpui_d3rs_modules: "shape::line",
        status: FeatureParityStatus::CheckedInputs,
        evidence: "validate_line_inputs and try_render_line validate finite line data coordinates, finite scale ranges and outputs, normalized opacity, non-negative sizes, and non-empty finite positive custom dash patterns while render_line remains backward-compatible.",
        release_requirement: "Keep checked line validation tests green and document permissive versus checked renderer APIs.",
    },
    FeatureParityEntry {
        id: "shape-scatter",
        d3_area: "d3-shape scatter renderers",
        gpui_d3rs_modules: "shape::scatter",
        status: FeatureParityStatus::CheckedInputs,
        evidence: "validate_scatter_inputs and try_render_scatter validate finite scatter data coordinates, finite scale ranges and outputs, normalized opacity, and non-negative point/stroke sizes while render_scatter remains backward-compatible.",
        release_requirement: "Keep checked scatter validation tests green and document permissive versus checked renderer APIs.",
    },
    FeatureParityEntry {
        id: "shape-stack",
        d3_area: "d3-shape stack layouts",
        gpui_d3rs_modules: "shape::stack",
        status: FeatureParityStatus::CheckedInputs,
        evidence: "Stack::try_generate plus try_stack, try_stack_expand, and try_streamgraph reject ragged rows and non-finite values before ordering or offset math while permissive APIs retain missing-cell-as-zero behavior.",
        release_requirement: "Keep checked stack layout tests green and document permissive versus checked APIs.",
    },
    FeatureParityEntry {
        id: "shape-symbol",
        d3_area: "d3-shape symbol generators",
        gpui_d3rs_modules: "shape::symbol",
        status: FeatureParityStatus::CheckedInputs,
        evidence: "Symbol::try_generate, try_generate_at, try_points, try_radius, and try_symbol_radius validate finite non-negative sizes plus finite translation coordinates while permissive APIs remain backward-compatible.",
        release_requirement: "Keep checked symbol generator tests green and document permissive versus checked APIs.",
    },
    FeatureParityEntry {
        id: "shape-link",
        d3_area: "d3-shape link generators",
        gpui_d3rs_modules: "shape::link",
        status: FeatureParityStatus::CheckedInputs,
        evidence: "try_link_horizontal, try_link_vertical, try_link_step, try_link_radial, and RadialLink::try_to_cartesian validate finite Cartesian/radial parameters and non-negative radial radii while permissive APIs remain backward-compatible.",
        release_requirement: "Keep checked link generator tests green and document permissive versus checked APIs.",
    },
    FeatureParityEntry {
        id: "shape-radial",
        d3_area: "d3-shape radial line/area generators",
        gpui_d3rs_modules: "shape::radial",
        status: FeatureParityStatus::CheckedInputs,
        evidence: "try_radial_line, try_radial_area, try_polar_grid_circles, try_polar_grid_rays, and RadialPoint::try_to_cartesian validate finite centers, angles, and non-negative radii while permissive APIs remain backward-compatible.",
        release_requirement: "Keep checked radial generator tests green and document permissive versus checked APIs.",
    },
    FeatureParityEntry {
        id: "chord",
        d3_area: "d3-chord",
        gpui_d3rs_modules: "chord",
        status: FeatureParityStatus::CheckedInputs,
        evidence: "ChordLayout::try_compute validates ragged, non-finite, and negative matrices; sort_groups/sort_subgroups behavior is covered by tests.",
        release_requirement: "Keep checked matrix tests green and document the recoverable error path.",
    },
    FeatureParityEntry {
        id: "quadtree",
        d3_area: "d3-quadtree",
        gpui_d3rs_modules: "quadtree",
        status: FeatureParityStatus::CheckedInputs,
        evidence: "QuadTree::try_from_data, try_add, and try_add_all reject non-finite coordinates before mutating checked batches.",
        release_requirement: "Keep checked coordinate tests green and document permissive versus checked APIs.",
    },
    FeatureParityEntry {
        id: "hexbin",
        d3_area: "d3-hexbin",
        gpui_d3rs_modules: "hexbin",
        status: FeatureParityStatus::CheckedInputs,
        evidence: "Hexbin::try_bin validates finite positive radius, finite ordered extents, and finite point accessor outputs while default accessors turn missing coordinates into recoverable invalid values and Hexbin::bin remains backward-compatible.",
        release_requirement: "Keep checked hexbin generation tests green and document permissive versus checked APIs.",
    },
    FeatureParityEntry {
        id: "sankey",
        d3_area: "d3-sankey",
        gpui_d3rs_modules: "sankey",
        status: FeatureParityStatus::CheckedInputs,
        evidence: "SankeyLayout::try_compute validates finite usable layout geometry, unique non-empty node ids, known link endpoints, and finite non-negative link values while SankeyLayout::compute remains backward-compatible.",
        release_requirement: "Keep checked Sankey layout tests green and document permissive versus checked APIs.",
    },
    FeatureParityEntry {
        id: "delaunay",
        d3_area: "d3-delaunay",
        gpui_d3rs_modules: "delaunay",
        status: FeatureParityStatus::CheckedInputs,
        evidence: "Delaunay::try_new, try_from_points_iter, try_find, try_find_within_radius, and try_voronoi validate finite point/query coordinates, finite non-negative radii, and finite ordered Voronoi bounds while permissive APIs remain backward-compatible.",
        release_requirement: "Keep checked Delaunay/Voronoi validation tests green and document permissive versus checked APIs.",
    },
    FeatureParityEntry {
        id: "hierarchy",
        d3_area: "d3-hierarchy",
        gpui_d3rs_modules: "hierarchy",
        status: FeatureParityStatus::CheckedInputs,
        evidence: "HierarchyNode::try_sum validates finite non-negative hierarchy values without mutating existing values on error, and TreeLayout::try_layout validates finite non-negative layout dimensions while permissive APIs remain backward-compatible.",
        release_requirement: "Keep checked hierarchy value/layout tests green and document permissive versus checked APIs.",
    },
    FeatureParityEntry {
        id: "force",
        d3_area: "d3-force",
        gpui_d3rs_modules: "force",
        status: FeatureParityStatus::CheckedInputs,
        evidence: "SimulationNode::try_new, Simulation::try_new, Simulation::try_tick, ForceCenter::try_new, ForceX/ForceY positional forces, ForceRadial, ForceCollide, ForceManyBody checked setters/validate, and ForceLink::try_new_for_nodes/checked setters validate finite node state, simulation parameters, force configuration, and link endpoints while permissive APIs remain backward-compatible.",
        release_requirement: "Keep checked force malformed-data tests green and document permissive versus checked APIs.",
    },
    FeatureParityEntry {
        id: "force-hierarchy-sankey-hexbin-delaunay",
        d3_area: "d3-force, d3-hierarchy, d3-sankey, d3-hexbin, d3-delaunay",
        gpui_d3rs_modules: "force, hierarchy, sankey, hexbin, delaunay",
        status: FeatureParityStatus::Partial,
        evidence: "Modules and showcase examples exist, force/hierarchy/hexbin/Sankey/Delaunay now have checked input validation, d3_option_parity_report() records per-module option support, and d3_benchmark_coverage_report() records large-dataset Criterion benchmark coverage.",
        release_requirement: "Run and attach large-dataset benchmark baselines plus document any remaining D3 semantic differences before claiming broad parity.",
    },
    FeatureParityEntry {
        id: "interaction-animation",
        d3_area: "d3-brush, d3-zoom, d3-transition, d3-dispatch, d3-timer, d3-ease",
        gpui_d3rs_modules: "brush, zoom, transition, dispatch, timer, ease",
        status: FeatureParityStatus::Partial,
        evidence: "Interaction and animation modules exist, but release QA has not recorded end-to-end GPUI interaction tests for brush/zoom/transition behavior.",
        release_requirement: "Record GPUI runtime interaction tests and document event/lifecycle differences from D3.js.",
    },
    FeatureParityEntry {
        id: "drag",
        d3_area: "d3-drag",
        gpui_d3rs_modules: "drag",
        status: FeatureParityStatus::CheckedInputs,
        evidence: "DragState validates finite pointer coordinates, click-distance thresholds, optional drag extents, active pointer identity, lifecycle transitions, per-update deltas, and total movement while staying renderer-independent.",
        release_requirement: "Keep drag state-machine tests green and connect the update surface from GPUI host events before claiming runtime interaction parity.",
    },
    FeatureParityEntry {
        id: "axis-grid-legend-text-gpu",
        d3_area: "Text rendering and GPU surfaces",
        gpui_d3rs_modules: "text, surface, gpu2d, gpu3d",
        status: FeatureParityStatus::Partial,
        evidence: "Rendering modules exist behind feature/test cfg boundaries; renderer-independent axis, grid, legend, and text layouts are checked, while GPUI text rendering and GPU surfaces still require visual or runtime QA.",
        release_requirement: "Attach visual-regression output and GPU/runtime smoke tests for every claimed rendering backend.",
    },
    FeatureParityEntry {
        id: "axis-layout",
        d3_area: "d3-axis",
        gpui_d3rs_modules: "axis",
        status: FeatureParityStatus::CheckedInputs,
        evidence: "AxisLayout::try_from_scale and axis_layout validate finite axis config, scale ranges, ticks, and scaled positions before returning renderer-independent domain-line, major/minor tick, label, and title geometry for all four orientations.",
        release_requirement: "Keep checked axis layout tests green and use the layout surface from GPUI/SVG/canvas renderers before claiming visual parity.",
    },
    FeatureParityEntry {
        id: "grid-layout",
        d3_area: "Cartesian grids",
        gpui_d3rs_modules: "grid",
        status: FeatureParityStatus::CheckedInputs,
        evidence: "GridLayout::try_from_scales and grid_layout validate finite chart sizes, grid visual configuration, opacities, scale ranges, ticks, and scaled tick positions before returning renderer-independent vertical lines, horizontal lines, and dot-intersection geometry.",
        release_requirement: "Keep checked grid layout tests green and use the layout surface from GPUI/SVG/canvas renderers before claiming visual parity.",
    },
    FeatureParityEntry {
        id: "legend-layout",
        d3_area: "Chart legends",
        gpui_d3rs_modules: "legend",
        status: FeatureParityStatus::CheckedInputs,
        evidence: "LegendLayout::try_from_config validates finite available width, legend sizing, max-width, and text metrics before returning renderer-independent title, row/column, item, symbol, and label geometry for vertical and horizontal legends.",
        release_requirement: "Keep checked legend layout tests green and use the layout surface from GPUI/SVG/canvas renderers before claiming visual parity.",
    },
    FeatureParityEntry {
        id: "text-layout",
        d3_area: "Chart text layout",
        gpui_d3rs_modules: "text_layout",
        status: FeatureParityStatus::CheckedInputs,
        evidence: "TextLayout::try_from_text and text_layout validate finite positive font sizes, line heights, max widths, finite letter spacing and rotation, then return renderer-independent line, baseline, anchor, wrapping, and rotated-bounds geometry.",
        release_requirement: "Keep checked text layout tests green and use the layout surface from GPUI/SVG/canvas renderers before claiming visual text parity.",
    },
    FeatureParityEntry {
        id: "tile",
        d3_area: "d3-tile",
        gpui_d3rs_modules: "tile",
        status: FeatureParityStatus::CheckedInputs,
        evidence: "TileLayout::try_tiles validates finite positive scale/tile size, finite ordered extents, finite translation, bounded zoom, and visible tile allocation size before returning tile coordinates plus screen-space bounds.",
        release_requirement: "Keep checked tile layout tests green and document the slippy-map coordinate convention.",
    },
    FeatureParityEntry {
        id: "selection",
        d3_area: "d3-selection",
        gpui_d3rs_modules: "selection",
        status: FeatureParityStatus::CheckedInputs,
        evidence: "keyed_data_join validates duplicate old/new keys and returns deterministic enter/update/exit buckets; index_data_join covers D3's default positional join while DOM mutation remains the GPUI host's responsibility.",
        release_requirement: "Keep selection data-join tests green and document that DOM mutation/chaining maps to GPUI component ownership.",
    },
];

const D3_OPTION_PARITY_ENTRIES: &[D3OptionParityEntry] = &[
    D3OptionParityEntry {
        module: "force",
        d3_option: "simulation alpha/alphaMin/alphaDecay/alphaTarget/velocityDecay/tick",
        gpui_surface: "Simulation fields plus tick/try_tick",
        status: D3OptionParityStatus::Supported,
        notes: "Core simulation knobs are public and checked ticking validates finite configuration.",
    },
    D3OptionParityEntry {
        module: "force",
        d3_option: "forceCenter(x, y)",
        gpui_surface: "ForceCenter::new / ForceCenter::try_new",
        status: D3OptionParityStatus::Supported,
        notes: "Center targets are configurable and checked construction rejects non-finite coordinates.",
    },
    D3OptionParityEntry {
        module: "force",
        d3_option: "forceManyBody().strength/theta/distanceMin/distanceMax",
        gpui_surface: "ForceManyBody fields and checked setters",
        status: D3OptionParityStatus::Supported,
        notes: "Many-body strength, theta, and distance bounds are configurable; checked setters validate malformed values.",
    },
    D3OptionParityEntry {
        module: "force",
        d3_option: "forceLink().links/distance/strength/iterations",
        gpui_surface: "ForceLink::new, try_new_for_nodes, distance/try_distance, strength/try_strength, iterations",
        status: D3OptionParityStatus::Supported,
        notes: "Link distance, strength, iterations, degree bias, and endpoint validation are represented.",
    },
    D3OptionParityEntry {
        module: "force",
        d3_option: "forceX, forceY",
        gpui_surface: "ForceX / ForceY with checked constructors and strength setters",
        status: D3OptionParityStatus::Supported,
        notes: "Position forces pull velocity toward fixed x/y targets and checked APIs reject malformed targets/strengths.",
    },
    D3OptionParityEntry {
        module: "force",
        d3_option: "forceRadial().radius/x/y/strength",
        gpui_surface: "ForceRadial with checked radius, center, and strength setters",
        status: D3OptionParityStatus::Supported,
        notes: "Radial force pulls velocity toward a target radius around a configurable center and checked APIs reject malformed radius/center/strength values.",
    },
    D3OptionParityEntry {
        module: "force",
        d3_option: "forceCollide().radius/strength/iterations",
        gpui_surface: "ForceCollide with constant/per-node radii, strength, and iterations",
        status: D3OptionParityStatus::Supported,
        notes: "Collision force pushes overlapping nodes apart, supports constant or per-node radii, and checked APIs reject malformed radius/strength values.",
    },
    D3OptionParityEntry {
        module: "hierarchy",
        d3_option: "hierarchy.sum/count/sort traversal basics",
        gpui_surface: "HierarchyNode::sum, try_sum, count, sort, each",
        status: D3OptionParityStatus::Supported,
        notes: "Core hierarchy value and traversal helpers exist; checked sum validates malformed values.",
    },
    D3OptionParityEntry {
        module: "hierarchy",
        d3_option: "tree.size/nodeSize",
        gpui_surface: "TreeLayout::size, node_size, layout, try_layout",
        status: D3OptionParityStatus::Supported,
        notes: "Tree layout dimensions are configurable and checked layout validates malformed dimensions.",
    },
    D3OptionParityEntry {
        module: "hierarchy",
        d3_option: "tree.separation",
        gpui_surface: "TreeLayout::separation",
        status: D3OptionParityStatus::Supported,
        notes: "Tree layout applies the configured typed separation callback between neighboring leaves, with checked validation for invalid outputs.",
    },
    D3OptionParityEntry {
        module: "hierarchy",
        d3_option: "treemap, pack, partition, cluster first-class layouts",
        gpui_surface: "TreemapLayout, PackLayout, PartitionLayout, ClusterLayout",
        status: D3OptionParityStatus::Supported,
        notes: "Hierarchy exports first-class treemap, pack, partition, and cluster layout APIs with checked geometry/value validation.",
    },
    D3OptionParityEntry {
        module: "sankey",
        d3_option: "node/link data validation",
        gpui_surface: "SankeyLayout::compute / try_compute",
        status: D3OptionParityStatus::Supported,
        notes: "Checked Sankey layout validates geometry, node ids, endpoints, and link values.",
    },
    D3OptionParityEntry {
        module: "sankey",
        d3_option: "nodeWidth/nodePadding/extent/iterations/linkSort/nodeAlign",
        gpui_surface: "SankeyLayout node_width/node_padding/extent/iterations/node_align/link_sort",
        status: D3OptionParityStatus::Supported,
        notes: "Sankey layout exposes D3-style geometry extent, node width/padding, relaxation iterations, node alignment, and custom link sorting with checked configuration tests.",
    },
    D3OptionParityEntry {
        module: "hexbin",
        d3_option: "radius/extent/x/y accessors",
        gpui_surface: "Hexbin configuration plus Hexbin::bin / try_bin",
        status: D3OptionParityStatus::Supported,
        notes: "Checked binning validates radius, extents, and accessor outputs.",
    },
    D3OptionParityEntry {
        module: "hexbin",
        d3_option: "hexagon path generator and centers",
        gpui_surface: "Hexbin::hexagon, try_hexagon, centers, try_centers",
        status: D3OptionParityStatus::Supported,
        notes: "Hexbin exposes D3-style hexagon SVG path generation, configured center generation, and checked variants for invalid radius/extent configuration.",
    },
    D3OptionParityEntry {
        module: "delaunay",
        d3_option: "Delaunay construction/find/neighbors/hull/triangles/rendering",
        gpui_surface: "Delaunay public API plus checked variants",
        status: D3OptionParityStatus::Supported,
        notes: "Core triangulation, lookup, neighbor, hull, triangle, and checked input paths are covered.",
    },
    D3OptionParityEntry {
        module: "delaunay",
        d3_option: "Voronoi bounds/render/cell polygons",
        gpui_surface: "Delaunay::voronoi / try_voronoi and Voronoi render/cell helpers",
        status: D3OptionParityStatus::Supported,
        notes: "Voronoi exposes checked bounds, whole-diagram rendering, bounds rendering, individual cell rendering, indexed cell polygons, containment, and neighbors.",
    },
];

const D3_BENCHMARK_COVERAGE_CASES: &[D3BenchmarkCoverageCase] = &[
    D3BenchmarkCoverageCase {
        id: "force-many-body-brute-force-5000",
        module: "force",
        bench_target: "force_many_body",
        benchmark_group: "force_many_body",
        benchmark_id: "brute_force/5000",
        dataset_scale: "5,000 nodes",
        status: D3BenchmarkCoverageStatus::CriterionBench,
        evidence: "Existing force_many_body Criterion bench compares brute-force many-body application at 5,000 deterministic nodes.",
    },
    D3BenchmarkCoverageCase {
        id: "force-many-body-barnes-hut-5000",
        module: "force",
        bench_target: "force_many_body",
        benchmark_group: "force_many_body",
        benchmark_id: "barnes_hut/5000",
        dataset_scale: "5,000 nodes",
        status: D3BenchmarkCoverageStatus::CriterionBench,
        evidence: "Existing force_many_body Criterion bench compares Barnes-Hut many-body application at 5,000 deterministic nodes.",
    },
    D3BenchmarkCoverageCase {
        id: "quadtree-build-10000",
        module: "quadtree",
        bench_target: "d3_large_datasets",
        benchmark_group: "quadtree",
        benchmark_id: "try_from_data/10000",
        dataset_scale: "10,000 points",
        status: D3BenchmarkCoverageStatus::CriterionBench,
        evidence: "d3_large_datasets builds a checked quadtree over deterministic 2D points.",
    },
    D3BenchmarkCoverageCase {
        id: "quadtree-find-10000",
        module: "quadtree",
        bench_target: "d3_large_datasets",
        benchmark_group: "quadtree",
        benchmark_id: "find/10000",
        dataset_scale: "10,000 points, 512 queries",
        status: D3BenchmarkCoverageStatus::CriterionBench,
        evidence: "d3_large_datasets performs repeated nearest-neighbor queries against the checked quadtree.",
    },
    D3BenchmarkCoverageCase {
        id: "hexbin-try-bin-20000",
        module: "hexbin",
        bench_target: "d3_large_datasets",
        benchmark_group: "hexbin",
        benchmark_id: "try_bin/20000",
        dataset_scale: "20,000 points",
        status: D3BenchmarkCoverageStatus::CriterionBench,
        evidence: "d3_large_datasets bins deterministic point clouds through Hexbin::try_bin.",
    },
    D3BenchmarkCoverageCase {
        id: "delaunay-build-2500",
        module: "delaunay",
        bench_target: "d3_large_datasets",
        benchmark_group: "delaunay",
        benchmark_id: "try_new/2500",
        dataset_scale: "2,500 points",
        status: D3BenchmarkCoverageStatus::CriterionBench,
        evidence: "d3_large_datasets constructs checked Delaunay triangulations over deterministic point clouds.",
    },
    D3BenchmarkCoverageCase {
        id: "delaunay-voronoi-render-2500",
        module: "delaunay",
        bench_target: "d3_large_datasets",
        benchmark_group: "delaunay",
        benchmark_id: "voronoi_render/2500",
        dataset_scale: "2,500 Voronoi cells",
        status: D3BenchmarkCoverageStatus::CriterionBench,
        evidence: "d3_large_datasets renders all checked Voronoi cells into a reusable path buffer.",
    },
    D3BenchmarkCoverageCase {
        id: "sankey-try-compute-240x720",
        module: "sankey",
        bench_target: "d3_large_datasets",
        benchmark_group: "sankey",
        benchmark_id: "try_compute/240x720",
        dataset_scale: "240 nodes, 720 links",
        status: D3BenchmarkCoverageStatus::CriterionBench,
        evidence: "d3_large_datasets computes a checked multi-layer Sankey layout with deterministic weighted links.",
    },
    D3BenchmarkCoverageCase {
        id: "hierarchy-tree-layout-4095",
        module: "hierarchy",
        bench_target: "d3_large_datasets",
        benchmark_group: "hierarchy",
        benchmark_id: "tree_try_layout/4095",
        dataset_scale: "4,095 nodes",
        status: D3BenchmarkCoverageStatus::CriterionBench,
        evidence: "d3_large_datasets builds and lays out a checked balanced hierarchy tree.",
    },
    D3BenchmarkCoverageCase {
        id: "path-write-svg-string",
        module: "path",
        bench_target: "path_strings",
        benchmark_group: "path_strings",
        benchmark_id: "path/write_svg_string",
        dataset_scale: "10,000 path commands",
        status: D3BenchmarkCoverageStatus::CriterionBench,
        evidence: "Existing path_strings Criterion bench covers allocation-conscious SVG path string generation.",
    },
    D3BenchmarkCoverageCase {
        id: "geo-path-render-into",
        module: "geo",
        bench_target: "path_strings",
        benchmark_group: "path_strings",
        benchmark_id: "geo_path/render_into",
        dataset_scale: "deterministic GeoJSON fixture",
        status: D3BenchmarkCoverageStatus::CriterionBench,
        evidence: "Existing path_strings Criterion bench covers reusable-buffer Geo path rendering.",
    },
];

/// Return the current feature parity report.
pub const fn feature_parity_report() -> FeatureParityReport {
    FeatureParityReport {
        schema_version: FEATURE_PARITY_SCHEMA_VERSION,
        report_type: FEATURE_PARITY_REPORT_TYPE,
        reviewed_on: "2026-07-08",
        entries: FEATURE_PARITY_ENTRIES,
    }
}

/// Return all feature parity entries.
pub const fn feature_parity_entries() -> &'static [FeatureParityEntry] {
    FEATURE_PARITY_ENTRIES
}

/// Return the current D3 option parity report for partially implemented D3 areas.
pub const fn d3_option_parity_report() -> D3OptionParityReport {
    D3OptionParityReport {
        schema_version: D3_OPTION_PARITY_SCHEMA_VERSION,
        report_type: D3_OPTION_PARITY_REPORT_TYPE,
        reviewed_on: "2026-07-08",
        entries: D3_OPTION_PARITY_ENTRIES,
    }
}

/// Return all D3 option parity entries.
pub const fn d3_option_parity_entries() -> &'static [D3OptionParityEntry] {
    D3_OPTION_PARITY_ENTRIES
}

/// Return the current large-dataset benchmark coverage report.
pub const fn d3_benchmark_coverage_report() -> D3BenchmarkCoverageReport {
    D3BenchmarkCoverageReport {
        schema_version: D3_BENCHMARK_COVERAGE_SCHEMA_VERSION,
        report_type: D3_BENCHMARK_COVERAGE_REPORT_TYPE,
        reviewed_on: "2026-07-08",
        command: "cargo bench -p gpui-d3rs --bench d3_large_datasets -- --save-baseline release-YYYYMMDD",
        baseline_policy: "Run before each toolkit release candidate, attach Criterion HTML/JSON artifacts, and compare against the previous release baseline for large-dataset regressions.",
        cases: D3_BENCHMARK_COVERAGE_CASES,
    }
}

/// Return all large-dataset benchmark coverage cases.
pub const fn d3_benchmark_coverage_cases() -> &'static [D3BenchmarkCoverageCase] {
    D3_BENCHMARK_COVERAGE_CASES
}

#[cfg(test)]
mod tests {
    use super::{
        D3_BENCHMARK_COVERAGE_REPORT_TYPE, D3_BENCHMARK_COVERAGE_SCHEMA_VERSION,
        D3_OPTION_PARITY_REPORT_TYPE, D3_OPTION_PARITY_SCHEMA_VERSION, FEATURE_PARITY_REPORT_TYPE,
        FEATURE_PARITY_SCHEMA_VERSION, FeatureParityStatus, d3_benchmark_coverage_report,
        d3_option_parity_report, feature_parity_report,
    };

    #[test]
    fn feature_parity_report_has_stable_contract() {
        let report = feature_parity_report();

        assert_eq!(report.schema_version, FEATURE_PARITY_SCHEMA_VERSION);
        assert_eq!(report.report_type, FEATURE_PARITY_REPORT_TYPE);
        assert_eq!(report.reviewed_on, "2026-07-08");
        assert!(!report.entries.is_empty());
        assert!(!report.all_release_ready());
    }

    #[test]
    fn feature_parity_report_has_unique_area_ids() {
        let report = feature_parity_report();
        let mut ids = std::collections::BTreeSet::new();

        for entry in report.entries {
            assert!(ids.insert(entry.id), "duplicate parity entry {}", entry.id);
            assert!(!entry.d3_area.is_empty());
            assert!(!entry.gpui_d3rs_modules.is_empty());
            assert!(!entry.evidence.is_empty());
            assert!(!entry.release_requirement.is_empty());
        }
    }

    #[test]
    fn feature_parity_report_names_checked_input_modules() {
        let report = feature_parity_report();

        assert!(report.entries.iter().any(|entry| {
            entry.id == "chord" && entry.status == FeatureParityStatus::CheckedInputs
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "quadtree" && entry.status == FeatureParityStatus::CheckedInputs
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "hexbin" && entry.status == FeatureParityStatus::CheckedInputs
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "sankey" && entry.status == FeatureParityStatus::CheckedInputs
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "delaunay" && entry.status == FeatureParityStatus::CheckedInputs
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "hierarchy" && entry.status == FeatureParityStatus::CheckedInputs
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "force" && entry.status == FeatureParityStatus::CheckedInputs
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "shape-pie" && entry.status == FeatureParityStatus::CheckedInputs
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "shape-arc" && entry.status == FeatureParityStatus::CheckedInputs
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "shape-area" && entry.status == FeatureParityStatus::CheckedInputs
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "shape-line" && entry.status == FeatureParityStatus::CheckedInputs
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "shape-scatter" && entry.status == FeatureParityStatus::CheckedInputs
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "shape-stack" && entry.status == FeatureParityStatus::CheckedInputs
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "shape-symbol" && entry.status == FeatureParityStatus::CheckedInputs
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "shape-link" && entry.status == FeatureParityStatus::CheckedInputs
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "shape-radial" && entry.status == FeatureParityStatus::CheckedInputs
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "axis-layout" && entry.status == FeatureParityStatus::CheckedInputs
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "grid-layout" && entry.status == FeatureParityStatus::CheckedInputs
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "legend-layout" && entry.status == FeatureParityStatus::CheckedInputs
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "text-layout" && entry.status == FeatureParityStatus::CheckedInputs
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "tile" && entry.status == FeatureParityStatus::CheckedInputs
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "drag" && entry.status == FeatureParityStatus::CheckedInputs
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "selection" && entry.status == FeatureParityStatus::CheckedInputs
        }));
    }

    #[test]
    fn feature_parity_report_names_d3_parity_blockers() {
        let report = feature_parity_report();
        let blocking_ids: Vec<_> = report.blocking_entries().map(|entry| entry.id).collect();

        assert!(blocking_ids.contains(&"force-hierarchy-sankey-hexbin-delaunay"));
        assert!(blocking_ids.contains(&"interaction-animation"));
        assert!(blocking_ids.contains(&"axis-grid-legend-text-gpu"));
        assert!(!blocking_ids.contains(&"missing-d3-modules"));
    }

    #[test]
    fn feature_parity_markdown_names_statuses() {
        let markdown = feature_parity_report().to_markdown_table();

        assert!(markdown.contains("gpui-d3rs-feature-parity"));
        assert!(markdown.contains("d3-chord"));
        assert!(markdown.contains("d3-axis"));
        assert!(markdown.contains("Cartesian grids"));
        assert!(markdown.contains("Chart legends"));
        assert!(markdown.contains("Chart text layout"));
        assert!(markdown.contains("d3-tile"));
        assert!(markdown.contains("d3-drag"));
        assert!(markdown.contains("d3-selection"));
        assert!(markdown.contains("checked-inputs"));
        assert!(markdown.contains("partial"));
    }

    #[test]
    fn d3_option_parity_report_has_stable_contract() {
        let report = d3_option_parity_report();

        assert_eq!(report.schema_version, D3_OPTION_PARITY_SCHEMA_VERSION);
        assert_eq!(report.report_type, D3_OPTION_PARITY_REPORT_TYPE);
        assert_eq!(report.reviewed_on, "2026-07-08");
        assert!(!report.entries.is_empty());
    }

    #[test]
    fn d3_option_parity_report_covers_partial_d3_modules() {
        let report = d3_option_parity_report();
        let modules: std::collections::BTreeSet<_> =
            report.entries.iter().map(|entry| entry.module).collect();

        for module in ["force", "hierarchy", "sankey", "hexbin", "delaunay"] {
            assert!(
                modules.contains(module),
                "missing option table for {module}"
            );
            assert!(
                report.entries_for_module(module).next().is_some(),
                "entries_for_module did not return {module}"
            );
        }
    }

    #[test]
    fn d3_option_parity_report_has_unique_option_rows() {
        let report = d3_option_parity_report();
        let mut keys = std::collections::BTreeSet::new();

        for entry in report.entries {
            assert!(
                keys.insert((entry.module, entry.d3_option)),
                "duplicate option parity row {} / {}",
                entry.module,
                entry.d3_option
            );
            assert!(!entry.gpui_surface.is_empty());
            assert!(!entry.notes.is_empty());
        }
    }

    #[test]
    fn d3_option_parity_report_names_blocking_options() {
        let report = d3_option_parity_report();
        let blocking: Vec<_> = report
            .blocking_entries()
            .map(|entry| (entry.module, entry.d3_option))
            .collect();

        assert!(!blocking.iter().any(|&(module, _)| module == "force"));
        assert!(!blocking.contains(&("hierarchy", "tree.separation")));
        assert!(!blocking.iter().any(|&(module, _)| module == "delaunay"));
        assert!(!blocking.iter().any(|&(module, _)| module == "hexbin"));
        assert!(!blocking.iter().any(|&(module, _)| module == "sankey"));
        assert!(blocking.is_empty());
    }

    #[test]
    fn d3_option_parity_markdown_names_statuses() {
        let markdown = d3_option_parity_report().to_markdown_table();

        assert!(markdown.contains("gpui-d3rs-d3-option-parity"));
        assert!(markdown.contains("forceManyBody"));
        assert!(markdown.contains("supported"));
    }

    #[test]
    fn d3_benchmark_coverage_report_has_stable_contract() {
        let report = d3_benchmark_coverage_report();

        assert_eq!(report.schema_version, D3_BENCHMARK_COVERAGE_SCHEMA_VERSION);
        assert_eq!(report.report_type, D3_BENCHMARK_COVERAGE_REPORT_TYPE);
        assert_eq!(report.reviewed_on, "2026-07-08");
        assert!(report.command.contains("d3_large_datasets"));
        assert!(report.case_count() >= 8);
    }

    #[test]
    fn d3_benchmark_coverage_report_covers_large_d3_modules() {
        let report = d3_benchmark_coverage_report();
        let modules: std::collections::BTreeSet<_> =
            report.cases.iter().map(|case| case.module).collect();

        for module in [
            "force",
            "quadtree",
            "hexbin",
            "delaunay",
            "sankey",
            "hierarchy",
        ] {
            assert!(
                modules.contains(module),
                "missing benchmark coverage for {module}"
            );
            assert!(
                report.cases_for_module(module).next().is_some(),
                "cases_for_module did not return {module}"
            );
        }
    }

    #[test]
    fn d3_benchmark_coverage_report_has_unique_case_ids() {
        let report = d3_benchmark_coverage_report();
        let mut ids = std::collections::BTreeSet::new();

        for case in report.cases {
            assert!(
                ids.insert(case.id),
                "duplicate benchmark coverage case {}",
                case.id
            );
            assert!(!case.bench_target.is_empty());
            assert!(!case.benchmark_group.is_empty());
            assert!(!case.benchmark_id.is_empty());
            assert!(!case.dataset_scale.is_empty());
            assert!(!case.evidence.is_empty());
        }
    }

    #[test]
    fn d3_benchmark_coverage_markdown_names_cases() {
        let markdown = d3_benchmark_coverage_report().to_markdown_table();

        assert!(markdown.contains("gpui-d3rs-large-dataset-benchmarks"));
        assert!(markdown.contains("quadtree"));
        assert!(markdown.contains("try_compute/240x720"));
        assert!(markdown.contains("criterion-bench"));
    }
}
