//! JSON bridge from the Python wrapper to `gpui-mesh` and
//! `gpui-dataset-transformation`.
//!
//! The Python facade (`gpui_toolkit.mesh`) builds a plain JSON spec: a
//! structured grid plus named scalar/vector fields, followed by a chain of
//! filter ops. The functions here parse that spec, run the real Rust
//! filters, and return a JSON summary (or streamline polylines). Errors are
//! returned as messages; an unknown op is a hard error, never a silent
//! no-op.

// Rust guideline compliant 2026-02-21

use gpui_dataset_transformation::{
    CellToPointLimits, CellToPointPlan, ParticleOptions, Plane, ThresholdMode, TransformError,
    append_datasets, calculator, cell_data_to_point_data, clip_halfspace, clip_sphere,
    contour_lines, convex_hull, dashed_lines, decimate, delaunay_2d, divergence, feature_edges,
    field_statistics, histogram, hyperstreamlines, isosurface, lambda2, lic, particle_frames,
    point_data_to_cell_data, q_criterion, resample_to_grid, slice, smooth, solid_arrows,
    stream_ribbons, streamlines, streamtubes, surface_normals, swirling_strength, tensor_glyphs,
    threshold, transform_dataset, vector_magnitude, vorticity, vorticity_magnitude,
};
use gpui_mesh::{Association, DataSet, Field};
use serde::Deserialize;
use serde_json::{Value, json};

/// Parses one dataset spec (`structured` grid + `fields`) into a [`DataSet`].
///
/// # Errors
///
/// Returns a message when the JSON is malformed, the grid dimensions are
/// invalid, a field length does not match the point count, or a field kind
/// is unknown.
pub fn dataset_from_spec(spec: &Value) -> Result<DataSet, String> {
    let grid: GridSpec =
        serde_json::from_value(spec.get("structured").cloned().unwrap_or(Value::Null))
            .map_err(|e| format!("invalid 'structured' grid: {e}"))?;
    let mut dataset = DataSet::structured_grid(
        spec.get("id").and_then(Value::as_str).unwrap_or("mesh"),
        grid.dims,
        grid.origin,
        grid.spacing,
    )
    .map_err(|e| e.to_string())?;
    let points = dataset.point_count();
    if let Some(fields) = spec.get("fields").and_then(Value::as_array) {
        for field_spec in fields {
            let field: FieldSpec =
                serde_json::from_value(field_spec.clone()).map_err(|e| e.to_string())?;
            let count = match field.kind.as_str() {
                "scalar" => field.values.len(),
                "vector" => field.vectors.as_ref().map_or(0, Vec::len),
                "tensor" => field.tensors.as_ref().map_or(0, Vec::len),
                other => return Err(format!("unknown field kind '{other}'")),
            };
            if count != points {
                return Err(format!(
                    "field '{}' has {count} entries but the grid has {points} points",
                    field.name
                ));
            }
            let built = match field.kind.as_str() {
                "scalar" => Field::scalar(field.name.clone(), field.values),
                "vector" => Field::vector(field.name.clone(), field.vectors.unwrap_or_default()),
                "tensor" => Field::tensor(field.name.clone(), field.tensors.unwrap_or_default()),
                _ => unreachable!(),
            };
            dataset = dataset.with_field(built);
        }
    }
    dataset.validate().map_err(|e| e.to_string())?;
    Ok(dataset)
}

