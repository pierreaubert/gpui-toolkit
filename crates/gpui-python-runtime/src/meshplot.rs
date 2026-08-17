//! Versioned Rust IR for declarative Python mesh plots.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const MESHPLOT_SPEC_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshPlotSpec {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default = "default_id")]
    pub id: String,
    #[serde(default)]
    pub revision: u64,
    pub geometry: Value,
    #[serde(default)]
    pub field: Option<Value>,
    #[serde(default = "default_view")]
    pub view: String,
    #[serde(default)]
    pub revolve: Option<Value>,
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_color_scale")]
    pub color_scale: String,
    #[serde(default = "default_color_range")]
    pub color_range: Value,
    #[serde(default = "default_missing_value_policy")]
    pub missing_value_policy: String,
    #[serde(default = "default_wireframe")]
    pub wireframe: bool,
    #[serde(default)]
    pub title: Option<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    #[serde(default)]
    pub selection: Option<Value>,
    #[serde(default)]
    pub camera: Option<Value>,
    #[serde(default)]
    pub viewport: Option<Value>,
    #[serde(default)]
    pub contour_levels: Option<Value>,
    #[serde(default = "default_equal_aspect")]
    pub equal_aspect: bool,
    #[serde(default)]
    pub axes: Option<Value>,
    #[serde(default)]
    pub interactions: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshResourceRef {
    pub role: String,
    pub resource_id: String,
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MeshPlotResourceError {
    #[error(
        "mesh plot {plot_id:?} references unavailable {role} resource {resource_id:?} generation {generation} (patch {patch_id:?})"
    )]
    Unavailable {
        plot_id: String,
        role: String,
        resource_id: String,
        generation: u64,
        patch_id: Option<String>,
    },
    #[error(
        "mesh plot {plot_id:?} has invalid {role} resource reference: {message} (patch {patch_id:?})"
    )]
    InvalidReference {
        plot_id: String,
        role: String,
        message: String,
        patch_id: Option<String>,
    },
}

fn default_schema_version() -> u32 {
    MESHPLOT_SPEC_SCHEMA_VERSION
}
fn default_view() -> String {
    "planar".into()
}
fn default_id() -> String {
    "mesh_plot".into()
}
fn default_mode() -> String {
    "mesh".into()
}
fn default_color_scale() -> String {
    "viridis".into()
}
fn default_color_range() -> Value {
    Value::String("auto".into())
}
fn default_missing_value_policy() -> String {
    "reject".into()
}
fn default_wireframe() -> bool {
    true
}
fn default_equal_aspect() -> bool {
    true
}

impl MeshPlotSpec {
    /// Stable cache identity shared by retained host state and patch ordering.
    pub fn cache_id(&self) -> String {
        if !self.id.trim().is_empty() && self.id != "mesh_plot" {
            return self.id.clone();
        }
        self.geometry
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or(&self.id)
            .to_string()
    }

