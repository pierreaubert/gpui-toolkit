use serde::{Deserialize, Serialize};

/// One key/value metadata item shown in the story inspector and JSON output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryMetadataItem {
    pub id: String,
    pub label: String,
    pub value: String,
}

impl StoryMetadataItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value: value.into(),
        }
    }
}