/// Applies one filter op, returning the transformed dataset.
///
/// # Errors
///
/// Returns a message for unknown ops, missing fields, or filter failures.
pub fn apply_op(dataset: &DataSet, op: &Value) -> Result<DataSet, String> {
    let name = op.get("op").and_then(Value::as_str).unwrap_or("");
    let err = |e: gpui_dataset_transformation::TransformError| e.to_string();
    let plane = |op: &Value| -> Result<Plane, String> {
        let normal = vec3(op, "normal")?;
        let point = vec3(op, "point").unwrap_or([0.0; 3]);
        Plane::new(point, normal).map_err(|e| e.to_string())
    };
    match name {
        "slice" => slice(dataset, &plane(op)?).map_err(err),
        "clip" => {
            let keep = op.get("keep").and_then(Value::as_str).unwrap_or("positive");
            let keep_positive = match keep {
                "positive" => true,
                "negative" => false,
                other => return Err(format!("unknown clip keep mode '{other}'")),
            };
            clip_halfspace(dataset, &plane(op)?, keep_positive).map_err(err)
        }
        "threshold" => {
            let mode = match op.get("mode").and_then(Value::as_str).unwrap_or("all") {
                "all" => ThresholdMode::All,
                "any" => ThresholdMode::Any,
                other => return Err(format!("unknown threshold mode '{other}'")),
            };
            threshold(
                dataset,
                str_field(op, "field")?,
                Association::Point,
                f64_field(op, "min")?,
                f64_field(op, "max")?,
                mode,
            )
            .map_err(err)
        }
        "isosurface" => {
            isosurface(dataset, str_field(op, "field")?, f64_field(op, "value")?).map_err(err)
        }
        "vorticity" => vorticity(dataset, str_field(op, "field")?, "vorticity").map_err(err),
        "vorticity_magnitude" => {
            vorticity_magnitude(dataset, str_field(op, "field")?, "vorticity_magnitude")
                .map_err(err)
        }
        "divergence" => divergence(dataset, str_field(op, "field")?, "divergence").map_err(err),
        "q_criterion" => q_criterion(dataset, str_field(op, "field")?, "q_criterion").map_err(err),
        "lambda2" => lambda2(dataset, str_field(op, "field")?, "lambda2").map_err(err),
        "swirling_strength" => {
            swirling_strength(dataset, str_field(op, "field")?, "swirling_strength").map_err(err)
        }
        "solid_arrows" => solid_arrows(
            dataset,
            str_field(op, "field")?,
            f64_field(op, "scale").unwrap_or(1.0),
            usize_field(op, "stride").unwrap_or(1),
            f64_field(op, "radius_fraction").unwrap_or(0.05),
        )
        .map_err(err),
        "tensor_glyphs" => tensor_glyphs(
            dataset,
            str_field(op, "field")?,
            f64_field(op, "scale").unwrap_or(1.0),
            usize_field(op, "stride").unwrap_or(1),
        )
        .map_err(err),
        "streamlines" => {
            let seeds: Vec<[f64; 3]> =
                serde_json::from_value(op.get("seeds").cloned().unwrap_or(Value::Null))
                    .map_err(|e| format!("invalid 'seeds': {e}"))?;
            streamlines(
                dataset,
                str_field(op, "field").unwrap_or("velocity"),
                &seeds,
                f64_field(op, "step").unwrap_or(0.05),
                usize_field(op, "max_steps").unwrap_or(500),
            )
            .map_err(err)
        }
        "streamtubes" => streamtubes(
            dataset,
            f64_field(op, "radius")?,
            usize_field(op, "sides").unwrap_or(6),
        )
        .map_err(err),
        "stream_ribbons" => stream_ribbons(dataset, f64_field(op, "width")?).map_err(err),
        "dashed_lines" => dashed_lines(
            dataset,
            f64_field(op, "phase").unwrap_or(0.0),
            f64_field(op, "dash")?,
            f64_field(op, "gap")?,
        )
        .map_err(err),
        "contour_lines" => {
            let levels: Vec<f64> =
                serde_json::from_value(op.get("levels").cloned().unwrap_or(Value::Null))
                    .map_err(|e| format!("invalid 'levels': {e}"))?;
            contour_lines(dataset, str_field(op, "field")?, &levels).map_err(err)
        }
        "lic" => lic(
            dataset,
            str_field(op, "field").unwrap_or("velocity"),
            usize_field(op, "kernel_steps").unwrap_or(10),
            f64_field(op, "step")?,
            usize_field(op, "seed").unwrap_or(0) as u64,
        )
        .map_err(err),
        "point_to_cell" => point_data_to_cell_data(
            dataset,
            str_field(op, "field")?,
            op.get("output")
                .and_then(Value::as_str)
                .unwrap_or("cell_value"),
        )
        .map_err(err),
        "cell_to_point" => cell_data_to_point_data(
            dataset,
            str_field(op, "field")?,
            op.get("output")
                .and_then(Value::as_str)
                .unwrap_or("point_value"),
        )
        .map_err(err),
        "normals" => surface_normals(dataset).map_err(err),
        "feature_edges" => {
            feature_edges(dataset, f64_field(op, "angle").unwrap_or(30.0)).map_err(err)
        }
        "decimate" => decimate(dataset, f64_field(op, "cell_size")?).map_err(err),
        "smooth" => smooth(
            dataset,
            f64_field(op, "lambda").unwrap_or(0.5),
            usize_field(op, "iterations").unwrap_or(1),
        )
        .map_err(err),
        "clip_sphere" => {
            let mode = match op.get("mode").and_then(Value::as_str).unwrap_or("all") {
                "all" => ThresholdMode::All,
                "any" => ThresholdMode::Any,
                other => return Err(format!("unknown clip_sphere mode '{other}'")),
            };
            clip_sphere(
                dataset,
                vec3(op, "centre")?,
                f64_field(op, "radius")?,
                op.get("keep_inside")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
                mode,
            )
            .map_err(err)
        }
        "calculator" => calculator(
            dataset,
            str_field(op, "expression")?,
            op.get("output").and_then(Value::as_str).unwrap_or("result"),
        )
        .map_err(err),
        "resample" => {
            let dims: [usize; 3] =
                serde_json::from_value(op.get("dims").cloned().unwrap_or(Value::Null))
                    .map_err(|e| format!("invalid 'dims': {e}"))?;
            resample_to_grid(
                dataset,
                dims,
                vec3(op, "origin").unwrap_or([0.0; 3]),
                vec3(op, "spacing").unwrap_or([1.0; 3]),
                f64_field(op, "fill").unwrap_or(0.0),
            )
            .map_err(err)
        }
        "hyperstreamlines" => {
            let seeds: Vec<[f64; 3]> =
                serde_json::from_value(op.get("seeds").cloned().unwrap_or(Value::Null))
                    .map_err(|e| format!("invalid 'seeds': {e}"))?;
            hyperstreamlines(
                dataset,
                str_field(op, "field")?,
                &seeds,
                f64_field(op, "step").unwrap_or(0.05),
                usize_field(op, "max_steps").unwrap_or(500),
            )
            .map_err(err)
        }
        "delaunay" => delaunay_2d(&dataset.positions).map_err(err),
        "convex_hull" => convex_hull(&dataset.positions).map_err(err),
        "transform" => {
            let matrix: [[f64; 4]; 3] =
                serde_json::from_value(op.get("matrix").cloned().unwrap_or(Value::Null))
                    .map_err(|e| format!("invalid 'matrix': {e}"))?;
            transform_dataset(dataset, matrix).map_err(err)
        }
        "append" => {
            let other = dataset_from_spec(op.get("dataset").unwrap_or(&Value::Null))?;
            append_datasets(dataset, &other).map_err(err)
        }
        "magnitude" => {
            let out = op
                .get("output")
                .and_then(Value::as_str)
                .unwrap_or("magnitude");
            vector_magnitude(dataset, str_field(op, "field")?, out).map_err(err)
        }
        other => Err(format!("unknown mesh op '{other}'")),
    }
}

