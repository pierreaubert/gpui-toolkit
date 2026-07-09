//! Renderer-free keyboard navigation helpers for list-like data widgets.

/// Keyboard action used by data-navigation widgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataNavigationAction {
    Previous,
    Next,
    First,
    Last,
    Activate,
    Expand,
    Collapse,
    Dismiss,
}

impl DataNavigationAction {
    /// Map GPUI key strings to data-navigation actions.
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "up" => Some(Self::Previous),
            "down" => Some(Self::Next),
            "home" => Some(Self::First),
            "end" => Some(Self::Last),
            "enter" | "space" => Some(Self::Activate),
            "right" => Some(Self::Expand),
            "left" => Some(Self::Collapse),
            "escape" => Some(Self::Dismiss),
            _ => None,
        }
    }
}

/// Stable selection state for an ordered data-navigation surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataNavigationState {
    pub item_count: usize,
    pub selected_index: Option<usize>,
    pub wraparound: bool,
}

impl DataNavigationState {
    pub const fn new(item_count: usize) -> Self {
        Self {
            item_count,
            selected_index: None,
            wraparound: false,
        }
    }

    pub const fn selected_index(mut self, index: Option<usize>) -> Self {
        self.selected_index = index;
        self
    }

    pub const fn wraparound(mut self, wraparound: bool) -> Self {
        self.wraparound = wraparound;
        self
    }

    /// Return the selected index after a movement action.
    pub fn move_selection(self, action: DataNavigationAction) -> Option<usize> {
        if self.item_count == 0 {
            return None;
        }

        let selected = self.selected_index.filter(|index| *index < self.item_count);

        match action {
            DataNavigationAction::Previous => match selected {
                Some(index) if index > 0 => Some(index - 1),
                Some(_) if self.wraparound => Some(self.item_count - 1),
                Some(index) => Some(index),
                None => Some(self.item_count - 1),
            },
            DataNavigationAction::Next => match selected {
                Some(index) if index + 1 < self.item_count => Some(index + 1),
                Some(_) if self.wraparound => Some(0),
                Some(index) => Some(index),
                None => Some(0),
            },
            DataNavigationAction::First => Some(0),
            DataNavigationAction::Last => Some(self.item_count - 1),
            _ => selected,
        }
    }
}

/// Renderer-free virtual window for large ordered data surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataVirtualWindow {
    pub total_items: usize,
    pub start: usize,
    pub end: usize,
}

impl DataVirtualWindow {
    pub const fn full(total_items: usize) -> Self {
        Self {
            total_items,
            start: 0,
            end: total_items,
        }
    }

    pub fn new(total_items: usize, start: usize, end: usize) -> Self {
        let start = start.min(total_items);
        let end = end.min(total_items).max(start);
        Self {
            total_items,
            start,
            end,
        }
    }

    pub fn from_viewport(
        total_items: usize,
        scroll_offset: f32,
        item_extent: f32,
        viewport_extent: f32,
        overscan: usize,
    ) -> Self {
        if total_items == 0 {
            return Self::full(0);
        }

        if !scroll_offset.is_finite()
            || !item_extent.is_finite()
            || !viewport_extent.is_finite()
            || item_extent <= 0.0
            || viewport_extent <= 0.0
        {
            return Self::full(total_items);
        }

        let first_visible = (scroll_offset.max(0.0) / item_extent).floor() as usize;
        let visible_count = (viewport_extent / item_extent).ceil().max(1.0) as usize;
        let start = first_visible.saturating_sub(overscan);
        let end = first_visible
            .saturating_add(visible_count)
            .saturating_add(overscan)
            .min(total_items);
        Self::new(total_items, start, end)
    }

    pub fn with_total(self, total_items: usize) -> Self {
        Self::new(total_items, self.start, self.end)
    }

    pub const fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub const fn is_empty(self) -> bool {
        self.start >= self.end
    }

    pub const fn before_count(self) -> usize {
        self.start
    }

    pub const fn after_count(self) -> usize {
        self.total_items.saturating_sub(self.end)
    }

    pub const fn contains(self, index: usize) -> bool {
        index >= self.start && index < self.end
    }

    pub fn ensure_index_visible(self, index: usize) -> Self {
        if self.total_items == 0 || index >= self.total_items || self.contains(index) {
            return self;
        }

        let window_len = self.len().max(1).min(self.total_items);
        if index < self.start {
            return Self::new(self.total_items, index, index.saturating_add(window_len));
        }

        let end = index.saturating_add(1).min(self.total_items);
        let start = end.saturating_sub(window_len);
        Self::new(self.total_items, start, end)
    }
}

