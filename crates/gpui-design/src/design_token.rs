use serde::{Serialize, Serializer, ser::SerializeStruct};

/// A style token in a shape compatible with Style Dictionary export.
#[derive(Debug, Clone, PartialEq)]
pub struct DesignToken {
    pub name: String,
    pub path: Vec<&'static str>,
    pub value: String,
    pub token_type: &'static str,
}

impl DesignToken {
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Serialize for DesignToken {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("DesignToken", 4)?;
        state.serialize_field("name", &self.name())?;
        state.serialize_field("path", &self.path)?;
        state.serialize_field("value", &self.value)?;
        state.serialize_field("token_type", &self.token_type)?;
        state.end()
    }
}

pub(super) fn token(
    path: &'static str,
    value: impl ToString,
    token_type: &'static str,
) -> DesignToken {
    let path_vec: Vec<&'static str> = path.split('.').collect();
    let name = path.to_owned();
    DesignToken {
        name,
        path: path_vec,
        value: value.to_string(),
        token_type,
    }
}
