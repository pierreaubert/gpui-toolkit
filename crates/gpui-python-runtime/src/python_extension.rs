//! Private, dependency-light native functions installed in the Python wheel.

use arrow_array::{
    ArrayRef, BooleanArray, Float64Array, Int64Array, NullArray, RecordBatch, StringArray,
};
use arrow_ipc::writer::StreamWriter;
use d3rs::axis::{
    AxisConfig as D3AxisConfig, AxisLayout as D3AxisLayout, AxisLayoutError as D3AxisLayoutError,
    AxisOrientation as D3AxisOrientation,
};
use d3rs::brush::{BrushSelection as D3BrushSelection, BrushState as D3BrushState};
use d3rs::chord::{
    Chord as NativeChord, ChordLayout as NativeChordLayout, ChordSubgroup as NativeChordSubgroup,
    RibbonGenerator as NativeRibbonGenerator,
};
use d3rs::color::{
    ColorScheme, D3Color, DivergingScale, DivergingScheme, Hcl, Lab, SequentialScale,
    SequentialScheme,
};
use d3rs::contour::{
    Contour, ContourBand, ContourGenerator, ContourRing, DensityEstimator, KernelType,
    epanechnikov_kernel, gaussian_kernel, threshold_freedman_diaconis, threshold_scott,
    threshold_sturges as contour_thresholds_sturges, try_density_2d,
};
use d3rs::delaunay::Delaunay;
use d3rs::drag::{
    DragConfig as D3DragConfig, DragError as D3DragError, DragExtent as D3DragExtent,
    DragPhase as D3DragPhase, DragState as D3DragState, DragUpdate as D3DragUpdate,
};
use d3rs::fetch::{
    AutoTyped as D3AutoTyped, ColumnPolicy as D3ColumnPolicy, DsvBudget as D3DsvBudget,
    DsvBudgetResource as D3DsvBudgetResource, DsvParseError as D3DsvParseError,
    DsvParseErrorKind as D3DsvParseErrorKind, DsvParser as D3DsvParser, DsvRow as D3DsvRow,
    auto_type as d3_auto_type,
};
use d3rs::force::{
    ForceCenter, ForceCollide, ForceLink, ForceManyBody, ForceRadial, ForceX, ForceY, Simulation,
    SimulationNode,
};
use d3rs::format::{Align, FormatType, Locale, Sign};
use d3rs::geo::versor::Versor as GeoVersor;
use d3rs::geo::{
    Albers as GeoAlbers, ConicEqualArea as GeoConicEqualArea,
    Equirectangular as GeoEquirectangular, GeoJsonGeometry as D3GeoJsonGeometry,
    GeoPath as D3GeoPath, Graticule, Mercator as GeoMercator, Orthographic as GeoOrthographic,
    Projection as GeoProjection, Rotation as GeoRotation, Stereographic as GeoStereographic,
    Stream as D3GeoStream, TopoJsonBudget as D3TopoJsonBudget,
    TransverseMercator as GeoTransverseMercator, geo_area as d3_geo_area,
    geo_bounds as d3_geo_bounds, geo_centroid as d3_geo_centroid, geo_contains as d3_geo_contains,
    geo_distance as d3_geo_distance, geo_interpolate as d3_geo_interpolate,
    geo_length as d3_geo_length, parse_land as d3_parse_land,
    parse_land_with_budget as d3_parse_land_with_budget, stream_geojson as d3_stream_geojson,
};
use d3rs::grid::{
    GridConfig as D3GridConfig, GridLayout as D3GridLayout, GridLayoutError as D3GridLayoutError,
};
use d3rs::hexbin::{Hexbin as D3Hexbin, HexbinError as D3HexbinError};
use d3rs::hierarchy::{
    ClusterLayout, HierarchyNode, PackLayout, PartitionLayout, TreeLayout, TreemapLayout,
};
use d3rs::interpolate::{Cubehelix as InterpolateCubehelix, Hsl as InterpolateHsl};
use d3rs::legend::{
    LegendConfig as D3LegendConfig, LegendItem as D3LegendItem,
    LegendLayoutError as D3LegendLayoutError, LegendOrientation as D3LegendOrientation,
    LegendPosition as D3LegendPosition, LegendSymbol as D3LegendSymbol,
};
use d3rs::lod::{DensityPyramid as D3DensityPyramid, LodBounds as D3LodBounds};
use d3rs::mesh::{
    CoordinateAxis as MeshCoordinateAxis, ScalarAssociation as MeshScalarAssociation,
    ScalarField as NativeMeshScalarField, TriGridIndex as NativeTriGridIndex,
    TriangleMesh as NativeTriangleMesh, project_2d as project_mesh_2d,
};
use d3rs::polygon::{
    polygon_area as d3_polygon_area, polygon_area_signed as d3_polygon_area_signed,
    polygon_centroid as d3_polygon_centroid, polygon_contains as d3_polygon_contains,
    polygon_hull as d3_polygon_hull, polygon_length as d3_polygon_length,
};
use d3rs::quadtree::{
    Aggregate as D3QuadAggregate, QuadNode as D3QuadNode, QuadTree as D3QuadTree,
};
use d3rs::random::{
    LcgRng, RandomBates, RandomBernoulli, RandomExponential, RandomIrwinHall, RandomLogNormal,
    RandomNormal, RandomPoisson, RandomUniform,
};
use d3rs::sankey::{
    SankeyLayout as D3SankeyLayout, SankeyLayoutError as D3SankeyLayoutError,
    SankeyLinkInput as D3SankeyLinkInput, SankeyNodeAlign as D3SankeyNodeAlign,
};
use d3rs::scale::{
    BandScale, LinearScale, LogScale, PointScale, PowScale, QuantileScale, QuantizeScale, Scale,
    SymlogScale, ThresholdScale,
};
use d3rs::shape::{
    Arc as ShapeArc, ArcDatum, Area as ShapeArea, Curve as ShapeCurve, Link as ShapeLink,
    LinkDirection, Path as NativeShapePath, PathBuilder as ShapePathBuilder, PathCommand,
    Pie as ShapePie, Point as ShapePoint, RadialAreaConfig, RadialLineConfig, RadialLink,
    RadialPoint, SimpleArea, Stack as ShapeStack, StackOffset, StackOrder, StackSeries,
    Symbol as ShapeSymbol, SymbolType, arc_points, link_horizontal as shape_link_horizontal,
    link_radial as shape_link_radial, link_step as shape_link_step,
    link_vertical as shape_link_vertical, polar_grid_circles as shape_polar_grid_circles,
    polar_grid_rays as shape_polar_grid_rays, radial_area as shape_radial_area,
    radial_line as shape_radial_line, try_arc_points, try_link_horizontal, try_link_radial,
    try_link_step, try_link_vertical, try_polar_grid_circles, try_polar_grid_rays, try_radial_area,
    try_radial_line,
};
use d3rs::tile::{TileError as D3TileError, TileLayout as D3TileLayout};
use d3rs::time::{Interval, TimeFormat, TimeFormatParts, TimeInterval, TimeScale};
use d3rs::timer::{Interval as D3TimerInterval, Timeout as D3Timeout, Timer as D3Timer};
use d3rs::transition::{Transition as D3Transition, TransitionState as D3TransitionState};
use d3rs::zoom::ZoomState as D3ZoomState;
use gpui_px::interaction::{
    ChartInteraction as PxChartInteraction, ChartKeyboardAction as PxChartKeyboardAction,
    InteractionMode as PxInteractionMode, keyboard_action_for_key as px_keyboard_action_for_key,
};
use gpui_px::mesh_plot::pick_2d as px_mesh_pick_2d;
use gpui_px::{
    AutoOrFixed as PxAutoOrFixed, ColorRange as PxColorRange, ColorScale as PxColorScale,
    TilingMethod as PxTilingMethod, TreemapNode as PxTreemapNode,
    chart_capability_report as px_native_chart_capability_report, treemap as px_treemap,
};
use pyo3::{
    exceptions::{PyRuntimeError, PyValueError},
    prelude::*,
    types::PyBytes,
};
use std::{
    cell::RefCell,
    cmp::Ordering,
    hash::{Hash, Hasher},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering as AtomicOrdering},
    },
};

#[derive(Clone, Copy, Debug)]
struct FiniteF64(f64);

impl PartialEq for FiniteF64 {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for FiniteF64 {}

impl PartialOrd for FiniteF64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FiniteF64 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.partial_cmp(&other.0).unwrap()
    }
}

impl Hash for FiniteF64 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let bits = if self.0 == 0.0 {
            0.0f64.to_bits()
        } else {
            self.0.to_bits()
        };
        bits.hash(state);
    }
}

fn finite_keys(values: Vec<f64>) -> PyResult<Vec<FiniteF64>> {
    finite_values(&values)?;
    Ok(values.into_iter().map(FiniteF64).collect())
}

fn key_values(values: Vec<FiniteF64>) -> Vec<f64> {
    values.into_iter().map(|value| value.0).collect()
}

fn finite(name: &str, value: f64) -> PyResult<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(PyValueError::new_err(format!("{name} must be finite")))
    }
}

fn finite_values(values: &[f64]) -> PyResult<()> {
    if let Some((index, _)) = values
        .iter()
        .enumerate()
        .find(|(_, value)| !value.is_finite())
    {
        Err(PyValueError::new_err(format!(
            "data[{index}] must be finite"
        )))
    } else {
        Ok(())
    }
}

fn sorted_values(values: &[f64]) -> PyResult<()> {
    if let Some((index, _)) = values
        .windows(2)
        .enumerate()
        .find(|(_, pair)| pair[0] > pair[1])
    {
        Err(PyValueError::new_err(format!(
            "data must be sorted in ascending order at data[{}]",
            index + 1
        )))
    } else {
        Ok(())
    }
}

fn color(value: &str) -> PyResult<D3Color> {
    let digits = value.strip_prefix('#').unwrap_or(value);
    if digits.len() != 6 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(PyValueError::new_err("color must be #RRGGBB"));
    }
    u32::from_str_radix(digits, 16)
        .map(D3Color::from_hex)
        .map_err(|_| PyValueError::new_err("color must be #RRGGBB"))
}

fn interpolation_arguments(a: f64, b: f64, t: f64) -> PyResult<()> {
    finite("a", a)?;
    finite("b", b)?;
    finite("t", t)
}

#[pyfunction]
fn interpolate_number(a: f64, b: f64, t: f64) -> PyResult<f64> {
    interpolation_arguments(a, b, t)?;
    Ok(d3rs::interpolate::interpolate(a, b)(t))
}

#[pyfunction]
fn interpolate_round(a: i64, b: i64, t: f64) -> PyResult<i64> {
    finite("t", t)?;
    Ok(d3rs::interpolate::interpolate_round(a, b)(t))
}

#[pyfunction]
fn interpolate_number_array(
    py: Python<'_>,
    a: Vec<f64>,
    b: Vec<f64>,
    t: f64,
) -> PyResult<Vec<f64>> {
    finite_values(&a)?;
    finite_values(&b)?;
    finite("t", t)?;
    if a.len() != b.len() {
        return Err(PyValueError::new_err(
            "interpolation arrays must have equal lengths",
        ));
    }
    Ok(py.allow_threads(move || d3rs::interpolate::interpolate_number_array(a, b)(t)))
}

macro_rules! color_interpolator {
    ($name:ident, $native:ident) => {
        #[pyfunction]
        fn $name(a: &str, b: &str, t: f64) -> PyResult<String> {
            finite("t", t)?;
            Ok(d3rs::interpolate::$native(color(a)?, color(b)?)(t).to_hex())
        }
    };
}

color_interpolator!(interpolate_rgb, interpolate_rgb);
color_interpolator!(interpolate_hsl, interpolate_hsl);
color_interpolator!(interpolate_hsl_long, interpolate_hsl_long);
color_interpolator!(interpolate_lab, interpolate_lab);
color_interpolator!(interpolate_hcl, interpolate_hcl);
color_interpolator!(interpolate_hcl_long, interpolate_hcl_long);
color_interpolator!(interpolate_cubehelix, interpolate_cubehelix);
color_interpolator!(interpolate_cubehelix_long, interpolate_cubehelix_long);

#[pyfunction]
fn color_luminance(value: &str) -> PyResult<f32> {
    Ok(color(value)?.luminance())
}

#[pyfunction]
fn color_lighten(value: &str, amount: f32) -> PyResult<String> {
    if !amount.is_finite() {
        return Err(PyValueError::new_err("amount must be finite"));
    }
    Ok(color(value)?.lighten(amount).to_hex())
}

#[pyfunction]
fn color_darken(value: &str, amount: f32) -> PyResult<String> {
    if !amount.is_finite() {
        return Err(PyValueError::new_err("amount must be finite"));
    }
    Ok(color(value)?.darken(amount).to_hex())
}

type ColorTuple = (f64, f64, f64, f64);
type LabTuple = (f64, f64, f64, f64);
type HclTuple = (f64, f64, f64, f64);
type HslTuple = (f64, f64, f64, f64);
type CubehelixTuple = (f64, f64, f64, f64);

fn color_tuple(color: D3Color) -> ColorTuple {
    (
        color.r as f64,
        color.g as f64,
        color.b as f64,
        color.a as f64,
    )
}

fn color_from_tuple(value: ColorTuple) -> PyResult<D3Color> {
    finite("color.r", value.0)?;
    finite("color.g", value.1)?;
    finite("color.b", value.2)?;
    finite("color.a", value.3)?;
    Ok(D3Color::from_rgba_f32(
        value.0 as f32,
        value.1 as f32,
        value.2 as f32,
        value.3 as f32,
    ))
}

fn interpolate_hsl_from_tuple(value: HslTuple) -> PyResult<InterpolateHsl> {
    finite("hsl.h", value.0)?;
    finite("hsl.s", value.1)?;
    finite("hsl.l", value.2)?;
    finite("hsl.a", value.3)?;
    Ok(InterpolateHsl {
        h: value.0,
        s: value.1,
        l: value.2,
        a: value.3,
    })
}

fn interpolate_hsl_tuple(value: InterpolateHsl) -> HslTuple {
    (value.h, value.s, value.l, value.a)
}

#[pyfunction]
fn interpolate_hsl_value_new(h: f64, s: f64, l: f64) -> PyResult<HslTuple> {
    finite("h", h)?;
    finite("s", s)?;
    finite("l", l)?;
    Ok(interpolate_hsl_tuple(InterpolateHsl::new(h, s, l)))
}

#[pyfunction]
fn interpolate_hsl_value_from_color(color: ColorTuple) -> PyResult<HslTuple> {
    Ok(interpolate_hsl_tuple(InterpolateHsl::from_rgb(
        &color_from_tuple(color)?,
    )))
}

#[pyfunction]
fn interpolate_hsl_value_to_color(value: HslTuple) -> PyResult<ColorTuple> {
    Ok(color_tuple(interpolate_hsl_from_tuple(value)?.to_rgb()))
}

fn interpolate_cubehelix_from_tuple(value: CubehelixTuple) -> PyResult<InterpolateCubehelix> {
    finite("cubehelix.h", value.0)?;
    finite("cubehelix.s", value.1)?;
    finite("cubehelix.l", value.2)?;
    finite("cubehelix.alpha", value.3)?;
    Ok(InterpolateCubehelix {
        h: value.0,
        s: value.1,
        l: value.2,
        alpha: value.3,
    })
}

fn interpolate_cubehelix_tuple(value: InterpolateCubehelix) -> CubehelixTuple {
    (value.h, value.s, value.l, value.alpha)
}

#[pyfunction]
fn interpolate_cubehelix_value_new(h: f64, s: f64, l: f64) -> PyResult<CubehelixTuple> {
    finite("h", h)?;
    finite("s", s)?;
    finite("l", l)?;
    Ok(interpolate_cubehelix_tuple(InterpolateCubehelix::new(
        h, s, l,
    )))
}

#[pyfunction]
fn interpolate_cubehelix_value_from_color(color: ColorTuple) -> PyResult<CubehelixTuple> {
    Ok(interpolate_cubehelix_tuple(InterpolateCubehelix::from_rgb(
        &color_from_tuple(color)?,
    )))
}

#[pyfunction]
fn interpolate_cubehelix_value_to_color(value: CubehelixTuple) -> PyResult<ColorTuple> {
    Ok(color_tuple(
        interpolate_cubehelix_from_tuple(value)?.to_rgb(),
    ))
}

#[pyfunction]
fn interpolate_cubehelix_default(t: f64) -> PyResult<ColorTuple> {
    finite("t", t)?;
    Ok(color_tuple(d3rs::interpolate::cubehelix_default()(t)))
}

#[pyfunction]
fn interpolate_cubehelix_custom(
    start: f64,
    rotations: f64,
    hue: f64,
    gamma: f64,
    t: f64,
) -> PyResult<ColorTuple> {
    for (name, value) in [
        ("start", start),
        ("rotations", rotations),
        ("hue", hue),
        ("gamma", gamma),
        ("t", t),
    ] {
        finite(name, value)?;
    }
    Ok(color_tuple(d3rs::interpolate::cubehelix_custom(
        start, rotations, hue, gamma,
    )(t)))
}

fn lab_tuple(value: Lab) -> LabTuple {
    (value.l, value.a, value.b, value.alpha)
}

fn lab_from_tuple(value: LabTuple) -> PyResult<Lab> {
    finite("lab.l", value.0)?;
    finite("lab.a", value.1)?;
    finite("lab.b", value.2)?;
    finite("lab.alpha", value.3)?;
    Ok(Lab::with_alpha(value.0, value.1, value.2, value.3))
}

fn hcl_tuple(value: Hcl) -> HclTuple {
    (value.h, value.c, value.l, value.alpha)
}

fn hcl_from_tuple(value: HclTuple) -> PyResult<Hcl> {
    finite("hcl.h", value.0)?;
    finite("hcl.c", value.1)?;
    finite("hcl.l", value.2)?;
    finite("hcl.alpha", value.3)?;
    Ok(Hcl::with_alpha(value.0, value.1, value.2, value.3))
}

#[pyfunction]
#[pyo3(signature = (r, g, b, a=None))]
fn d3_color_rgb(r: u8, g: u8, b: u8, a: Option<u8>) -> ColorTuple {
    color_tuple(match a {
        Some(alpha) => D3Color::rgba(r, g, b, alpha),
        None => D3Color::rgb(r, g, b),
    })
}

#[pyfunction]
fn d3_color_from_hex(hex: u32) -> PyResult<ColorTuple> {
    if hex > 0x00ff_ffff {
        return Err(PyValueError::new_err(
            "hex color must be in 0x000000..=0xffffff",
        ));
    }
    Ok(color_tuple(D3Color::from_hex(hex)))
}

#[pyfunction]
#[pyo3(signature = (r, g, b, a=None))]
fn d3_color_from_f32(r: f32, g: f32, b: f32, a: Option<f32>) -> PyResult<ColorTuple> {
    for (name, value) in [("r", r), ("g", g), ("b", b)] {
        if !value.is_finite() {
            return Err(PyValueError::new_err(format!("{name} must be finite")));
        }
    }
    let color = match a {
        Some(alpha) => {
            if !alpha.is_finite() {
                return Err(PyValueError::new_err("a must be finite"));
            }
            D3Color::from_rgba_f32(r, g, b, alpha)
        }
        None => D3Color::from_rgb_f32(r, g, b),
    };
    Ok(color_tuple(color))
}

#[pyfunction]
fn d3_color_from_hsl(h: f32, s: f32, l: f32) -> PyResult<ColorTuple> {
    for (name, value) in [("h", h), ("s", s), ("l", l)] {
        if !value.is_finite() {
            return Err(PyValueError::new_err(format!("{name} must be finite")));
        }
    }
    Ok(color_tuple(D3Color::from_hsl(h, s, l)))
}

#[pyfunction]
fn d3_color_transform(
    operation: &str,
    color: ColorTuple,
    value: f64,
    other: Option<ColorTuple>,
) -> PyResult<ColorTuple> {
    finite("value", value)?;
    let color = color_from_tuple(color)?;
    let result = match operation {
        "with_alpha" => color.with_alpha(value as f32),
        "interpolate" => color.interpolate(
            &color_from_tuple(
                other.ok_or_else(|| PyValueError::new_err("interpolate requires other color"))?,
            )?,
            value as f32,
        ),
        "lighten" => color.lighten(value as f32),
        "darken" => color.darken(value as f32),
        "brighter" => color.brighter(value as f32),
        "darker" => color.darker(value as f32),
        "with_opacity" => color.with_opacity(value as f32),
        _ => {
            return Err(PyValueError::new_err(format!(
                "unsupported D3Color operation {operation:?}"
            )));
        }
    };
    Ok(color_tuple(result))
}

#[pyfunction]
fn d3_color_to_hex(color: ColorTuple, alpha: bool) -> PyResult<String> {
    let color = color_from_tuple(color)?;
    Ok(if alpha {
        color.to_hex_alpha()
    } else {
        color.to_hex()
    })
}

#[pyfunction]
fn d3_color_luminance(color: ColorTuple) -> PyResult<f64> {
    Ok(color_from_tuple(color)?.luminance() as f64)
}

#[pyfunction]
fn d3_color_to_lab(color: ColorTuple) -> PyResult<LabTuple> {
    Ok(lab_tuple(color_from_tuple(color)?.to_lab()))
}

#[pyfunction]
fn d3_color_to_hcl(color: ColorTuple) -> PyResult<HclTuple> {
    Ok(hcl_tuple(color_from_tuple(color)?.to_hcl()))
}

#[pyfunction]
#[pyo3(signature = (l, a, b, alpha=None))]
fn d3_lab_create(l: f64, a: f64, b: f64, alpha: Option<f64>) -> PyResult<LabTuple> {
    for (name, value) in [
        ("l", l),
        ("a", a),
        ("b", b),
        ("alpha", alpha.unwrap_or(1.0)),
    ] {
        finite(name, value)?;
    }
    Ok(lab_tuple(match alpha {
        Some(alpha) => Lab::with_alpha(l, a, b, alpha),
        None => Lab::new(l, a, b),
    }))
}

#[pyfunction]
fn d3_lab_from_color(color: ColorTuple) -> PyResult<LabTuple> {
    Ok(lab_tuple(Lab::from_rgb(&color_from_tuple(color)?)))
}

#[pyfunction]
fn d3_lab_to_color(lab: LabTuple) -> PyResult<ColorTuple> {
    Ok(color_tuple(lab_from_tuple(lab)?.to_rgb()))
}

#[pyfunction]
fn d3_lab_delta_e(left: LabTuple, right: LabTuple) -> PyResult<f64> {
    Ok(lab_from_tuple(left)?.delta_e(&lab_from_tuple(right)?))
}

#[pyfunction]
fn d3_lab_chroma(lab: LabTuple) -> PyResult<f64> {
    Ok(lab_from_tuple(lab)?.chroma())
}

#[pyfunction]
#[pyo3(signature = (h, c, l, alpha=None))]
fn d3_hcl_create(h: f64, c: f64, l: f64, alpha: Option<f64>) -> PyResult<HclTuple> {
    for (name, value) in [
        ("h", h),
        ("c", c),
        ("l", l),
        ("alpha", alpha.unwrap_or(1.0)),
    ] {
        finite(name, value)?;
    }
    Ok(hcl_tuple(match alpha {
        Some(alpha) => Hcl::with_alpha(h, c, l, alpha),
        None => Hcl::new(h, c, l),
    }))
}

#[pyfunction]
fn d3_hcl_from_lab(lab: LabTuple) -> PyResult<HclTuple> {
    Ok(hcl_tuple(Hcl::from_lab(&lab_from_tuple(lab)?)))
}

#[pyfunction]
fn d3_hcl_from_color(color: ColorTuple) -> PyResult<HclTuple> {
    Ok(hcl_tuple(Hcl::from_rgb(&color_from_tuple(color)?)))
}

#[pyfunction]
fn d3_hcl_to_lab(hcl: HclTuple) -> PyResult<LabTuple> {
    Ok(lab_tuple(hcl_from_tuple(hcl)?.to_lab()))
}

#[pyfunction]
fn d3_hcl_to_color(hcl: HclTuple) -> PyResult<ColorTuple> {
    Ok(color_tuple(hcl_from_tuple(hcl)?.to_rgb()))
}

#[pyfunction]
fn d3_hcl_interpolate(left: HclTuple, right: HclTuple, t: f64, long: bool) -> PyResult<HclTuple> {
    finite("t", t)?;
    let left = hcl_from_tuple(left)?;
    let right = hcl_from_tuple(right)?;
    Ok(hcl_tuple(if long {
        left.interpolate_long(&right, t)
    } else {
        left.interpolate(&right, t)
    }))
}

#[pyfunction]
fn d3_color_scheme(kind: &str) -> PyResult<Vec<ColorTuple>> {
    let scheme = match kind {
        "category10" => ColorScheme::category10(),
        "tableau10" => ColorScheme::tableau10(),
        "pastel" => ColorScheme::pastel(),
        _ => {
            return Err(PyValueError::new_err(format!(
                "unsupported categorical color scheme {kind:?}"
            )));
        }
    };
    Ok(scheme.colors().iter().copied().map(color_tuple).collect())
}

#[pyfunction]
fn d3_color_scheme_color(colors: Vec<ColorTuple>, index: usize) -> PyResult<ColorTuple> {
    let colors = colors
        .into_iter()
        .map(color_from_tuple)
        .collect::<PyResult<Vec<_>>>()?;
    Ok(color_tuple(ColorScheme::new(colors).color(index)))
}

fn hcl_values(values: Vec<HclTuple>) -> PyResult<Vec<Hcl>> {
    values.into_iter().map(hcl_from_tuple).collect()
}

fn sequential_scheme(name: &str) -> Option<SequentialScale> {
    SequentialScheme::get(name)
}

fn diverging_scheme(name: &str) -> Option<DivergingScale> {
    DivergingScheme::get(name)
}

#[pyfunction]
fn d3_interpolate_colors(colors: Vec<ColorTuple>, t: f32) -> PyResult<ColorTuple> {
    if !t.is_finite() {
        return Err(PyValueError::new_err("t must be finite"));
    }
    let colors = colors
        .into_iter()
        .map(color_from_tuple)
        .collect::<PyResult<Vec<_>>>()?;
    Ok(color_tuple(d3rs::color::interpolate_colors(&colors, t)))
}

#[pyfunction]
fn d3_sequential_color(t: f32) -> PyResult<ColorTuple> {
    if !t.is_finite() {
        return Err(PyValueError::new_err("t must be finite"));
    }
    Ok(color_tuple(d3rs::color::sequential_color(t)))
}

#[pyfunction]
fn d3_sequential_scheme_name(name: &str) -> Option<String> {
    sequential_scheme(name).map(|scale| scale.name().to_string())
}

fn resolve_sequential_scale(
    colors: Option<Vec<HclTuple>>,
    scheme: Option<&str>,
) -> PyResult<SequentialScale> {
    match (colors, scheme) {
        (Some(colors), None) => Ok(SequentialScale::new(hcl_values(colors)?, "Python")),
        (None, Some(name)) => sequential_scheme(name).ok_or_else(|| {
            PyValueError::new_err(format!("unknown sequential color scheme {name:?}"))
        }),
        _ => Err(PyValueError::new_err(
            "sequential scale requires exactly one of colors or scheme",
        )),
    }
}

#[pyfunction]
fn d3_sequential_scale_get(
    colors: Option<Vec<HclTuple>>,
    scheme: Option<&str>,
    t: f64,
) -> PyResult<ColorTuple> {
    finite("t", t)?;
    Ok(color_tuple(
        resolve_sequential_scale(colors, scheme)?.get(t),
    ))
}

#[pyfunction]
fn d3_sequential_scale_sample(
    colors: Option<Vec<HclTuple>>,
    scheme: Option<&str>,
    n: usize,
) -> PyResult<Vec<ColorTuple>> {
    Ok(resolve_sequential_scale(colors, scheme)?
        .sample(n)
        .into_iter()
        .map(color_tuple)
        .collect())
}

#[pyfunction]
fn d3_diverging_scheme_name(name: &str) -> Option<String> {
    diverging_scheme(name).map(|scale| scale.name().to_string())
}

fn resolve_diverging_scale(
    negative: Option<Vec<HclTuple>>,
    neutral: Option<HclTuple>,
    positive: Option<Vec<HclTuple>>,
    scheme: Option<&str>,
) -> PyResult<DivergingScale> {
    match (negative, neutral, positive, scheme) {
        (Some(negative), Some(neutral), Some(positive), None) => Ok(DivergingScale::new(
            hcl_values(negative)?,
            hcl_from_tuple(neutral)?,
            hcl_values(positive)?,
            "Python",
        )),
        (None, None, None, Some(name)) => diverging_scheme(name).ok_or_else(|| {
            PyValueError::new_err(format!("unknown diverging color scheme {name:?}"))
        }),
        _ => Err(PyValueError::new_err(
            "diverging scale requires either all custom stops or one scheme",
        )),
    }
}

#[pyfunction]
fn d3_diverging_scale_get(
    negative: Option<Vec<HclTuple>>,
    neutral: Option<HclTuple>,
    positive: Option<Vec<HclTuple>>,
    scheme: Option<&str>,
    t: f64,
) -> PyResult<ColorTuple> {
    finite("t", t)?;
    Ok(color_tuple(
        resolve_diverging_scale(negative, neutral, positive, scheme)?.get(t),
    ))
}

#[pyfunction]
fn d3_diverging_scale_sample(
    negative: Option<Vec<HclTuple>>,
    neutral: Option<HclTuple>,
    positive: Option<Vec<HclTuple>>,
    scheme: Option<&str>,
    n: usize,
) -> PyResult<Vec<ColorTuple>> {
    Ok(
        resolve_diverging_scale(negative, neutral, positive, scheme)?
            .sample(n)
            .into_iter()
            .map(color_tuple)
            .collect(),
    )
}

fn px_color_scale(kind: &str) -> PyResult<PxColorScale> {
    match kind {
        "viridis" => Ok(PxColorScale::Viridis),
        "plasma" => Ok(PxColorScale::Plasma),
        "inferno" => Ok(PxColorScale::Inferno),
        "magma" => Ok(PxColorScale::Magma),
        "heat" => Ok(PxColorScale::Heat),
        "coolwarm" => Ok(PxColorScale::Coolwarm),
        "greys" => Ok(PxColorScale::Greys),
        _ => Err(PyValueError::new_err(format!(
            "unsupported gpui-px color scale {kind:?}"
        ))),
    }
}

fn px_interaction_mode(mode: &str) -> PyResult<PxInteractionMode> {
    match mode {
        "none" => Ok(PxInteractionMode::None),
        "brush" => Ok(PxInteractionMode::Brush),
        "pan" => Ok(PxInteractionMode::Pan),
        "zoom" => Ok(PxInteractionMode::Zoom),
        _ => Err(PyValueError::new_err(format!(
            "unsupported gpui-px interaction mode {mode:?}"
        ))),
    }
}

fn px_interaction_domains(x_min: f64, x_max: f64, y_min: f64, y_max: f64) -> PyResult<()> {
    finite("x_min", x_min)?;
    finite("x_max", x_max)?;
    finite("y_min", y_min)?;
    finite("y_max", y_max)?;
    if x_min >= x_max || y_min >= y_max {
        return Err(PyValueError::new_err(
            "interaction domains must be ordered non-degenerate ranges",
        ));
    }
    Ok(())
}

/// Renderer-independent gpui-px brush, hover, pan, and zoom orchestration.
#[pyclass(name = "_PxChartInteraction")]
struct NativePxChartInteraction {
    state: PxChartInteraction,
}

#[pymethods]
impl NativePxChartInteraction {
    #[new]
    fn new(x_min: f64, x_max: f64, y_min: f64, y_max: f64) -> PyResult<Self> {
        px_interaction_domains(x_min, x_max, y_min, y_max)?;
        Ok(Self {
            state: PxChartInteraction::new(x_min, x_max, y_min, y_max),
        })
    }

    fn with_log_x(&self, enabled: bool) -> PyResult<Self> {
        if enabled && self.state.x_domain().0 <= 0.0 {
            return Err(PyValueError::new_err("log x domain must be positive"));
        }
        Ok(Self {
            state: self.state.clone().with_log_x(enabled),
        })
    }

    fn with_log_y(&self, enabled: bool) -> PyResult<Self> {
        if enabled && self.state.y_domain().0 <= 0.0 {
            return Err(PyValueError::new_err("log y domain must be positive"));
        }
        Ok(Self {
            state: self.state.clone().with_log_y(enabled),
        })
    }

    fn with_size(&self, width: f32, height: f32) -> PyResult<Self> {
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(PyValueError::new_err(
                "interaction size must be positive and finite",
            ));
        }
        Ok(Self {
            state: self.state.clone().with_size(width, height),
        })
    }

    fn with_mode(&self, mode: &str) -> PyResult<Self> {
        Ok(Self {
            state: self.state.clone().with_mode(px_interaction_mode(mode)?),
        })
    }

    fn start_brush(&mut self, x: f32, y: f32) -> PyResult<()> {
        if !x.is_finite() || !y.is_finite() {
            return Err(PyValueError::new_err("brush coordinates must be finite"));
        }
        self.state.start_brush(x, y);
        Ok(())
    }

    fn update_brush(&mut self, x: f32, y: f32) -> PyResult<()> {
        if !x.is_finite() || !y.is_finite() {
            return Err(PyValueError::new_err("brush coordinates must be finite"));
        }
        self.state.update_brush(x, y);
        Ok(())
    }

    fn end_brush(&mut self, apply_zoom: bool) -> Option<(f64, f64, f64, f64)> {
        self.state
            .end_brush(apply_zoom)
            .map(|value| (value.x0, value.y0, value.x1, value.y1))
    }

    fn cancel_brush(&mut self) {
        self.state.cancel_brush();
    }

    fn current_brush_selection(&self) -> Option<(f64, f64, f64, f64)> {
        self.state
            .current_brush_selection()
            .map(|value| (value.x0, value.y0, value.x1, value.y1))
    }

    fn is_brushing(&self) -> bool {
        self.state.is_brushing()
    }

    fn zoom_to(&mut self, x_min: f64, x_max: f64, y_min: f64, y_max: f64) -> PyResult<()> {
        px_interaction_domains(x_min, x_max, y_min, y_max)?;
        self.state.zoom_to(x_min, x_max, y_min, y_max);
        Ok(())
    }

    fn set_viewport_without_history(
        &mut self,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
    ) -> PyResult<()> {
        px_interaction_domains(x_min, x_max, y_min, y_max)?;
        self.state
            .set_viewport_without_history(x_min, x_max, y_min, y_max);
        Ok(())
    }

    fn reset_zoom(&mut self) {
        self.state.reset_zoom();
    }

    fn zoom_back(&mut self) -> bool {
        self.state.zoom_back()
    }

    fn is_zoomed(&self) -> bool {
        self.state.is_zoomed()
    }

    fn x_domain(&self) -> (f64, f64) {
        self.state.x_domain()
    }

    fn y_domain(&self) -> (f64, f64) {
        self.state.y_domain()
    }

    fn zoom_level(&self) -> usize {
        self.state.zoom_level()
    }

    fn point_to_domain(&self, x: f32, y: f32) -> PyResult<(f64, f64)> {
        if !x.is_finite() || !y.is_finite() {
            return Err(PyValueError::new_err("point coordinates must be finite"));
        }
        Ok(self.state.point_to_domain(x, y))
    }

    fn update_hover_pixel(&mut self, x: f32, y: f32) -> Option<(f64, f64)> {
        self.state.update_hover_pixel(x, y)
    }

    fn clear_hover(&mut self) {
        self.state.clear_hover();
    }

    fn hover_domain(&self) -> Option<(f64, f64)> {
        self.state.hover_domain()
    }

    fn pan_by_pixels(&mut self, dx: f32, dy: f32) -> PyResult<()> {
        if !dx.is_finite() || !dy.is_finite() {
            return Err(PyValueError::new_err("pan deltas must be finite"));
        }
        self.state.pan_by_pixels(dx, dy);
        Ok(())
    }

    fn zoom_around_pixel(&mut self, x: f32, y: f32, factor: f64) -> PyResult<()> {
        if !x.is_finite() || !y.is_finite() || !factor.is_finite() || factor <= 0.0 {
            return Err(PyValueError::new_err(
                "zoom point must be finite and factor must be positive",
            ));
        }
        self.state.zoom_around_pixel(x, y, factor);
        Ok(())
    }

    fn zoom_around_domain(&mut self, x: f64, y: f64, factor: f64) -> PyResult<()> {
        finite("x", x)?;
        finite("y", y)?;
        if !factor.is_finite() || factor <= 0.0 {
            return Err(PyValueError::new_err(
                "zoom factor must be positive and finite",
            ));
        }
        self.state.zoom_around_domain(x, y, factor);
        Ok(())
    }
}

#[pyfunction]
fn px_chart_keyboard_action(key: &str) -> Option<&'static str> {
    px_keyboard_action_for_key(key).map(|action| match action {
        PxChartKeyboardAction::ZoomIn => "zoom_in",
        PxChartKeyboardAction::ZoomOut => "zoom_out",
        PxChartKeyboardAction::PanLeft => "pan_left",
        PxChartKeyboardAction::PanRight => "pan_right",
        PxChartKeyboardAction::PanUp => "pan_up",
        PxChartKeyboardAction::PanDown => "pan_down",
        PxChartKeyboardAction::ResetZoom => "reset_zoom",
    })
}

type NativePxCapabilityEntry = (
    String,
    String,
    String,
    Vec<String>,
    Vec<String>,
    String,
    String,
    String,
);

type NativePxCapabilityReport = (
    u32,
    String,
    String,
    bool,
    Vec<NativePxCapabilityEntry>,
    Vec<String>,
    String,
);

/// Return gpui-px's authoritative, allocation-bounded capability inventory.
#[pyfunction]
fn px_chart_capability_report() -> NativePxCapabilityReport {
    let report = px_native_chart_capability_report();
    let entries = report
        .entries
        .iter()
        .map(|entry| {
            (
                entry.id.to_owned(),
                entry.capability.to_owned(),
                entry.chart_families.to_owned(),
                entry
                    .story_ids
                    .iter()
                    .map(|value| (*value).to_owned())
                    .collect(),
                entry
                    .test_contracts
                    .iter()
                    .map(|value| (*value).to_owned())
                    .collect(),
                entry.status.as_str().to_owned(),
                entry.evidence.to_owned(),
                entry.release_requirement.to_owned(),
            )
        })
        .collect();
    let blocking = report
        .blocking_entries()
        .map(|entry| entry.id.to_owned())
        .collect();
    (
        report.schema_version,
        report.report_type.to_owned(),
        report.reviewed_on.to_owned(),
        report.all_release_ready(),
        entries,
        blocking,
        report.to_markdown_table(),
    )
}

