use gpui::SharedString;

/// A single menu item
#[derive(Clone)]
pub struct MenuItem {
    pub(super) id: SharedString,
    pub(super) label: SharedString,
    pub(super) shortcut: Option<SharedString>,
    pub(super) icon: Option<SharedString>,
    pub(super) disabled: bool,
    pub(super) is_separator: bool,
    pub(super) is_checkbox: bool,
    pub(super) checked: bool,
    pub(super) is_danger: bool,
    pub(super) children: Vec<MenuItem>,
    /// Pre-computed element ID so `Menu::build_with_theme` does not format a
    /// string for every item on every render.
    pub(super) element_id: SharedString,
}

impl MenuItem {
    fn make_element_id(id: &SharedString) -> SharedString {
        SharedString::from(format!("menu-item-{id}"))
    }

    /// Create a new menu item
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        let id: SharedString = id.into();
        let element_id = Self::make_element_id(&id);
        Self {
            id,
            label: label.into(),
            shortcut: None,
            icon: None,
            disabled: false,
            is_separator: false,
            is_checkbox: false,
            checked: false,
            is_danger: false,
            children: Vec::new(),
            element_id,
        }
    }

    /// Create a separator item
    pub fn separator() -> Self {
        let id: SharedString = "separator".into();
        let element_id = Self::make_element_id(&id);
        Self {
            id,
            label: "".into(),
            shortcut: None,
            icon: None,
            disabled: true,
            is_separator: true,
            is_checkbox: false,
            checked: false,
            is_danger: false,
            children: Vec::new(),
            element_id,
        }
    }

    /// Create a checkbox menu item
    pub fn checkbox(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        checked: bool,
    ) -> Self {
        let id: SharedString = id.into();
        let element_id = Self::make_element_id(&id);
        Self {
            id,
            label: label.into(),
            shortcut: None,
            icon: None,
            disabled: false,
            is_separator: false,
            is_checkbox: true,
            checked,
            is_danger: false,
            children: Vec::new(),
            element_id,
        }
    }

    /// Add a keyboard shortcut display
    pub fn with_shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Add an icon
    pub fn with_icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Disable the menu item
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Add submenu items
    pub fn with_children(mut self, children: Vec<MenuItem>) -> Self {
        self.children = children;
        self
    }

    /// Get the item ID
    pub fn id(&self) -> &SharedString {
        &self.id
    }

    /// Check if this is a separator
    pub fn is_separator(&self) -> bool {
        self.is_separator
    }

    /// Mark as a danger/destructive action (e.g., Quit, Delete)
    pub fn danger(mut self) -> Self {
        self.is_danger = true;
        self
    }

    /// Check if this is a danger item
    pub fn is_danger(&self) -> bool {
        self.is_danger
    }
}