    pub fn from_value(value: Value) -> Result<Self, String> {
        let version = value
            .get("schema_version")
            .and_then(Value::as_u64)
            .unwrap_or(MESHPLOT_SPEC_SCHEMA_VERSION as u64);
        if version != MESHPLOT_SPEC_SCHEMA_VERSION as u64 {
            return Err(format!(
                "unsupported mesh_plot schema version {version}; supported version is {MESHPLOT_SPEC_SCHEMA_VERSION}"
            ));
        }
        let spec: Self = serde_json::from_value(value).map_err(|error| error.to_string())?;
        spec.validate()?;
        Ok(spec)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != MESHPLOT_SPEC_SCHEMA_VERSION {
            return Err(format!(
                "unsupported mesh_plot schema version {}",
                self.schema_version
            ));
        }
        if self.id.trim().is_empty() {
            return Err("mesh_plot id must not be empty".into());
        }
        if !self.geometry.is_object() {
            return Err("mesh_plot geometry must be an object".into());
        }
        let geometry_resource = self.geometry.get("resource_id");
        let split_geometry_resources = self.geometry.get("positions").is_some_and(Value::is_object)
            || self.geometry.get("triangles").is_some_and(Value::is_object);
        if geometry_resource.is_some() {
            validate_resource_handle(&self.geometry, "geometry")?;
        } else if split_geometry_resources {
            validate_resource_handle(
                self.geometry
                    .get("positions")
                    .ok_or("mesh_plot geometry requires a positions resource handle")?,
                "geometry.positions",
            )?;
            validate_resource_handle(
                self.geometry
                    .get("triangles")
                    .ok_or("mesh_plot geometry requires a triangles resource handle")?,
                "geometry.triangles",
            )?;
        } else {
            let positions = self
                .geometry
                .get("positions")
                .and_then(Value::as_array)
                .ok_or("mesh_plot geometry requires positions and triangles or resource_id")?;
            if positions.is_empty() {
                return Err("mesh_plot geometry positions must not be empty".into());
            }
            for (index, position) in positions.iter().enumerate() {
                let values = position.as_array().ok_or_else(|| {
                    format!("mesh_plot geometry position {index} must be an array")
                })?;
                if values.len() != 3
                    || values
                        .iter()
                        .any(|value| value.as_f64().is_none_or(|value| !value.is_finite()))
                {
                    return Err(format!(
                        "mesh_plot geometry position {index} must contain three finite numbers"
                    ));
                }
            }
            let triangles = self
                .geometry
                .get("triangles")
                .and_then(Value::as_array)
                .ok_or("mesh_plot geometry requires positions and triangles or resource_id")?;
            for (index, triangle) in triangles.iter().enumerate() {
                let values = triangle.as_array().ok_or_else(|| {
                    format!("mesh_plot geometry triangle {index} must be an array")
                })?;
                if values.len() != 3 {
                    return Err(format!(
                        "mesh_plot geometry triangle {index} must contain three indices"
                    ));
                }
                for value in values {
                    let index_value = value.as_u64().ok_or_else(|| {
                        format!("mesh_plot geometry triangle {index} has a non-integer index")
                    })?;
                    if usize::try_from(index_value)
                        .ok()
                        .is_none_or(|index| index >= positions.len())
                    {
                        return Err(format!(
                            "mesh_plot geometry triangle {index} references an invalid vertex"
                        ));
                    }
                }
            }
            validate_id_length(
                self.geometry.get("vertex_ids"),
                Some(positions.len()),
                "vertex_ids",
            )?;
            validate_id_length(
                self.geometry.get("cell_ids"),
                Some(triangles.len()),
                "cell_ids",
            )?;
        }
        if geometry_resource.is_some() || split_geometry_resources {
            validate_id_length(self.geometry.get("vertex_ids"), None, "vertex_ids")?;
            validate_id_length(self.geometry.get("cell_ids"), None, "cell_ids")?;
        }
        if self.view.is_empty() || self.mode.is_empty() {
            return Err("mesh_plot view and mode are required".into());
        }
        if !matches!(
            self.view.as_str(),
            "planar" | "axisymmetric_section" | "axisymmetric_revolve" | "surface3d"
        ) {
            return Err(format!("unsupported mesh_plot view {:?}", self.view));
        }
        if !matches!(
            self.mode.as_str(),
            "mesh" | "scalar_fill" | "filled_contours" | "isolines" | "fill_and_isolines"
        ) {
            return Err(format!("unsupported mesh_plot mode {:?}", self.mode));
        }
        if !matches!(
            self.color_scale.as_str(),
            "viridis"
                | "plasma"
                | "inferno"
                | "magma"
                | "cividis"
                | "turbo"
                | "coolwarm"
                | "cool_warm"
        ) {
            return Err(format!(
                "unsupported mesh_plot color scale {:?}",
                self.color_scale
            ));
        }
        if !matches!(self.mode.as_str(), "mesh") && self.field.is_none() {
            return Err(format!(
                "mesh_plot mode {:?} requires a scalar field",
                self.mode
            ));
        }
        if self.revolve.is_some() && self.view != "axisymmetric_revolve" {
            return Err("mesh_plot revolve settings require view='axisymmetric_revolve'".into());
        }
        if let Some(revolve) = &self.revolve {
            validate_revolve(revolve)?;
        }
        if let Some(field) = &self.field {
            if !field.is_object() {
                return Err("mesh_plot field must be an object".into());
            }
            let association = field
                .get("association")
                .and_then(Value::as_str)
                .unwrap_or("vertex");
            if !matches!(association, "vertex" | "cell") {
                return Err(format!(
                    "unsupported mesh_plot field association {association:?}"
                ));
            }
            if matches!(
                self.mode.as_str(),
                "filled_contours" | "isolines" | "fill_and_isolines"
            ) && association == "cell"
            {
                return Err("mesh_plot contours require a vertex field".into());
            }
            if field.get("resource_id").is_some() {
                validate_resource_handle(field, "field")?;
            } else {
                let values = field
                    .get("values")
                    .and_then(Value::as_array)
                    .ok_or("mesh_plot field requires values or resource_id")?;
                if values
                    .iter()
                    .any(|value| value.as_f64().is_none_or(|value| !value.is_finite()))
                {
                    return Err("mesh_plot field values must be finite numbers".into());
                }
                if let Some(geometry_positions) =
                    self.geometry.get("positions").and_then(Value::as_array)
                {
                    let expected = if association == "vertex" {
                        geometry_positions.len()
                    } else {
                        self.geometry
                            .get("triangles")
                            .and_then(Value::as_array)
                            .map_or(0, Vec::len)
                    };
                    if values.len() != expected {
                        return Err(format!(
                            "mesh_plot {association} field has {} values; expected {expected}",
                            values.len()
                        ));
                    }
                }
            }
            if let Some(valid) = field.get("valid") {
                if valid.is_object() {
                    validate_resource_handle(valid, "field.valid")?;
                } else {
                    let valid = valid
                        .as_array()
                        .ok_or("mesh_plot field valid mask must be an array or resource handle")?;
                    if field
                        .get("values")
                        .and_then(Value::as_array)
                        .is_some_and(|values| valid.len() != values.len())
                    {
                        return Err(
                            "mesh_plot field valid mask length does not match values".into()
                        );
                    }
                    if valid.iter().any(|value| !value.is_boolean()) {
                        return Err("mesh_plot field valid mask must contain booleans".into());
                    }
                }
            }
        }
        validate_range(&self.color_range)?;
        if !matches!(self.missing_value_policy.as_str(), "reject" | "mask_nan") {
            return Err(format!(
                "unsupported mesh_plot missing_value_policy {:?}",
                self.missing_value_policy
            ));
        }
        for (name, value) in [("width", self.width), ("height", self.height)] {
            if value.is_some_and(|v| !v.is_finite() || v <= 0.0) {
                return Err(format!("mesh_plot {name} must be positive and finite"));
            }
        }
        for (name, value) in [
            ("selection", self.selection.as_ref()),
            ("camera", self.camera.as_ref()),
            ("viewport", self.viewport.as_ref()),
        ] {
            if let Some(value) = value
                && !value.is_object()
            {
                return Err(format!("mesh_plot {name} must be an object when present"));
            }
        }
        if let Some(levels) = &self.contour_levels {
            validate_contour_levels(levels)?;
        }
        if let Some(axes) = &self.axes {
            validate_axes(axes)?;
        }
        if let Some(interactions) = &self.interactions {
            let mut seen_interactions = std::collections::HashSet::new();
            for interaction in interactions {
                if !matches!(
                    interaction.as_str(),
                    "pan" | "zoom" | "inspect" | "select" | "reset" | "fit"
                ) {
                    return Err(format!("unsupported mesh_plot interaction {interaction:?}"));
                }
                if !seen_interactions.insert(interaction) {
                    return Err(format!("duplicate mesh_plot interaction {interaction:?}"));
                }
            }
        }
        Ok(())
    }