type NativePxTreemapRect = (f64, f64, f64, f64, String, f64, usize, usize);

fn px_treemap_node_from_preorder(
    index: usize,
    names: &[String],
    values: &[f64],
    children: &[Vec<usize>],
) -> PxTreemapNode {
    let child_nodes = children[index]
        .iter()
        .map(|child| px_treemap_node_from_preorder(*child, names, values, children))
        .collect::<Vec<_>>();
    if child_nodes.is_empty() {
        PxTreemapNode::new(names[index].clone(), values[index])
    } else {
        PxTreemapNode::with_children(names[index].clone(), child_nodes)
    }
}

/// Compute renderer-independent gpui-px treemap rectangles from a flattened preorder tree.
#[pyfunction]
fn px_treemap_layout(
    names: Vec<String>,
    values: Vec<f64>,
    parents: Vec<i64>,
    method: &str,
    padding: f64,
    width: f64,
    height: f64,
) -> PyResult<Vec<NativePxTreemapRect>> {
    if names.is_empty() || names.len() != values.len() || names.len() != parents.len() {
        return Err(PyValueError::new_err(
            "names, values, and parents must have the same non-zero length",
        ));
    }
    if parents[0] != -1 {
        return Err(PyValueError::new_err("parents[0] must be -1 for the root"));
    }
    finite_values(&values)?;

    let mut children = vec![Vec::new(); names.len()];
    for (index, parent) in parents.iter().copied().enumerate().skip(1) {
        if parent < 0 || parent as usize >= index {
            return Err(PyValueError::new_err(format!(
                "parents[{index}] must reference an earlier preorder node"
            )));
        }
        children[parent as usize].push(index);
    }

    let method = match method {
        "squarify" => PxTilingMethod::Squarify,
        "binary" => PxTilingMethod::Binary,
        "slice" => PxTilingMethod::Slice,
        "dice" => PxTilingMethod::Dice,
        "slice_dice" => PxTilingMethod::SliceDice,
        _ => {
            return Err(PyValueError::new_err(format!(
                "unsupported treemap tiling method: {method}"
            )));
        }
    };
    let root = px_treemap_node_from_preorder(0, &names, &values, &children);
    px_treemap(&root)
        .tiling_method(method)
        .padding(padding)
        .layout(width, height)
        .map(|rects| {
            rects
                .into_iter()
                .map(|rect| {
                    (
                        rect.x0,
                        rect.y0,
                        rect.x1,
                        rect.y1,
                        rect.name,
                        rect.value,
                        rect.depth,
                        rect.category_index,
                    )
                })
                .collect()
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

fn native_buffer_word<const N: usize>(values: &[u8], offset: usize) -> [u8; N] {
    std::array::from_fn(|index| values[offset + index])
}

fn decode_native_float_buffer(
    values: &[u8],
    dtype: &str,
    count: usize,
    name: &str,
) -> PyResult<Vec<f64>> {
    let width = match dtype.to_ascii_lowercase().as_str() {
        "f32" | "float32" => 4,
        "f64" | "float64" => 8,
        _ => {
            return Err(PyValueError::new_err(format!(
                "{name} dtype must be f32 or f64"
            )));
        }
    };
    if values.len() != count.saturating_mul(width) {
        return Err(PyValueError::new_err(format!(
            "{name} buffer length does not match shape and dtype"
        )));
    }
    Ok((0..count)
        .map(|index| {
            let offset = index * width;
            if width == 4 {
                f32::from_ne_bytes(native_buffer_word::<4>(values, offset)) as f64
            } else {
                f64::from_ne_bytes(native_buffer_word::<8>(values, offset))
            }
        })
        .collect())
}

fn native_unsigned_width(dtype: &str, name: &str) -> PyResult<usize> {
    match dtype.to_ascii_lowercase().as_str() {
        "u8" | "uint8" => Ok(1),
        "u16" | "uint16" => Ok(2),
        "u32" | "uint32" => Ok(4),
        "u64" | "uint64" => Ok(8),
        _ => Err(PyValueError::new_err(format!(
            "{name} dtype must be an unsigned integer"
        ))),
    }
}

fn decode_native_unsigned_buffer(
    values: &[u8],
    dtype: &str,
    count: usize,
    name: &str,
) -> PyResult<Vec<u64>> {
    let width = native_unsigned_width(dtype, name)?;
    if values.len() != count.saturating_mul(width) {
        return Err(PyValueError::new_err(format!(
            "{name} buffer length does not match shape and dtype"
        )));
    }
    Ok((0..count)
        .map(|index| {
            let offset = index * width;
            match width {
                1 => values[offset] as u64,
                2 => u16::from_ne_bytes(native_buffer_word::<2>(values, offset)) as u64,
                4 => u32::from_ne_bytes(native_buffer_word::<4>(values, offset)) as u64,
                8 => u64::from_ne_bytes(native_buffer_word::<8>(values, offset)),
                _ => unreachable!("validated unsigned width"),
            }
        })
        .collect())
}

fn native_mesh_axis(value: &str) -> PyResult<MeshCoordinateAxis> {
    match value.to_ascii_lowercase().as_str() {
        "x" => Ok(MeshCoordinateAxis::X),
        "y" => Ok(MeshCoordinateAxis::Y),
        "z" => Ok(MeshCoordinateAxis::Z),
        _ => Err(PyValueError::new_err(format!(
            "unsupported mesh coordinate axis {value:?}"
        ))),
    }
}

type NativeMeshPick = (
    String,
    String,
    u32,
    Option<u64>,
    Option<u32>,
    Option<u64>,
    (f64, f64, f64),
    Option<f64>,
    Option<String>,
);

/// Retained gpui-px mesh and spatial index built once per ArrayData generation.
#[pyclass(name = "_PxMeshPickIndex")]
struct NativePxMeshPickIndex {
    mesh: Option<NativeTriangleMesh>,
    field: Option<NativeMeshScalarField>,
    index: Option<NativeTriGridIndex>,
    horizontal: MeshCoordinateAxis,
    vertical: MeshCoordinateAxis,
    plot_id: String,
}

#[pymethods]
impl NativePxMeshPickIndex {
    #[new]
    #[pyo3(signature = (
        positions, positions_dtype, vertex_count,
        triangles, triangles_dtype, triangle_count,
        mesh_id, plot_id, horizontal="x", vertical="y",
        field=None, field_dtype=None, field_association="vertex", field_id="field",
        valid=None, vertex_ids=None, vertex_ids_dtype=None,
        cell_ids=None, cell_ids_dtype=None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        positions: Vec<u8>,
        positions_dtype: &str,
        vertex_count: usize,
        triangles: Vec<u8>,
        triangles_dtype: &str,
        triangle_count: usize,
        mesh_id: &str,
        plot_id: &str,
        horizontal: &str,
        vertical: &str,
        field: Option<Vec<u8>>,
        field_dtype: Option<&str>,
        field_association: &str,
        field_id: &str,
        valid: Option<Vec<u8>>,
        vertex_ids: Option<Vec<u8>>,
        vertex_ids_dtype: Option<&str>,
        cell_ids: Option<Vec<u8>>,
        cell_ids_dtype: Option<&str>,
    ) -> PyResult<Self> {
        if mesh_id.is_empty() || plot_id.is_empty() {
            return Err(PyValueError::new_err("mesh and plot ids must be non-empty"));
        }
        let position_values = decode_native_float_buffer(
            &positions,
            positions_dtype,
            vertex_count.saturating_mul(3),
            "positions",
        )?;
        let positions = position_values
            .chunks_exact(3)
            .map(|value| [value[0], value[1], value[2]])
            .collect::<Vec<_>>();

        let triangle_values = decode_native_unsigned_buffer(
            &triangles,
            triangles_dtype,
            triangle_count.saturating_mul(3),
            "triangles",
        )?;
        let triangles = triangle_values
            .chunks_exact(3)
            .map(|value| {
                Ok([
                    u32::try_from(value[0])
                        .map_err(|_| PyValueError::new_err("triangle index exceeds u32 range"))?,
                    u32::try_from(value[1])
                        .map_err(|_| PyValueError::new_err("triangle index exceeds u32 range"))?,
                    u32::try_from(value[2])
                        .map_err(|_| PyValueError::new_err("triangle index exceeds u32 range"))?,
                ])
            })
            .collect::<PyResult<Vec<_>>>()?;

        let decode_ids = |buffer: Option<Vec<u8>>,
                          dtype: Option<&str>,
                          count: usize,
                          name: &str|
         -> PyResult<Option<Arc<[u64]>>> {
            buffer
                .map(|buffer| {
                    decode_native_unsigned_buffer(
                        &buffer,
                        dtype.ok_or_else(|| {
                            PyValueError::new_err(format!("{name} dtype is required"))
                        })?,
                        count,
                        name,
                    )
                    .map(Arc::from)
                })
                .transpose()
        };
        let mesh = NativeTriangleMesh {
            id: Arc::from(mesh_id),
            positions: Arc::from(positions),
            triangles: Arc::from(triangles),
            vertex_ids: decode_ids(vertex_ids, vertex_ids_dtype, vertex_count, "vertex_ids")?,
            cell_ids: decode_ids(cell_ids, cell_ids_dtype, triangle_count, "cell_ids")?,
        };
        mesh.validate()
            .map_err(|error| PyValueError::new_err(error.to_string()))?;

        let association = match field_association {
            "vertex" => MeshScalarAssociation::Vertex,
            "cell" => MeshScalarAssociation::Cell,
            _ => {
                return Err(PyValueError::new_err(
                    "field association must be vertex or cell",
                ));
            }
        };
        let field_count = match association {
            MeshScalarAssociation::Vertex => vertex_count,
            MeshScalarAssociation::Cell => triangle_count,
        };
        let field = field
            .map(|buffer| {
                let values = decode_native_float_buffer(
                    &buffer,
                    field_dtype.ok_or_else(|| PyValueError::new_err("field dtype is required"))?,
                    field_count,
                    "field",
                )?;
                let valid = valid
                    .as_ref()
                    .map(|buffer| {
                        let bytes = buffer.as_slice();
                        if bytes.len() != field_count {
                            return Err(PyValueError::new_err(
                                "valid buffer length must match field length",
                            ));
                        }
                        Ok(Arc::from(
                            bytes.iter().map(|value| *value != 0).collect::<Vec<_>>(),
                        ))
                    })
                    .transpose()?;
                let field = NativeMeshScalarField {
                    id: Arc::from(field_id),
                    label: Arc::from(field_id),
                    unit: None,
                    values: Arc::from(values),
                    association,
                    valid,
                };
                field
                    .validate(&mesh)
                    .map_err(|error| PyValueError::new_err(error.to_string()))?;
                Ok::<NativeMeshScalarField, PyErr>(field)
            })
            .transpose()?;

        let horizontal = native_mesh_axis(horizontal)?;
        let vertical = native_mesh_axis(vertical)?;
        if horizontal == vertical {
            return Err(PyValueError::new_err(
                "mesh projection axes must be distinct",
            ));
        }
        let projected = mesh
            .positions
            .iter()
            .map(|&position| project_mesh_2d(horizontal, vertical, position))
            .collect::<Vec<_>>();
        let index = NativeTriGridIndex::build(&projected, &mesh.triangles);
        Ok(Self {
            mesh: Some(mesh),
            field,
            index: Some(index),
            horizontal,
            vertical,
            plot_id: plot_id.to_owned(),
        })
    }

    fn pick(&self, x: f64, y: f64) -> PyResult<Option<NativeMeshPick>> {
        finite("x", x)?;
        finite("y", y)?;
        let mesh = self
            .mesh
            .as_ref()
            .ok_or_else(|| PyValueError::new_err("mesh pick index is closed"))?;
        let index = self
            .index
            .as_ref()
            .ok_or_else(|| PyValueError::new_err("mesh pick index is closed"))?;
        Ok(px_mesh_pick_2d(
            mesh,
            self.field.as_ref(),
            index,
            self.horizontal,
            self.vertical,
            [x, y],
            &self.plot_id,
        )
        .map(|pick| {
            (
                pick.plot_id.to_string(),
                pick.mesh_id.to_string(),
                pick.cell_index,
                pick.cell_id,
                pick.nearest_vertex_index,
                pick.vertex_id,
                (
                    pick.world_position[0],
                    pick.world_position[1],
                    pick.world_position[2],
                ),
                pick.displayed_value,
                pick.field_id.map(|value| value.to_string()),
            )
        }))
    }

    fn close(&mut self) {
        self.index = None;
        self.field = None;
        self.mesh = None;
    }

    fn is_closed(&self) -> bool {
        self.mesh.is_none()
    }

    fn vertex_count(&self) -> usize {
        self.mesh.as_ref().map_or(0, |mesh| mesh.positions.len())
    }

    fn triangle_count(&self) -> usize {
        self.mesh.as_ref().map_or(0, |mesh| mesh.triangles.len())
    }
}

/// Map a normalized scalar through a built-in gpui-px color scale.
#[pyfunction]
fn px_color_scale_map(kind: &str, t: f64) -> PyResult<String> {
    finite("t", t)?;
    Ok(px_color_scale(kind)?.map(t).to_hex())
}

/// Return the stable shader colormap index used by gpui-px and gpui-d3rs.
#[pyfunction]
fn px_color_scale_index(kind: &str) -> PyResult<u32> {
    Ok(px_color_scale(kind)?.to_colormap_index())
}

/// Resolve gpui-px automatic, fixed, or symmetric scalar display ranges.
#[pyfunction]
#[pyo3(signature = (kind, data_min, data_max, lower=None, upper=None, center=None, extent=None))]
fn px_color_range_resolve(
    kind: &str,
    data_min: f64,
    data_max: f64,
    lower: Option<f64>,
    upper: Option<f64>,
    center: Option<f64>,
    extent: Option<f64>,
) -> PyResult<(f64, f64)> {
    let range = match kind {
        "auto" => PxColorRange::Auto,
        "fixed" => PxColorRange::Fixed {
            min: lower.ok_or_else(|| PyValueError::new_err("fixed color range requires lower"))?,
            max: upper.ok_or_else(|| PyValueError::new_err("fixed color range requires upper"))?,
        },
        "symmetric" => PxColorRange::Symmetric {
            center: center
                .ok_or_else(|| PyValueError::new_err("symmetric color range requires center"))?,
            extent: extent.map_or(PxAutoOrFixed::Auto, PxAutoOrFixed::Fixed),
        },
        _ => {
            return Err(PyValueError::new_err(format!(
                "unsupported gpui-px color range {kind:?}"
            )));
        }
    };
    range
        .resolve(data_min, data_max)
        .map(|[min, max]| (min, max))
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

/// Return the insertion point before equal values in a sorted numeric slice.
#[pyfunction]
fn bisect_left(py: Python<'_>, data: Vec<f64>, value: f64) -> PyResult<usize> {
    finite_values(&data)?;
    sorted_values(&data)?;
    finite("value", value)?;
    Ok(py.allow_threads(|| d3rs::array::bisect_left_f64(&data, value)))
}

/// Return the insertion point after equal values in a sorted numeric slice.
#[pyfunction]
fn bisect_right(py: Python<'_>, data: Vec<f64>, value: f64) -> PyResult<usize> {
    finite_values(&data)?;
    sorted_values(&data)?;
    finite("value", value)?;
    Ok(py.allow_threads(|| d3rs::array::bisect_right_f64(&data, value)))
}

/// Compute a d3-array quantile, sorting a private copy of the input.
#[pyfunction]
fn quantile(py: Python<'_>, data: Vec<f64>, percentile: f64) -> PyResult<Option<f64>> {
    finite_values(&data)?;
    if !percentile.is_finite() || !(0.0..=1.0).contains(&percentile) {
        return Err(PyValueError::new_err("percentile must be in [0, 1]"));
    }
    Ok(py.allow_threads(move || {
        let mut data = data;
        d3rs::array::quantile(&mut data, percentile)
    }))
}

#[pyfunction]
fn quantile_sorted(py: Python<'_>, data: Vec<f64>, percentile: f64) -> PyResult<Option<f64>> {
    finite_values(&data)?;
    sorted_values(&data)?;
    if !percentile.is_finite() || !(0.0..=1.0).contains(&percentile) {
        return Err(PyValueError::new_err("percentile must be in [0, 1]"));
    }
    Ok(py.allow_threads(|| d3rs::array::quantile_sorted(&data, percentile)))
}

#[pyfunction]
fn least_index(py: Python<'_>, data: Vec<f64>, value: f64) -> PyResult<Option<usize>> {
    finite_values(&data)?;
    sorted_values(&data)?;
    finite("value", value)?;
    Ok(py.allow_threads(|| d3rs::array::least_index(&data, value)))
}

#[pyfunction(name = "min")]
fn array_min(py: Python<'_>, data: Vec<f64>) -> PyResult<Option<f64>> {
    finite_values(&data)?;
    Ok(py.allow_threads(|| data.iter().copied().reduce(f64::min)))
}

#[pyfunction(name = "max")]
fn array_max(py: Python<'_>, data: Vec<f64>) -> PyResult<Option<f64>> {
    finite_values(&data)?;
    Ok(py.allow_threads(|| data.iter().copied().reduce(f64::max)))
}

#[pyfunction]
fn min_index(py: Python<'_>, data: Vec<f64>) -> PyResult<Option<usize>> {
    finite_values(&data)?;
    Ok(py.allow_threads(|| {
        if data.is_empty() {
            return None;
        }
        let mut result = 0;
        for index in 1..data.len() {
            if data[index] < data[result] {
                result = index;
            }
        }
        Some(result)
    }))
}

#[pyfunction]
fn max_index(py: Python<'_>, data: Vec<f64>) -> PyResult<Option<usize>> {
    finite_values(&data)?;
    Ok(py.allow_threads(|| {
        if data.is_empty() {
            return None;
        }
        let mut result = 0;
        for index in 1..data.len() {
            if data[index] > data[result] {
                result = index;
            }
        }
        Some(result)
    }))
}

#[pyfunction(name = "sum")]
fn array_sum(py: Python<'_>, data: Vec<f64>) -> PyResult<f64> {
    finite_values(&data)?;
    Ok(py.allow_threads(|| d3rs::array::sum(&data)))
}

#[pyfunction]
fn mean(py: Python<'_>, data: Vec<f64>) -> PyResult<Option<f64>> {
    finite_values(&data)?;
    Ok(py.allow_threads(|| d3rs::array::mean(&data)))
}

#[pyfunction]
fn median(py: Python<'_>, data: Vec<f64>) -> PyResult<Option<f64>> {
    finite_values(&data)?;
    Ok(py.allow_threads(move || {
        let mut data = data;
        d3rs::array::median(&mut data)
    }))
}

#[pyfunction]
fn variance(py: Python<'_>, data: Vec<f64>) -> PyResult<Option<f64>> {
    finite_values(&data)?;
    Ok(py.allow_threads(|| d3rs::array::variance(&data)))
}

#[pyfunction]
fn deviation(py: Python<'_>, data: Vec<f64>) -> PyResult<Option<f64>> {
    finite_values(&data)?;
    Ok(py.allow_threads(|| d3rs::array::deviation(&data)))
}

#[pyfunction]
fn extent(py: Python<'_>, data: Vec<f64>) -> PyResult<Option<(f64, f64)>> {
    finite_values(&data)?;
    Ok(py.allow_threads(|| {
        let minimum = data.iter().copied().reduce(f64::min);
        let maximum = data.iter().copied().reduce(f64::max);
        minimum.zip(maximum)
    }))
}

#[pyfunction]
fn cumsum(py: Python<'_>, data: Vec<f64>) -> PyResult<Vec<f64>> {
    finite_values(&data)?;
    Ok(py.allow_threads(|| d3rs::array::cumsum(&data)))
}

type NativeHistogramBin = (f64, f64, Vec<f64>);

/// Generate d3-array histogram bins without holding the Python GIL.
#[pyfunction]
#[pyo3(signature = (data, *, strategy="sturges", count=None, thresholds=None, domain=None))]
fn histogram(
    py: Python<'_>,
    data: Vec<f64>,
    strategy: &str,
    count: Option<i64>,
    thresholds: Option<Vec<f64>>,
    domain: Option<(f64, f64)>,
) -> PyResult<Vec<NativeHistogramBin>> {
    finite_values(&data)?;
    if let Some((minimum, maximum)) = domain {
        finite("domain[0]", minimum)?;
        finite("domain[1]", maximum)?;
        if minimum >= maximum {
            return Err(PyValueError::new_err(
                "histogram domain must be finite and increasing",
            ));
        }
    }

    let count = match count {
        Some(value) if value <= 0 => {
            return Err(PyValueError::new_err(
                "histogram count must be a positive integer",
            ));
        }
        Some(value) => Some(value as usize),
        None => None,
    };
    if let Some(values) = thresholds.as_deref() {
        finite_values(values)?;
        if values.len() < 2 || values.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(PyValueError::new_err(
                "histogram thresholds must contain at least two increasing finite values",
            ));
        }
    }

    let mut generator = d3rs::array::BinGenerator::new().value(|value: &f64| *value);
    if let Some((minimum, maximum)) = domain {
        generator = generator.domain(minimum, maximum);
    }
    generator = match strategy {
        "sturges" if count.is_none() && thresholds.is_none() => generator.thresholds_sturges(),
        "freedman_diaconis" if count.is_none() && thresholds.is_none() => {
            generator.thresholds_freedman_diaconis()
        }
        "scott" if count.is_none() && thresholds.is_none() => generator.thresholds_scott(),
        "count" if thresholds.is_none() => generator.thresholds_count(
            count
                .ok_or_else(|| PyValueError::new_err("histogram count strategy requires count"))?,
        ),
        "values" if count.is_none() => generator.thresholds(thresholds.ok_or_else(|| {
            PyValueError::new_err("histogram values strategy requires thresholds")
        })?),
        "sturges" | "freedman_diaconis" | "scott" => {
            return Err(PyValueError::new_err(format!(
                "histogram {strategy} strategy does not accept count or thresholds"
            )));
        }
        "count" => {
            return Err(PyValueError::new_err(
                "histogram count strategy does not accept thresholds",
            ));
        }
        "values" => {
            return Err(PyValueError::new_err(
                "histogram values strategy does not accept count",
            ));
        }
        _ => {
            return Err(PyValueError::new_err(format!(
                "unsupported histogram threshold strategy {strategy:?}"
            )));
        }
    };

    Ok(py.allow_threads(move || {
        generator
            .generate(&data)
            .into_iter()
            .map(|bin| (bin.x0, bin.x1, bin.values))
            .collect()
    }))
}

#[pyfunction]
fn threshold_sturges(size: i64) -> PyResult<usize> {
    if size < 0 {
        return Err(PyValueError::new_err(
            "threshold_sturges size must be non-negative",
        ));
    }
    Ok(d3rs::array::threshold_sturges(size as usize))
}

#[pyfunction]
fn nice_bin_edges(minimum: f64, maximum: f64, count: i64) -> PyResult<Vec<f64>> {
    finite("minimum", minimum)?;
    finite("maximum", maximum)?;
    if minimum > maximum {
        return Err(PyValueError::new_err(
            "nice_bin_edges minimum must not exceed maximum",
        ));
    }
    if count <= 0 {
        return Err(PyValueError::new_err(
            "nice_bin_edges count must be a positive integer",
        ));
    }
    Ok(d3rs::array::nice_bin_edges(
        minimum,
        maximum,
        count as usize,
    ))
}

#[pyfunction]
fn reverse(py: Python<'_>, mut data: Vec<f64>) -> PyResult<Vec<f64>> {
    finite_values(&data)?;
    Ok(py.allow_threads(move || {
        d3rs::array::reverse(&mut data);
        data
    }))
}

#[pyfunction]
fn shuffle_seeded(py: Python<'_>, mut data: Vec<f64>, seed: i64) -> PyResult<Vec<f64>> {
    finite_values(&data)?;
    if seed < 0 {
        return Err(PyValueError::new_err(
            "shuffle_seeded seed must be non-negative",
        ));
    }
    Ok(py.allow_threads(move || {
        d3rs::array::shuffle_seeded(&mut data, seed as u64);
        data
    }))
}

#[pyfunction]
fn shuffle(py: Python<'_>, mut data: Vec<f64>) -> PyResult<Vec<f64>> {
    finite_values(&data)?;
    Ok(py.allow_threads(move || {
        d3rs::array::shuffle(&mut data);
        data
    }))
}

#[pyfunction]
fn pairs(py: Python<'_>, data: Vec<f64>) -> PyResult<Vec<(f64, f64)>> {
    finite_values(&data)?;
    Ok(py.allow_threads(|| {
        d3rs::array::pairs(&data)
            .into_iter()
            .map(|(left, right)| (*left, *right))
            .collect()
    }))
}

#[pyfunction]
fn cross(py: Python<'_>, left: Vec<f64>, right: Vec<f64>) -> PyResult<Vec<(f64, f64)>> {
    finite_values(&left)?;
    finite_values(&right)?;
    Ok(py.allow_threads(|| {
        d3rs::array::cross(&left, &right)
            .into_iter()
            .map(|(left, right)| (*left, *right))
            .collect()
    }))
}

#[pyfunction]
fn unique(py: Python<'_>, data: Vec<f64>) -> PyResult<Vec<f64>> {
    let data = finite_keys(data)?;
    Ok(py.allow_threads(|| key_values(d3rs::array::unique(&data))))
}

#[pyfunction(name = "sort")]
fn array_sort(py: Python<'_>, data: Vec<f64>) -> PyResult<Vec<f64>> {
    let mut data = finite_keys(data)?;
    Ok(py.allow_threads(move || {
        d3rs::array::sort_by(&mut data, |value| *value);
        key_values(data)
    }))
}

#[pyfunction]
fn sort_descending(py: Python<'_>, data: Vec<f64>) -> PyResult<Vec<f64>> {
    let mut data = finite_keys(data)?;
    Ok(py.allow_threads(move || {
        d3rs::array::sort_by_desc(&mut data, |value| *value);
        key_values(data)
    }))
}

#[pyfunction]
fn merge_sorted(py: Python<'_>, slices: Vec<Vec<f64>>) -> PyResult<Vec<f64>> {
    for values in &slices {
        finite_values(values)?;
        sorted_values(values)?;
    }
    let slices: Vec<Vec<FiniteF64>> = slices
        .into_iter()
        .map(|values| values.into_iter().map(FiniteF64).collect())
        .collect();
    Ok(py.allow_threads(|| {
        let references: Vec<&[FiniteF64]> = slices.iter().map(Vec::as_slice).collect();
        key_values(d3rs::array::merge_sorted(&references))
    }))
}

#[pyfunction]
fn binary_search(py: Python<'_>, data: Vec<f64>, value: f64) -> PyResult<Option<usize>> {
    let data = finite_keys(data)?;
    finite("value", value)?;
    let raw: Vec<f64> = data.iter().map(|value| value.0).collect();
    sorted_values(&raw)?;
    Ok(py.allow_threads(|| d3rs::array::binary_search(&data, &FiniteF64(value))))
}

#[pyfunction]
fn difference(py: Python<'_>, left: Vec<f64>, right: Vec<f64>) -> PyResult<Vec<f64>> {
    let left = finite_keys(left)?;
    let right = finite_keys(right)?;
    Ok(py.allow_threads(|| key_values(d3rs::array::difference(&left, &right))))
}

#[pyfunction]
fn intersection(py: Python<'_>, left: Vec<f64>, right: Vec<f64>) -> PyResult<Vec<f64>> {
    let left = finite_keys(left)?;
    let right = finite_keys(right)?;
    Ok(py.allow_threads(|| key_values(d3rs::array::intersection(&left, &right))))
}

#[pyfunction(name = "union")]
fn array_union(py: Python<'_>, left: Vec<f64>, right: Vec<f64>) -> PyResult<Vec<f64>> {
    let left = finite_keys(left)?;
    let right = finite_keys(right)?;
    Ok(py.allow_threads(|| key_values(d3rs::array::union(&left, &right))))
}

#[pyfunction]
fn symmetric_difference(py: Python<'_>, left: Vec<f64>, right: Vec<f64>) -> PyResult<Vec<f64>> {
    let left = finite_keys(left)?;
    let right = finite_keys(right)?;
    Ok(py.allow_threads(|| key_values(d3rs::array::symmetric_difference(&left, &right))))
}

#[pyfunction]
fn is_subset(py: Python<'_>, left: Vec<f64>, right: Vec<f64>) -> PyResult<bool> {
    let left = finite_keys(left)?;
    let right = finite_keys(right)?;
    Ok(py.allow_threads(|| d3rs::array::is_subset(&left, &right)))
}

#[pyfunction]
fn is_superset(py: Python<'_>, left: Vec<f64>, right: Vec<f64>) -> PyResult<bool> {
    let left = finite_keys(left)?;
    let right = finite_keys(right)?;
    Ok(py.allow_threads(|| d3rs::array::is_superset(&left, &right)))
}

#[pyfunction]
fn is_disjoint(py: Python<'_>, left: Vec<f64>, right: Vec<f64>) -> PyResult<bool> {
    let left = finite_keys(left)?;
    let right = finite_keys(right)?;
    Ok(py.allow_threads(|| d3rs::array::is_disjoint(&left, &right)))
}

fn tick_arguments(start: f64, stop: f64) -> PyResult<()> {
    finite("start", start)?;
    finite("stop", stop)
}

#[pyfunction]
fn nice_number(range: f64, round: bool) -> PyResult<f64> {
    finite("range", range)?;
    Ok(d3rs::array::nice_number(range, round))
}

#[pyfunction]
fn scale_nice_number(range: f64, round: bool) -> PyResult<f64> {
    finite("range", range)?;
    Ok(d3rs::scale::nice_number(range, round))
}

#[pyfunction]
fn generate_linear_ticks(py: Python<'_>, min: f64, max: f64, count: usize) -> PyResult<Vec<f64>> {
    finite("min", min)?;
    finite("max", max)?;
    Ok(py.allow_threads(|| d3rs::scale::generate_linear_ticks(min, max, count)))
}

#[pyfunction]
fn generate_log_ticks(
    py: Python<'_>,
    min: f64,
    max: f64,
    base: f64,
    subdivisions: bool,
) -> PyResult<Vec<f64>> {
    finite("min", min)?;
    finite("max", max)?;
    finite("base", base)?;
    Ok(py.allow_threads(|| d3rs::scale::generate_log_ticks(min, max, base, subdivisions)))
}

fn tick_count(count: i64) -> PyResult<usize> {
    usize::try_from(count).map_err(|_| PyValueError::new_err("count must be non-negative"))
}

#[pyfunction]
#[pyo3(signature = (start, stop, count=10))]
fn ticks(py: Python<'_>, start: f64, stop: f64, count: i64) -> PyResult<Vec<f64>> {
    tick_arguments(start, stop)?;
    let count = tick_count(count)?;
    Ok(py.allow_threads(|| d3rs::array::ticks(start, stop, count)))
}

#[pyfunction]
#[pyo3(signature = (start, stop, count=10))]
fn tick_step(py: Python<'_>, start: f64, stop: f64, count: i64) -> PyResult<f64> {
    tick_arguments(start, stop)?;
    let count = tick_count(count)?;
    Ok(py.allow_threads(|| d3rs::array::tick_step(start, stop, count)))
}

#[pyfunction]
#[pyo3(signature = (start, stop, count=10))]
fn tick_increment(py: Python<'_>, start: f64, stop: f64, count: i64) -> PyResult<f64> {
    tick_arguments(start, stop)?;
    let count = tick_count(count)?;
    Ok(py.allow_threads(|| d3rs::array::tick_increment(start, stop, count)))
}

#[pyfunction]
#[pyo3(signature = (start, stop, count=10))]
fn nice(py: Python<'_>, start: f64, stop: f64, count: i64) -> PyResult<(f64, f64)> {
    tick_arguments(start, stop)?;
    let count = tick_count(count)?;
    Ok(py.allow_threads(|| d3rs::array::nice(start, stop, count)))
}

#[pyfunction]
fn ticks_interval(py: Python<'_>, start: f64, stop: f64, interval: f64) -> PyResult<Vec<f64>> {
    tick_arguments(start, stop)?;
    if !interval.is_finite() || interval <= 0.0 {
        return Err(PyValueError::new_err(
            "interval must be positive and finite",
        ));
    }
    Ok(py.allow_threads(|| d3rs::array::ticks_interval(start, stop, interval)))
}

#[pyfunction]
#[pyo3(signature = (start, stop, *, base=10.0, subdivisions=true))]
fn log_ticks(
    py: Python<'_>,
    start: f64,
    stop: f64,
    base: f64,
    subdivisions: bool,
) -> PyResult<Vec<f64>> {
    tick_arguments(start, stop)?;
    if !base.is_finite() || base <= 1.0 {
        return Err(PyValueError::new_err(
            "base must be finite and greater than 1",
        ));
    }
    Ok(py.allow_threads(|| d3rs::array::log_ticks(start, stop, base, subdivisions)))
}

#[pyfunction]
#[pyo3(signature = (start, stop, count=10))]
fn time_ticks(py: Python<'_>, start: f64, stop: f64, count: i64) -> PyResult<Vec<f64>> {
    tick_arguments(start, stop)?;
    let count = tick_count(count)?;
    Ok(py.allow_threads(|| d3rs::array::time_ticks(start, stop, count)))
}

/// Apply gpui-d3rs's `LinearScale` without holding the Python GIL.
#[pyfunction]
#[pyo3(signature = (value, *, domain, range, clamp=false))]
fn linear_scale(
    py: Python<'_>,
    value: f64,
    domain: (f64, f64),
    range: (f64, f64),
    clamp: bool,
) -> PyResult<f64> {
    finite("value", value)?;
    finite("domain[0]", domain.0)?;
    finite("domain[1]", domain.1)?;
    finite("range[0]", range.0)?;
    finite("range[1]", range.1)?;
    Ok(py.allow_threads(|| {
        LinearScale::new()
            .domain(domain.0, domain.1)
            .range(range.0, range.1)
            .clamp(clamp)
            .scale(value)
    }))
}

#[pyfunction]
#[pyo3(signature = (value, *, domain, range, clamp=false))]
fn linear_scale_invert(
    py: Python<'_>,
    value: f64,
    domain: (f64, f64),
    range: (f64, f64),
    clamp: bool,
) -> PyResult<Option<f64>> {
    finite("value", value)?;
    finite("domain[0]", domain.0)?;
    finite("domain[1]", domain.1)?;
    finite("range[0]", range.0)?;
    finite("range[1]", range.1)?;
    Ok(py.allow_threads(|| {
        LinearScale::new()
            .domain(domain.0, domain.1)
            .range(range.0, range.1)
            .clamp(clamp)
            .invert(value)
    }))
}

#[pyfunction]
#[pyo3(signature = (domain, count=None))]
fn linear_scale_nice(
    py: Python<'_>,
    domain: (f64, f64),
    count: Option<usize>,
) -> PyResult<(f64, f64)> {
    finite("domain[0]", domain.0)?;
    finite("domain[1]", domain.1)?;
    Ok(py.allow_threads(|| {
        let scale = LinearScale::new().domain(domain.0, domain.1).nice(count);
        (scale.domain_min(), scale.domain_max())
    }))
}

#[pyfunction]
fn linear_scale_ticks(py: Python<'_>, domain: (f64, f64), count: usize) -> PyResult<Vec<f64>> {
    finite("domain[0]", domain.0)?;
    finite("domain[1]", domain.1)?;
    Ok(py.allow_threads(|| LinearScale::new().domain(domain.0, domain.1).ticks(count)))
}

fn validate_log_scale(domain: (f64, f64), range: (f64, f64), base: f64) -> PyResult<()> {
    finite("domain[0]", domain.0)?;
    finite("domain[1]", domain.1)?;
    finite("range[0]", range.0)?;
    finite("range[1]", range.1)?;
    finite("base", base)?;
    if domain.0 <= 0.0 || domain.1 <= 0.0 || domain.0 == domain.1 {
        return Err(PyValueError::new_err(
            "log scale domain endpoints must be positive and different",
        ));
    }
    if range.0 == range.1 {
        return Err(PyValueError::new_err(
            "log scale range endpoints must differ",
        ));
    }
    if base <= 0.0 || base == 1.0 {
        return Err(PyValueError::new_err(
            "log scale base must be positive and different from 1",
        ));
    }
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (value, *, domain, range, base=10.0, clamp=true))]
fn log_scale(
    py: Python<'_>,
    value: f64,
    domain: (f64, f64),
    range: (f64, f64),
    base: f64,
    clamp: bool,
) -> PyResult<f64> {
    finite("value", value)?;
    validate_log_scale(domain, range, base)?;
    if value <= 0.0 {
        return Err(PyValueError::new_err("log scale value must be positive"));
    }
    Ok(py.allow_threads(|| {
        LogScale::new()
            .domain(domain.0, domain.1)
            .range(range.0, range.1)
            .base(base)
            .clamp(clamp)
            .scale(value)
    }))
}

#[pyfunction]
#[pyo3(signature = (value, *, domain, range, base=10.0, clamp=true))]
fn log_scale_invert(
    py: Python<'_>,
    value: f64,
    domain: (f64, f64),
    range: (f64, f64),
    base: f64,
    clamp: bool,
) -> PyResult<Option<f64>> {
    finite("value", value)?;
    validate_log_scale(domain, range, base)?;
    Ok(py.allow_threads(|| {
        LogScale::new()
            .domain(domain.0, domain.1)
            .range(range.0, range.1)
            .base(base)
            .clamp(clamp)
            .invert(value)
    }))
}

#[pyfunction]
#[pyo3(signature = (domain, count, base=10.0))]
fn log_scale_ticks(
    py: Python<'_>,
    domain: (f64, f64),
    count: usize,
    base: f64,
) -> PyResult<Vec<f64>> {
    validate_log_scale(domain, (0.0, 1.0), base)?;
    Ok(py.allow_threads(|| {
        LogScale::new()
            .domain(domain.0, domain.1)
            .base(base)
            .ticks(count)
    }))
}

fn validate_pow_scale(domain: (f64, f64), range: (f64, f64), exponent: f64) -> PyResult<()> {
    finite("domain[0]", domain.0)?;
    finite("domain[1]", domain.1)?;
    finite("range[0]", range.0)?;
    finite("range[1]", range.1)?;
    finite("exponent", exponent)?;
    if exponent <= 0.0 {
        return Err(PyValueError::new_err(
            "power scale exponent must be positive",
        ));
    }
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (value, *, domain, range, exponent=1.0, clamp=false))]
fn pow_scale(
    py: Python<'_>,
    value: f64,
    domain: (f64, f64),
    range: (f64, f64),
    exponent: f64,
    clamp: bool,
) -> PyResult<f64> {
    finite("value", value)?;
    validate_pow_scale(domain, range, exponent)?;
    Ok(py.allow_threads(|| {
        PowScale::new()
            .domain(domain.0, domain.1)
            .range(range.0, range.1)
            .exponent(exponent)
            .clamp(clamp)
            .scale(value)
    }))
}