fn apply_ops(mut dataset: DataSet, ops: &[Value]) -> Result<DataSet, String> {
    let mut cursor = 0;
    while cursor < ops.len() {
        let mut end = cursor;
        while end < ops.len() && ops[end].get("op").and_then(Value::as_str) == Some("cell_to_point")
        {
            end += 1;
        }
        if end - cursor >= 2 {
            dataset = apply_cell_run(&dataset, &ops[cursor..end], CellToPointLimits::default())?;
            cursor = end;
        } else {
            dataset = apply_op(&dataset, &ops[cursor])?;
            cursor += 1;
        }
    }
    Ok(dataset)
}

fn apply_cell_run(
    dataset: &DataSet,
    ops: &[Value],
    limits: CellToPointLimits,
) -> Result<DataSet, String> {
    // Cell-to-point changes only point fields. All cell selections therefore
    // refer to the same snapshot, including when an output replaces a name.
    let selections = ops
        .iter()
        .map(|op| {
            Some((
                op.get("field")?.as_str()?,
                op.get("output")
                    .and_then(Value::as_str)
                    .unwrap_or("point_value"),
            ))
        })
        .collect::<Option<Vec<_>>>();
    if let Some(selections) = selections {
        let source = dataset.validated().map_err(|error| error.to_string())?;
        match CellToPointPlan::with_limits(source, limits) {
            Ok(mut plan) => {
                return plan
                    .convert_fields(&selections)
                    .map_err(|error| error.to_string());
            }
            Err(TransformError::ResourceLimit { .. }) => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    // Preserve sequential error precedence for malformed syntax, and keep
    // previously valid pipelines working when reusable weights exceed budget.
    let mut output = apply_op(dataset, &ops[0])?;
    for op in &ops[1..] {
        output = apply_op(&output, op)?;
    }
    Ok(output)
}

/// Runs the full `{"dataset": ..., "ops": [...]}` request and returns a JSON
/// summary of the final dataset.
///
/// # Errors
///
/// Returns a message for malformed JSON or any failing op.
pub fn mesh_filters(request_json: &str) -> Result<String, String> {
    let request: Value = serde_json::from_str(request_json).map_err(|e| e.to_string())?;
    let mut dataset = dataset_from_spec(request.get("dataset").unwrap_or(&Value::Null))?;
    if let Some(ops) = request.get("ops").and_then(Value::as_array) {
        dataset = apply_ops(dataset, ops)?;
    }
    Ok(serde_json::to_string(&summarize(&dataset)).unwrap_or_default())
}

/// Validates a dataset spec without running filters. Returns an empty
/// string when valid, else the problem description.
#[must_use]
pub fn mesh_problem(spec_json: &str) -> String {
    let parsed: Result<Value, _> = serde_json::from_str(spec_json);
    match parsed {
        Ok(value) => match dataset_from_spec(value.get("dataset").unwrap_or(&value)) {
            Ok(_) => String::new(),
            Err(message) => message,
        },
        Err(e) => e.to_string(),
    }
}

/// Integrates streamlines and returns `{"lines": [[[x, y, z], ...], ...]}`.
///
/// # Errors
///
/// Returns a message for malformed JSON or integration failures.
pub fn mesh_streamlines(request_json: &str) -> Result<String, String> {
    let request: Value = serde_json::from_str(request_json).map_err(|e| e.to_string())?;
    let dataset = dataset_from_spec(request.get("dataset").unwrap_or(&Value::Null))?;
    let field = request
        .get("velocity_field")
        .and_then(Value::as_str)
        .unwrap_or("velocity");
    let seeds: Vec<[f64; 3]> =
        serde_json::from_value(request.get("seeds").cloned().unwrap_or(Value::Null))
            .map_err(|e| format!("invalid 'seeds': {e}"))?;
    let step = request.get("step").and_then(Value::as_f64).unwrap_or(0.05);
    let max_steps = request
        .get("max_steps")
        .and_then(Value::as_u64)
        .unwrap_or(500) as usize;
    let lines = streamlines(&dataset, field, &seeds, step, max_steps).map_err(|e| e.to_string())?;
    // The filter emits one line cell per integrated segment; stitch cells
    // that share an endpoint back into one polyline per seed.
    let mut polylines: Vec<Vec<[f64; 3]>> = Vec::new();
    for cell in 0..lines.cell_count() {
        let nodes = lines.cells.cell(cell).unwrap_or(&[]);
        let segment: Vec<[f64; 3]> = nodes.iter().map(|&n| lines.positions[n as usize]).collect();
        match polylines.last_mut() {
            Some(current)
                if !current.is_empty()
                    && !segment.is_empty()
                    && current[current.len() - 1] == segment[0] =>
            {
                current.extend_from_slice(&segment[1..]);
            }
            _ => polylines.push(segment),
        }
    }
    Ok(serde_json::to_string(&json!({ "lines": polylines })).unwrap_or_default())
}

fn usize_field(op: &Value, key: &str) -> Option<usize> {
    op.get(key).and_then(Value::as_u64).map(|v| v as usize)
}

/// Runs the full request and returns the final dataset's full geometry:
/// points, typed cells, convenience triangle/line index lists, and fields.
///
/// # Errors
///
/// Returns a message for malformed JSON or any failing op.
pub fn mesh_geometry(request_json: &str) -> Result<String, String> {
    let request: Value = serde_json::from_str(request_json).map_err(|e| e.to_string())?;
    let mut dataset = dataset_from_spec(request.get("dataset").unwrap_or(&Value::Null))?;
    if let Some(ops) = request.get("ops").and_then(Value::as_array) {
        dataset = apply_ops(dataset, ops)?;
    }
    Ok(serde_json::to_string(&dataset_geometry(&dataset)).unwrap_or_default())
}

/// Advects particles and returns one geometry object per frame.
///
/// The request is `{"series": [dataset, ...], "times": [...], ...}` plus
/// the scalar options of [`particle_frames`]; series datasets are built
/// with the same spec format as `dataset`.
///
/// # Errors
///
/// Returns a message for malformed JSON or advection failures.
pub fn mesh_particles(request_json: &str) -> Result<String, String> {
    let request: Value = serde_json::from_str(request_json).map_err(|e| e.to_string())?;
    let specs = request
        .get("series")
        .and_then(Value::as_array)
        .ok_or("request is missing 'series'")?;
    let mut steps = Vec::with_capacity(specs.len());
    for spec in specs {
        steps.push(dataset_from_spec(spec)?);
    }
    let times: Vec<f64> =
        serde_json::from_value(request.get("times").cloned().unwrap_or(Value::Null))
            .map_err(|e| format!("invalid 'times': {e}"))?;
    let series = gpui_mesh::TimeSeries::new(steps, times).map_err(|e| e.to_string())?;
    let field = request
        .get("velocity_field")
        .and_then(Value::as_str)
        .unwrap_or("velocity");
    let seeds: Vec<[f64; 3]> =
        serde_json::from_value(request.get("seeds").cloned().unwrap_or(Value::Null))
            .map_err(|e| format!("invalid 'seeds': {e}"))?;
    let options = ParticleOptions {
        dt: request
            .get("dt")
            .and_then(Value::as_f64)
            .unwrap_or(1.0 / 24.0),
        frames: usize_field(&request, "frames").unwrap_or(120),
        trail: usize_field(&request, "trail").unwrap_or(24),
    };
    let frames = particle_frames(&series, field, &seeds, options).map_err(|e| e.to_string())?;
    let geometries: Vec<Value> = frames.iter().map(dataset_geometry).collect();
    Ok(serde_json::to_string(&json!({ "frames": geometries })).unwrap_or_default())
}

/// Serialises a dataset: points, typed cells, triangle/line index lists,
/// and point fields (scalar, vector, tensor values as nested arrays).
fn dataset_geometry(dataset: &DataSet) -> Value {
    let mut triangles: Vec<u32> = Vec::new();
    let mut lines: Vec<u32> = Vec::new();
    let mut cells: Vec<Value> = Vec::new();
    for cell in 0..dataset.cell_count() {
        let kind = dataset.cells.types[cell];
        let nodes = dataset.cells.cell(cell).unwrap_or(&[]);
        let name = match kind {
            gpui_mesh::CellType::Vertex => "vertex",
            gpui_mesh::CellType::Line => "line",
            gpui_mesh::CellType::Triangle => "triangle",
            gpui_mesh::CellType::Quad => "quad",
            gpui_mesh::CellType::Tetra => "tetra",
            gpui_mesh::CellType::Hexahedron => "hexahedron",
            gpui_mesh::CellType::Wedge => "wedge",
            gpui_mesh::CellType::Pyramid => "pyramid",
            gpui_mesh::CellType::Polygon => "polygon",
        };
        cells.push(json!({ "type": name, "nodes": nodes }));
        match kind {
            gpui_mesh::CellType::Triangle => triangles.extend_from_slice(nodes),
            gpui_mesh::CellType::Line => lines.extend_from_slice(nodes),
            _ => {}
        }
    }
    let mut fields = serde_json::Map::new();
    for field in dataset.point_data.iter().chain(dataset.cell_data.iter()) {
        let value = match &field.data {
            gpui_mesh::FieldData::Scalar(values) => {
                json!({ "kind": "scalar", "values": values.as_ref() })
            }
            gpui_mesh::FieldData::Vector(values) => {
                json!({ "kind": "vector", "values": values.as_ref() })
            }
            gpui_mesh::FieldData::Tensor(values) => {
                json!({ "kind": "tensor", "values": values.as_ref() })
            }
            gpui_mesh::FieldData::Complex(values) => {
                json!({ "kind": "complex", "values": values.as_ref() })
            }
        };
        fields.insert(field.name.to_string(), value);
    }
    json!({
        "points": dataset.positions.as_ref(),
        "cells": cells,
        "triangles": triangles,
        "lines": lines,
        "fields": Value::Object(fields),
    })
}

/// Field statistics (and optional histogram) for one field.
///
/// # Errors
///
/// Returns a message for malformed JSON or a missing field.
pub fn mesh_statistics(request_json: &str) -> Result<String, String> {
    let request: Value = serde_json::from_str(request_json).map_err(|e| e.to_string())?;
    let dataset = dataset_from_spec(request.get("dataset").unwrap_or(&Value::Null))?;
    let field = request
        .get("field")
        .and_then(Value::as_str)
        .ok_or("request is missing 'field'")?;
    let association = match request.get("association").and_then(Value::as_str) {
        Some("cell") => Association::Cell,
        _ => Association::Point,
    };
    let stats = field_statistics(&dataset, field, association).map_err(|e| e.to_string())?;
    let mut out = json!({
        "count": stats.count,
        "min": stats.min,
        "max": stats.max,
        "mean": stats.mean,
        "std_dev": stats.std_dev,
        "median": stats.median,
    });
    if let Some(bins) = request.get("bins").and_then(Value::as_u64) {
        let hist = histogram(&dataset, field, bins as usize).map_err(|e| e.to_string())?;
        out["histogram"] = json!(
            hist.iter()
                .map(|(centre, count)| json!({"centre": centre, "count": count}))
                .collect::<Vec<_>>()
        );
    }
    Ok(serde_json::to_string(&out).unwrap_or_default())
}

/// FTLE field of a time series at `t0` over `duration`, as a summary.
///
/// # Errors
///
/// Returns a message for malformed JSON or computation failures.
pub fn mesh_ftle(request_json: &str) -> Result<String, String> {
    let request: Value = serde_json::from_str(request_json).map_err(|e| e.to_string())?;
    let specs = request
        .get("series")
        .and_then(Value::as_array)
        .ok_or("request is missing 'series'")?;
    let mut steps = Vec::with_capacity(specs.len());
    for spec in specs {
        steps.push(dataset_from_spec(spec)?);
    }
    let times: Vec<f64> =
        serde_json::from_value(request.get("times").cloned().unwrap_or(Value::Null))
            .map_err(|e| format!("invalid 'times': {e}"))?;
    let series = gpui_mesh::TimeSeries::new(steps, times).map_err(|e| e.to_string())?;
    let field = request
        .get("velocity_field")
        .and_then(Value::as_str)
        .unwrap_or("velocity");
    let out = gpui_dataset_transformation::ftle(
        &series,
        field,
        request.get("t0").and_then(Value::as_f64).unwrap_or(0.0),
        request
            .get("duration")
            .and_then(Value::as_f64)
            .unwrap_or(1.0),
        request
            .get("epsilon")
            .and_then(Value::as_f64)
            .unwrap_or(0.05),
        usize_field(&request, "steps").unwrap_or(16),
    )
    .map_err(|e| e.to_string())?;
    Ok(serde_json::to_string(&summarize(&out)).unwrap_or_default())
}

fn summarize(dataset: &DataSet) -> Value {
    let fields: Vec<Value> = dataset
        .point_data
        .iter()
        .chain(dataset.cell_data.iter())
        .map(|field| {
            let (kind, min, max) = match &field.data {
                gpui_mesh::FieldData::Scalar(values) => (
                    "scalar",
                    values.iter().copied().fold(f64::INFINITY, f64::min),
                    values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                ),
                gpui_mesh::FieldData::Vector(values) => {
                    let magnitudes: Vec<f64> = values
                        .iter()
                        .map(|v| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt())
                        .collect();
                    (
                        "vector",
                        magnitudes.iter().copied().fold(f64::INFINITY, f64::min),
                        magnitudes.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                    )
                }
                gpui_mesh::FieldData::Tensor(_) => ("tensor", 0.0, 0.0),
                gpui_mesh::FieldData::Complex(_) => ("complex", 0.0, 0.0),
            };
            json!({
                "name": field.name.as_ref(),
                "association": if field.association == Association::Point { "point" } else { "cell" },
                "kind": kind,
                "min": min,
                "max": max,
            })
        })
        .collect();
    let bounds = dataset.bounds();
    json!({
        "points": dataset.point_count(),
        "cells": dataset.cell_count(),
        "bounds": bounds.map(|b| [b.min, b.max]),
        "fields": fields,
    })
}

#[derive(Deserialize)]
struct GridSpec {
    dims: [usize; 3],
    #[serde(default)]
    origin: [f64; 3],
    #[serde(default = "unit_spacing")]
    spacing: [f64; 3],
}

fn unit_spacing() -> [f64; 3] {
    [1.0, 1.0, 1.0]
}

#[derive(Deserialize)]
struct FieldSpec {
    name: String,
    kind: String,
    #[serde(default)]
    values: Vec<f64>,
    vectors: Option<Vec<[f64; 3]>>,
    tensors: Option<Vec<[f64; 6]>>,
}

fn vec3(op: &Value, key: &str) -> Result<[f64; 3], String> {
    serde_json::from_value(op.get(key).cloned().unwrap_or(Value::Null))
        .map_err(|e| format!("invalid '{key}': {e}"))
}

fn str_field<'a>(op: &'a Value, key: &str) -> Result<&'a str, String> {
    op.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("op is missing '{key}'"))
}