    /// Return every resource referenced by this plot, including split
    /// geometry, external IDs, scalar values, and validity masks.
    pub fn resource_refs(&self) -> Result<Vec<MeshResourceRef>, String> {
        let mut refs = Vec::new();
        let mut push = |role: &str, value: &Value| -> Result<(), String> {
            let object = value
                .as_object()
                .ok_or_else(|| format!("mesh_plot {role} resource handle must be an object"))?;
            let resource_id = object
                .get("resource_id")
                .or_else(|| object.get("id"))
                .and_then(Value::as_str)
                .filter(|id| !id.trim().is_empty())
                .ok_or_else(|| format!("mesh_plot {role} resource_id must be non-empty"))?;
            let generation = object
                .get("generation")
                .and_then(Value::as_u64)
                .filter(|generation| *generation > 0)
                .ok_or_else(|| format!("mesh_plot {role} resource generation must be positive"))?;
            refs.push(MeshResourceRef {
                role: role.into(),
                resource_id: resource_id.into(),
                generation,
            });
            Ok(())
        };

        let geometry = &self.geometry;
        if geometry.get("resource_id").is_some() {
            push("geometry", geometry)?;
        } else if geometry.get("positions").is_some_and(Value::is_object)
            || geometry.get("triangles").is_some_and(Value::is_object)
        {
            push(
                "geometry.positions",
                geometry
                    .get("positions")
                    .ok_or("mesh_plot geometry is missing positions resource")?,
            )?;
            push(
                "geometry.triangles",
                geometry
                    .get("triangles")
                    .ok_or("mesh_plot geometry is missing triangles resource")?,
            )?;
        }
        for (name, value) in [
            ("vertex_ids", geometry.get("vertex_ids")),
            ("cell_ids", geometry.get("cell_ids")),
        ] {
            if let Some(value) = value.filter(|value| value.is_object()) {
                push(&format!("geometry.{name}"), value)?;
            }
        }
        if let Some(field) = &self.field {
            if field.get("resource_id").is_some() {
                push("field", field)?;
            }
            if let Some(valid) = field.get("valid").filter(|value| value.is_object()) {
                push("field.valid", valid)?;
            }
        }
        Ok(refs)
    }
}