#[pyfunction]
#[pyo3(signature = (value, *, domain, range, exponent=1.0, clamp=false))]
fn pow_scale_invert(
    py: Python<'_>,
    value: f64,
    domain: (f64, f64),
    range: (f64, f64),
    exponent: f64,
    clamp: bool,
) -> PyResult<Option<f64>> {
    finite("value", value)?;
    validate_pow_scale(domain, range, exponent)?;
    Ok(py.allow_threads(|| {
        PowScale::new()
            .domain(domain.0, domain.1)
            .range(range.0, range.1)
            .exponent(exponent)
            .clamp(clamp)
            .invert(value)
    }))
}

#[pyfunction]
#[pyo3(signature = (domain, count=None))]
fn pow_scale_nice(
    py: Python<'_>,
    domain: (f64, f64),
    count: Option<usize>,
) -> PyResult<(f64, f64)> {
    validate_pow_scale(domain, (0.0, 1.0), 1.0)?;
    Ok(py.allow_threads(|| {
        let scale = PowScale::new().domain(domain.0, domain.1).nice(count);
        (scale.domain_min(), scale.domain_max())
    }))
}

#[pyfunction]
fn pow_scale_ticks(py: Python<'_>, domain: (f64, f64), count: usize) -> PyResult<Vec<f64>> {
    validate_pow_scale(domain, (0.0, 1.0), 1.0)?;
    Ok(py.allow_threads(|| PowScale::new().domain(domain.0, domain.1).ticks(count)))
}

fn validate_symlog_scale(domain: (f64, f64), range: (f64, f64), constant: f64) -> PyResult<()> {
    finite("domain[0]", domain.0)?;
    finite("domain[1]", domain.1)?;
    finite("range[0]", range.0)?;
    finite("range[1]", range.1)?;
    finite("constant", constant)?;
    if constant <= 0.0 {
        return Err(PyValueError::new_err(
            "symlog scale constant must be positive",
        ));
    }
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (value, *, domain, range, constant=1.0, clamp=false))]
fn symlog_scale(
    py: Python<'_>,
    value: f64,
    domain: (f64, f64),
    range: (f64, f64),
    constant: f64,
    clamp: bool,
) -> PyResult<f64> {
    finite("value", value)?;
    validate_symlog_scale(domain, range, constant)?;
    Ok(py.allow_threads(|| {
        SymlogScale::new()
            .domain(domain.0, domain.1)
            .range(range.0, range.1)
            .constant(constant)
            .clamp(clamp)
            .scale(value)
    }))
}

#[pyfunction]
#[pyo3(signature = (value, *, domain, range, constant=1.0, clamp=false))]
fn symlog_scale_invert(
    py: Python<'_>,
    value: f64,
    domain: (f64, f64),
    range: (f64, f64),
    constant: f64,
    clamp: bool,
) -> PyResult<Option<f64>> {
    finite("value", value)?;
    validate_symlog_scale(domain, range, constant)?;
    Ok(py.allow_threads(|| {
        SymlogScale::new()
            .domain(domain.0, domain.1)
            .range(range.0, range.1)
            .constant(constant)
            .clamp(clamp)
            .invert(value)
    }))
}

#[pyfunction]
#[pyo3(signature = (domain, count=None))]
fn symlog_scale_nice(
    py: Python<'_>,
    domain: (f64, f64),
    count: Option<usize>,
) -> PyResult<(f64, f64)> {
    validate_symlog_scale(domain, (0.0, 1.0), 1.0)?;
    Ok(py.allow_threads(|| {
        let scale = SymlogScale::new().domain(domain.0, domain.1).nice(count);
        (scale.domain_min(), scale.domain_max())
    }))
}

#[pyfunction]
fn symlog_scale_ticks(py: Python<'_>, domain: (f64, f64), count: usize) -> PyResult<Vec<f64>> {
    validate_symlog_scale(domain, (0.0, 1.0), 1.0)?;
    Ok(py.allow_threads(|| SymlogScale::new().domain(domain.0, domain.1).ticks(count)))
}

fn validate_thresholds(thresholds: &[f64], name: &str) -> PyResult<()> {
    finite_values(thresholds)?;
    if thresholds.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(PyValueError::new_err(format!(
            "{name} must be strictly increasing"
        )));
    }
    Ok(())
}

fn range_indices(range_len: usize, scale_name: &str) -> PyResult<Vec<usize>> {
    if range_len == 0 {
        return Err(PyValueError::new_err(format!(
            "{scale_name} requires at least one range value"
        )));
    }
    Ok((0..range_len).collect())
}

#[pyfunction]
fn threshold_scale_index(
    py: Python<'_>,
    value: f64,
    thresholds: Vec<f64>,
    range_len: usize,
) -> PyResult<usize> {
    validate_thresholds(&thresholds, "threshold scale thresholds")?;
    let range = range_indices(range_len, "threshold scale")?;
    Ok(py.allow_threads(|| {
        ThresholdScale::with_range(range)
            .domain(thresholds)
            .scale(value)
    }))
}

#[pyfunction]
fn threshold_scale_invert_extent(
    py: Python<'_>,
    thresholds: Vec<f64>,
    range_len: usize,
    index: usize,
) -> PyResult<Option<(f64, f64)>> {
    validate_thresholds(&thresholds, "threshold scale thresholds")?;
    let range = range_indices(range_len, "threshold scale")?;
    Ok(py.allow_threads(|| {
        ThresholdScale::with_range(range)
            .domain(thresholds)
            .invert_extent(index)
    }))
}

fn validate_quantize_domain(domain: (f64, f64)) -> PyResult<()> {
    finite("domain[0]", domain.0)?;
    finite("domain[1]", domain.1)
}

#[pyfunction]
fn quantize_scale_index(
    py: Python<'_>,
    value: f64,
    domain: (f64, f64),
    range_len: usize,
) -> PyResult<usize> {
    validate_quantize_domain(domain)?;
    let range = range_indices(range_len, "quantize scale")?;
    Ok(py.allow_threads(|| {
        QuantizeScale::with_range(range)
            .domain(domain.0, domain.1)
            .scale(value)
    }))
}

#[pyfunction]
fn quantize_scale_thresholds(
    py: Python<'_>,
    domain: (f64, f64),
    range_len: usize,
) -> PyResult<Vec<f64>> {
    validate_quantize_domain(domain)?;
    if range_len == 0 {
        return Ok(Vec::new());
    }
    let range: Vec<usize> = (0..range_len).collect();
    Ok(py.allow_threads(|| {
        QuantizeScale::with_range(range)
            .domain(domain.0, domain.1)
            .thresholds()
    }))
}

#[pyfunction]
fn quantize_scale_invert_extent(
    py: Python<'_>,
    domain: (f64, f64),
    range_len: usize,
    index: usize,
) -> PyResult<Option<(f64, f64)>> {
    validate_quantize_domain(domain)?;
    if range_len == 0 {
        return Ok(None);
    }
    let range: Vec<usize> = (0..range_len).collect();
    Ok(py.allow_threads(|| {
        QuantizeScale::with_range(range)
            .domain(domain.0, domain.1)
            .invert_extent(index)
    }))
}

fn validate_quantile_samples(samples: &[f64]) -> PyResult<()> {
    for (index, value) in samples.iter().enumerate() {
        if value.is_infinite() {
            return Err(PyValueError::new_err(format!(
                "quantile scale samples[{index}] must not be infinite"
            )));
        }
    }
    Ok(())
}

#[pyfunction]
fn quantile_scale_prepare(
    py: Python<'_>,
    samples: Vec<f64>,
    range_len: usize,
) -> PyResult<(Vec<f64>, Vec<f64>)> {
    validate_quantile_samples(&samples)?;
    let range: Vec<usize> = (0..range_len).collect();
    Ok(py.allow_threads(|| {
        let scale = QuantileScale::with_range(range).domain(samples);
        (scale.domain_samples().to_vec(), scale.quantiles().to_vec())
    }))
}

#[pyfunction]
fn quantile_scale_index(
    py: Python<'_>,
    value: f64,
    samples: Vec<f64>,
    range_len: usize,
) -> PyResult<usize> {
    validate_quantile_samples(&samples)?;
    let range = range_indices(range_len, "quantile scale")?;
    Ok(py.allow_threads(|| {
        QuantileScale::with_range(range)
            .domain(samples)
            .scale(value)
    }))
}

#[pyfunction]
fn quantile_scale_invert_extent(
    py: Python<'_>,
    samples: Vec<f64>,
    range_len: usize,
    index: usize,
) -> PyResult<Option<(f64, f64)>> {
    validate_quantile_samples(&samples)?;
    if range_len == 0 {
        return Ok(None);
    }
    let range: Vec<usize> = (0..range_len).collect();
    Ok(py.allow_threads(|| {
        QuantileScale::with_range(range)
            .domain(samples)
            .invert_extent(index)
    }))
}

fn validate_ordinal_layout(range: (f64, f64), padding: &[(&str, f64)]) -> PyResult<()> {
    finite("range[0]", range.0)?;
    finite("range[1]", range.1)?;
    for (name, value) in padding {
        finite(name, *value)?;
    }
    Ok(())
}

type BandScaleLayout = (Vec<f64>, f64, f64);

#[pyfunction]
#[pyo3(signature = (domain_len, *, range, padding_inner=0.0, padding_outer=0.0, align=0.5, round=false))]
fn band_scale_layout(
    py: Python<'_>,
    domain_len: usize,
    range: (f64, f64),
    padding_inner: f64,
    padding_outer: f64,
    align: f64,
    round: bool,
) -> PyResult<BandScaleLayout> {
    validate_ordinal_layout(
        range,
        &[
            ("padding_inner", padding_inner),
            ("padding_outer", padding_outer),
            ("align", align),
        ],
    )?;
    Ok(py.allow_threads(|| {
        let domain: Vec<usize> = (0..domain_len).collect();
        let scale = BandScale::new()
            .domain(domain.clone())
            .range(range.0, range.1)
            .padding_inner(padding_inner)
            .padding_outer(padding_outer)
            .align(align)
            .round(round);
        let positions = domain
            .iter()
            .filter_map(|value| scale.scale(value))
            .collect();
        (positions, scale.bandwidth(), scale.step())
    }))
}

type PointScaleLayout = (Vec<f64>, f64);

#[pyfunction]
#[pyo3(signature = (domain_len, *, range, padding=0.0, align=0.5, round=false))]
fn point_scale_layout(
    py: Python<'_>,
    domain_len: usize,
    range: (f64, f64),
    padding: f64,
    align: f64,
    round: bool,
) -> PyResult<PointScaleLayout> {
    validate_ordinal_layout(range, &[("padding", padding), ("align", align)])?;
    Ok(py.allow_threads(|| {
        let domain: Vec<usize> = (0..domain_len).collect();
        let scale = PointScale::new()
            .domain(domain.clone())
            .range(range.0, range.1)
            .padding(padding)
            .align(align)
            .round(round);
        let positions = domain
            .iter()
            .filter_map(|value| scale.scale(value))
            .collect();
        (positions, scale.step())
    }))
}

#[pyfunction]
fn clamp01(t: f64) -> PyResult<f64> {
    finite("t", t)?;
    Ok(d3rs::interpolate::clamp01(t))
}

macro_rules! numeric_interpolator {
    ($name:ident, $native:ident) => {
        #[pyfunction]
        fn $name(a: f64, b: f64, t: f64) -> PyResult<f64> {
            finite("a", a)?;
            finite("b", b)?;
            finite("t", t)?;
            let result = d3rs::interpolate::$native(a, b)(t);
            finite("interpolation result", result)?;
            Ok(result)
        }
    };
}

numeric_interpolator!(interpolate_clamped, interpolate_clamped);
numeric_interpolator!(interpolate_exp, interpolate_exp);
numeric_interpolator!(interpolate_date, interpolate_date);

#[pyfunction]
fn interpolate_basis(py: Python<'_>, values: Vec<f64>, t: f64) -> PyResult<f64> {
    finite_values(&values)?;
    finite("t", t)?;
    Ok(py.allow_threads(|| d3rs::interpolate::interpolate_basis(&values)(t)))
}

#[pyfunction]
fn interpolate_basis_closed(py: Python<'_>, values: Vec<f64>, t: f64) -> PyResult<f64> {
    finite_values(&values)?;
    finite("t", t)?;
    Ok(py.allow_threads(|| d3rs::interpolate::interpolate_basis_closed(&values)(t)))
}

#[pyfunction]
fn interpolate_discrete(py: Python<'_>, values: Vec<f64>, t: f64) -> PyResult<f64> {
    finite_values(&values)?;
    finite("t", t)?;
    if values.is_empty() {
        return Err(PyValueError::new_err(
            "interpolate_discrete requires at least one value",
        ));
    }
    Ok(py.allow_threads(|| d3rs::interpolate::interpolate_discrete(&values)(t)))
}

#[pyfunction]
fn interpolate_quantize(a: f64, b: f64, levels: i64, t: f64) -> PyResult<f64> {
    finite("a", a)?;
    finite("b", b)?;
    finite("t", t)?;
    let levels = usize::try_from(levels)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| PyValueError::new_err("levels must be positive"))?;
    Ok(d3rs::interpolate::interpolate_quantize(a, b, levels)(t))
}

#[pyfunction]
fn piecewise(py: Python<'_>, values: Vec<f64>, t: f64) -> PyResult<f64> {
    finite_values(&values)?;
    finite("t", t)?;
    if values.is_empty() {
        return Err(PyValueError::new_err(
            "piecewise requires at least one value",
        ));
    }
    Ok(py.allow_threads(|| d3rs::interpolate::piecewise(&values)(t)))
}

#[pyfunction]
fn piecewise_domain(
    py: Python<'_>,
    positions: Vec<f64>,
    values: Vec<f64>,
    t: f64,
) -> PyResult<f64> {
    finite_values(&positions)?;
    finite_values(&values)?;
    finite("t", t)?;
    if positions.is_empty() || positions.len() != values.len() {
        return Err(PyValueError::new_err(
            "piecewise_domain positions and values must have the same non-zero length",
        ));
    }
    sorted_values(&positions)?;
    Ok(py.allow_threads(|| d3rs::interpolate::piecewise_domain(&positions, &values)(t)))
}

#[pyfunction]
fn quantize_values(py: Python<'_>, values: Vec<f64>, t: f64) -> PyResult<f64> {
    finite_values(&values)?;
    finite("t", t)?;
    if values.is_empty() {
        return Err(PyValueError::new_err(
            "quantize requires at least one value",
        ));
    }
    Ok(py.allow_threads(|| d3rs::interpolate::quantize(&values)(t)))
}

fn ease_function(value: &str) -> PyResult<d3rs::interpolate::EaseFunction> {
    use d3rs::interpolate::EaseFunction;
    match value {
        "linear" => Ok(EaseFunction::Linear),
        "quad_in" => Ok(EaseFunction::QuadIn),
        "quad_out" => Ok(EaseFunction::QuadOut),
        "quad_in_out" => Ok(EaseFunction::QuadInOut),
        "cubic_in" => Ok(EaseFunction::CubicIn),
        "cubic_out" => Ok(EaseFunction::CubicOut),
        "cubic_in_out" => Ok(EaseFunction::CubicInOut),
        "sin_in" => Ok(EaseFunction::SinIn),
        "sin_out" => Ok(EaseFunction::SinOut),
        "sin_in_out" => Ok(EaseFunction::SinInOut),
        "exp_in" => Ok(EaseFunction::ExpIn),
        "exp_out" => Ok(EaseFunction::ExpOut),
        "exp_in_out" => Ok(EaseFunction::ExpInOut),
        "circle_in" => Ok(EaseFunction::CircleIn),
        "circle_out" => Ok(EaseFunction::CircleOut),
        "circle_in_out" => Ok(EaseFunction::CircleInOut),
        "elastic_in" => Ok(EaseFunction::ElasticIn),
        "elastic_out" => Ok(EaseFunction::ElasticOut),
        "bounce_out" => Ok(EaseFunction::BounceOut),
        "back_in" => Ok(EaseFunction::BackIn),
        "back_out" => Ok(EaseFunction::BackOut),
        "back_in_out" => Ok(EaseFunction::BackInOut),
        _ => Err(PyValueError::new_err(format!(
            "easing {value:?} is not supported by interpolate_ease"
        ))),
    }
}

#[pyfunction]
fn interpolate_ease(a: f64, b: f64, ease: &str, t: f64) -> PyResult<f64> {
    finite("a", a)?;
    finite("b", b)?;
    finite("t", t)?;
    Ok(d3rs::interpolate::interpolate_ease(
        a,
        b,
        ease_function(ease)?,
    )(t))
}

#[pyfunction]
fn interpolate_matrix(
    py: Python<'_>,
    a: Vec<Vec<f64>>,
    b: Vec<Vec<f64>>,
    t: f64,
) -> PyResult<Vec<Vec<f64>>> {
    for row in a.iter().chain(&b) {
        finite_values(row)?;
    }
    finite("t", t)?;
    Ok(py.allow_threads(|| d3rs::interpolate::interpolate_matrix(&a, &b)(t)))
}

#[pyfunction]
fn interpolate_zoom_vector(
    a: (f64, f64, f64),
    b: (f64, f64, f64),
    t: f64,
) -> PyResult<(f64, f64, f64)> {
    for (name, value) in [
        ("a[0]", a.0),
        ("a[1]", a.1),
        ("a[2]", a.2),
        ("b[0]", b.0),
        ("b[1]", b.1),
        ("b[2]", b.2),
        ("t", t),
    ] {
        finite(name, value)?;
    }
    if a.2 <= 0.0 || b.2 <= 0.0 {
        return Err(PyValueError::new_err("zoom widths must be positive"));
    }
    let [x, y, width] = d3rs::interpolate::interpolate_zoom([a.0, a.1, a.2], [b.0, b.1, b.2])(t);
    Ok((x, y, width))
}

#[pyfunction]
fn interpolate_string(a: &str, b: &str, t: f64) -> PyResult<String> {
    finite("t", t)?;
    Ok(d3rs::interpolate::interpolate_string(a, b)(t))
}

#[pyfunction]
fn interpolate_transform_css(a: &str, b: &str, t: f64) -> PyResult<String> {
    finite("t", t)?;
    Ok(d3rs::interpolate::interpolate_transform_css(a, b)(t))
}

type NativeTransform2D = (f64, f64, f64, f64, f64, f64);

fn transform_from_tuple(
    value: NativeTransform2D,
    name: &str,
) -> PyResult<d3rs::interpolate::Transform2D> {
    let values = [value.0, value.1, value.2, value.3, value.4, value.5];
    for (index, component) in values.into_iter().enumerate() {
        finite(&format!("{name}[{index}]"), component)?;
    }
    Ok(d3rs::interpolate::Transform2D {
        translate_x: value.0,
        translate_y: value.1,
        rotate: value.2,
        scale_x: value.3,
        scale_y: value.4,
        skew_x: value.5,
    })
}

fn transform_tuple(value: d3rs::interpolate::Transform2D) -> NativeTransform2D {
    (
        value.translate_x,
        value.translate_y,
        value.rotate,
        value.scale_x,
        value.scale_y,
        value.skew_x,
    )
}

macro_rules! transform_constructor {
    ($name:ident, $native:ident, $($argument:ident),+ $(,)?) => {
        #[pyfunction]
        fn $name($($argument: f64),+) -> PyResult<NativeTransform2D> {
            $(finite(stringify!($argument), $argument)?;)+
            Ok(transform_tuple(d3rs::interpolate::Transform2D::$native($($argument),+)))
        }
    };
}

#[pyfunction]
fn transform_identity() -> NativeTransform2D {
    transform_tuple(d3rs::interpolate::Transform2D::identity())
}

transform_constructor!(transform_translate, translate, x, y);
transform_constructor!(transform_rotate_deg, rotate_deg, degrees);
transform_constructor!(transform_rotate_rad, rotate_rad, radians);
transform_constructor!(transform_scale, scale, sx, sy);
transform_constructor!(transform_scale_uniform, scale_uniform, scale);
transform_constructor!(transform_skew_x_deg, skew_x_deg, degrees);

#[pyfunction]
fn transform_from_matrix(matrix: (f64, f64, f64, f64, f64, f64)) -> PyResult<NativeTransform2D> {
    let values = [matrix.0, matrix.1, matrix.2, matrix.3, matrix.4, matrix.5];
    for (index, value) in values.into_iter().enumerate() {
        finite(&format!("matrix[{index}]"), value)?;
    }
    Ok(transform_tuple(
        d3rs::interpolate::Transform2D::from_matrix(
            matrix.0, matrix.1, matrix.2, matrix.3, matrix.4, matrix.5,
        ),
    ))
}

#[pyfunction]
fn transform_to_matrix(transform: NativeTransform2D) -> PyResult<NativeTransform2D> {
    let [a, b, c, d, e, f] = transform_from_tuple(transform, "transform")?.to_matrix();
    Ok((a, b, c, d, e, f))
}

#[pyfunction]
fn transform_apply(transform: NativeTransform2D, x: f64, y: f64) -> PyResult<(f64, f64)> {
    finite("x", x)?;
    finite("y", y)?;
    Ok(transform_from_tuple(transform, "transform")?.apply(x, y))
}

#[pyfunction]
fn transform_interpolate(
    a: NativeTransform2D,
    b: NativeTransform2D,
    t: f64,
) -> PyResult<NativeTransform2D> {
    finite("t", t)?;
    let a = transform_from_tuple(a, "a")?;
    let b = transform_from_tuple(b, "b")?;
    Ok(transform_tuple(d3rs::interpolate::interpolate_transform(
        a, b,
    )(t)))
}

#[pyfunction]
fn transform_to_css(transform: NativeTransform2D) -> PyResult<String> {
    Ok(transform_from_tuple(transform, "transform")?.to_css())
}

#[pyfunction]
fn transform_to_svg(transform: NativeTransform2D) -> PyResult<String> {
    Ok(transform_from_tuple(transform, "transform")?.to_svg())
}

#[pyfunction]
fn interpolate_transform_svg(
    a: NativeTransform2D,
    b: NativeTransform2D,
    t: f64,
) -> PyResult<NativeTransform2D> {
    finite("t", t)?;
    let a = [a.0, a.1, a.2, a.3, a.4, a.5];
    let b = [b.0, b.1, b.2, b.3, b.4, b.5];
    for (name, values) in [("a", a), ("b", b)] {
        for (index, value) in values.into_iter().enumerate() {
            finite(&format!("{name}[{index}]"), value)?;
        }
    }
    let [a, b, c, d, e, f] = d3rs::interpolate::interpolate_transform_svg(a, b)(t);
    Ok((a, b, c, d, e, f))
}

fn zoom_view(value: (f64, f64, f64), name: &str) -> PyResult<d3rs::interpolate::zoom::ZoomView> {
    finite(&format!("{name}.cx"), value.0)?;
    finite(&format!("{name}.cy"), value.1)?;
    finite(&format!("{name}.size"), value.2)?;
    if value.2 <= 0.0 {
        return Err(PyValueError::new_err(format!(
            "{name}.size must be positive"
        )));
    }
    Ok(d3rs::interpolate::zoom::ZoomView::new(
        value.0, value.1, value.2,
    ))
}

fn zoom_tuple(value: d3rs::interpolate::zoom::ZoomView) -> (f64, f64, f64) {
    value.as_tuple()
}

fn zoom_rho(value: Option<f64>) -> PyResult<f64> {
    let rho = value.unwrap_or_else(|| 2.0_f64.sqrt());
    finite("rho", rho)?;
    if rho <= 0.0 {
        return Err(PyValueError::new_err("rho must be positive"));
    }
    Ok(rho)
}

#[pyfunction]
fn interpolate_zoom_view(
    a: (f64, f64, f64),
    b: (f64, f64, f64),
    t: f64,
    rho: Option<f64>,
) -> PyResult<(f64, f64, f64)> {
    finite("t", t)?;
    let a = zoom_view(a, "a")?;
    let b = zoom_view(b, "b")?;
    let rho = zoom_rho(rho)?;
    Ok(zoom_tuple(
        d3rs::interpolate::zoom::interpolate_zoom_with_params(
            a,
            b,
            d3rs::interpolate::zoom::ZoomParams { rho },
        )(t),
    ))
}

#[pyfunction]
fn zoom_duration(a: (f64, f64, f64), b: (f64, f64, f64), rho: Option<f64>) -> PyResult<f64> {
    let a = zoom_view(a, "a")?;
    let b = zoom_view(b, "b")?;
    match rho {
        Some(rho) => Ok(d3rs::interpolate::zoom::zoom_duration_with_rho(
            a,
            b,
            zoom_rho(Some(rho))?,
        )),
        None => Ok(d3rs::interpolate::zoom::zoom_duration(a, b)),
    }
}

type NativeDensityGrid = (usize, usize, Vec<f32>, usize);

fn lod_bounds(value: (f64, f64, f64, f64)) -> PyResult<D3LodBounds> {
    D3LodBounds::new(value.0, value.1, value.2, value.3)
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyclass(name = "_DensityPyramid")]
struct PyDensityPyramid {
    inner: D3DensityPyramid,
}

#[pymethods]
impl PyDensityPyramid {
    #[new]
    fn new(
        py: Python<'_>,
        x: Vec<f64>,
        y: Vec<f64>,
        bounds: (f64, f64, f64, f64),
        base_dimension: usize,
    ) -> PyResult<Self> {
        let bounds = lod_bounds(bounds)?;
        let inner = py
            .allow_threads(|| D3DensityPyramid::build(&x, &y, bounds, base_dimension))
            .map_err(|error| PyValueError::new_err(error.to_string()))?;
        Ok(Self { inner })
    }

    fn bounds(&self) -> (f64, f64, f64, f64) {
        let bounds = self.inner.bounds();
        (bounds.x0, bounds.x1, bounds.y0, bounds.y1)
    }

    fn level_count(&self) -> usize {
        self.inner.level_count()
    }

    fn compose(
        &self,
        view: (f64, f64, f64, f64),
        width: usize,
        height: usize,
        max_upsample: usize,
    ) -> PyResult<Option<NativeDensityGrid>> {
        let view = lod_bounds(view)?;
        Ok(self
            .inner
            .compose(view, width, height, max_upsample)
            .map(|grid| (grid.width, grid.height, grid.values, grid.level)))
    }
}

#[pyfunction]
fn lod_m4_indices(
    py: Python<'_>,
    x: Vec<f64>,
    y: Vec<f64>,
    x0: f64,
    x1: f64,
    columns: usize,
) -> Vec<usize> {
    py.allow_threads(|| d3rs::lod::m4_indices(&x, &y, x0, x1, columns))
}

#[pyfunction]
fn lod_m4_point_indices(py: Python<'_>, points: Vec<(f32, f32)>, columns: usize) -> Vec<usize> {
    py.allow_threads(|| d3rs::lod::m4_point_indices(&points, columns))
}

type NativeHierarchyNode = Rc<RefCell<HierarchyNode<usize>>>;

fn hierarchy_nodes(
    parents: Vec<Option<usize>>,
    values: Vec<f64>,
    count: bool,
) -> PyResult<(NativeHierarchyNode, Vec<NativeHierarchyNode>)> {
    if values.is_empty() || parents.len() != values.len() {
        return Err(PyValueError::new_err(
            "hierarchy parents and values must have the same non-zero length",
        ));
    }
    finite_values(&values)?;
    if parents[0].is_some() || parents.iter().skip(1).any(Option::is_none) {
        return Err(PyValueError::new_err(
            "hierarchy must have exactly one root at index 0",
        ));
    }
    let nodes = (0..values.len())
        .map(HierarchyNode::new)
        .collect::<Vec<_>>();
    let mut children = vec![Vec::new(); nodes.len()];
    for (index, parent) in parents.into_iter().enumerate().skip(1) {
        let parent = parent.expect("validated non-root parent");
        if parent >= index {
            return Err(PyValueError::new_err(format!(
                "hierarchy parent at index {index} must refer to an earlier node"
            )));
        }
        children[parent].push(nodes[index].clone());
    }
    for (index, node_children) in children.into_iter().enumerate() {
        if !node_children.is_empty() {
            nodes[index]
                .borrow_mut()
                .set_children(&nodes[index], node_children);
        }
    }
    let root = nodes[0].clone();
    if count {
        HierarchyNode::count(root.clone());
    } else {
        HierarchyNode::try_sum(root.clone(), |index| values[*index])
            .map_err(|error| PyValueError::new_err(error.to_string()))?;
    }
    Ok((root, nodes))
}

#[pyfunction]
fn hierarchy_sum(parents: Vec<Option<usize>>, values: Vec<f64>) -> PyResult<Vec<f64>> {
    let (_, nodes) = hierarchy_nodes(parents, values, false)?;
    Ok(nodes
        .into_iter()
        .map(|node| node.borrow().value.unwrap_or(0.0))
        .collect())
}

#[pyfunction]
fn hierarchy_count(parents: Vec<Option<usize>>) -> PyResult<Vec<f64>> {
    let values = vec![0.0; parents.len()];
    let (_, nodes) = hierarchy_nodes(parents, values, true)?;
    Ok(nodes
        .into_iter()
        .map(|node| node.borrow().value.unwrap_or(0.0))
        .collect())
}

type NativeHierarchyRect = (usize, f64, f64, f64, f64, usize, f64);
type NativeHierarchyCircle = (usize, f64, f64, f64, usize, f64);
type NativeHierarchyPoint = (usize, f64, f64, usize, f64);

struct HierarchySeparationState {
    callback: Py<PyAny>,
    error: Option<PyErr>,
}

thread_local! {
    static HIERARCHY_SEPARATION: RefCell<Option<HierarchySeparationState>> = const { RefCell::new(None) };
}

struct HierarchySeparationGuard;

impl HierarchySeparationGuard {
    fn install(callback: Py<PyAny>) -> PyResult<Self> {
        HIERARCHY_SEPARATION.with(|state| {
            let mut state = state.borrow_mut();
            if state.is_some() {
                return Err(PyRuntimeError::new_err(
                    "hierarchy separation callbacks may not be nested on one thread",
                ));
            }
            *state = Some(HierarchySeparationState {
                callback,
                error: None,
            });
            Ok(Self)
        })
    }

    fn take_error(&self) -> Option<PyErr> {
        HIERARCHY_SEPARATION.with(|state| {
            state
                .borrow_mut()
                .as_mut()
                .and_then(|state| state.error.take())
        })
    }
}

impl Drop for HierarchySeparationGuard {
    fn drop(&mut self) {
        HIERARCHY_SEPARATION.with(|state| *state.borrow_mut() = None);
    }
}

fn hierarchy_python_separation(left: &HierarchyNode<usize>, right: &HierarchyNode<usize>) -> f64 {
    Python::with_gil(|py| {
        let callback = HIERARCHY_SEPARATION.with(|state| {
            state
                .borrow()
                .as_ref()
                .map(|state| state.callback.clone_ref(py))
        });
        let Some(callback) = callback else {
            return f64::NAN;
        };
        match callback
            .bind(py)
            .call1((left.data, right.data))
            .and_then(|value| value.extract::<f64>())
        {
            Ok(value) => value,
            Err(error) => {
                HIERARCHY_SEPARATION.with(|state| {
                    if let Some(state) = state.borrow_mut().as_mut()
                        && state.error.is_none()
                    {
                        state.error = Some(error);
                    }
                });
                f64::NAN
            }
        }
    })
}

fn hierarchy_layout_arguments(width: f64, height: f64, padding: f64) -> PyResult<()> {
    for (name, value) in [("width", width), ("height", height), ("padding", padding)] {
        finite(name, value)?;
        if value < 0.0 {
            return Err(PyValueError::new_err(format!(
                "hierarchy layout {name} must be non-negative"
            )));
        }
    }
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (kind, parents, values, *, width=1.0, height=1.0, padding=0.0, count=false))]
fn hierarchy_rect_layout(
    kind: &str,
    parents: Vec<Option<usize>>,
    values: Vec<f64>,
    width: f64,
    height: f64,
    padding: f64,
    count: bool,
) -> PyResult<Vec<NativeHierarchyRect>> {
    hierarchy_layout_arguments(width, height, padding)?;
    let (root, _) = hierarchy_nodes(parents, values, count)?;
    let rects = match kind {
        "treemap" => TreemapLayout::<usize>::new()
            .size((width, height))
            .padding(padding)
            .try_layout(root),
        "partition" => PartitionLayout::<usize>::new()
            .size((width, height))
            .padding(padding)
            .try_layout(root),
        _ => {
            return Err(PyValueError::new_err(format!(
                "unknown rectangular hierarchy layout {kind:?}"
            )));
        }
    }
    .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(rects
        .into_iter()
        .map(|rect| {
            let index = rect.node.borrow().data;
            (
                index, rect.x0, rect.y0, rect.x1, rect.y1, rect.depth, rect.value,
            )
        })
        .collect())
}

#[pyfunction]
#[pyo3(signature = (parents, values, *, width=1.0, height=1.0, padding=0.0, count=false))]
fn hierarchy_pack_layout(
    parents: Vec<Option<usize>>,
    values: Vec<f64>,
    width: f64,
    height: f64,
    padding: f64,
    count: bool,
) -> PyResult<Vec<NativeHierarchyCircle>> {
    hierarchy_layout_arguments(width, height, padding)?;
    let (root, _) = hierarchy_nodes(parents, values, count)?;
    let circles = PackLayout::<usize>::new()
        .size((width, height))
        .padding(padding)
        .try_layout(root)
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(circles
        .into_iter()
        .map(|circle| {
            let index = circle.node.borrow().data;
            (
                index,
                circle.x,
                circle.y,
                circle.r,
                circle.depth,
                circle.value,
            )
        })
        .collect())
}

#[pyfunction]
#[pyo3(signature = (kind, parents, values, *, width=1.0, height=1.0, node_size=None, count=false, separation=None))]
fn hierarchy_point_layout(
    kind: &str,
    parents: Vec<Option<usize>>,
    values: Vec<f64>,
    width: f64,
    height: f64,
    node_size: Option<(f64, f64)>,
    count: bool,
    separation: Option<Py<PyAny>>,
) -> PyResult<Vec<NativeHierarchyPoint>> {
    hierarchy_layout_arguments(width, height, 0.0)?;
    if let Some((node_width, node_height)) = node_size {
        hierarchy_layout_arguments(node_width, node_height, 0.0)?;
    }
    let (root, nodes) = hierarchy_nodes(parents, values, count)?;
    let separation_guard = separation
        .map(HierarchySeparationGuard::install)
        .transpose()?;
    match kind {
        "tree" => {
            let mut layout = TreeLayout::<usize>::new().size((width, height));
            if let Some(size) = node_size {
                layout = layout.node_size(size);
            }
            if separation_guard.is_some() {
                layout = layout.separation(hierarchy_python_separation);
            }
            let result = layout.try_layout(root);
            if let Some(error) = separation_guard
                .as_ref()
                .and_then(HierarchySeparationGuard::take_error)
            {
                return Err(error);
            }
            result.map_err(|error| PyValueError::new_err(error.to_string()))?;
        }
        "cluster" => {
            let mut layout = ClusterLayout::<usize>::new().size((width, height));
            if let Some(size) = node_size {
                layout = layout.node_size(size);
            }
            if separation_guard.is_some() {
                layout = layout.separation(hierarchy_python_separation);
            }
            let result = layout.try_layout(root);
            if let Some(error) = separation_guard
                .as_ref()
                .and_then(HierarchySeparationGuard::take_error)
            {
                return Err(error);
            }
            result.map_err(|error| PyValueError::new_err(error.to_string()))?;
        }
        _ => {
            return Err(PyValueError::new_err(format!(
                "unknown point hierarchy layout {kind:?}"
            )));
        }
    }
    if let Some(error) = separation_guard
        .as_ref()
        .and_then(HierarchySeparationGuard::take_error)
    {
        return Err(error);
    }
    Ok(nodes
        .into_iter()
        .map(|node| {
            let node = node.borrow();
            (
                node.data,
                node.x,
                node.y,
                node.depth,
                node.value.unwrap_or(0.0),
            )
        })
        .collect())
}

type NativePoint = (f64, f64);
type NativeContour = (f64, Vec<Vec<NativePoint>>);
type NativeContourBand = (f64, f64, Vec<Vec<NativePoint>>);
type NativeContourSegment = (f64, NativePoint, NativePoint);

fn contour_grid(values: &[f64], width: usize, height: usize) -> PyResult<()> {
    if width < 2 || height < 2 {
        return Err(PyValueError::new_err(
            "contour grid dimensions must both be at least two",
        ));
    }
    let length = width
        .checked_mul(height)
        .ok_or_else(|| PyValueError::new_err("contour grid dimensions overflow"))?;
    if values.len() != length {
        return Err(PyValueError::new_err(format!(
            "contour values length {} does not match grid size {width}x{height}",
            values.len()
        )));
    }
    finite_values(values)
}

fn contour_domain(name: &str, domain: Option<(f64, f64)>) -> PyResult<Option<(f64, f64)>> {
    if let Some((start, end)) = domain {
        finite(&format!("{name} domain start"), start)?;
        finite(&format!("{name} domain end"), end)?;
        if start == end {
            return Err(PyValueError::new_err(format!(
                "contour {name} domain endpoints must differ"
            )));
        }
    }
    Ok(domain)
}

