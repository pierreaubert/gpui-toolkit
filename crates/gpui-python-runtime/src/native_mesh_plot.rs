//! Translation from the versioned Python MeshPlot IR to native `gpui-px`
//! builder options.
//!
//! Keep this conversion in the runtime library rather than the showcase
//! binary: every native host must reject unsupported configuration before it
//! mutates a retained plot or its last valid frame.

use crate::mesh_frames::{MeshDtype, MeshFrameKind, MeshFrameStore, RetainedMeshResource};
use crate::meshplot::MeshPlotSpec;
use d3rs::mesh::{ContourLevels, MissingValuePolicy};
use gpui_px::{
    AutoOrFixed, Axes2d, ColorRange, ColorScale, FieldInterpolation, MeshPlotPick, MeshRenderMode,
    PlotInteractions,
};
use serde_json::Value;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use d3rs::mesh::{CoordinateAxis, ScalarAssociation, ScalarField, TriangleMesh, project_2d};
use gpui::{AnyElement, IntoElement, ParentElement, Styled, div};
use gpui_px::{Colorbar, MeshPlotState, MeshPlotView, Wireframe, mesh_plot};

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
            ));
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

fn resource_ref<'a>(value: &'a Value, name: &str) -> Result<(&'a str, u64), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{name} resource handle must be an object"))?;
    let resource_id = object
        .get("resource_id")
        .or_else(|| object.get("id"))
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| format!("{name} resource_id must be a non-empty string"))?;
    let generation = object
        .get("generation")
        .and_then(Value::as_u64)
        .filter(|generation| *generation > 0)
        .ok_or_else(|| format!("{name} resource generation must be positive"))?;
    Ok((resource_id, generation))
}

fn resource<'a>(
    store: &'a MeshFrameStore,
    value: &Value,
    name: &str,
    expected_kind: MeshFrameKind,
) -> Result<&'a RetainedMeshResource, String> {
    let (resource_id, generation) = resource_ref(value, name)?;
    let resource = store.get(resource_id, generation).ok_or_else(|| {
        format!("missing {name} resource {resource_id:?} generation {generation}")
    })?;
    if resource.kind != expected_kind {
        return Err(format!(
            "{name} resource {resource_id:?} has kind {:?}, expected {:?}",
            resource.kind, expected_kind
        ));
    }
    Ok(resource)
}

fn shape_elements(resource: &RetainedMeshResource, name: &str) -> Result<usize, String> {
    if resource.shape.is_empty() || resource.shape.contains(&0) {
        return Err(format!(
            "{name} resource shape must contain positive dimensions"
        ));
    }
    resource.shape.iter().try_fold(1usize, |count, dimension| {
        count
            .checked_mul(*dimension as usize)
            .ok_or_else(|| format!("{name} resource shape is too large"))
    })
}

fn floats(
    resource: &RetainedMeshResource,
    name: &str,
    allow_nan: bool,
) -> Result<Vec<f64>, String> {
    let elements = shape_elements(resource, name)?;
    let width = match resource.dtype {
        MeshDtype::F32LE => 4,
        MeshDtype::F64LE => 8,
        dtype => {
            return Err(format!(
                "{name} resource dtype {dtype:?} is not f32le/f64le"
            ));
        }
    };
    let expected = elements
        .checked_mul(width)
        .ok_or_else(|| format!("{name} resource payload is too large"))?;
    if resource.payload.len() != expected {
        return Err(format!(
            "{name} resource payload has {} bytes, expected {expected}",
            resource.payload.len()
        ));
    }
    resource
        .payload
        .chunks_exact(width)
        .map(|chunk| {
            let value = if width == 4 {
                f32::from_le_bytes(chunk.try_into().map_err(|_| "invalid f32 bytes")?) as f64
            } else {
                f64::from_le_bytes(chunk.try_into().map_err(|_| "invalid f64 bytes")?)
            };
            if value.is_infinite() || (!allow_nan && value.is_nan()) {
                return Err(format!("{name} resource contains a non-finite value"));
            }
            Ok(value)
        })
        .collect()
}

