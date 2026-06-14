//! Responsive layout snapshot helpers.
//!
//! These helpers solve one layout declaration across named viewport sizes and
//! produce stable text output for examples, CI logs, and regression tests.

use std::fmt::Write as _;

use crate::util::format_number;
use crate::{Axis, LayoutNode, LayoutPreferences, SolvedNode, solve};

/// A named viewport used for responsive layout snapshots.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutViewport<'a> {
    /// Human-readable viewport label, such as `desktop` or `phone_portrait`.
    pub label: &'a str,
    /// Viewport width in pixels.
    pub width: f32,
    /// Viewport height in pixels.
    pub height: f32,
}

impl<'a> LayoutViewport<'a> {
    /// Construct a named viewport.
    pub const fn new(label: &'a str, width: f32, height: f32) -> Self {
        Self {
            label,
            width,
            height,
        }
    }
}

/// A solved layout for one viewport.
#[derive(Debug, Clone)]
pub struct LayoutSnapshot<'a> {
    /// Viewport label.
    pub label: String,
    /// Viewport width in pixels.
    pub width: f32,
    /// Viewport height in pixels.
    pub height: f32,
    /// Solved root node for this viewport.
    pub root: SolvedNode<'a>,
}

impl<'a> LayoutSnapshot<'a> {
    /// Return the ids of visible nodes in depth-first order.
    pub fn visible_ids(&self) -> Vec<&str> {
        let mut ids = Vec::new();
        collect_visible_ids(&self.root, &mut ids);
        ids
    }

    /// Return collapsed tabs as `id:label` pairs in depth-first order.
    pub fn collapsed_labels(&self) -> Vec<String> {
        self.root
            .collapsed_tabs()
            .into_iter()
            .map(|(id, label)| format!("{id}:{label}"))
            .collect()
    }

    /// Return active display tiers as `id:tier` pairs in depth-first order.
    pub fn active_tiers(&self) -> Vec<String> {
        let mut tiers = Vec::new();
        collect_active_tiers(&self.root, &mut tiers);
        tiers
    }

    /// Return resolved container axes as `id:axis` pairs in depth-first order.
    pub fn resolved_axes(&self) -> Vec<String> {
        let mut axes = Vec::new();
        collect_resolved_axes(&self.root, &mut axes);
        axes
    }

    /// Render a stable tree for this viewport.
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        let _ = writeln!(
            output,
            "## {} ({}x{})",
            self.label,
            format_number(self.width),
            format_number(self.height)
        );
        append_node_text(&mut output, &self.root, 0);

        let collapsed = self.collapsed_labels();
        if !collapsed.is_empty() {
            let _ = writeln!(output, "collapsed: {}", collapsed.join(", "));
        }

        let tiers = self.active_tiers();
        if !tiers.is_empty() {
            let _ = writeln!(output, "tiers: {}", tiers.join(", "));
        }

        output
    }
}

/// A set of solved snapshots for one layout declaration.
#[derive(Debug, Clone, Default)]
pub struct LayoutSnapshotMatrix<'a> {
    /// Snapshots in the same order as the input viewports.
    pub snapshots: Vec<LayoutSnapshot<'a>>,
}

impl<'a> LayoutSnapshotMatrix<'a> {
    /// Return true when there are no snapshots.
    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    /// Render every snapshot as a stable tree-oriented text report.
    pub fn to_text(&self) -> String {
        let mut sections = Vec::with_capacity(self.snapshots.len());
        for snapshot in &self.snapshots {
            sections.push(snapshot.to_text());
        }
        sections.join("\n")
    }

