//! Translation from the versioned Python MeshPlot IR to native `gpui-px`
//! builder options.
//!
//! Keep this conversion in the runtime library rather than the showcase
//! binary: every native host must reject unsupported configuration before it
//! mutates a retained plot or its last valid frame.

use crate::meshplot::MeshPlotSpec;
use d3rs::mesh::{ContourLevels, MissingValuePolicy};
use gpui_px::{
    AutoOrFixed, Axes2d, ColorRange, ColorScale, FieldInterpolation, MeshPlotPick, MeshRenderMode,
    PlotInteractions,
};
use serde_json::Value;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct NativeMeshPlotOptions {
    pub mode: MeshRenderMode,
    pub color_scale: ColorScale,
    pub color_range: ColorRange,
    pub missing_value_policy: MissingValuePolicy,
    pub axes: Axes2d,
    pub interactions: PlotInteractions,
    pub viewport: Option<[f64; 4]>,
    pub selection: Option<MeshPlotPick>,
}

fn finite_json_pair(value: &Value, name: &str) -> Result<[f64; 2], String> {
    let values = value
        .as_array()
        .ok_or_else(|| format!("mesh_plot {name} must be an array of two numbers"))?;
    if values.len() != 2 {
        return Err(format!("mesh_plot {name} must contain exactly two values"));
    }
    let min = values[0]
        .as_f64()
        .ok_or_else(|| format!("mesh_plot {name} values must be numbers"))?;
    let max = values[1]
        .as_f64()
        .ok_or_else(|| format!("mesh_plot {name} values must be numbers"))?;
    if !min.is_finite() || !max.is_finite() || min >= max {
        return Err(format!(
            "mesh_plot {name} values must be finite and increasing"
        ));
    }
    Ok([min, max])
}

fn color_range(value: &Value) -> Result<ColorRange, String> {
    match value {
        Value::String(value) if value == "auto" => Ok(ColorRange::Auto),
        Value::Array(_) => {
            let [min, max] = finite_json_pair(value, "color_range")?;
            Ok(ColorRange::Fixed { min, max })
        }
        Value::Object(value) => {
            let symmetric = value
                .get("symmetric")
                .and_then(Value::as_object)
                .filter(|_| value.len() == 1)
                .ok_or("mesh_plot symmetric color_range must contain a symmetric object")?;
            let center = symmetric
                .get("center")
                .and_then(Value::as_f64)
                .filter(|center| center.is_finite())
                .ok_or("mesh_plot symmetric color_range center must be finite")?;
            let extent = match symmetric.get("extent") {
                Some(Value::String(value)) if value == "auto" => AutoOrFixed::Auto,
                Some(value) => AutoOrFixed::Fixed(
                    value
                        .as_f64()
                        .filter(|extent| extent.is_finite() && *extent > 0.0)
                        .ok_or("mesh_plot symmetric color_range extent must be 'auto' or positive finite")?,
                ),
                None => return Err("mesh_plot symmetric color_range requires extent".into()),
            };
            Ok(ColorRange::Symmetric { center, extent })
        }
        _ => Err("mesh_plot color_range must be 'auto', [min, max], or a symmetric range".into()),
    }
}

fn contour_levels(value: Option<&Value>) -> Result<ContourLevels, String> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(ContourLevels::Count(8));
    };
    let object = value
        .as_object()
        .ok_or("mesh_plot contour_levels must be an object")?;
    if let Some(count) = object.get("count") {
        let count = count
            .as_u64()
            .and_then(|count| u32::try_from(count).ok())
            .filter(|count| *count > 0)
            .ok_or("mesh_plot contour_levels.count must be a positive integer")?;
        return Ok(ContourLevels::Count(count));
    }
    let values = object
        .get("values")
        .and_then(Value::as_array)
        .ok_or("mesh_plot contour_levels requires count or values")?
        .iter()
        .map(|value| {
            value
                .as_f64()
                .filter(|value| value.is_finite())
                .ok_or("mesh_plot contour level must be finite")
        })
        .collect::<Result<Vec<_>, _>>()?;
    if values.len() < 2 || values.windows(2).any(|pair| pair[1] <= pair[0]) {
        return Err("mesh_plot contour_levels.values must contain increasing finite values".into());
    }
    Ok(ContourLevels::Explicit(Arc::from(values)))
}