fn u32s(resource: &RetainedMeshResource, name: &str) -> Result<Vec<u32>, String> {
    let elements = shape_elements(resource, name)?;
    if resource.dtype != MeshDtype::U32LE {
        return Err(format!(
            "{name} resource dtype {:?} is not u32le",
            resource.dtype
        ));
    }
    let expected = elements
        .checked_mul(4)
        .ok_or_else(|| format!("{name} resource payload is too large"))?;
    if resource.payload.len() != expected {
        return Err(format!(
            "{name} resource payload has {} bytes, expected {expected}",
            resource.payload.len()
        ));
    }
    resource
        .payload
        .chunks_exact(4)
        .map(|chunk| {
            Ok(u32::from_le_bytes(
                chunk.try_into().map_err(|_| "invalid u32 bytes")?,
            ))
        })
        .collect()
}

fn u64s(resource: &RetainedMeshResource, name: &str) -> Result<Vec<u64>, String> {
    let elements = shape_elements(resource, name)?;
    if resource.dtype != MeshDtype::U64LE {
        return Err(format!(
            "{name} resource dtype {:?} is not u64le",
            resource.dtype
        ));
    }
    let expected = elements
        .checked_mul(8)
        .ok_or_else(|| format!("{name} resource payload is too large"))?;
    if resource.payload.len() != expected {
        return Err(format!(
            "{name} resource payload has {} bytes, expected {expected}",
            resource.payload.len()
        ));
    }
    resource
        .payload
        .chunks_exact(8)
        .map(|chunk| {
            Ok(u64::from_le_bytes(
                chunk.try_into().map_err(|_| "invalid u64 bytes")?,
            ))
        })
        .collect()
}