fn f64_field(op: &Value, key: &str) -> Result<f64, String> {
    op.get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("op is missing numeric '{key}'"))
}

/// Reads a mesh file sent as base64 and returns its full geometry JSON
/// (the same shape as [`mesh_geometry`]).
///
/// The request is `{"format": "vtu"|"stl"|"obj"|"ply",
/// "data_b64": "..."}`. XDMF is a multi-file bundle and stays
/// Rust-side.
///
/// # Errors
///
/// Returns a message for malformed JSON, bad base64, an unknown format,
/// or unparseable file contents.
pub fn mesh_read(request_json: &str) -> Result<String, String> {
    let request: Value = serde_json::from_str(request_json).map_err(|e| e.to_string())?;
    let format = request
        .get("format")
        .and_then(Value::as_str)
        .ok_or("request is missing 'format'")?;
    let encoded = request
        .get("data_b64")
        .and_then(Value::as_str)
        .ok_or("request is missing 'data_b64'")?;
    let bytes = gpui_dataset_transformation::decode_base64(encoded)?;
    let dataset = read_dataset_bytes(&bytes, format)?;
    Ok(serde_json::to_string(&dataset_geometry(&dataset)).unwrap_or_default())
}

/// Serialises the request's dataset spec in `format` and returns
/// `{"data_b64": "..."}` with the file bytes.
///
/// Formats: "vtu", "vtu-zlib", "obj", "ply", "stl".
///
/// # Errors
///
/// Returns a message for malformed JSON, an invalid spec, an unknown
/// format, or a serialisation failure.
pub fn mesh_write(request_json: &str) -> Result<String, String> {
    let request: Value = serde_json::from_str(request_json).map_err(|e| e.to_string())?;
    let format = request
        .get("format")
        .and_then(Value::as_str)
        .ok_or("request is missing 'format'")?;
    let dataset = dataset_from_spec(request.get("dataset").unwrap_or(&Value::Null))?;
    let bytes: Vec<u8> = match format {
        "vtu" => gpui_dataset_transformation::write_vtu(&dataset).into_bytes(),
        "vtu-zlib" => gpui_dataset_transformation::write_vtu_compressed(&dataset).into_bytes(),
        "obj" => gpui_dataset_transformation::write_obj(&dataset).into_bytes(),
        "ply" => gpui_dataset_transformation::write_ply(&dataset).into_bytes(),
        "stl" => gpui_dataset_transformation::write_stl(&dataset)?,
        other => return Err(format!("unsupported mesh format '{other}'")),
    };
    Ok(serde_json::to_string(
        &json!({ "data_b64": gpui_dataset_transformation::encode_base64(&bytes) }),
    )
    .unwrap_or_default())
}