fn missing_value_policy(value: &str) -> Result<MissingValuePolicy, String> {
    match value {
        "reject" => Ok(MissingValuePolicy::Reject),
        "mask_nan" => Ok(MissingValuePolicy::MaskNaN),
        value => Err(format!(
            "unsupported mesh_plot missing_value_policy {value:?}"
        )),
    }
}

fn viewport(value: Option<&Value>) -> Result<Option<[f64; 4]>, String> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let object = value
        .as_object()
        .ok_or("mesh_plot viewport must be an object")?;
    let [x_min, x_max] = finite_json_pair(
        object
            .get("x")
            .ok_or("mesh_plot viewport requires an x range")?,
        "viewport.x",
    )?;
    let [y_min, y_max] = finite_json_pair(
        object
            .get("y")
            .ok_or("mesh_plot viewport requires a y range")?,
        "viewport.y",
    )?;
    Ok(Some([x_min, x_max, y_min, y_max]))
}

fn selection(
    value: Option<&Value>,
    plot_id: &str,
    mesh_id: &str,
) -> Result<Option<MeshPlotPick>, String> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let object = value
        .as_object()
        .ok_or("mesh_plot selection must be an object or null")?;
    let cell_index = object
        .get("cell_index")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or("mesh_plot selection requires a u32 cell_index")?;
    let world_position = match object.get("world_position") {
        None => [0.0; 3],
        Some(value) => {
            let values = value
                .as_array()
                .filter(|values| values.len() == 3)
                .ok_or("mesh_plot selection world_position must contain three values")?;
            let values = values
                .iter()
                .map(|value| {
                    value
                        .as_f64()
                        .filter(|value| value.is_finite())
                        .ok_or("mesh_plot selection world_position must be finite")
                })
                .collect::<Result<Vec<_>, _>>()?;
            [values[0], values[1], values[2]]
        }
    };
    let optional_u32 = |name: &str| -> Result<Option<u32>, String> {
        object
            .get(name)
            .filter(|value| !value.is_null())
            .map(|value| {
                value
                    .as_u64()
                    .and_then(|value| u32::try_from(value).ok())
                    .ok_or_else(|| format!("mesh_plot selection {name} must be a u32"))
            })
            .transpose()
    };
    let optional_u64 = |name: &str| -> Result<Option<u64>, String> {
        object
            .get(name)
            .filter(|value| !value.is_null())
            .map(|value| {
                value
                    .as_u64()
                    .ok_or_else(|| format!("mesh_plot selection {name} must be a u64"))
            })
            .transpose()
    };
    let displayed_value = object
        .get("displayed_value")
        .filter(|value| !value.is_null())
        .map(|value| {
            value
                .as_f64()
                .filter(|value| value.is_finite())
                .ok_or("mesh_plot selection displayed_value must be finite")
        })
        .transpose()?;
    let field_id = object
        .get("field_id")
        .filter(|value| !value.is_null())
        .map(|value| {
            value
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .map(Arc::from)
                .ok_or("mesh_plot selection field_id must be a non-empty string")
        })
        .transpose()?;
    Ok(Some(MeshPlotPick {
        plot_id: Arc::from(plot_id),
        mesh_id: Arc::from(mesh_id),
        cell_index,
        cell_id: optional_u64("cell_id")?,
        nearest_vertex_index: optional_u32("nearest_vertex_index")?,
        vertex_id: optional_u64("vertex_id")?,
        world_position,
        displayed_value,
        field_id,
    }))
}

