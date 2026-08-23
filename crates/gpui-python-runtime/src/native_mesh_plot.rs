//! Translation from the versioned Python MeshPlot IR to native `gpui-px`
//! builder options.
//!
//! Keep this conversion in the runtime library rather than the showcase
//! binary: every native host must reject unsupported configuration before it
//! mutates a retained plot or its last valid frame.

use crate::cache::structural_fingerprint;
use crate::mesh_frames::MeshFrameStore;
use crate::meshplot::MeshPlotSpec;
use d3rs::mesh::{ContourLevels, MissingValuePolicy, RevolveSpec};
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

fn axes(spec: &MeshPlotSpec) -> Result<Axes2d, String> {
    let mut axes = if spec.equal_aspect {
        Axes2d::equal_aspect()
    } else {
        Axes2d::default().fill_aspect()
    };
    let Some(value) = spec.axes.as_ref().filter(|value| !value.is_null()) else {
        return Ok(axes);
    };
    let object = value
        .as_object()
        .ok_or("mesh_plot axes must be an object")?;
    let (default_horizontal, default_vertical) = match spec.view.as_str() {
        "axisymmetric_section" | "axisymmetric_revolve" => ("r", "z"),
        _ => ("x", "y"),
    };
    let horizontal = object
        .get("horizontal_label")
        .map(|value| {
            value
                .as_str()
                .ok_or("mesh_plot axes horizontal_label must be a string")
        })
        .transpose()?;
    let vertical = object
        .get("vertical_label")
        .map(|value| {
            value
                .as_str()
                .ok_or("mesh_plot axes vertical_label must be a string")
        })
        .transpose()?;
    if horizontal.is_some() || vertical.is_some() {
        axes = axes.labels(
            horizontal.unwrap_or(default_horizontal),
            vertical.unwrap_or(default_vertical),
        );
    }
    if let Some(unit) = object.get("unit") {
        axes = axes.unit(
            unit.as_str()
                .ok_or("mesh_plot axes unit must be a string")?,
        );
    }
    if let Some(range) = object.get("x_range") {
        let [min, max] = finite_json_pair(range, "axes.x_range")?;
        axes = axes.horizontal_range(min, max);
    }
    if let Some(range) = object.get("y_range") {
        let [min, max] = finite_json_pair(range, "axes.y_range")?;
        axes = axes.vertical_range(min, max);
    }
    if let Some(show_grid) = object.get("show_grid") {
        axes = axes.grid(
            show_grid
                .as_bool()
                .ok_or("mesh_plot axes show_grid must be boolean")?,
        );
    }
    Ok(axes)
}

fn interactions(spec: &MeshPlotSpec) -> Result<PlotInteractions, String> {
    let Some(interactions) = spec.interactions.as_ref() else {
        // A missing field is the legacy/default interactive preset.
        return Ok(PlotInteractions::InspectAndNavigate);
    };
    PlotInteractions::from_names(interactions)
}

fn coordinate_axis(
    value: Option<&Value>,
    default: CoordinateAxis,
    name: &str,
) -> Result<CoordinateAxis, String> {
    match value.and_then(Value::as_str) {
        None => Ok(default),
        Some("x") => Ok(CoordinateAxis::X),
        Some("y") => Ok(CoordinateAxis::Y),
        Some("z") => Ok(CoordinateAxis::Z),
        Some(_) => Err(format!("mesh_plot revolve {name} must be 'x', 'y', or 'z'")),
    }
}