/// Return the moved key for a visible keyed collection.
pub fn move_key<T: PartialEq + Clone>(
    visible_keys: &[T],
    current: Option<&T>,
    action: DataNavigationAction,
    wraparound: bool,
) -> Option<T> {
    let selected_index =
        current.and_then(|current| visible_keys.iter().position(|key| key == current));
    DataNavigationState::new(visible_keys.len())
        .selected_index(selected_index)
        .wraparound(wraparound)
        .move_selection(action)
        .and_then(|index| visible_keys.get(index).cloned())
}

#[cfg(test)]
mod tests {
    use super::{DataNavigationAction, DataNavigationState, DataVirtualWindow, move_key};

    #[test]
    fn data_navigation_maps_common_keys() {
        assert_eq!(
            DataNavigationAction::from_key("down"),
            Some(DataNavigationAction::Next)
        );
        assert_eq!(
            DataNavigationAction::from_key("enter"),
            Some(DataNavigationAction::Activate)
        );
        assert_eq!(DataNavigationAction::from_key("tab"), None);
    }

    #[test]
    fn data_navigation_moves_indices_with_boundaries() {
        let state = DataNavigationState::new(3).selected_index(Some(1));

        assert_eq!(
            state.move_selection(DataNavigationAction::Previous),
            Some(0)
        );
        assert_eq!(state.move_selection(DataNavigationAction::Next), Some(2));
        assert_eq!(state.move_selection(DataNavigationAction::First), Some(0));
        assert_eq!(state.move_selection(DataNavigationAction::Last), Some(2));
        assert_eq!(
            DataNavigationState::new(3)
                .selected_index(Some(2))
                .move_selection(DataNavigationAction::Next),
            Some(2)
        );
    }

    #[test]
    fn data_navigation_wraps_when_enabled() {
        assert_eq!(
            DataNavigationState::new(3)
                .selected_index(Some(2))
                .wraparound(true)
                .move_selection(DataNavigationAction::Next),
            Some(0)
        );
        assert_eq!(
            DataNavigationState::new(3)
                .selected_index(Some(0))
                .wraparound(true)
                .move_selection(DataNavigationAction::Previous),
            Some(2)
        );
    }

    #[test]
    fn data_navigation_moves_visible_keys() {
        let keys = ["a", "b", "c"];

        assert_eq!(
            move_key(&keys, Some(&"b"), DataNavigationAction::Next, false),
            Some("c")
        );
        assert_eq!(
            move_key(&keys, None, DataNavigationAction::Previous, false),
            Some("c")
        );
    }

    #[test]
    fn virtual_window_clamps_manual_ranges() {
        let window = DataVirtualWindow::new(10, 8, 99);

        assert_eq!(window.start, 8);
        assert_eq!(window.end, 10);
        assert_eq!(window.len(), 2);
        assert_eq!(window.before_count(), 8);
        assert_eq!(window.after_count(), 0);
        assert!(window.contains(8));
        assert!(!window.contains(7));
    }

    #[test]
    fn virtual_window_computes_viewport_ranges_with_overscan() {
        let window = DataVirtualWindow::from_viewport(100, 45.0, 10.0, 35.0, 2);

        assert_eq!(window.start, 2);
        assert_eq!(window.end, 10);
        assert_eq!(window.total_items, 100);
    }

    #[test]
    fn virtual_window_falls_back_to_full_for_invalid_geometry() {
        assert_eq!(
            DataVirtualWindow::from_viewport(7, 0.0, 0.0, 100.0, 2),
            DataVirtualWindow::full(7)
        );
        assert_eq!(
            DataVirtualWindow::from_viewport(7, f32::NAN, 10.0, 100.0, 2),
            DataVirtualWindow::full(7)
        );
    }

    #[test]
    fn virtual_window_can_keep_focused_index_visible() {
        let window = DataVirtualWindow::new(20, 5, 10);

        assert_eq!(
            window.ensure_index_visible(2),
            DataVirtualWindow::new(20, 2, 7)
        );
        assert_eq!(
            window.ensure_index_visible(14),
            DataVirtualWindow::new(20, 10, 15)
        );
        assert_eq!(window.ensure_index_visible(7), window);
    }
}
