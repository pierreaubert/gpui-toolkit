//! Simulator debug hot-reload manifest support.
//!
//! The Swift shell owns `dlopen`/`dlsym`; Rust validates and publishes the
//! manifest format used by watcher scripts and debug builds.

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotReloadManifest {
    pub dylib_path: PathBuf,
    pub entry_symbol: String,
    pub generation: u64,
}

impl HotReloadManifest {
    pub fn new(dylib_path: impl Into<PathBuf>, entry_symbol: impl Into<String>) -> Self {
        Self {
            dylib_path: dylib_path.into(),
            entry_symbol: entry_symbol.into(),
            generation: 0,
        }
    }

    pub fn generation(mut self, generation: u64) -> Self {
        self.generation = generation;
        self
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.dylib_path.as_os_str().is_empty() {
            return Err("hot reload dylib path must not be empty".to_string());
        }
        if self.entry_symbol.trim().is_empty() {
            return Err("hot reload entry symbol must not be empty".to_string());
        }
        Ok(())
    }

    pub fn to_manifest_text(&self) -> Result<String, String> {
        self.validate()?;
        Ok(format!(
            "dylib_path={}\nentry_symbol={}\ngeneration={}\n",
            self.dylib_path.display(),
            self.entry_symbol,
            self.generation
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotReloadState {
    pub active_generation: u64,
    pub loaded_dylib_path: Option<PathBuf>,
}

impl HotReloadState {
    pub fn should_reload(&self, manifest: &HotReloadManifest) -> bool {
        manifest.generation > self.active_generation
            || self.loaded_dylib_path.as_ref() != Some(&manifest.dylib_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hot_reload_manifest_validates_and_serializes() {
        let manifest =
            HotReloadManifest::new("/tmp/libshowcase.dylib", "showcase_ios_start").generation(7);

        let text = manifest.to_manifest_text().unwrap();
        assert!(text.contains("generation=7"));
        assert!(
            HotReloadState {
                active_generation: 6,
                loaded_dylib_path: None,
            }
            .should_reload(&manifest)
        );
    }
}