fn validate_resource_handle(value: &Value, name: &str) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("mesh_plot {name} resource handle must be an object"))?;
    let resource_id = object
        .get("resource_id")
        .or_else(|| object.get("id"))
        .and_then(Value::as_str)
        .ok_or_else(|| format!("mesh_plot {name} resource handle requires resource_id"))?;
    if resource_id.trim().is_empty() {
        return Err(format!("mesh_plot {name} resource_id must not be empty"));
    }
    object
        .get("generation")
        .and_then(Value::as_u64)
        .filter(|generation| *generation > 0)
        .ok_or_else(|| format!("mesh_plot {name} resource generation must be positive"))?;
    Ok(())
}

fn validate_id_length(
    value: Option<&Value>,
    expected: Option<usize>,
    name: &str,
) -> Result<(), String> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.is_object() {
        return validate_resource_handle(value, &format!("geometry.{name}"));
    }
    let values = value
        .as_array()
        .ok_or_else(|| format!("mesh_plot geometry {name} must be an array"))?;
    if expected.is_some_and(|expected| values.len() != expected)
        || values.iter().any(|value| value.as_u64().is_none())
    {
        let expected = expected.map_or("the expected".into(), |expected| expected.to_string());
        return Err(format!(
            "mesh_plot geometry {name} must contain {expected} integer ids"
        ));
    }
    Ok(())
}

fn validate_range(value: &Value) -> Result<(), String> {
    match value {
        Value::String(value) if value == "auto" => Ok(()),
        Value::Array(values) if values.len() == 2 => {
            let min = values[0].as_f64();
            let max = values[1].as_f64();
            if min.is_some_and(f64::is_finite) && max.is_some_and(f64::is_finite) && min < max {
                Ok(())
            } else {
                Err("mesh_plot color_range must be increasing finite values".into())
            }
        }
        Value::Object(value) => {
            let symmetric = value
                .get("symmetric")
                .and_then(Value::as_object)
                .filter(|_| value.len() == 1)
                .ok_or("mesh_plot symmetric color_range must contain a symmetric object")?;
            if symmetric.len() != 2 {
                return Err("mesh_plot symmetric color_range requires center and extent".into());
            }
            let center = symmetric.get("center").and_then(Value::as_f64);
            let extent = symmetric.get("extent");
            if center.is_none_or(|center| !center.is_finite()) {
                return Err("mesh_plot symmetric color_range center must be finite".into());
            }
            match extent {
                Some(Value::String(value)) if value == "auto" => Ok(()),
                Some(value)
                    if value
                        .as_f64()
                        .is_some_and(|value| value.is_finite() && value > 0.0) =>
                {
                    Ok(())
                }
                _ => Err(
                    "mesh_plot symmetric color_range extent must be 'auto' or positive finite"
                        .into(),
                ),
            }
        }
        _ => Err("mesh_plot color_range must be 'auto', [min, max], or a symmetric range".into()),
    }
}