#[allow(clippy::too_many_arguments)]
fn configured_contour_generator(
    width: usize,
    height: usize,
    x_domain: Option<(f64, f64)>,
    y_domain: Option<(f64, f64)>,
    x_values: Option<Vec<f64>>,
    y_values: Option<Vec<f64>>,
    upsample_factor: usize,
    x_log_interpolation: bool,
    y_log_interpolation: bool,
) -> PyResult<ContourGenerator> {
    if !(1..=8).contains(&upsample_factor) {
        return Err(PyValueError::new_err(
            "contour upsample factor must be between one and eight",
        ));
    }
    let x_domain = contour_domain("x", x_domain)?;
    let y_domain = contour_domain("y", y_domain)?;
    if let Some(values) = &x_values {
        if values.len() != width {
            return Err(PyValueError::new_err(
                "contour x values length must equal grid width",
            ));
        }
        finite_values(values)?;
        if x_log_interpolation && values.iter().any(|value| *value <= 0.0) {
            return Err(PyValueError::new_err(
                "log-interpolated contour x values must be positive",
            ));
        }
    }
    if let Some(values) = &y_values {
        if values.len() != height {
            return Err(PyValueError::new_err(
                "contour y values length must equal grid height",
            ));
        }
        finite_values(values)?;
        if y_log_interpolation && values.iter().any(|value| *value <= 0.0) {
            return Err(PyValueError::new_err(
                "log-interpolated contour y values must be positive",
            ));
        }
    }

    let mut generator = ContourGenerator::new(width, height).upsample_factor(upsample_factor);
    if let Some((start, end)) = x_domain {
        generator = generator.x(start, end);
    }
    if let Some((start, end)) = y_domain {
        generator = generator.y(start, end);
    }
    if let Some(values) = x_values {
        generator = generator.x_values(values);
    }
    if let Some(values) = y_values {
        generator = generator.y_values(values);
    }
    Ok(generator
        .x_log_interpolation(x_log_interpolation)
        .y_log_interpolation(y_log_interpolation))
}

fn native_ring(ring: ContourRing) -> Vec<NativePoint> {
    ring.points
        .into_iter()
        .map(|point| (point.x, point.y))
        .collect()
}

fn native_contour(contour: Contour) -> NativeContour {
    (
        contour.value,
        contour.coordinates.into_iter().map(native_ring).collect(),
    )
}

fn native_band(band: ContourBand) -> NativeContourBand {
    (
        band.lower,
        band.upper,
        band.polygons.into_iter().map(native_ring).collect(),
    )
}

#[pyfunction]
fn contour_ring_metrics(points: Vec<NativePoint>) -> PyResult<(bool, f64)> {
    if points.iter().any(|(x, y)| !x.is_finite() || !y.is_finite()) {
        return Err(PyValueError::new_err("contour ring points must be finite"));
    }
    let ring = ContourRing::new(
        points
            .into_iter()
            .map(|(x, y)| d3rs::shape::Point::new(x, y))
            .collect(),
    );
    let area = ring
        .try_area()
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok((ring.is_closed(), area))
}

#[pyfunction]
#[pyo3(signature = (
    values, width, height, thresholds, *, x_domain=None, y_domain=None,
    x_values=None, y_values=None, upsample_factor=1,
    x_log_interpolation=false, y_log_interpolation=false
))]
#[allow(clippy::too_many_arguments)]
fn contour_generate(
    py: Python<'_>,
    values: Vec<f64>,
    width: usize,
    height: usize,
    thresholds: Vec<f64>,
    x_domain: Option<(f64, f64)>,
    y_domain: Option<(f64, f64)>,
    x_values: Option<Vec<f64>>,
    y_values: Option<Vec<f64>>,
    upsample_factor: usize,
    x_log_interpolation: bool,
    y_log_interpolation: bool,
) -> PyResult<Vec<NativeContour>> {
    contour_grid(&values, width, height)?;
    finite_values(&thresholds)?;
    py.allow_threads(move || {
        let generator = configured_contour_generator(
            width,
            height,
            x_domain,
            y_domain,
            x_values,
            y_values,
            upsample_factor,
            x_log_interpolation,
            y_log_interpolation,
        )?;
        Ok(generator
            .contours(&values, &thresholds)
            .into_iter()
            .map(native_contour)
            .collect())
    })
}

#[pyfunction]
#[pyo3(signature = (
    values, width, height, thresholds, *, x_domain=None, y_domain=None,
    x_values=None, y_values=None, upsample_factor=1,
    x_log_interpolation=false, y_log_interpolation=false
))]
#[allow(clippy::too_many_arguments)]
fn contour_band_generate(
    py: Python<'_>,
    values: Vec<f64>,
    width: usize,
    height: usize,
    thresholds: Vec<f64>,
    x_domain: Option<(f64, f64)>,
    y_domain: Option<(f64, f64)>,
    x_values: Option<Vec<f64>>,
    y_values: Option<Vec<f64>>,
    upsample_factor: usize,
    x_log_interpolation: bool,
    y_log_interpolation: bool,
) -> PyResult<Vec<NativeContourBand>> {
    contour_grid(&values, width, height)?;
    finite_values(&thresholds)?;
    if thresholds.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(PyValueError::new_err(
            "contour band thresholds must be strictly increasing",
        ));
    }
    py.allow_threads(move || {
        let generator = configured_contour_generator(
            width,
            height,
            x_domain,
            y_domain,
            x_values,
            y_values,
            upsample_factor,
            x_log_interpolation,
            y_log_interpolation,
        )?;
        Ok(generator
            .contour_bands(&values, &thresholds)
            .into_iter()
            .map(native_band)
            .collect())
    })
}

#[pyfunction]
#[pyo3(signature = (
    values, width, height, thresholds, *, x_domain=None, y_domain=None,
    x_values=None, y_values=None, upsample_factor=1,
    x_log_interpolation=false, y_log_interpolation=false
))]
#[allow(clippy::too_many_arguments)]
fn contour_segment_generate(
    py: Python<'_>,
    values: Vec<f64>,
    width: usize,
    height: usize,
    thresholds: Vec<f64>,
    x_domain: Option<(f64, f64)>,
    y_domain: Option<(f64, f64)>,
    x_values: Option<Vec<f64>>,
    y_values: Option<Vec<f64>>,
    upsample_factor: usize,
    x_log_interpolation: bool,
    y_log_interpolation: bool,
) -> PyResult<Vec<NativeContourSegment>> {
    contour_grid(&values, width, height)?;
    finite_values(&thresholds)?;
    py.allow_threads(move || {
        let generator = configured_contour_generator(
            width,
            height,
            x_domain,
            y_domain,
            x_values,
            y_values,
            upsample_factor,
            x_log_interpolation,
            y_log_interpolation,
        )?;
        Ok(generator
            .contour_segments(&values, &thresholds)
            .into_iter()
            .map(|segment| {
                (
                    segment.value,
                    (segment.start.x, segment.start.y),
                    (segment.end.x, segment.end.y),
                )
            })
            .collect())
    })
}

fn density_kernel_type(kernel: &str) -> PyResult<KernelType> {
    match kernel {
        "gaussian" => Ok(KernelType::Gaussian),
        "epanechnikov" => Ok(KernelType::Epanechnikov),
        _ => Err(PyValueError::new_err(format!(
            "unknown density kernel {kernel:?}"
        ))),
    }
}

fn density_arguments(
    width: usize,
    height: usize,
    x_domain: (f64, f64),
    y_domain: (f64, f64),
    bandwidth: f64,
) -> PyResult<()> {
    if width == 0 || height == 0 {
        return Err(PyValueError::new_err(
            "density grid dimensions must be non-zero",
        ));
    }
    finite("density bandwidth", bandwidth)?;
    if bandwidth <= 0.0 {
        return Err(PyValueError::new_err("density bandwidth must be positive"));
    }
    contour_domain("density x", Some(x_domain))?;
    contour_domain("density y", Some(y_domain))?;
    Ok(())
}

#[pyfunction]
fn density_kernel(kind: &str, x: f64, bandwidth: f64) -> PyResult<f64> {
    finite("density kernel x", x)?;
    finite("density bandwidth", bandwidth)?;
    if bandwidth <= 0.0 {
        return Err(PyValueError::new_err("density bandwidth must be positive"));
    }
    match density_kernel_type(kind)? {
        KernelType::Gaussian => Ok(gaussian_kernel(x, bandwidth)),
        KernelType::Epanechnikov => Ok(epanechnikov_kernel(x, bandwidth)),
    }
}

#[pyfunction]
#[pyo3(signature = (
    points, *, width=100, height=100, x_domain=(0.0, 1.0),
    y_domain=(0.0, 1.0), bandwidth=0.1, kernel="gaussian"
))]
#[allow(clippy::too_many_arguments)]
fn density_estimate(
    py: Python<'_>,
    points: Vec<NativePoint>,
    width: usize,
    height: usize,
    x_domain: (f64, f64),
    y_domain: (f64, f64),
    bandwidth: f64,
    kernel: &str,
) -> PyResult<Vec<f64>> {
    density_arguments(width, height, x_domain, y_domain, bandwidth)?;
    if points.iter().any(|(x, y)| !x.is_finite() || !y.is_finite()) {
        return Err(PyValueError::new_err("density points must be finite"));
    }
    let estimator = DensityEstimator::new()
        .size(width, height)
        .x(x_domain.0, x_domain.1)
        .y(y_domain.0, y_domain.1)
        .bandwidth(bandwidth)
        .kernel(density_kernel_type(kernel)?);
    py.allow_threads(move || estimator.try_estimate(&points))
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
#[pyo3(signature = (
    points, *, width=100, height=100, x_domain=(0.0, 1.0),
    y_domain=(0.0, 1.0), bandwidth=0.1, kernel="gaussian"
))]
#[allow(clippy::too_many_arguments)]
fn density_estimate_weighted(
    py: Python<'_>,
    points: Vec<(f64, f64, f64)>,
    width: usize,
    height: usize,
    x_domain: (f64, f64),
    y_domain: (f64, f64),
    bandwidth: f64,
    kernel: &str,
) -> PyResult<Vec<f64>> {
    density_arguments(width, height, x_domain, y_domain, bandwidth)?;
    if points.iter().any(|(x, y, weight)| {
        !x.is_finite() || !y.is_finite() || !weight.is_finite() || *weight < 0.0
    }) {
        return Err(PyValueError::new_err(
            "weighted density points and non-negative weights must be finite",
        ));
    }
    let estimator = DensityEstimator::new()
        .size(width, height)
        .x(x_domain.0, x_domain.1)
        .y(y_domain.0, y_domain.1)
        .bandwidth(bandwidth)
        .kernel(density_kernel_type(kernel)?);
    py.allow_threads(move || estimator.try_estimate_weighted(&points))
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
#[pyo3(signature = (points, *, width, height, bandwidth))]
fn density_2d_auto(
    py: Python<'_>,
    points: Vec<NativePoint>,
    width: usize,
    height: usize,
    bandwidth: f64,
) -> PyResult<(Vec<f64>, usize, usize)> {
    if points.iter().any(|(x, y)| !x.is_finite() || !y.is_finite()) {
        return Err(PyValueError::new_err("density points must be finite"));
    }
    finite("density bandwidth", bandwidth)?;
    py.allow_threads(move || try_density_2d(&points, width, height, bandwidth))
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
fn contour_threshold_sturges(minimum: f64, maximum: f64, count: usize) -> PyResult<Vec<f64>> {
    finite("contour threshold minimum", minimum)?;
    finite("contour threshold maximum", maximum)?;
    if minimum >= maximum {
        return Err(PyValueError::new_err(
            "contour threshold minimum must be less than maximum",
        ));
    }
    if count == 0 {
        return Err(PyValueError::new_err(
            "contour threshold count must be non-zero",
        ));
    }
    Ok(contour_thresholds_sturges(minimum, maximum, count))
}

#[pyfunction]
fn contour_threshold_scott(values: Vec<f64>, minimum: f64, maximum: f64) -> PyResult<Vec<f64>> {
    finite_values(&values)?;
    if values.is_empty() {
        return Err(PyValueError::new_err(
            "contour threshold values must not be empty",
        ));
    }
    contour_threshold_sturges(minimum, maximum, values.len())?;
    Ok(threshold_scott(&values, minimum, maximum))
}

#[pyfunction]
fn contour_threshold_freedman_diaconis(
    values: Vec<f64>,
    minimum: f64,
    maximum: f64,
) -> PyResult<Vec<f64>> {
    finite_values(&values)?;
    if values.is_empty() {
        return Err(PyValueError::new_err(
            "contour threshold values must not be empty",
        ));
    }
    contour_threshold_sturges(minimum, maximum, values.len())?;
    Ok(threshold_freedman_diaconis(&values, minimum, maximum))
}

fn finite_points(name: &str, points: &[NativePoint]) -> PyResult<()> {
    for (index, &(x, y)) in points.iter().enumerate() {
        if !x.is_finite() {
            return Err(PyValueError::new_err(format!(
                "{name} x coordinate at index {index} must be finite"
            )));
        }
        if !y.is_finite() {
            return Err(PyValueError::new_err(format!(
                "{name} y coordinate at index {index} must be finite"
            )));
        }
    }
    Ok(())
}

#[pyfunction]
fn polygon_area_value(points: Vec<NativePoint>) -> PyResult<f64> {
    finite_points("polygon", &points)?;
    Ok(d3_polygon_area(&points))
}

#[pyfunction]
fn polygon_area_signed_value(points: Vec<NativePoint>) -> PyResult<f64> {
    finite_points("polygon", &points)?;
    Ok(d3_polygon_area_signed(&points))
}

#[pyfunction]
fn polygon_centroid_value(points: Vec<NativePoint>) -> PyResult<NativePoint> {
    finite_points("polygon", &points)?;
    Ok(d3_polygon_centroid(&points))
}

#[pyfunction]
fn polygon_contains_value(points: Vec<NativePoint>, point: NativePoint) -> PyResult<bool> {
    finite_points("polygon", &points)?;
    finite_points("polygon query", &[point])?;
    Ok(d3_polygon_contains(&points, point))
}

#[pyfunction]
fn polygon_length_value(points: Vec<NativePoint>) -> PyResult<f64> {
    finite_points("polygon", &points)?;
    Ok(d3_polygon_length(&points))
}

#[pyfunction]
fn polygon_hull_value(points: Vec<NativePoint>) -> PyResult<Vec<NativePoint>> {
    finite_points("polygon", &points)?;
    Ok(d3_polygon_hull(&points))
}

type NativeVoronoiSnapshot = (
    (f64, f64, f64, f64),
    Vec<NativePoint>,
    Vec<Option<Vec<NativePoint>>>,
    String,
    String,
    Vec<Option<String>>,
    Vec<Vec<usize>>,
);
type NativeDelaunaySnapshot = (
    Vec<(usize, usize, usize)>,
    Vec<usize>,
    Vec<NativePoint>,
    Vec<(usize, usize)>,
    String,
    String,
    NativeVoronoiSnapshot,
);

fn native_voronoi_snapshot(
    delaunay: &Delaunay,
    bounds: Option<(f64, f64, f64, f64)>,
) -> PyResult<NativeVoronoiSnapshot> {
    let bounds = bounds.map(|(x0, y0, x1, y1)| [x0, y0, x1, y1]);
    let voronoi = delaunay
        .try_voronoi(bounds)
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let [x0, y0, x1, y1] = voronoi.bounds();
    let cells = (0..voronoi.cell_count())
        .map(|index| voronoi.cell_polygon(index))
        .collect::<Vec<_>>();
    let cell_paths = (0..voronoi.cell_count())
        .map(|index| voronoi.render_cell_to_path(index))
        .collect::<Vec<_>>();
    let neighbors = (0..voronoi.cell_count())
        .map(|index| voronoi.neighbors(index).collect())
        .collect::<Vec<_>>();
    Ok((
        (x0, y0, x1, y1),
        voronoi.bounds_polygon().to_vec(),
        cells,
        voronoi.render_to_path(),
        voronoi.render_bounds_to_path(),
        cell_paths,
        neighbors,
    ))
}

#[pyfunction]
fn delaunay_snapshot(
    py: Python<'_>,
    points: Vec<NativePoint>,
    bounds: Option<(f64, f64, f64, f64)>,
) -> PyResult<NativeDelaunaySnapshot> {
    finite_points("delaunay point", &points)?;
    if let Some(bounds) = bounds {
        finite_points(
            "voronoi bound",
            &[(bounds.0, bounds.1), (bounds.2, bounds.3)],
        )?;
    }
    py.allow_threads(move || {
        let delaunay =
            Delaunay::try_new(&points).map_err(|error| PyValueError::new_err(error.to_string()))?;
        let triangles = delaunay.triangles().collect();
        let hull = delaunay.hull().to_vec();
        let hull_polygon = delaunay.hull_polygon();
        let edges = delaunay.edges().collect();
        let path = delaunay.render_to_path();
        let hull_path = delaunay.render_hull_to_path();
        let voronoi = native_voronoi_snapshot(&delaunay, bounds)?;
        Ok((
            triangles,
            hull,
            hull_polygon,
            edges,
            path,
            hull_path,
            voronoi,
        ))
    })
}

#[pyfunction]
#[pyo3(signature = (points, x, y, *, start=None, radius=None))]
fn delaunay_find(
    py: Python<'_>,
    points: Vec<NativePoint>,
    x: f64,
    y: f64,
    start: Option<usize>,
    radius: Option<f64>,
) -> PyResult<Option<usize>> {
    finite_points("delaunay point", &points)?;
    finite("delaunay query x", x)?;
    finite("delaunay query y", y)?;
    if let Some(radius) = radius {
        finite("delaunay radius", radius)?;
        if radius < 0.0 {
            return Err(PyValueError::new_err(
                "delaunay radius must be non-negative",
            ));
        }
    }
    py.allow_threads(move || {
        let delaunay =
            Delaunay::try_new(&points).map_err(|error| PyValueError::new_err(error.to_string()))?;
        match radius {
            Some(radius) => delaunay
                .try_find_within_radius(x, y, radius)
                .map_err(|error| PyValueError::new_err(error.to_string())),
            None => delaunay
                .try_find(x, y, start)
                .map_err(|error| PyValueError::new_err(error.to_string())),
        }
    })
}

type NativeSimulationNode = (usize, f64, f64, f64, f64, Option<f64>, Option<f64>);
type NativeCollideForce = (f64, f64, usize, Option<Vec<f64>>);
type NativeLinkForce = (Vec<(usize, usize)>, Option<f64>, f64, usize);

#[pyfunction]
#[pyo3(signature = (
    nodes, ticks, *, alpha=1.0, alpha_min=0.001,
    alpha_decay=0.02276277904418933, alpha_target=0.0, velocity_decay=0.6,
    centers=Vec::new(), x_forces=Vec::new(), y_forces=Vec::new(),
    radial_forces=Vec::new(), collide_forces=Vec::new(),
    many_body_forces=Vec::new(), link_forces=Vec::new()
))]
#[allow(clippy::too_many_arguments)]
fn force_simulate(
    py: Python<'_>,
    nodes: Vec<NativeSimulationNode>,
    ticks: usize,
    alpha: f64,
    alpha_min: f64,
    alpha_decay: f64,
    alpha_target: f64,
    velocity_decay: f64,
    centers: Vec<(f64, f64)>,
    x_forces: Vec<(f64, f64)>,
    y_forces: Vec<(f64, f64)>,
    radial_forces: Vec<(f64, f64, f64, f64)>,
    collide_forces: Vec<NativeCollideForce>,
    many_body_forces: Vec<(f64, f64, f64, f64)>,
    link_forces: Vec<NativeLinkForce>,
) -> PyResult<(Vec<NativeSimulationNode>, f64)> {
    for (name, value) in [
        ("alpha", alpha),
        ("alpha_min", alpha_min),
        ("alpha_decay", alpha_decay),
        ("alpha_target", alpha_target),
        ("velocity_decay", velocity_decay),
    ] {
        finite(name, value)?;
    }
    if alpha_min < 0.0 || alpha_decay < 0.0 || velocity_decay < 0.0 {
        return Err(PyValueError::new_err(
            "alpha_min, alpha_decay, and velocity_decay must be non-negative",
        ));
    }

    py.allow_threads(move || {
        let node_count = nodes.len();
        let mut simulation_nodes = Vec::with_capacity(node_count);
        for (index, x, y, vx, vy, fx, fy) in nodes {
            let node = SimulationNode::try_new(index, x, y)
                .map_err(|error| PyValueError::new_err(error.to_string()))?;
            for (name, value) in [("vx", vx), ("vy", vy)] {
                if !value.is_finite() {
                    return Err(PyValueError::new_err(format!(
                        "simulation node {name} must be finite"
                    )));
                }
            }
            if fx.is_some_and(|value| !value.is_finite())
                || fy.is_some_and(|value| !value.is_finite())
            {
                return Err(PyValueError::new_err(
                    "fixed simulation node coordinates must be finite",
                ));
            }
            {
                let mut node_mut = node.borrow_mut();
                node_mut.vx = vx;
                node_mut.vy = vy;
                node_mut.fx = fx;
                node_mut.fy = fy;
            }
            simulation_nodes.push(node);
        }

        let mut simulation = Simulation::try_new(simulation_nodes)
            .map_err(|error| PyValueError::new_err(error.to_string()))?;
        simulation.alpha = alpha;
        simulation.alpha_min = alpha_min;
        simulation.alpha_decay = alpha_decay;
        simulation.alpha_target = alpha_target;
        simulation.velocity_decay = velocity_decay;

        for (x, y) in centers {
            simulation = simulation.force(Box::new(
                ForceCenter::try_new(x, y)
                    .map_err(|error| PyValueError::new_err(error.to_string()))?,
            ));
        }
        for (target, strength) in x_forces {
            simulation = simulation.force(Box::new(
                ForceX::try_new(target)
                    .and_then(|force| force.try_strength(strength))
                    .map_err(|error| PyValueError::new_err(error.to_string()))?,
            ));
        }
        for (target, strength) in y_forces {
            simulation = simulation.force(Box::new(
                ForceY::try_new(target)
                    .and_then(|force| force.try_strength(strength))
                    .map_err(|error| PyValueError::new_err(error.to_string()))?,
            ));
        }
        for (radius, x, y, strength) in radial_forces {
            simulation = simulation.force(Box::new(
                ForceRadial::try_with_center(radius, x, y)
                    .and_then(|force| force.try_strength(strength))
                    .map_err(|error| PyValueError::new_err(error.to_string()))?,
            ));
        }
        for (radius, strength, iterations, radii) in collide_forces {
            let mut force = ForceCollide::try_with_radius(radius)
                .and_then(|force| force.try_strength(strength))
                .map_err(|error| PyValueError::new_err(error.to_string()))?
                .iterations(iterations);
            if let Some(radii) = radii {
                force = force
                    .try_radii_for_nodes(radii, node_count)
                    .map_err(|error| PyValueError::new_err(error.to_string()))?;
            }
            force
                .validate()
                .map_err(|error| PyValueError::new_err(error.to_string()))?;
            simulation = simulation.force(Box::new(force));
        }
        for (strength, theta, distance_min, distance_max) in many_body_forces {
            let force = ForceManyBody::try_new()
                .and_then(|force| force.try_strength(strength))
                .and_then(|force| force.try_theta(theta))
                .and_then(|force| force.try_distance_min(distance_min))
                .and_then(|force| force.try_distance_max(distance_max))
                .map_err(|error| PyValueError::new_err(error.to_string()))?;
            simulation = simulation.force(Box::new(force));
        }
        for (links, strength, distance, iterations) in link_forces {
            let mut force = ForceLink::try_new_for_nodes(links, node_count)
                .and_then(|force| force.try_distance(distance))
                .map_err(|error| PyValueError::new_err(error.to_string()))?
                .iterations(iterations);
            if let Some(strength) = strength {
                force = force
                    .try_strength(strength)
                    .map_err(|error| PyValueError::new_err(error.to_string()))?;
            }
            simulation = simulation.force(Box::new(force));
        }

        for _ in 0..ticks {
            simulation
                .try_tick()
                .map_err(|error| PyValueError::new_err(error.to_string()))?;
        }
        let result = simulation
            .nodes
            .iter()
            .map(|node| {
                let node = node.borrow();
                (
                    node.index, node.x, node.y, node.vx, node.vy, node.fx, node.fy,
                )
            })
            .collect();
        Ok((result, simulation.alpha))
    })
}

type NativeArcDatum = (f64, f64, f64, f64, f64, f64);
type NativeArcResult = (String, NativePoint, Vec<NativePoint>);
type NativeSymbolResult = (String, Vec<NativePoint>, f64);
type NativePieSlice = (usize, f64, NativeArcDatum);

fn arc_datum(value: NativeArcDatum) -> ArcDatum {
    ArcDatum::new()
        .inner_radius(value.0)
        .outer_radius(value.1)
        .start_angle(value.2)
        .end_angle(value.3)
        .corner_radius(value.4)
        .pad_angle(value.5)
}

fn native_arc_datum(value: ArcDatum) -> NativeArcDatum {
    (
        value.inner_radius,
        value.outer_radius,
        value.start_angle,
        value.end_angle,
        value.corner_radius,
        value.pad_angle,
    )
}

#[pyfunction]
#[pyo3(signature = (datum, *, center=(0.0, 0.0), segments=32, checked=true))]
fn shape_arc(
    datum: NativeArcDatum,
    center: NativePoint,
    segments: usize,
    checked: bool,
) -> PyResult<NativeArcResult> {
    let datum = arc_datum(datum);
    let arc = ShapeArc::new().center(center.0, center.1);
    let path = if checked {
        arc.try_path_string(&datum)
            .map_err(|error| PyValueError::new_err(error.to_string()))?
    } else {
        arc.path_string(&datum)
    };
    let centroid = datum.centroid();
    let points = if checked {
        try_arc_points(&datum, segments, center.0, center.1)
            .map_err(|error| PyValueError::new_err(error.to_string()))?
    } else {
        arc_points(&datum, segments, center.0, center.1)
    }
    .into_iter()
    .map(|point| (point.x, point.y))
    .collect();
    Ok((path, (centroid.x + center.0, centroid.y + center.1), points))
}

fn symbol_type(kind: &str) -> PyResult<SymbolType> {
    match kind {
        "circle" => Ok(SymbolType::Circle),
        "cross" => Ok(SymbolType::Cross),
        "diamond" => Ok(SymbolType::Diamond),
        "square" => Ok(SymbolType::Square),
        "star" => Ok(SymbolType::Star),
        "triangle" => Ok(SymbolType::Triangle),
        "triangle_down" => Ok(SymbolType::TriangleDown),
        "triangle_left" => Ok(SymbolType::TriangleLeft),
        "triangle_right" => Ok(SymbolType::TriangleRight),
        "wye" => Ok(SymbolType::Wye),
        _ => Err(PyValueError::new_err(format!(
            "unknown symbol type {kind:?}"
        ))),
    }
}

#[pyfunction]
#[pyo3(signature = (kind, size, *, center=(0.0, 0.0), checked=true))]
fn shape_symbol(
    kind: &str,
    size: f64,
    center: NativePoint,
    checked: bool,
) -> PyResult<NativeSymbolResult> {
    let symbol = ShapeSymbol::new(symbol_type(kind)?, size);
    let path = if checked {
        symbol
            .try_generate_at(center.0, center.1)
            .map_err(|error| PyValueError::new_err(error.to_string()))?
    } else {
        symbol.generate_at(center.0, center.1)
    };
    let raw_points = if checked {
        symbol
            .try_points()
            .map_err(|error| PyValueError::new_err(error.to_string()))?
    } else {
        symbol.points()
    };
    let radius = if checked {
        symbol
            .try_radius()
            .map_err(|error| PyValueError::new_err(error.to_string()))?
    } else {
        raw_points
            .iter()
            .map(|point| point.x.hypot(point.y))
            .fold(0.0_f64, f64::max)
    };
    let points = raw_points
        .into_iter()
        .map(|point| (point.x + center.0, point.y + center.1))
        .collect();
    Ok((path.to_svg_string(), points, radius))
}

#[pyfunction]
#[pyo3(signature = (kind, source, target, *, checked=true))]
fn shape_link(
    kind: &str,
    source: NativePoint,
    target: NativePoint,
    checked: bool,
) -> PyResult<String> {
    let link = ShapeLink::from_points(source, target);
    let direction = match kind {
        "horizontal" | "vertical" => None,
        "step_horizontal" => Some(LinkDirection::Horizontal),
        "step_vertical" => Some(LinkDirection::Vertical),
        "step_radial" => Some(LinkDirection::Radial),
        _ => {
            return Err(PyValueError::new_err(format!("unknown link type {kind:?}")));
        }
    };
    if checked {
        match direction {
            Some(direction) => try_link_step(&link, direction),
            None if kind == "horizontal" => try_link_horizontal(&link),
            None => try_link_vertical(&link),
        }
        .map_err(|error| PyValueError::new_err(error.to_string()))
    } else {
        Ok(match direction {
            Some(direction) => shape_link_step(&link, direction),
            None if kind == "horizontal" => shape_link_horizontal(&link),
            None => shape_link_vertical(&link),
        })
    }
}

#[pyfunction]
#[pyo3(signature = (
    source_angle, source_radius, target_angle, target_radius, center, *, checked=true
))]
fn shape_radial_link(
    source_angle: f64,
    source_radius: f64,
    target_angle: f64,
    target_radius: f64,
    center: NativePoint,
    checked: bool,
) -> PyResult<(String, NativePoint, NativePoint)> {
    let link = RadialLink::new(source_angle, source_radius, target_angle, target_radius);
    let cartesian = if checked {
        link.try_to_cartesian(center.0, center.1)
            .map_err(|error| PyValueError::new_err(error.to_string()))?
    } else {
        link.to_cartesian(center.0, center.1)
    };
    let path = if checked {
        try_link_radial(&link, center.0, center.1)
            .map_err(|error| PyValueError::new_err(error.to_string()))?
    } else {
        shape_link_radial(&link, center.0, center.1)
    };
    Ok((
        path,
        (cartesian.source_x, cartesian.source_y),
        (cartesian.target_x, cartesian.target_y),
    ))
}

#[pyfunction]
#[pyo3(signature = (
    values, *, start_angle=0.0, end_angle=std::f64::consts::TAU,
    pad_angle=0.0, inner_radius=0.0, outer_radius=100.0,
    corner_radius=0.0, sort=false, descending=true, checked=true
))]
#[allow(clippy::too_many_arguments)]
fn shape_pie(
    values: Vec<f64>,
    start_angle: f64,
    end_angle: f64,
    pad_angle: f64,
    inner_radius: f64,
    outer_radius: f64,
    corner_radius: f64,
    sort: bool,
    descending: bool,
    checked: bool,
) -> PyResult<Vec<NativePieSlice>> {
    let pie = ShapePie::new()
        .start_angle(start_angle)
        .end_angle(end_angle)
        .pad_angle(pad_angle)
        .inner_radius(inner_radius)
        .outer_radius(outer_radius)
        .corner_radius(corner_radius)
        .sort(sort)
        .sort_descending(descending);
    let slices = if checked {
        pie.try_generate(&values, |value| *value)
            .map_err(|error| PyValueError::new_err(error.to_string()))?
    } else {
        pie.generate(&values, |value| *value)
    };
    Ok(slices
        .into_iter()
        .map(|slice| (slice.index, slice.value, native_arc_datum(slice.arc)))
        .collect())
}

type NativeStackSeries = (String, Vec<f64>, Vec<NativePoint>, usize);

fn stack_order(value: &str) -> PyResult<StackOrder> {
    match value {
        "none" => Ok(StackOrder::None),
        "ascending" => Ok(StackOrder::Ascending),
        "descending" => Ok(StackOrder::Descending),
        "appearance" => Ok(StackOrder::Appearance),
        "inside_out" => Ok(StackOrder::InsideOut),
        "reverse" => Ok(StackOrder::Reverse),
        _ => Err(PyValueError::new_err(format!(
            "unknown stack order {value:?}"
        ))),
    }
}

fn stack_offset(value: &str) -> PyResult<StackOffset> {
    match value {
        "none" => Ok(StackOffset::None),
        "expand" => Ok(StackOffset::Expand),
        "diverging" => Ok(StackOffset::Diverging),
        "silhouette" => Ok(StackOffset::Silhouette),
        "wiggle" => Ok(StackOffset::Wiggle),
        _ => Err(PyValueError::new_err(format!(
            "unknown stack offset {value:?}"
        ))),
    }
}

#[pyfunction]
fn shape_stack(
    py: Python<'_>,
    data: Vec<Vec<f64>>,
    keys: Vec<String>,
    order: &str,
    offset: &str,
    checked: bool,
) -> PyResult<Vec<NativeStackSeries>> {
    let order = stack_order(order)?;
    let offset = stack_offset(offset)?;
    let result = py.allow_threads(move || {
        let stack = ShapeStack::new().keys(keys).order(order).offset(offset);
        if checked {
            stack.try_generate(&data)
        } else {
            Ok(stack.generate(&data))
        }
    });
    Ok(result
        .map_err(|error| PyValueError::new_err(error.to_string()))?
        .into_iter()
        .map(|series: StackSeries| {
            (
                series.key,
                series.data,
                series
                    .values
                    .into_iter()
                    .map(|value| (value[0], value[1]))
                    .collect(),
                series.index,
            )
        })
        .collect())
}

fn shape_curve(value: &str, parameter: Option<f64>) -> PyResult<ShapeCurve> {
    if let Some(parameter) = parameter {
        finite("curve parameter", parameter)?;
    }
    let required_parameter = || {
        parameter
            .ok_or_else(|| PyValueError::new_err(format!("curve {value:?} requires a parameter")))
    };
    let no_parameter = || {
        if parameter.is_some() {
            Err(PyValueError::new_err(format!(
                "curve {value:?} does not accept a parameter"
            )))
        } else {
            Ok(())
        }
    };
    match value {
        "linear" => no_parameter().map(|()| ShapeCurve::Linear),
        "step" => no_parameter().map(|()| ShapeCurve::Step),
        "step_before" => no_parameter().map(|()| ShapeCurve::StepBefore),
        "step_after" => no_parameter().map(|()| ShapeCurve::StepAfter),
        "basis" => no_parameter().map(|()| ShapeCurve::Basis),
        "basis_closed" => no_parameter().map(|()| ShapeCurve::BasisClosed),
        "basis_open" => no_parameter().map(|()| ShapeCurve::BasisOpen),
        "bundle" => Ok(ShapeCurve::Bundle {
            beta: required_parameter()?,
        }),
        "cardinal" => Ok(ShapeCurve::cardinal(required_parameter()?)),
        "cardinal_closed" => Ok(ShapeCurve::CardinalClosed {
            tension: required_parameter()?,
        }),
        "cardinal_open" => Ok(ShapeCurve::CardinalOpen {
            tension: required_parameter()?,
        }),
        "catmull_rom" => Ok(ShapeCurve::catmull_rom(required_parameter()?)),
        "catmull_rom_closed" => Ok(ShapeCurve::CatmullRomClosed {
            alpha: required_parameter()?,
        }),
        "catmull_rom_open" => Ok(ShapeCurve::CatmullRomOpen {
            alpha: required_parameter()?,
        }),
        "monotone_x" => no_parameter().map(|()| ShapeCurve::MonotoneX),
        "monotone_y" => no_parameter().map(|()| ShapeCurve::MonotoneY),
        "natural" => no_parameter().map(|()| ShapeCurve::Natural),
        _ => Err(PyValueError::new_err(format!(
            "unknown curve type {value:?}"
        ))),
    }
}

#[pyfunction]
fn shape_curve_interpolate(
    py: Python<'_>,
    kind: &str,
    parameter: Option<f64>,
    points: Vec<NativePoint>,
) -> PyResult<Vec<NativePoint>> {
    let curve = shape_curve(kind, parameter)?;
    for (index, (x, y)) in points.iter().copied().enumerate() {
        finite(&format!("points[{index}].x"), x)?;
        finite(&format!("points[{index}].y"), y)?;
    }
    let points = points
        .into_iter()
        .map(|(x, y)| d3rs::shape::Point::new(x, y))
        .collect::<Vec<_>>();
    Ok(py
        .allow_threads(move || curve.interpolate(&points))
        .into_iter()
        .map(|point| (point.x, point.y))
        .collect())
}

#[pyfunction]
#[pyo3(signature = (angle, radius, cx, cy, *, checked=true))]
fn radial_point_to_cartesian(
    angle: f64,
    radius: f64,
    cx: f64,
    cy: f64,
    checked: bool,
) -> PyResult<NativePoint> {
    let point = RadialPoint::new(angle, radius);
    if checked {
        point
            .try_to_cartesian(cx, cy)
            .map_err(|error| PyValueError::new_err(error.to_string()))
    } else {
        Ok(point.to_cartesian(cx, cy))
    }
}

#[pyfunction]
#[pyo3(signature = (x, y, cx, cy, *, checked=true))]
fn radial_point_from_cartesian(
    x: f64,
    y: f64,
    cx: f64,
    cy: f64,
    checked: bool,
) -> PyResult<NativePoint> {
    if checked {
        finite("x", x)?;
        finite("y", y)?;
        finite("cx", cx)?;
        finite("cy", cy)?;
    }
    let point = RadialPoint::from_cartesian(x, y, cx, cy);
    Ok((point.angle, point.radius))
}

fn radial_points(values: Vec<NativePoint>) -> Vec<RadialPoint> {
    values
        .into_iter()
        .map(|(angle, radius)| RadialPoint::new(angle, radius))
        .collect()
}