fn revolve_spec(value: Option<&Value>) -> Result<RevolveSpec, String> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(RevolveSpec::default());
    };
    let object = value
        .as_object()
        .ok_or("mesh_plot revolve must be an object")?;
    let defaults = RevolveSpec::default();
    let radial = coordinate_axis(object.get("radial"), defaults.radial, "radial")?;
    let axial = coordinate_axis(object.get("axial"), defaults.axial, "axial")?;
    if radial == axial {
        return Err("mesh_plot revolve radial and axial axes must be distinct".into());
    }
    let start_angle = object
        .get("start_angle")
        .map_or(Some(defaults.start_angle), Value::as_f64)
        .filter(|value| value.is_finite())
        .ok_or("mesh_plot revolve start_angle must be finite")?;
    let sweep_angle = object
        .get("sweep_angle")
        .map_or(Some(defaults.sweep_angle), Value::as_f64)
        .filter(|value| value.is_finite() && *value > 0.0 && *value <= std::f64::consts::TAU)
        .ok_or("mesh_plot revolve sweep_angle must be in (0, 2*pi]")?;
    let segments = object
        .get("segments")
        .map_or(Some(defaults.segments), |value| {
            value.as_u64().and_then(|value| u32::try_from(value).ok())
        })
        .filter(|value| *value >= 3)
        .ok_or("mesh_plot revolve segments must be an integer of at least 3")?;
    let end_caps = object
        .get("end_caps")
        .map_or(Some(defaults.end_caps), Value::as_bool)
        .ok_or("mesh_plot revolve end_caps must be boolean")?;
    Ok(RevolveSpec {
        radial,
        axial,
        start_angle,
        sweep_angle,
        segments,
        end_caps,
    })
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
        axes: axes(spec)?,
        interactions: interactions(spec)?,
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

fn inline_float(value: &Value, name: &str, allow_nan: bool) -> Result<f64, String> {
    let value = value
        .as_f64()
        .ok_or_else(|| format!("{name} must be numeric"))?;
    if value.is_infinite() || (!allow_nan && value.is_nan()) {
        return Err(format!("{name} must be finite"));
    }
    Ok(value)
}

/// Resolve inline or retained-resource geometry while rejecting non-finite coordinates.
pub fn decode_geometry(
    geometry: &Value,
    store: &MeshFrameStore,
) -> Result<(Arc<[[f64; 3]]>, Arc<[[u32; 3]]>), String> {
    if geometry.get("resource_id").is_some() {
        return Err(
            "native mesh plot geometry resource_id is unsupported; provide separate positions and triangles resource handles"
                .into(),
        );
    }
    let split_resources = geometry.get("positions").is_some_and(Value::is_object)
        || geometry.get("triangles").is_some_and(Value::is_object);
    if split_resources {
        let (positions_id, positions_generation) = resource_ref(
            geometry
                .get("positions")
                .ok_or("mesh geometry is missing positions resource")?,
            "geometry.positions",
        )?;
        let (triangles_id, triangles_generation) = resource_ref(
            geometry
                .get("triangles")
                .ok_or("mesh geometry is missing triangles resource")?,
            "geometry.triangles",
        )?;
        return Ok((
            store.decoded_positions(positions_id, positions_generation)?,
            store.decoded_triangles(triangles_id, triangles_generation)?,
        ));
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
                inline_float(x, "mesh x", false)?,
                inline_float(y, "mesh y", false)?,
                inline_float(z, "mesh z", false)?,
            ])
        })
        .collect::<Result<Vec<_>, String>>()?
        .into();
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
        .collect::<Result<Vec<_>, String>>()?
        .into();
    Ok((positions, triangles))
}

