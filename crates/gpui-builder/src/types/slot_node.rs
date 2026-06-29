use super::DisplayTier;
use super::layout_node::LayoutNode;
use super::sizing::Sizing;

/// A leaf node: a named slot where the consumer renders content.
#[derive(Debug, Clone, Copy)]
pub struct SlotNode<'a> {
    /// Unique identifier (e.g., "library", "queue", "header").
    pub id: &'a str,
    /// How this slot claims space.
    pub sizing: Sizing<'a>,
    /// Collapse priority: 0.0 = collapses first, 1.0 = never collapses.
    pub priority: f32,
    /// Whether this slot can be collapsed when space is insufficient.
    pub collapsible: bool,
    /// Display mode tiers based on resolved size (evaluated largest-first).
    pub display_tiers: &'a [DisplayTier<'a>],
    /// Tab label when this slot is collapsed. `None` = no tab.
    pub collapse_label: Option<&'a str>,
}

impl<'a> SlotNode<'a> {
    /// Construct a slot with the standard non-collapsible defaults.
    pub const fn new(id: &'a str, sizing: Sizing<'a>) -> Self {
        Self {
            id,
            sizing,
            priority: 1.0,
            collapsible: false,
            display_tiers: &[],
            collapse_label: None,
        }
    }

    /// Set the collapse priority.
    pub const fn priority(mut self, priority: f32) -> Self {
        self.priority = priority;
        self
    }

    /// Mark the slot as collapsible and set its collapse tab label.
    pub const fn collapsible(mut self, priority: f32, collapse_label: &'a str) -> Self {
        self.priority = priority;
        self.collapsible = true;
        self.collapse_label = Some(collapse_label);
        self
    }

    /// Set display tiers used by responsive renderers.
    pub const fn display_tiers(mut self, display_tiers: &'a [DisplayTier<'a>]) -> Self {
        self.display_tiers = display_tiers;
        self
    }

    /// Set or clear the collapse tab label without changing collapsibility.
    pub const fn collapse_label(mut self, collapse_label: Option<&'a str>) -> Self {
        self.collapse_label = collapse_label;
        self
    }

    /// Convert this slot into a layout node.
    pub const fn into_node(self) -> LayoutNode<'a> {
        LayoutNode::Slot(self)
    }
}