#[pyfunction]
#[pyo3(signature = (points, cx, cy, kind, parameter=None, closed=false, checked=true))]
fn radial_line_path(
    py: Python<'_>,
    points: Vec<NativePoint>,
    cx: f64,
    cy: f64,
    kind: &str,
    parameter: Option<f64>,
    closed: bool,
    checked: bool,
) -> PyResult<String> {
    let curve = shape_curve(kind, parameter)?;
    let points = radial_points(points);
    py.allow_threads(move || {
        let config = RadialLineConfig::new(cx, cy).curve(curve).closed(closed);
        if checked {
            try_radial_line(&points, &config)
        } else {
            Ok(shape_radial_line(&points, &config))
        }
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
#[pyo3(signature = (points, cx, cy, inner_radius, kind, parameter=None, checked=true))]
fn radial_area_path(
    py: Python<'_>,
    points: Vec<NativePoint>,
    cx: f64,
    cy: f64,
    inner_radius: f64,
    kind: &str,
    parameter: Option<f64>,
    checked: bool,
) -> PyResult<String> {
    let curve = shape_curve(kind, parameter)?;
    let points = radial_points(points);
    py.allow_threads(move || {
        let config = RadialAreaConfig::new(cx, cy)
            .inner_radius(inner_radius)
            .curve(curve);
        if checked {
            try_radial_area(&points, &config)
        } else {
            Ok(shape_radial_area(&points, &config))
        }
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
#[pyo3(signature = (cx, cy, radii, *, checked=true))]
fn polar_grid_circle_paths(
    py: Python<'_>,
    cx: f64,
    cy: f64,
    radii: Vec<f64>,
    checked: bool,
) -> PyResult<Vec<String>> {
    py.allow_threads(move || {
        if checked {
            try_polar_grid_circles(cx, cy, &radii)
        } else {
            Ok(shape_polar_grid_circles(cx, cy, &radii))
        }
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
#[pyo3(signature = (cx, cy, outer_radius, angles, inner_radius, *, checked=true))]
fn polar_grid_ray_paths(
    py: Python<'_>,
    cx: f64,
    cy: f64,
    outer_radius: f64,
    angles: Vec<f64>,
    inner_radius: f64,
    checked: bool,
) -> PyResult<Vec<String>> {
    py.allow_threads(move || {
        if checked {
            try_polar_grid_rays(cx, cy, outer_radius, &angles, inner_radius)
        } else {
            Ok(shape_polar_grid_rays(
                cx,
                cy,
                outer_radius,
                &angles,
                inner_radius,
            ))
        }
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

type NativePathCommand = (String, Vec<f64>);
type NativePathAnalysis = (String, Option<(f64, f64, f64, f64)>, Vec<NativePoint>);

fn native_path_commands(path: &NativeShapePath) -> Vec<NativePathCommand> {
    path.commands()
        .iter()
        .map(|command| match *command {
            PathCommand::MoveTo { x, y } => ("move_to".to_string(), vec![x, y]),
            PathCommand::LineTo { x, y } => ("line_to".to_string(), vec![x, y]),
            PathCommand::HorizontalLineTo { x } => ("horizontal_line_to".to_string(), vec![x]),
            PathCommand::VerticalLineTo { y } => ("vertical_line_to".to_string(), vec![y]),
            PathCommand::ClosePath => ("close_path".to_string(), Vec::new()),
            PathCommand::QuadraticCurveTo { x1, y1, x, y } => {
                ("quadratic_curve_to".to_string(), vec![x1, y1, x, y])
            }
            PathCommand::CubicCurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => ("cubic_curve_to".to_string(), vec![x1, y1, x2, y2, x, y]),
            PathCommand::Arc {
                x,
                y,
                radius,
                start_angle,
                end_angle,
                anticlockwise,
            } => (
                "arc".to_string(),
                vec![
                    x,
                    y,
                    radius,
                    start_angle,
                    end_angle,
                    if anticlockwise { 1.0 } else { 0.0 },
                ],
            ),
            PathCommand::EllipticalArc {
                rx,
                ry,
                x_axis_rotation,
                large_arc,
                sweep,
                x,
                y,
            } => (
                "elliptical_arc".to_string(),
                vec![
                    rx,
                    ry,
                    x_axis_rotation,
                    if large_arc { 1.0 } else { 0.0 },
                    if sweep { 1.0 } else { 0.0 },
                    x,
                    y,
                ],
            ),
            PathCommand::Rect {
                x,
                y,
                width,
                height,
            } => ("rect".to_string(), vec![x, y, width, height]),
        })
        .collect()
}

fn path_command_arity(kind: &str) -> PyResult<usize> {
    match kind {
        "move_to" | "line_to" => Ok(2),
        "horizontal_line_to" | "vertical_line_to" => Ok(1),
        "close_path" => Ok(0),
        "quadratic_curve_to" | "rect" => Ok(4),
        "cubic_curve_to" | "arc" => Ok(6),
        "elliptical_arc" => Ok(7),
        _ => Err(PyValueError::new_err(format!(
            "unknown path command {kind:?}"
        ))),
    }
}

fn path_bool(command_index: usize, field: &str, value: f64) -> PyResult<bool> {
    if value == 0.0 {
        Ok(false)
    } else if value == 1.0 {
        Ok(true)
    } else {
        Err(PyValueError::new_err(format!(
            "path command {command_index} {field} must be 0 or 1"
        )))
    }
}

fn build_native_path(commands: Vec<NativePathCommand>) -> PyResult<NativeShapePath> {
    let mut builder = ShapePathBuilder::new();
    for (index, (kind, values)) in commands.into_iter().enumerate() {
        let expected = path_command_arity(&kind)?;
        if values.len() != expected {
            return Err(PyValueError::new_err(format!(
                "path command {index} {kind:?} has {} values, expected {expected}",
                values.len()
            )));
        }
        for (value_index, value) in values.iter().copied().enumerate() {
            finite(&format!("path command {index} value[{value_index}]"), value)?;
        }
        builder = match kind.as_str() {
            "move_to" => builder.move_to(values[0], values[1]),
            "line_to" => builder.line_to(values[0], values[1]),
            "horizontal_line_to" => builder.horizontal_line_to(values[0]),
            "vertical_line_to" => builder.vertical_line_to(values[0]),
            "close_path" => builder.close_path(),
            "quadratic_curve_to" => {
                builder.quadratic_curve_to(values[0], values[1], values[2], values[3])
            }
            "cubic_curve_to" => builder.cubic_curve_to(
                values[0], values[1], values[2], values[3], values[4], values[5],
            ),
            "arc" => {
                if values[2] < 0.0 {
                    return Err(PyValueError::new_err(format!(
                        "path command {index} radius must be non-negative"
                    )));
                }
                builder.arc(
                    values[0],
                    values[1],
                    values[2],
                    values[3],
                    values[4],
                    path_bool(index, "anticlockwise", values[5])?,
                )
            }
            "elliptical_arc" => builder.elliptical_arc(
                values[0],
                values[1],
                values[2],
                path_bool(index, "large_arc", values[3])?,
                path_bool(index, "sweep", values[4])?,
                values[5],
                values[6],
            ),
            "rect" => builder.rect(values[0], values[1], values[2], values[3]),
            _ => unreachable!("path command kind was validated"),
        };
    }
    Ok(builder.build())
}

#[pyfunction]
fn shape_path_analyze(
    py: Python<'_>,
    commands: Vec<NativePathCommand>,
    tolerance: f64,
) -> PyResult<NativePathAnalysis> {
    finite("tolerance", tolerance)?;
    if tolerance <= 0.0 {
        return Err(PyValueError::new_err("tolerance must be positive"));
    }
    let path = build_native_path(commands)?;
    Ok(py.allow_threads(move || {
        let svg = path.to_svg_string();
        let bounds = path.bounds();
        let points = path
            .flatten(tolerance)
            .into_iter()
            .map(|point| (point.x, point.y))
            .collect();
        (svg, bounds, points)
    }))
}

#[pyfunction]
fn shape_point_distance(a: NativePoint, b: NativePoint) -> PyResult<f64> {
    finite("a.x", a.0)?;
    finite("a.y", a.1)?;
    finite("b.x", b.0)?;
    finite("b.y", b.1)?;
    Ok(ShapePoint::new(a.0, a.1).distance(&ShapePoint::new(b.0, b.1)))
}

#[pyfunction]
fn shape_point_lerp(a: NativePoint, b: NativePoint, t: f64) -> PyResult<NativePoint> {
    finite("a.x", a.0)?;
    finite("a.y", a.1)?;
    finite("b.x", b.0)?;
    finite("b.y", b.1)?;
    finite("t", t)?;
    let point = ShapePoint::new(a.0, a.1).lerp(&ShapePoint::new(b.0, b.1), t);
    Ok((point.x, point.y))
}

#[pyfunction]
#[pyo3(signature = (top, bottom, defined, kind, parameter, *, checked=true))]
fn shape_area_generate(
    py: Python<'_>,
    top: Vec<NativePoint>,
    bottom: Vec<NativePoint>,
    defined: Vec<bool>,
    kind: &str,
    parameter: Option<f64>,
    checked: bool,
) -> PyResult<Vec<NativePathCommand>> {
    if top.len() != bottom.len() || top.len() != defined.len() {
        return Err(PyValueError::new_err(format!(
            "area arrays have mismatched lengths: top={}, bottom={}, defined={}",
            top.len(),
            bottom.len(),
            defined.len()
        )));
    }
    let curve = shape_curve(kind, parameter)?;
    let path = py.allow_threads(move || {
        let indices = (0..top.len()).collect::<Vec<_>>();
        let top_x = top.iter().map(|point| point.0).collect::<Vec<_>>();
        let top_y = top.iter().map(|point| point.1).collect::<Vec<_>>();
        let bottom_x = bottom.iter().map(|point| point.0).collect::<Vec<_>>();
        let bottom_y = bottom.iter().map(|point| point.1).collect::<Vec<_>>();
        let area = || {
            ShapeArea::new()
                .x1(move |index: &usize| top_x[*index])
                .y1(move |index: &usize| top_y[*index])
                .x0(move |index: &usize| bottom_x[*index])
                .y0(move |index: &usize| bottom_y[*index])
                .defined(move |index: &usize| defined[*index])
                .curve(curve)
        };
        if checked {
            area().try_generate(&indices)
        } else {
            Ok(area().generate(&indices))
        }
    });
    path.map(|path| native_path_commands(&path))
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

type NativeSimpleArea = (Vec<NativePoint>, Vec<NativePathCommand>);

#[pyfunction]
#[pyo3(signature = (x, y0, y1, *, checked=true))]
fn shape_simple_area(
    py: Python<'_>,
    x: Vec<f64>,
    y0: Vec<f64>,
    y1: Vec<f64>,
    checked: bool,
) -> PyResult<NativeSimpleArea> {
    let result = py.allow_threads(move || {
        let area = SimpleArea::new(x, y0, y1);
        let points = if checked {
            area.try_points()?
        } else {
            area.points()
        };
        let path = if checked {
            area.try_path()?
        } else {
            area.path()
        };
        Ok::<_, d3rs::shape::AreaGenerationError>((points, path))
    });
    let (points, path) = result.map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok((
        points.into_iter().map(|point| (point.x, point.y)).collect(),
        native_path_commands(&path),
    ))
}

type NativeChordPart = (usize, f64, f64, f64);
type NativeChordValue = (NativeChordPart, NativeChordPart);
type NativeChordLayoutResult = (Vec<NativeChordValue>, Vec<NativeChordPart>);

fn ascending_f64(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

fn descending_f64(a: f64, b: f64) -> Ordering {
    b.partial_cmp(&a).unwrap_or(Ordering::Equal)
}

fn chord_sort_order(value: &str) -> PyResult<Option<fn(f64, f64) -> Ordering>> {
    match value {
        "none" => Ok(None),
        "ascending" => Ok(Some(ascending_f64)),
        "descending" => Ok(Some(descending_f64)),
        _ => Err(PyValueError::new_err(format!(
            "unknown chord sort order {value:?}"
        ))),
    }
}

fn native_chord_part(value: NativeChordSubgroup) -> NativeChordPart {
    (value.index, value.start_angle, value.end_angle, value.value)
}

#[pyfunction]
#[pyo3(signature = (matrix, pad_angle=0.0, sort_groups="none", sort_subgroups="none", sort_chords="none"))]
fn chord_layout(
    py: Python<'_>,
    matrix: Vec<Vec<f64>>,
    pad_angle: f64,
    sort_groups: &str,
    sort_subgroups: &str,
    sort_chords: &str,
) -> PyResult<NativeChordLayoutResult> {
    finite("pad_angle", pad_angle)?;
    if pad_angle < 0.0 {
        return Err(PyValueError::new_err("pad_angle must be non-negative"));
    }
    let group_order = chord_sort_order(sort_groups)?;
    let subgroup_order = chord_sort_order(sort_subgroups)?;
    let chord_order = chord_sort_order(sort_chords)?;
    let result = py.allow_threads(move || {
        let mut layout = NativeChordLayout::new().pad_angle(pad_angle);
        if let Some(order) = group_order {
            layout = layout.sort_groups(order);
        }
        if let Some(order) = subgroup_order {
            layout = layout.sort_subgroups(order);
        }
        if let Some(order) = chord_order {
            layout = layout.sort_chords(order);
        }
        layout.try_compute(&matrix)
    });
    let result = result.map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok((
        result
            .chords
            .into_iter()
            .map(|chord| {
                (
                    native_chord_part(chord.source),
                    native_chord_part(chord.target),
                )
            })
            .collect(),
        result
            .groups
            .into_iter()
            .map(|group| (group.index, group.start_angle, group.end_angle, group.value))
            .collect(),
    ))
}

#[pyfunction]
fn chord_ribbon_path(
    py: Python<'_>,
    chord: NativeChordValue,
    radius: f64,
    center: NativePoint,
) -> PyResult<Vec<NativePathCommand>> {
    finite("radius", radius)?;
    finite("center.x", center.0)?;
    finite("center.y", center.1)?;
    if radius < 0.0 {
        return Err(PyValueError::new_err("radius must be non-negative"));
    }
    for (name, part) in [("source", chord.0), ("target", chord.1)] {
        finite(&format!("{name}.start_angle"), part.1)?;
        finite(&format!("{name}.end_angle"), part.2)?;
        finite(&format!("{name}.value"), part.3)?;
        if part.3 < 0.0 {
            return Err(PyValueError::new_err(format!(
                "{name}.value must be non-negative"
            )));
        }
    }
    let chord = NativeChord {
        source: NativeChordSubgroup {
            index: chord.0.0,
            start_angle: chord.0.1,
            end_angle: chord.0.2,
            value: chord.0.3,
        },
        target: NativeChordSubgroup {
            index: chord.1.0,
            start_angle: chord.1.1,
            end_angle: chord.1.2,
            value: chord.1.3,
        },
    };
    let path = py.allow_threads(move || {
        NativeRibbonGenerator::new(radius)
            .center(center.0, center.1)
            .generate_path(&chord)
    });
    Ok(native_path_commands(&path))
}

type NativeDragUpdate = (
    String,
    u64,
    NativePoint,
    NativePoint,
    NativePoint,
    NativePoint,
    NativePoint,
    f64,
    bool,
);
type NativeDragError = (u8, String, f64, String, u64, u64, String);

fn native_drag_update(update: D3DragUpdate) -> NativeDragUpdate {
    let phase = match update.phase {
        D3DragPhase::Start => "start",
        D3DragPhase::Drag => "drag",
        D3DragPhase::End => "end",
        D3DragPhase::Cancel => "cancel",
    };
    (
        phase.to_owned(),
        update.pointer_id,
        (update.start.x, update.start.y),
        (update.previous.x, update.previous.y),
        (update.current.x, update.current.y),
        (update.delta.dx, update.delta.dy),
        (update.total_delta.dx, update.total_delta.dy),
        update.distance,
        update.exceeds_click_distance,
    )
}

fn native_drag_error(error: D3DragError) -> NativeDragError {
    let (kind, axis, value, reason, active, received) = match error {
        D3DragError::NonFiniteCoordinate { axis, value } => {
            (0, axis.to_owned(), value, String::new(), 0, 0)
        }
        D3DragError::InvalidExtent { reason } => (1, String::new(), 0.0, reason.to_owned(), 0, 0),
        D3DragError::InvalidClickDistance(value) => (2, String::new(), value, String::new(), 0, 0),
        D3DragError::AlreadyActive { pointer_id } => {
            (3, String::new(), 0.0, String::new(), pointer_id, 0)
        }
        D3DragError::Inactive => (4, String::new(), 0.0, String::new(), 0, 0),
        D3DragError::PointerMismatch { active, received } => {
            (5, String::new(), 0.0, String::new(), active, received)
        }
    };
    let message = format!("{error:?}");
    (kind, axis, value, reason, active, received, message)
}

type NativeDragResult = (Option<NativeDragUpdate>, Option<NativeDragError>);

#[pyclass(name = "_DragState", unsendable)]
struct NativeDragState {
    state: D3DragState,
}

#[pymethods]
impl NativeDragState {
    #[new]
    #[pyo3(signature = (click_distance=0.0, extent=None))]
    fn new(click_distance: f64, extent: Option<(f64, f64, f64, f64)>) -> PyResult<Self> {
        let mut config = D3DragConfig::default()
            .with_click_distance(click_distance)
            .map_err(|error| PyValueError::new_err(format!("{error:?}")))?;
        if let Some((x0, y0, x1, y1)) = extent {
            let extent = D3DragExtent::try_new(x0, y0, x1, y1)
                .map_err(|error| PyValueError::new_err(format!("{error:?}")))?;
            config = config.with_extent(extent);
        }
        Ok(Self {
            state: D3DragState::with_config(config)
                .map_err(|error| PyValueError::new_err(format!("{error:?}")))?,
        })
    }

    fn config(&self) -> (f64, Option<(f64, f64, f64, f64)>) {
        let config = self.state.config();
        (
            config.click_distance,
            config
                .extent
                .map(|extent| (extent.x0, extent.y0, extent.x1, extent.y1)),
        )
    }

    fn start(&mut self, pointer_id: u64, x: f64, y: f64) -> NativeDragResult {
        match self.state.start(pointer_id, x, y) {
            Ok(update) => (Some(native_drag_update(update)), None),
            Err(error) => (None, Some(native_drag_error(error))),
        }
    }

    fn drag(&mut self, pointer_id: u64, x: f64, y: f64) -> NativeDragResult {
        match self.state.drag(pointer_id, x, y) {
            Ok(update) => (Some(native_drag_update(update)), None),
            Err(error) => (None, Some(native_drag_error(error))),
        }
    }

    fn end(&mut self, pointer_id: u64, x: f64, y: f64) -> NativeDragResult {
        match self.state.end(pointer_id, x, y) {
            Ok(update) => (Some(native_drag_update(update)), None),
            Err(error) => (None, Some(native_drag_error(error))),
        }
    }

    fn cancel(&mut self, pointer_id: u64) -> NativeDragResult {
        match self.state.cancel(pointer_id) {
            Ok(update) => (Some(native_drag_update(update)), None),
            Err(error) => (None, Some(native_drag_error(error))),
        }
    }

    fn is_active(&self) -> bool {
        self.state.is_active()
    }

    fn active_pointer_id(&self) -> Option<u64> {
        self.state.active_pointer_id()
    }

    fn current_update(&self) -> Option<NativeDragUpdate> {
        self.state.current_update().map(native_drag_update)
    }
}

#[derive(Clone)]
enum NativeTimerResource {
    Timer(D3Timer),
    Interval(D3TimerInterval),
    Timeout(D3Timeout),
}

fn timer_callback(
    callback: Py<PyAny>,
    error: Arc<std::sync::Mutex<Option<String>>>,
) -> impl FnMut(f64) -> bool + Send + 'static {
    move |elapsed| {
        Python::with_gil(|py| {
            match callback
                .bind(py)
                .call1((elapsed,))
                .and_then(|result| result.extract::<bool>())
            {
                Ok(keep_running) => keep_running,
                Err(callback_error) => {
                    *error
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner()) =
                        Some(callback_error.to_string());
                    false
                }
            }
        })
    }
}

#[pyclass(name = "_TimerResource", unsendable)]
struct NativeTimerHandle {
    resource: NativeTimerResource,
    callback_error: Arc<std::sync::Mutex<Option<String>>>,
}

#[pymethods]
impl NativeTimerHandle {
    #[new]
    #[pyo3(signature = (kind, callback, delay=None, time=None, interval_ms=None))]
    fn new(
        kind: &str,
        callback: Py<PyAny>,
        delay: Option<f64>,
        time: Option<f64>,
        interval_ms: Option<f64>,
    ) -> PyResult<Self> {
        for (field, value) in [
            ("delay", delay),
            ("time", time),
            ("interval_ms", interval_ms),
        ] {
            if let Some(value) = value {
                finite(field, value)?;
            }
        }
        if delay.is_some_and(|value| value < 0.0) || interval_ms.is_some_and(|value| value <= 0.0) {
            return Err(PyValueError::new_err(
                "timer delays must be non-negative and intervals must be positive",
            ));
        }
        let callback_error = Arc::new(std::sync::Mutex::new(None));
        let resource = match kind {
            "timer" => NativeTimerResource::Timer(D3Timer::new(
                timer_callback(callback, callback_error.clone()),
                delay,
                time,
            )),
            "interval" => NativeTimerResource::Interval(D3TimerInterval::new(
                timer_callback(callback, callback_error.clone()),
                interval_ms.unwrap_or(1.0),
                time,
            )),
            "timeout" => {
                let callback_error_for_callback = callback_error.clone();
                NativeTimerResource::Timeout(D3Timeout::new(
                    move |elapsed| {
                        Python::with_gil(|py| {
                            if let Err(error) = callback.bind(py).call1((elapsed,)) {
                                *callback_error_for_callback
                                    .lock()
                                    .unwrap_or_else(|poisoned| poisoned.into_inner()) =
                                    Some(error.to_string());
                            }
                        });
                    },
                    delay.unwrap_or(0.0),
                    time,
                ))
            }
            _ => {
                return Err(PyValueError::new_err(format!(
                    "unknown timer resource kind {kind:?}"
                )));
            }
        };
        Ok(Self {
            resource,
            callback_error,
        })
    }

    fn stop(&self) {
        match &self.resource {
            NativeTimerResource::Timer(timer) => timer.stop(),
            NativeTimerResource::Interval(timer) => timer.stop(),
            NativeTimerResource::Timeout(timer) => timer.stop(),
        }
    }

    fn is_stopped(&self) -> bool {
        match &self.resource {
            NativeTimerResource::Timer(timer) => timer.is_stopped(),
            NativeTimerResource::Interval(timer) => timer.is_stopped(),
            NativeTimerResource::Timeout(timer) => timer.is_stopped(),
        }
    }

    fn restart(
        &mut self,
        callback: Py<PyAny>,
        delay: Option<f64>,
        time: Option<f64>,
    ) -> PyResult<()> {
        let NativeTimerResource::Timer(timer) = &mut self.resource else {
            return Err(PyValueError::new_err("restart is only available on Timer"));
        };
        *self
            .callback_error
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
        timer.restart(
            timer_callback(callback, self.callback_error.clone()),
            delay,
            time,
        );
        Ok(())
    }

    fn id(&self) -> PyResult<u64> {
        match &self.resource {
            NativeTimerResource::Timer(timer) => Ok(timer.id()),
            _ => Err(PyValueError::new_err("id is only available on Timer")),
        }
    }

    fn delay(&self) -> PyResult<f64> {
        match &self.resource {
            NativeTimerResource::Timer(timer) => Ok(timer.delay()),
            _ => Err(PyValueError::new_err("delay is only available on Timer")),
        }
    }

    fn start_time(&self) -> PyResult<f64> {
        match &self.resource {
            NativeTimerResource::Timer(timer) => Ok(timer.start_time()),
            _ => Err(PyValueError::new_err(
                "start_time is only available on Timer",
            )),
        }
    }

    fn join(&self, py: Python<'_>) {
        let resource = self.resource.clone();
        py.allow_threads(move || match resource {
            NativeTimerResource::Timer(timer) => timer.join(),
            NativeTimerResource::Interval(timer) => timer.join(),
            NativeTimerResource::Timeout(timer) => timer.join(),
        });
    }

    fn try_join(&self, py: Python<'_>, timeout_ms: f64) -> PyResult<bool> {
        finite("timeout_ms", timeout_ms)?;
        if timeout_ms < 0.0 {
            return Err(PyValueError::new_err("timeout_ms must be non-negative"));
        }
        let timeout = std::time::Duration::from_secs_f64(timeout_ms / 1000.0);
        Ok(py.allow_threads(|| match &self.resource {
            NativeTimerResource::Timer(timer) => timer.try_join(timeout),
            NativeTimerResource::Interval(timer) => timer.try_join(timeout),
            NativeTimerResource::Timeout(timer) => timer.try_join(timeout),
        }))
    }

    fn take_callback_error(&self) -> Option<String> {
        self.callback_error
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
    }
}

#[pyfunction]
fn timer_now() -> f64 {
    d3rs::timer::now()
}

#[pyfunction]
fn timer_set_now(value: f64) -> PyResult<()> {
    finite("value", value)?;
    d3rs::timer::set_now(value);
    Ok(())
}

#[pyfunction]
fn timer_flush() {
    d3rs::timer::timer_flush();
}

fn transition_ease(value: &str) -> PyResult<fn(f64) -> f64> {
    use d3rs::ease::*;
    match value {
        "linear" => Ok(ease_linear),
        "quad_in" => Ok(ease_quad_in),
        "quad_out" => Ok(ease_quad_out),
        "quad_in_out" => Ok(ease_quad_in_out),
        "cubic_in" => Ok(ease_cubic_in),
        "cubic_out" => Ok(ease_cubic_out),
        "cubic_in_out" => Ok(ease_cubic_in_out),
        "sin_in" => Ok(ease_sin_in),
        "sin_out" => Ok(ease_sin_out),
        "sin_in_out" => Ok(ease_sin_in_out),
        "exp_in" => Ok(ease_exp_in),
        "exp_out" => Ok(ease_exp_out),
        "exp_in_out" => Ok(ease_exp_in_out),
        "circle_in" => Ok(ease_circle_in),
        "circle_out" => Ok(ease_circle_out),
        "circle_in_out" => Ok(ease_circle_in_out),
        "elastic_in" => Ok(ease_elastic_in),
        "elastic_out" => Ok(ease_elastic_out),
        "elastic_in_out" => Ok(ease_elastic_in_out),
        "back_in" => Ok(ease_back_in),
        "back_out" => Ok(ease_back_out),
        "back_in_out" => Ok(ease_back_in_out),
        "bounce_in" => Ok(ease_bounce_in),
        "bounce_out" => Ok(ease_bounce_out),
        "bounce_in_out" => Ok(ease_bounce_in_out),
        _ => Err(PyValueError::new_err(format!(
            "unknown transition easing {value:?}"
        ))),
    }
}

fn transition_state_name(state: D3TransitionState) -> &'static str {
    match state {
        D3TransitionState::Pending => "pending",
        D3TransitionState::Active => "active",
        D3TransitionState::Ended => "ended",
        D3TransitionState::Interrupted => "interrupted",
    }
}

#[pyclass(name = "_TransitionState", unsendable)]
struct NativeTransitionState {
    transition: D3Transition,
}

#[pymethods]
impl NativeTransitionState {
    #[new]
    fn new(
        duration: f64,
        delay: f64,
        ease: &str,
        name: Option<String>,
        start: f64,
        end: f64,
    ) -> PyResult<Self> {
        for (field, value) in [
            ("duration", duration),
            ("delay", delay),
            ("start", start),
            ("end", end),
        ] {
            finite(field, value)?;
        }
        if duration < 0.0 || delay < 0.0 {
            return Err(PyValueError::new_err(
                "transition duration and delay must be non-negative",
            ));
        }
        let mut transition = D3Transition::new()
            .duration(duration)
            .delay(delay)
            .ease(transition_ease(ease)?)
            .from_to(start, end);
        if let Some(name) = name {
            transition = transition.name(name);
        }
        Ok(Self { transition })
    }

    fn tick(&mut self, dt: f64) -> PyResult<(f64, String, String)> {
        finite("dt", dt)?;
        if dt < 0.0 {
            return Err(PyValueError::new_err("dt must be non-negative"));
        }
        let before = transition_state_name(self.transition.state()).to_owned();
        let value = self.transition.tick(dt);
        let after = transition_state_name(self.transition.state()).to_owned();
        Ok((value, before, after))
    }

    fn value(&self) -> f64 {
        self.transition.value()
    }

    fn state(&self) -> String {
        transition_state_name(self.transition.state()).to_owned()
    }

    fn is_complete(&self) -> bool {
        self.transition.is_complete()
    }

    fn interrupt(&mut self) -> (String, String) {
        let before = transition_state_name(self.transition.state()).to_owned();
        self.transition.interrupt();
        let after = transition_state_name(self.transition.state()).to_owned();
        (before, after)
    }

    fn reset(&mut self) {
        self.transition.reset();
    }
}

#[pyclass(name = "_LcgRng", unsendable)]
struct PyLcgRng {
    inner: LcgRng,
}

#[pymethods]
impl PyLcgRng {
    #[new]
    #[pyo3(signature = (seed=None))]
    fn new(seed: Option<u64>) -> Self {
        Self {
            inner: seed.map_or_else(LcgRng::default_seed, LcgRng::new),
        }
    }

    fn next_f64(&self) -> f64 {
        self.inner.next_f64()
    }

    fn next_u64(&self, max: u64) -> PyResult<u64> {
        if max == 0 {
            return Err(PyValueError::new_err("max must be positive"));
        }
        Ok(self.inner.next_u64(max))
    }
}

fn random_finite(name: &str, value: f64) -> PyResult<()> {
    finite(name, value)
}

fn random_non_negative(name: &str, value: f64) -> PyResult<()> {
    random_finite(name, value)?;
    if value < 0.0 {
        return Err(PyValueError::new_err(format!(
            "{name} must be non-negative"
        )));
    }
    Ok(())
}

fn random_positive(name: &str, value: f64) -> PyResult<()> {
    random_finite(name, value)?;
    if value <= 0.0 {
        return Err(PyValueError::new_err(format!("{name} must be positive")));
    }
    Ok(())
}

#[pyclass(name = "_RandomUniform", unsendable)]
struct PyRandomUniform {
    inner: RandomUniform,
}

#[pymethods]
impl PyRandomUniform {
    #[new]
    #[pyo3(signature = (min, max, seed=None))]
    fn new(min: f64, max: f64, seed: Option<u64>) -> PyResult<Self> {
        random_finite("min", min)?;
        random_finite("max", max)?;
        Ok(Self {
            inner: seed.map_or_else(
                || RandomUniform::new(min, max),
                |seed| RandomUniform::with_seed(min, max, seed),
            ),
        })
    }

    fn sample(&self) -> f64 {
        self.inner.sample()
    }
}

#[pyclass(name = "_RandomNormal", unsendable)]
struct PyRandomNormal {
    inner: RandomNormal,
}

#[pymethods]
impl PyRandomNormal {
    #[new]
    #[pyo3(signature = (mean, std_dev, seed=None))]
    fn new(mean: f64, std_dev: f64, seed: Option<u64>) -> PyResult<Self> {
        random_finite("mean", mean)?;
        random_non_negative("std_dev", std_dev)?;
        Ok(Self {
            inner: seed.map_or_else(
                || RandomNormal::new(mean, std_dev),
                |seed| RandomNormal::with_seed(mean, std_dev, seed),
            ),
        })
    }

    fn sample(&self) -> f64 {
        self.inner.sample()
    }
}

#[pyclass(name = "_RandomLogNormal", unsendable)]
struct PyRandomLogNormal {
    inner: RandomLogNormal,
}

#[pymethods]
impl PyRandomLogNormal {
    #[new]
    #[pyo3(signature = (mu, sigma, seed=None))]
    fn new(mu: f64, sigma: f64, seed: Option<u64>) -> PyResult<Self> {
        random_finite("mu", mu)?;
        random_non_negative("sigma", sigma)?;
        Ok(Self {
            inner: seed.map_or_else(
                || RandomLogNormal::new(mu, sigma),
                |seed| RandomLogNormal::with_seed(mu, sigma, seed),
            ),
        })
    }

    fn sample(&self) -> f64 {
        self.inner.sample()
    }
}

#[pyclass(name = "_RandomExponential", unsendable)]
struct PyRandomExponential {
    inner: RandomExponential,
}

#[pymethods]
impl PyRandomExponential {
    #[new]
    #[pyo3(signature = (lambda_, seed=None))]
    fn new(lambda_: f64, seed: Option<u64>) -> PyResult<Self> {
        random_positive("lambda", lambda_)?;
        Ok(Self {
            inner: seed.map_or_else(
                || RandomExponential::new(lambda_),
                |seed| RandomExponential::with_seed(lambda_, seed),
            ),
        })
    }

    fn sample(&self) -> f64 {
        self.inner.sample()
    }
}

#[pyclass(name = "_RandomBernoulli", unsendable)]
struct PyRandomBernoulli {
    inner: RandomBernoulli,
}

#[pymethods]
impl PyRandomBernoulli {
    #[new]
    #[pyo3(signature = (p, seed=None))]
    fn new(p: f64, seed: Option<u64>) -> PyResult<Self> {
        random_finite("p", p)?;
        Ok(Self {
            inner: seed.map_or_else(
                || RandomBernoulli::new(p),
                |seed| RandomBernoulli::with_seed(p, seed),
            ),
        })
    }

    fn sample(&self) -> bool {
        self.inner.sample()
    }

    fn sample_int(&self) -> u32 {
        self.inner.sample_int()
    }
}

#[pyclass(name = "_RandomPoisson", unsendable)]
struct PyRandomPoisson {
    inner: RandomPoisson,
}

#[pymethods]
impl PyRandomPoisson {
    #[new]
    #[pyo3(signature = (lambda_, seed=None))]
    fn new(lambda_: f64, seed: Option<u64>) -> PyResult<Self> {
        random_non_negative("lambda", lambda_)?;
        Ok(Self {
            inner: seed.map_or_else(
                || RandomPoisson::new(lambda_),
                |seed| RandomPoisson::with_seed(lambda_, seed),
            ),
        })
    }

    fn sample(&self) -> u64 {
        self.inner.sample()
    }
}

#[pyclass(name = "_RandomIrwinHall", unsendable)]
struct PyRandomIrwinHall {
    inner: RandomIrwinHall,
}

#[pymethods]
impl PyRandomIrwinHall {
    #[new]
    #[pyo3(signature = (n, seed=None))]
    fn new(n: usize, seed: Option<u64>) -> Self {
        Self {
            inner: seed.map_or_else(
                || RandomIrwinHall::new(n),
                |seed| RandomIrwinHall::with_seed(n, seed),
            ),
        }
    }

    fn sample(&self) -> f64 {
        self.inner.sample()
    }
}

#[pyclass(name = "_RandomBates", unsendable)]
struct PyRandomBates {
    inner: RandomBates,
}

#[pymethods]
impl PyRandomBates {
    #[new]
    #[pyo3(signature = (n, seed=None))]
    fn new(n: usize, seed: Option<u64>) -> PyResult<Self> {
        if n == 0 {
            return Err(PyValueError::new_err("n must be positive"));
        }
        Ok(Self {
            inner: seed.map_or_else(
                || RandomBates::new(n),
                |seed| RandomBates::with_seed(n, seed),
            ),
        })
    }

    fn sample(&self) -> f64 {
        self.inner.sample()
    }
}

fn finite_geo_point(name: &str, point: NativePoint) -> PyResult<()> {
    finite(&format!("{name}.longitude"), point.0)?;
    finite(&format!("{name}.latitude"), point.1)
}

fn finite_geo_points(points: &[NativePoint]) -> PyResult<()> {
    for (index, point) in points.iter().copied().enumerate() {
        finite_geo_point(&format!("coordinates[{index}]"), point)?;
    }
    Ok(())
}

#[pyfunction]
fn geo_radians_value(degrees: f64) -> PyResult<f64> {
    finite("degrees", degrees)?;
    Ok(d3rs::geo::radians(degrees))
}

#[pyfunction]
fn geo_degrees_value(radians: f64) -> PyResult<f64> {
    finite("radians", radians)?;
    Ok(d3rs::geo::degrees(radians))
}

#[pyfunction]
fn geo_distance_value(a: NativePoint, b: NativePoint) -> PyResult<f64> {
    finite_geo_point("a", a)?;
    finite_geo_point("b", b)?;
    Ok(d3_geo_distance(a.0, a.1, b.0, b.1))
}

#[pyfunction]
fn geo_length_value(points: Vec<NativePoint>) -> PyResult<f64> {
    finite_geo_points(&points)?;
    Ok(d3_geo_length(&points))
}

#[pyfunction]
fn geo_interpolate_value(a: NativePoint, b: NativePoint, t: f64) -> PyResult<NativePoint> {
    finite_geo_point("a", a)?;
    finite_geo_point("b", b)?;
    finite("t", t)?;
    Ok(d3_geo_interpolate(a.0, a.1, b.0, b.1, t))
}

#[pyfunction]
fn geo_area_value(points: Vec<NativePoint>) -> PyResult<f64> {
    finite_geo_points(&points)?;
    Ok(d3_geo_area(&points))
}

#[pyfunction]
fn geo_bounds_value(points: Vec<NativePoint>) -> PyResult<(NativePoint, NativePoint)> {
    finite_geo_points(&points)?;
    Ok(d3_geo_bounds(&points))
}

#[pyfunction]
fn geo_centroid_value(points: Vec<NativePoint>) -> PyResult<NativePoint> {
    finite_geo_points(&points)?;
    Ok(d3_geo_centroid(&points))
}

#[pyfunction]
fn geo_contains_value(points: Vec<NativePoint>, point: NativePoint) -> PyResult<bool> {
    finite_geo_points(&points)?;
    finite_geo_point("point", point)?;
    Ok(d3_geo_contains(&points, point.0, point.1))
}

type NativeExtent = (NativePoint, NativePoint);
type NativeGraticuleResult = (Vec<Vec<NativePoint>>, Vec<NativePoint>);

fn validate_geo_extent(name: &str, extent: NativeExtent) -> PyResult<()> {
    finite_geo_point(&format!("{name}[0]"), extent.0)?;
    finite_geo_point(&format!("{name}[1]"), extent.1)?;
    if extent.0.0 >= extent.1.0 || extent.0.1 >= extent.1.1 {
        return Err(PyValueError::new_err(format!(
            "{name} minimums must be less than maximums"
        )));
    }
    Ok(())
}

fn validate_geo_step(name: &str, step: NativePoint) -> PyResult<()> {
    finite(&format!("{name}[0]"), step.0)?;
    finite(&format!("{name}[1]"), step.1)?;
    if step.0 <= 0.0 || step.1 <= 0.0 {
        return Err(PyValueError::new_err(format!(
            "{name} values must be positive"
        )));
    }
    Ok(())
}

#[pyfunction]
fn geo_graticule(
    py: Python<'_>,
    extent_major: NativeExtent,
    extent_minor: NativeExtent,
    step_major: NativePoint,
    step_minor: NativePoint,
    precision: f64,
) -> PyResult<NativeGraticuleResult> {
    validate_geo_extent("extent_major", extent_major)?;
    validate_geo_extent("extent_minor", extent_minor)?;
    validate_geo_step("step_major", step_major)?;
    validate_geo_step("step_minor", step_minor)?;
    random_positive("precision", precision)?;
    Ok(py.allow_threads(move || {
        let graticule = Graticule::new()
            .extent_major([
                [extent_major.0.0, extent_major.0.1],
                [extent_major.1.0, extent_major.1.1],
            ])
            .extent_minor([
                [extent_minor.0.0, extent_minor.0.1],
                [extent_minor.1.0, extent_minor.1.1],
            ])
            .step_major([step_major.0, step_major.1])
            .step_minor([step_minor.0, step_minor.1])
            .precision(precision);
        (graticule.lines(), graticule.outline())
    }))
}

#[pyfunction]
fn geo_rotation_value(
    angles: (f64, f64, f64),
    point: NativePoint,
    invert: bool,
) -> PyResult<NativePoint> {
    finite("lambda", angles.0)?;
    finite("phi", angles.1)?;
    finite("gamma", angles.2)?;
    finite_geo_point("point", point)?;
    let rotation = GeoRotation::new().angles(angles.0, angles.1, angles.2);
    Ok(if invert {
        rotation.invert(point.0, point.1)
    } else {
        rotation.rotate(point.0, point.1)
    })
}

type NativeVersor = (f64, f64, f64, f64);
type NativeCartesian = (f64, f64, f64);

fn finite_versor(name: &str, value: NativeVersor) -> PyResult<()> {
    finite(&format!("{name}.w"), value.0)?;
    finite(&format!("{name}.x"), value.1)?;
    finite(&format!("{name}.y"), value.2)?;
    finite(&format!("{name}.z"), value.3)
}

fn geo_versor(value: NativeVersor) -> GeoVersor {
    GeoVersor::new(value.0, value.1, value.2, value.3)
}

fn native_versor(value: GeoVersor) -> NativeVersor {
    (value.w, value.x, value.y, value.z)
}

#[pyfunction]
fn geo_versor_from_angles(angles: (f64, f64, f64)) -> PyResult<NativeVersor> {
    finite("lambda", angles.0)?;
    finite("phi", angles.1)?;
    finite("gamma", angles.2)?;
    Ok(native_versor(GeoVersor::from_angles(
        angles.0, angles.1, angles.2,
    )))
}

#[pyfunction]
fn geo_versor_to_angles(value: NativeVersor) -> PyResult<(f64, f64, f64)> {
    finite_versor("versor", value)?;
    Ok(geo_versor(value).to_angles())
}

#[pyfunction]
fn geo_versor_from_cartesian(value: NativeCartesian) -> PyResult<NativeVersor> {
    finite("cartesian.x", value.0)?;
    finite("cartesian.y", value.1)?;
    finite("cartesian.z", value.2)?;
    Ok(native_versor(GeoVersor::from_cartesian(
        value.0, value.1, value.2,
    )))
}

#[pyfunction]
fn geo_spherical_to_cartesian(point: NativePoint) -> PyResult<NativeCartesian> {
    finite_geo_point("point", point)?;
    let value = GeoVersor::spherical_to_cartesian(point.0, point.1);
    Ok((value[0], value[1], value[2]))
}

#[pyfunction]
fn geo_versor_multiply(a: NativeVersor, b: NativeVersor) -> PyResult<NativeVersor> {
    finite_versor("a", a)?;
    finite_versor("b", b)?;
    Ok(native_versor(geo_versor(a).multiply(geo_versor(b))))
}

#[pyfunction]
fn geo_versor_dot(a: NativeVersor, b: NativeVersor) -> PyResult<f64> {
    finite_versor("a", a)?;
    finite_versor("b", b)?;
    Ok(geo_versor(a).dot(geo_versor(b)))
}

#[pyfunction]
fn geo_versor_unary(value: NativeVersor, operation: &str) -> PyResult<NativeVersor> {
    finite_versor("versor", value)?;
    let value = geo_versor(value);
    match operation {
        "normalize" => Ok(native_versor(value.normalize())),
        "conjugate" => Ok(native_versor(value.conjugate())),
        _ => Err(PyValueError::new_err(format!(
            "unknown versor operation {operation:?}"
        ))),
    }
}

fn finite_cartesian(name: &str, value: NativeCartesian) -> PyResult<()> {
    finite(&format!("{name}.x"), value.0)?;
    finite(&format!("{name}.y"), value.1)?;
    finite(&format!("{name}.z"), value.2)
}

#[pyfunction]
fn geo_versor_delta(
    v0: NativeCartesian,
    v1: NativeCartesian,
    alpha: f64,
) -> PyResult<NativeVersor> {
    finite_cartesian("v0", v0)?;
    finite_cartesian("v1", v1)?;
    finite("alpha", alpha)?;
    Ok(native_versor(GeoVersor::delta_alpha(
        [v0.0, v0.1, v0.2],
        [v1.0, v1.1, v1.2],
        alpha,
    )))
}

#[pyfunction]
fn geo_versor_slerp(a: NativeVersor, b: NativeVersor, t: f64) -> PyResult<NativeVersor> {
    finite_versor("a", a)?;
    finite_versor("b", b)?;
    finite("t", t)?;
    Ok(native_versor(geo_versor(a).slerp(geo_versor(b), t)))
}

#[pyfunction]
fn geo_versor_rotate_spherical(value: NativeVersor, point: NativePoint) -> PyResult<NativePoint> {
    finite_versor("versor", value)?;
    finite("lambda", point.0)?;
    finite("phi", point.1)?;
    Ok(geo_versor(value).rotate_spherical(point.0, point.1))
}

#[pyfunction]
fn geo_versor_rotate_degrees(angles: (f64, f64, f64), point: NativePoint) -> PyResult<NativePoint> {
    finite("lambda", angles.0)?;
    finite("phi", angles.1)?;
    finite("gamma", angles.2)?;
    finite_geo_point("point", point)?;
    Ok(GeoVersor::rotate_degrees(angles, point.0, point.1))
}

#[derive(Clone)]
enum NativeProjection {
    Mercator(GeoMercator),
    Equirectangular(GeoEquirectangular),
    Orthographic(GeoOrthographic),
    Stereographic(GeoStereographic),
    TransverseMercator(GeoTransverseMercator),
    ConicEqualArea(GeoConicEqualArea),
    Albers(GeoAlbers),
}

macro_rules! projection_call {
    ($projection:expr, $method:ident $(, $argument:expr)*) => {
        match $projection {
            NativeProjection::Mercator(value) => value.$method($($argument),*),
            NativeProjection::Equirectangular(value) => value.$method($($argument),*),
            NativeProjection::Orthographic(value) => value.$method($($argument),*),
            NativeProjection::Stereographic(value) => value.$method($($argument),*),
            NativeProjection::TransverseMercator(value) => value.$method($($argument),*),
            NativeProjection::ConicEqualArea(value) => value.$method($($argument),*),
            NativeProjection::Albers(value) => value.$method($($argument),*),
        }
    };
}

fn configure_geo_projection<P: GeoProjection>(
    projection: &mut P,
    scale: Option<f64>,
    translate: Option<NativePoint>,
    center: Option<NativePoint>,
    rotate: Option<(f64, f64, f64)>,
) {
    if let Some(scale) = scale {
        projection.set_scale(scale);
    }
    if let Some((x, y)) = translate {
        projection.set_translate(x, y);
    }
    if let Some((lon, lat)) = center {
        projection.set_center(lon, lat);
    }
    if let Some((lambda, phi, gamma)) = rotate {
        projection.set_rotate(lambda, phi, gamma);
    }
}

fn build_geo_projection(
    kind: &str,
    scale: Option<f64>,
    translate: Option<NativePoint>,
    center: Option<NativePoint>,
    rotate: Option<(f64, f64, f64)>,
    parallels: Option<NativePoint>,
) -> PyResult<NativeProjection> {
    if let Some(scale) = scale {
        finite("scale", scale)?;
    }
    if let Some(translate) = translate {
        finite("translate.x", translate.0)?;
        finite("translate.y", translate.1)?;
    }
    if let Some(center) = center {
        finite_geo_point("center", center)?;
    }
    if let Some(rotate) = rotate {
        finite("rotate.lambda", rotate.0)?;
        finite("rotate.phi", rotate.1)?;
        finite("rotate.gamma", rotate.2)?;
    }
    if let Some(parallels) = parallels {
        finite("parallels[0]", parallels.0)?;
        finite("parallels[1]", parallels.1)?;
        if (parallels.0.to_radians().sin() + parallels.1.to_radians().sin()).abs() < 1e-12 {
            return Err(PyValueError::new_err(
                "conic parallels must not produce a zero cone constant",
            ));
        }
    }

    let mut projection = match kind {
        "mercator" => NativeProjection::Mercator(GeoMercator::new()),
        "equirectangular" => NativeProjection::Equirectangular(GeoEquirectangular::new()),
        "orthographic" => NativeProjection::Orthographic(GeoOrthographic::new()),
        "stereographic" => NativeProjection::Stereographic(GeoStereographic::new()),
        "transverse_mercator" => NativeProjection::TransverseMercator(GeoTransverseMercator::new()),
        "conic_equal_area" => NativeProjection::ConicEqualArea(match parallels {
            Some((phi0, phi1)) => GeoConicEqualArea::with_parallels(phi0, phi1),
            None => GeoConicEqualArea::new(),
        }),
        "albers" => NativeProjection::Albers(GeoAlbers::new()),
        _ => {
            return Err(PyValueError::new_err(format!(
                "unknown geo projection {kind:?}"
            )));
        }
    };
    if parallels.is_some() && kind != "conic_equal_area" {
        return Err(PyValueError::new_err(format!(
            "projection {kind:?} does not accept parallels"
        )));
    }
    match &mut projection {
        NativeProjection::Mercator(value) => {
            configure_geo_projection(value, scale, translate, center, rotate)
        }
        NativeProjection::Equirectangular(value) => {
            configure_geo_projection(value, scale, translate, center, rotate)
        }
        NativeProjection::Orthographic(value) => {
            configure_geo_projection(value, scale, translate, center, rotate)
        }
        NativeProjection::Stereographic(value) => {
            configure_geo_projection(value, scale, translate, center, rotate)
        }
        NativeProjection::TransverseMercator(value) => {
            configure_geo_projection(value, scale, translate, center, rotate)
        }
        NativeProjection::ConicEqualArea(value) => {
            configure_geo_projection(value, scale, translate, center, rotate)
        }
        NativeProjection::Albers(value) => {
            configure_geo_projection(value, scale, translate, center, rotate)
        }
    }
    Ok(projection)
}

#[pyfunction]
#[pyo3(signature = (kind, operation, point, scale=None, translate=None, center=None, rotate=None, parallels=None))]
fn geo_projection_apply(
    py: Python<'_>,
    kind: &str,
    operation: &str,
    point: NativePoint,
    scale: Option<f64>,
    translate: Option<NativePoint>,
    center: Option<NativePoint>,
    rotate: Option<(f64, f64, f64)>,
    parallels: Option<NativePoint>,
) -> PyResult<Option<NativePoint>> {
    finite("point.x", point.0)?;
    finite("point.y", point.1)?;
    let projection = build_geo_projection(kind, scale, translate, center, rotate, parallels)?;
    let operation = match operation {
        "project" => 0,
        "project_rotated" => 1,
        "invert" => 2,
        _ => {
            return Err(PyValueError::new_err(format!(
                "unknown projection operation {operation:?}"
            )));
        }
    };
    Ok(py.allow_threads(move || match operation {
        0 => Some(projection_call!(&projection, project, point.0, point.1)),
        1 => Some(projection_call!(
            &projection,
            project_rotated,
            point.0,
            point.1
        )),
        _ => projection_call!(&projection, invert, point.0, point.1),
    }))
}

type NativeProjectionMetadata = (
    f64,
    NativePoint,
    NativePoint,
    (f64, f64, f64),
    Option<f64>,
    Option<NativeExtent>,
    Option<f64>,
);

#[pyfunction]
#[pyo3(signature = (kind, scale=None, translate=None, center=None, rotate=None, parallels=None))]
fn geo_projection_metadata(
    kind: &str,
    scale: Option<f64>,
    translate: Option<NativePoint>,
    center: Option<NativePoint>,
    rotate: Option<(f64, f64, f64)>,
    parallels: Option<NativePoint>,
) -> PyResult<NativeProjectionMetadata> {
    let projection = build_geo_projection(kind, scale, translate, center, rotate, parallels)?;
    Ok((
        projection_call!(&projection, scale),
        projection_call!(&projection, translate),
        projection_call!(&projection, center),
        projection_call!(&projection, rotate),
        projection_call!(&projection, clip_angle),
        projection_call!(&projection, clip_extent),
        projection_call!(&projection, longitude_unwrap_center),
    ))
}

#[pyfunction]
#[pyo3(signature = (kind, point, scale=None, translate=None, center=None, rotate=None, parallels=None))]
fn geo_projection_visible(
    kind: &str,
    point: NativePoint,
    scale: Option<f64>,
    translate: Option<NativePoint>,
    center: Option<NativePoint>,
    rotate: Option<(f64, f64, f64)>,
    parallels: Option<NativePoint>,
) -> PyResult<bool> {
    finite_geo_point("point", point)?;
    let projection = build_geo_projection(kind, scale, translate, center, rotate, parallels)?;
    Ok(projection_call!(&projection, is_visible, point.0, point.1))
}

fn geo_geometry(kind: &str, coordinates: &Bound<'_, PyAny>) -> PyResult<D3GeoJsonGeometry> {
    let invalid = || {
        PyValueError::new_err(format!(
            "coordinates has an incompatible shape for GeoJSON geometry {kind:?}"
        ))
    };
    let geometry = match kind {
        "point" => {
            let (lon, lat) = coordinates
                .extract::<NativePoint>()
                .map_err(|_| invalid())?;
            D3GeoJsonGeometry::Point(lon, lat)
        }
        "multi_point" => D3GeoJsonGeometry::MultiPoint(
            coordinates
                .extract::<Vec<NativePoint>>()
                .map_err(|_| invalid())?,
        ),
        "line_string" => D3GeoJsonGeometry::LineString(
            coordinates
                .extract::<Vec<NativePoint>>()
                .map_err(|_| invalid())?,
        ),
        "multi_line_string" => D3GeoJsonGeometry::MultiLineString(
            coordinates
                .extract::<Vec<Vec<NativePoint>>>()
                .map_err(|_| invalid())?,
        ),
        "polygon" => D3GeoJsonGeometry::Polygon(
            coordinates
                .extract::<Vec<Vec<NativePoint>>>()
                .map_err(|_| invalid())?,
        ),
        "multi_polygon" => D3GeoJsonGeometry::MultiPolygon(
            coordinates
                .extract::<Vec<Vec<Vec<NativePoint>>>>()
                .map_err(|_| invalid())?,
        ),
        _ => {
            return Err(PyValueError::new_err(format!(
                "unknown GeoJSON geometry kind {kind:?}"
            )));
        }
    };
    validate_geo_geometry(&geometry)?;
    Ok(geometry)
}

fn validate_geo_line(points: &[NativePoint], path: &str) -> PyResult<()> {
    for (index, point) in points.iter().copied().enumerate() {
        finite_geo_point(&format!("{path}[{index}]"), point)?;
    }
    Ok(())
}

fn validate_geo_geometry(geometry: &D3GeoJsonGeometry) -> PyResult<()> {
    match geometry {
        D3GeoJsonGeometry::Point(lon, lat) => finite_geo_point("coordinates", (*lon, *lat)),
        D3GeoJsonGeometry::MultiPoint(points) | D3GeoJsonGeometry::LineString(points) => {
            validate_geo_line(points, "coordinates")
        }
        D3GeoJsonGeometry::MultiLineString(lines) | D3GeoJsonGeometry::Polygon(lines) => {
            for (index, line) in lines.iter().enumerate() {
                validate_geo_line(line, &format!("coordinates[{index}]"))?;
            }
            Ok(())
        }
        D3GeoJsonGeometry::MultiPolygon(polygons) => {
            for (polygon_index, polygon) in polygons.iter().enumerate() {
                for (ring_index, ring) in polygon.iter().enumerate() {
                    validate_geo_line(
                        ring,
                        &format!("coordinates[{polygon_index}][{ring_index}]"),
                    )?;
                }
            }
            Ok(())
        }
    }
}

type NativeGeoStreamEvent = (u8, f64, f64, i32);

#[derive(Default)]
struct GeoStreamCollector {
    events: Vec<NativeGeoStreamEvent>,
}

impl D3GeoStream for GeoStreamCollector {
    fn point(&mut self, x: f64, y: f64, marker: i32) {
        self.events.push((0, x, y, marker));
    }

    fn line_start(&mut self) {
        self.events.push((1, 0.0, 0.0, 0));
    }

    fn line_end(&mut self) {
        self.events.push((2, 0.0, 0.0, 0));
    }

    fn polygon_start(&mut self) {
        self.events.push((3, 0.0, 0.0, 0));
    }

    fn polygon_end(&mut self) {
        self.events.push((4, 0.0, 0.0, 0));
    }

    fn sphere(&mut self) {
        self.events.push((5, 0.0, 0.0, 0));
    }
}

#[pyfunction]
fn geo_stream_events(
    py: Python<'_>,
    geometry_kind: &str,
    coordinates: &Bound<'_, PyAny>,
) -> PyResult<Vec<NativeGeoStreamEvent>> {
    let geometry = geo_geometry(geometry_kind, coordinates)?;
    Ok(py.allow_threads(move || {
        let mut collector = GeoStreamCollector::default();
        d3_stream_geojson(&geometry, &mut collector);
        collector.events
    }))
}

type NativeMultiPolygon = Vec<Vec<Vec<NativePoint>>>;

type NativeAutoTyped = (u8, bool, i64, f64, String);

fn native_auto_typed(value: D3AutoTyped) -> NativeAutoTyped {
    match value {
        D3AutoTyped::Null => (0, false, 0, 0.0, String::new()),
        D3AutoTyped::Bool(value) => (1, value, 0, 0.0, String::new()),
        D3AutoTyped::Integer(value) => (2, false, value, 0.0, String::new()),
        D3AutoTyped::Float(value) => (3, false, 0, value, String::new()),
        D3AutoTyped::String(value) => (4, false, 0, 0.0, value),
        D3AutoTyped::Date(value) => (5, false, 0, 0.0, value),
    }
}

#[pyfunction]
fn fetch_auto_type_values(py: Python<'_>, values: Vec<String>) -> Vec<NativeAutoTyped> {
    py.allow_threads(move || {
        values
            .iter()
            .map(|value| native_auto_typed(d3_auto_type(value)))
            .collect()
    })
}

type NativeDsvBudget = (usize, usize, usize, usize, usize);
type NativeDsvError = (usize, usize, usize, u8, String, usize, usize, String);

fn dsv_budget(value: NativeDsvBudget) -> D3DsvBudget {
    D3DsvBudget::new(value.0, value.1, value.2, value.3, value.4)
}

fn native_dsv_error(error: D3DsvParseError) -> NativeDsvError {
    let (kind, detail, first, second) = match &error.kind {
        D3DsvParseErrorKind::UnterminatedQuotedField => (0, String::new(), 0, 0),
        D3DsvParseErrorKind::UnexpectedQuote => (1, String::new(), 0, 0),
        D3DsvParseErrorKind::InvalidDelimiter => (2, String::new(), 0, 0),
        D3DsvParseErrorKind::HeaderColumnMismatch { expected, actual } => {
            (3, String::new(), *expected, *actual)
        }
        D3DsvParseErrorKind::EmptyHeader { index } => (4, String::new(), *index, 0),
        D3DsvParseErrorKind::DuplicateHeader { name } => (5, name.clone(), 0, 0),
        D3DsvParseErrorKind::BudgetExceeded {
            resource,
            limit,
            actual,
        } => {
            let resource = match resource {
                D3DsvBudgetResource::InputBytes => "input_bytes",
                D3DsvBudgetResource::Records => "records",
                D3DsvBudgetResource::Columns => "columns",
                D3DsvBudgetResource::FieldBytes => "field_bytes",
                D3DsvBudgetResource::Cells => "cells",
            };
            (6, resource.to_owned(), *limit, *actual)
        }
        D3DsvParseErrorKind::Cancelled => (7, String::new(), 0, 0),
    };
    (
        error.line,
        error.column,
        error.byte_offset,
        kind,
        detail,
        first,
        second,
        error.message,
    )
}

fn configured_dsv_parser(
    delimiter: char,
    skip_empty_lines: bool,
    trim_values: bool,
    column_policy: &str,
    budget: NativeDsvBudget,
) -> PyResult<D3DsvParser> {
    let policy = match column_policy {
        "d3_compatible" => D3ColumnPolicy::D3Compatible,
        "strict" => D3ColumnPolicy::Strict,
        _ => {
            return Err(PyValueError::new_err(format!(
                "unknown DSV column policy {column_policy:?}"
            )));
        }
    };
    Ok(D3DsvParser::new(delimiter)
        .skip_empty_lines(skip_empty_lines)
        .trim_values(trim_values)
        .column_policy(policy)
        .budget(dsv_budget(budget)))
}

#[pyclass(name = "_DsvCancellationToken")]
#[derive(Clone, Default)]
struct NativeDsvCancellationToken {
    cancelled: Arc<AtomicBool>,
}

#[pymethods]
impl NativeDsvCancellationToken {
    #[new]
    fn new() -> Self {
        Self::default()
    }

    fn cancel(&self) {
        self.cancelled.store(true, AtomicOrdering::Release);
    }

    fn reset(&self) {
        self.cancelled.store(false, AtomicOrdering::Release);
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(AtomicOrdering::Acquire)
    }
}

#[pyfunction]
#[pyo3(signature = (text, delimiter, skip_empty_lines, trim_values, column_policy, budget, cancellation=None))]
fn fetch_parse_dsv(
    py: Python<'_>,
    text: String,
    delimiter: char,
    skip_empty_lines: bool,
    trim_values: bool,
    column_policy: &str,
    budget: NativeDsvBudget,
    cancellation: Option<PyRef<'_, NativeDsvCancellationToken>>,
) -> PyResult<(Vec<D3DsvRow>, Option<NativeDsvError>)> {
    let parser = configured_dsv_parser(
        delimiter,
        skip_empty_lines,
        trim_values,
        column_policy,
        budget,
    )?;
    let cancellation = cancellation.map(|token| Arc::clone(&token.cancelled));
    Ok(py.allow_threads(move || {
        let result = parser.parse_with_budget_and_cancel(&text, dsv_budget(budget), || {
            cancellation
                .as_ref()
                .is_some_and(|flag| flag.load(AtomicOrdering::Acquire))
        });
        match result {
            Ok(rows) => (rows, None),
            Err(error) => (Vec::new(), Some(native_dsv_error(error))),
        }
    }))
}

#[pyfunction]
#[pyo3(signature = (text, delimiter, skip_empty_lines, trim_values, column_policy, budget))]
fn fetch_parse_dsv_rows(
    py: Python<'_>,
    text: String,
    delimiter: char,
    skip_empty_lines: bool,
    trim_values: bool,
    column_policy: &str,
    budget: NativeDsvBudget,
) -> PyResult<(Vec<Vec<String>>, Option<NativeDsvError>)> {
    let parser = configured_dsv_parser(
        delimiter,
        skip_empty_lines,
        trim_values,
        column_policy,
        budget,
    )?;
    Ok(py.allow_threads(move || match parser.parse_rows(&text) {
        Ok(rows) => (rows, None),
        Err(error) => (Vec::new(), Some(native_dsv_error(error))),
    }))
}

#[pyfunction]
fn fetch_format_dsv(
    rows: Vec<D3DsvRow>,
    columns: Vec<String>,
    delimiter: char,
) -> PyResult<String> {
    let parser = configured_dsv_parser(
        delimiter,
        true,
        true,
        "d3_compatible",
        (usize::MAX, usize::MAX, usize::MAX, usize::MAX, usize::MAX),
    )?;
    let columns = columns.iter().map(String::as_str).collect::<Vec<_>>();
    Ok(parser.format(&rows, &columns))
}

type NativeSankeyNode = (String, usize, f64, f64, f64, f64, f64, usize, usize, usize);
type NativeSankeyLink = (usize, usize, f64, f64, f64, f64, String);
type NativeSankeyResult = (Vec<NativeSankeyNode>, Vec<NativeSankeyLink>);
type NativeSankeyError = (u8, String, String, usize, usize, f64, String);

fn native_sankey_error(error: D3SankeyLayoutError) -> NativeSankeyError {
    let fields = match &error {
        D3SankeyLayoutError::NonFiniteConfigField { field, value } => {
            (0, (*field).to_owned(), String::new(), 0, 0, *value)
        }
        D3SankeyLayoutError::NonPositiveConfigField { field, value } => {
            (1, (*field).to_owned(), String::new(), 0, 0, *value)
        }
        D3SankeyLayoutError::NegativeConfigField { field, value } => {
            (2, (*field).to_owned(), String::new(), 0, 0, *value)
        }
        D3SankeyLayoutError::InvalidDrawableArea { axis, available } => {
            (3, (*axis).to_owned(), String::new(), 0, 0, *available)
        }
        D3SankeyLayoutError::EmptyNodeName { index } => {
            (4, String::new(), String::new(), *index, 0, 0.0)
        }
        D3SankeyLayoutError::DuplicateNodeName {
            name,
            first_index,
            duplicate_index,
        } => (
            5,
            name.clone(),
            String::new(),
            *first_index,
            *duplicate_index,
            0.0,
        ),
        D3SankeyLayoutError::UnknownLinkEndpoint {
            link_index,
            endpoint,
            name,
        } => (6, (*endpoint).to_owned(), name.clone(), *link_index, 0, 0.0),
        D3SankeyLayoutError::NonFiniteLinkValue { link_index, value } => {
            (7, String::new(), String::new(), *link_index, 0, *value)
        }
        D3SankeyLayoutError::NegativeLinkValue { link_index, value } => {
            (8, String::new(), String::new(), *link_index, 0, *value)
        }
    };
    (
        fields.0,
        fields.1,
        fields.2,
        fields.3,
        fields.4,
        fields.5,
        error.to_string(),
    )
}

#[pyfunction]
#[pyo3(signature = (node_names, links, width, height, margins, extent, node_width, node_padding, iterations, node_align, input_order, checked))]
fn sankey_layout(
    py: Python<'_>,
    node_names: Vec<String>,
    links: Vec<(String, String, f64)>,
    width: f64,
    height: f64,
    margins: (f64, f64, f64, f64),
    extent: Option<(NativePoint, NativePoint)>,
    node_width: f64,
    node_padding: f64,
    iterations: usize,
    node_align: &str,
    input_order: bool,
    checked: bool,
) -> PyResult<(Option<NativeSankeyResult>, Option<NativeSankeyError>)> {
    let align = match node_align {
        "left" => D3SankeyNodeAlign::Left,
        "right" => D3SankeyNodeAlign::Right,
        "center" => D3SankeyNodeAlign::Center,
        "justify" => D3SankeyNodeAlign::Justify,
        _ => {
            return Err(PyValueError::new_err(format!(
                "unknown Sankey node alignment {node_align:?}"
            )));
        }
    };
    let links = links
        .into_iter()
        .map(|(source, target, value)| D3SankeyLinkInput {
            source,
            target,
            value,
        })
        .collect::<Vec<_>>();
    let mut layout = D3SankeyLayout::new()
        .width(width)
        .height(height)
        .margins(margins.0, margins.1, margins.2, margins.3)
        .node_width(node_width)
        .node_padding(node_padding)
        .iterations(iterations)
        .node_align(align);
    if let Some(extent) = extent {
        layout = layout.extent(extent.0.0, extent.0.1, extent.1.0, extent.1.1);
    }
    if input_order {
        layout = layout.link_sort_input_order();
    }
    Ok(py.allow_threads(move || {
        match if checked {
            layout.try_compute(&node_names, &links)
        } else {
            Ok(layout.compute(&node_names, &links))
        } {
            Ok(result) => (
                Some((
                    result
                        .nodes
                        .into_iter()
                        .map(|node| {
                            (
                                node.id,
                                node.index,
                                node.x0,
                                node.x1,
                                node.y0,
                                node.y1,
                                node.value,
                                node.depth,
                                node.height,
                                node.layer,
                            )
                        })
                        .collect(),
                    result
                        .links
                        .into_iter()
                        .map(|link| {
                            (
                                link.source,
                                link.target,
                                link.value,
                                link.y0,
                                link.y1,
                                link.width,
                                link.path,
                            )
                        })
                        .collect(),
                )),
                None,
            ),
            Err(error) => (None, Some(native_sankey_error(error))),
        }
    }))
}

type NativeHexPoint = (f64, f64, usize);
type NativeHexBin = (f64, f64, Vec<usize>);
type NativeHexError = (u8, usize, String, String, f64, f64, String);

fn native_hex_error(error: D3HexbinError) -> NativeHexError {
    let fields = match &error {
        D3HexbinError::NonFiniteRadius { radius } => {
            (0, 0, String::new(), String::new(), *radius, 0.0)
        }
        D3HexbinError::NonPositiveRadius { radius } => {
            (1, 0, String::new(), String::new(), *radius, 0.0)
        }
        D3HexbinError::NonFiniteExtentCoordinate {
            corner,
            coordinate,
            value,
        } => (
            2,
            0,
            (*corner).to_owned(),
            (*coordinate).to_owned(),
            *value,
            0.0,
        ),
        D3HexbinError::ReversedExtent { axis, min, max } => {
            (3, 0, (*axis).to_owned(), String::new(), *min, *max)
        }
        D3HexbinError::NonFinitePointCoordinate {
            index,
            coordinate,
            value,
        } => (
            4,
            *index,
            (*coordinate).to_owned(),
            String::new(),
            *value,
            0.0,
        ),
    };
    (
        fields.0,
        fields.1,
        fields.2,
        fields.3,
        fields.4,
        fields.5,
        error.to_string(),
    )
}

fn configured_hexbin(radius: f64, extent: (NativePoint, NativePoint)) -> D3Hexbin<NativeHexPoint> {
    D3Hexbin::with_accessors(|point: &NativeHexPoint| point.0, |point| point.1)
        .radius(radius)
        .extent(extent.0.0, extent.0.1, extent.1.0, extent.1.1)
}

#[pyfunction]
fn hexbin_bin(
    py: Python<'_>,
    points: Vec<NativeHexPoint>,
    radius: f64,
    extent: (NativePoint, NativePoint),
) -> (Option<Vec<NativeHexBin>>, Option<NativeHexError>) {
    py.allow_threads(
        move || match configured_hexbin(radius, extent).try_bin(points) {
            Ok(bins) => (
                Some(
                    bins.into_iter()
                        .map(|bin| {
                            (
                                bin.x,
                                bin.y,
                                bin.points.into_iter().map(|point| point.2).collect(),
                            )
                        })
                        .collect(),
                ),
                None,
            ),
            Err(error) => (None, Some(native_hex_error(error))),
        },
    )
}

#[pyfunction]
fn hexbin_hexagon(
    radius: f64,
    extent: (NativePoint, NativePoint),
    override_radius: Option<f64>,
) -> (Option<String>, Option<NativeHexError>) {
    let hexbin = configured_hexbin(radius, extent);
    let result = match override_radius {
        Some(radius) => hexbin.try_hexagon_with_radius(radius),
        None => hexbin.try_hexagon(),
    };
    match result {
        Ok(path) => (Some(path), None),
        Err(error) => (None, Some(native_hex_error(error))),
    }
}

#[pyfunction]
fn hexbin_centers(
    py: Python<'_>,
    radius: f64,
    extent: (NativePoint, NativePoint),
) -> (Option<Vec<NativePoint>>, Option<NativeHexError>) {
    py.allow_threads(
        move || match configured_hexbin(radius, extent).try_centers() {
            Ok(centers) => (Some(centers), None),
            Err(error) => (None, Some(native_hex_error(error))),
        },
    )
}

type NativeTileSet = (Vec<(i64, i64, u32)>, u32, f64, NativePoint);
type NativeTileError = (u8, String);

fn native_tile_error(error: D3TileError) -> NativeTileError {
    let kind = match error {
        D3TileError::NonFiniteScale => 0,
        D3TileError::NonPositiveScale => 1,
        D3TileError::NonFiniteTileSize => 2,
        D3TileError::NonPositiveTileSize => 3,
        D3TileError::NonFiniteTranslate => 4,
        D3TileError::NonFiniteExtent => 5,
        D3TileError::InvalidExtent => 6,
        D3TileError::ZoomOutOfRange => 7,
        D3TileError::TooManyTiles => 8,
    };
    (kind, error.to_string())
}

#[pyfunction]
#[pyo3(signature = (extent, scale, translate, tile_size, zoom_delta, clamp_x, clamp_y))]
fn tile_layout(
    py: Python<'_>,
    extent: (NativePoint, NativePoint),
    scale: f64,
    translate: NativePoint,
    tile_size: f64,
    zoom_delta: i32,
    clamp_x: bool,
    clamp_y: bool,
) -> (Option<NativeTileSet>, Option<NativeTileError>) {
    py.allow_threads(move || {
        let layout = D3TileLayout::new()
            .extent([[extent.0.0, extent.0.1], [extent.1.0, extent.1.1]])
            .scale(scale)
            .translate([translate.0, translate.1])
            .tile_size(tile_size)
            .zoom_delta(zoom_delta)
            .clamp(clamp_x, clamp_y);
        match layout.try_tiles() {
            Ok(set) => (
                Some((
                    set.tiles
                        .into_iter()
                        .map(|tile| (tile.x, tile.y, tile.z))
                        .collect(),
                    set.zoom,
                    set.tile_screen_size,
                    (set.origin[0], set.origin[1]),
                )),
                None,
            ),
            Err(error) => (None, Some(native_tile_error(error))),
        }
    })
}

type NativeAxisPoint = (f32, f32);
type NativeAxisLine = (NativeAxisPoint, NativeAxisPoint);
type NativeAxisTick = (
    f64,
    f64,
    NativeAxisLine,
    Option<NativeAxisPoint>,
    Option<String>,
    f32,
    bool,
);
type NativeAxisLayout = (
    String,
    f32,
    Option<NativeAxisLine>,
    Vec<NativeAxisTick>,
    Vec<NativeAxisTick>,
    Option<(String, NativeAxisPoint, f32)>,
);
type NativeAxisError = (u8, String, f64, String);

fn native_axis_error(error: D3AxisLayoutError) -> NativeAxisError {
    let (kind, field, value) = match &error {
        D3AxisLayoutError::NonFiniteConfig { field } => (0, (*field).to_owned(), 0.0),
        D3AxisLayoutError::NegativeConfig { field } => (1, (*field).to_owned(), 0.0),
        D3AxisLayoutError::NonFiniteRange => (2, String::new(), 0.0),
        D3AxisLayoutError::NonFiniteTick { value } => (3, String::new(), *value),
        D3AxisLayoutError::NonFiniteTickPosition { value } => (4, String::new(), *value),
    };
    (kind, field, value, error.to_string())
}

fn native_axis_layout<S: Scale<f64, f64>>(
    scale: &S,
    config: &D3AxisConfig,
    size: f32,
) -> Result<D3AxisLayout, D3AxisLayoutError> {
    D3AxisLayout::try_from_scale(scale, config, size)
}

#[pyfunction]
#[pyo3(signature = (scale_kind, domain, range, parameter, clamp, nice_count, orientation, tick_count, tick_values, minor_tick_values, minor_tick_size, tick_size, tick_padding, label_font_size, show_domain_line, domain_line_width, title, title_font_size, title_padding, label_angle, size))]
#[allow(clippy::too_many_arguments)]
fn axis_layout(
    py: Python<'_>,
    scale_kind: &str,
    domain: (f64, f64),
    range: (f64, f64),
    parameter: f64,
    clamp: bool,
    nice_count: Option<usize>,
    orientation: &str,
    tick_count: usize,
    tick_values: Option<Vec<f64>>,
    minor_tick_values: Option<Vec<f64>>,
    minor_tick_size: f32,
    tick_size: f32,
    tick_padding: f32,
    label_font_size: f32,
    show_domain_line: bool,
    domain_line_width: f32,
    title: Option<String>,
    title_font_size: f32,
    title_padding: f32,
    label_angle: f32,
    size: f32,
) -> PyResult<(Option<NativeAxisLayout>, Option<NativeAxisError>)> {
    for (name, value) in [
        ("domain[0]", domain.0),
        ("domain[1]", domain.1),
        ("range[0]", range.0),
        ("range[1]", range.1),
        ("parameter", parameter),
    ] {
        finite(name, value)?;
    }
    match scale_kind {
        "log" if domain.0 <= 0.0 || domain.1 <= 0.0 => {
            return Err(PyValueError::new_err(
                "log scale domain values must be positive",
            ));
        }
        "log" if parameter <= 0.0 || parameter == 1.0 => {
            return Err(PyValueError::new_err(
                "log scale base must be positive and not 1",
            ));
        }
        "pow" | "symlog" if parameter <= 0.0 => {
            return Err(PyValueError::new_err("scale parameter must be positive"));
        }
        "linear" | "log" | "pow" | "symlog" => {}
        _ => {
            return Err(PyValueError::new_err(format!(
                "unknown axis scale {scale_kind:?}"
            )));
        }
    }
    let orientation = match orientation {
        "top" => D3AxisOrientation::Top,
        "right" => D3AxisOrientation::Right,
        "bottom" => D3AxisOrientation::Bottom,
        "left" => D3AxisOrientation::Left,
        _ => {
            return Err(PyValueError::new_err(format!(
                "unknown axis orientation {orientation:?}"
            )));
        }
    };
    let mut config = D3AxisConfig::bottom();
    config.orientation = orientation;
    config.tick_count = tick_count;
    config.tick_values = tick_values;
    config.minor_tick_values = minor_tick_values;
    config.minor_tick_size = minor_tick_size;
    config.tick_size = tick_size;
    config.tick_padding = tick_padding;
    config.label_font_size = label_font_size;
    config.show_domain_line = show_domain_line;
    config.domain_line_width = domain_line_width;
    config.title = title;
    config.title_font_size = title_font_size;
    config.title_padding = title_padding;
    config.label_angle = label_angle;

    let result = py.allow_threads(move || match scale_kind {
        "linear" => {
            let mut scale = LinearScale::new()
                .domain(domain.0, domain.1)
                .range(range.0, range.1)
                .clamp(clamp);
            if let Some(count) = nice_count {
                scale = scale.nice(Some(count));
            }
            native_axis_layout(&scale, &config, size)
        }
        "log" => native_axis_layout(
            &LogScale::new()
                .domain(domain.0, domain.1)
                .range(range.0, range.1)
                .base(parameter)
                .clamp(clamp),
            &config,
            size,
        ),
        "pow" => {
            let mut scale = PowScale::new()
                .domain(domain.0, domain.1)
                .range(range.0, range.1)
                .exponent(parameter)
                .clamp(clamp);
            if let Some(count) = nice_count {
                scale = scale.nice(Some(count));
            }
            native_axis_layout(&scale, &config, size)
        }
        "symlog" => {
            let mut scale = SymlogScale::new()
                .domain(domain.0, domain.1)
                .range(range.0, range.1)
                .constant(parameter)
                .clamp(clamp);
            if let Some(count) = nice_count {
                scale = scale.nice(Some(count));
            }
            native_axis_layout(&scale, &config, size)
        }
        _ => unreachable!("validated above"),
    });
    Ok(match result {
        Ok(layout) => {
            let point = |point: d3rs::axis::AxisPoint| (point.x, point.y);
            let line = |line: d3rs::axis::AxisLine| (point(line.start), point(line.end));
            let tick = |tick: d3rs::axis::AxisTick| {
                (
                    tick.value,
                    tick.position,
                    line(tick.line),
                    tick.label_position.map(point),
                    tick.label,
                    tick.label_angle_degrees,
                    tick.is_minor,
                )
            };
            let orientation = match layout.orientation {
                D3AxisOrientation::Top => "top",
                D3AxisOrientation::Right => "right",
                D3AxisOrientation::Bottom => "bottom",
                D3AxisOrientation::Left => "left",
            };
            let title = layout
                .title
                .map(|title| (title.text, point(title.position), title.angle_degrees));
            (
                Some((
                    orientation.to_owned(),
                    layout.size,
                    layout.domain_line.map(line),
                    layout.major_ticks.into_iter().map(tick).collect(),
                    layout.minor_ticks.into_iter().map(tick).collect(),
                    title,
                )),
                None,
            )
        }
        Err(error) => (None, Some(native_axis_error(error))),
    })
}

enum NativeNumericScale {
    Linear(LinearScale),
    Log(LogScale),
    Pow(PowScale),
    Symlog(SymlogScale),
}

impl Scale<f64, f64> for NativeNumericScale {
    fn scale(&self, value: f64) -> f64 {
        match self {
            Self::Linear(scale) => scale.scale(value),
            Self::Log(scale) => scale.scale(value),
            Self::Pow(scale) => scale.scale(value),
            Self::Symlog(scale) => scale.scale(value),
        }
    }
    fn invert(&self, value: f64) -> Option<f64> {
        match self {
            Self::Linear(scale) => scale.invert(value),
            Self::Log(scale) => scale.invert(value),
            Self::Pow(scale) => scale.invert(value),
            Self::Symlog(scale) => scale.invert(value),
        }
    }
    fn ticks(&self, count: usize) -> Vec<f64> {
        match self {
            Self::Linear(scale) => scale.ticks(count),
            Self::Log(scale) => scale.ticks(count),
            Self::Pow(scale) => scale.ticks(count),
            Self::Symlog(scale) => scale.ticks(count),
        }
    }
    fn domain(&self) -> (f64, f64) {
        match self {
            Self::Linear(scale) => scale.domain(),
            Self::Log(scale) => scale.domain(),
            Self::Pow(scale) => scale.domain(),
            Self::Symlog(scale) => scale.domain(),
        }
    }
    fn range(&self) -> (f64, f64) {
        match self {
            Self::Linear(scale) => scale.range(),
            Self::Log(scale) => scale.range(),
            Self::Pow(scale) => scale.range(),
            Self::Symlog(scale) => scale.range(),
        }
    }
}

fn configured_numeric_scale(
    kind: &str,
    domain: (f64, f64),
    range: (f64, f64),
    parameter: f64,
    clamp: bool,
    nice_count: Option<usize>,
) -> PyResult<NativeNumericScale> {
    for (name, value) in [
        ("domain[0]", domain.0),
        ("domain[1]", domain.1),
        ("range[0]", range.0),
        ("range[1]", range.1),
        ("parameter", parameter),
    ] {
        finite(name, value)?;
    }
    match kind {
        "linear" => {
            let mut scale = LinearScale::new()
                .domain(domain.0, domain.1)
                .range(range.0, range.1)
                .clamp(clamp);
            if let Some(count) = nice_count {
                scale = scale.nice(Some(count));
            }
            Ok(NativeNumericScale::Linear(scale))
        }
        "log" => {
            if domain.0 <= 0.0 || domain.1 <= 0.0 || parameter <= 0.0 || parameter == 1.0 {
                return Err(PyValueError::new_err(
                    "log scale requires a positive domain and a positive base other than 1",
                ));
            }
            Ok(NativeNumericScale::Log(
                LogScale::new()
                    .domain(domain.0, domain.1)
                    .range(range.0, range.1)
                    .base(parameter)
                    .clamp(clamp),
            ))
        }
        "pow" => {
            if parameter <= 0.0 {
                return Err(PyValueError::new_err("power exponent must be positive"));
            }
            let mut scale = PowScale::new()
                .domain(domain.0, domain.1)
                .range(range.0, range.1)
                .exponent(parameter)
                .clamp(clamp);
            if let Some(count) = nice_count {
                scale = scale.nice(Some(count));
            }
            Ok(NativeNumericScale::Pow(scale))
        }
        "symlog" => {
            if parameter <= 0.0 {
                return Err(PyValueError::new_err("symlog constant must be positive"));
            }
            let mut scale = SymlogScale::new()
                .domain(domain.0, domain.1)
                .range(range.0, range.1)
                .constant(parameter)
                .clamp(clamp);
            if let Some(count) = nice_count {
                scale = scale.nice(Some(count));
            }
            Ok(NativeNumericScale::Symlog(scale))
        }
        _ => Err(PyValueError::new_err(format!(
            "unknown numeric scale {kind:?}"
        ))),
    }
}

#[pyfunction]
#[pyo3(signature = (selection, x_kind, x_domain, x_range, x_parameter, x_clamp, x_nice_count, y_kind, y_domain, y_range, y_parameter, y_clamp, y_nice_count))]
#[allow(clippy::too_many_arguments)]
fn brush_to_domain(
    selection: (f64, f64, f64, f64),
    x_kind: &str,
    x_domain: (f64, f64),
    x_range: (f64, f64),
    x_parameter: f64,
    x_clamp: bool,
    x_nice_count: Option<usize>,
    y_kind: &str,
    y_domain: (f64, f64),
    y_range: (f64, f64),
    y_parameter: f64,
    y_clamp: bool,
    y_nice_count: Option<usize>,
) -> PyResult<(f64, f64, f64, f64)> {
    for (name, value) in [
        ("selection.x0", selection.0),
        ("selection.y0", selection.1),
        ("selection.x1", selection.2),
        ("selection.y1", selection.3),
    ] {
        finite(name, value)?;
    }
    let x_scale = configured_numeric_scale(
        x_kind,
        x_domain,
        x_range,
        x_parameter,
        x_clamp,
        x_nice_count,
    )?;
    let y_scale = configured_numeric_scale(
        y_kind,
        y_domain,
        y_range,
        y_parameter,
        y_clamp,
        y_nice_count,
    )?;
    let domain = D3BrushSelection::new(selection.0, selection.1, selection.2, selection.3)
        .to_domain(&x_scale, &y_scale);
    Ok((domain.x0, domain.y0, domain.x1, domain.y1))
}

#[pyclass(name = "_BrushState", unsendable)]
struct NativeBrushState {
    state: D3BrushState,
}

#[pymethods]
impl NativeBrushState {
    #[new]
    fn new() -> Self {
        Self {
            state: D3BrushState::new(),
        }
    }

    fn start(&mut self, x: f64, y: f64) -> PyResult<()> {
        finite("x", x)?;
        finite("y", y)?;
        self.state.start(x, y);
        Ok(())
    }

    fn update(&mut self, x: f64, y: f64) -> PyResult<()> {
        finite("x", x)?;
        finite("y", y)?;
        self.state.update(x, y);
        Ok(())
    }

    fn end(&mut self) -> Option<(f64, f64, f64, f64)> {
        self.state
            .end()
            .map(|selection| (selection.x0, selection.y0, selection.x1, selection.y1))
    }

    fn reset(&mut self) {
        self.state.reset();
    }

    fn is_active(&self) -> bool {
        self.state.is_active()
    }

    fn current_selection(&self) -> Option<(f64, f64, f64, f64)> {
        self.state
            .current_selection()
            .map(|selection| (selection.x0, selection.y0, selection.x1, selection.y1))
    }
}

#[pyclass(name = "_ZoomState", unsendable)]
struct NativeZoomState {
    state: D3ZoomState,
}

#[pymethods]
impl NativeZoomState {
    #[new]
    #[pyo3(signature = (x_min, x_max, y_min, y_max, log_x=false, log_y=false))]
    fn new(x_min: f64, x_max: f64, y_min: f64, y_max: f64, log_x: bool, log_y: bool) -> Self {
        Self {
            state: D3ZoomState::new(x_min, x_max, y_min, y_max)
                .with_log_x(log_x)
                .with_log_y(log_y),
        }
    }

    fn with_log_x(&self, is_log: bool) -> Self {
        Self {
            state: self.state.clone().with_log_x(is_log),
        }
    }

    fn with_log_y(&self, is_log: bool) -> Self {
        Self {
            state: self.state.clone().with_log_y(is_log),
        }
    }

    fn zoom_to(&mut self, x_min: f64, x_max: f64, y_min: f64, y_max: f64) {
        self.state.zoom_to(x_min, x_max, y_min, y_max);
    }

    fn set_viewport(&mut self, x_min: f64, x_max: f64, y_min: f64, y_max: f64) {
        self.state.set_viewport(x_min, x_max, y_min, y_max);
    }

    fn reset(&mut self) {
        self.state.reset();
    }

    fn zoom_back(&mut self) -> bool {
        self.state.zoom_back()
    }

    fn is_zoomed(&self) -> bool {
        self.state.is_zoomed()
    }

    fn x_domain(&self) -> (f64, f64) {
        self.state.x_domain()
    }

    fn y_domain(&self) -> (f64, f64) {
        self.state.y_domain()
    }

    fn original_x_domain(&self) -> (f64, f64) {
        self.state.original_x_domain()
    }

    fn original_y_domain(&self) -> (f64, f64) {
        self.state.original_y_domain()
    }

    fn zoom_level(&self) -> usize {
        self.state.zoom_level()
    }

    fn set_original(&mut self, x_min: f64, x_max: f64, y_min: f64, y_max: f64) {
        self.state.set_original(x_min, x_max, y_min, y_max);
    }
}

type NativeGridPoint = (f32, f32);
type NativeGridLine = (f64, NativeGridPoint, NativeGridPoint);
type NativeGridDot = (f64, f64, NativeGridPoint);
type NativeGridLayout = (
    f32,
    f32,
    Vec<NativeGridLine>,
    Vec<NativeGridLine>,
    Vec<NativeGridDot>,
);
type NativeGridError = (u8, String, String, f64, String);

fn native_grid_error(error: D3GridLayoutError) -> NativeGridError {
    let (kind, field, axis, value) = match &error {
        D3GridLayoutError::NonFiniteSize { field } => (0, (*field).to_owned(), String::new(), 0.0),
        D3GridLayoutError::NegativeSize { field } => (1, (*field).to_owned(), String::new(), 0.0),
        D3GridLayoutError::NonFiniteConfig { field } => {
            (2, (*field).to_owned(), String::new(), 0.0)
        }
        D3GridLayoutError::NegativeConfig { field } => (3, (*field).to_owned(), String::new(), 0.0),
        D3GridLayoutError::InvalidOpacity { field, value } => {
            (4, (*field).to_owned(), String::new(), f64::from(*value))
        }
        D3GridLayoutError::NonFiniteRange { axis } => (5, String::new(), (*axis).to_owned(), 0.0),
        D3GridLayoutError::DegenerateRange { axis } => (6, String::new(), (*axis).to_owned(), 0.0),
        D3GridLayoutError::NonFiniteTick { axis, value } => {
            (7, String::new(), (*axis).to_owned(), *value)
        }
        D3GridLayoutError::NonFiniteTickPosition { axis, value } => {
            (8, String::new(), (*axis).to_owned(), *value)
        }
    };
    (kind, field, axis, value, error.to_string())
}

#[pyfunction]
#[pyo3(signature = (x_kind, x_domain, x_range, x_parameter, x_clamp, x_nice_count, y_kind, y_domain, y_range, y_parameter, y_clamp, y_nice_count, show_vertical_lines, show_horizontal_lines, show_dots, line_width, dot_radius, line_opacity, dot_opacity, vertical_values, horizontal_values, width, height))]
#[allow(clippy::too_many_arguments)]
fn grid_layout(
    py: Python<'_>,
    x_kind: &str,
    x_domain: (f64, f64),
    x_range: (f64, f64),
    x_parameter: f64,
    x_clamp: bool,
    x_nice_count: Option<usize>,
    y_kind: &str,
    y_domain: (f64, f64),
    y_range: (f64, f64),
    y_parameter: f64,
    y_clamp: bool,
    y_nice_count: Option<usize>,
    show_vertical_lines: bool,
    show_horizontal_lines: bool,
    show_dots: bool,
    line_width: f32,
    dot_radius: f32,
    line_opacity: f32,
    dot_opacity: f32,
    vertical_values: Option<Vec<f64>>,
    horizontal_values: Option<Vec<f64>>,
    width: f32,
    height: f32,
) -> PyResult<(Option<NativeGridLayout>, Option<NativeGridError>)> {
    let x_scale = configured_numeric_scale(
        x_kind,
        x_domain,
        x_range,
        x_parameter,
        x_clamp,
        x_nice_count,
    )?;
    let y_scale = configured_numeric_scale(
        y_kind,
        y_domain,
        y_range,
        y_parameter,
        y_clamp,
        y_nice_count,
    )?;
    let mut config = D3GridConfig::new();
    config.show_vertical_lines = show_vertical_lines;
    config.show_horizontal_lines = show_horizontal_lines;
    config.show_dots = show_dots;
    config.line_width = line_width;
    config.dot_radius = dot_radius;
    config.line_opacity = line_opacity;
    config.dot_opacity = dot_opacity;
    config.vertical_line_values = vertical_values;
    config.horizontal_line_values = horizontal_values;
    Ok(py.allow_threads(move || {
        match D3GridLayout::try_from_scales(&x_scale, &y_scale, &config, width, height) {
            Ok(layout) => (
                Some((
                    layout.width,
                    layout.height,
                    layout
                        .vertical_lines
                        .into_iter()
                        .map(|line| {
                            (
                                line.value,
                                (line.start.x, line.start.y),
                                (line.end.x, line.end.y),
                            )
                        })
                        .collect(),
                    layout
                        .horizontal_lines
                        .into_iter()
                        .map(|line| {
                            (
                                line.value,
                                (line.start.x, line.start.y),
                                (line.end.x, line.end.y),
                            )
                        })
                        .collect(),
                    layout
                        .dots
                        .into_iter()
                        .map(|dot| (dot.x_value, dot.y_value, (dot.center.x, dot.center.y)))
                        .collect(),
                )),
                None,
            ),
            Err(error) => (None, Some(native_grid_error(error))),
        }
    }))
}

type NativeLegendItemInput = (String, String, String, Option<String>);
type NativeLegendRect = (f64, f64, f64, f64);
type NativeLegendItemLayout = (
    usize,
    usize,
    usize,
    String,
    String,
    NativeLegendRect,
    NativeLegendRect,
    NativeLegendRect,
);
type NativeLegendLayout = (
    f64,
    f64,
    usize,
    usize,
    Vec<f64>,
    Option<(String, NativeLegendRect)>,
    Vec<NativeLegendItemLayout>,
);
type NativeLegendError = (u8, String, f64, String);

fn legend_symbol(value: &str) -> PyResult<D3LegendSymbol> {
    match value {
        "circle" => Ok(D3LegendSymbol::Circle),
        "square" => Ok(D3LegendSymbol::Square),
        "line" => Ok(D3LegendSymbol::Line),
        "line_with_marker" => Ok(D3LegendSymbol::LineWithMarker),
        "dashed_line" => Ok(D3LegendSymbol::DashedLine),
        "none" => Ok(D3LegendSymbol::None),
        _ => Err(PyValueError::new_err(format!(
            "unknown legend symbol {value:?}"
        ))),
    }
}

fn legend_symbol_name(value: D3LegendSymbol) -> String {
    match value {
        D3LegendSymbol::Circle => "circle",
        D3LegendSymbol::Square => "square",
        D3LegendSymbol::Line => "line",
        D3LegendSymbol::LineWithMarker => "line_with_marker",
        D3LegendSymbol::DashedLine => "dashed_line",
        D3LegendSymbol::None => "none",
    }
    .to_owned()
}

#[allow(clippy::too_many_arguments)]
fn configured_legend(
    position: &str,
    orientation: &str,
    title: Option<String>,
    items: Vec<NativeLegendItemInput>,
    symbol_size: f64,
    item_spacing: f64,
    padding: f64,
    background: bool,
    background_color: &str,
    border_width: f64,
    border_color: &str,
    font_size: f64,
    max_width: Option<f64>,
) -> PyResult<D3LegendConfig> {
    let position = match position {
        "top_left" => D3LegendPosition::TopLeft,
        "top_right" => D3LegendPosition::TopRight,
        "bottom_left" => D3LegendPosition::BottomLeft,
        "bottom_right" => D3LegendPosition::BottomRight,
        "top" => D3LegendPosition::Top,
        "bottom" => D3LegendPosition::Bottom,
        "left" => D3LegendPosition::Left,
        "right" => D3LegendPosition::Right,
        _ => {
            return Err(PyValueError::new_err(format!(
                "unknown legend position {position:?}"
            )));
        }
    };
    let orientation = match orientation {
        "horizontal" => D3LegendOrientation::Horizontal,
        "vertical" => D3LegendOrientation::Vertical,
        _ => {
            return Err(PyValueError::new_err(format!(
                "unknown legend orientation {orientation:?}"
            )));
        }
    };
    let items = items
        .into_iter()
        .map(|(label, item_color, symbol, data)| {
            let mut item =
                D3LegendItem::with_symbol(label, color(&item_color)?, legend_symbol(&symbol)?);
            if let Some(data) = data {
                item = item.data(data);
            }
            Ok(item)
        })
        .collect::<PyResult<Vec<_>>>()?;
    let mut config = D3LegendConfig::new()
        .position(position)
        .orientation(orientation)
        .items(items)
        .symbol_size(symbol_size)
        .item_spacing(item_spacing)
        .padding(padding)
        .background(background)
        .background_color(color(background_color)?)
        .border_width(border_width)
        .border_color(color(border_color)?)
        .font_size(font_size);
    if let Some(title) = title {
        config = config.title(title);
    }
    if let Some(max_width) = max_width {
        config = config.max_width(max_width);
    }
    Ok(config)
}

fn native_legend_error(error: D3LegendLayoutError) -> NativeLegendError {
    let (kind, field, value) = match &error {
        D3LegendLayoutError::NonFiniteSize { field } => (0, (*field).to_owned(), 0.0),
        D3LegendLayoutError::NegativeSize { field } => (1, (*field).to_owned(), 0.0),
        D3LegendLayoutError::NonFiniteConfig { field } => (2, (*field).to_owned(), 0.0),
        D3LegendLayoutError::NegativeConfig { field } => (3, (*field).to_owned(), 0.0),
        D3LegendLayoutError::NonPositiveAverageCharWidth { value } => (4, String::new(), *value),
    };
    (kind, field, value, error.to_string())
}

#[pyfunction]
#[pyo3(signature = (position, orientation, title, items, symbol_size, item_spacing, padding, background, background_color, border_width, border_color, font_size, max_width, available_width, avg_char_width=None))]
#[allow(clippy::too_many_arguments)]
fn legend_layout(
    py: Python<'_>,
    position: &str,
    orientation: &str,
    title: Option<String>,
    items: Vec<NativeLegendItemInput>,
    symbol_size: f64,
    item_spacing: f64,
    padding: f64,
    background: bool,
    background_color: &str,
    border_width: f64,
    border_color: &str,
    font_size: f64,
    max_width: Option<f64>,
    available_width: f64,
    avg_char_width: Option<f64>,
) -> PyResult<(Option<NativeLegendLayout>, Option<NativeLegendError>)> {
    let config = configured_legend(
        position,
        orientation,
        title,
        items,
        symbol_size,
        item_spacing,
        padding,
        background,
        background_color,
        border_width,
        border_color,
        font_size,
        max_width,
    )?;
    Ok(py.allow_threads(move || {
        let result = if let Some(avg_char_width) = avg_char_width {
            d3rs::legend::LegendLayout::try_from_config_with_char_width(
                &config,
                available_width,
                avg_char_width,
            )
        } else {
            d3rs::legend::LegendLayout::try_from_config(&config, available_width)
        };
        match result {
            Ok(layout) => {
                let rect = |bounds: d3rs::legend::LegendRect| {
                    (
                        bounds.origin.x,
                        bounds.origin.y,
                        bounds.width,
                        bounds.height,
                    )
                };
                let title = layout.title.map(|title| (title.text, rect(title.bounds)));
                let items = layout
                    .items
                    .into_iter()
                    .map(|item| {
                        (
                            item.index,
                            item.row,
                            item.column,
                            item.label,
                            legend_symbol_name(item.symbol),
                            rect(item.item_bounds),
                            rect(item.symbol_bounds),
                            rect(item.label_bounds),
                        )
                    })
                    .collect();
                (
                    Some((
                        layout.width,
                        layout.height,
                        layout.columns,
                        layout.rows,
                        layout.column_widths,
                        title,
                        items,
                    )),
                    None,
                )
            }
            Err(error) => (None, Some(native_legend_error(error))),
        }
    }))
}

type NativeQuadPoint = (f64, f64, usize);
type NativeQuadAggregate = (f64, f64, f64);
type NativeQuadNodeSnapshot = (
    f64,
    f64,
    f64,
    f64,
    u8,
    Vec<NativeQuadPoint>,
    Option<NativeQuadAggregate>,
);

fn native_quad_aggregate(value: Option<D3QuadAggregate>) -> Option<NativeQuadAggregate> {
    value.map(|aggregate| (aggregate.mass, aggregate.x, aggregate.y))
}

fn native_quad_snapshot(
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    node: &D3QuadNode<usize>,
) -> NativeQuadNodeSnapshot {
    match node {
        D3QuadNode::Leaf(point) => {
            let mut points = Vec::new();
            let mut current = Some(point);
            while let Some(point) = current {
                points.push((point.x, point.y, point.data));
                current = point.next.as_deref();
            }
            (
                x0,
                y0,
                x1,
                y1,
                1,
                points,
                native_quad_aggregate(node.aggregate()),
            )
        }
        D3QuadNode::Internal(_, aggregate) => (
            x0,
            y0,
            x1,
            y1,
            0,
            Vec::new(),
            native_quad_aggregate(*aggregate),
        ),
    }
}

fn validate_quad_point(path: &str, x: f64, y: f64) -> PyResult<()> {
    finite(&format!("{path}.x"), x)?;
    finite(&format!("{path}.y"), y)
}

#[pyclass(name = "_QuadTreeIndex")]
#[derive(Clone, Default)]
struct NativeQuadTreeIndex {
    tree: D3QuadTree<usize>,
}

#[pymethods]
impl NativeQuadTreeIndex {
    #[new]
    #[pyo3(signature = (points=None))]
    fn new(points: Option<Vec<NativeQuadPoint>>) -> PyResult<Self> {
        let mut index = Self::default();
        if let Some(points) = points {
            index.add_all(points)?;
        }
        Ok(index)
    }

    fn copy(&self) -> Self {
        self.clone()
    }

    fn cover(&mut self, x: f64, y: f64) -> PyResult<()> {
        validate_quad_point("point", x, y)?;
        self.tree.cover(x, y);
        Ok(())
    }

    fn add(&mut self, x: f64, y: f64, id: usize) -> PyResult<()> {
        validate_quad_point("point", x, y)?;
        self.tree
            .try_add(x, y, id)
            .map(|_| ())
            .map_err(|error| PyValueError::new_err(error.to_string()))
    }

    fn add_all(&mut self, points: Vec<NativeQuadPoint>) -> PyResult<()> {
        for (index, (x, y, _)) in points.iter().copied().enumerate() {
            validate_quad_point(&format!("points[{index}]"), x, y)?;
        }
        for (x, y, id) in points {
            self.tree.add(x, y, id);
        }
        Ok(())
    }

    fn remove(&mut self, x: f64, y: f64) -> PyResult<Option<usize>> {
        validate_quad_point("point", x, y)?;
        let removed = self
            .tree
            .data()
            .into_iter()
            .find(|(point_x, point_y, _)| {
                (point_x - x).abs() < 1e-12 && (point_y - y).abs() < 1e-12
            })
            .map(|(_, _, id)| id);
        if removed.is_some() {
            self.tree.remove(x, y);
        }
        Ok(removed)
    }

    fn find(&self, x: f64, y: f64, radius: Option<f64>) -> PyResult<Option<usize>> {
        validate_quad_point("point", x, y)?;
        if let Some(radius) = radius {
            finite("radius", radius)?;
            if radius < 0.0 {
                return Err(PyValueError::new_err("radius must be non-negative"));
            }
        }
        Ok(self.tree.find(x, y, radius).copied())
    }

    fn find_all(&self, x: f64, y: f64, radius: f64) -> PyResult<Vec<usize>> {
        validate_quad_point("point", x, y)?;
        finite("radius", radius)?;
        if radius < 0.0 {
            return Err(PyValueError::new_err("radius must be non-negative"));
        }
        Ok(self
            .tree
            .find_all(x, y, radius)
            .into_iter()
            .copied()
            .collect())
    }

    fn data(&self) -> Vec<NativeQuadPoint> {
        self.tree.data()
    }

    fn size(&self) -> usize {
        self.tree.size()
    }

    fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }

    fn extent(&self) -> Option<(f64, f64, f64, f64)> {
        self.tree
            .extent()
            .map(|extent| (extent.x0, extent.y0, extent.x1, extent.y1))
    }

    fn compute_aggregates(&mut self) {
        self.tree.compute_aggregates();
    }

    fn snapshots(&self, after: bool) -> Vec<NativeQuadNodeSnapshot> {
        let snapshots = RefCell::new(Vec::new());
        if after {
            self.tree.visit_after(|x0, y0, x1, y1, node| {
                snapshots
                    .borrow_mut()
                    .push(native_quad_snapshot(x0, y0, x1, y1, node));
            });
        } else {
            self.tree.visit(|x0, y0, x1, y1, node| {
                snapshots
                    .borrow_mut()
                    .push(native_quad_snapshot(x0, y0, x1, y1, node));
                true
            });
        }
        snapshots.into_inner()
    }

    fn visit(&self, py: Python<'_>, callback: Py<PyAny>) -> PyResult<()> {
        let failure = RefCell::new(None);
        self.tree.visit(|x0, y0, x1, y1, node| {
            if failure.borrow().is_some() {
                return false;
            }
            let snapshot = native_quad_snapshot(x0, y0, x1, y1, node);
            match callback
                .bind(py)
                .call1((snapshot,))
                .and_then(|result| result.extract::<bool>())
            {
                Ok(visit_children) => visit_children,
                Err(error) => {
                    *failure.borrow_mut() = Some(error);
                    false
                }
            }
        });
        match failure.into_inner() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    fn visit_after(&self, py: Python<'_>, callback: Py<PyAny>) -> PyResult<()> {
        let failure = RefCell::new(None);
        self.tree.visit_after(|x0, y0, x1, y1, node| {
            if failure.borrow().is_some() {
                return;
            }
            let snapshot = native_quad_snapshot(x0, y0, x1, y1, node);
            if let Err(error) = callback.bind(py).call1((snapshot,)) {
                *failure.borrow_mut() = Some(error);
            }
        });
        match failure.into_inner() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    fn visit_aggregate(&self, py: Python<'_>, callback: Py<PyAny>) -> PyResult<()> {
        let failure = RefCell::new(None);
        self.tree
            .visit_aggregate(|x0, y0, x1, y1, node, aggregate| {
                if failure.borrow().is_some() {
                    return false;
                }
                let snapshot = native_quad_snapshot(x0, y0, x1, y1, node);
                match callback
                    .bind(py)
                    .call1((snapshot, native_quad_aggregate(aggregate)))
                    .and_then(|result| result.extract::<bool>())
                {
                    Ok(visit_children) => visit_children,
                    Err(error) => {
                        *failure.borrow_mut() = Some(error);
                        false
                    }
                }
            });
        match failure.into_inner() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

fn geo_multi_polygon(geometry: D3GeoJsonGeometry) -> Option<NativeMultiPolygon> {
    match geometry {
        D3GeoJsonGeometry::MultiPolygon(polygons) => Some(polygons),
        _ => None,
    }
}

#[pyfunction]
fn topojson_parse_land(py: Python<'_>, json: String) -> Option<NativeMultiPolygon> {
    py.allow_threads(move || d3_parse_land(&json).and_then(geo_multi_polygon))
}

#[pyfunction]
#[pyo3(signature = (json, max_input_bytes, max_arcs, max_arc_points, max_output_points, max_geometries))]
fn topojson_parse_land_with_budget(
    py: Python<'_>,
    json: String,
    max_input_bytes: usize,
    max_arcs: usize,
    max_arc_points: usize,
    max_output_points: usize,
    max_geometries: usize,
) -> PyResult<NativeMultiPolygon> {
    let budget = D3TopoJsonBudget::new(
        max_input_bytes,
        max_arcs,
        max_arc_points,
        max_output_points,
        max_geometries,
    );
    py.allow_threads(move || {
        d3_parse_land_with_budget(&json, &budget)
            .map(geo_multi_polygon)
            .map_err(|error| PyValueError::new_err(error.to_string()))?
            .ok_or_else(|| PyValueError::new_err("TopoJSON land geometry was not a MultiPolygon"))
    })
}

fn configured_geo_path<P: GeoProjection>(
    projection: P,
    digits: usize,
    point_radius: f64,
) -> D3GeoPath<P> {
    D3GeoPath::new(projection)
        .digits(digits)
        .point_radius(point_radius)
}

macro_rules! geo_path_call {
    ($projection:expr, $function:ident $(, $argument:expr)*) => {
        match $projection {
            NativeProjection::Mercator(value) => $function(value $(, $argument)*),
            NativeProjection::Equirectangular(value) => $function(value $(, $argument)*),
            NativeProjection::Orthographic(value) => $function(value $(, $argument)*),
            NativeProjection::Stereographic(value) => $function(value $(, $argument)*),
            NativeProjection::TransverseMercator(value) => $function(value $(, $argument)*),
            NativeProjection::ConicEqualArea(value) => $function(value $(, $argument)*),
            NativeProjection::Albers(value) => $function(value $(, $argument)*),
        }
    };
}

fn render_geo_path<P: GeoProjection>(
    projection: P,
    geometry: &D3GeoJsonGeometry,
    digits: usize,
    point_radius: f64,
) -> String {
    configured_geo_path(projection, digits, point_radius).render(geometry)
}

fn bounds_geo_path<P: GeoProjection>(
    projection: P,
    geometry: &D3GeoJsonGeometry,
    digits: usize,
    point_radius: f64,
) -> (NativePoint, NativePoint) {
    configured_geo_path(projection, digits, point_radius).bounds(geometry)
}

fn centroid_geo_path<P: GeoProjection>(
    projection: P,
    geometry: &D3GeoJsonGeometry,
    digits: usize,
    point_radius: f64,
) -> NativePoint {
    configured_geo_path(projection, digits, point_radius).centroid(geometry)
}

fn project_geo_path<P: GeoProjection>(
    projection: P,
    coordinates: &[NativePoint],
) -> Vec<NativePoint> {
    D3GeoPath::new(projection).project_coords(coordinates)
}

fn validate_geo_path_config(digits: usize, point_radius: f64) -> PyResult<()> {
    if digits > 15 {
        return Err(PyValueError::new_err("digits must be between 0 and 15"));
    }
    finite("point_radius", point_radius)?;
    if point_radius < 0.0 {
        return Err(PyValueError::new_err("point_radius must be non-negative"));
    }
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (geometry_kind, coordinates, projection_kind, digits=3, point_radius=4.5, scale=None, translate=None, center=None, rotate=None, parallels=None))]
fn geo_path_render(
    py: Python<'_>,
    geometry_kind: &str,
    coordinates: &Bound<'_, PyAny>,
    projection_kind: &str,
    digits: usize,
    point_radius: f64,
    scale: Option<f64>,
    translate: Option<NativePoint>,
    center: Option<NativePoint>,
    rotate: Option<(f64, f64, f64)>,
    parallels: Option<NativePoint>,
) -> PyResult<String> {
    validate_geo_path_config(digits, point_radius)?;
    let geometry = geo_geometry(geometry_kind, coordinates)?;
    let projection =
        build_geo_projection(projection_kind, scale, translate, center, rotate, parallels)?;
    Ok(py.allow_threads(move || {
        geo_path_call!(projection, render_geo_path, &geometry, digits, point_radius)
    }))
}

#[pyfunction]
#[pyo3(signature = (geometry_kind, coordinates, projection_kind, digits=3, point_radius=4.5, scale=None, translate=None, center=None, rotate=None, parallels=None))]
fn geo_path_bounds(
    py: Python<'_>,
    geometry_kind: &str,
    coordinates: &Bound<'_, PyAny>,
    projection_kind: &str,
    digits: usize,
    point_radius: f64,
    scale: Option<f64>,
    translate: Option<NativePoint>,
    center: Option<NativePoint>,
    rotate: Option<(f64, f64, f64)>,
    parallels: Option<NativePoint>,
) -> PyResult<(NativePoint, NativePoint)> {
    validate_geo_path_config(digits, point_radius)?;
    let geometry = geo_geometry(geometry_kind, coordinates)?;
    let projection =
        build_geo_projection(projection_kind, scale, translate, center, rotate, parallels)?;
    Ok(py.allow_threads(move || {
        geo_path_call!(projection, bounds_geo_path, &geometry, digits, point_radius)
    }))
}

#[pyfunction]
#[pyo3(signature = (geometry_kind, coordinates, projection_kind, digits=3, point_radius=4.5, scale=None, translate=None, center=None, rotate=None, parallels=None))]
fn geo_path_centroid(
    py: Python<'_>,
    geometry_kind: &str,
    coordinates: &Bound<'_, PyAny>,
    projection_kind: &str,
    digits: usize,
    point_radius: f64,
    scale: Option<f64>,
    translate: Option<NativePoint>,
    center: Option<NativePoint>,
    rotate: Option<(f64, f64, f64)>,
    parallels: Option<NativePoint>,
) -> PyResult<NativePoint> {
    validate_geo_path_config(digits, point_radius)?;
    let geometry = geo_geometry(geometry_kind, coordinates)?;
    let projection =
        build_geo_projection(projection_kind, scale, translate, center, rotate, parallels)?;
    Ok(py.allow_threads(move || {
        geo_path_call!(
            projection,
            centroid_geo_path,
            &geometry,
            digits,
            point_radius
        )
    }))
}

#[pyfunction]
#[pyo3(signature = (coordinates, projection_kind, scale=None, translate=None, center=None, rotate=None, parallels=None))]
fn geo_path_project_coords(
    py: Python<'_>,
    coordinates: Vec<NativePoint>,
    projection_kind: &str,
    scale: Option<f64>,
    translate: Option<NativePoint>,
    center: Option<NativePoint>,
    rotate: Option<(f64, f64, f64)>,
    parallels: Option<NativePoint>,
) -> PyResult<Vec<NativePoint>> {
    validate_geo_line(&coordinates, "coordinates")?;
    let projection =
        build_geo_projection(projection_kind, scale, translate, center, rotate, parallels)?;
    Ok(py.allow_threads(move || geo_path_call!(projection, project_geo_path, &coordinates)))
}

type NativeFormatSpecifier = (
    String,
    String,
    String,
    Option<String>,
    bool,
    Option<usize>,
    bool,
    Option<usize>,
    bool,
    String,
);

fn align_name(value: Align) -> &'static str {
    match value {
        Align::Left => "left",
        Align::Right => "right",
        Align::Center => "center",
        Align::AfterSign => "after_sign",
    }
}

fn sign_name(value: Sign) -> &'static str {
    match value {
        Sign::Minus => "minus",
        Sign::Plus => "plus",
        Sign::Space => "space",
        Sign::Parens => "parens",
    }
}

fn format_type_name(value: FormatType) -> &'static str {
    match value {
        FormatType::None => "none",
        FormatType::Exponent => "exponent",
        FormatType::Fixed => "fixed",
        FormatType::General => "general",
        FormatType::Round => "round",
        FormatType::Si => "si",
        FormatType::Percent => "percent",
        FormatType::PercentRounded => "percent_rounded",
        FormatType::Binary => "binary",
        FormatType::Octal => "octal",
        FormatType::Decimal => "decimal",
        FormatType::HexLower => "hex_lower",
        FormatType::HexUpper => "hex_upper",
        FormatType::Character => "character",
    }
}

#[pyfunction]
fn parse_format_specifier(specifier: &str) -> NativeFormatSpecifier {
    let parsed = d3rs::format::parse_specifier(specifier);
    (
        parsed.fill.to_string(),
        align_name(parsed.align).into(),
        sign_name(parsed.sign).into(),
        parsed.symbol.map(|value| value.to_string()),
        parsed.zero,
        parsed.width,
        parsed.comma,
        parsed.precision,
        parsed.trim,
        format_type_name(parsed.format_type).into(),
    )
}

#[pyfunction]
fn format_value(specifier: &str, value: f64) -> String {
    d3rs::format::format_value(specifier, value)
}

#[pyfunction(name = "_format_prefix_value")]
fn format_prefix_value(specifier: &str, reference: f64, value: f64) -> PyResult<String> {
    finite("reference", reference)?;
    Ok(d3rs::format::format_prefix(specifier, reference)(value))
}

#[pyfunction]
fn prefix_exponent(value: f64) -> PyResult<i32> {
    finite("value", value)?;
    Ok(d3rs::format::prefix_exponent(value))
}

#[pyfunction(name = "_format_locale_value")]
#[pyo3(signature = (specifier, value, decimal, thousands, currency_prefix, currency_suffix, grouping, numerals, minus, percent))]
#[allow(clippy::too_many_arguments)]
fn format_locale_value(
    specifier: &str,
    value: f64,
    decimal: &str,
    thousands: &str,
    currency_prefix: Option<String>,
    currency_suffix: Option<String>,
    grouping: Vec<usize>,
    numerals: Option<Vec<String>>,
    minus: &str,
    percent: &str,
) -> PyResult<String> {
    if decimal.is_empty() {
        return Err(PyValueError::new_err("locale decimal must not be empty"));
    }
    if grouping.is_empty() || grouping.contains(&0) {
        return Err(PyValueError::new_err(
            "locale grouping must contain positive integers",
        ));
    }
    if numerals.as_ref().is_some_and(|values| values.len() != 10) {
        return Err(PyValueError::new_err(
            "locale numerals must contain exactly ten strings",
        ));
    }
    let numeral_refs = numerals
        .as_ref()
        .map(|values| values.iter().map(String::as_str).collect::<Vec<&str>>());
    let locale = Locale {
        decimal,
        thousands,
        currency_prefix: currency_prefix.as_deref(),
        currency_suffix: currency_suffix.as_deref(),
        grouping: &grouping,
        numerals: numeral_refs.as_deref(),
        minus,
        percent,
    };
    let parsed = d3rs::format::parse_specifier(specifier);
    Ok(locale.format(&parsed, value))
}

fn time_interval(value: &str) -> PyResult<TimeInterval> {
    match value {
        "second" => Ok(TimeInterval::Second),
        "minute" => Ok(TimeInterval::Minute),
        "hour" => Ok(TimeInterval::Hour),
        "day" => Ok(TimeInterval::Day),
        "week" => Ok(TimeInterval::Week),
        "monday" => Ok(TimeInterval::Monday),
        "month" => Ok(TimeInterval::Month),
        "year" => Ok(TimeInterval::Year),
        _ => Err(PyValueError::new_err(format!(
            "unknown time interval {value:?}"
        ))),
    }
}

fn time_interval_name(value: TimeInterval) -> &'static str {
    match value {
        TimeInterval::Second => "second",
        TimeInterval::Minute => "minute",
        TimeInterval::Hour => "hour",
        TimeInterval::Day => "day",
        TimeInterval::Week => "week",
        TimeInterval::Monday => "monday",
        TimeInterval::Month => "month",
        TimeInterval::Year => "year",
    }
}

#[pyfunction]
fn time_interval_floor(interval: &str, timestamp: i64) -> PyResult<i64> {
    Ok(time_interval(interval)?.floor(timestamp))
}

#[pyfunction]
fn time_interval_ceil(interval: &str, timestamp: i64) -> PyResult<i64> {
    Ok(time_interval(interval)?.ceil(timestamp))
}

#[pyfunction]
fn time_interval_round(interval: &str, timestamp: i64) -> PyResult<i64> {
    Ok(time_interval(interval)?.round(timestamp))
}

#[pyfunction]
fn time_interval_offset(interval: &str, timestamp: i64, step: i64) -> PyResult<i64> {
    Ok(time_interval(interval)?.offset(timestamp, step))
}

#[pyfunction]
fn time_interval_count(interval: &str, start: i64, end: i64) -> PyResult<i64> {
    Ok(time_interval(interval)?.count(start, end))
}

#[pyfunction]
fn time_interval_range(interval: &str, start: i64, stop: i64, step: i64) -> PyResult<Vec<i64>> {
    if step <= 0 {
        return Err(PyValueError::new_err("step must be positive"));
    }
    Ok(time_interval(interval)?.range(start, stop, step))
}

#[pyfunction]
fn time_interval_duration(interval: &str) -> PyResult<i64> {
    Ok(time_interval(interval)?.duration())
}

#[pyfunction]
fn time_interval_format_pattern(interval: &str) -> PyResult<&'static str> {
    Ok(time_interval(interval)?.format_pattern())
}

#[pyfunction]
fn time_interval_for_span(span_seconds: i64) -> &'static str {
    time_interval_name(TimeInterval::for_span(span_seconds))
}

type NativeTimeFormatParts = (i64, u32, u32, u32, u32, u32, u32, u32);

#[pyfunction]
fn time_format_parts(timestamp: i64) -> NativeTimeFormatParts {
    let parts = TimeFormatParts::from_unix_seconds(timestamp);
    (
        parts.year,
        parts.month,
        parts.day,
        parts.hour,
        parts.minute,
        parts.second,
        parts.weekday,
        parts.day_of_year,
    )
}

#[pyfunction]
fn time_format_value(pattern: &str, timestamp: i64) -> String {
    TimeFormat::new(pattern).format(timestamp)
}

fn native_time_scale(domain: (i64, i64), range: (f64, f64), clamp: bool) -> PyResult<TimeScale> {
    if domain.0 == domain.1 {
        return Err(PyValueError::new_err(
            "time scale domain endpoints must differ",
        ));
    }
    finite("range[0]", range.0)?;
    finite("range[1]", range.1)?;
    if range.0 == range.1 {
        return Err(PyValueError::new_err(
            "time scale range endpoints must differ",
        ));
    }
    Ok(TimeScale::new()
        .domain(domain.0, domain.1)
        .range(range.0, range.1)
        .clamp(clamp))
}

#[pyfunction]
#[pyo3(signature = (value, *, domain, range, clamp=false))]
fn time_scale_value(
    value: i64,
    domain: (i64, i64),
    range: (f64, f64),
    clamp: bool,
) -> PyResult<f64> {
    Ok(native_time_scale(domain, range, clamp)?.scale(value))
}

#[pyfunction]
#[pyo3(signature = (value, *, domain, range, clamp=false))]
fn time_scale_invert(
    value: f64,
    domain: (i64, i64),
    range: (f64, f64),
    clamp: bool,
) -> PyResult<Option<i64>> {
    finite("value", value)?;
    Ok(native_time_scale(domain, range, clamp)?.invert(value))
}

#[pyfunction]
fn time_scale_nice(domain: (i64, i64), count: Option<i64>) -> PyResult<(i64, i64)> {
    let count = count
        .map(|value| {
            usize::try_from(value)
                .ok()
                .filter(|value| *value > 0)
                .ok_or_else(|| PyValueError::new_err("count must be positive"))
        })
        .transpose()?;
    let scale = TimeScale::new().domain(domain.0, domain.1).nice(count);
    Ok((scale.domain_min(), scale.domain_max()))
}

#[pyfunction]
fn time_scale_ticks(domain: (i64, i64), count: i64) -> PyResult<Vec<i64>> {
    let count = usize::try_from(count)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| PyValueError::new_err("count must be positive"))?;
    Ok(TimeScale::new()
        .domain(domain.0, domain.1)
        .time_ticks(count))
}

#[pyfunction]
fn time_scale_interval(domain: (i64, i64)) -> &'static str {
    time_interval_name(TimeScale::new().domain(domain.0, domain.1).interval())
}

#[pyfunction]
fn timestamp_from_millis(millis: i64) -> i64 {
    d3rs::time::timestamp_from_millis(millis)
}

#[pyfunction]
fn millis_from_timestamp(timestamp: i64) -> i64 {
    d3rs::time::millis_from_timestamp(timestamp)
}

fn easing_value(name: &str, t: f64, apply: impl FnOnce(f64) -> f64) -> PyResult<f64> {
    finite("t", t)?;
    let result = apply(t);
    if result.is_finite() {
        Ok(result)
    } else {
        Err(PyValueError::new_err(format!(
            "{name} produced a non-finite result"
        )))
    }
}

macro_rules! easing_function {
    ($name:ident) => {
        #[pyfunction]
        fn $name(t: f64) -> PyResult<f64> {
            easing_value(stringify!($name), t, d3rs::ease::$name)
        }
    };
}

easing_function!(ease_linear);
easing_function!(ease_quad_in);
easing_function!(ease_quad_out);
easing_function!(ease_quad_in_out);
easing_function!(ease_cubic_in);
easing_function!(ease_cubic_out);
easing_function!(ease_cubic_in_out);
easing_function!(ease_sin_in);
easing_function!(ease_sin_out);
easing_function!(ease_sin_in_out);
easing_function!(ease_exp_in);
easing_function!(ease_exp_out);
easing_function!(ease_exp_in_out);
easing_function!(ease_circle_in);
easing_function!(ease_circle_out);
easing_function!(ease_circle_in_out);
easing_function!(ease_elastic_in);
easing_function!(ease_elastic_out);
easing_function!(ease_elastic_in_out);
easing_function!(ease_back_in);
easing_function!(ease_back_out);
easing_function!(ease_back_in_out);
easing_function!(ease_bounce_in);
easing_function!(ease_bounce_out);
easing_function!(ease_bounce_in_out);

macro_rules! parameterized_easing_function {
    ($name:ident, $parameter:ident) => {
        #[pyfunction]
        fn $name($parameter: f64, t: f64) -> PyResult<f64> {
            finite(stringify!($parameter), $parameter)?;
            easing_value(stringify!($name), t, d3rs::ease::$name($parameter))
        }
    };
}

parameterized_easing_function!(ease_poly_in, exponent);
parameterized_easing_function!(ease_poly_out, exponent);
parameterized_easing_function!(ease_poly_in_out, exponent);
parameterized_easing_function!(ease_back_in_with, overshoot);
parameterized_easing_function!(ease_back_out_with, overshoot);
parameterized_easing_function!(ease_back_in_out_with, overshoot);

macro_rules! elastic_easing_function {
    ($name:ident) => {
        #[pyfunction]
        fn $name(amplitude: f64, period: f64, t: f64) -> PyResult<f64> {
            finite("amplitude", amplitude)?;
            finite("period", period)?;
            if period <= 0.0 {
                return Err(PyValueError::new_err("period must be positive"));
            }
            easing_value(stringify!($name), t, d3rs::ease::$name(amplitude, period))
        }
    };
}

elastic_easing_function!(ease_elastic_in_with);
elastic_easing_function!(ease_elastic_out_with);

#[pyfunction]
fn abi3_minimum_python() -> &'static str {
    "3.10"
}

#[pyfunction]
fn dataset_arrow_ipc(
    py: Python<'_>,
    columns: Vec<(String, String, Vec<Py<PyAny>>)>,
) -> PyResult<Py<PyBytes>> {
    if columns.is_empty() {
        return Err(PyValueError::new_err(
            "dataset requires at least one column",
        ));
    }
    let row_count = columns[0].2.len();
    let mut arrays: Vec<(String, ArrayRef)> = Vec::with_capacity(columns.len());
    for (name, logical_type, values) in columns {
        if name.is_empty() {
            return Err(PyValueError::new_err(
                "dataset column name must be non-empty",
            ));
        }
        if values.len() != row_count {
            return Err(PyValueError::new_err(format!(
                "dataset column {name:?} has length {}, expected {row_count}",
                values.len()
            )));
        }
        let value_error = |index: usize| {
            PyValueError::new_err(format!(
                "dataset column {name:?} value at row {index} is incompatible with {logical_type}"
            ))
        };
        let array: ArrayRef = match logical_type.as_str() {
            "null" => Arc::new(NullArray::new(row_count)),
            "bool" => Arc::new(BooleanArray::from(
                values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        let value = value.bind(py);
                        if value.is_none() {
                            Ok(None)
                        } else {
                            value
                                .extract::<bool>()
                                .map(Some)
                                .map_err(|_| value_error(index))
                        }
                    })
                    .collect::<PyResult<Vec<_>>>()?,
            )),
            "int64" => Arc::new(Int64Array::from(
                values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        let value = value.bind(py);
                        if value.is_none() {
                            Ok(None)
                        } else {
                            value
                                .extract::<i64>()
                                .map(Some)
                                .map_err(|_| value_error(index))
                        }
                    })
                    .collect::<PyResult<Vec<_>>>()?,
            )),
            "float64" => Arc::new(Float64Array::from(
                values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        let value = value.bind(py);
                        if value.is_none() {
                            Ok(None)
                        } else {
                            value
                                .extract::<f64>()
                                .map(Some)
                                .map_err(|_| value_error(index))
                        }
                    })
                    .collect::<PyResult<Vec<_>>>()?,
            )),
            "utf8" | "dictionary" => Arc::new(StringArray::from(
                values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        let value = value.bind(py);
                        if value.is_none() {
                            Ok(None)
                        } else {
                            value
                                .extract::<String>()
                                .map(Some)
                                .map_err(|_| value_error(index))
                        }
                    })
                    .collect::<PyResult<Vec<_>>>()?,
            )),
            unsupported => {
                return Err(PyValueError::new_err(format!(
                    "built-in dataset transport does not support logical type {unsupported:?} in column {name:?}; install pyarrow for nested or temporal values"
                )));
            }
        };
        arrays.push((name, array));
    }
    let bytes = py
        .allow_threads(move || -> Result<Vec<u8>, String> {
            let batch = RecordBatch::try_from_iter(
                arrays
                    .iter()
                    .map(|(name, array)| (name.as_str(), array.clone())),
            )
            .map_err(|error| error.to_string())?;
            let mut bytes = Vec::new();
            {
                let mut writer = StreamWriter::try_new(&mut bytes, &batch.schema())
                    .map_err(|error| error.to_string())?;
                writer.write(&batch).map_err(|error| error.to_string())?;
                writer.finish().map_err(|error| error.to_string())?;
            }
            Ok(bytes)
        })
        .map_err(PyValueError::new_err)?;
    Ok(PyBytes::new(py, &bytes).unbind())
}