/// Resolve field samples and their optional validity mask. NaNs are deliberately
/// preserved so `MissingValuePolicy::MaskNaN` can create the native invalid mask.
pub fn decode_field(
    field: &Value,
    store: &MeshFrameStore,
) -> Result<(Arc<[f64]>, Option<Arc<[bool]>>), String> {
    let values = if field.get("resource_id").is_some() {
        let (resource_id, generation) = resource_ref(field, "field")?;
        store.decoded_field(resource_id, generation)?
    } else {
        field
            .get("values")
            .and_then(Value::as_array)
            .ok_or("native mesh plot requires inline field values or a field resource")?
            .iter()
            .map(|value| inline_float(value, "mesh field value", true))
            .collect::<Result<Vec<_>, _>>()?
            .into()
    };
    let valid = match field.get("valid") {
        Some(value) if value.is_object() => {
            let (resource_id, generation) = resource_ref(value, "field.valid")?;
            Some(store.decoded_mask(resource_id, generation, values.len())?)
        }
        Some(value) => Some(
            value
                .as_array()
                .ok_or("mesh field valid mask must be an array")?
                .iter()
                .map(|value| {
                    value
                        .as_bool()
                        .ok_or_else(|| "mesh field valid mask must be boolean".to_string())
                })
                .collect::<Result<Vec<bool>, String>>()?
                .into(),
        ),
        None => None,
    };
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
) -> Result<Option<Arc<[u64]>>, String> {
    let Some(value) = geometry.get(name) else {
        return Ok(None);
    };
    if value.is_object() {
        let label = format!("geometry.{name}");
        let (resource_id, generation) = resource_ref(value, &label)?;
        return store
            .decoded_ids(resource_id, generation, expected, &label)
            .map(Some);
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
        .map(|values| Some(values.into()))
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
    let mesh = mesh_frames.cached_triangle_mesh(structural_fingerprint(geometry), || {
        let (positions, triangles) = decode_geometry(geometry, mesh_frames)?;
        let vertex_ids = decode_ids(geometry, "vertex_ids", positions.len(), mesh_frames)?;
        let cell_ids = decode_ids(geometry, "cell_ids", triangles.len(), mesh_frames)?;
        Ok(TriangleMesh {
            id: Arc::from(mesh_id),
            positions,
            triangles,
            vertex_ids,
            cell_ids,
        })
    })?;
    let mut plot = mesh_plot((*mesh).clone()).plot_id(spec.id.clone());
    if let Some(field) = spec.field.as_ref() {
        let field = mesh_frames.cached_scalar_field(structural_fingerprint(field), || {
            let (values, valid) = decode_field(field, mesh_frames)?;
            let association = match field
                .get("association")
                .and_then(Value::as_str)
                .unwrap_or("vertex")
            {
                "cell" => ScalarAssociation::Cell,
                _ => ScalarAssociation::Vertex,
            };
            Ok(ScalarField {
                id: Arc::from(field.get("id").and_then(Value::as_str).unwrap_or("field")),
                label: Arc::from(
                    field
                        .get("label")
                        .and_then(Value::as_str)
                        .unwrap_or("Field"),
                ),
                unit: field.get("unit").and_then(Value::as_str).map(Arc::from),
                values,
                association,
                valid,
            })
        })?;
        plot = plot.field((*field).clone());
    }
    let view = match spec.view.as_str() {
        "axisymmetric_section" => MeshPlotView::AxisymmetricSection {
            radial: CoordinateAxis::X,
            axial: CoordinateAxis::Z,
        },
        "axisymmetric_revolve" => {
            MeshPlotView::AxisymmetricRevolve(revolve_spec(spec.revolve.as_ref())?)
        }
        "surface3d" => MeshPlotView::Surface3d,
        _ => MeshPlotView::Planar {
            horizontal: CoordinateAxis::X,
            vertical: CoordinateAxis::Y,
        },
    };
    let options = options(spec, mesh_id)?;
    // Parse all 3D camera values before touching a retained state owner. A
    // malformed camera patch must take the same transactional path as a
    // malformed resource or render option and leave the last-valid state
    // untouched.
    #[cfg(feature = "gpu-3d")]
    let parsed_camera = if matches!(spec.view.as_str(), "surface3d" | "axisymmetric_revolve") {
        Some(parse_camera(spec.camera.as_ref())?)
    } else {
        None
    };
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
    let configuration_snapshot = state.borrow().configuration_snapshot();
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
    if let Some(camera) = parsed_camera {
        apply_camera(&mut state.borrow_mut(), camera);
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
    let element = match plot.build() {
        Ok(element) => element,
        Err(error) => {
            state
                .borrow_mut()
                .restore_configuration(configuration_snapshot);
            return Err(error.to_string());
        }
    };
    Ok((div().size_full().child(element).into_any_element(), state))
}

#[cfg(feature = "gpu-3d")]
#[derive(Debug, Clone, Copy)]
struct ParsedCamera {
    distance: Option<f32>,
    azimuth: Option<f32>,
    elevation: Option<f32>,
    target: Option<[f32; 3]>,
}

#[cfg(feature = "gpu-3d")]
fn parse_camera(value: Option<&Value>) -> Result<ParsedCamera, String> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(ParsedCamera {
            distance: None,
            azimuth: None,
            elevation: None,
            target: None,
        });
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
    let target = if let Some(value) = object.get("target") {
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
        Some([values[0], values[1], values[2]])
    } else {
        None
    };
    Ok(ParsedCamera {
        distance: finite_f32("distance")?,
        azimuth: finite_f32("azimuth")?,
        elevation: finite_f32("elevation")?,
        target,
    })
}

#[cfg(feature = "gpu-3d")]
fn apply_camera(state: &mut MeshPlotState, camera: ParsedCamera) {
    if let Some(value) = camera.distance {
        state.orbit.distance = value.clamp(state.orbit.min_distance, state.orbit.max_distance);
    }
    if let Some(value) = camera.azimuth {
        state.orbit.azimuth = value;
    }
    if let Some(value) = camera.elevation {
        state.orbit.elevation = value.clamp(state.orbit.min_elevation, state.orbit.max_elevation);
    }
    if let Some(target) = camera.target {
        state.orbit.target = glam::Vec3::from_array(target);
    }
    state.orbit.update_camera(&mut state.camera);
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

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn invalid_camera_is_rejected_before_retained_state_mutation() {
        let mut plot_spec = spec(serde_json::json!("auto"));
        plot_spec.view = "surface3d".into();
        plot_spec.camera = Some(serde_json::json!({
            "distance": 2.0,
            "target": [0.0, "not-a-number", 0.0]
        }));

        let state = Rc::new(RefCell::new(MeshPlotState::new(0.0, 1.0, 0.0, 1.0)));
        let before = {
            let state = state.borrow();
            (
                state.interaction.zoom.clone(),
                state.selection.clone(),
                state.wireframe,
                state.render_mode.clone(),
                state.color_range.clone(),
                state.geometry_revision,
                state.field_revision,
                state.orbit.distance,
                state.orbit.azimuth,
                state.orbit.elevation,
                state.orbit.target,
            )
        };

        let error = match build(
            &plot_spec,
            &MeshFrameStore::new(),
            Some(state.clone()),
            None,
        ) {
            Ok(_) => panic!("an invalid camera must reject the native build"),
            Err(error) => error,
        };
        assert!(error.contains("camera target"));

        let after = {
            let state = state.borrow();
            (
                state.interaction.zoom.clone(),
                state.selection.clone(),
                state.wireframe,
                state.render_mode.clone(),
                state.color_range.clone(),
                state.geometry_revision,
                state.field_revision,
                state.orbit.distance,
                state.orbit.azimuth,
                state.orbit.elevation,
                state.orbit.target,
            )
        };
        assert_eq!(after, before);
    }

    #[test]
    fn invalid_decoded_field_length_restores_retained_configuration() {
        let mut plot_spec = spec(serde_json::json!("auto"));
        plot_spec.field = Some(serde_json::json!({
            "id": "field",
            "values": [0.0, 1.0],
            "association": "vertex"
        }));

        let state = Rc::new(RefCell::new(MeshPlotState::new(0.0, 1.0, 0.0, 1.0)));
        let before = {
            let state = state.borrow();
            (
                state.interaction.zoom.clone(),
                state.selection.clone(),
                state.wireframe,
                state.render_mode.clone(),
                state.color_range.clone(),
            )
        };

        let error = match build(
            &plot_spec,
            &MeshFrameStore::new(),
            Some(state.clone()),
            None,
        ) {
            Ok(_) => panic!("a field-length mismatch must reject the native build"),
            Err(error) => error,
        };
        assert!(error.contains("expected 3"));

        let after = {
            let state = state.borrow();
            (
                state.interaction.zoom.clone(),
                state.selection.clone(),
                state.wireframe,
                state.render_mode.clone(),
                state.color_range.clone(),
            )
        };
        assert_eq!(after, before);
    }

    #[test]
    fn options_convert_python_axes_configuration() {
        let mut value = serde_json::json!({
            "schema_version": 1,
            "id": "axes",
            "geometry": {
                "id": "mesh",
                "positions": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                "triangles": [[0, 1, 2]]
            },
            "axes": {
                "horizontal_label": "distance",
                "unit": "m",
                "x_range": [0.0, 2.0],
                "y_range": [-1.0, 3.0],
                "show_grid": false
            }
        });
        let spec = MeshPlotSpec::from_value(value.take()).unwrap();
        let options = options(&spec, "mesh").unwrap();
        assert_eq!(
            options.axes,
            Axes2d::equal_aspect()
                .labels("distance", "y")
                .unit("m")
                .horizontal_range(0.0, 2.0)
                .vertical_range(-1.0, 3.0)
                .grid(false)
        );
    }

    #[test]
    fn options_preserve_default_disable_and_map_partial_interactions() {
        let mut default_spec = spec(serde_json::json!("auto"));
        default_spec.interactions = None;
        assert_eq!(
            options(&default_spec, "mesh").unwrap().interactions,
            PlotInteractions::InspectAndNavigate
        );

        let mut disabled_spec = default_spec.clone();
        disabled_spec.interactions = Some(Vec::new());
        assert_eq!(
            options(&disabled_spec, "mesh").unwrap().interactions,
            PlotInteractions::None
        );

        let mut partial_spec = default_spec;
        partial_spec.interactions = Some(vec!["pan".into()]);
        let interactions = options(&partial_spec, "mesh").unwrap().interactions;
        assert!(interactions.allows_pan());
        assert!(!interactions.allows_zoom());
    }

    #[test]
    fn revolve_settings_preserve_partial_sweep_segments_and_caps() {
        let revolve = revolve_spec(Some(&serde_json::json!({
            "radial": "y",
            "axial": "z",
            "start_angle": 0.25,
            "sweep_angle": 1.5,
            "segments": 32,
            "end_caps": true
        })))
        .unwrap();
        assert_eq!(revolve.radial, CoordinateAxis::Y);
        assert_eq!(revolve.axial, CoordinateAxis::Z);
        assert_eq!(revolve.start_angle, 0.25);
        assert_eq!(revolve.sweep_angle, 1.5);
        assert_eq!(revolve.segments, 32);
        assert!(revolve.end_caps);
    }

    #[test]
    fn inline_decoders_preserve_the_resource_validation_contract() {
        let store = MeshFrameStore::new();
        let geometry = serde_json::json!({
            "positions": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            "triangles": [[0, 1, 2]]
        });
        let (positions, triangles) = decode_geometry(&geometry, &store).unwrap();
        assert_eq!(positions.len(), 3);
        assert_eq!(triangles.as_ref(), &[[0, 1, 2]]);

        let field = serde_json::json!({
            "values": [1.0, 2.0, 3.0],
            "valid": [true, false, true]
        });
        let (values, valid) = decode_field(&field, &store).unwrap();
        assert_eq!(values.as_ref(), &[1.0, 2.0, 3.0]);
        assert_eq!(valid.as_deref(), Some(&[true, false, true][..]));
    }

    #[test]
    fn inline_decoders_reject_non_numeric_geometry_and_field_values() {
        let store = MeshFrameStore::new();
        let geometry = serde_json::json!({
            "positions": [[0.0, "invalid", 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            "triangles": [[0, 1, 2]]
        });
        assert!(
            decode_geometry(&geometry, &store)
                .unwrap_err()
                .contains("mesh y must be numeric")
        );

        let field = serde_json::json!({"values": [1.0, "invalid", 3.0]});
        assert!(
            decode_field(&field, &store)
                .unwrap_err()
                .contains("mesh field value must be numeric")
        );
    }

    #[test]
    fn geometry_decoder_rejects_unsupported_whole_resource_handles() {
        let store = MeshFrameStore::new();
        let geometry = serde_json::json!({
            "resource_id": "geometry",
            "generation": 1
        });
        assert!(
            decode_geometry(&geometry, &store)
                .unwrap_err()
                .contains("geometry resource_id is unsupported")
        );
    }
}