    /// Render a compact Markdown table for breakpoint diffs and PR notes.
    pub fn to_markdown_table(&self) -> String {
        let mut output = String::from(
            "| viewport | size | axes | visible | collapsed | tiers |\n\
             | --- | ---: | --- | --- | --- | --- |\n",
        );

        for snapshot in &self.snapshots {
            let axes = display_list(snapshot.resolved_axes());
            let visible = display_list(snapshot.visible_ids());
            let collapsed = display_list(snapshot.collapsed_labels());
            let tiers = display_list(snapshot.active_tiers());
            let _ = writeln!(
                output,
                "| {} | {}x{} | {} | {} | {} | {} |",
                escape_table_cell(&snapshot.label),
                format_number(snapshot.width),
                format_number(snapshot.height),
                escape_table_cell(&axes),
                escape_table_cell(&visible),
                escape_table_cell(&collapsed),
                escape_table_cell(&tiers),
            );
        }

        output
    }
}

/// Solve a layout for each viewport and return the snapshot matrix.
pub fn solve_snapshot_matrix<'a>(
    root: &LayoutNode<'a>,
    viewports: &[LayoutViewport<'a>],
    prefs: &LayoutPreferences<'a>,
) -> LayoutSnapshotMatrix<'a> {
    let snapshots = viewports
        .iter()
        .map(|viewport| LayoutSnapshot {
            label: viewport.label.to_string(),
            width: viewport.width,
            height: viewport.height,
            root: solve(root, viewport.width, viewport.height, prefs),
        })
        .collect();

    LayoutSnapshotMatrix { snapshots }
}

fn collect_visible_ids<'b>(node: &SolvedNode<'b>, ids: &mut Vec<&'b str>) {
    if node.visible {
        ids.push(node.id);
    }
    for child in &node.children {
        collect_visible_ids(child, ids);
    }
}

fn collect_active_tiers(node: &SolvedNode<'_>, tiers: &mut Vec<String>) {
    if let Some(tier) = node.active_tier {
        tiers.push(format!("{}:{tier}", node.id));
    }
    for child in &node.children {
        collect_active_tiers(child, tiers);
    }
}

fn collect_resolved_axes(node: &SolvedNode<'_>, axes: &mut Vec<String>) {
    if let Some(axis) = node.resolved_axis {
        axes.push(format!("{}:{}", node.id, axis_label(axis)));
    }
    for child in &node.children {
        collect_resolved_axes(child, axes);
    }
}

fn append_node_text(output: &mut String, node: &SolvedNode<'_>, depth: usize) {
    let indent = "  ".repeat(depth);
    let mut suffix = String::new();

    if let Some(axis) = node.resolved_axis {
        let _ = write!(suffix, " axis={}", axis_label(axis));
    }
    if let Some(tier) = node.active_tier {
        let _ = write!(suffix, " tier={tier}");
    }
    if !node.visible {
        suffix.push_str(" collapsed");
        if let Some(label) = node.collapse_label {
            let _ = write!(suffix, " label={label:?}");
        }
    }

    let _ = writeln!(
        output,
        "{indent}{} {}x{}{}",
        node.id,
        format_number(node.width),
        format_number(node.height),
        suffix
    );

    for child in &node.children {
        append_node_text(output, child, depth + 1);
    }
}