#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(interpolate_number, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_round, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_number_array, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_rgb, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_hsl, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_hsl_long, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_lab, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_hcl, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_hcl_long, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_cubehelix, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_cubehelix_long, module)?)?;
    module.add_function(wrap_pyfunction!(color_luminance, module)?)?;
    module.add_function(wrap_pyfunction!(color_lighten, module)?)?;
    module.add_function(wrap_pyfunction!(color_darken, module)?)?;
    module.add_function(wrap_pyfunction!(d3_color_rgb, module)?)?;
    module.add_function(wrap_pyfunction!(d3_color_from_hex, module)?)?;
    module.add_function(wrap_pyfunction!(d3_color_from_f32, module)?)?;
    module.add_function(wrap_pyfunction!(d3_color_from_hsl, module)?)?;
    module.add_function(wrap_pyfunction!(d3_color_transform, module)?)?;
    module.add_function(wrap_pyfunction!(d3_color_to_hex, module)?)?;
    module.add_function(wrap_pyfunction!(d3_color_luminance, module)?)?;
    module.add_function(wrap_pyfunction!(d3_color_to_lab, module)?)?;
    module.add_function(wrap_pyfunction!(d3_color_to_hcl, module)?)?;
    module.add_function(wrap_pyfunction!(d3_lab_create, module)?)?;
    module.add_function(wrap_pyfunction!(d3_lab_from_color, module)?)?;
    module.add_function(wrap_pyfunction!(d3_lab_to_color, module)?)?;
    module.add_function(wrap_pyfunction!(d3_lab_delta_e, module)?)?;
    module.add_function(wrap_pyfunction!(d3_lab_chroma, module)?)?;
    module.add_function(wrap_pyfunction!(d3_hcl_create, module)?)?;
    module.add_function(wrap_pyfunction!(d3_hcl_from_lab, module)?)?;
    module.add_function(wrap_pyfunction!(d3_hcl_from_color, module)?)?;
    module.add_function(wrap_pyfunction!(d3_hcl_to_lab, module)?)?;
    module.add_function(wrap_pyfunction!(d3_hcl_to_color, module)?)?;
    module.add_function(wrap_pyfunction!(d3_hcl_interpolate, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_hsl_value_new, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_hsl_value_from_color, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_hsl_value_to_color, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_cubehelix_value_new, module)?)?;
    module.add_function(wrap_pyfunction!(
        interpolate_cubehelix_value_from_color,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        interpolate_cubehelix_value_to_color,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(interpolate_cubehelix_default, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_cubehelix_custom, module)?)?;
    module.add_function(wrap_pyfunction!(d3_color_scheme, module)?)?;
    module.add_function(wrap_pyfunction!(d3_color_scheme_color, module)?)?;
    module.add_function(wrap_pyfunction!(d3_interpolate_colors, module)?)?;
    module.add_function(wrap_pyfunction!(d3_sequential_color, module)?)?;
    module.add_function(wrap_pyfunction!(d3_sequential_scheme_name, module)?)?;
    module.add_function(wrap_pyfunction!(d3_sequential_scale_get, module)?)?;
    module.add_function(wrap_pyfunction!(d3_sequential_scale_sample, module)?)?;
    module.add_function(wrap_pyfunction!(d3_diverging_scheme_name, module)?)?;
    module.add_function(wrap_pyfunction!(d3_diverging_scale_get, module)?)?;
    module.add_function(wrap_pyfunction!(d3_diverging_scale_sample, module)?)?;
    module.add_function(wrap_pyfunction!(px_color_scale_map, module)?)?;
    module.add_function(wrap_pyfunction!(px_color_scale_index, module)?)?;
    module.add_function(wrap_pyfunction!(px_color_range_resolve, module)?)?;
    module.add_function(wrap_pyfunction!(px_chart_keyboard_action, module)?)?;
    module.add_function(wrap_pyfunction!(px_chart_capability_report, module)?)?;
    module.add_function(wrap_pyfunction!(px_treemap_layout, module)?)?;
    module.add_function(wrap_pyfunction!(bisect_left, module)?)?;
    module.add_function(wrap_pyfunction!(bisect_right, module)?)?;
    module.add_function(wrap_pyfunction!(quantile, module)?)?;
    module.add_function(wrap_pyfunction!(quantile_sorted, module)?)?;
    module.add_function(wrap_pyfunction!(least_index, module)?)?;
    module.add_function(wrap_pyfunction!(array_min, module)?)?;
    module.add_function(wrap_pyfunction!(array_max, module)?)?;
    module.add_function(wrap_pyfunction!(min_index, module)?)?;
    module.add_function(wrap_pyfunction!(max_index, module)?)?;
    module.add_function(wrap_pyfunction!(array_sum, module)?)?;
    module.add_function(wrap_pyfunction!(mean, module)?)?;
    module.add_function(wrap_pyfunction!(median, module)?)?;
    module.add_function(wrap_pyfunction!(variance, module)?)?;
    module.add_function(wrap_pyfunction!(deviation, module)?)?;
    module.add_function(wrap_pyfunction!(extent, module)?)?;
    module.add_function(wrap_pyfunction!(cumsum, module)?)?;
    module.add_function(wrap_pyfunction!(histogram, module)?)?;
    module.add_function(wrap_pyfunction!(threshold_sturges, module)?)?;
    module.add_function(wrap_pyfunction!(nice_bin_edges, module)?)?;
    module.add_function(wrap_pyfunction!(reverse, module)?)?;
    module.add_function(wrap_pyfunction!(shuffle_seeded, module)?)?;
    module.add_function(wrap_pyfunction!(shuffle, module)?)?;
    module.add_function(wrap_pyfunction!(pairs, module)?)?;
    module.add_function(wrap_pyfunction!(cross, module)?)?;
    module.add_function(wrap_pyfunction!(unique, module)?)?;
    module.add_function(wrap_pyfunction!(array_sort, module)?)?;
    module.add_function(wrap_pyfunction!(sort_descending, module)?)?;
    module.add_function(wrap_pyfunction!(merge_sorted, module)?)?;
    module.add_function(wrap_pyfunction!(binary_search, module)?)?;
    module.add_function(wrap_pyfunction!(difference, module)?)?;
    module.add_function(wrap_pyfunction!(intersection, module)?)?;
    module.add_function(wrap_pyfunction!(array_union, module)?)?;
    module.add_function(wrap_pyfunction!(symmetric_difference, module)?)?;
    module.add_function(wrap_pyfunction!(is_subset, module)?)?;
    module.add_function(wrap_pyfunction!(is_superset, module)?)?;
    module.add_function(wrap_pyfunction!(is_disjoint, module)?)?;
    module.add_function(wrap_pyfunction!(ticks, module)?)?;
    module.add_function(wrap_pyfunction!(nice_number, module)?)?;
    module.add_function(wrap_pyfunction!(scale_nice_number, module)?)?;
    module.add_function(wrap_pyfunction!(generate_linear_ticks, module)?)?;
    module.add_function(wrap_pyfunction!(generate_log_ticks, module)?)?;
    module.add_function(wrap_pyfunction!(tick_step, module)?)?;
    module.add_function(wrap_pyfunction!(tick_increment, module)?)?;
    module.add_function(wrap_pyfunction!(nice, module)?)?;
    module.add_function(wrap_pyfunction!(ticks_interval, module)?)?;
    module.add_function(wrap_pyfunction!(log_ticks, module)?)?;
    module.add_function(wrap_pyfunction!(time_ticks, module)?)?;
    module.add_function(wrap_pyfunction!(linear_scale, module)?)?;
    module.add_function(wrap_pyfunction!(linear_scale_invert, module)?)?;
    module.add_function(wrap_pyfunction!(linear_scale_nice, module)?)?;
    module.add_function(wrap_pyfunction!(linear_scale_ticks, module)?)?;
    module.add_function(wrap_pyfunction!(log_scale, module)?)?;
    module.add_function(wrap_pyfunction!(log_scale_invert, module)?)?;
    module.add_function(wrap_pyfunction!(log_scale_ticks, module)?)?;
    module.add_function(wrap_pyfunction!(pow_scale, module)?)?;
    module.add_function(wrap_pyfunction!(pow_scale_invert, module)?)?;
    module.add_function(wrap_pyfunction!(pow_scale_nice, module)?)?;
    module.add_function(wrap_pyfunction!(pow_scale_ticks, module)?)?;
    module.add_function(wrap_pyfunction!(symlog_scale, module)?)?;
    module.add_function(wrap_pyfunction!(symlog_scale_invert, module)?)?;
    module.add_function(wrap_pyfunction!(symlog_scale_nice, module)?)?;
    module.add_function(wrap_pyfunction!(symlog_scale_ticks, module)?)?;
    module.add_function(wrap_pyfunction!(threshold_scale_index, module)?)?;
    module.add_function(wrap_pyfunction!(threshold_scale_invert_extent, module)?)?;
    module.add_function(wrap_pyfunction!(quantize_scale_index, module)?)?;
    module.add_function(wrap_pyfunction!(quantize_scale_thresholds, module)?)?;
    module.add_function(wrap_pyfunction!(quantize_scale_invert_extent, module)?)?;
    module.add_function(wrap_pyfunction!(quantile_scale_prepare, module)?)?;
    module.add_function(wrap_pyfunction!(quantile_scale_index, module)?)?;
    module.add_function(wrap_pyfunction!(quantile_scale_invert_extent, module)?)?;
    module.add_function(wrap_pyfunction!(band_scale_layout, module)?)?;
    module.add_function(wrap_pyfunction!(point_scale_layout, module)?)?;
    module.add_function(wrap_pyfunction!(clamp01, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_clamped, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_basis, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_basis_closed, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_exp, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_discrete, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_quantize, module)?)?;
    module.add_function(wrap_pyfunction!(piecewise, module)?)?;
    module.add_function(wrap_pyfunction!(piecewise_domain, module)?)?;
    module.add_function(wrap_pyfunction!(quantize_values, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_ease, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_matrix, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_zoom_vector, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_string, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_transform_css, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_date, module)?)?;
    module.add_function(wrap_pyfunction!(transform_identity, module)?)?;
    module.add_function(wrap_pyfunction!(transform_translate, module)?)?;
    module.add_function(wrap_pyfunction!(transform_rotate_deg, module)?)?;
    module.add_function(wrap_pyfunction!(transform_rotate_rad, module)?)?;
    module.add_function(wrap_pyfunction!(transform_scale, module)?)?;
    module.add_function(wrap_pyfunction!(transform_scale_uniform, module)?)?;
    module.add_function(wrap_pyfunction!(transform_skew_x_deg, module)?)?;
    module.add_function(wrap_pyfunction!(transform_from_matrix, module)?)?;
    module.add_function(wrap_pyfunction!(transform_to_matrix, module)?)?;
    module.add_function(wrap_pyfunction!(transform_apply, module)?)?;
    module.add_function(wrap_pyfunction!(transform_interpolate, module)?)?;
    module.add_function(wrap_pyfunction!(transform_to_css, module)?)?;
    module.add_function(wrap_pyfunction!(transform_to_svg, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_transform_svg, module)?)?;
    module.add_function(wrap_pyfunction!(interpolate_zoom_view, module)?)?;
    module.add_function(wrap_pyfunction!(zoom_duration, module)?)?;
    module.add_class::<PyDensityPyramid>()?;
    module.add_function(wrap_pyfunction!(lod_m4_indices, module)?)?;
    module.add_function(wrap_pyfunction!(lod_m4_point_indices, module)?)?;
    module.add_function(wrap_pyfunction!(hierarchy_sum, module)?)?;
    module.add_function(wrap_pyfunction!(hierarchy_count, module)?)?;
    module.add_function(wrap_pyfunction!(hierarchy_rect_layout, module)?)?;
    module.add_function(wrap_pyfunction!(hierarchy_pack_layout, module)?)?;
    module.add_function(wrap_pyfunction!(hierarchy_point_layout, module)?)?;
    module.add_function(wrap_pyfunction!(contour_ring_metrics, module)?)?;
    module.add_function(wrap_pyfunction!(contour_generate, module)?)?;
    module.add_function(wrap_pyfunction!(contour_band_generate, module)?)?;
    module.add_function(wrap_pyfunction!(contour_segment_generate, module)?)?;
    module.add_function(wrap_pyfunction!(density_kernel, module)?)?;
    module.add_function(wrap_pyfunction!(density_estimate, module)?)?;
    module.add_function(wrap_pyfunction!(density_estimate_weighted, module)?)?;
    module.add_function(wrap_pyfunction!(density_2d_auto, module)?)?;
    module.add_function(wrap_pyfunction!(contour_threshold_sturges, module)?)?;
    module.add_function(wrap_pyfunction!(contour_threshold_scott, module)?)?;
    module.add_function(wrap_pyfunction!(
        contour_threshold_freedman_diaconis,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(polygon_area_value, module)?)?;
    module.add_function(wrap_pyfunction!(polygon_area_signed_value, module)?)?;
    module.add_function(wrap_pyfunction!(polygon_centroid_value, module)?)?;
    module.add_function(wrap_pyfunction!(polygon_contains_value, module)?)?;
    module.add_function(wrap_pyfunction!(polygon_length_value, module)?)?;
    module.add_function(wrap_pyfunction!(polygon_hull_value, module)?)?;
    module.add_function(wrap_pyfunction!(delaunay_snapshot, module)?)?;
    module.add_function(wrap_pyfunction!(delaunay_find, module)?)?;
    module.add_function(wrap_pyfunction!(force_simulate, module)?)?;
    module.add_function(wrap_pyfunction!(shape_arc, module)?)?;
    module.add_function(wrap_pyfunction!(shape_symbol, module)?)?;
    module.add_function(wrap_pyfunction!(shape_link, module)?)?;
    module.add_function(wrap_pyfunction!(shape_radial_link, module)?)?;
    module.add_function(wrap_pyfunction!(shape_pie, module)?)?;
    module.add_function(wrap_pyfunction!(shape_stack, module)?)?;
    module.add_function(wrap_pyfunction!(shape_curve_interpolate, module)?)?;
    module.add_function(wrap_pyfunction!(radial_point_to_cartesian, module)?)?;
    module.add_function(wrap_pyfunction!(radial_point_from_cartesian, module)?)?;
    module.add_function(wrap_pyfunction!(radial_line_path, module)?)?;
    module.add_function(wrap_pyfunction!(radial_area_path, module)?)?;
    module.add_function(wrap_pyfunction!(polar_grid_circle_paths, module)?)?;
    module.add_function(wrap_pyfunction!(polar_grid_ray_paths, module)?)?;
    module.add_function(wrap_pyfunction!(shape_path_analyze, module)?)?;
    module.add_function(wrap_pyfunction!(shape_point_distance, module)?)?;
    module.add_function(wrap_pyfunction!(shape_point_lerp, module)?)?;
    module.add_function(wrap_pyfunction!(shape_area_generate, module)?)?;
    module.add_function(wrap_pyfunction!(shape_simple_area, module)?)?;
    module.add_function(wrap_pyfunction!(chord_layout, module)?)?;
    module.add_function(wrap_pyfunction!(chord_ribbon_path, module)?)?;
    module.add_class::<PyLcgRng>()?;
    module.add_class::<NativeTransitionState>()?;
    module.add_class::<NativeTimerHandle>()?;
    module.add_class::<NativeDragState>()?;
    module.add_class::<NativeBrushState>()?;
    module.add_class::<NativeZoomState>()?;
    module.add_class::<NativePxChartInteraction>()?;
    module.add_class::<NativePxMeshPickIndex>()?;
    module.add_class::<PyRandomUniform>()?;
    module.add_class::<PyRandomNormal>()?;
    module.add_class::<PyRandomLogNormal>()?;
    module.add_class::<PyRandomExponential>()?;
    module.add_class::<PyRandomBernoulli>()?;
    module.add_class::<PyRandomPoisson>()?;
    module.add_class::<PyRandomIrwinHall>()?;
    module.add_class::<PyRandomBates>()?;
    module.add_class::<NativeDsvCancellationToken>()?;
    module.add_class::<NativeQuadTreeIndex>()?;
    module.add_function(wrap_pyfunction!(geo_radians_value, module)?)?;
    module.add_function(wrap_pyfunction!(geo_degrees_value, module)?)?;
    module.add_function(wrap_pyfunction!(geo_distance_value, module)?)?;
    module.add_function(wrap_pyfunction!(geo_length_value, module)?)?;
    module.add_function(wrap_pyfunction!(geo_interpolate_value, module)?)?;
    module.add_function(wrap_pyfunction!(geo_area_value, module)?)?;
    module.add_function(wrap_pyfunction!(geo_bounds_value, module)?)?;
    module.add_function(wrap_pyfunction!(geo_centroid_value, module)?)?;
    module.add_function(wrap_pyfunction!(geo_contains_value, module)?)?;
    module.add_function(wrap_pyfunction!(geo_graticule, module)?)?;
    module.add_function(wrap_pyfunction!(geo_rotation_value, module)?)?;
    module.add_function(wrap_pyfunction!(geo_versor_from_angles, module)?)?;
    module.add_function(wrap_pyfunction!(geo_versor_to_angles, module)?)?;
    module.add_function(wrap_pyfunction!(geo_versor_from_cartesian, module)?)?;
    module.add_function(wrap_pyfunction!(geo_spherical_to_cartesian, module)?)?;
    module.add_function(wrap_pyfunction!(geo_versor_multiply, module)?)?;
    module.add_function(wrap_pyfunction!(geo_versor_dot, module)?)?;
    module.add_function(wrap_pyfunction!(geo_versor_unary, module)?)?;
    module.add_function(wrap_pyfunction!(geo_versor_delta, module)?)?;
    module.add_function(wrap_pyfunction!(geo_versor_slerp, module)?)?;
    module.add_function(wrap_pyfunction!(geo_versor_rotate_spherical, module)?)?;
    module.add_function(wrap_pyfunction!(geo_versor_rotate_degrees, module)?)?;
    module.add_function(wrap_pyfunction!(geo_projection_apply, module)?)?;
    module.add_function(wrap_pyfunction!(geo_projection_metadata, module)?)?;
    module.add_function(wrap_pyfunction!(geo_projection_visible, module)?)?;
    module.add_function(wrap_pyfunction!(geo_path_render, module)?)?;
    module.add_function(wrap_pyfunction!(geo_path_bounds, module)?)?;
    module.add_function(wrap_pyfunction!(geo_path_centroid, module)?)?;
    module.add_function(wrap_pyfunction!(geo_path_project_coords, module)?)?;
    module.add_function(wrap_pyfunction!(geo_stream_events, module)?)?;
    module.add_function(wrap_pyfunction!(topojson_parse_land, module)?)?;
    module.add_function(wrap_pyfunction!(topojson_parse_land_with_budget, module)?)?;
    module.add_function(wrap_pyfunction!(fetch_auto_type_values, module)?)?;
    module.add_function(wrap_pyfunction!(fetch_parse_dsv, module)?)?;
    module.add_function(wrap_pyfunction!(fetch_parse_dsv_rows, module)?)?;
    module.add_function(wrap_pyfunction!(fetch_format_dsv, module)?)?;
    module.add_function(wrap_pyfunction!(tile_layout, module)?)?;
    module.add_function(wrap_pyfunction!(hexbin_bin, module)?)?;
    module.add_function(wrap_pyfunction!(hexbin_hexagon, module)?)?;
    module.add_function(wrap_pyfunction!(hexbin_centers, module)?)?;
    module.add_function(wrap_pyfunction!(sankey_layout, module)?)?;
    module.add_function(wrap_pyfunction!(legend_layout, module)?)?;
    module.add_function(wrap_pyfunction!(axis_layout, module)?)?;
    module.add_function(wrap_pyfunction!(grid_layout, module)?)?;
    module.add_function(wrap_pyfunction!(brush_to_domain, module)?)?;
    module.add_function(wrap_pyfunction!(parse_format_specifier, module)?)?;
    module.add_function(wrap_pyfunction!(format_value, module)?)?;
    module.add_function(wrap_pyfunction!(format_prefix_value, module)?)?;
    module.add_function(wrap_pyfunction!(prefix_exponent, module)?)?;
    module.add_function(wrap_pyfunction!(format_locale_value, module)?)?;
    module.add_function(wrap_pyfunction!(time_interval_floor, module)?)?;
    module.add_function(wrap_pyfunction!(time_interval_ceil, module)?)?;
    module.add_function(wrap_pyfunction!(time_interval_round, module)?)?;
    module.add_function(wrap_pyfunction!(time_interval_offset, module)?)?;
    module.add_function(wrap_pyfunction!(time_interval_count, module)?)?;
    module.add_function(wrap_pyfunction!(time_interval_range, module)?)?;
    module.add_function(wrap_pyfunction!(time_interval_duration, module)?)?;
    module.add_function(wrap_pyfunction!(time_interval_format_pattern, module)?)?;
    module.add_function(wrap_pyfunction!(time_interval_for_span, module)?)?;
    module.add_function(wrap_pyfunction!(time_format_parts, module)?)?;
    module.add_function(wrap_pyfunction!(time_format_value, module)?)?;
    module.add_function(wrap_pyfunction!(time_scale_value, module)?)?;
    module.add_function(wrap_pyfunction!(time_scale_invert, module)?)?;
    module.add_function(wrap_pyfunction!(time_scale_nice, module)?)?;
    module.add_function(wrap_pyfunction!(time_scale_ticks, module)?)?;
    module.add_function(wrap_pyfunction!(time_scale_interval, module)?)?;
    module.add_function(wrap_pyfunction!(timestamp_from_millis, module)?)?;
    module.add_function(wrap_pyfunction!(millis_from_timestamp, module)?)?;
    module.add_function(wrap_pyfunction!(ease_linear, module)?)?;
    module.add_function(wrap_pyfunction!(ease_quad_in, module)?)?;
    module.add_function(wrap_pyfunction!(ease_quad_out, module)?)?;
    module.add_function(wrap_pyfunction!(ease_quad_in_out, module)?)?;
    module.add_function(wrap_pyfunction!(ease_cubic_in, module)?)?;
    module.add_function(wrap_pyfunction!(ease_cubic_out, module)?)?;
    module.add_function(wrap_pyfunction!(ease_cubic_in_out, module)?)?;
    module.add_function(wrap_pyfunction!(ease_poly_in, module)?)?;
    module.add_function(wrap_pyfunction!(ease_poly_out, module)?)?;
    module.add_function(wrap_pyfunction!(ease_poly_in_out, module)?)?;
    module.add_function(wrap_pyfunction!(ease_sin_in, module)?)?;
    module.add_function(wrap_pyfunction!(ease_sin_out, module)?)?;
    module.add_function(wrap_pyfunction!(ease_sin_in_out, module)?)?;
    module.add_function(wrap_pyfunction!(ease_exp_in, module)?)?;
    module.add_function(wrap_pyfunction!(ease_exp_out, module)?)?;
    module.add_function(wrap_pyfunction!(ease_exp_in_out, module)?)?;
    module.add_function(wrap_pyfunction!(ease_circle_in, module)?)?;
    module.add_function(wrap_pyfunction!(ease_circle_out, module)?)?;
    module.add_function(wrap_pyfunction!(ease_circle_in_out, module)?)?;
    module.add_function(wrap_pyfunction!(ease_elastic_in_with, module)?)?;
    module.add_function(wrap_pyfunction!(ease_elastic_out_with, module)?)?;
    module.add_function(wrap_pyfunction!(ease_elastic_in, module)?)?;
    module.add_function(wrap_pyfunction!(ease_elastic_out, module)?)?;
    module.add_function(wrap_pyfunction!(ease_elastic_in_out, module)?)?;
    module.add_function(wrap_pyfunction!(ease_back_in_with, module)?)?;
    module.add_function(wrap_pyfunction!(ease_back_out_with, module)?)?;
    module.add_function(wrap_pyfunction!(ease_back_in_out_with, module)?)?;
    module.add_function(wrap_pyfunction!(ease_back_in, module)?)?;
    module.add_function(wrap_pyfunction!(ease_back_out, module)?)?;
    module.add_function(wrap_pyfunction!(ease_back_in_out, module)?)?;
    module.add_function(wrap_pyfunction!(ease_bounce_out, module)?)?;
    module.add_function(wrap_pyfunction!(ease_bounce_in, module)?)?;
    module.add_function(wrap_pyfunction!(ease_bounce_in_out, module)?)?;
    module.add_function(wrap_pyfunction!(abi3_minimum_python, module)?)?;
    module.add_function(wrap_pyfunction!(dataset_arrow_ipc, module)?)?;
    module.add_function(wrap_pyfunction!(timer_now, module)?)?;
    module.add_function(wrap_pyfunction!(timer_set_now, module)?)?;
    module.add_function(wrap_pyfunction!(timer_flush, module)?)?;
    Ok(())
}