/// Loads a PVD collection: `{"pvd": "<xml>", "files": {"name": "<b64>"}}`
/// and returns `{"times": [...], "steps": [geometry, ...]}` with parallel
/// parts merged per timestep.
///
/// # Errors
///
/// Returns a message for malformed JSON, unresolvable files, or parse
/// failures.
pub fn mesh_read_pvd(request_json: &str) -> Result<String, String> {
    let request: Value = serde_json::from_str(request_json).map_err(|e| e.to_string())?;
    let xml = request
        .get("pvd")
        .and_then(Value::as_str)
        .ok_or("request is missing 'pvd'")?;
    let files = request.get("files").cloned().unwrap_or(Value::Null);
    let series = gpui_dataset_transformation::read_pvd(xml, |name| {
        files
            .get(name)
            .and_then(Value::as_str)
            .and_then(|encoded| gpui_dataset_transformation::decode_base64(encoded).ok())
    })?;
    let steps: Vec<Value> = series.steps.iter().map(dataset_geometry).collect();
    Ok(
        serde_json::to_string(&json!({ "times": series.times, "steps": steps }))
            .unwrap_or_default(),
    )
}

fn read_dataset_bytes(bytes: &[u8], format: &str) -> Result<DataSet, String> {
    match format {
        "vtu" => gpui_dataset_transformation::read_vtu_bytes(bytes),
        "stl" => gpui_dataset_transformation::read_stl(bytes),
        "obj" => gpui_dataset_transformation::read_obj(
            std::str::from_utf8(bytes).map_err(|_| "OBJ document is not UTF-8".to_string())?,
        ),
        "ply" => gpui_dataset_transformation::read_ply(
            std::str::from_utf8(bytes).map_err(|_| "PLY document is not UTF-8".to_string())?,
        ),
        other => Err(format!("unsupported mesh format '{other}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rotation_request() -> String {
        // 5x5x5 solid-body rotation v = (-y, x, 0): vorticity is (0, 0, 2).
        let mut scalars = Vec::new();
        let mut vectors = Vec::new();
        for k in 0..5u32 {
            for j in 0..5u32 {
                for i in 0..5u32 {
                    let (x, y) = (f64::from(i) * 0.25, f64::from(j) * 0.25);
                    scalars.push(x + y + f64::from(k) * 0.25);
                    vectors.push([-y, x, 0.0]);
                }
            }
        }
        json!({
            "dataset": {
                "id": "rotation",
                "structured": {"dims": [5, 5, 5], "origin": [0, 0, 0], "spacing": [0.25, 0.25, 0.25]},
                "fields": [
                    {"name": "s", "kind": "scalar", "values": scalars},
                    {"name": "velocity", "kind": "vector", "vectors": vectors},
                ],
            },
            "ops": [],
        })
        .to_string()
    }

    #[test]
    fn builds_structured_dataset_and_summarizes() {
        let summary = mesh_filters(&rotation_request()).unwrap();
        let value: Value = serde_json::from_str(&summary).unwrap();
        assert_eq!(value["points"], 125);
        assert_eq!(value["cells"], 64);
        assert!(mesh_problem(&rotation_request()).is_empty());
    }

    #[test]
    fn vorticity_of_solid_rotation_is_two() {
        let mut request: Value = serde_json::from_str(&rotation_request()).unwrap();
        request["ops"] = json!([{"op": "vorticity_magnitude", "field": "velocity"}]);
        let summary = mesh_filters(&request.to_string()).unwrap();
        let value: Value = serde_json::from_str(&summary).unwrap();
        let field = value["fields"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["name"] == "vorticity_magnitude")
            .unwrap();
        assert!((field["min"].as_f64().unwrap() - 2.0).abs() < 1e-9);
        assert!((field["max"].as_f64().unwrap() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn slice_reduces_to_surface_and_streamlines_run() {
        let mut request: Value = serde_json::from_str(&rotation_request()).unwrap();
        request["ops"] = json!([{"op": "slice", "normal": [0, 0, 1], "point": [0, 0, 0.5]}]);
        let summary = mesh_filters(&request.to_string()).unwrap();
        let value: Value = serde_json::from_str(&summary).unwrap();
        assert_eq!(value["cells"], 32);

        let stream_request = json!({
            "dataset": request["dataset"],
            "velocity_field": "velocity",
            "seeds": [[0.5, 0.1, 0.5]],
            "step": 0.05,
            "max_steps": 40,
        });
        let out = mesh_streamlines(&stream_request.to_string()).unwrap();
        let lines: Value = serde_json::from_str(&out).unwrap();
        // One seed traces both directions, yielding two polylines.
        assert_eq!(lines["lines"].as_array().unwrap().len(), 2);
        assert!(lines["lines"][0].as_array().unwrap().len() > 10);
    }

    #[test]
    fn geometry_ops_return_triangles_and_particles_advance() {
        let mut request: Value = serde_json::from_str(&rotation_request()).unwrap();
        request["ops"] = json!([
            {"op": "slice", "normal": [0, 0, 1], "point": [0, 0, 0.5]},
            {"op": "solid_arrows", "field": "velocity", "scale": 0.4, "stride": 4, "radius_fraction": 0.05},
        ]);
        let out = mesh_geometry(&request.to_string()).unwrap();
        let geometry: Value = serde_json::from_str(&out).unwrap();
        // Closed solid arrows have 34 triangles (102 indices), including caps.
        let indices = geometry["triangles"].as_array().unwrap().len();
        assert!(
            indices > 0 && indices.is_multiple_of(102),
            "indices {indices}"
        );
        assert!(geometry["fields"]["magnitude"].is_object());

        let mut request: Value = serde_json::from_str(&rotation_request()).unwrap();
        request["ops"] = json!([
            {"op": "streamlines", "field": "velocity", "seeds": [[0.5, 0.1, 0.5]], "step": 0.05, "max_steps": 30},
            {"op": "streamtubes", "radius": 0.02, "sides": 4},
        ]);
        let out = mesh_geometry(&request.to_string()).unwrap();
        let geometry: Value = serde_json::from_str(&out).unwrap();
        assert!(!geometry["triangles"].as_array().unwrap().is_empty());

        let series_request = json!({
            "series": [request["dataset"], request["dataset"]],
            "times": [0.0, 10.0],
            "velocity_field": "velocity",
            "seeds": [[0.5, 0.1, 0.5]],
            "dt": 0.1,
            "frames": 3,
            "trail": 5,
        });
        let out = mesh_particles(&series_request.to_string()).unwrap();
        let frames: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(frames["frames"].as_array().unwrap().len(), 3);
        let last = &frames["frames"][2];
        assert!(last["fields"]["age"].is_object());
        assert!(!last["lines"].as_array().unwrap().is_empty());
    }

    #[test]
    fn mesh_io_round_trips_obj_and_pvd() {
        // Write a structured spec as OBJ through the bridge, read it back.
        let spec = json!({
            "id": "g",
            "structured": {"dims": [2, 2, 2], "origin": [0.0, 0.0, 0.0], "spacing": [1.0, 1.0, 1.0]},
            "fields": []
        });
        let written = mesh_write(&json!({"dataset": spec, "format": "obj"}).to_string()).unwrap();
        let data_b64 = serde_json::from_str::<Value>(&written).unwrap()["data_b64"]
            .as_str()
            .unwrap()
            .to_string();
        let back = mesh_read(&json!({"format": "obj", "data_b64": data_b64}).to_string()).unwrap();
        let geometry: Value = serde_json::from_str(&back).unwrap();
        assert_eq!(geometry["points"].as_array().unwrap().len(), 8);

        // Unknown formats are hard errors on both paths.
        mesh_read(&json!({"format": "nope", "data_b64": ""}).to_string()).unwrap_err();
        mesh_write(&json!({"dataset": spec, "format": "nope"}).to_string()).unwrap_err();

        // PVD: one VTU part, one step.
        let vtu_b64 = serde_json::from_str::<Value>(
            &mesh_write(&json!({"dataset": spec, "format": "vtu"}).to_string()).unwrap(),
        )
        .unwrap()["data_b64"]
            .as_str()
            .unwrap()
            .to_string();
        let pvd = "<VTKFile type=\"Collection\"><Collection>                   <DataSet timestep=\"2.5\" part=\"0\" file=\"a.vtu\"/>                   </Collection></VTKFile>";
        let series =
            mesh_read_pvd(&json!({"pvd": pvd, "files": {"a.vtu": vtu_b64}}).to_string()).unwrap();
        let series: Value = serde_json::from_str(&series).unwrap();
        assert_eq!(series["times"], json!([2.5]));
        assert_eq!(series["steps"].as_array().unwrap().len(), 1);
    }

    fn sequential_ops(mut data: DataSet, ops: &[Value]) -> Result<DataSet, String> {
        for op in ops {
            data = apply_op(&data, op)?;
        }
        Ok(data)
    }

    fn unequal_lines() -> DataSet {
        use gpui_mesh::{CellType, Cells};
        let mut vector = Field::vector("v", vec![[3.0; 3], [9.0; 3]]).on_cells();
        vector.valid = Some(vec![true, false].into());
        DataSet::new(
            "lines",
            vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 2.0, 0.0]],
            Cells::from_cells(&[(CellType::Line, &[0, 1]), (CellType::Line, &[0, 2])]),
        )
        .with_field(Field::scalar("s", vec![2.0, 8.0]).on_cells())
        .with_field(vector)
    }

    #[test]
    fn grouped_cell_conversions_match_aliases_defaults_masks_and_budget_fallback() {
        let data = unequal_lines();
        let ops = json!([
            {"op": "cell_to_point", "field": "s", "output": "alias"},
            {"op": "cell_to_point", "field": "v"},
            {"op": "cell_to_point", "field": "s", "output": "point_value"},
            {"op": "cell_to_point", "field": "v", "output": ""},
        ]);
        let ops = ops.as_array().unwrap();
        let expected = sequential_ops(data.clone(), ops).unwrap();
        let grouped = apply_ops(data.clone(), ops).unwrap();
        assert_eq!(dataset_geometry(&grouped), dataset_geometry(&expected));
        let limits = CellToPointLimits {
            max_geometry_bytes: 0,
            ..Default::default()
        };
        assert!(matches!(
            CellToPointPlan::with_limits(data.validated().unwrap(), limits),
            Err(TransformError::ResourceLimit { .. })
        ));
        let fallback = apply_cell_run(&data, ops, limits).unwrap();
        assert_eq!(dataset_geometry(&fallback), dataset_geometry(&expected));
    }

    #[test]
    fn grouped_conversions_rebuild_after_geometry_changes_and_keep_error_order() {
        let data = unequal_lines();
        let ops = json!([
            {"op": "cell_to_point", "field": "s", "output": "before"},
            {"op": "cell_to_point", "field": "v", "output": "vector"},
            {"op": "transform", "matrix": [[4, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0]]},
            {"op": "cell_to_point", "field": "s", "output": "after"},
            {"op": "cell_to_point", "field": "v", "output": "vector"},
        ]);
        let ops = ops.as_array().unwrap();
        let output = apply_ops(data.clone(), ops).unwrap();
        assert_eq!(
            dataset_geometry(&output),
            dataset_geometry(&sequential_ops(data.clone(), ops).unwrap())
        );
        assert_eq!(
            output
                .field(Association::Point, "before")
                .unwrap()
                .as_scalar()
                .unwrap()[0],
            6.0
        );
        assert_eq!(
            output
                .field(Association::Point, "after")
                .unwrap()
                .as_scalar()
                .unwrap()[0],
            4.0
        );
        for ops in [
            json!([{"op":"cell_to_point", "field":"absent"}, {"op":"cell_to_point"}]),
            json!([{"op":"cell_to_point", "field":"s"}, {"op":"cell_to_point", "field":123}]),
            json!([{"op":"cell_to_point", "field":"s"}, {"op":"cell_to_point", "field":"absent"}]),
            json!([{"op":"cell_to_point", "field":"s"}, {"op":"unknown"}, {"op":"cell_to_point"}]),
        ] {
            let ops = ops.as_array().unwrap();
            assert_eq!(
                apply_ops(data.clone(), ops).unwrap_err(),
                sequential_ops(data.clone(), ops).unwrap_err()
            );
        }
    }

    #[test]
    fn both_request_entry_points_use_grouped_pipeline() {
        let mut request: Value = serde_json::from_str(&rotation_request()).unwrap();
        request["ops"] = json!([
            {"op":"point_to_cell", "field":"s", "output":"s"},
            {"op":"point_to_cell", "field":"velocity", "output":"v"},
            {"op":"cell_to_point", "field":"s", "output":"pressure"},
            {"op":"cell_to_point", "field":"v", "output":"velocity"},
            {"op":"cell_to_point", "field":"s", "output":"pressure"},
        ]);
        let data = dataset_from_spec(&request["dataset"]).unwrap();
        let expected = sequential_ops(data, request["ops"].as_array().unwrap()).unwrap();
        let wire = request.to_string();
        assert_eq!(
            serde_json::from_str::<Value>(&mesh_filters(&wire).unwrap()).unwrap(),
            summarize(&expected)
        );
        assert_eq!(
            serde_json::from_str::<Value>(&mesh_geometry(&wire).unwrap()).unwrap(),
            dataset_geometry(&expected)
        );
    }

    #[test]
    fn unknown_op_and_bad_field_are_errors() {
        let mut request: Value = serde_json::from_str(&rotation_request()).unwrap();
        request["ops"] = json!([{"op": "explode"}]);
        assert!(
            mesh_filters(&request.to_string())
                .unwrap_err()
                .contains("explode")
        );
        request["ops"] = json!([{"op": "threshold", "field": "nope", "min": 0.0, "max": 1.0}]);
        assert!(
            mesh_filters(&request.to_string())
                .unwrap_err()
                .contains("nope")
        );
        assert!(!mesh_problem("{\"dataset\": {\"structured\": {\"dims\": [0, 2, 2]}}}").is_empty());
    }
}