/// Resolve inline or retained-resource geometry while rejecting non-finite coordinates.
pub fn decode_geometry(
    geometry: &Value,
    store: &MeshFrameStore,
) -> Result<(Vec<[f64; 3]>, Vec<[u32; 3]>), String> {
    let split_resources = geometry.get("positions").is_some_and(Value::is_object)
        || geometry.get("triangles").is_some_and(Value::is_object);
    if split_resources {
        let positions_resource = resource(
            store,
            geometry
                .get("positions")
                .ok_or("mesh geometry is missing positions resource")?,
            "geometry.positions",
            MeshFrameKind::Geometry,
        )?;
        let triangles_resource = resource(
            store,
            geometry
                .get("triangles")
                .ok_or("mesh geometry is missing triangles resource")?,
            "geometry.triangles",
            MeshFrameKind::Geometry,
        )?;
        if positions_resource.shape.len() != 2 || positions_resource.shape[1] != 3 {
            return Err("geometry.positions resource shape must be [vertex_count, 3]".into());
        }
        if triangles_resource.shape.len() != 2 || triangles_resource.shape[1] != 3 {
            return Err("geometry.triangles resource shape must be [triangle_count, 3]".into());
        }
        let positions = floats(positions_resource, "geometry.positions", false)?
            .chunks_exact(3)
            .map(|values| [values[0], values[1], values[2]])
            .collect();
        let triangles = u32s(triangles_resource, "geometry.triangles")?
            .chunks_exact(3)
            .map(|values| [values[0], values[1], values[2]])
            .collect();
        return Ok((positions, triangles));
    }
    let positions = geometry
        .get("positions")
        .and_then(Value::as_array)
        .ok_or("native mesh plot requires inline positions or split resources")?
        .iter()
        .map(|point| {
            let values = point.as_array().ok_or("mesh position must be an array")?;
            let [x, y, z] = values.as_slice() else {
                return Err("mesh position must contain three values".into());
            };
            Ok([
                x.as_f64().ok_or("mesh x must be numeric")?,
                y.as_f64().ok_or("mesh y must be numeric")?,
                z.as_f64().ok_or("mesh z must be numeric")?,
            ])
        })
        .collect::<Result<Vec<_>, String>>()?;
    let triangles = geometry
        .get("triangles")
        .and_then(Value::as_array)
        .ok_or("native mesh plot requires inline triangles or split resources")?
        .iter()
        .map(|triangle| {
            let values = triangle
                .as_array()
                .ok_or("mesh triangle must be an array")?;
            let [a, b, c] = values.as_slice() else {
                return Err("mesh triangle must contain three indices".into());
            };
            Ok([
                a.as_u64().ok_or("mesh index must be an integer")? as u32,
                b.as_u64().ok_or("mesh index must be an integer")? as u32,
                c.as_u64().ok_or("mesh index must be an integer")? as u32,
            ])
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok((positions, triangles))
}

/// Resolve field samples and their optional validity mask. NaNs are deliberately
/// preserved so `MissingValuePolicy::MaskNaN` can create the native invalid mask.
pub fn decode_field(
    field: &Value,
    store: &MeshFrameStore,
) -> Result<(Vec<f64>, Option<Vec<bool>>), String> {
    let values = if field.get("resource_id").is_some() {
        let resource = resource(store, field, "field", MeshFrameKind::Field)?;
        if resource.shape.len() != 1 {
            return Err("field resource shape must be [value_count]".into());
        }
        floats(resource, "field", true)?
    } else {
        field
            .get("values")
            .and_then(Value::as_array)
            .ok_or("native mesh plot requires inline field values or a field resource")?
            .iter()
            .map(|value| value.as_f64().ok_or("mesh field value must be numeric"))
            .collect::<Result<Vec<_>, _>>()?
    };
    let valid = field
        .get("valid")
        .map(|value| {
            if value.is_object() {
                let resource = resource(store, value, "field.valid", MeshFrameKind::Mask)?;
                let elements = shape_elements(resource, "field.valid")?;
                if resource.shape.len() != 1 {
                    return Err("field.valid resource shape must be [value_count]".into());
                }
                if elements != values.len() {
                    return Err(format!(
                        "field.valid resource has {elements} values, expected {}",
                        values.len()
                    ));
                }
                let expected = match resource.dtype {
                    MeshDtype::BoolBytes => elements,
                    MeshDtype::BoolPacked => {
                        elements
                            .checked_add(7)
                            .ok_or("field.valid resource shape is too large")?
                            / 8
                    }
                    dtype => {
                        return Err(format!(
                            "field.valid resource dtype {dtype:?} is not bool_bytes/bool_packed"
                        ));
                    }
                };
                if resource.payload.len() != expected {
                    return Err(format!(
                        "field.valid resource payload has {} bytes, expected {expected}",
                        resource.payload.len()
                    ));
                }
                match resource.dtype {
                    MeshDtype::BoolBytes => resource
                        .payload
                        .iter()
                        .enumerate()
                        .map(|(index, value)| match value {
                            0 => Ok(false),
                            1 => Ok(true),
                            _ => Err(format!("field.valid resource byte {index} is not boolean")),
                        })
                        .collect::<Result<Vec<bool>, String>>(),
                    MeshDtype::BoolPacked => (0..elements)
                        .map(|index| Ok(resource.payload[index / 8] & (1 << (index % 8)) != 0))
                        .collect::<Result<Vec<bool>, String>>(),
                    _ => unreachable!("dtype checked above"),
                }
            } else {
                value
                    .as_array()
                    .ok_or("mesh field valid mask must be an array")?
                    .iter()
                    .map(|value| {
                        value
                            .as_bool()
                            .ok_or_else(|| "mesh field valid mask must be boolean".to_string())
                    })
                    .collect::<Result<Vec<bool>, String>>()
            }
        })
        .transpose()?;
    if valid
        .as_ref()
        .is_some_and(|valid| valid.len() != values.len())
    {
        return Err("mesh field valid mask length must match values".into());
    }
    Ok((values, valid))
}

/// Resolve optional stable vertex or cell IDs from inline data or a retained resource.
pub fn decode_ids(
    geometry: &Value,
    name: &str,
    expected: usize,
    store: &MeshFrameStore,
) -> Result<Option<Vec<u64>>, String> {
    let Some(value) = geometry.get(name) else {
        return Ok(None);
    };
    if value.is_object() {
        let resource = resource(
            store,
            value,
            &format!("geometry.{name}"),
            MeshFrameKind::Ids,
        )?;
        if resource.shape.len() != 1 || resource.shape[0] as usize != expected {
            return Err(format!(
                "geometry.{name} resource shape must be [{expected}]"
            ));
        }
        return u64s(resource, &format!("geometry.{name}")).map(Some);
    }
    let values = value
        .as_array()
        .ok_or_else(|| format!("mesh geometry {name} must be an array"))?;
    if values.len() != expected {
        return Err(format!(
            "mesh geometry {name} has {}, expected {expected} values",
            values.len()
        ));
    }
    values
        .iter()
        .map(|value| {
            value
                .as_u64()
                .ok_or_else(|| format!("mesh geometry {name} must contain u64 ids"))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

/// Callback emitted when the retained native plot changes its selection.
pub type SelectionCallback = Rc<dyn Fn(Option<MeshPlotPick>)>;

/// Construct a retained native MeshPlot from a versioned Python spec.
///
/// This is deliberately in the runtime library rather than a particular host
/// binary so resource-backed and inline plots have identical native state,
/// validation, camera, selection, and colorbar behavior everywhere.
pub fn build(
    spec: &MeshPlotSpec,
    mesh_frames: &MeshFrameStore,
    retained_state: Option<Rc<RefCell<MeshPlotState>>>,
    selection_callback: Option<SelectionCallback>,
) -> Result<(AnyElement, Rc<RefCell<MeshPlotState>>), String> {
    let geometry = &spec.geometry;
    let mesh_id = geometry.get("id").and_then(Value::as_str).unwrap_or("mesh");
    let (positions, triangles) = decode_geometry(geometry, mesh_frames)?;
    let vertex_ids =
        decode_ids(geometry, "vertex_ids", positions.len(), mesh_frames)?.map(Arc::from);
    let cell_ids = decode_ids(geometry, "cell_ids", triangles.len(), mesh_frames)?.map(Arc::from);
    let mesh = TriangleMesh {
        id: Arc::from(mesh_id),
        positions: positions.into(),
        triangles: triangles.into(),
        vertex_ids,
        cell_ids,
    };
    let mut plot = mesh_plot(mesh.clone()).plot_id(spec.id.clone());
    if let Some(field) = spec.field.as_ref() {
        let (values, valid) = decode_field(field, mesh_frames)?;
        let association = match field
            .get("association")
            .and_then(Value::as_str)
            .unwrap_or("vertex")
        {
            "cell" => ScalarAssociation::Cell,
            _ => ScalarAssociation::Vertex,
        };
        plot = plot.field(ScalarField {
            id: Arc::from(field.get("id").and_then(Value::as_str).unwrap_or("field")),
            label: Arc::from(
                field
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or("Field"),
            ),
            unit: field.get("unit").and_then(Value::as_str).map(Arc::from),
            values: values.into(),
            association,
            valid: valid.map(Arc::from),
        });
    }
    let view = match spec.view.as_str() {
        "axisymmetric_section" => MeshPlotView::AxisymmetricSection {
            radial: CoordinateAxis::X,
            axial: CoordinateAxis::Z,
        },
        "axisymmetric_revolve" => MeshPlotView::AxisymmetricRevolve(Default::default()),
        "surface3d" => MeshPlotView::Surface3d,
        _ => MeshPlotView::Planar {
            horizontal: CoordinateAxis::X,
            vertical: CoordinateAxis::Y,
        },
    };
    let options = options(spec, mesh_id)?;
    let (horizontal, vertical) = match spec.view.as_str() {
        "axisymmetric_section" | "axisymmetric_revolve" => (CoordinateAxis::X, CoordinateAxis::Z),
        _ => (CoordinateAxis::X, CoordinateAxis::Y),
    };
    let mut x_min = f64::INFINITY;
    let mut x_max = f64::NEG_INFINITY;
    let mut y_min = f64::INFINITY;
    let mut y_max = f64::NEG_INFINITY;
    for position in mesh.positions.iter() {
        let projected = project_2d(horizontal, vertical, *position);
        x_min = x_min.min(projected[0]);
        x_max = x_max.max(projected[0]);
        y_min = y_min.min(projected[1]);
        y_max = y_max.max(projected[1]);
    }
    let x_min = if x_min.is_finite() { x_min } else { 0.0 };
    let y_min = if y_min.is_finite() { y_min } else { 0.0 };
    let x_max = if x_max.is_finite() {
        x_max.max(x_min + f64::EPSILON)
    } else {
        x_min + 1.0
    };
    let y_max = if y_max.is_finite() {
        y_max.max(y_min + f64::EPSILON)
    } else {
        y_min + 1.0
    };
    let state = retained_state
        .unwrap_or_else(|| Rc::new(RefCell::new(MeshPlotState::new(x_min, x_max, y_min, y_max))));
    if let Some([x_min, x_max, y_min, y_max]) = options.viewport {
        state
            .borrow_mut()
            .set_viewport_without_history(x_min, x_max, y_min, y_max);
    }
    if let Some(selection) = &options.selection {
        state.borrow_mut().set_selection(Some(selection.clone()));
    }
    state.borrow_mut().set_style(
        options.mode.clone(),
        if spec.wireframe {
            Wireframe::Overlay
        } else {
            Wireframe::Hidden
        },
        options.color_range.clone(),
    );
    #[cfg(feature = "gpu-3d")]
    if matches!(spec.view.as_str(), "surface3d" | "axisymmetric_revolve") {
        apply_camera(&mut state.borrow_mut(), spec.camera.as_ref())?;
    }
    plot = plot
        .view(view)
        .mode(options.mode)
        .color_scale(options.color_scale)
        .color_range(options.color_range)
        .missing_value_policy(options.missing_value_policy)
        .axes(options.axes)
        .interactions(options.interactions)
        .wireframe(if spec.wireframe {
            Wireframe::Overlay
        } else {
            Wireframe::Hidden
        })
        .on_selection(move |selection| {
            if let Some(callback) = &selection_callback {
                callback(selection);
            }
        })
        .with_state(state.clone());
    if let Some(selection) = options.selection {
        plot = plot.selection(selection);
    }
    if let Some(field) = spec.field.as_ref() {
        let mut colorbar = Colorbar::new(
            field
                .get("label")
                .and_then(Value::as_str)
                .unwrap_or("Field"),
        );
        if let Some(unit) = field.get("unit").and_then(Value::as_str) {
            colorbar = colorbar.unit(unit);
        }
        plot = plot.colorbar(colorbar);
    }
    if let Some(title) = &spec.title {
        plot = plot.title(title.clone());
    }
    if let (Some(width), Some(height)) = (spec.width, spec.height) {
        plot = plot.size(width, height);
    }
    plot.build()
        .map(|element| (div().size_full().child(element).into_any_element(), state))
        .map_err(|error| error.to_string())
}

#[cfg(feature = "gpu-3d")]
fn apply_camera(state: &mut MeshPlotState, value: Option<&Value>) -> Result<(), String> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(());
    };
    let object = value
        .as_object()
        .ok_or("mesh_plot camera must be an object or null")?;
    let finite_f32 = |name: &str| -> Result<Option<f32>, String> {
        object
            .get(name)
            .filter(|value| !value.is_null())
            .map(|value| {
                let value = value
                    .as_f64()
                    .filter(|value| value.is_finite())
                    .ok_or_else(|| format!("mesh_plot camera {name} must be finite"))?;
                if !(value as f32).is_finite() {
                    return Err(format!("mesh_plot camera {name} is outside f32 range"));
                }
                Ok(value as f32)
            })
            .transpose()
    };
    if let Some(value) = finite_f32("distance")? {
        state.orbit.distance = value.clamp(state.orbit.min_distance, state.orbit.max_distance);
    }
    if let Some(value) = finite_f32("azimuth")? {
        state.orbit.azimuth = value;
    }
    if let Some(value) = finite_f32("elevation")? {
        state.orbit.elevation = value.clamp(state.orbit.min_elevation, state.orbit.max_elevation);
    }
    if let Some(value) = object.get("target") {
        let values = value
            .as_array()
            .filter(|values| values.len() == 3)
            .ok_or("mesh_plot camera target must contain three values")?
            .iter()
            .map(|value| {
                value
                    .as_f64()
                    .filter(|value| value.is_finite())
                    .map(|value| value as f32)
                    .filter(|value| value.is_finite())
                    .ok_or("mesh_plot camera target must contain finite values")
            })
            .collect::<Result<Vec<_>, _>>()?;
        state.orbit.target = glam::Vec3::new(values[0], values[1], values[2]);
    }
    state.orbit.update_camera(&mut state.camera);
    Ok(())
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