fn validate_contour_levels(value: &Value) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or("mesh_plot contour_levels must be an object")?;
    if let Some(count) = object.get("count") {
        if count.as_u64().is_none_or(|count| count == 0) {
            return Err("mesh_plot contour_levels.count must be positive".into());
        }
    } else if let Some(values) = object.get("values").and_then(Value::as_array) {
        if values.len() < 2
            || values
                .iter()
                .any(|value| value.as_f64().is_none_or(|value| !value.is_finite()))
            || values
                .windows(2)
                .any(|pair| pair[0].as_f64() >= pair[1].as_f64())
        {
            return Err(
                "mesh_plot contour_levels.values must contain increasing finite values".into(),
            );
        }
    } else {
        return Err("mesh_plot contour_levels requires count or values".into());
    }
    Ok(())
}

fn validate_axes(value: &Value) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or("mesh_plot axes must be an object")?;
    const ALLOWED: [&str; 6] = [
        "horizontal_label",
        "vertical_label",
        "unit",
        "x_range",
        "y_range",
        "show_grid",
    ];
    if let Some(name) = object
        .keys()
        .find(|name| !ALLOWED.iter().any(|allowed| allowed == name))
    {
        return Err(format!("unsupported mesh_plot axes property {name:?}"));
    }
    for name in ["horizontal_label", "vertical_label", "unit"] {
        if let Some(value) = object.get(name)
            && !value.is_string()
        {
            return Err(format!("mesh_plot axes {name} must be a string"));
        }
    }
    for name in ["x_range", "y_range"] {
        if let Some(value) = object.get(name) {
            let values = value
                .as_array()
                .ok_or_else(|| format!("mesh_plot axes {name} must be an array"))?;
            if values.len() != 2
                || values
                    .iter()
                    .any(|value| value.as_f64().is_none_or(|value| !value.is_finite()))
                || values[0].as_f64() >= values[1].as_f64()
            {
                return Err(format!(
                    "mesh_plot axes {name} must contain two increasing finite values"
                ));
            }
        }
    }
    if let Some(value) = object.get("show_grid")
        && !value.is_boolean()
    {
        return Err("mesh_plot axes show_grid must be boolean".into());
    }
    Ok(())
}

