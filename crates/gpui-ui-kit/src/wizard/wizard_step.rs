use gpui::SharedString;

/// A single step in the wizard
#[derive(Clone)]
pub struct WizardStep {
    /// Unique identifier for this step
    pub id: SharedString,
    /// Display label for this step
    pub label: SharedString,
    /// Optional description
    pub description: Option<SharedString>,
    /// Optional icon (emoji or text)
    pub icon: Option<SharedString>,
    /// Whether this step can be skipped
    pub can_skip: bool,
    /// Whether this step is disabled
    pub disabled: bool,
}

impl WizardStep {
    /// Create a new wizard step
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: None,
            icon: None,
            can_skip: false,
            disabled: false,
        }
    }

    /// Add a description
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add an icon
    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set whether this step can be skipped
    pub fn can_skip(mut self, can_skip: bool) -> Self {
        self.can_skip = can_skip;
        self
    }

    /// Set whether this step is disabled
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}