fn display_list(items: Vec<impl AsRef<str>>) -> String {
    if items.is_empty() {
        "none".to_string()
    } else {
        items
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn axis_label(axis: Axis) -> &'static str {
    match axis {
        Axis::Horizontal => "horizontal",
        Axis::Vertical => "vertical",
    }
}

fn escape_table_cell(value: &str) -> String {
    value.replace('|', "\\|")
}

#[cfg(test)]
mod tests {
    use super::{LayoutNode, LayoutPreferences, LayoutViewport, solve_snapshot_matrix};
    use crate::{Axis, ContainerNode, DisplayTier, Sizing, SlotNode};

    static CHART_TIERS: &[DisplayTier<'_>] = &[
        DisplayTier {
            name: "full",
            min_size: 300.0,
        },
        DisplayTier {
            name: "compact",
            min_size: 150.0,
        },
    ];

    fn dashboard_root<'a>(root_children: &'a [LayoutNode<'a>]) -> LayoutNode<'a> {
        LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Vertical,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: root_children,
            divider_size: 0.0,
        })
    }

    #[test]
    fn snapshot_matrix_solves_each_viewport_in_order() {
        let chart_children = [
            LayoutNode::Slot(SlotNode {
                id: "chart",
                sizing: Sizing::flex(100.0),
                priority: 1.0,
                collapsible: false,
                display_tiers: CHART_TIERS,
                collapse_label: None,
            }),
            LayoutNode::Slot(SlotNode {
                id: "table",
                sizing: Sizing::flex(80.0),
                priority: 0.4,
                collapsible: true,
                display_tiers: &[],
                collapse_label: Some("Table"),
            }),
        ];
        let content_children = [
            LayoutNode::Slot(SlotNode {
                id: "sidebar",
                sizing: Sizing::fractional(0.25, 120.0),
                priority: 0.3,
                collapsible: true,
                display_tiers: &[],
                collapse_label: Some("Nav"),
            }),
            LayoutNode::Container(ContainerNode {
                id: "main",
                axis: Axis::Vertical,
                auto_axis: None,
                sizing: Sizing::flex(200.0),
                children: &chart_children,
                divider_size: 2.0,
            }),
        ];
        let root_children = [
            LayoutNode::Slot(SlotNode {
                id: "toolbar",
                sizing: Sizing::Fixed(48.0),
                priority: 1.0,
                collapsible: false,
                display_tiers: &[],
                collapse_label: None,
            }),
            LayoutNode::Container(ContainerNode {
                id: "content",
                axis: Axis::Horizontal,
                auto_axis: Some(1.0),
                sizing: Sizing::flex(0.0),
                children: &content_children,
                divider_size: 4.0,
            }),
            LayoutNode::Slot(SlotNode {
                id: "status",
                sizing: Sizing::Fixed(24.0),
                priority: 1.0,
                collapsible: false,
                display_tiers: &[],
                collapse_label: None,
            }),
        ];
        let root = dashboard_root(&root_children);
        let viewports = [
            LayoutViewport::new("desktop", 1200.0, 800.0),
            LayoutViewport::new("portrait", 500.0, 800.0),
        ];

        let matrix = solve_snapshot_matrix(&root, &viewports, &LayoutPreferences::default());

        assert_eq!(matrix.snapshots.len(), 2);
        assert_eq!(matrix.snapshots[0].label, "desktop");
        assert_eq!(
            matrix.snapshots[0]
                .root
                .find("content")
                .unwrap()
                .resolved_axis,
            Some(Axis::Horizontal)
        );
        assert_eq!(
            matrix.snapshots[1]
                .root
                .find("content")
                .unwrap()
                .resolved_axis,
            Some(Axis::Vertical)
        );
    }

    #[test]
    fn snapshot_matrix_markdown_is_stable() {
        let children = [
            LayoutNode::Slot(SlotNode {
                id: "chart",
                sizing: Sizing::flex(0.0),
                priority: 1.0,
                collapsible: false,
                display_tiers: CHART_TIERS,
                collapse_label: None,
            }),
            LayoutNode::Slot(SlotNode {
                id: "details",
                sizing: Sizing::Fixed(120.0),
                priority: 0.2,
                collapsible: true,
                display_tiers: &[],
                collapse_label: Some("Details"),
            }),
        ];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Horizontal,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 0.0,
        });
        let matrix = solve_snapshot_matrix(
            &root,
            &[LayoutViewport::new("narrow", 80.0, 240.0)],
            &LayoutPreferences::default(),
        );

        assert_eq!(
            matrix.to_markdown_table(),
            concat!(
                "| viewport | size | axes | visible | collapsed | tiers |\n",
                "| --- | ---: | --- | --- | --- | --- |\n",
                "| narrow | 80x240 | root:horizontal | root, chart | ",
                "details:Details | none |\n",
            )
        );
        assert_eq!(
            matrix.to_text(),
            concat!(
                "## narrow (80x240)\n",
                "root 80x240 axis=horizontal\n",
                "  chart 80x240\n",
                "  details 0x0 collapsed label=\"Details\"\n",
                "collapsed: details:Details\n",
            )
        );
    }
}