fn validate_revolve(value: &Value) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or("mesh_plot revolve must be an object")?;
    let axis = |name: &str, default: &str| -> Result<String, String> {
        object.get(name).map_or_else(
            || Ok(default.to_owned()),
            |value| {
                value
                    .as_str()
                    .filter(|axis| matches!(*axis, "x" | "y" | "z"))
                    .map(str::to_owned)
                    .ok_or_else(|| format!("mesh_plot revolve {name} must be 'x', 'y', or 'z'"))
            },
        )
    };
    let radial = axis("radial", "x")?;
    let axial = axis("axial", "z")?;
    if radial == axial {
        return Err("mesh_plot revolve radial and axial axes must be distinct".into());
    }
    let finite_number = |name: &str, default: f64| {
        object.get(name).map_or(Ok(default), |value| {
            value
                .as_f64()
                .filter(|value| value.is_finite())
                .ok_or_else(|| format!("mesh_plot revolve {name} must be finite"))
        })
    };
    finite_number("start_angle", 0.0)?;
    let sweep_angle = finite_number("sweep_angle", std::f64::consts::TAU)?;
    if !(sweep_angle > 0.0 && sweep_angle <= std::f64::consts::TAU) {
        return Err("mesh_plot revolve sweep_angle must be in (0, 2*pi]".into());
    }
    let segments = object.get("segments").map_or(Ok(64_u64), |value| {
        value
            .as_u64()
            .ok_or("mesh_plot revolve segments must be an integer of at least 3")
    })?;
    if segments < 3 || segments > u32::MAX as u64 {
        return Err("mesh_plot revolve segments must be an integer of at least 3".into());
    }
    if let Some(value) = object.get("end_caps")
        && value.as_bool().is_none()
    {
        return Err("mesh_plot revolve end_caps must be boolean".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_spec() -> Value {
        serde_json::json!({
            "schema_version": 1,
            "id": "plot",
            "geometry": {
                "id": "mesh",
                "positions": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                "triangles": [[0, 1, 2]]
            },
            "field": {"values": [0.0, 0.5, 1.0], "association": "vertex"},
            "mode": "scalar_fill"
        })
    }

    #[test]
    fn validates_inline_mesh_topology_and_field_lengths() {
        let spec = MeshPlotSpec::from_value(valid_spec()).unwrap();
        assert_eq!(spec.id, "plot");

        let mut invalid = valid_spec();
        invalid["geometry"]["triangles"] = serde_json::json!([[0, 1, 3]]);
        assert!(MeshPlotSpec::from_value(invalid).is_err());

        let mut invalid = valid_spec();
        invalid["field"]["values"] = serde_json::json!([1.0]);
        assert!(MeshPlotSpec::from_value(invalid).is_err());
    }

    #[test]
    fn accepts_resource_handles_and_rejects_bad_ranges() {
        let mut value = valid_spec();
        value["geometry"] = serde_json::json!({
            "id": "mesh", "resource_id": "geometry", "generation": 4
        });
        value["field"] = serde_json::json!({
            "resource_id": "field", "generation": 4, "association": "vertex"
        });
        assert!(MeshPlotSpec::from_value(value).is_ok());

        let mut invalid = valid_spec();
        invalid["color_range"] = serde_json::json!([1.0, 1.0]);
        assert!(MeshPlotSpec::from_value(invalid).is_err());
    }

    #[test]
    fn accepts_symmetric_color_ranges() {
        let mut automatic = valid_spec();
        automatic["color_range"] = serde_json::json!({
            "symmetric": {"center": 0.0, "extent": "auto"}
        });
        assert!(MeshPlotSpec::from_value(automatic).is_ok());

        let mut fixed = valid_spec();
        fixed["color_range"] = serde_json::json!({
            "symmetric": {"center": 1.0, "extent": 3.5}
        });
        assert!(MeshPlotSpec::from_value(fixed).is_ok());

        let mut invalid = valid_spec();
        invalid["color_range"] = serde_json::json!({
            "symmetric": {"center": 0.0, "extent": 0.0}
        });
        assert!(MeshPlotSpec::from_value(invalid).is_err());
    }

    #[test]
    fn accepts_and_validates_missing_value_policy() {
        let mut masked = valid_spec();
        masked["missing_value_policy"] = serde_json::json!("mask_nan");
        assert_eq!(
            MeshPlotSpec::from_value(masked)
                .unwrap()
                .missing_value_policy,
            "mask_nan"
        );

        let mut invalid = valid_spec();
        invalid["missing_value_policy"] = serde_json::json!("interpolate");
        assert!(MeshPlotSpec::from_value(invalid).is_err());
    }

    #[test]
    fn validates_revolve_settings_and_view_ownership() {
        let mut valid = valid_spec();
        valid["view"] = serde_json::json!("axisymmetric_revolve");
        valid["revolve"] = serde_json::json!({
            "radial": "y",
            "axial": "z",
            "start_angle": 0.25,
            "sweep_angle": 1.5,
            "segments": 32,
            "end_caps": true
        });
        assert!(MeshPlotSpec::from_value(valid.clone()).is_ok());

        let mut same_axes = valid.clone();
        same_axes["revolve"]["axial"] = serde_json::json!("y");
        assert!(MeshPlotSpec::from_value(same_axes).is_err());

        let mut invalid_sweep = valid.clone();
        invalid_sweep["revolve"]["sweep_angle"] = serde_json::json!(0.0);
        assert!(MeshPlotSpec::from_value(invalid_sweep).is_err());

        let mut wrong_view = valid;
        wrong_view["view"] = serde_json::json!("planar");
        assert!(MeshPlotSpec::from_value(wrong_view).is_err());
    }

    #[test]
    fn validates_split_geometry_ids_and_field_masks_as_handles() {
        let mut value = valid_spec();
        value["geometry"] = serde_json::json!({
            "id": "mesh",
            "positions": {"resource_id": "positions", "generation": 1},
            "triangles": {"resource_id": "triangles", "generation": 1},
            "vertex_ids": {"resource_id": "vertex_ids", "generation": 1},
            "cell_ids": {"resource_id": "cell_ids", "generation": 1}
        });
        value["field"] = serde_json::json!({
            "values": [0.0, 0.5, 1.0],
            "association": "vertex",
            "valid": {"resource_id": "mask", "generation": 1}
        });
        assert!(MeshPlotSpec::from_value(value).is_ok());

        let mut invalid = valid_spec();
        invalid["field"]["valid"] = serde_json::json!({"resource_id": "mask"});
        assert!(MeshPlotSpec::from_value(invalid).is_err());
    }

    #[test]
    fn resource_refs_include_split_geometry_ids_field_and_mask() {
        let mut value = valid_spec();
        value["geometry"] = serde_json::json!({
            "positions": {"resource_id": "positions", "generation": 2},
            "triangles": {"resource_id": "triangles", "generation": 3},
            "vertex_ids": {"resource_id": "vertex_ids", "generation": 4},
            "cell_ids": {"resource_id": "cell_ids", "generation": 5}
        });
        value["field"] = serde_json::json!({
            "resource_id": "values", "generation": 6, "association": "vertex",
            "valid": {"resource_id": "mask", "generation": 7}
        });
        let spec = MeshPlotSpec::from_value(value).unwrap();
        let refs = spec.resource_refs().unwrap();
        assert_eq!(refs.len(), 6);
        assert_eq!(refs[0].role, "geometry.positions");
        assert_eq!(refs[5].resource_id, "mask");
    }

    #[test]
    fn validates_contour_configuration_and_future_schema() {
        let mut value = valid_spec();
        value["contour_levels"] = serde_json::json!({"count": 12});
        value["equal_aspect"] = serde_json::json!(true);
        value["interactions"] = serde_json::json!(["pan", "zoom", "select"]);
        assert!(MeshPlotSpec::from_value(value).is_ok());

        let mut invalid = valid_spec();
        invalid["contour_levels"] = serde_json::json!({"values": [1.0, 1.0]});
        assert!(MeshPlotSpec::from_value(invalid).is_err());

        let mut future = valid_spec();
        future["schema_version"] = serde_json::json!(2);
        assert!(
            MeshPlotSpec::from_value(future)
                .unwrap_err()
                .contains("unsupported mesh_plot schema version")
        );
    }

    #[test]
    fn validates_optional_axes_configuration() {
        let mut value = valid_spec();
        value["axes"] = serde_json::json!({
            "horizontal_label": "distance",
            "vertical_label": "height",
            "unit": "m",
            "x_range": [0.0, 2.0],
            "y_range": [-1.0, 3.0],
            "show_grid": false
        });
        let spec = MeshPlotSpec::from_value(value).unwrap();
        assert_eq!(
            spec.axes.as_ref().and_then(|axes| axes["unit"].as_str()),
            Some("m")
        );

        for (property, invalid) in [
            ("horizontal_label", serde_json::json!(12)),
            ("x_range", serde_json::json!([1.0, 1.0])),
            ("y_range", serde_json::json!([0.0, f64::NAN])),
            ("show_grid", serde_json::json!("false")),
            ("unknown", serde_json::json!(true)),
        ] {
            let mut invalid_spec = valid_spec();
            invalid_spec["axes"] = serde_json::json!({property: invalid});
            assert!(
                MeshPlotSpec::from_value(invalid_spec).is_err(),
                "{property}"
            );
        }
    }

    #[test]
    fn rejects_invalid_meshplot_validation_surface() {
        let mut invalid = valid_spec();
        invalid["geometry"] = serde_json::json!({
            "positions": {"resource_id": "positions", "generation": 0},
            "triangles": {"resource_id": "triangles", "generation": 1}
        });
        assert!(
            MeshPlotSpec::from_value(invalid)
                .unwrap_err()
                .contains("geometry.positions resource generation must be positive")
        );

        let mut invalid = valid_spec();
        invalid["geometry"] = serde_json::json!({
            "positions": {"resource_id": "positions", "generation": 1}
        });
        assert!(
            MeshPlotSpec::from_value(invalid)
                .unwrap_err()
                .contains("requires a triangles resource handle")
        );

        for (property, value, expected) in [
            (
                "view",
                serde_json::json!("volume"),
                "unsupported mesh_plot view",
            ),
            (
                "mode",
                serde_json::json!("volume_fill"),
                "unsupported mesh_plot mode",
            ),
            (
                "color_scale",
                serde_json::json!("rainbow"),
                "unsupported mesh_plot color scale",
            ),
        ] {
            let mut invalid = valid_spec();
            invalid[property] = value;
            assert!(
                MeshPlotSpec::from_value(invalid)
                    .unwrap_err()
                    .contains(expected),
                "{property}"
            );
        }

        let mut invalid = valid_spec();
        invalid["mode"] = serde_json::json!("scalar_fill");
        invalid["field"] = serde_json::Value::Null;
        assert!(
            MeshPlotSpec::from_value(invalid)
                .unwrap_err()
                .contains("requires a scalar field")
        );

        let mut invalid = valid_spec();
        invalid["field"]["association"] = serde_json::json!("edge");
        assert!(
            MeshPlotSpec::from_value(invalid)
                .unwrap_err()
                .contains("unsupported mesh_plot field association")
        );

        let mut invalid = valid_spec();
        invalid["mode"] = serde_json::json!("isolines");
        invalid["field"]["association"] = serde_json::json!("cell");
        invalid["field"]["values"] = serde_json::json!([1.0]);
        assert!(
            MeshPlotSpec::from_value(invalid)
                .unwrap_err()
                .contains("contours require a vertex field")
        );

        for (property, value, expected) in [
            (
                "values",
                serde_json::json!([0.0, 0.5, f64::NAN]),
                "field values must be finite",
            ),
            (
                "valid",
                serde_json::json!([true, false]),
                "valid mask length does not match values",
            ),
            (
                "valid",
                serde_json::json!([true, "false", true]),
                "valid mask must contain booleans",
            ),
        ] {
            let mut invalid = valid_spec();
            invalid["field"][property] = value;
            assert!(
                MeshPlotSpec::from_value(invalid)
                    .unwrap_err()
                    .contains(expected),
                "field.{property}"
            );
        }

        let mut invalid = valid_spec();
        invalid["width"] = serde_json::json!(0.0);
        assert!(
            MeshPlotSpec::from_value(invalid)
                .unwrap_err()
                .contains("width must be positive and finite")
        );

        let mut invalid = valid_spec();
        invalid["camera"] = serde_json::json!([0.0, 1.0]);
        assert!(
            MeshPlotSpec::from_value(invalid)
                .unwrap_err()
                .contains("camera must be an object")
        );
    }

    #[test]
    fn rejects_invalid_meshplot_geometry_and_interactions() {
        for (geometry, expected) in [
            (
                serde_json::json!({
                    "positions": [[0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                    "triangles": [[0, 1, 2]]
                }),
                "position 0 must contain three finite numbers",
            ),
            (
                serde_json::json!({
                    "positions": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                    "triangles": [[0, 1]]
                }),
                "triangle 0 must contain three indices",
            ),
            (
                serde_json::json!({
                    "positions": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                    "triangles": [[0, 1, "two"]]
                }),
                "triangle 0 has a non-integer index",
            ),
            (
                serde_json::json!({
                    "positions": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                    "triangles": [[0, 1, 2]],
                    "vertex_ids": [10]
                }),
                "vertex_ids must contain 3 integer ids",
            ),
        ] {
            let mut invalid = valid_spec();
            invalid["geometry"] = geometry;
            assert!(
                MeshPlotSpec::from_value(invalid)
                    .unwrap_err()
                    .contains(expected),
                "{expected}"
            );
        }

        for interactions in [
            serde_json::json!(["pan", "rotate"]),
            serde_json::json!(["pan", "pan"]),
        ] {
            let mut invalid = valid_spec();
            invalid["interactions"] = interactions;
            assert!(
                MeshPlotSpec::from_value(invalid)
                    .unwrap_err()
                    .contains("mesh_plot interaction")
            );
        }
    }
}
