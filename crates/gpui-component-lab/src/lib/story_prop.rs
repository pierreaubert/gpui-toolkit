use super::types::StoryPropValue;
use serde::{Deserialize, Serialize};

/// Editable story property definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoryProp {
    pub name: String,
    pub label: String,
    pub value: StoryPropValue,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
}

impl StoryProp {
    pub fn new(name: impl Into<String>, label: impl Into<String>, value: StoryPropValue) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            value,
            options: Vec::new(),
        }
    }

    pub fn options(mut self, options: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.options = options.into_iter().map(Into::into).collect();
        self
    }
}
