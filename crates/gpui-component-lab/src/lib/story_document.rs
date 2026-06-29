use super::component_story::ComponentStory;
use super::types::StoryPropValue;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

/// Persisted designer/story state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoryDocument {
    pub story: ComponentStory,
    #[serde(default)]
    pub layout: Value,
}

impl StoryDocument {
    pub fn new(story: ComponentStory) -> Self {
        Self {
            story,
            layout: Value::Object(Default::default()),
        }
    }

    pub fn set_prop_value(&mut self, name: &str, value: StoryPropValue) -> Result<()> {
        let Some(prop) = self.story.props.iter_mut().find(|prop| prop.name == name) else {
            bail!("story '{}' has no prop '{name}'", self.story.id);
        };
        prop.value = value;
        Ok(())
    }

    pub fn save_story_json(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self).context("serialize story document")?;
        std::fs::write(path, json).with_context(|| format!("write {}", path.display()))
    }

    pub fn load_story_json(path: &Path) -> Result<Self> {
        let input =
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        serde_json::from_str(&input).with_context(|| format!("parse {}", path.display()))
    }
}

/// Load all story documents from a directory.
pub fn load_story_documents(dir: &Path) -> Result<Vec<StoryDocument>> {
    let mut docs = Vec::new();
    if !dir.exists() {
        return Ok(docs);
    }
    for entry in std::fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json")
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".story.json"))
        {
            docs.push(StoryDocument::load_story_json(&path)?);
        }
    }
    Ok(docs)
}
