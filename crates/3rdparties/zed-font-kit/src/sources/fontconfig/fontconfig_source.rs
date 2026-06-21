use crate::error::SelectionError;
use crate::family_handle::FamilyHandle;
use crate::family_name::FamilyName;
use crate::handle::Handle;
use crate::properties::Properties;
use crate::source::Source;
use std::any::Any;
use super::misc::fc;

/// A source that contains the fonts installed on the system, as reported by the Fontconfig
/// library.
///
/// On macOS and Windows, the Cargo feature `source-fontconfig` can be used to opt into fontconfig
/// support. To prefer it over the native font source (only if you know what you're doing), use the
/// `source-fontconfig-default` feature.
#[allow(missing_debug_implementations)]
pub struct FontconfigSource {
    pub(super) config: fc::Config,
}

impl Default for FontconfigSource {
    fn default() -> Self {
        Self::new()
    }
}

impl FontconfigSource {
    /// Initializes Fontconfig and prepares it for queries.
    pub fn new() -> FontconfigSource {
        FontconfigSource {
            config: fc::Config::new(),
        }
    }

    /// Returns paths of all fonts installed on the system.
    pub fn all_fonts(&self) -> Result<Vec<Handle>, SelectionError> {
        let pattern = fc::Pattern::new();

        // We want the family name.
        let mut object_set = fc::ObjectSet::new();
        object_set.push_string(fc::Object::File);
        object_set.push_string(fc::Object::Index);

        let patterns = pattern
            .list(&self.config, object_set)
            .map_err(|_| SelectionError::NotFound)?;

        let mut handles = vec![];
        for patt in patterns {
            let path = match patt.get_string(fc::Object::File) {
                Some(v) => v,
                None => continue,
            };

            let index = match patt.get_integer(fc::Object::Index) {
                Some(v) => v,
                None => continue,
            };

            handles.push(Handle::Path {
                path: path.into(),
                font_index: index as u32,
            });
        }

        if !handles.is_empty() {
            Ok(handles)
        } else {
            Err(SelectionError::NotFound)
        }
    }

    /// Returns the names of all families installed on the system.
    pub fn all_families(&self) -> Result<Vec<String>, SelectionError> {
        let pattern = fc::Pattern::new();

        // We want the family name.
        let mut object_set = fc::ObjectSet::new();
        object_set.push_string(fc::Object::Family);

        let patterns = pattern
            .list(&self.config, object_set)
            .map_err(|_| SelectionError::NotFound)?;

        let mut result_families = vec![];
        for patt in patterns {
            if let Some(family) = patt.get_string(fc::Object::Family) {
                result_families.push(family);
            }
        }

        result_families.sort();
        result_families.dedup();

        if !result_families.is_empty() {
            Ok(result_families)
        } else {
            Err(SelectionError::NotFound)
        }
    }

    /// Looks up a font family by name and returns the handles of all the fonts in that family.
    pub fn select_family_by_name(&self, family_name: &str) -> Result<FamilyHandle, SelectionError> {
        use std::borrow::Cow;

        let family_name = match family_name {
            "serif" | "sans-serif" | "monospace" | "cursive" | "fantasy" => {
                Cow::from(self.select_generic_font(family_name)?)
            }
            _ => Cow::from(family_name),
        };

        let pattern = fc::Pattern::from_name(family_name.as_ref());

        let mut object_set = fc::ObjectSet::new();
        object_set.push_string(fc::Object::File);
        object_set.push_string(fc::Object::Index);

        let patterns = pattern
            .list(&self.config, object_set)
            .map_err(|_| SelectionError::NotFound)?;

        let mut handles = vec![];
        for patt in patterns {
            let font_path = patt.get_string(fc::Object::File).unwrap();
            let font_index = patt.get_integer(fc::Object::Index).unwrap() as u32;
            let handle = Handle::from_path(std::path::PathBuf::from(font_path), font_index);
            handles.push(handle);
        }

        if !handles.is_empty() {
            Ok(FamilyHandle::from_font_handles(handles.into_iter()))
        } else {
            Err(SelectionError::NotFound)
        }
    }

    /// Selects a font by a generic name.
    ///
    /// Accepts: serif, sans-serif, monospace, cursive and fantasy.
    pub(super) fn select_generic_font(&self, name: &str) -> Result<String, SelectionError> {
        let mut pattern = fc::Pattern::from_name(name);
        pattern.config_substitute(fc::MatchKind::Pattern);
        pattern.default_substitute();

        let patterns = pattern
            .sorted(&self.config)
            .map_err(|_| SelectionError::NotFound)?;

        if let Some(patt) = patterns.into_iter().next() {
            if let Some(family) = patt.get_string(fc::Object::Family) {
                return Ok(family);
            }
        }

        Err(SelectionError::NotFound)
    }

    /// Selects a font by PostScript name, which should be a unique identifier.
    ///
    /// The default implementation, which is used by the DirectWrite and the filesystem backends,
    /// does a brute-force search of installed fonts to find the one that matches.
    pub fn select_by_postscript_name(
        &self,
        postscript_name: &str,
    ) -> Result<Handle, SelectionError> {
        let mut pattern = fc::Pattern::new();
        pattern.push_string(fc::Object::PostScriptName, postscript_name.to_owned());

        // We want the file path and the font index.
        let mut object_set = fc::ObjectSet::new();
        object_set.push_string(fc::Object::File);
        object_set.push_string(fc::Object::Index);

        let patterns = pattern
            .list(&self.config, object_set)
            .map_err(|_| SelectionError::NotFound)?;

        if let Some(patt) = patterns.into_iter().next() {
            let font_path = patt.get_string(fc::Object::File).unwrap();
            let font_index = patt.get_integer(fc::Object::Index).unwrap() as u32;
            let handle = Handle::from_path(std::path::PathBuf::from(font_path), font_index);
            Ok(handle)
        } else {
            Err(SelectionError::NotFound)
        }
    }

    /// Performs font matching according to the CSS Fonts Level 3 specification and returns the
    /// handle.
    #[inline]
    pub fn select_best_match(
        &self,
        family_names: &[FamilyName],
        properties: &Properties,
    ) -> Result<Handle, SelectionError> {
        <Self as Source>::select_best_match(self, family_names, properties)
    }
}

impl Source for FontconfigSource {
    #[inline]
    fn all_fonts(&self) -> Result<Vec<Handle>, SelectionError> {
        self.all_fonts()
    }

    #[inline]
    fn all_families(&self) -> Result<Vec<String>, SelectionError> {
        self.all_families()
    }

    #[inline]
    fn select_family_by_name(&self, family_name: &str) -> Result<FamilyHandle, SelectionError> {
        self.select_family_by_name(family_name)
    }

    #[inline]
    fn select_by_postscript_name(&self, postscript_name: &str) -> Result<Handle, SelectionError> {
        self.select_by_postscript_name(postscript_name)
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