/// Convert all native-renderer configuration before mutating retained state.
pub fn options(spec: &MeshPlotSpec, mesh_id: &str) -> Result<NativeMeshPlotOptions, String> {
    let levels = contour_levels(spec.contour_levels.as_ref())?;
    let mode = match spec.mode.as_str() {
        "scalar_fill" => MeshRenderMode::ScalarFill {
            interpolation: FieldInterpolation::Smooth,
        },
        "filled_contours" => MeshRenderMode::FilledContours { levels },
        "isolines" => MeshRenderMode::Isolines { levels },
        "fill_and_isolines" => MeshRenderMode::FillAndIsolines { levels },
        "mesh" => MeshRenderMode::Mesh,
        mode => return Err(format!("unsupported mesh_plot mode {mode:?}")),
    };
    let color_scale = match spec.color_scale.as_str() {
        "viridis" => ColorScale::Viridis,
        "plasma" => ColorScale::Plasma,
        "inferno" => ColorScale::Inferno,
        "magma" => ColorScale::Magma,
        "heat" => ColorScale::Heat,
        "coolwarm" | "cool_warm" => ColorScale::Coolwarm,
        "greys" => ColorScale::Greys,
        "cividis" | "turbo" => {
            return Err(format!(
                "mesh_plot color scale {:?} is not available in the native renderer",
                spec.color_scale
            ))
        }
        scale => return Err(format!("unsupported mesh_plot color scale {scale:?}")),
    };
    Ok(NativeMeshPlotOptions {
        mode,
        color_scale,
        color_range: color_range(&spec.color_range)?,
        missing_value_policy: missing_value_policy(&spec.missing_value_policy)?,
        axes: if spec.equal_aspect {
            Axes2d::equal_aspect()
        } else {
            Axes2d::default().fill_aspect()
        },
        interactions: PlotInteractions::InspectAndNavigate,
        viewport: viewport(spec.viewport.as_ref())?,
        selection: selection(spec.selection.as_ref(), &spec.id, mesh_id)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(value: Value) -> MeshPlotSpec {
        MeshPlotSpec::from_value(serde_json::json!({
            "schema_version": 1,
            "id": "native-options",
            "geometry": {
                "id": "mesh",
                "positions": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                "triangles": [[0, 1, 2]]
            },
            "field": {"id": "field", "values": [0.0, 1.0, 2.0], "association": "vertex"},
            "mode": "fill_and_isolines",
            "color_scale": "magma",
            "color_range": value,
            "missing_value_policy": "mask_nan",
            "equal_aspect": true,
            "viewport": {"x": [1.0, 3.0], "y": [-2.0, 2.0]},
            "contour_levels": {"values": [0.0, 1.0, 2.0]},
            "selection": {"cell_index": 3, "cell_id": 12, "vertex_id": 24}
        }))
        .unwrap()
    }

    #[test]
    fn options_preserve_native_rendering_configuration() {
        let options = options(
            &spec(serde_json::json!({"symmetric": {"center": 0.0, "extent": 2.0}})),
            "mesh",
        )
        .unwrap();
        assert!(matches!(
            options.mode,
            MeshRenderMode::FillAndIsolines { .. }
        ));
        assert!(matches!(options.color_scale, ColorScale::Magma));
        assert!(matches!(
            options.missing_value_policy,
            MissingValuePolicy::MaskNaN
        ));
        assert_eq!(options.viewport, Some([1.0, 3.0, -2.0, 2.0]));
        assert_eq!(
            options.selection.as_ref().and_then(|pick| pick.cell_id),
            Some(12)
        );
    }

    #[test]
    fn options_reject_invalid_renderer_configuration_before_state_mutation() {
        let mut spec = spec(serde_json::json!("auto"));
        spec.contour_levels = Some(serde_json::json!({"values": [2.0, 1.0]}));
        assert!(options(&spec, "mesh").unwrap_err().contains("increasing"));
    }
}
