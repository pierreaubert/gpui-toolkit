"""Optional pure-computation functions supplied by the abi3 wheel extension."""

from __future__ import annotations

import builtins
import math
from dataclasses import dataclass, field, replace
from enum import Enum
from functools import cmp_to_key
from inspect import signature
from typing import Callable, Final, Iterable, Mapping, Protocol, Sequence

try:
    from ._native import (
        _LcgRng as _NativeLcgRng,
        _RandomBates as _NativeRandomBates,
        _DsvCancellationToken as _NativeDsvCancellationToken,
        _QuadTreeIndex as _NativeQuadTreeIndex,
        _TransitionState as _NativeTransitionState,
        _TimerResource as _NativeTimerResource,
        _DragState as _NativeDragState,
        _BrushState as _NativeBrushState,
        _ZoomState as _NativeZoomState,
        brush_to_domain as _brush_to_domain,
        timer_now as _timer_now,
        timer_set_now as _timer_set_now,
        timer_flush as _timer_flush,
        _RandomBernoulli as _NativeRandomBernoulli,
        _RandomExponential as _NativeRandomExponential,
        _RandomIrwinHall as _NativeRandomIrwinHall,
        _RandomLogNormal as _NativeRandomLogNormal,
        _RandomNormal as _NativeRandomNormal,
        _RandomPoisson as _NativeRandomPoisson,
        _RandomUniform as _NativeRandomUniform,
        _format_locale_value,
        _format_prefix_value,
        interpolate_transform_svg as _interpolate_transform_svg,
        interpolate_zoom_view as _interpolate_zoom_view,
        millis_from_timestamp as _millis_from_timestamp,
        abi3_minimum_python,
        binary_search,
        bisect_left,
        bisect_right,
        clamp01,
        color_darken,
        d3_color_rgb,
        d3_color_from_hex,
        d3_color_from_f32,
        d3_color_from_hsl,
        d3_color_transform,
        d3_color_to_hex,
        d3_color_luminance,
        d3_color_to_lab,
        d3_color_to_hcl,
        d3_lab_create,
        d3_lab_from_color,
        d3_lab_to_color,
        d3_lab_delta_e,
        d3_lab_chroma,
        d3_hcl_create,
        d3_hcl_from_lab,
        d3_hcl_from_color,
        d3_hcl_to_lab,
        d3_hcl_to_color,
        d3_hcl_interpolate,
        interpolate_hsl_value_new,
        interpolate_hsl_value_from_color,
        interpolate_hsl_value_to_color,
        interpolate_cubehelix_value_new,
        interpolate_cubehelix_value_from_color,
        interpolate_cubehelix_value_to_color,
        interpolate_cubehelix_default,
        interpolate_cubehelix_custom,
        d3_color_scheme,
        d3_color_scheme_color,
        d3_interpolate_colors,
        d3_sequential_color,
        d3_sequential_scheme_name,
        d3_sequential_scale_get,
        d3_sequential_scale_sample,
        d3_diverging_scheme_name,
        d3_diverging_scale_get,
        d3_diverging_scale_sample,
        color_lighten,
        color_luminance,
        cross,
        cumsum,
        deviation,
        difference,
        ease_back_in,
        ease_back_in_out,
        ease_back_in_out_with,
        ease_back_in_with,
        ease_back_out,
        ease_back_out_with,
        ease_bounce_in,
        ease_bounce_in_out,
        ease_bounce_out,
        ease_circle_in,
        ease_circle_in_out,
        ease_circle_out,
        ease_cubic_in,
        ease_cubic_in_out,
        ease_cubic_out,
        ease_elastic_in,
        ease_elastic_in_out,
        ease_elastic_in_with,
        ease_elastic_out,
        ease_elastic_out_with,
        ease_exp_in,
        ease_exp_in_out,
        ease_exp_out,
        ease_linear,
        ease_poly_in,
        ease_poly_in_out,
        ease_poly_out,
        ease_quad_in,
        ease_quad_in_out,
        ease_quad_out,
        ease_sin_in,
        ease_sin_in_out,
        ease_sin_out,
        extent,
        format_value as _format_value,
        contour_band_generate as _contour_band_generate,
        contour_generate as _contour_generate,
        contour_ring_metrics as _contour_ring_metrics,
        contour_segment_generate as _contour_segment_generate,
        contour_threshold_freedman_diaconis as _contour_threshold_freedman_diaconis,
        contour_threshold_scott as _contour_threshold_scott,
        contour_threshold_sturges as _contour_threshold_sturges,
        density_2d_auto as _density_2d_auto,
        density_estimate as _density_estimate,
        density_estimate_weighted as _density_estimate_weighted,
        density_kernel as _density_kernel,
        delaunay_find as _delaunay_find,
        delaunay_snapshot as _delaunay_snapshot,
        polygon_area_signed_value as _polygon_area_signed,
        polygon_area_value as _polygon_area,
        polygon_centroid_value as _polygon_centroid,
        polygon_contains_value as _polygon_contains,
        polygon_hull_value as _polygon_hull,
        polygon_length_value as _polygon_length,
        force_simulate as _force_simulate,
        shape_arc as _shape_arc,
        shape_link as _shape_link,
        shape_pie as _shape_pie,
        shape_radial_link as _shape_radial_link,
        shape_stack as _shape_stack,
        shape_curve_interpolate as _shape_curve_interpolate,
        shape_path_analyze as _shape_path_analyze,
        shape_point_distance as _shape_point_distance,
        shape_point_lerp as _shape_point_lerp,
        shape_area_generate as _shape_area_generate,
        shape_simple_area as _shape_simple_area,
        chord_layout as _chord_layout,
        chord_ribbon_path as _chord_ribbon_path,
        geo_radians_value as _geo_radians,
        geo_degrees_value as _geo_degrees,
        geo_distance_value as _geo_distance,
        geo_length_value as _geo_length,
        geo_interpolate_value as _geo_interpolate,
        geo_area_value as _geo_area,
        geo_bounds_value as _geo_bounds,
        geo_centroid_value as _geo_centroid,
        geo_contains_value as _geo_contains,
        geo_graticule as _geo_graticule,
        geo_rotation_value as _geo_rotation,
        geo_versor_from_angles as _geo_versor_from_angles,
        geo_versor_to_angles as _geo_versor_to_angles,
        geo_versor_from_cartesian as _geo_versor_from_cartesian,
        geo_spherical_to_cartesian as _geo_spherical_to_cartesian,
        geo_versor_multiply as _geo_versor_multiply,
        geo_versor_dot as _geo_versor_dot,
        geo_versor_unary as _geo_versor_unary,
        geo_versor_delta as _geo_versor_delta,
        geo_versor_slerp as _geo_versor_slerp,
        geo_versor_rotate_spherical as _geo_versor_rotate_spherical,
        geo_versor_rotate_degrees as _geo_versor_rotate_degrees,
        geo_projection_apply as _geo_projection_apply,
        geo_projection_metadata as _geo_projection_metadata,
        geo_projection_visible as _geo_projection_visible,
        geo_path_render as _geo_path_render,
        geo_path_bounds as _geo_path_bounds,
        geo_path_centroid as _geo_path_centroid,
        geo_path_project_coords as _geo_path_project_coords,
        geo_stream_events as _geo_stream_events,
        topojson_parse_land as _topojson_parse_land,
        topojson_parse_land_with_budget as _topojson_parse_land_with_budget,
        fetch_auto_type_values as _fetch_auto_type_values,
        fetch_parse_dsv as _fetch_parse_dsv,
        fetch_parse_dsv_rows as _fetch_parse_dsv_rows,
        fetch_format_dsv as _fetch_format_dsv,
        tile_layout as _tile_layout,
        hexbin_bin as _hexbin_bin,
        hexbin_hexagon as _hexbin_hexagon,
        hexbin_centers as _hexbin_centers,
        sankey_layout as _sankey_layout,
        legend_layout as _legend_layout,
        axis_layout as _axis_layout,
        grid_layout as _grid_layout,
        shape_symbol as _shape_symbol,
        radial_point_to_cartesian as _radial_point_to_cartesian,
        radial_point_from_cartesian as _radial_point_from_cartesian,
        radial_line_path as _radial_line_path,
        radial_area_path as _radial_area_path,
        polar_grid_circle_paths as _polar_grid_circle_paths,
        polar_grid_ray_paths as _polar_grid_ray_paths,
        _DensityPyramid as _NativeDensityPyramid,
        lod_m4_indices as _lod_m4_indices,
        lod_m4_point_indices as _lod_m4_point_indices,
        hierarchy_count as _hierarchy_count,
        hierarchy_pack_layout as _hierarchy_pack_layout,
        hierarchy_point_layout as _hierarchy_point_layout,
        hierarchy_rect_layout as _hierarchy_rect_layout,
        hierarchy_sum as _hierarchy_sum,
        histogram as _histogram,
        intersection,
        interpolate_basis,
        interpolate_basis_closed,
        interpolate_clamped,
        interpolate_cubehelix,
        interpolate_cubehelix_long,
        interpolate_date,
        interpolate_discrete,
        interpolate_ease as _interpolate_ease,
        interpolate_exp,
        interpolate_hcl,
        interpolate_hcl_long,
        interpolate_hsl,
        interpolate_hsl_long,
        interpolate_lab,
        interpolate_matrix,
        interpolate_number,
        interpolate_number_array,
        interpolate_rgb,
        interpolate_round,
        interpolate_string,
        interpolate_transform_css,
        interpolate_quantize,
        interpolate_zoom_vector,
        is_disjoint,
        is_subset,
        is_superset,
        least_index,
        linear_scale,
        linear_scale_invert,
        linear_scale_nice,
        linear_scale_ticks,
        log_scale,
        log_scale_invert,
        log_scale_ticks,
        pow_scale,
        pow_scale_invert,
        pow_scale_nice,
        pow_scale_ticks,
        symlog_scale,
        symlog_scale_invert,
        symlog_scale_nice,
        symlog_scale_ticks,
        threshold_scale_index,
        threshold_scale_invert_extent,
        quantize_scale_index,
        quantize_scale_thresholds,
        quantize_scale_invert_extent,
        quantile_scale_prepare,
        quantile_scale_index,
        quantile_scale_invert_extent,
        band_scale_layout,
        point_scale_layout,
        log_ticks,
        max,
        max_index,
        mean,
        merge_sorted,
        median,
        min,
        min_index,
        nice,
        nice_bin_edges,
        nice_number,
        scale_nice_number,
        generate_linear_ticks,
        generate_log_ticks,
        pairs,
        parse_format_specifier as _parse_format_specifier,
        piecewise,
        piecewise_domain,
        prefix_exponent as _prefix_exponent,
        quantile,
        quantile_sorted,
        quantize_values as _quantize_values,
        reverse,
        shuffle as _shuffle_values,
        shuffle_seeded,
        sort,
        sort_descending,
        sum,
        symmetric_difference,
        tick_increment,
        tick_step,
        ticks,
        ticks_interval,
        time_ticks,
        threshold_sturges,
        time_format_parts as _time_format_parts,
        time_format_value as _time_format_value,
        time_interval_ceil as _time_interval_ceil,
        time_interval_count as _time_interval_count,
        time_interval_duration as _time_interval_duration,
        time_interval_floor as _time_interval_floor,
        time_interval_for_span as _time_interval_for_span,
        time_interval_format_pattern as _time_interval_format_pattern,
        time_interval_offset as _time_interval_offset,
        time_interval_range as _time_interval_range,
        time_interval_round as _time_interval_round,
        time_scale_interval as _time_scale_interval,
        time_scale_invert as _time_scale_invert,
        time_scale_nice as _time_scale_nice,
        time_scale_ticks as _time_scale_ticks,
        time_scale_value as _time_scale_value,
        transform_apply as _transform_apply,
        transform_from_matrix as _transform_from_matrix,
        transform_identity as _transform_identity,
        transform_interpolate as _transform_interpolate,
        transform_rotate_deg as _transform_rotate_deg,
        transform_rotate_rad as _transform_rotate_rad,
        transform_scale as _transform_scale,
        transform_scale_uniform as _transform_scale_uniform,
        transform_skew_x_deg as _transform_skew_x_deg,
        transform_to_css as _transform_to_css,
        transform_to_matrix as _transform_to_matrix,
        transform_to_svg as _transform_to_svg,
        transform_translate as _transform_translate,
        union,
        unique,
        variance,
        timestamp_from_millis as _timestamp_from_millis,
        zoom_duration as _zoom_duration,
    )
except ImportError:  # Source declarations remain usable without a built wheel.
    AVAILABLE = False

    def abi3_minimum_python() -> str:
        return "3.10"

    def linear_scale(
        value: float,
        *,
        domain: tuple[float, float],
        range: tuple[float, float],
        clamp: bool = False,
    ) -> float:
        raise RuntimeError("gpui_toolkit native extension is not installed")

    def _missing(*_args: object, **_kwargs: object) -> object:
        raise RuntimeError("gpui_toolkit native extension is not installed")

    linear_scale_invert = _missing
    linear_scale_nice = _missing
    linear_scale_ticks = _missing
    log_scale = _missing
    log_scale_invert = _missing
    log_scale_ticks = _missing
    pow_scale = _missing
    pow_scale_invert = _missing
    pow_scale_nice = _missing
    pow_scale_ticks = _missing
    symlog_scale = _missing
    symlog_scale_invert = _missing
    symlog_scale_nice = _missing
    symlog_scale_ticks = _missing
    threshold_scale_index = _missing
    threshold_scale_invert_extent = _missing
    quantize_scale_index = _missing
    quantize_scale_thresholds = _missing
    quantize_scale_invert_extent = _missing
    quantile_scale_prepare = _missing
    quantile_scale_index = _missing
    quantile_scale_invert_extent = _missing
    band_scale_layout = _missing
    point_scale_layout = _missing
    _format_locale_value = _missing
    d3_color_rgb = _missing
    d3_color_from_hex = _missing
    d3_color_from_f32 = _missing
    d3_color_from_hsl = _missing
    d3_color_transform = _missing
    d3_color_to_hex = _missing
    d3_color_luminance = _missing
    d3_color_to_lab = _missing
    d3_color_to_hcl = _missing
    d3_lab_create = _missing
    d3_lab_from_color = _missing
    d3_lab_to_color = _missing
    d3_lab_delta_e = _missing
    d3_lab_chroma = _missing
    d3_hcl_create = _missing
    d3_hcl_from_lab = _missing
    d3_hcl_from_color = _missing
    d3_hcl_to_lab = _missing
    d3_hcl_to_color = _missing
    d3_hcl_interpolate = _missing
    d3_color_scheme = _missing
    d3_color_scheme_color = _missing
    d3_interpolate_colors = _missing
    d3_sequential_color = _missing
    d3_sequential_scheme_name = _missing
    d3_sequential_scale_get = _missing
    d3_sequential_scale_sample = _missing
    d3_diverging_scheme_name = _missing
    d3_diverging_scale_get = _missing
    d3_diverging_scale_sample = _missing
    _format_prefix_value = _missing
    _format_value = _missing
    _parse_format_specifier = _missing
    _interpolate_ease = _missing
    _quantize_values = _missing
    _interpolate_transform_svg = _missing
    _interpolate_zoom_view = _missing
    _time_format_parts = _missing
    _time_format_value = _missing
    _time_interval_ceil = _missing
    _time_interval_count = _missing
    _time_interval_duration = _missing
    _time_interval_floor = _missing
    _time_interval_for_span = _missing
    _time_interval_format_pattern = _missing
    _time_interval_offset = _missing
    _time_interval_range = _missing
    _time_interval_round = _missing
    _time_scale_interval = _missing
    _time_scale_invert = _missing
    _time_scale_nice = _missing
    _time_scale_ticks = _missing
    _time_scale_value = _missing
    _transform_apply = _missing
    _transform_from_matrix = _missing
    _transform_identity = _missing
    _transform_interpolate = _missing
    _transform_rotate_deg = _missing
    _transform_rotate_rad = _missing
    _transform_scale = _missing
    _transform_scale_uniform = _missing
    _transform_skew_x_deg = _missing
    _transform_to_css = _missing
    _transform_to_matrix = _missing
    _transform_to_svg = _missing
    _transform_translate = _missing
    _zoom_duration = _missing
    _NativeDensityPyramid = None
    _lod_m4_indices = _missing
    _lod_m4_point_indices = _missing
    _hierarchy_count = _missing
    _hierarchy_pack_layout = _missing
    _hierarchy_point_layout = _missing
    _hierarchy_rect_layout = _missing
    _hierarchy_sum = _missing
    _contour_band_generate = _missing
    _contour_generate = _missing
    _contour_ring_metrics = _missing
    _contour_segment_generate = _missing
    _contour_threshold_freedman_diaconis = _missing
    _contour_threshold_scott = _missing
    _contour_threshold_sturges = _missing
    _density_2d_auto = _missing
    _density_estimate = _missing
    _density_estimate_weighted = _missing
    _density_kernel = _missing
    _delaunay_find = _missing
    _delaunay_snapshot = _missing
    _polygon_area = _missing
    _polygon_area_signed = _missing
    _polygon_centroid = _missing
    _polygon_contains = _missing
    _polygon_hull = _missing
    _polygon_length = _missing
    _force_simulate = _missing
    _shape_arc = _missing
    _shape_link = _missing
    _shape_pie = _missing
    _shape_radial_link = _missing
    _shape_stack = _missing
    _shape_curve_interpolate = _missing
    _shape_path_analyze = _missing
    _shape_point_distance = _missing
    _shape_point_lerp = _missing
    _shape_area_generate = _missing
    _shape_simple_area = _missing
    _chord_layout = _missing
    _chord_ribbon_path = _missing
    _NativeLcgRng = _missing
    _NativeRandomUniform = _missing
    _NativeRandomNormal = _missing
    _NativeRandomLogNormal = _missing
    _NativeRandomExponential = _missing
    _NativeRandomBernoulli = _missing
    _NativeRandomPoisson = _missing
    _NativeRandomIrwinHall = _missing
    _NativeRandomBates = _missing
    _geo_radians = _missing
    _geo_degrees = _missing
    _geo_distance = _missing
    _geo_length = _missing
    _geo_interpolate = _missing
    _geo_area = _missing
    _geo_bounds = _missing
    _geo_centroid = _missing
    _geo_contains = _missing
    _geo_graticule = _missing
    _geo_rotation = _missing
    _geo_versor_from_angles = _missing
    _geo_versor_to_angles = _missing
    _geo_versor_from_cartesian = _missing
    _geo_spherical_to_cartesian = _missing
    _geo_versor_multiply = _missing
    _geo_versor_dot = _missing
    _geo_versor_unary = _missing
    _geo_versor_delta = _missing
    _geo_versor_slerp = _missing
    _geo_versor_rotate_spherical = _missing
    _geo_versor_rotate_degrees = _missing
    _geo_projection_apply = _missing
    _geo_projection_metadata = _missing
    _geo_projection_visible = _missing
    _geo_path_render = _missing
    _geo_path_bounds = _missing
    _geo_path_centroid = _missing
    _geo_path_project_coords = _missing
    _geo_stream_events = _missing
    _topojson_parse_land = _missing
    _topojson_parse_land_with_budget = _missing
    _fetch_auto_type_values = _missing
    _fetch_parse_dsv = _missing
    _fetch_parse_dsv_rows = _missing
    _fetch_format_dsv = _missing
    _tile_layout = _missing
    _hexbin_bin = _missing
    _hexbin_hexagon = _missing
    _hexbin_centers = _missing
    _sankey_layout = _missing
    _legend_layout = _missing
    _axis_layout = _missing
    _grid_layout = _missing
    _NativeDsvCancellationToken = None
    _NativeQuadTreeIndex = None
    _NativeTransitionState = None
    _NativeTimerResource = None
    _NativeDragState = None
    _NativeBrushState = None
    _NativeZoomState = None
    _brush_to_domain = _missing
    _timer_now = _missing
    _timer_set_now = _missing
    _timer_flush = _missing
    _shape_symbol = _missing
    _radial_point_to_cartesian = _missing
    _radial_point_from_cartesian = _missing
    _radial_line_path = _missing
    _radial_area_path = _missing
    _polar_grid_circle_paths = _missing
    _polar_grid_ray_paths = _missing
    bisect_left = _missing
    bisect_right = _missing
    binary_search = _missing
    clamp01 = _missing
    color_darken = _missing
    color_lighten = _missing
    color_luminance = _missing
    cross = _missing
    difference = _missing
    intersection = _missing
    is_disjoint = _missing
    is_subset = _missing
    is_superset = _missing
    quantile = _missing
    quantile_sorted = _missing
    reverse = _missing
    _shuffle_values = _missing
    shuffle_seeded = _missing
    pairs = _missing
    piecewise = _missing
    piecewise_domain = _missing
    _prefix_exponent = _missing
    sum = _missing
    min = _missing
    max = _missing
    min_index = _missing
    max_index = _missing
    least_index = _missing
    merge_sorted = _missing
    mean = _missing
    _millis_from_timestamp = _missing
    median = _missing
    variance = _missing
    deviation = _missing
    extent = _missing
    ease_back_in = _missing
    ease_back_in_out = _missing
    ease_back_in_out_with = _missing
    ease_back_in_with = _missing
    ease_back_out = _missing
    ease_back_out_with = _missing
    ease_bounce_in = _missing
    ease_bounce_in_out = _missing
    ease_bounce_out = _missing
    ease_circle_in = _missing
    ease_circle_in_out = _missing
    ease_circle_out = _missing
    ease_cubic_in = _missing
    ease_cubic_in_out = _missing
    ease_cubic_out = _missing
    ease_elastic_in = _missing
    ease_elastic_in_out = _missing
    ease_elastic_in_with = _missing
    ease_elastic_out = _missing
    ease_elastic_out_with = _missing
    ease_exp_in = _missing
    ease_exp_in_out = _missing
    ease_exp_out = _missing
    ease_linear = _missing
    ease_poly_in = _missing
    ease_poly_in_out = _missing
    ease_poly_out = _missing
    ease_quad_in = _missing
    ease_quad_in_out = _missing
    ease_quad_out = _missing
    ease_sin_in = _missing
    ease_sin_in_out = _missing
    ease_sin_out = _missing
    _histogram = _missing
    interpolate_basis = _missing
    interpolate_basis_closed = _missing
    interpolate_clamped = _missing
    interpolate_cubehelix = _missing
    interpolate_cubehelix_long = _missing
    interpolate_date = _missing
    interpolate_discrete = _missing
    interpolate_exp = _missing
    interpolate_hcl = _missing
    interpolate_hcl_long = _missing
    interpolate_hsl = _missing
    interpolate_hsl_long = _missing
    interpolate_lab = _missing
    interpolate_matrix = _missing
    interpolate_number = _missing
    interpolate_number_array = _missing
    interpolate_rgb = _missing
    interpolate_round = _missing
    interpolate_string = _missing
    interpolate_transform_css = _missing
    interpolate_quantize = _missing
    interpolate_zoom_vector = _missing
    cumsum = _missing
    ticks = _missing
    tick_step = _missing
    tick_increment = _missing
    nice = _missing
    nice_bin_edges = _missing
    nice_number = _missing
    scale_nice_number = _missing
    generate_linear_ticks = _missing
    generate_log_ticks = _missing
    sort = _missing
    sort_descending = _missing
    symmetric_difference = _missing
    ticks_interval = _missing
    log_ticks = _missing
    time_ticks = _missing
    threshold_sturges = _missing
    _timestamp_from_millis = _missing
    union = _missing
    unique = _missing
else:
    AVAILABLE = True


class FormatType(str, Enum):
    NONE = "none"
    EXPONENT = "exponent"
    FIXED = "fixed"
    GENERAL = "general"
    ROUND = "round"
    SI = "si"
    PERCENT = "percent"
    PERCENT_ROUNDED = "percent_rounded"
    BINARY = "binary"
    OCTAL = "octal"
    DECIMAL = "decimal"
    HEX_LOWER = "hex_lower"
    HEX_UPPER = "hex_upper"
    CHARACTER = "character"


class FormatAlign(str, Enum):
    LEFT = "left"
    RIGHT = "right"
    CENTER = "center"
    AFTER_SIGN = "after_sign"


class FormatSign(str, Enum):
    MINUS = "minus"
    PLUS = "plus"
    SPACE = "space"
    PARENS = "parens"


@dataclass(frozen=True)
class FormatSpecifier:
    fill: str
    align: FormatAlign
    sign: FormatSign
    symbol: str | None
    zero: bool
    width: int | None
    comma: bool
    precision: int | None
    trim: bool
    format_type: FormatType


@dataclass(frozen=True)
class Locale:
    decimal: str = "."
    thousands: str = ","
    currency_prefix: str | None = "$"
    currency_suffix: str | None = None
    grouping: tuple[int, ...] = (3,)
    numerals: tuple[str, ...] | None = None
    minus: str = "-"
    percent: str = "%"

    def __post_init__(self) -> None:
        if not self.decimal:
            raise ValueError("locale decimal must not be empty")
        if not self.grouping or any(
            not isinstance(value, int) or isinstance(value, bool) or value <= 0
            for value in self.grouping
        ):
            raise ValueError("locale grouping must contain positive integers")
        if self.numerals is not None and len(self.numerals) != 10:
            raise ValueError("locale numerals must contain exactly ten strings")

    def format(self, specifier: str, value: float) -> str:
        return format_locale_value(self, specifier, value)


DEFAULT_LOCALE = Locale()


def prefix_exponent(value: float) -> int:
    """Return the SI prefix exponent selected for a finite numeric value."""

    return int(_prefix_exponent(value))


def parse_format_specifier(specifier: str) -> FormatSpecifier:
    """Parse one d3-format specifier into an immutable typed value."""
    (
        fill,
        align,
        sign,
        symbol,
        zero,
        width,
        comma,
        precision,
        trim,
        format_type,
    ) = _parse_format_specifier(specifier)
    return FormatSpecifier(
        fill=fill,
        align=FormatAlign(align),
        sign=FormatSign(sign),
        symbol=symbol,
        zero=zero,
        width=width,
        comma=comma,
        precision=precision,
        trim=trim,
        format_type=FormatType(format_type),
    )


def format_value(specifier: str, value: float) -> str:
    """Format one numeric value with the default gpui-d3rs locale."""
    return _format_value(specifier, value)


def format(specifier: str) -> Callable[[float], str]:
    """Create a reusable default-locale formatter."""
    return lambda value: format_value(specifier, value)


def format_prefix(specifier: str, reference: float) -> Callable[[float], str]:
    """Create a formatter that retains the SI prefix selected by reference."""
    return lambda value: _format_prefix_value(specifier, reference, value)


def format_locale_value(locale: Locale, specifier: str, value: float) -> str:
    """Format one value with an immutable caller-owned locale."""
    return _format_locale_value(
        specifier,
        value,
        locale.decimal,
        locale.thousands,
        locale.currency_prefix,
        locale.currency_suffix,
        locale.grouping,
        locale.numerals,
        locale.minus,
        locale.percent,
    )


def format_locale(locale: Locale, specifier: str) -> Callable[[float], str]:
    """Create a reusable formatter for a caller-owned locale."""
    return lambda value: format_locale_value(locale, specifier, value)


def interpolate_ease(a: float, b: float, ease: str | Enum, t: float) -> float:
    """Interpolate through gpui-d3rs's typed piecewise easing strategy."""
    name = ease.value if isinstance(ease, Enum) else ease
    return _interpolate_ease(a, b, name, t)


def quantize(values: Sequence[float], t: float) -> float:
    """Select a numeric value with gpui-d3rs's generic quantizer."""
    return _quantize_values(values, t)


@dataclass(frozen=True)
class Transform2D:
    translate_x: float = 0.0
    translate_y: float = 0.0
    rotate: float = 0.0
    scale_x: float = 1.0
    scale_y: float = 1.0
    skew_x: float = 0.0

    def __post_init__(self) -> None:
        if not all(math.isfinite(value) for value in self.as_tuple()):
            raise ValueError("transform components must be finite")

    def as_tuple(self) -> tuple[float, float, float, float, float, float]:
        return (
            self.translate_x,
            self.translate_y,
            self.rotate,
            self.scale_x,
            self.scale_y,
            self.skew_x,
        )

    @classmethod
    def _from_tuple(
        cls, value: tuple[float, float, float, float, float, float]
    ) -> "Transform2D":
        return cls(*value)

    @classmethod
    def identity(cls) -> "Transform2D":
        return cls._from_tuple(_transform_identity())

    @classmethod
    def translate(cls, x: float, y: float) -> "Transform2D":
        return cls._from_tuple(_transform_translate(x, y))

    @classmethod
    def rotate_deg(cls, degrees: float) -> "Transform2D":
        return cls._from_tuple(_transform_rotate_deg(degrees))

    @classmethod
    def rotate_rad(cls, radians: float) -> "Transform2D":
        return cls._from_tuple(_transform_rotate_rad(radians))

    @classmethod
    def scale(cls, sx: float, sy: float) -> "Transform2D":
        return cls._from_tuple(_transform_scale(sx, sy))

    @classmethod
    def scale_uniform(cls, scale: float) -> "Transform2D":
        return cls._from_tuple(_transform_scale_uniform(scale))

    @classmethod
    def skew_x_deg(cls, degrees: float) -> "Transform2D":
        return cls._from_tuple(_transform_skew_x_deg(degrees))

    @classmethod
    def from_matrix(cls, matrix: Sequence[float]) -> "Transform2D":
        if len(matrix) != 6:
            raise ValueError("transform matrix must contain exactly six values")
        return cls._from_tuple(_transform_from_matrix(tuple(matrix)))

    def to_matrix(self) -> tuple[float, float, float, float, float, float]:
        return _transform_to_matrix(self.as_tuple())

    def apply(self, x: float, y: float) -> tuple[float, float]:
        return _transform_apply(self.as_tuple(), x, y)

    def interpolate(self, other: "Transform2D", t: float) -> "Transform2D":
        return self._from_tuple(_transform_interpolate(self.as_tuple(), other.as_tuple(), t))

    def to_css(self) -> str:
        return _transform_to_css(self.as_tuple())

    def to_svg(self) -> str:
        return _transform_to_svg(self.as_tuple())


def interpolate_transform(a: Transform2D, b: Transform2D, t: float) -> Transform2D:
    return a.interpolate(b, t)


def interpolate_transform_svg(
    a: Sequence[float], b: Sequence[float], t: float
) -> tuple[float, float, float, float, float, float]:
    if len(a) != 6 or len(b) != 6:
        raise ValueError("SVG transform matrices must contain exactly six values")
    return _interpolate_transform_svg(tuple(a), tuple(b), t)


@dataclass(frozen=True)
class ZoomParams:
    rho: float = math.sqrt(2.0)

    def __post_init__(self) -> None:
        if not math.isfinite(self.rho) or self.rho <= 0.0:
            raise ValueError("rho must be finite and positive")


@dataclass(frozen=True)
class ZoomView:
    cx: float
    cy: float
    size: float

    def __post_init__(self) -> None:
        if not math.isfinite(self.cx) or not math.isfinite(self.cy):
            raise ValueError("zoom center must be finite")
        if not math.isfinite(self.size) or self.size <= 0.0:
            raise ValueError("zoom size must be finite and positive")

    def as_tuple(self) -> tuple[float, float, float]:
        return self.cx, self.cy, self.size

    def interpolate(
        self, other: "ZoomView", t: float, params: ZoomParams | None = None
    ) -> "ZoomView":
        return interpolate_zoom_view(self, other, t, params=params)

    def duration(self, other: "ZoomView", params: ZoomParams | None = None) -> float:
        return zoom_duration(self, other, params=params)


def interpolate_zoom_view(
    a: ZoomView,
    b: ZoomView,
    t: float,
    *,
    params: ZoomParams | None = None,
) -> ZoomView:
    rho = None if params is None else params.rho
    return ZoomView(*_interpolate_zoom_view(a.as_tuple(), b.as_tuple(), t, rho))


def zoom_duration(
    a: ZoomView, b: ZoomView, *, params: ZoomParams | None = None
) -> float:
    rho = None if params is None else params.rho
    return _zoom_duration(a.as_tuple(), b.as_tuple(), rho)


class TimeInterval(str, Enum):
    SECOND = "second"
    MINUTE = "minute"
    HOUR = "hour"
    DAY = "day"
    WEEK = "week"
    MONDAY = "monday"
    MONTH = "month"
    YEAR = "year"

    def floor(self, timestamp: int) -> int:
        return _time_interval_floor(self.value, timestamp)

    def ceil(self, timestamp: int) -> int:
        return _time_interval_ceil(self.value, timestamp)

    def round(self, timestamp: int) -> int:
        return _time_interval_round(self.value, timestamp)

    def offset(self, timestamp: int, step: int = 1) -> int:
        return _time_interval_offset(self.value, timestamp, step)

    def count(self, start: int, end: int) -> int:
        return _time_interval_count(self.value, start, end)

    def range(self, start: int, stop: int, step: int = 1) -> list[int]:
        return _time_interval_range(self.value, start, stop, step)

    @property
    def duration(self) -> int:
        return _time_interval_duration(self.value)

    @property
    def format_pattern(self) -> str:
        return _time_interval_format_pattern(self.value)

    @classmethod
    def for_span(cls, span_seconds: int) -> "TimeInterval":
        return cls(_time_interval_for_span(span_seconds))


SECOND = 1
MINUTE = 60
HOUR = 3_600
DAY = 86_400
WEEK = 604_800


def time_second() -> TimeInterval:
    return TimeInterval.SECOND


def time_minute() -> TimeInterval:
    return TimeInterval.MINUTE


def time_hour() -> TimeInterval:
    return TimeInterval.HOUR


def time_day() -> TimeInterval:
    return TimeInterval.DAY


def time_week() -> TimeInterval:
    return TimeInterval.WEEK


def time_monday() -> TimeInterval:
    return TimeInterval.MONDAY


def time_month() -> TimeInterval:
    return TimeInterval.MONTH


def time_year() -> TimeInterval:
    return TimeInterval.YEAR


@dataclass(frozen=True)
class TimeFormatParts:
    year: int
    month: int
    day: int
    hour: int
    minute: int
    second: int
    weekday: int
    day_of_year: int

    @classmethod
    def from_unix_seconds(cls, timestamp: int) -> "TimeFormatParts":
        return cls(*_time_format_parts(timestamp))


@dataclass(frozen=True)
class TimeFormat:
    pattern: str

    def format(self, timestamp: int) -> str:
        return _time_format_value(self.pattern, timestamp)


def time_format(pattern: str) -> Callable[[int], str]:
    formatter = TimeFormat(pattern)
    return formatter.format


def time_format_value(pattern: str, timestamp: int) -> str:
    """Format one Unix timestamp with a d3-time-format pattern."""

    return _time_format_value(pattern, timestamp)


def timestamp_from_millis(millis: int) -> int:
    """Convert integer milliseconds to whole Unix seconds."""

    return int(_timestamp_from_millis(millis))


def millis_from_timestamp(timestamp: int) -> int:
    """Convert whole Unix seconds to integer milliseconds."""

    return int(_millis_from_timestamp(timestamp))


@dataclass(frozen=True, init=False)
class TimeScale:
    _domain: tuple[int, int] = (0, 1)
    _range: tuple[float, float] = (0.0, 1.0)
    _clamp: bool = False

    def __init__(self) -> None:
        object.__setattr__(self, "_domain", (0, 1))
        object.__setattr__(self, "_range", (0.0, 1.0))
        object.__setattr__(self, "_clamp", False)

    def _updated(self, **changes: object) -> "TimeScale":
        updated = object.__new__(type(self))
        object.__setattr__(updated, "_domain", changes.get("_domain", self._domain))
        object.__setattr__(updated, "_range", changes.get("_range", self._range))
        object.__setattr__(updated, "_clamp", changes.get("_clamp", self._clamp))
        return updated

    def domain(self, minimum: int, maximum: int) -> "TimeScale":
        if minimum == maximum:
            raise ValueError("time scale domain endpoints must differ")
        return self._updated(_domain=(minimum, maximum))

    def range(self, minimum: float, maximum: float) -> "TimeScale":
        if not math.isfinite(minimum) or not math.isfinite(maximum):
            raise ValueError("time scale range endpoints must be finite")
        if minimum == maximum:
            raise ValueError("time scale range endpoints must differ")
        return self._updated(_range=(minimum, maximum))

    def clamp(self, enabled: bool = True) -> "TimeScale":
        if not isinstance(enabled, bool):
            raise TypeError("time scale clamp must be a bool")
        return self._updated(_clamp=enabled)

    def nice(self, count: int | None = None) -> "TimeScale":
        domain = _time_scale_nice(self._domain, count)
        return self._updated(_domain=domain)

    def scale(self, value: int) -> float:
        return _time_scale_value(
            value, domain=self._domain, range=self._range, clamp=self._clamp
        )

    def invert(self, value: float) -> int | None:
        return _time_scale_invert(
            value, domain=self._domain, range=self._range, clamp=self._clamp
        )

    def ticks(self, count: int = 10) -> list[int]:
        return _time_scale_ticks(self._domain, count)

    def time_ticks(self, count: int = 10) -> list[int]:
        """Return calendar-aware ticks using the native method name."""

        return self.ticks(count)

    def interval(self) -> TimeInterval:
        return TimeInterval(_time_scale_interval(self._domain))

    def copy(self) -> "TimeScale":
        return self

    def domain_min(self) -> int:
        return self._domain[0]

    def domain_max(self) -> int:
        return self._domain[1]

    @property
    def domain_values(self) -> tuple[int, int]:
        return self._domain

    @property
    def range_values(self) -> tuple[float, float]:
        return self._range

    @property
    def clamped(self) -> bool:
        return self._clamp


class LodErrorKind(str, Enum):
    INVALID_BOUNDS = "invalid_bounds"
    INVALID_BASE_DIMENSION = "invalid_base_dimension"
    UNEQUAL_COORDINATES = "unequal_coordinates"


class LodError(ValueError):
    """Path-qualified error raised by checked level-of-detail operations."""

    def __init__(
        self, message: str, *, path: str = "lod", kind: LodErrorKind | None = None
    ) -> None:
        super().__init__(f"{path}: {message}")
        self.path = path
        self.kind = kind


@dataclass(frozen=True)
class LodBounds:
    x0: float
    x1: float
    y0: float
    y1: float

    def __post_init__(self) -> None:
        values = tuple(float(value) for value in (self.x0, self.x1, self.y0, self.y1))
        if not all(math.isfinite(value) for value in values):
            raise LodError(
                "bounds must be finite and non-empty",
                path="bounds",
                kind=LodErrorKind.INVALID_BOUNDS,
            )
        if values[1] <= values[0] or values[3] <= values[2]:
            raise LodError(
                "bounds must be finite and non-empty",
                path="bounds",
                kind=LodErrorKind.INVALID_BOUNDS,
            )
        for name, value in zip(("x0", "x1", "y0", "y1"), values):
            object.__setattr__(self, name, value)

    @classmethod
    def new(cls, x0: float, x1: float, y0: float, y1: float) -> "LodBounds":
        return cls(x0, x1, y0, y1)

    def as_tuple(self) -> tuple[float, float, float, float]:
        return self.x0, self.x1, self.y0, self.y1


@dataclass(frozen=True)
class LodDensityGrid:
    width: int
    height: int
    values: tuple[float, ...]
    level: int

    def __post_init__(self) -> None:
        if self.width <= 0 or self.height <= 0:
            raise LodError("grid dimensions must be positive", path="grid")
        values = tuple(float(value) for value in self.values)
        if len(values) != self.width * self.height:
            raise LodError("values length must match dimensions", path="grid.values")
        if self.level < 0:
            raise LodError("level must be non-negative", path="grid.level")
        object.__setattr__(self, "values", values)


@dataclass(frozen=True)
class DensityPyramid:
    _native: object

    @classmethod
    def build(
        cls,
        x: Sequence[float],
        y: Sequence[float],
        bounds: LodBounds,
        base_dimension: int,
    ) -> "DensityPyramid":
        if not isinstance(bounds, LodBounds):
            raise TypeError("bounds must be LodBounds")
        if len(x) != len(y):
            raise LodError(
                "x and y coordinate columns must have equal length",
                path="density_pyramid.coordinates",
                kind=LodErrorKind.UNEQUAL_COORDINATES,
            )
        if base_dimension < 2 or base_dimension & (base_dimension - 1):
            raise LodError(
                "base dimension must be a power of two >= 2",
                path="density_pyramid.base_dimension",
                kind=LodErrorKind.INVALID_BASE_DIMENSION,
            )
        if _NativeDensityPyramid is None:
            raise RuntimeError("gpui_toolkit native extension not installed")
        try:
            native = _NativeDensityPyramid(
                list(x), list(y), bounds.as_tuple(), base_dimension
            )
        except ValueError as error:
            raise LodError(str(error), path="density_pyramid") from error
        return cls(native)

    def bounds(self) -> LodBounds:
        return LodBounds(*self._native.bounds())  # type: ignore[attr-defined]

    def level_count(self) -> int:
        return int(self._native.level_count())  # type: ignore[attr-defined]

    def compose(
        self,
        view: LodBounds,
        width: int,
        height: int,
        max_upsample: int,
    ) -> LodDensityGrid | None:
        if not isinstance(view, LodBounds):
            raise TypeError("view must be LodBounds")
        result = self._native.compose(  # type: ignore[attr-defined]
            view.as_tuple(), width, height, max_upsample
        )
        if result is None:
            return None
        grid_width, grid_height, values, level = result
        return LodDensityGrid(grid_width, grid_height, tuple(values), level)


def m4_indices(
    x: Sequence[float],
    y: Sequence[float],
    x0: float,
    x1: float,
    columns: int,
) -> list[int]:
    return list(_lod_m4_indices(list(x), list(y), x0, x1, columns))


def m4_point_indices(points: Sequence[tuple[float, float]], columns: int) -> list[int]:
    return list(_lod_m4_point_indices(list(points), columns))


class HierarchyErrorKind(str, Enum):
    NON_FINITE_VALUE = "non_finite_value"
    NEGATIVE_VALUE = "negative_value"
    NON_FINITE_LAYOUT_SIZE = "non_finite_layout_size"
    NEGATIVE_LAYOUT_SIZE = "negative_layout_size"
    NON_FINITE_LAYOUT_PADDING = "non_finite_layout_padding"
    NEGATIVE_LAYOUT_PADDING = "negative_layout_padding"
    NON_FINITE_LAYOUT_SEPARATION = "non_finite_layout_separation"
    NEGATIVE_LAYOUT_SEPARATION = "negative_layout_separation"


class HierarchyError(ValueError):
    """Typed, path-qualified failure from a checked hierarchy operation."""

    def __init__(
        self,
        kind: HierarchyErrorKind,
        message: str,
        *,
        path: str = "hierarchy",
        node_index: int | None = None,
        coordinate: str | None = None,
        value: float | None = None,
    ) -> None:
        super().__init__(f"{path}: {message}")
        self.kind = kind
        self.path = path
        self.node_index = node_index
        self.coordinate = coordinate
        self.value = value


def _validate_hierarchy_value(value: float, node_index: int, *, checked: bool) -> None:
    if not math.isfinite(value):
        raise HierarchyError(
            HierarchyErrorKind.NON_FINITE_VALUE,
            f"value at traversal index {node_index} must be finite",
            path=f"hierarchy.nodes[{node_index}].value",
            node_index=node_index,
            value=value,
        )
    if checked and value < 0.0:
        raise HierarchyError(
            HierarchyErrorKind.NEGATIVE_VALUE,
            f"value at traversal index {node_index} must be non-negative",
            path=f"hierarchy.nodes[{node_index}].value",
            node_index=node_index,
            value=value,
        )


def _validate_layout_number(value: float, coordinate: str, *, padding: bool = False) -> None:
    path = f"hierarchy.layout.{coordinate}"
    if not math.isfinite(value):
        kind = (
            HierarchyErrorKind.NON_FINITE_LAYOUT_PADDING
            if padding
            else HierarchyErrorKind.NON_FINITE_LAYOUT_SIZE
        )
        raise HierarchyError(
            kind,
            f"{coordinate} must be finite",
            path=path,
            coordinate=None if padding else coordinate,
            value=value,
        )
    if value < 0.0:
        kind = (
            HierarchyErrorKind.NEGATIVE_LAYOUT_PADDING
            if padding
            else HierarchyErrorKind.NEGATIVE_LAYOUT_SIZE
        )
        raise HierarchyError(
            kind,
            f"{coordinate} must be non-negative",
            path=path,
            coordinate=None if padding else coordinate,
            value=value,
        )


@dataclass(frozen=True)
class HierarchyNode:
    """Immutable Python value corresponding to a gpui-d3rs hierarchy node."""

    data: object
    value: float = 0.0
    children: tuple["HierarchyNode", ...] = ()

    def __post_init__(self) -> None:
        value = float(self.value)
        _validate_hierarchy_value(value, 0, checked=False)
        children = tuple(self.children)
        if any(not isinstance(child, HierarchyNode) for child in children):
            raise TypeError("hierarchy children must be HierarchyNode values")
        object.__setattr__(self, "value", value)
        object.__setattr__(self, "children", children)

    @classmethod
    def new(cls, data: object) -> "HierarchyNode":
        return cls(data)

    def set_children(self, children: Sequence["HierarchyNode"]) -> "HierarchyNode":
        return replace(self, children=tuple(children))

    def sum(
        self, value: Callable[[object], float] | None = None
    ) -> "Hierarchy":
        nodes, parents, _ = _flatten_hierarchy(self)
        values = [
            node.value if value is None else float(value(node.data)) for node in nodes
        ]
        for index, item in enumerate(values):
            _validate_hierarchy_value(item, index, checked=False)
        return Hierarchy(
            self,
            "sum",
            tuple(_hierarchy_sum(parents, values)),
            tuple(values),
        )

    def try_sum(self, value_fn: Callable[[object], float]) -> "Hierarchy":
        nodes, parents, _ = _flatten_hierarchy(self)
        values = [float(value_fn(node.data)) for node in nodes]
        for index, item in enumerate(values):
            _validate_hierarchy_value(item, index, checked=True)
        return Hierarchy(
            self,
            "sum",
            tuple(_hierarchy_sum(parents, values)),
            tuple(values),
        )

    def count(self) -> "Hierarchy":
        nodes, parents, _ = _flatten_hierarchy(self)
        return Hierarchy(
            self,
            "count",
            tuple(_hierarchy_count(parents)),
            tuple(0.0 for _ in nodes),
        )

    def sort(
        self,
        compare_fn: Callable[..., object],
        reverse: bool = False,
    ) -> "HierarchyNode":
        descendants = tuple(
            child.sort(compare_fn, reverse=reverse) for child in self.children
        )
        try:
            signature(compare_fn).bind(object(), object())
        except (TypeError, ValueError):
            children = tuple(sorted(descendants, key=compare_fn, reverse=reverse))
        else:
            def compare(left: "HierarchyNode", right: "HierarchyNode") -> int:
                result = compare_fn(left, right)
                if isinstance(result, bool) or not isinstance(result, (int, float)):
                    raise TypeError("hierarchy comparator must return a negative, zero, or positive number")
                return -1 if result < 0 else 1 if result > 0 else 0

            children = tuple(
                sorted(descendants, key=cmp_to_key(compare), reverse=reverse)
            )
        return replace(self, children=children)

    def sort_by(
        self,
        key: Callable[["HierarchyNode"], object],
        reverse: bool = False,
    ) -> "HierarchyNode":
        return replace(
            self,
            children=tuple(
                sorted(
                    (child.sort_by(key, reverse=reverse) for child in self.children),
                    key=key,
                    reverse=reverse,
                )
            ),
        )

    def each(self, callback: Callable[["HierarchyNode"], None]) -> "HierarchyNode":
        for node in _flatten_hierarchy(self)[0]:
            callback(node)
        return self


@dataclass(frozen=True)
class HierarchyNodeSnapshot:
    """Immutable traversal metadata corresponding to Rust's mutable node fields."""

    index: int
    node: HierarchyNode
    parent: int | None
    children: tuple[int, ...]
    value: float
    depth: int
    height: int
    x: float = 0.0
    y: float = 0.0

    @property
    def data(self) -> object:
        return self.node.data


@dataclass(frozen=True)
class Hierarchy:
    root: HierarchyNode
    aggregate: str = "sum"
    values: tuple[float, ...] = ()
    _source_values: tuple[float, ...] = ()

    def __post_init__(self) -> None:
        if self.aggregate not in {"sum", "count"}:
            raise ValueError("hierarchy aggregate must be 'sum' or 'count'")
        nodes, _, _ = _flatten_hierarchy(self.root)
        if self.values and len(self.values) != len(nodes):
            raise ValueError("hierarchy aggregate values must match its node count")
        if self._source_values and len(self._source_values) != len(nodes):
            raise ValueError("hierarchy source values must match its node count")

    def snapshot(
        self, layout: Sequence["HierarchyPoint"] | None = None
    ) -> tuple[HierarchyNodeSnapshot, ...]:
        nodes, parents, source_values = _flatten_hierarchy(self.root)
        values = self.values or tuple(source_values)
        child_indices: list[list[int]] = [[] for _ in nodes]
        depths = [0 for _ in nodes]
        for index, parent in enumerate(parents):
            if parent is not None:
                child_indices[parent].append(index)
                depths[index] = depths[parent] + 1
        heights = [0 for _ in nodes]
        for index in range(len(nodes) - 1, -1, -1):
            children = child_indices[index]
            heights[index] = (
                0
                if not children
                else 1 + builtins.max(heights[child] for child in children)
            )
        positions = {point.index: (float(point.x), float(point.y)) for point in layout or ()}
        if any(index < 0 or index >= len(nodes) for index in positions):
            raise ValueError("hierarchy layout contains an unknown traversal index")
        return tuple(
            HierarchyNodeSnapshot(
                index=index,
                node=node,
                parent=parents[index],
                children=tuple(child_indices[index]),
                value=float(values[index]),
                depth=depths[index],
                height=heights[index],
                x=positions.get(index, (0.0, 0.0))[0],
                y=positions.get(index, (0.0, 0.0))[1],
            )
            for index, node in enumerate(nodes)
        )


def _flatten_hierarchy(
    root: HierarchyNode,
) -> tuple[list[HierarchyNode], list[int | None], list[float]]:
    if not isinstance(root, HierarchyNode):
        raise TypeError("hierarchy root must be a HierarchyNode")
    nodes: list[HierarchyNode] = []
    parents: list[int | None] = []
    values: list[float] = []
    seen: set[int] = set()
    active: set[int] = set()

    def visit(node: HierarchyNode, parent: int | None) -> None:
        identity = id(node)
        if identity in active:
            raise ValueError("hierarchy contains a cycle")
        if identity in seen:
            raise ValueError("hierarchy children may not be shared between parents")
        seen.add(identity)
        active.add(identity)
        index = len(nodes)
        nodes.append(node)
        parents.append(parent)
        values.append(node.value)
        for child in node.children:
            visit(child, index)
        active.remove(identity)

    visit(root, None)
    return nodes, parents, values


def _layout_input(
    root: HierarchyNode | Hierarchy,
) -> tuple[list[HierarchyNode], list[int | None], list[float], bool]:
    hierarchy = root if isinstance(root, Hierarchy) else root.sum()
    nodes, parents, values = _flatten_hierarchy(hierarchy.root)
    if hierarchy._source_values:
        values = list(hierarchy._source_values)
    return nodes, parents, values, hierarchy.aggregate == "count"


@dataclass(frozen=True)
class HierarchyRect:
    index: int
    node: HierarchyNode
    x0: float
    y0: float
    x1: float
    y1: float
    depth: int
    value: float


@dataclass(frozen=True)
class HierarchyCircle:
    index: int
    node: HierarchyNode
    x: float
    y: float
    radius: float
    depth: int
    value: float

    @property
    def r(self) -> float:
        return self.radius


@dataclass(frozen=True)
class HierarchyPoint:
    index: int
    node: HierarchyNode
    x: float
    y: float
    depth: int
    value: float


@dataclass(frozen=True)
class _RectLayout:
    _width: float = 1.0
    _height: float = 1.0
    _padding: float = 0.0
    _kind: str = "treemap"

    def size(self, width: float, height: float) -> "_RectLayout":
        return replace(self, _width=float(width), _height=float(height))

    def padding(self, value: float) -> "_RectLayout":
        return replace(self, _padding=float(value))

    def layout(self, root: HierarchyNode | Hierarchy) -> list[HierarchyRect]:
        _validate_layout_number(self._width, "width")
        _validate_layout_number(self._height, "height")
        _validate_layout_number(self._padding, "padding", padding=True)
        nodes, parents, values, count = _layout_input(root)
        result = _hierarchy_rect_layout(
            self._kind,
            parents,
            values,
            width=self._width,
            height=self._height,
            padding=self._padding,
            count=count,
        )
        return [HierarchyRect(index, nodes[index], x0, y0, x1, y1, depth, value)
                for index, x0, y0, x1, y1, depth, value in result]

    try_layout = layout


@dataclass(frozen=True)
class TreemapLayout(_RectLayout):
    _kind: str = "treemap"


@dataclass(frozen=True)
class PartitionLayout(_RectLayout):
    _kind: str = "partition"


@dataclass(frozen=True)
class PackLayout:
    _width: float = 1.0
    _height: float = 1.0
    _padding: float = 0.0

    def size(self, width: float, height: float) -> "PackLayout":
        return replace(self, _width=float(width), _height=float(height))

    def padding(self, value: float) -> "PackLayout":
        return replace(self, _padding=float(value))

    def layout(self, root: HierarchyNode | Hierarchy) -> list[HierarchyCircle]:
        _validate_layout_number(self._width, "width")
        _validate_layout_number(self._height, "height")
        _validate_layout_number(self._padding, "padding", padding=True)
        nodes, parents, values, count = _layout_input(root)
        result = _hierarchy_pack_layout(
            parents,
            values,
            width=self._width,
            height=self._height,
            padding=self._padding,
            count=count,
        )
        return [HierarchyCircle(index, nodes[index], x, y, radius, depth, value)
                for index, x, y, radius, depth, value in result]

    try_layout = layout


@dataclass(frozen=True)
class _PointLayout:
    _width: float = 1.0
    _height: float = 1.0
    _node_size: tuple[float, float] | None = None
    _separation: Callable[[HierarchyNode, HierarchyNode], float] | None = None
    _kind: str = "tree"

    def size(self, width: float, height: float) -> "_PointLayout":
        return replace(
            self, _width=float(width), _height=float(height), _node_size=None
        )

    def node_size(self, width: float, height: float) -> "_PointLayout":
        return replace(self, _node_size=(float(width), float(height)))

    def separation(
        self, callback: Callable[[HierarchyNode, HierarchyNode], float]
    ) -> "_PointLayout":
        if not callable(callback):
            raise TypeError("separation must be callable")
        return replace(self, _separation=callback)

    def layout(self, root: HierarchyNode | Hierarchy) -> list[HierarchyPoint]:
        if self._node_size is None:
            _validate_layout_number(self._width, "width")
            _validate_layout_number(self._height, "height")
        else:
            _validate_layout_number(self._node_size[0], "node_width")
            _validate_layout_number(self._node_size[1], "node_height")
        nodes, parents, values, count = _layout_input(root)
        native_separation = None
        if self._separation is not None:
            def native_separation(left: int, right: int) -> float:
                value = float(self._separation(nodes[left], nodes[right]))
                if not math.isfinite(value):
                    raise HierarchyError(
                        HierarchyErrorKind.NON_FINITE_LAYOUT_SEPARATION,
                        "separation callback must return a finite value",
                        path="hierarchy.layout.separation",
                        value=value,
                    )
                if value < 0.0:
                    raise HierarchyError(
                        HierarchyErrorKind.NEGATIVE_LAYOUT_SEPARATION,
                        "separation callback must return a non-negative value",
                        path="hierarchy.layout.separation",
                        value=value,
                    )
                return value

        result = _hierarchy_point_layout(
            self._kind,
            parents,
            values,
            width=self._width,
            height=self._height,
            node_size=self._node_size,
            count=count,
            separation=native_separation,
        )
        return [HierarchyPoint(index, nodes[index], x, y, depth, value)
                for index, x, y, depth, value in result]

    try_layout = layout


@dataclass(frozen=True)
class TreeLayout(_PointLayout):
    _kind: str = "tree"


@dataclass(frozen=True)
class ClusterLayout(_PointLayout):
    _kind: str = "cluster"


class DensityKernel(str, Enum):
    GAUSSIAN = "gaussian"
    EPANECHNIKOV = "epanechnikov"


class DensityError(ValueError):
    """Typed checked-density failure preserving the Rust error message."""

    def __init__(self, message: str, *, path: str = "density") -> None:
        super().__init__(f"{path}: {message}")
        self.message = message
        self.path = path


def _density_checked(call: Callable[[], object], *, path: str) -> object:
    try:
        return call()
    except ValueError as error:
        raise DensityError(str(error), path=path) from error


@dataclass(frozen=True)
class DensityGrid:
    values: tuple[float, ...]
    width: int
    height: int

    def __post_init__(self) -> None:
        if self.width < 0 or self.height < 0:
            raise ValueError("density grid dimensions must be non-negative")
        if len(self.values) != self.width * self.height:
            raise ValueError("density values length must match grid dimensions")

    def at(self, x: int, y: int) -> float:
        if not (0 <= x < self.width and 0 <= y < self.height):
            raise IndexError("density grid coordinate is out of bounds")
        return self.values[y * self.width + x]

    def __len__(self) -> int:
        return len(self.values)


def gaussian_kernel(x: float, bandwidth: float) -> float:
    return _density_kernel(DensityKernel.GAUSSIAN.value, x, bandwidth)


def epanechnikov_kernel(x: float, bandwidth: float) -> float:
    return _density_kernel(DensityKernel.EPANECHNIKOV.value, x, bandwidth)


@dataclass(frozen=True)
class DensityEstimator:
    _width: int = 100
    _height: int = 100
    _x_domain: tuple[float, float] = (0.0, 1.0)
    _y_domain: tuple[float, float] = (0.0, 1.0)
    _bandwidth: float = 0.1
    _kernel: DensityKernel = DensityKernel.GAUSSIAN

    @classmethod
    def new(cls) -> "DensityEstimator":
        return cls()

    def size(self, width: int, height: int) -> "DensityEstimator":
        return replace(self, _width=width, _height=height)

    def x(self, minimum: float, maximum: float) -> "DensityEstimator":
        return replace(self, _x_domain=(float(minimum), float(maximum)))

    def y(self, minimum: float, maximum: float) -> "DensityEstimator":
        return replace(self, _y_domain=(float(minimum), float(maximum)))

    def bandwidth(self, value: float) -> "DensityEstimator":
        return replace(self, _bandwidth=float(value))

    def kernel(self, value: DensityKernel | str) -> "DensityEstimator":
        return replace(self, _kernel=DensityKernel(value))

    def validate(self) -> None:
        # A zero-point estimate still exercises the native configuration path.
        _density_checked(
            lambda: _density_estimate(
                (),
                width=self._width,
                height=self._height,
                x_domain=self._x_domain,
                y_domain=self._y_domain,
                bandwidth=self._bandwidth,
                kernel=self._kernel.value,
            ),
            path="density.estimator",
        )

    def try_estimate(self, points: Sequence[tuple[float, float]]) -> DensityGrid:
        values = _density_checked(
            lambda: _density_estimate(
                points,
                width=self._width,
                height=self._height,
                x_domain=self._x_domain,
                y_domain=self._y_domain,
                bandwidth=self._bandwidth,
                kernel=self._kernel.value,
            ),
            path="density.estimator.points",
        )
        assert isinstance(values, list)
        return DensityGrid(tuple(values), self._width, self._height)

    def estimate(self, points: Sequence[tuple[float, float]]) -> DensityGrid:
        try:
            return self.try_estimate(points)
        except DensityError:
            return DensityGrid(
                tuple(0.0 for _ in builtins.range(self._width * self._height)),
                self._width,
                self._height,
            )

    def try_estimate_weighted(
        self, points: Sequence[tuple[float, float, float]]
    ) -> DensityGrid:
        values = _density_checked(
            lambda: _density_estimate_weighted(
                points,
                width=self._width,
                height=self._height,
                x_domain=self._x_domain,
                y_domain=self._y_domain,
                bandwidth=self._bandwidth,
                kernel=self._kernel.value,
            ),
            path="density.estimator.weighted_points",
        )
        assert isinstance(values, list)
        return DensityGrid(tuple(values), self._width, self._height)

    def estimate_weighted(
        self, points: Sequence[tuple[float, float, float]]
    ) -> DensityGrid:
        try:
            return self.try_estimate_weighted(points)
        except DensityError:
            return DensityGrid(
                tuple(0.0 for _ in builtins.range(self._width * self._height)),
                self._width,
                self._height,
            )


def density_2d(
    points: Sequence[tuple[float, float]],
    width: int,
    height: int,
    bandwidth: float,
) -> DensityGrid:
    try:
        return try_density_2d(points, width, height, bandwidth)
    except DensityError:
        return DensityGrid(
            tuple(0.0 for _ in builtins.range(width * height)),
            width,
            height,
        )


def try_density_2d(
    points: Sequence[tuple[float, float]],
    width: int,
    height: int,
    bandwidth: float,
) -> DensityGrid:
    result = _density_checked(
        lambda: _density_2d_auto(
            points, width=width, height=height, bandwidth=bandwidth
        ),
        path="density.points",
    )
    assert isinstance(result, tuple)
    values, result_width, result_height = result
    return DensityGrid(tuple(values), result_width, result_height)


@dataclass(frozen=True)
class ContourRing:
    points: tuple[tuple[float, float], ...]

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "points",
            tuple((float(x), float(y)) for x, y in self.points),
        )

    @classmethod
    def new(cls, points: Sequence[tuple[float, float]]) -> "ContourRing":
        return cls(tuple(points))

    def is_closed(self) -> bool:
        if len(self.points) < 2:
            return False
        first = self.points[0]
        last = self.points[-1]
        return abs(first[0] - last[0]) < 1e-10 and abs(first[1] - last[1]) < 1e-10

    def area(self) -> float:
        if len(self.points) < 3:
            return 0.0
        return builtins.sum(
            (self.points[index + 1][0] - self.points[index][0])
            * (self.points[index + 1][1] + self.points[index][1])
            for index in builtins.range(len(self.points) - 1)
        ) / 2.0

    def try_area(self) -> float:
        for index, point in enumerate(self.points):
            for coordinate, value in zip(("x", "y"), point):
                if not math.isfinite(value):
                    raise ContourRingError(index, coordinate, value)
        return self.area()

    @property
    def closed(self) -> bool:
        return self.is_closed()

    @property
    def signed_area(self) -> float:
        return self.area()


class ContourRingError(ValueError):
    """Typed checked-ring failure retaining point location and value."""

    def __init__(self, index: int, coordinate: str, value: float) -> None:
        message = (
            f"contour ring point {coordinate} at index {index} is not finite: {value}"
        )
        super().__init__(f"contour.ring.points[{index}].{coordinate}: {message}")
        self.kind = "non_finite_point"
        self.index = index
        self.coordinate = coordinate
        self.value = value
        self.path = f"contour.ring.points[{index}].{coordinate}"


@dataclass(frozen=True)
class Contour:
    value: float
    coordinates: tuple[ContourRing, ...] = ()

    @classmethod
    def new(cls, value: float) -> "Contour":
        return cls(value)

    def add_ring(self, ring: ContourRing) -> "Contour":
        if not isinstance(ring, ContourRing):
            raise TypeError("contour ring must be a ContourRing")
        return replace(self, coordinates=(*self.coordinates, ring))


@dataclass(frozen=True)
class ContourBand:
    lower: float
    upper: float
    polygons: tuple[ContourRing, ...] = ()

    @classmethod
    def new(cls, lower: float, upper: float) -> "ContourBand":
        return cls(lower, upper)

    def mid_value(self) -> float:
        return (self.lower + self.upper) / 2.0


@dataclass(frozen=True)
class ContourSegment:
    value: float
    start: tuple[float, float]
    end: tuple[float, float]


def _contour_result(item: tuple[float, Sequence[Sequence[tuple[float, float]]]]) -> Contour:
    value, rings = item
    return Contour(value, tuple(ContourRing(tuple(ring)) for ring in rings))


@dataclass(frozen=True)
class ContourGenerator:
    width: int
    height: int
    _x_domain: tuple[float, float] | None = None
    _y_domain: tuple[float, float] | None = None
    _x_values: tuple[float, ...] | None = None
    _y_values: tuple[float, ...] | None = None
    _upsample_factor: int = 1
    _x_log_interpolation: bool = False
    _y_log_interpolation: bool = False

    @classmethod
    def new(cls, width: int, height: int) -> "ContourGenerator":
        return cls(width, height)

    def x(self, minimum: float, maximum: float) -> "ContourGenerator":
        return replace(
            self,
            _x_domain=(float(minimum), float(maximum)),
            _x_values=None,
        )

    def y(self, minimum: float, maximum: float) -> "ContourGenerator":
        return replace(
            self,
            _y_domain=(float(minimum), float(maximum)),
            _y_values=None,
        )

    def x_values(self, values: Sequence[float]) -> "ContourGenerator":
        return replace(self, _x_values=tuple(float(value) for value in values))

    def y_values(self, values: Sequence[float]) -> "ContourGenerator":
        return replace(self, _y_values=tuple(float(value) for value in values))

    def upsample_factor(self, factor: int) -> "ContourGenerator":
        return replace(self, _upsample_factor=factor)

    def x_log_interpolation(self, enabled: bool = True) -> "ContourGenerator":
        if not isinstance(enabled, bool):
            raise TypeError("x log interpolation flag must be bool")
        return replace(self, _x_log_interpolation=enabled)

    def y_log_interpolation(self, enabled: bool = True) -> "ContourGenerator":
        if not isinstance(enabled, bool):
            raise TypeError("y log interpolation flag must be bool")
        return replace(self, _y_log_interpolation=enabled)

    def _options(self) -> dict[str, object]:
        return {
            "x_domain": self._x_domain,
            "y_domain": self._y_domain,
            "x_values": self._x_values,
            "y_values": self._y_values,
            "upsample_factor": self._upsample_factor,
            "x_log_interpolation": self._x_log_interpolation,
            "y_log_interpolation": self._y_log_interpolation,
        }

    def contours(
        self, values: Sequence[float], thresholds: Sequence[float]
    ) -> list[Contour]:
        result = _contour_generate(
            values, self.width, self.height, thresholds, **self._options()
        )
        return [_contour_result(item) for item in result]

    def contour(self, values: Sequence[float], threshold: float) -> Contour:
        return self.contours(values, (threshold,))[0]

    def contour_into(
        self, values: Sequence[float], threshold: float, out: Contour
    ) -> Contour:
        if not isinstance(out, Contour):
            raise TypeError("out must be a Contour")
        # Rust mutates the caller-owned buffer; immutable Python values return its replacement.
        return self.contour(values, threshold)

    def contour_bands(
        self, values: Sequence[float], thresholds: Sequence[float]
    ) -> list[ContourBand]:
        result = _contour_band_generate(
            values, self.width, self.height, thresholds, **self._options()
        )
        return [
            ContourBand(
                lower,
                upper,
                tuple(ContourRing(tuple(ring)) for ring in polygons),
            )
            for lower, upper, polygons in result
        ]

    def contour_bands_into(
        self,
        values: Sequence[float],
        thresholds: Sequence[float],
        out: list[ContourBand],
    ) -> None:
        if not isinstance(out, list):
            raise TypeError("out must be a list of ContourBand values")
        out[:] = self.contour_bands(values, thresholds)

    def contour_segments(
        self, values: Sequence[float], thresholds: Sequence[float]
    ) -> list[ContourSegment]:
        result = _contour_segment_generate(
            values, self.width, self.height, thresholds, **self._options()
        )
        return [ContourSegment(value, start, end) for value, start, end in result]


def contour(
    values: Sequence[float], width: int, height: int, threshold: float
) -> Contour:
    return ContourGenerator(width, height).contour(values, threshold)


def contours(
    values: Sequence[float],
    width: int,
    height: int,
    thresholds: Sequence[float],
) -> list[Contour]:
    return ContourGenerator(width, height).contours(values, thresholds)


def contour_threshold_sturges(
    minimum: float, maximum: float, count: int
) -> list[float]:
    return _contour_threshold_sturges(minimum, maximum, count)


def contour_threshold_scott(
    values: Sequence[float], minimum: float, maximum: float
) -> list[float]:
    return _contour_threshold_scott(values, minimum, maximum)


def contour_threshold_freedman_diaconis(
    values: Sequence[float], minimum: float, maximum: float
) -> list[float]:
    return _contour_threshold_freedman_diaconis(values, minimum, maximum)


def polygon_area(polygon: Sequence[tuple[float, float]]) -> float:
    return _polygon_area(polygon)


def polygon_area_signed(polygon: Sequence[tuple[float, float]]) -> float:
    return _polygon_area_signed(polygon)


def polygon_centroid(
    polygon: Sequence[tuple[float, float]],
) -> tuple[float, float]:
    return _polygon_centroid(polygon)


def polygon_contains(
    polygon: Sequence[tuple[float, float]], point: tuple[float, float]
) -> bool:
    return _polygon_contains(polygon, point)


def polygon_length(polygon: Sequence[tuple[float, float]]) -> float:
    return _polygon_length(polygon)


def polygon_hull(
    points: Sequence[tuple[float, float]],
) -> list[tuple[float, float]]:
    return _polygon_hull(points)


@dataclass(frozen=True)
class Voronoi:
    _bounds: tuple[float, float, float, float]
    _bounds_polygon: tuple[tuple[float, float], ...]
    _cells: tuple[tuple[tuple[float, float], ...] | None, ...]
    _path: str
    _bounds_path: str
    _cell_paths: tuple[str | None, ...]
    _neighbors: tuple[tuple[int, ...], ...]

    @classmethod
    def _from_snapshot(cls, snapshot: tuple[object, ...]) -> "Voronoi":
        bounds, bounds_polygon, cells, path, bounds_path, cell_paths, neighbors = snapshot
        return cls(
            tuple(bounds),
            tuple(tuple(point) for point in bounds_polygon),
            tuple(None if cell is None else tuple(tuple(point) for point in cell) for cell in cells),
            str(path),
            str(bounds_path),
            tuple(cell_paths),
            tuple(tuple(indices) for indices in neighbors),
        )

    def bounds(self) -> tuple[float, float, float, float]:
        return self._bounds

    def bounds_polygon(self) -> tuple[tuple[float, float], ...]:
        return self._bounds_polygon

    def cell_count(self) -> int:
        return len(self._cells)

    def cell_polygon(self, index: int) -> tuple[tuple[float, float], ...] | None:
        if not 0 <= index < len(self._cells):
            return None
        return self._cells[index]

    def cell_polygons(self) -> tuple[tuple[tuple[float, float], ...], ...]:
        return tuple(cell for cell in self._cells if cell is not None)

    def indexed_cell_polygons(
        self,
    ) -> tuple[tuple[int, tuple[tuple[float, float], ...]], ...]:
        return tuple(
            (index, cell)
            for index, cell in enumerate(self._cells)
            if cell is not None
        )

    def render_to_path(self) -> str:
        return self._path

    render_to_path_into = render_to_path

    def render_bounds_to_path(self) -> str:
        return self._bounds_path

    render_bounds_to_path_into = render_bounds_to_path

    def render_cell_to_path(self, index: int) -> str | None:
        if not 0 <= index < len(self._cell_paths):
            return None
        return self._cell_paths[index]

    render_cell_to_path_into = render_cell_to_path

    def contains(self, index: int, x: float, y: float) -> bool:
        cell = self.cell_polygon(index)
        return cell is not None and polygon_contains(cell, (x, y))

    def neighbors(self, index: int) -> tuple[int, ...]:
        if not 0 <= index < len(self._neighbors):
            return ()
        return self._neighbors[index]


@dataclass(frozen=True, init=False)
class Delaunay:
    _points: tuple[tuple[float, float], ...]
    _triangles: tuple[tuple[int, int, int], ...] = field(init=False)
    _hull: tuple[int, ...] = field(init=False)
    _hull_polygon: tuple[tuple[float, float], ...] = field(init=False)
    _edges: tuple[tuple[int, int], ...] = field(init=False)
    _path: str = field(init=False)
    _hull_path: str = field(init=False)
    _voronoi: Voronoi = field(init=False)

    def __init__(self, points: Iterable[tuple[float, float]]) -> None:
        normalized = tuple((float(x), float(y)) for x, y in points)
        snapshot = _delaunay_snapshot(normalized, None)
        triangles, hull, hull_polygon, edges, path, hull_path, voronoi = snapshot
        object.__setattr__(self, "_points", normalized)
        object.__setattr__(self, "_triangles", tuple(tuple(item) for item in triangles))
        object.__setattr__(self, "_hull", tuple(hull))
        object.__setattr__(
            self, "_hull_polygon", tuple(tuple(point) for point in hull_polygon)
        )
        object.__setattr__(self, "_edges", tuple(tuple(edge) for edge in edges))
        object.__setattr__(self, "_path", str(path))
        object.__setattr__(self, "_hull_path", str(hull_path))
        object.__setattr__(self, "_voronoi", Voronoi._from_snapshot(voronoi))

    @classmethod
    def new(cls, points: Sequence[tuple[float, float]]) -> "Delaunay":
        return cls(points)

    try_new = new

    @classmethod
    def from_points_iter(
        cls, points: Iterable[tuple[float, float]]
    ) -> "Delaunay":
        return cls(points)

    try_from_points_iter = from_points_iter

    def __len__(self) -> int:
        return len(self._points)

    def is_empty(self) -> bool:
        return not self._points

    def points(self) -> tuple[tuple[float, float], ...]:
        return self._points

    def point(self, index: int) -> tuple[float, float] | None:
        if not 0 <= index < len(self._points):
            return None
        return self._points[index]

    def find(self, x: float, y: float, start: int | None = None) -> int | None:
        return _delaunay_find(self._points, x, y, start=start)

    try_find = find

    def find_within_radius(self, x: float, y: float, radius: float) -> int | None:
        return _delaunay_find(self._points, x, y, radius=radius)

    try_find_within_radius = find_within_radius

    def neighbors(self, index: int) -> tuple[int, ...]:
        return self._voronoi.neighbors(index)

    def triangles(self) -> tuple[tuple[int, int, int], ...]:
        return self._triangles

    def triangle_count(self) -> int:
        return len(self._triangles)

    def hull(self) -> tuple[int, ...]:
        return self._hull

    def hull_polygon(self) -> tuple[tuple[float, float], ...]:
        return self._hull_polygon

    def render_to_path(self) -> str:
        return self._path

    render_to_path_into = render_to_path

    def render_hull_to_path(self) -> str:
        return self._hull_path

    def edges(self) -> tuple[tuple[int, int], ...]:
        return self._edges

    def voronoi(
        self, bounds: tuple[float, float, float, float] | None = None
    ) -> Voronoi:
        if bounds is None:
            return self._voronoi
        return Voronoi._from_snapshot(_delaunay_snapshot(self._points, bounds)[6])

    try_voronoi = voronoi


@dataclass(frozen=True)
class SimulationNode:
    index: int
    x: float
    y: float
    vx: float = 0.0
    vy: float = 0.0
    fx: float | None = None
    fy: float | None = None

    def __post_init__(self) -> None:
        if not isinstance(self.index, int) or isinstance(self.index, bool) or self.index < 0:
            raise ValueError("simulation node index must be a non-negative integer")
        values = (self.x, self.y, self.vx, self.vy)
        if any(not math.isfinite(value) for value in values):
            raise ValueError("simulation node coordinates and velocities must be finite")
        if self.fx is not None and not math.isfinite(self.fx):
            raise ValueError("fixed simulation x coordinate must be finite")
        if self.fy is not None and not math.isfinite(self.fy):
            raise ValueError("fixed simulation y coordinate must be finite")

    def fix_x(self, value: float | None) -> "SimulationNode":
        return replace(self, fx=value)

    def fix_y(self, value: float | None) -> "SimulationNode":
        return replace(self, fy=value)


ForceError = ValueError


def _frozen_updated(value: object, **changes: object) -> object:
    result = object.__new__(type(value))
    for name in value.__dataclass_fields__:
        object.__setattr__(result, name, changes.get(name, getattr(value, name)))
    return result


@dataclass(frozen=True)
class ForceCenter:
    x: float
    y: float


@dataclass(frozen=True, init=False)
class ForceX:
    _target: float
    _strength: float = 0.1

    def __init__(self, target: float) -> None:
        object.__setattr__(self, "_target", float(target))
        object.__setattr__(self, "_strength", 0.1)

    def target(self, value: float) -> "ForceX":
        return _frozen_updated(self, _target=float(value))

    def strength(self, value: float) -> "ForceX":
        return _frozen_updated(self, _strength=float(value))

    try_target = target
    try_strength = strength


@dataclass(frozen=True, init=False)
class ForceY:
    _target: float
    _strength: float = 0.1

    def __init__(self, target: float) -> None:
        object.__setattr__(self, "_target", float(target))
        object.__setattr__(self, "_strength", 0.1)

    def target(self, value: float) -> "ForceY":
        return _frozen_updated(self, _target=float(value))

    def strength(self, value: float) -> "ForceY":
        return _frozen_updated(self, _strength=float(value))

    try_target = target
    try_strength = strength


@dataclass(frozen=True, init=False)
class ForceRadial:
    _radius: float
    _x: float = 0.0
    _y: float = 0.0
    _strength: float = 0.1

    def __init__(self, radius: float) -> None:
        object.__setattr__(self, "_radius", float(radius))
        object.__setattr__(self, "_x", 0.0)
        object.__setattr__(self, "_y", 0.0)
        object.__setattr__(self, "_strength", 0.1)

    @classmethod
    def with_center(cls, radius: float, x: float, y: float) -> "ForceRadial":
        return cls(radius).x(x).y(y)

    try_with_center = with_center

    def radius(self, value: float) -> "ForceRadial":
        return _frozen_updated(self, _radius=float(value))

    def x(self, value: float) -> "ForceRadial":
        return _frozen_updated(self, _x=float(value))

    def y(self, value: float) -> "ForceRadial":
        return _frozen_updated(self, _y=float(value))

    def strength(self, value: float) -> "ForceRadial":
        return _frozen_updated(self, _strength=float(value))

    try_radius = radius
    try_x = x
    try_y = y
    try_strength = strength


@dataclass(frozen=True)
class ForceCollide:
    _radius: float = 1.0
    _strength: float = 1.0
    _iterations: int = 1
    _radii: tuple[float, ...] | None = None

    @classmethod
    def with_radius(cls, radius: float) -> "ForceCollide":
        return cls(_radius=float(radius))

    try_with_radius = with_radius

    def radius(self, value: float) -> "ForceCollide":
        return replace(self, _radius=float(value), _radii=None)

    def radii(self, values: Sequence[float]) -> "ForceCollide":
        return replace(self, _radii=tuple(float(value) for value in values))

    def strength(self, value: float) -> "ForceCollide":
        return replace(self, _strength=float(value))

    def iterations(self, value: int) -> "ForceCollide":
        return replace(self, _iterations=value)

    try_radius = radius
    try_radii = radii
    try_radii_for_nodes = radii
    try_strength = strength


@dataclass(frozen=True)
class ForceManyBody:
    _strength: float = -30.0
    _theta: float = math.inf
    _distance_min: float = 0.0
    _distance_max: float = math.inf

    def strength(self, value: float) -> "ForceManyBody":
        return replace(self, _strength=float(value))

    def theta(self, value: float) -> "ForceManyBody":
        return replace(self, _theta=float(value))

    def distance_min(self, value: float) -> "ForceManyBody":
        return replace(self, _distance_min=float(value))

    def distance_max(self, value: float) -> "ForceManyBody":
        return replace(self, _distance_max=float(value))

    try_strength = strength
    try_theta = theta
    try_distance_min = distance_min
    try_distance_max = distance_max


@dataclass(frozen=True)
class ForceLink:
    links: tuple[tuple[int, int], ...]
    _strength: float | None = None
    _distance: float = 30.0
    _iterations: int = 1

    def __post_init__(self) -> None:
        object.__setattr__(self, "links", tuple(tuple(link) for link in self.links))

    def strength(self, value: float) -> "ForceLink":
        return replace(self, _strength=float(value))

    def distance(self, value: float) -> "ForceLink":
        return replace(self, _distance=float(value))

    def iterations(self, value: int) -> "ForceLink":
        return replace(self, _iterations=value)

    try_strength = strength
    try_distance = distance


ForceConfiguration = (
    ForceCenter | ForceX | ForceY | ForceRadial | ForceCollide | ForceManyBody | ForceLink
)


@dataclass(frozen=True, init=False)
class Simulation:
    _nodes: tuple[SimulationNode, ...]
    _forces: tuple[ForceConfiguration, ...]
    _alpha: float
    _alpha_min: float
    _alpha_decay: float
    _alpha_target: float
    _velocity_decay: float

    def __init__(self, nodes: Sequence[SimulationNode]) -> None:
        normalized = tuple(nodes)
        if any(not isinstance(node, SimulationNode) for node in normalized):
            raise TypeError("simulation nodes must be SimulationNode values")
        object.__setattr__(self, "_nodes", normalized)
        object.__setattr__(self, "_forces", ())
        object.__setattr__(self, "_alpha", 1.0)
        object.__setattr__(self, "_alpha_min", 0.001)
        object.__setattr__(self, "_alpha_decay", 1.0 - 0.001 ** (1.0 / 300.0))
        object.__setattr__(self, "_alpha_target", 0.0)
        object.__setattr__(self, "_velocity_decay", 0.6)

    def nodes(self) -> tuple[SimulationNode, ...]:
        return self._nodes

    @property
    def current_alpha(self) -> float:
        return self._alpha

    def _updated(self, **changes: object) -> "Simulation":
        result = object.__new__(Simulation)
        for name in (
            "_nodes",
            "_forces",
            "_alpha",
            "_alpha_min",
            "_alpha_decay",
            "_alpha_target",
            "_velocity_decay",
        ):
            object.__setattr__(result, name, changes.get(name, getattr(self, name)))
        return result

    def force(self, force: ForceConfiguration) -> "Simulation":
        if not isinstance(
            force,
            (ForceCenter, ForceX, ForceY, ForceRadial, ForceCollide, ForceManyBody, ForceLink),
        ):
            raise TypeError("unsupported force configuration")
        return self._updated(_forces=(*self._forces, force))

    def alpha(self, value: float) -> "Simulation":
        return self._updated(_alpha=float(value))

    def alpha_min(self, value: float) -> "Simulation":
        return self._updated(_alpha_min=float(value))

    def alpha_decay(self, value: float) -> "Simulation":
        return self._updated(_alpha_decay=float(value))

    def alpha_target(self, value: float) -> "Simulation":
        return self._updated(_alpha_target=float(value))

    def velocity_decay(self, value: float) -> "Simulation":
        return self._updated(_velocity_decay=float(value))

    def tick(self, count: int = 1) -> "Simulation":
        if not isinstance(count, int) or isinstance(count, bool) or count < 0:
            raise ValueError("simulation tick count must be a non-negative integer")
        centers: list[tuple[float, float]] = []
        x_forces: list[tuple[float, float]] = []
        y_forces: list[tuple[float, float]] = []
        radial_forces: list[tuple[float, float, float, float]] = []
        collide_forces: list[tuple[float, float, int, tuple[float, ...] | None]] = []
        many_body_forces: list[tuple[float, float, float, float]] = []
        link_forces: list[
            tuple[tuple[tuple[int, int], ...], float | None, float, int]
        ] = []
        for force in self._forces:
            if isinstance(force, ForceCenter):
                centers.append((force.x, force.y))
            elif isinstance(force, ForceX):
                x_forces.append((force._target, force._strength))
            elif isinstance(force, ForceY):
                y_forces.append((force._target, force._strength))
            elif isinstance(force, ForceRadial):
                radial_forces.append(
                    (force._radius, force._x, force._y, force._strength)
                )
            elif isinstance(force, ForceCollide):
                collide_forces.append(
                    (force._radius, force._strength, force._iterations, force._radii)
                )
            elif isinstance(force, ForceManyBody):
                many_body_forces.append(
                    (
                        force._strength,
                        force._theta,
                        force._distance_min,
                        force._distance_max,
                    )
                )
            elif isinstance(force, ForceLink):
                link_forces.append(
                    (force.links, force._strength, force._distance, force._iterations)
                )
        result, alpha = _force_simulate(
            [
                (node.index, node.x, node.y, node.vx, node.vy, node.fx, node.fy)
                for node in self._nodes
            ],
            count,
            alpha=self._alpha,
            alpha_min=self._alpha_min,
            alpha_decay=self._alpha_decay,
            alpha_target=self._alpha_target,
            velocity_decay=self._velocity_decay,
            centers=centers,
            x_forces=x_forces,
            y_forces=y_forces,
            radial_forces=radial_forces,
            collide_forces=collide_forces,
            many_body_forces=many_body_forces,
            link_forces=link_forces,
        )
        return self._updated(
            _nodes=tuple(SimulationNode(*node) for node in result),
            _alpha=alpha,
        )

    try_tick = tick
    run = tick


@dataclass(frozen=True)
class Point:
    x: float = 0.0
    y: float = 0.0

    def distance(self, other: "Point") -> float:
        return float(_shape_point_distance((self.x, self.y), (other.x, other.y)))

    def lerp(self, other: "Point", t: float) -> "Point":
        return Point(*_shape_point_lerp((self.x, self.y), (other.x, other.y), t))


class PathCommandKind(str, Enum):
    MOVE_TO = "move_to"
    LINE_TO = "line_to"
    HORIZONTAL_LINE_TO = "horizontal_line_to"
    VERTICAL_LINE_TO = "vertical_line_to"
    CLOSE_PATH = "close_path"
    QUADRATIC_CURVE_TO = "quadratic_curve_to"
    CUBIC_CURVE_TO = "cubic_curve_to"
    ARC = "arc"
    ELLIPTICAL_ARC = "elliptical_arc"
    RECT = "rect"


@dataclass(frozen=True)
class PathCommand:
    kind: PathCommandKind
    values: tuple[float, ...] = ()


def _path_command_values(
    commands: Sequence[PathCommand],
) -> list[tuple[str, list[float]]]:
    return [(command.kind.value, list(command.values)) for command in commands]


def _path_from_native(commands: Sequence[tuple[str, Sequence[float]]]) -> "Path":
    return Path(
        tuple(
            PathCommand(PathCommandKind(kind), tuple(float(value) for value in values))
            for kind, values in commands
        )
    )


@dataclass(frozen=True)
class Path:
    _commands: tuple[PathCommand, ...] = ()

    def commands(self) -> tuple[PathCommand, ...]:
        return self._commands

    def is_empty(self) -> bool:
        return not self._commands

    def _analysis(
        self, tolerance: float = 0.5
    ) -> tuple[str, tuple[float, float, float, float] | None, tuple[Point, ...]]:
        svg, bounds, points = _shape_path_analyze(
            _path_command_values(self._commands), float(tolerance)
        )
        return (
            svg,
            tuple(bounds) if bounds is not None else None,
            tuple(Point(*point) for point in points),
        )

    def bounds(self) -> tuple[float, float, float, float] | None:
        return self._analysis()[1]

    def flatten(self, tolerance: float) -> tuple[Point, ...]:
        return self._analysis(tolerance)[2]

    def write_svg_string(self, buf: list[str]) -> None:
        buf.append(self.to_svg_string())

    def to_svg_string(self) -> str:
        return self._analysis()[0]

    def __str__(self) -> str:
        return self.to_svg_string()


@dataclass(frozen=True)
class PathBuilder:
    _commands: tuple[PathCommand, ...] = ()
    _current_point: Point = Point()
    _start_point: Point = Point()

    def _append(
        self,
        kind: PathCommandKind,
        values: Sequence[float],
        current: Point,
        start: Point | None = None,
    ) -> "PathBuilder":
        return PathBuilder(
            self._commands + (PathCommand(kind, tuple(float(v) for v in values)),),
            current,
            self._start_point if start is None else start,
        )

    def move_to(self, x: float, y: float) -> "PathBuilder":
        point = Point(float(x), float(y))
        return self._append(PathCommandKind.MOVE_TO, (x, y), point, point)

    def line_to(self, x: float, y: float) -> "PathBuilder":
        return self._append(
            PathCommandKind.LINE_TO, (x, y), Point(float(x), float(y))
        )

    def horizontal_line_to(self, x: float) -> "PathBuilder":
        return self._append(
            PathCommandKind.HORIZONTAL_LINE_TO,
            (x,),
            Point(float(x), self._current_point.y),
        )

    def vertical_line_to(self, y: float) -> "PathBuilder":
        return self._append(
            PathCommandKind.VERTICAL_LINE_TO,
            (y,),
            Point(self._current_point.x, float(y)),
        )

    def close_path(self) -> "PathBuilder":
        return self._append(
            PathCommandKind.CLOSE_PATH, (), self._start_point
        )

    def quadratic_curve_to(
        self, x1: float, y1: float, x: float, y: float
    ) -> "PathBuilder":
        return self._append(
            PathCommandKind.QUADRATIC_CURVE_TO,
            (x1, y1, x, y),
            Point(float(x), float(y)),
        )

    def cubic_curve_to(
        self,
        x1: float,
        y1: float,
        x2: float,
        y2: float,
        x: float,
        y: float,
    ) -> "PathBuilder":
        return self._append(
            PathCommandKind.CUBIC_CURVE_TO,
            (x1, y1, x2, y2, x, y),
            Point(float(x), float(y)),
        )

    def arc(
        self,
        x: float,
        y: float,
        radius: float,
        start_angle: float,
        end_angle: float,
        anticlockwise: bool = False,
    ) -> "PathBuilder":
        current = Point(
            float(x) + float(radius) * math.cos(float(end_angle)),
            float(y) + float(radius) * math.sin(float(end_angle)),
        )
        return self._append(
            PathCommandKind.ARC,
            (x, y, radius, start_angle, end_angle, float(bool(anticlockwise))),
            current,
        )

    def elliptical_arc(
        self,
        rx: float,
        ry: float,
        x_axis_rotation: float,
        large_arc: bool,
        sweep: bool,
        x: float,
        y: float,
    ) -> "PathBuilder":
        return self._append(
            PathCommandKind.ELLIPTICAL_ARC,
            (rx, ry, x_axis_rotation, float(bool(large_arc)), float(bool(sweep)), x, y),
            Point(float(x), float(y)),
        )

    def rect(self, x: float, y: float, width: float, height: float) -> "PathBuilder":
        return self._append(
            PathCommandKind.RECT,
            (x, y, width, height),
            Point(float(x), float(y)),
        )

    def build(self) -> Path:
        return Path(self._commands)

    def current_point(self) -> Point:
        return self._current_point


def _append_path_command(builder: PathBuilder, command: PathCommand) -> PathBuilder:
    values = command.values
    if command.kind is PathCommandKind.MOVE_TO:
        return builder.move_to(*values)
    if command.kind is PathCommandKind.LINE_TO:
        return builder.line_to(*values)
    if command.kind is PathCommandKind.HORIZONTAL_LINE_TO:
        return builder.horizontal_line_to(*values)
    if command.kind is PathCommandKind.VERTICAL_LINE_TO:
        return builder.vertical_line_to(*values)
    if command.kind is PathCommandKind.CLOSE_PATH:
        return builder.close_path()
    if command.kind is PathCommandKind.QUADRATIC_CURVE_TO:
        return builder.quadratic_curve_to(*values)
    if command.kind is PathCommandKind.CUBIC_CURVE_TO:
        return builder.cubic_curve_to(*values)
    if command.kind is PathCommandKind.ARC:
        return builder.arc(*values[:5], bool(values[5]))
    if command.kind is PathCommandKind.ELLIPTICAL_ARC:
        return builder.elliptical_arc(
            values[0], values[1], values[2], bool(values[3]), bool(values[4]), values[5], values[6]
        )
    return builder.rect(*values)


@dataclass(frozen=True)
class ShapePath:
    svg: str
    points: tuple[tuple[float, float], ...] = ()

    def to_svg_string(self) -> str:
        return self.svg

    def __str__(self) -> str:
        return self.svg


ShapeGenerationError = ValueError


@dataclass(frozen=True)
class ArcDatum:
    _inner_radius: float = 0.0
    _outer_radius: float = 100.0
    _start_angle: float = 0.0
    _end_angle: float = math.tau
    _corner_radius: float = 0.0
    _pad_angle: float = 0.0

    def inner_radius(self, value: float) -> "ArcDatum":
        return replace(self, _inner_radius=float(value))

    def outer_radius(self, value: float) -> "ArcDatum":
        return replace(self, _outer_radius=float(value))

    def start_angle(self, value: float) -> "ArcDatum":
        return replace(self, _start_angle=float(value))

    def end_angle(self, value: float) -> "ArcDatum":
        return replace(self, _end_angle=float(value))

    def corner_radius(self, value: float) -> "ArcDatum":
        return replace(self, _corner_radius=float(value))

    def pad_angle(self, value: float) -> "ArcDatum":
        return replace(self, _pad_angle=float(value))

    def _tuple(self) -> tuple[float, float, float, float, float, float]:
        return (
            self._inner_radius,
            self._outer_radius,
            self._start_angle,
            self._end_angle,
            self._corner_radius,
            self._pad_angle,
        )

    def centroid(self) -> tuple[float, float]:
        return tuple(_shape_arc(self._tuple(), segments=1)[1])


@dataclass(frozen=True)
class Arc:
    _center: tuple[float, float] = (0.0, 0.0)

    def center(self, x: float, y: float) -> "Arc":
        return replace(self, _center=(float(x), float(y)))

    def generate(self, datum: ArcDatum, segments: int = 32) -> ShapePath:
        path, _, points = _shape_arc(
            datum._tuple(), center=self._center, segments=segments
        )
        return ShapePath(path, tuple(tuple(point) for point in points))

    try_generate = generate

    def path_string(self, datum: ArcDatum) -> str:
        return self.generate(datum).svg

    try_path_string = path_string


def arc_points(
    datum: ArcDatum,
    segments: int,
    cx: float = 0.0,
    cy: float = 0.0,
) -> tuple[tuple[float, float], ...]:
    return Arc().center(cx, cy).generate(datum, segments).points


try_arc_points = arc_points


class SymbolType(str, Enum):
    CIRCLE = "circle"
    CROSS = "cross"
    DIAMOND = "diamond"
    SQUARE = "square"
    STAR = "star"
    TRIANGLE = "triangle"
    TRIANGLE_DOWN = "triangle_down"
    TRIANGLE_LEFT = "triangle_left"
    TRIANGLE_RIGHT = "triangle_right"
    WYE = "wye"


@dataclass(frozen=True)
class Symbol:
    _symbol_type: SymbolType = SymbolType.CIRCLE
    _size: float = 64.0

    @classmethod
    def circle(cls, size: float) -> "Symbol":
        return cls(SymbolType.CIRCLE, float(size))

    @classmethod
    def cross(cls, size: float) -> "Symbol":
        return cls(SymbolType.CROSS, float(size))

    @classmethod
    def diamond(cls, size: float) -> "Symbol":
        return cls(SymbolType.DIAMOND, float(size))

    @classmethod
    def square(cls, size: float) -> "Symbol":
        return cls(SymbolType.SQUARE, float(size))

    @classmethod
    def star(cls, size: float) -> "Symbol":
        return cls(SymbolType.STAR, float(size))

    @classmethod
    def triangle(cls, size: float) -> "Symbol":
        return cls(SymbolType.TRIANGLE, float(size))

    def symbol_type(self, value: SymbolType | str) -> "Symbol":
        return replace(self, _symbol_type=SymbolType(value))

    def size(self, value: float) -> "Symbol":
        return replace(self, _size=float(value))

    def generate_at(self, x: float = 0.0, y: float = 0.0) -> ShapePath:
        path, points, _ = _shape_symbol(
            self._symbol_type.value, self._size, center=(x, y)
        )
        return ShapePath(path, tuple(tuple(point) for point in points))

    try_generate_at = generate_at

    def generate(self) -> ShapePath:
        return self.generate_at()

    try_generate = generate

    def points(self) -> tuple[tuple[float, float], ...]:
        return self.generate().points

    try_points = points

    def radius(self) -> float:
        return float(_shape_symbol(self._symbol_type.value, self._size)[2])

    try_radius = radius


def symbol_radius(symbol_type: SymbolType | str, size: float) -> float:
    return Symbol(SymbolType(symbol_type), float(size)).radius()


try_symbol_radius = symbol_radius


class LinkDirection(str, Enum):
    HORIZONTAL = "horizontal"
    VERTICAL = "vertical"
    RADIAL = "radial"


@dataclass(frozen=True)
class Link:
    source_x: float
    source_y: float
    target_x: float
    target_y: float

    @classmethod
    def from_points(
        cls, source: tuple[float, float], target: tuple[float, float]
    ) -> "Link":
        return cls(source[0], source[1], target[0], target[1])

    def validate(self) -> None:
        _shape_link("horizontal", self.source, self.target)

    @property
    def source(self) -> tuple[float, float]:
        return (self.source_x, self.source_y)

    @property
    def target(self) -> tuple[float, float]:
        return (self.target_x, self.target_y)


def link_horizontal(link: Link) -> str:
    return _shape_link("horizontal", link.source, link.target)


try_link_horizontal = link_horizontal


def link_vertical(link: Link) -> str:
    return _shape_link("vertical", link.source, link.target)


try_link_vertical = link_vertical


def link_step(link: Link, direction: LinkDirection | str) -> str:
    return _shape_link(f"step_{LinkDirection(direction).value}", link.source, link.target)


try_link_step = link_step


@dataclass(frozen=True)
class RadialLink:
    source_angle: float
    source_radius: float
    target_angle: float
    target_radius: float

    def to_cartesian(self, cx: float = 0.0, cy: float = 0.0) -> Link:
        _, source, target = _shape_radial_link(
            self.source_angle,
            self.source_radius,
            self.target_angle,
            self.target_radius,
            (cx, cy),
        )
        return Link.from_points(source, target)

    try_to_cartesian = to_cartesian


def link_radial(link: RadialLink, cx: float = 0.0, cy: float = 0.0) -> str:
    return _shape_radial_link(
        link.source_angle,
        link.source_radius,
        link.target_angle,
        link.target_radius,
        (cx, cy),
    )[0]


try_link_radial = link_radial


@dataclass(frozen=True)
class PieSlice:
    data: object
    arc: ArcDatum
    index: int
    value: float


@dataclass(frozen=True)
class Pie:
    _start_angle: float = 0.0
    _end_angle: float = math.tau
    _pad_angle: float = 0.0
    _inner_radius: float = 0.0
    _outer_radius: float = 100.0
    _corner_radius: float = 0.0
    _sort: bool = False
    _descending: bool = True

    def start_angle(self, value: float) -> "Pie":
        return replace(self, _start_angle=float(value))

    def end_angle(self, value: float) -> "Pie":
        return replace(self, _end_angle=float(value))

    def pad_angle(self, value: float) -> "Pie":
        return replace(self, _pad_angle=float(value))

    def inner_radius(self, value: float) -> "Pie":
        return replace(self, _inner_radius=float(value))

    def outer_radius(self, value: float) -> "Pie":
        return replace(self, _outer_radius=float(value))

    def corner_radius(self, value: float) -> "Pie":
        return replace(self, _corner_radius=float(value))

    def sort(self, enabled: bool = True) -> "Pie":
        return replace(self, _sort=enabled)

    def sort_descending(self, enabled: bool = True) -> "Pie":
        return replace(self, _descending=enabled)

    def generate(
        self,
        data: Sequence[object],
        value: Callable[[object], float] = float,
    ) -> list[PieSlice]:
        values = [float(value(item)) for item in data]
        result = _shape_pie(
            values,
            start_angle=self._start_angle,
            end_angle=self._end_angle,
            pad_angle=self._pad_angle,
            inner_radius=self._inner_radius,
            outer_radius=self._outer_radius,
            corner_radius=self._corner_radius,
            sort=self._sort,
            descending=self._descending,
        )
        return [
            PieSlice(data[index], ArcDatum(*arc), index, slice_value)
            for index, slice_value, arc in result
        ]

    try_generate = generate


def pie(values: Sequence[float], radius: float) -> list[PieSlice]:
    return Pie().outer_radius(radius).generate(values)


try_pie = pie


def donut(
    values: Sequence[float], inner_radius: float, outer_radius: float
) -> list[PieSlice]:
    return Pie().inner_radius(inner_radius).outer_radius(outer_radius).generate(values)


try_donut = donut


def half_pie(values: Sequence[float], radius: float) -> list[PieSlice]:
    return (
        Pie()
        .outer_radius(radius)
        .start_angle(-math.pi / 2.0)
        .end_angle(math.pi / 2.0)
        .generate(values)
    )


try_half_pie = half_pie


StackLayoutError = ValueError
RadialGenerationError = ValueError


class StackOrder(str, Enum):
    NONE = "none"
    ASCENDING = "ascending"
    DESCENDING = "descending"
    APPEARANCE = "appearance"
    INSIDE_OUT = "inside_out"
    REVERSE = "reverse"


class StackOffset(str, Enum):
    NONE = "none"
    EXPAND = "expand"
    DIVERGING = "diverging"
    SILHOUETTE = "silhouette"
    WIGGLE = "wiggle"


@dataclass(frozen=True)
class StackSeries:
    key: str
    data: tuple[float, ...]
    values: tuple[tuple[float, float], ...]
    index: int

    def get(self, index: int) -> tuple[float, float] | None:
        if index < 0 or index >= len(self.values):
            return None
        return self.values[index]


@dataclass(frozen=True)
class Stack:
    _keys: tuple[str, ...] = ()
    _order: StackOrder = StackOrder.NONE
    _offset: StackOffset = StackOffset.NONE

    def keys(self, values: Sequence[str]) -> "Stack":
        return replace(self, _keys=tuple(str(value) for value in values))

    def order(self, value: StackOrder | str) -> "Stack":
        return replace(self, _order=StackOrder(value))

    def offset(self, value: StackOffset | str) -> "Stack":
        return replace(self, _offset=StackOffset(value))

    def generate(self, data: Sequence[Sequence[float]]) -> list[StackSeries]:
        rows = [[float(value) for value in row] for row in data]
        result = _shape_stack(
            rows,
            list(self._keys),
            self._order.value,
            self._offset.value,
        )
        return [
            StackSeries(
                key,
                tuple(values),
                tuple(tuple(bounds) for bounds in stacked),
                index,
            )
            for key, values, stacked, index in result
        ]

    try_generate = generate


def _stack_keys(data: Sequence[Sequence[float]]) -> tuple[str, ...]:
    return tuple(str(index) for index in range(len(data[0]) if data else 0))


def stack(data: Sequence[Sequence[float]]) -> list[StackSeries]:
    return Stack().keys(_stack_keys(data)).generate(data)


try_stack = stack


def stack_expand(data: Sequence[Sequence[float]]) -> list[StackSeries]:
    return Stack().keys(_stack_keys(data)).offset(StackOffset.EXPAND).generate(data)


try_stack_expand = stack_expand


def streamgraph(data: Sequence[Sequence[float]]) -> list[StackSeries]:
    return (
        Stack()
        .keys(_stack_keys(data))
        .order(StackOrder.INSIDE_OUT)
        .offset(StackOffset.WIGGLE)
        .generate(data)
    )


try_streamgraph = streamgraph


class CurveKind(str, Enum):
    LINEAR = "linear"
    STEP = "step"
    STEP_BEFORE = "step_before"
    STEP_AFTER = "step_after"
    BASIS = "basis"
    BASIS_CLOSED = "basis_closed"
    BASIS_OPEN = "basis_open"
    BUNDLE = "bundle"
    CARDINAL = "cardinal"
    CARDINAL_CLOSED = "cardinal_closed"
    CARDINAL_OPEN = "cardinal_open"
    CATMULL_ROM = "catmull_rom"
    CATMULL_ROM_CLOSED = "catmull_rom_closed"
    CATMULL_ROM_OPEN = "catmull_rom_open"
    MONOTONE_X = "monotone_x"
    MONOTONE_Y = "monotone_y"
    NATURAL = "natural"


@dataclass(frozen=True)
class Curve:
    kind: CurveKind = CurveKind.LINEAR
    parameter: float | None = None

    @classmethod
    def linear(cls) -> "Curve":
        return cls(CurveKind.LINEAR)

    @classmethod
    def step(cls) -> "Curve":
        return cls(CurveKind.STEP)

    @classmethod
    def step_before(cls) -> "Curve":
        return cls(CurveKind.STEP_BEFORE)

    @classmethod
    def step_after(cls) -> "Curve":
        return cls(CurveKind.STEP_AFTER)

    @classmethod
    def basis(cls) -> "Curve":
        return cls(CurveKind.BASIS)

    @classmethod
    def basis_closed(cls) -> "Curve":
        return cls(CurveKind.BASIS_CLOSED)

    @classmethod
    def basis_open(cls) -> "Curve":
        return cls(CurveKind.BASIS_OPEN)

    @classmethod
    def bundle(cls, beta: float) -> "Curve":
        return cls(CurveKind.BUNDLE, float(beta))

    @classmethod
    def cardinal(cls, tension: float) -> "Curve":
        return cls(CurveKind.CARDINAL, float(tension))

    @classmethod
    def cardinal_closed(cls, tension: float) -> "Curve":
        return cls(CurveKind.CARDINAL_CLOSED, float(tension))

    @classmethod
    def cardinal_open(cls, tension: float) -> "Curve":
        return cls(CurveKind.CARDINAL_OPEN, float(tension))

    @classmethod
    def catmull_rom(cls, alpha: float) -> "Curve":
        return cls(CurveKind.CATMULL_ROM, float(alpha))

    @classmethod
    def catmull_rom_closed(cls, alpha: float) -> "Curve":
        return cls(CurveKind.CATMULL_ROM_CLOSED, float(alpha))

    @classmethod
    def catmull_rom_open(cls, alpha: float) -> "Curve":
        return cls(CurveKind.CATMULL_ROM_OPEN, float(alpha))

    @classmethod
    def monotone_x(cls) -> "Curve":
        return cls(CurveKind.MONOTONE_X)

    @classmethod
    def monotone_y(cls) -> "Curve":
        return cls(CurveKind.MONOTONE_Y)

    @classmethod
    def natural(cls) -> "Curve":
        return cls(CurveKind.NATURAL)

    def interpolate(
        self, points: Sequence[tuple[float, float]]
    ) -> tuple[tuple[float, float], ...]:
        return tuple(
            tuple(point)
            for point in _shape_curve_interpolate(
                self.kind.value,
                self.parameter,
                [(float(x), float(y)) for x, y in points],
            )
        )

    def interpolate_into(
        self,
        points: Sequence[tuple[float, float]],
        out: list[tuple[float, float]],
    ) -> None:
        out.clear()
        out.extend(self.interpolate(points))

    def subdivisions(self) -> int:
        if self.kind is CurveKind.LINEAR:
            return 1
        if self.kind in (CurveKind.STEP, CurveKind.STEP_BEFORE, CurveKind.STEP_AFTER):
            return 2
        return 16


def _curve(value: Curve | CurveKind | str) -> Curve:
    if isinstance(value, Curve):
        return value
    return Curve(CurveKind(value))


@dataclass(frozen=True)
class RadialPoint:
    angle: float
    radius: float

    def to_cartesian(self, cx: float = 0.0, cy: float = 0.0) -> tuple[float, float]:
        return tuple(_radial_point_to_cartesian(self.angle, self.radius, cx, cy))

    try_to_cartesian = to_cartesian

    @classmethod
    def from_cartesian(
        cls, x: float, y: float, cx: float = 0.0, cy: float = 0.0
    ) -> "RadialPoint":
        return cls(*_radial_point_from_cartesian(x, y, cx, cy))


def _radial_values(
    points: Sequence[RadialPoint | tuple[float, float]],
) -> list[tuple[float, float]]:
    return [
        (float(point.angle), float(point.radius))
        if isinstance(point, RadialPoint)
        else (float(point[0]), float(point[1]))
        for point in points
    ]


@dataclass(frozen=True)
class RadialLineConfig:
    cx: float
    cy: float
    _curve_value: Curve = Curve()
    _closed: bool = False

    def curve(self, value: Curve | CurveKind | str) -> "RadialLineConfig":
        return replace(self, _curve_value=_curve(value))

    def closed(self, value: bool) -> "RadialLineConfig":
        return replace(self, _closed=bool(value))

    def generate(self, points: Sequence[RadialPoint | tuple[float, float]]) -> str:
        return _radial_line_path(
            _radial_values(points),
            self.cx,
            self.cy,
            self._curve_value.kind.value,
            self._curve_value.parameter,
            self._closed,
        )


@dataclass(frozen=True)
class RadialAreaConfig:
    cx: float
    cy: float
    _inner_radius: float = 0.0
    _curve_value: Curve = Curve()

    def inner_radius(self, value: float) -> "RadialAreaConfig":
        return replace(self, _inner_radius=float(value))

    def curve(self, value: Curve | CurveKind | str) -> "RadialAreaConfig":
        return replace(self, _curve_value=_curve(value))

    def generate(self, points: Sequence[RadialPoint | tuple[float, float]]) -> str:
        return _radial_area_path(
            _radial_values(points),
            self.cx,
            self.cy,
            self._inner_radius,
            self._curve_value.kind.value,
            self._curve_value.parameter,
        )


def radial_line(
    points: Sequence[RadialPoint | tuple[float, float]], config: RadialLineConfig
) -> str:
    return config.generate(points)


try_radial_line = radial_line


def radial_area(
    points: Sequence[RadialPoint | tuple[float, float]], config: RadialAreaConfig
) -> str:
    return config.generate(points)


try_radial_area = radial_area


def polar_grid_circles(cx: float, cy: float, radii: Sequence[float]) -> list[str]:
    return list(_polar_grid_circle_paths(cx, cy, [float(radius) for radius in radii]))


try_polar_grid_circles = polar_grid_circles


def polar_grid_rays(
    cx: float,
    cy: float,
    outer_radius: float,
    angles: Sequence[float],
    inner_radius: float = 0.0,
) -> list[str]:
    return list(
        _polar_grid_ray_paths(
            cx,
            cy,
            outer_radius,
            [float(angle) for angle in angles],
            inner_radius,
        )
    )


try_polar_grid_rays = polar_grid_rays


AreaGenerationError = ValueError


def _area_zero(_: object) -> float:
    return 0.0


def _area_defined(_: object) -> bool:
    return True


@dataclass(frozen=True)
class Area:
    _x: Callable[[object], float] = _area_zero
    _x0: Callable[[object], float] | None = None
    _x1: Callable[[object], float] | None = None
    _y: Callable[[object], float] = _area_zero
    _y0: Callable[[object], float] = _area_zero
    _y1: Callable[[object], float] | None = None
    _defined: Callable[[object], bool] = _area_defined
    _curve_value: Curve = Curve()

    def x(self, accessor: Callable[[object], float]) -> "Area":
        return replace(self, _x=accessor)

    def x0(self, accessor: Callable[[object], float]) -> "Area":
        return replace(self, _x0=accessor)

    def x1(self, accessor: Callable[[object], float]) -> "Area":
        return replace(self, _x1=accessor)

    def y(self, accessor: Callable[[object], float]) -> "Area":
        return replace(self, _y=accessor)

    def y0(self, accessor: Callable[[object], float]) -> "Area":
        return replace(self, _y0=accessor)

    def y1(self, accessor: Callable[[object], float]) -> "Area":
        return replace(self, _y1=accessor)

    def defined(self, accessor: Callable[[object], bool]) -> "Area":
        return replace(self, _defined=accessor)

    def curve(self, value: Curve | CurveKind | str) -> "Area":
        return replace(self, _curve_value=_curve(value))

    def generate(self, data: Sequence[object]) -> Path:
        top: list[tuple[float, float]] = []
        bottom: list[tuple[float, float]] = []
        defined: list[bool] = []
        for item in data:
            is_defined = bool(self._defined(item))
            defined.append(is_defined)
            if not is_defined:
                top.append((0.0, 0.0))
                bottom.append((0.0, 0.0))
                continue
            shared_x = (
                float(self._x(item))
                if self._x1 is None or self._x0 is None
                else None
            )
            top.append(
                (
                    float(self._x1(item)) if self._x1 is not None else shared_x,
                    float(self._y1(item)) if self._y1 is not None else float(self._y(item)),
                )
            )
            bottom.append(
                (
                    float(self._x0(item)) if self._x0 is not None else shared_x,
                    float(self._y0(item)),
                )
            )
        commands = _shape_area_generate(
            top,
            bottom,
            defined,
            self._curve_value.kind.value,
            self._curve_value.parameter,
        )
        return _path_from_native(commands)

    try_generate = generate

    def generate_into(self, data: Sequence[object], builder: PathBuilder) -> PathBuilder:
        result = builder
        for command in self.generate(data).commands():
            result = _append_path_command(result, command)
        return result

    try_generate_into = generate_into


@dataclass(frozen=True)
class SimpleArea:
    x: tuple[float, ...]
    y0: tuple[float, ...]
    y1: tuple[float, ...]

    def __init__(
        self, x: Sequence[float], y0: Sequence[float], y1: Sequence[float]
    ) -> None:
        object.__setattr__(self, "x", tuple(float(value) for value in x))
        object.__setattr__(self, "y0", tuple(float(value) for value in y0))
        object.__setattr__(self, "y1", tuple(float(value) for value in y1))

    def _native(self) -> tuple[tuple[Point, ...], Path]:
        points, commands = _shape_simple_area(list(self.x), list(self.y0), list(self.y1))
        return tuple(Point(*point) for point in points), _path_from_native(commands)

    def points(self) -> tuple[Point, ...]:
        return self._native()[0]

    try_points = points

    def path(self) -> Path:
        return self._native()[1]

    try_path = path


def area_points(
    data: Sequence[object],
    x: Callable[[object], float],
    y0: Callable[[object], float],
    y1: Callable[[object], float],
) -> tuple[Point, ...]:
    return SimpleArea(
        [x(item) for item in data],
        [y0(item) for item in data],
        [y1(item) for item in data],
    ).points()


try_area_points = area_points


ChordLayoutError = ValueError


class ChordSort(str, Enum):
    NONE = "none"
    ASCENDING = "ascending"
    DESCENDING = "descending"


@dataclass(frozen=True)
class ChordSubgroup:
    index: int
    start_angle: float
    end_angle: float
    value: float

    def _tuple(self) -> tuple[int, float, float, float]:
        return self.index, self.start_angle, self.end_angle, self.value


@dataclass(frozen=True)
class ChordGroup:
    index: int
    start_angle: float
    end_angle: float
    value: float


@dataclass(frozen=True)
class Chord:
    source: ChordSubgroup
    target: ChordSubgroup


@dataclass(frozen=True)
class ChordResult:
    chords: tuple[Chord, ...]
    groups: tuple[ChordGroup, ...]


@dataclass(frozen=True)
class ChordLayout:
    _pad_angle: float = 0.0
    _sort_groups: ChordSort = ChordSort.NONE
    _sort_subgroups: ChordSort = ChordSort.NONE
    _sort_chords: ChordSort = ChordSort.NONE

    def pad_angle(self, angle: float) -> "ChordLayout":
        return replace(self, _pad_angle=float(angle))

    def sort_groups(self, order: ChordSort | str) -> "ChordLayout":
        return replace(self, _sort_groups=ChordSort(order))

    def sort_subgroups(self, order: ChordSort | str) -> "ChordLayout":
        return replace(self, _sort_subgroups=ChordSort(order))

    def sort_chords(self, order: ChordSort | str) -> "ChordLayout":
        return replace(self, _sort_chords=ChordSort(order))

    def compute(self, matrix: Sequence[Sequence[float]]) -> ChordResult:
        chords, groups = _chord_layout(
            [[float(value) for value in row] for row in matrix],
            self._pad_angle,
            self._sort_groups.value,
            self._sort_subgroups.value,
            self._sort_chords.value,
        )
        return ChordResult(
            tuple(
                Chord(ChordSubgroup(*source), ChordSubgroup(*target))
                for source, target in chords
            ),
            tuple(ChordGroup(*group) for group in groups),
        )

    try_compute = compute


@dataclass(frozen=True)
class RibbonGenerator:
    radius: float
    center_x: float = 0.0
    center_y: float = 0.0

    def center(self, x: float, y: float) -> "RibbonGenerator":
        return replace(self, center_x=float(x), center_y=float(y))

    def generate_path(self, chord: Chord) -> Path:
        return _path_from_native(
            _chord_ribbon_path(
                (chord.source._tuple(), chord.target._tuple()),
                self.radius,
                (self.center_x, self.center_y),
            )
        )

    def generate(self, chord: Chord) -> str:
        return self.generate_path(chord).to_svg_string()


def _random_wrapper(cls: type, inner: object):
    value = object.__new__(cls)
    object.__setattr__(value, "_inner", inner)
    return value


@dataclass(frozen=True, init=False)
class LcgRng:
    _inner: object

    def __init__(self, seed: int) -> None:
        object.__setattr__(self, "_inner", _NativeLcgRng(int(seed)))

    @classmethod
    def new(cls, seed: int) -> "LcgRng":
        return cls(seed)

    @classmethod
    def default_seed(cls) -> "LcgRng":
        return _random_wrapper(cls, _NativeLcgRng())

    def next_f64(self) -> float:
        return float(self._inner.next_f64())

    def next_u64(self, max: int) -> int:
        return int(self._inner.next_u64(int(max)))


@dataclass(frozen=True, init=False)
class RandomUniform:
    _inner: object

    def __init__(self, min: float, max: float) -> None:
        object.__setattr__(self, "_inner", _NativeRandomUniform(min, max))

    @classmethod
    def new(cls, min: float, max: float) -> "RandomUniform":
        return cls(min, max)

    @classmethod
    def with_seed(cls, min: float, max: float, seed: int) -> "RandomUniform":
        return _random_wrapper(cls, _NativeRandomUniform(min, max, int(seed)))

    @classmethod
    def unit(cls) -> "RandomUniform":
        return cls(0.0, 1.0)

    def sample(self) -> float:
        return float(self._inner.sample())


@dataclass(frozen=True, init=False)
class RandomNormal:
    _inner: object

    def __init__(self, mean: float, std_dev: float) -> None:
        object.__setattr__(self, "_inner", _NativeRandomNormal(mean, std_dev))

    @classmethod
    def new(cls, mean: float, std_dev: float) -> "RandomNormal":
        return cls(mean, std_dev)

    @classmethod
    def with_seed(cls, mean: float, std_dev: float, seed: int) -> "RandomNormal":
        return _random_wrapper(cls, _NativeRandomNormal(mean, std_dev, int(seed)))

    @classmethod
    def standard(cls) -> "RandomNormal":
        return cls(0.0, 1.0)

    def sample(self) -> float:
        return float(self._inner.sample())


@dataclass(frozen=True, init=False)
class RandomLogNormal:
    _inner: object

    def __init__(self, mu: float, sigma: float) -> None:
        object.__setattr__(self, "_inner", _NativeRandomLogNormal(mu, sigma))

    @classmethod
    def new(cls, mu: float, sigma: float) -> "RandomLogNormal":
        return cls(mu, sigma)

    @classmethod
    def with_seed(cls, mu: float, sigma: float, seed: int) -> "RandomLogNormal":
        return _random_wrapper(cls, _NativeRandomLogNormal(mu, sigma, int(seed)))

    def sample(self) -> float:
        return float(self._inner.sample())


@dataclass(frozen=True, init=False)
class RandomExponential:
    _inner: object

    def __init__(self, lambda_: float) -> None:
        object.__setattr__(self, "_inner", _NativeRandomExponential(lambda_))

    @classmethod
    def new(cls, lambda_: float) -> "RandomExponential":
        return cls(lambda_)

    @classmethod
    def with_seed(cls, lambda_: float, seed: int) -> "RandomExponential":
        return _random_wrapper(cls, _NativeRandomExponential(lambda_, int(seed)))

    def sample(self) -> float:
        return float(self._inner.sample())


@dataclass(frozen=True, init=False)
class RandomBernoulli:
    _inner: object

    def __init__(self, p: float) -> None:
        object.__setattr__(self, "_inner", _NativeRandomBernoulli(p))

    @classmethod
    def new(cls, p: float) -> "RandomBernoulli":
        return cls(p)

    @classmethod
    def with_seed(cls, p: float, seed: int) -> "RandomBernoulli":
        return _random_wrapper(cls, _NativeRandomBernoulli(p, int(seed)))

    def sample(self) -> bool:
        return bool(self._inner.sample())

    def sample_int(self) -> int:
        return int(self._inner.sample_int())


@dataclass(frozen=True, init=False)
class RandomPoisson:
    _inner: object

    def __init__(self, lambda_: float) -> None:
        object.__setattr__(self, "_inner", _NativeRandomPoisson(lambda_))

    @classmethod
    def new(cls, lambda_: float) -> "RandomPoisson":
        return cls(lambda_)

    @classmethod
    def with_seed(cls, lambda_: float, seed: int) -> "RandomPoisson":
        return _random_wrapper(cls, _NativeRandomPoisson(lambda_, int(seed)))

    def sample(self) -> int:
        return int(self._inner.sample())


@dataclass(frozen=True, init=False)
class RandomIrwinHall:
    _inner: object

    def __init__(self, n: int) -> None:
        object.__setattr__(self, "_inner", _NativeRandomIrwinHall(int(n)))

    @classmethod
    def new(cls, n: int) -> "RandomIrwinHall":
        return cls(n)

    @classmethod
    def with_seed(cls, n: int, seed: int) -> "RandomIrwinHall":
        return _random_wrapper(cls, _NativeRandomIrwinHall(int(n), int(seed)))

    def sample(self) -> float:
        return float(self._inner.sample())


@dataclass(frozen=True, init=False)
class RandomBates:
    _inner: object

    def __init__(self, n: int) -> None:
        object.__setattr__(self, "_inner", _NativeRandomBates(int(n)))

    @classmethod
    def new(cls, n: int) -> "RandomBates":
        return cls(n)

    @classmethod
    def with_seed(cls, n: int, seed: int) -> "RandomBates":
        return _random_wrapper(cls, _NativeRandomBates(int(n), int(seed)))

    def sample(self) -> float:
        return float(self._inner.sample())


def shuffle(
    rng: LcgRng | Sequence[object], data: Sequence[object] | None = None
) -> list[object]:
    """Return a shuffled copy, consuming the supplied RNG.

    The one-argument form is retained as the strict v1 numeric convenience shim.
    """

    if data is None:
        return list(_shuffle_values(rng))
    if not isinstance(rng, LcgRng):
        raise TypeError("rng must be an LcgRng")
    result = list(data)
    shuffle_in_place(rng, result)
    return result


def shuffle_in_place(rng: LcgRng, data: list[object]) -> None:
    if not isinstance(rng, LcgRng):
        raise TypeError("rng must be an LcgRng")
    if not isinstance(data, list):
        raise TypeError("data must be a mutable list")
    for index in builtins.range(builtins.len(data) - 1, 0, -1):
        other = rng.next_u64(index + 1)
        data[index], data[other] = data[other], data[index]


HALF_PI: Final[float] = math.pi / 2.0
TAU: Final[float] = math.tau
EPSILON: Final[float] = 1e-6


def radians(degrees: float) -> float:
    return float(_geo_radians(degrees))


def degrees(radians: float) -> float:
    return float(_geo_degrees(radians))


def geo_distance(lon1: float, lat1: float, lon2: float, lat2: float) -> float:
    return float(_geo_distance((lon1, lat1), (lon2, lat2)))


def geo_length(coordinates: Sequence[tuple[float, float]]) -> float:
    return float(_geo_length(coordinates))


def geo_interpolate(
    lon1: float, lat1: float, lon2: float, lat2: float, t: float
) -> tuple[float, float]:
    return tuple(_geo_interpolate((lon1, lat1), (lon2, lat2), t))


def geo_area(coordinates: Sequence[tuple[float, float]]) -> float:
    return float(_geo_area(coordinates))


def geo_bounds(
    coordinates: Sequence[tuple[float, float]],
) -> tuple[tuple[float, float], tuple[float, float]]:
    bounds = _geo_bounds(coordinates)
    return tuple(bounds[0]), tuple(bounds[1])


def geo_centroid(
    coordinates: Sequence[tuple[float, float]],
) -> tuple[float, float]:
    return tuple(_geo_centroid(coordinates))


def geo_contains(
    coordinates: Sequence[tuple[float, float]], lon: float, lat: float
) -> bool:
    return bool(_geo_contains(coordinates, (lon, lat)))


@dataclass(frozen=True)
class GraticuleConfig:
    extent_major: tuple[tuple[float, float], tuple[float, float]] = (
        (-180.0, -90.0 + EPSILON),
        (180.0, 90.0 - EPSILON),
    )
    extent_minor: tuple[tuple[float, float], tuple[float, float]] = (
        (-180.0, -80.0 - EPSILON),
        (180.0, 80.0 + EPSILON),
    )
    step_major: tuple[float, float] = (90.0, 360.0)
    step_minor: tuple[float, float] = (10.0, 10.0)
    precision: float = 2.5


@dataclass(frozen=True)
class Graticule:
    _config: GraticuleConfig = GraticuleConfig()

    def extent(
        self, extent: tuple[tuple[float, float], tuple[float, float]]
    ) -> "Graticule":
        return replace(
            self,
            _config=replace(self._config, extent_major=extent, extent_minor=extent),
        )

    def extent_major(
        self, extent: tuple[tuple[float, float], tuple[float, float]]
    ) -> "Graticule":
        return replace(self, _config=replace(self._config, extent_major=extent))

    def extent_minor(
        self, extent: tuple[tuple[float, float], tuple[float, float]]
    ) -> "Graticule":
        return replace(self, _config=replace(self._config, extent_minor=extent))

    def step(self, step: tuple[float, float]) -> "Graticule":
        return replace(
            self,
            _config=replace(self._config, step_major=step, step_minor=step),
        )

    def step_major(self, step: tuple[float, float]) -> "Graticule":
        return replace(self, _config=replace(self._config, step_major=step))

    def step_minor(self, step: tuple[float, float]) -> "Graticule":
        return replace(self, _config=replace(self._config, step_minor=step))

    def precision(self, precision: float) -> "Graticule":
        return replace(self, _config=replace(self._config, precision=float(precision)))

    def _native(self) -> tuple[list[list[tuple[float, float]]], list[tuple[float, float]]]:
        return _geo_graticule(
            self._config.extent_major,
            self._config.extent_minor,
            self._config.step_major,
            self._config.step_minor,
            self._config.precision,
        )

    def lines(self) -> tuple[tuple[tuple[float, float], ...], ...]:
        return tuple(tuple(tuple(point) for point in line) for line in self._native()[0])

    def outline(self) -> tuple[tuple[float, float], ...]:
        return tuple(tuple(point) for point in self._native()[1])


def graticule10() -> tuple[tuple[tuple[float, float], ...], ...]:
    return Graticule().lines()


@dataclass(frozen=True)
class Rotation:
    lambda_: float = 0.0
    phi: float = 0.0
    gamma: float = 0.0

    def angles(self, lambda_: float, phi: float, gamma: float) -> "Rotation":
        return Rotation(float(lambda_), float(phi), float(gamma))

    def rotate(self, lon: float, lat: float) -> tuple[float, float]:
        return tuple(_geo_rotation((self.lambda_, self.phi, self.gamma), (lon, lat), False))

    def invert(self, lon: float, lat: float) -> tuple[float, float]:
        return tuple(_geo_rotation((self.lambda_, self.phi, self.gamma), (lon, lat), True))


@dataclass(frozen=True)
class Versor:
    w: float = 1.0
    x: float = 0.0
    y: float = 0.0
    z: float = 0.0

    @classmethod
    def from_array(cls, values: Sequence[float]) -> "Versor":
        if len(values) != 4:
            raise ValueError("versor array must contain four values")
        return cls(*(float(value) for value in values))

    def to_array(self) -> tuple[float, float, float, float]:
        return self.w, self.x, self.y, self.z

    @classmethod
    def from_angles(cls, lambda_deg: float, phi_deg: float, gamma_deg: float) -> "Versor":
        return cls(*_geo_versor_from_angles((lambda_deg, phi_deg, gamma_deg)))

    def to_angles(self) -> tuple[float, float, float]:
        return tuple(_geo_versor_to_angles(self.to_array()))

    @classmethod
    def from_cartesian(cls, x: float, y: float, z: float) -> "Versor":
        return cls(*_geo_versor_from_cartesian((x, y, z)))

    @staticmethod
    def spherical_to_cartesian(lon_deg: float, lat_deg: float) -> tuple[float, float, float]:
        return tuple(_geo_spherical_to_cartesian((lon_deg, lat_deg)))

    def multiply(self, other: "Versor") -> "Versor":
        return Versor(*_geo_versor_multiply(self.to_array(), other.to_array()))

    def dot(self, other: "Versor") -> float:
        return float(_geo_versor_dot(self.to_array(), other.to_array()))

    def norm(self) -> float:
        return math.sqrt(self.dot(self))

    def normalize(self) -> "Versor":
        return Versor(*_geo_versor_unary(self.to_array(), "normalize"))

    def conjugate(self) -> "Versor":
        return Versor(*_geo_versor_unary(self.to_array(), "conjugate"))

    @classmethod
    def delta(
        cls, v0: Sequence[float], v1: Sequence[float], alpha: float = 1.0
    ) -> "Versor":
        if len(v0) != 3 or len(v1) != 3:
            raise ValueError("versor delta vectors must contain three values")
        return cls(*_geo_versor_delta(tuple(v0), tuple(v1), alpha))

    def slerp(self, other: "Versor", t: float) -> "Versor":
        return Versor(*_geo_versor_slerp(self.to_array(), other.to_array(), t))

    def rotate_spherical(self, lambda_: float, phi: float) -> tuple[float, float]:
        return tuple(_geo_versor_rotate_spherical(self.to_array(), (lambda_, phi)))

    @staticmethod
    def rotate_degrees(
        rotation_angles: tuple[float, float, float], lon: float, lat: float
    ) -> tuple[float, float]:
        return tuple(_geo_versor_rotate_degrees(rotation_angles, (lon, lat)))


@dataclass(frozen=True)
class Projection:
    _kind: str = field(default="", init=False, repr=False)
    _scale_value: float | None = None
    _translate_value: tuple[float, float] | None = None
    _center_value: tuple[float, float] | None = None
    _rotate_value: tuple[float, float, float] | None = None
    _parallels_value: tuple[float, float] | None = None

    def scale(self, value: float) -> "Projection":
        return replace(self, _scale_value=float(value))

    set_scale = scale

    def translate(self, x: float, y: float) -> "Projection":
        return replace(self, _translate_value=(float(x), float(y)))

    set_translate = translate

    def center(self, lon: float, lat: float) -> "Projection":
        return replace(self, _center_value=(float(lon), float(lat)))

    set_center = center

    def rotate(self, lambda_: float, phi: float, gamma: float) -> "Projection":
        return replace(self, _rotate_value=(float(lambda_), float(phi), float(gamma)))

    set_rotate = rotate

    def _arguments(self) -> dict[str, object]:
        return {
            "scale": self._scale_value,
            "translate": self._translate_value,
            "center": self._center_value,
            "rotate": self._rotate_value,
            "parallels": self._parallels_value,
        }

    def project(self, lon: float, lat: float) -> tuple[float, float]:
        result = _geo_projection_apply(
            self._kind, "project", (lon, lat), **self._arguments()
        )
        if result is None:
            raise ValueError("projection unexpectedly returned no point")
        return tuple(result)

    def project_rotated(self, lambda_: float, phi: float) -> tuple[float, float]:
        result = _geo_projection_apply(
            self._kind, "project_rotated", (lambda_, phi), **self._arguments()
        )
        if result is None:
            raise ValueError("projection unexpectedly returned no point")
        return tuple(result)

    def invert(self, x: float, y: float) -> tuple[float, float] | None:
        result = _geo_projection_apply(
            self._kind, "invert", (x, y), **self._arguments()
        )
        return tuple(result) if result is not None else None

    def _metadata(self):
        return _geo_projection_metadata(self._kind, **self._arguments())

    @property
    def scale_value(self) -> float:
        return float(self._metadata()[0])

    @property
    def translate_value(self) -> tuple[float, float]:
        return tuple(self._metadata()[1])

    @property
    def center_value(self) -> tuple[float, float]:
        return tuple(self._metadata()[2])

    @property
    def rotate_value(self) -> tuple[float, float, float]:
        return tuple(self._metadata()[3])

    def clip_angle(self) -> float | None:
        value = self._metadata()[4]
        return None if value is None else float(value)

    def clip_extent(
        self,
    ) -> tuple[tuple[float, float], tuple[float, float]] | None:
        value = self._metadata()[5]
        return None if value is None else (tuple(value[0]), tuple(value[1]))

    def longitude_unwrap_center(self) -> float | None:
        value = self._metadata()[6]
        return None if value is None else float(value)

    def is_visible(self, lon: float, lat: float) -> bool:
        return bool(
            _geo_projection_visible(
                self._kind, (lon, lat), **self._arguments()
            )
        )


@dataclass(frozen=True)
class Mercator(Projection):
    _kind: str = field(default="mercator", init=False, repr=False)


@dataclass(frozen=True)
class Equirectangular(Projection):
    _kind: str = field(default="equirectangular", init=False, repr=False)


@dataclass(frozen=True)
class Orthographic(Projection):
    _kind: str = field(default="orthographic", init=False, repr=False)


@dataclass(frozen=True)
class Stereographic(Projection):
    _kind: str = field(default="stereographic", init=False, repr=False)


@dataclass(frozen=True)
class TransverseMercator(Projection):
    _kind: str = field(default="transverse_mercator", init=False, repr=False)


@dataclass(frozen=True)
class ConicEqualArea(Projection):
    _kind: str = field(default="conic_equal_area", init=False, repr=False)

    @classmethod
    def with_parallels(cls, phi0: float, phi1: float) -> "ConicEqualArea":
        return cls(_parallels_value=(float(phi0), float(phi1)))

    def parallels(self, phi0: float, phi1: float) -> "ConicEqualArea":
        return replace(self, _parallels_value=(float(phi0), float(phi1)))


@dataclass(frozen=True)
class Albers(Projection):
    _kind: str = field(default="albers", init=False, repr=False)


class GeoJsonKind(str, Enum):
    POINT = "point"
    MULTI_POINT = "multi_point"
    LINE_STRING = "line_string"
    MULTI_LINE_STRING = "multi_line_string"
    POLYGON = "polygon"
    MULTI_POLYGON = "multi_polygon"


def _geo_sequence(value: object, path: str) -> Sequence[object]:
    if isinstance(value, (str, bytes, bytearray)) or not isinstance(value, Sequence):
        raise ValueError(f"{path} must be a sequence")
    return value


def _geo_point_tuple(value: object, path: str) -> tuple[float, float]:
    sequence = _geo_sequence(value, path)
    if len(sequence) != 2:
        raise ValueError(f"{path} must contain longitude and latitude")
    point = (float(sequence[0]), float(sequence[1]))
    if not all(math.isfinite(component) for component in point):
        raise ValueError(f"{path} coordinates must be finite")
    return point


def _geo_line_tuple(value: object, path: str) -> tuple[tuple[float, float], ...]:
    return tuple(
        _geo_point_tuple(point, f"{path}[{index}]")
        for index, point in enumerate(_geo_sequence(value, path))
    )


def _normalize_geo_coordinates(kind: GeoJsonKind, value: object) -> object:
    if kind is GeoJsonKind.POINT:
        return _geo_point_tuple(value, "coordinates")
    if kind in (GeoJsonKind.MULTI_POINT, GeoJsonKind.LINE_STRING):
        return _geo_line_tuple(value, "coordinates")
    if kind in (GeoJsonKind.MULTI_LINE_STRING, GeoJsonKind.POLYGON):
        return tuple(
            _geo_line_tuple(line, f"coordinates[{index}]")
            for index, line in enumerate(_geo_sequence(value, "coordinates"))
        )
    return tuple(
        tuple(
            _geo_line_tuple(ring, f"coordinates[{polygon_index}][{ring_index}]")
            for ring_index, ring in enumerate(
                _geo_sequence(polygon, f"coordinates[{polygon_index}]")
            )
        )
        for polygon_index, polygon in enumerate(_geo_sequence(value, "coordinates"))
    )


@dataclass(frozen=True)
class GeoJsonGeometry:
    kind: GeoJsonKind
    coordinates: object

    def __post_init__(self) -> None:
        kind = GeoJsonKind(self.kind)
        object.__setattr__(self, "kind", kind)
        object.__setattr__(
            self, "coordinates", _normalize_geo_coordinates(kind, self.coordinates)
        )

    @classmethod
    def point(cls, lon: float, lat: float) -> "GeoJsonGeometry":
        return cls(GeoJsonKind.POINT, (lon, lat))

    @classmethod
    def multi_point(cls, points: Sequence[Sequence[float]]) -> "GeoJsonGeometry":
        return cls(GeoJsonKind.MULTI_POINT, points)

    @classmethod
    def line_string(cls, points: Sequence[Sequence[float]]) -> "GeoJsonGeometry":
        return cls(GeoJsonKind.LINE_STRING, points)

    @classmethod
    def multi_line_string(
        cls, lines: Sequence[Sequence[Sequence[float]]]
    ) -> "GeoJsonGeometry":
        return cls(GeoJsonKind.MULTI_LINE_STRING, lines)

    @classmethod
    def polygon(
        cls, rings: Sequence[Sequence[Sequence[float]]]
    ) -> "GeoJsonGeometry":
        return cls(GeoJsonKind.POLYGON, rings)

    @classmethod
    def multi_polygon(
        cls, polygons: Sequence[Sequence[Sequence[Sequence[float]]]]
    ) -> "GeoJsonGeometry":
        return cls(GeoJsonKind.MULTI_POLYGON, polygons)


class GeoStreamEventKind(str, Enum):
    POINT = "point"
    LINE_START = "line_start"
    LINE_END = "line_end"
    POLYGON_START = "polygon_start"
    POLYGON_END = "polygon_end"
    SPHERE = "sphere"


@dataclass(frozen=True)
class GeoStreamEvent:
    kind: GeoStreamEventKind
    x: float | None = None
    y: float | None = None
    marker: int = 0


class GeoStream(Protocol):
    def point(self, x: float, y: float, marker: int) -> None: ...

    def line_start(self) -> None: ...

    def line_end(self) -> None: ...

    def polygon_start(self) -> None: ...

    def polygon_end(self) -> None: ...

    def sphere(self) -> None: ...


def geo_stream_events(geometry: GeoJsonGeometry) -> tuple[GeoStreamEvent, ...]:
    if not isinstance(geometry, GeoJsonGeometry):
        raise TypeError("geometry must be a GeoJsonGeometry")
    kinds = tuple(GeoStreamEventKind)
    return tuple(
        GeoStreamEvent(
            kinds[kind],
            x if kind == 0 else None,
            y if kind == 0 else None,
            marker,
        )
        for kind, x, y, marker in _geo_stream_events(
            geometry.kind.value, geometry.coordinates
        )
    )


def stream_geojson(geometry: GeoJsonGeometry, stream: GeoStream) -> None:
    for event in geo_stream_events(geometry):
        if event.kind is GeoStreamEventKind.POINT:
            stream.point(float(event.x), float(event.y), event.marker)
        elif event.kind is GeoStreamEventKind.LINE_START:
            stream.line_start()
        elif event.kind is GeoStreamEventKind.LINE_END:
            stream.line_end()
        elif event.kind is GeoStreamEventKind.POLYGON_START:
            stream.polygon_start()
        elif event.kind is GeoStreamEventKind.POLYGON_END:
            stream.polygon_end()
        else:
            stream.sphere()


class TopoJsonError(ValueError):
    pass


class TopoJsonInvalidError(TopoJsonError):
    pass


class TopoJsonBudgetError(TopoJsonError):
    pass


class TopoJsonEmptyLandError(TopoJsonError):
    pass


@dataclass(frozen=True)
class TopoJsonBudget:
    max_input_bytes: int = 32 * 1024 * 1024
    max_arcs: int = 1_000_000
    max_arc_points: int = 1_000_000
    max_output_points: int = 10_000_000
    max_geometries: int = 1_000_000

    def __post_init__(self) -> None:
        for name in (
            "max_input_bytes",
            "max_arcs",
            "max_arc_points",
            "max_output_points",
            "max_geometries",
        ):
            value = getattr(self, name)
            if not isinstance(value, int) or isinstance(value, bool) or value < 0:
                raise ValueError(f"{name} must be a non-negative integer")


def parse_land(json: str) -> GeoJsonGeometry | None:
    if not isinstance(json, str):
        raise TypeError("json must be str")
    polygons = _topojson_parse_land(json)
    return None if polygons is None else GeoJsonGeometry.multi_polygon(polygons)


def parse_land_with_budget(
    json: str, budget: TopoJsonBudget = TopoJsonBudget()
) -> GeoJsonGeometry:
    if not isinstance(json, str):
        raise TypeError("json must be str")
    if not isinstance(budget, TopoJsonBudget):
        raise TypeError("budget must be a TopoJsonBudget")
    try:
        polygons = _topojson_parse_land_with_budget(
            json,
            budget.max_input_bytes,
            budget.max_arcs,
            budget.max_arc_points,
            budget.max_output_points,
            budget.max_geometries,
        )
    except ValueError as error:
        message = str(error)
        if "budget exceeded" in message:
            raise TopoJsonBudgetError(message) from error
        if "invalid TopoJSON" in message:
            raise TopoJsonInvalidError(message) from error
        if "land object is empty" in message:
            raise TopoJsonEmptyLandError(message) from error
        raise TopoJsonError(message) from error
    return GeoJsonGeometry.multi_polygon(polygons)


class AutoTypeKind(str, Enum):
    NULL = "null"
    BOOL = "bool"
    INTEGER = "integer"
    FLOAT = "float"
    STRING = "string"
    DATE = "date"


@dataclass(frozen=True)
class AutoTyped:
    kind: AutoTypeKind
    value: bool | int | float | str | None

    def as_f64(self) -> float | None:
        if self.kind in (AutoTypeKind.INTEGER, AutoTypeKind.FLOAT):
            return float(self.value)
        return None

    def as_i64(self) -> int | None:
        if self.kind is AutoTypeKind.INTEGER:
            return int(self.value)
        if self.kind is not AutoTypeKind.FLOAT:
            return None
        value = float(self.value)
        if math.isnan(value):
            return 0
        if not math.isfinite(value):
            return 2**63 - 1 if value > 0 else -(2**63)
        converted = int(value)
        if converted < -(2**63):
            return -(2**63)
        if converted > 2**63 - 1:
            return 2**63 - 1
        return converted

    def as_bool(self) -> bool | None:
        return bool(self.value) if self.kind is AutoTypeKind.BOOL else None

    def as_str(self) -> str | None:
        if self.kind in (AutoTypeKind.STRING, AutoTypeKind.DATE):
            return str(self.value)
        return None

    def is_null(self) -> bool:
        return self.kind is AutoTypeKind.NULL


def _decode_auto_typed(value: tuple[int, bool, int, float, str]) -> AutoTyped:
    kind_index, bool_value, int_value, float_value, string_value = value
    kind = tuple(AutoTypeKind)[kind_index]
    decoded: bool | int | float | str | None
    if kind is AutoTypeKind.NULL:
        decoded = None
    elif kind is AutoTypeKind.BOOL:
        decoded = bool_value
    elif kind is AutoTypeKind.INTEGER:
        decoded = int_value
    elif kind is AutoTypeKind.FLOAT:
        decoded = float_value
    else:
        decoded = string_value
    return AutoTyped(kind, decoded)


def auto_type(value: str) -> AutoTyped:
    if not isinstance(value, str):
        raise TypeError("value must be str")
    return _decode_auto_typed(_fetch_auto_type_values((value,))[0])


def auto_type_row(row: Mapping[str, str]) -> dict[str, AutoTyped]:
    if not isinstance(row, Mapping):
        raise TypeError("row must be a mapping")
    items = tuple(row.items())
    for key, value in items:
        if not isinstance(key, str):
            raise TypeError("row keys must be str")
        if not isinstance(value, str):
            raise TypeError(f"row[{key!r}] must be str")
    typed = _fetch_auto_type_values(tuple(value for _, value in items))
    return {key: _decode_auto_typed(value) for (key, _), value in zip(items, typed)}


def auto_type_rows(rows: Sequence[Mapping[str, str]]) -> list[dict[str, AutoTyped]]:
    prepared: list[tuple[tuple[str, str], ...]] = []
    flat_values: list[str] = []
    for row_index, row in enumerate(rows):
        if not isinstance(row, Mapping):
            raise TypeError(f"rows[{row_index}] must be a mapping")
        items = tuple(row.items())
        for key, value in items:
            if not isinstance(key, str):
                raise TypeError(f"rows[{row_index}] keys must be str")
            if not isinstance(value, str):
                raise TypeError(f"rows[{row_index}][{key!r}] must be str")
            flat_values.append(value)
        prepared.append(items)
    typed = iter(_fetch_auto_type_values(flat_values))
    return [
        {key: _decode_auto_typed(next(typed)) for key, _ in items}
        for items in prepared
    ]


class DsvParseErrorKind(str, Enum):
    UNTERMINATED_QUOTED_FIELD = "unterminated_quoted_field"
    UNEXPECTED_QUOTE = "unexpected_quote"
    INVALID_DELIMITER = "invalid_delimiter"
    HEADER_COLUMN_MISMATCH = "header_column_mismatch"
    EMPTY_HEADER = "empty_header"
    DUPLICATE_HEADER = "duplicate_header"
    BUDGET_EXCEEDED = "budget_exceeded"
    CANCELLED = "cancelled"


class DsvBudgetResource(str, Enum):
    INPUT_BYTES = "input_bytes"
    RECORDS = "records"
    COLUMNS = "columns"
    FIELD_BYTES = "field_bytes"
    CELLS = "cells"


class DsvParseError(ValueError):
    def __init__(
        self,
        message: str,
        *,
        line: int,
        column: int,
        byte_offset: int,
        kind: DsvParseErrorKind,
        resource: DsvBudgetResource | None = None,
        expected: int | None = None,
        actual: int | None = None,
        header_index: int | None = None,
        header_name: str | None = None,
        limit: int | None = None,
    ) -> None:
        super().__init__(f"line {line}, column {column}: {message}")
        self.line = line
        self.column = column
        self.byte_offset = byte_offset
        self.kind = kind
        self.message = message
        self.resource = resource
        self.expected = expected
        self.actual = actual
        self.header_index = header_index
        self.header_name = header_name
        self.limit = limit


class DsvBudgetError(DsvParseError):
    pass


class DsvCancelledError(DsvParseError):
    pass


@dataclass(frozen=True)
class DsvBudget:
    max_input_bytes: int = 16 * 1024 * 1024
    max_records: int = 1_000_000
    max_columns: int = 1_024
    max_field_bytes: int = 4 * 1024 * 1024
    max_cells: int = 10_000_000

    def __post_init__(self) -> None:
        for name in (
            "max_input_bytes",
            "max_records",
            "max_columns",
            "max_field_bytes",
            "max_cells",
        ):
            value = getattr(self, name)
            if not isinstance(value, int) or isinstance(value, bool) or value < 0:
                raise ValueError(f"{name} must be a non-negative integer")

    @classmethod
    def unlimited(cls) -> "DsvBudget":
        maximum = 2**64 - 1
        return cls(maximum, maximum, maximum, maximum, maximum)

    def _native(self) -> tuple[int, int, int, int, int]:
        return (
            self.max_input_bytes,
            self.max_records,
            self.max_columns,
            self.max_field_bytes,
            self.max_cells,
        )


class ColumnPolicy(str, Enum):
    D3_COMPATIBLE = "d3_compatible"
    STRICT = "strict"


@dataclass(frozen=True)
class CsvOptions:
    _skip_empty_lines: bool = field(default=True, repr=False)
    _trim_values: bool = field(default=True, repr=False)

    @classmethod
    def new(cls) -> "CsvOptions":
        return cls()

    def skip_empty_lines(self, skip: bool) -> "CsvOptions":
        return replace(self, _skip_empty_lines=bool(skip))

    def trim_values(self, trim: bool) -> "CsvOptions":
        return replace(self, _trim_values=bool(trim))

    @property
    def skip_empty_lines_value(self) -> bool:
        return self._skip_empty_lines

    @property
    def trim_values_value(self) -> bool:
        return self._trim_values


class DsvCancellationToken:
    def __init__(self) -> None:
        if _NativeDsvCancellationToken is None:
            raise RuntimeError("native extension is not installed")
        self._native_token = _NativeDsvCancellationToken()

    def cancel(self) -> None:
        self._native_token.cancel()

    def reset(self) -> None:
        self._native_token.reset()

    def is_cancelled(self) -> bool:
        return bool(self._native_token.is_cancelled())


def _decode_dsv_error(value: tuple[object, ...]) -> DsvParseError:
    line, column, byte_offset, kind_index, detail, first, second, message = value
    kind = tuple(DsvParseErrorKind)[int(kind_index)]
    arguments: dict[str, object] = {
        "line": int(line),
        "column": int(column),
        "byte_offset": int(byte_offset),
        "kind": kind,
    }
    error_type: type[DsvParseError] = DsvParseError
    if kind is DsvParseErrorKind.HEADER_COLUMN_MISMATCH:
        arguments.update(expected=int(first), actual=int(second))
    elif kind is DsvParseErrorKind.EMPTY_HEADER:
        arguments["header_index"] = int(first)
    elif kind is DsvParseErrorKind.DUPLICATE_HEADER:
        arguments["header_name"] = str(detail)
    elif kind is DsvParseErrorKind.BUDGET_EXCEEDED:
        error_type = DsvBudgetError
        arguments.update(
            resource=DsvBudgetResource(str(detail)),
            limit=int(first),
            actual=int(second),
        )
    elif kind is DsvParseErrorKind.CANCELLED:
        error_type = DsvCancelledError
    return error_type(str(message), **arguments)


@dataclass(frozen=True)
class DsvParser:
    delimiter: str
    _skip_empty_lines: bool = field(default=True, repr=False)
    _trim_values: bool = field(default=True, repr=False)
    _column_policy: ColumnPolicy = field(default=ColumnPolicy.D3_COMPATIBLE, repr=False)
    _budget: DsvBudget = field(default_factory=DsvBudget, repr=False)

    def __post_init__(self) -> None:
        if not isinstance(self.delimiter, str) or len(self.delimiter) != 1:
            raise ValueError("delimiter must be one Unicode character")
        object.__setattr__(self, "_column_policy", ColumnPolicy(self._column_policy))
        if not isinstance(self._budget, DsvBudget):
            raise TypeError("budget must be a DsvBudget")

    @classmethod
    def new(cls, delimiter: str) -> "DsvParser":
        return cls(delimiter)

    def skip_empty_lines(self, skip: bool) -> "DsvParser":
        return replace(self, _skip_empty_lines=bool(skip))

    def trim_values(self, trim: bool) -> "DsvParser":
        return replace(self, _trim_values=bool(trim))

    def column_policy(self, policy: ColumnPolicy) -> "DsvParser":
        return replace(self, _column_policy=ColumnPolicy(policy))

    def budget(self, budget: DsvBudget) -> "DsvParser":
        return replace(self, _budget=budget)

    def _parse(
        self,
        text: str,
        budget: DsvBudget,
        cancellation: DsvCancellationToken | None = None,
    ) -> list[dict[str, str]]:
        if not isinstance(text, str):
            raise TypeError("text must be str")
        if cancellation is not None and not isinstance(cancellation, DsvCancellationToken):
            raise TypeError("cancellation must be a DsvCancellationToken")
        rows, error = _fetch_parse_dsv(
            text,
            self.delimiter,
            self._skip_empty_lines,
            self._trim_values,
            self._column_policy.value,
            budget._native(),
            None if cancellation is None else cancellation._native_token,
        )
        if error is not None:
            raise _decode_dsv_error(tuple(error))
        return [dict(row) for row in rows]

    def parse(self, text: str) -> list[dict[str, str]]:
        return self._parse(text, self._budget)

    def parse_with_budget(self, text: str, budget: DsvBudget) -> list[dict[str, str]]:
        if not isinstance(budget, DsvBudget):
            raise TypeError("budget must be a DsvBudget")
        return self._parse(text, budget)

    def parse_with_budget_and_cancel(
        self, text: str, budget: DsvBudget, cancellation: DsvCancellationToken
    ) -> list[dict[str, str]]:
        if not isinstance(budget, DsvBudget):
            raise TypeError("budget must be a DsvBudget")
        return self._parse(text, budget, cancellation)

    def parse_lossy(self, text: str) -> list[dict[str, str]]:
        try:
            return self.parse(text)
        except DsvParseError:
            return []

    try_parse = parse

    def parse_rows(self, text: str) -> list[list[str]]:
        if not isinstance(text, str):
            raise TypeError("text must be str")
        rows, error = _fetch_parse_dsv_rows(
            text,
            self.delimiter,
            self._skip_empty_lines,
            self._trim_values,
            self._column_policy.value,
            self._budget._native(),
        )
        if error is not None:
            raise _decode_dsv_error(tuple(error))
        return [list(row) for row in rows]

    def parse_rows_lossy(self, text: str) -> list[list[str]]:
        try:
            return self.parse_rows(text)
        except DsvParseError:
            return []

    try_parse_rows = parse_rows

    def format(self, rows: Sequence[Mapping[str, str]], columns: Sequence[str]) -> str:
        normalized: list[dict[str, str]] = []
        for row_index, row in enumerate(rows):
            if not isinstance(row, Mapping):
                raise TypeError(f"rows[{row_index}] must be a mapping")
            normalized_row: dict[str, str] = {}
            for key, value in row.items():
                if not isinstance(key, str) or not isinstance(value, str):
                    raise TypeError(f"rows[{row_index}] keys and values must be str")
                normalized_row[key] = value
            normalized.append(normalized_row)
        normalized_columns = tuple(columns)
        if not all(isinstance(column, str) for column in normalized_columns):
            raise TypeError("columns must contain only str values")
        return str(_fetch_format_dsv(normalized, normalized_columns, self.delimiter))


def parse_dsv(text: str, delimiter: str) -> list[dict[str, str]]:
    return DsvParser(delimiter).parse(text)


def parse_dsv_with_budget(
    text: str, delimiter: str, budget: DsvBudget
) -> list[dict[str, str]]:
    return DsvParser(delimiter).parse_with_budget(text, budget)


def parse_dsv_lossy(text: str, delimiter: str) -> list[dict[str, str]]:
    return DsvParser(delimiter).parse_lossy(text)


try_parse_dsv = parse_dsv


def parse_csv(text: str) -> list[dict[str, str]]:
    return DsvParser(",").parse(text)


def parse_csv_with_budget(text: str, budget: DsvBudget) -> list[dict[str, str]]:
    return DsvParser(",").parse_with_budget(text, budget)


def parse_csv_with_budget_and_cancel(
    text: str, budget: DsvBudget, cancellation: DsvCancellationToken
) -> list[dict[str, str]]:
    return DsvParser(",").parse_with_budget_and_cancel(text, budget, cancellation)


def parse_csv_lossy(text: str) -> list[dict[str, str]]:
    return DsvParser(",").parse_lossy(text)


try_parse_csv = parse_csv


def parse_csv_with_options(text: str, options: CsvOptions) -> list[dict[str, str]]:
    if not isinstance(options, CsvOptions):
        raise TypeError("options must be CsvOptions")
    return (
        DsvParser(",")
        .skip_empty_lines(options.skip_empty_lines_value)
        .trim_values(options.trim_values_value)
        .parse(text)
    )


def parse_csv_lossy_with_options(text: str, options: CsvOptions) -> list[dict[str, str]]:
    try:
        return parse_csv_with_options(text, options)
    except DsvParseError:
        return []


try_parse_csv_with_options = parse_csv_with_options


def parse_tsv(text: str) -> list[dict[str, str]]:
    return DsvParser("\t").parse(text)


def parse_tsv_with_budget(text: str, budget: DsvBudget) -> list[dict[str, str]]:
    return DsvParser("\t").parse_with_budget(text, budget)


def parse_tsv_with_budget_and_cancel(
    text: str, budget: DsvBudget, cancellation: DsvCancellationToken
) -> list[dict[str, str]]:
    return DsvParser("\t").parse_with_budget_and_cancel(text, budget, cancellation)


def parse_tsv_lossy(text: str) -> list[dict[str, str]]:
    return DsvParser("\t").parse_lossy(text)


try_parse_tsv = parse_tsv


def parse_tsv_with_options(text: str, options: CsvOptions) -> list[dict[str, str]]:
    if not isinstance(options, CsvOptions):
        raise TypeError("options must be CsvOptions")
    return (
        DsvParser("\t")
        .skip_empty_lines(options.skip_empty_lines_value)
        .trim_values(options.trim_values_value)
        .parse(text)
    )


def parse_tsv_lossy_with_options(text: str, options: CsvOptions) -> list[dict[str, str]]:
    try:
        return parse_tsv_with_options(text, options)
    except DsvParseError:
        return []


try_parse_tsv_with_options = parse_tsv_with_options


def format_csv(rows: Sequence[Mapping[str, str]], columns: Sequence[str]) -> str:
    return DsvParser(",").format(rows, columns)


def format_tsv(rows: Sequence[Mapping[str, str]], columns: Sequence[str]) -> str:
    return DsvParser("\t").format(rows, columns)


MAX_TILE_ZOOM: Final[int] = 30
MAX_VISIBLE_TILES: Final[int] = 1_000_000


@dataclass(frozen=True)
class HexbinBin:
    x: float
    y: float
    points: tuple[object, ...]

    def len(self) -> int:
        return builtins.len(self.points)

    def __len__(self) -> int:
        return builtins.len(self.points)

    def is_empty(self) -> bool:
        return not self.points


class HexbinErrorKind(str, Enum):
    NON_FINITE_RADIUS = "non_finite_radius"
    NON_POSITIVE_RADIUS = "non_positive_radius"
    NON_FINITE_EXTENT_COORDINATE = "non_finite_extent_coordinate"
    REVERSED_EXTENT = "reversed_extent"
    NON_FINITE_POINT_COORDINATE = "non_finite_point_coordinate"


class HexbinError(ValueError):
    def __init__(
        self,
        kind: HexbinErrorKind,
        message: str,
        *,
        index: int | None = None,
        coordinate: str | None = None,
        corner: str | None = None,
        axis: str | None = None,
        value: float | None = None,
        minimum: float | None = None,
        maximum: float | None = None,
    ) -> None:
        super().__init__(message)
        self.kind = kind
        self.index = index
        self.coordinate = coordinate
        self.corner = corner
        self.axis = axis
        self.value = value
        self.minimum = minimum
        self.maximum = maximum


def _decode_hexbin_error(value: tuple[object, ...]) -> HexbinError:
    kind_index, index, first, second, number, other, message = value
    kind = tuple(HexbinErrorKind)[int(kind_index)]
    arguments: dict[str, object] = {}
    if kind in (HexbinErrorKind.NON_FINITE_RADIUS, HexbinErrorKind.NON_POSITIVE_RADIUS):
        arguments["value"] = float(number)
    elif kind is HexbinErrorKind.NON_FINITE_EXTENT_COORDINATE:
        arguments.update(corner=str(first), coordinate=str(second), value=float(number))
    elif kind is HexbinErrorKind.REVERSED_EXTENT:
        arguments.update(axis=str(first), minimum=float(number), maximum=float(other))
    else:
        arguments.update(index=int(index), coordinate=str(first), value=float(number))
    return HexbinError(kind, str(message), **arguments)


def _hexbin_default_x(value: object) -> float:
    try:
        return float(value[0])
    except (IndexError, KeyError, TypeError, ValueError):
        return math.nan


def _hexbin_default_y(value: object) -> float:
    try:
        return float(value[1])
    except (IndexError, KeyError, TypeError, ValueError):
        return math.nan


@dataclass(frozen=True)
class Hexbin:
    _x: Callable[[object], float] = field(default=_hexbin_default_x, repr=False)
    _y: Callable[[object], float] = field(default=_hexbin_default_y, repr=False)
    _radius: float = field(default=1.0, repr=False)
    _extent: tuple[tuple[float, float], tuple[float, float]] = field(
        default=((0.0, 0.0), (1.0, 1.0)), repr=False
    )

    @classmethod
    def new(cls) -> "Hexbin":
        return cls()

    @classmethod
    def with_accessors(
        cls, x: Callable[[object], float], y: Callable[[object], float]
    ) -> "Hexbin":
        if not callable(x) or not callable(y):
            raise TypeError("x and y must be callable")
        return cls(x, y)

    def x(self, accessor: Callable[[object], float]) -> "Hexbin":
        if not callable(accessor):
            raise TypeError("x accessor must be callable")
        return replace(self, _x=accessor)

    def y(self, accessor: Callable[[object], float]) -> "Hexbin":
        if not callable(accessor):
            raise TypeError("y accessor must be callable")
        return replace(self, _y=accessor)

    def radius(self, radius: float) -> "Hexbin":
        return replace(self, _radius=float(radius))

    def extent(self, x0: float, y0: float, x1: float, y1: float) -> "Hexbin":
        return replace(
            self,
            _extent=((float(x0), float(y0)), (float(x1), float(y1))),
        )

    def _native_bins(self, data: Sequence[object], *, checked: bool) -> tuple[HexbinBin, ...]:
        values = tuple(data)
        points: list[tuple[float, float, int]] = []
        for index, value in enumerate(values):
            point_x = float(self._x(value))
            point_y = float(self._y(value))
            if not math.isfinite(point_x) or not math.isfinite(point_y):
                if checked:
                    coordinate = "x" if not math.isfinite(point_x) else "y"
                    number = point_x if coordinate == "x" else point_y
                    raise HexbinError(
                        HexbinErrorKind.NON_FINITE_POINT_COORDINATE,
                        f"hexbin point at index {index} has non-finite {coordinate}: {number}",
                        index=index,
                        coordinate=coordinate,
                        value=number,
                    )
                continue
            points.append((point_x, point_y, index))
        result, error = _hexbin_bin(points, self._radius, self._extent)
        if error is not None:
            raise _decode_hexbin_error(tuple(error))
        if result is None:
            raise RuntimeError("native hexbin returned neither bins nor an error")
        return tuple(
            HexbinBin(float(x), float(y), tuple(values[int(index)] for index in indices))
            for x, y, indices in result
        )

    def bin(self, data: Sequence[object]) -> tuple[HexbinBin, ...]:
        try:
            return self._native_bins(data, checked=False)
        except HexbinError:
            return ()

    def try_bin(self, data: Sequence[object]) -> tuple[HexbinBin, ...]:
        return self._native_bins(data, checked=True)

    def _hexagon(self, radius: float | None) -> str:
        result, error = _hexbin_hexagon(self._radius, self._extent, radius)
        if error is not None:
            raise _decode_hexbin_error(tuple(error))
        if result is None:
            raise RuntimeError("native hexbin returned neither a path nor an error")
        return str(result)

    def hexagon(self) -> str:
        try:
            return self.try_hexagon()
        except HexbinError:
            return ""

    def hexagon_with_radius(self, radius: float) -> str:
        try:
            return self.try_hexagon_with_radius(radius)
        except HexbinError:
            return ""

    def try_hexagon(self) -> str:
        return self._hexagon(None)

    def try_hexagon_with_radius(self, radius: float) -> str:
        return self._hexagon(float(radius))

    def centers(self) -> tuple[tuple[float, float], ...]:
        try:
            return self.try_centers()
        except HexbinError:
            return ()

    def try_centers(self) -> tuple[tuple[float, float], ...]:
        result, error = _hexbin_centers(self._radius, self._extent)
        if error is not None:
            raise _decode_hexbin_error(tuple(error))
        if result is None:
            raise RuntimeError("native hexbin returned neither centers nor an error")
        return tuple(tuple(point) for point in result)


@dataclass(frozen=True)
class SankeyNode:
    id: str
    index: int
    x0: float
    x1: float
    y0: float
    y1: float
    value: float
    depth: int
    height: int
    layer: int


@dataclass(frozen=True)
class SankeyLink:
    source: int
    target: int
    value: float
    y0: float
    y1: float
    width: float
    path: str


@dataclass(frozen=True)
class SankeyLinkInput:
    source: str
    target: str
    value: float


@dataclass(frozen=True)
class SankeyResult:
    nodes: tuple[SankeyNode, ...]
    links: tuple[SankeyLink, ...]


class SankeyNodeAlign(str, Enum):
    LEFT = "left"
    RIGHT = "right"
    CENTER = "center"
    JUSTIFY = "justify"


class SankeyLinkSort(str, Enum):
    DEFAULT = "default"
    INPUT_ORDER = "input_order"


@dataclass(frozen=True)
class SankeyLinkSortContext:
    index: int
    source: int
    target: int
    value: float
    source_layer: int
    target_layer: int
    source_y0: float
    target_y0: float


class SankeyLayoutErrorKind(str, Enum):
    NON_FINITE_CONFIG_FIELD = "non_finite_config_field"
    NON_POSITIVE_CONFIG_FIELD = "non_positive_config_field"
    NEGATIVE_CONFIG_FIELD = "negative_config_field"
    INVALID_DRAWABLE_AREA = "invalid_drawable_area"
    EMPTY_NODE_NAME = "empty_node_name"
    DUPLICATE_NODE_NAME = "duplicate_node_name"
    UNKNOWN_LINK_ENDPOINT = "unknown_link_endpoint"
    NON_FINITE_LINK_VALUE = "non_finite_link_value"
    NEGATIVE_LINK_VALUE = "negative_link_value"


class SankeyLayoutError(ValueError):
    def __init__(
        self,
        kind: SankeyLayoutErrorKind,
        message: str,
        *,
        field: str | None = None,
        axis: str | None = None,
        node_name: str | None = None,
        endpoint: str | None = None,
        index: int | None = None,
        first_index: int | None = None,
        duplicate_index: int | None = None,
        value: float | None = None,
    ) -> None:
        super().__init__(message)
        self.kind = kind
        self.field = field
        self.axis = axis
        self.node_name = node_name
        self.endpoint = endpoint
        self.index = index
        self.first_index = first_index
        self.duplicate_index = duplicate_index
        self.value = value


def _decode_sankey_error(value: tuple[object, ...]) -> SankeyLayoutError:
    kind_index, first, second, index, other_index, number, message = value
    kind = tuple(SankeyLayoutErrorKind)[int(kind_index)]
    arguments: dict[str, object] = {}
    if kind in (
        SankeyLayoutErrorKind.NON_FINITE_CONFIG_FIELD,
        SankeyLayoutErrorKind.NON_POSITIVE_CONFIG_FIELD,
        SankeyLayoutErrorKind.NEGATIVE_CONFIG_FIELD,
    ):
        arguments.update(field=str(first), value=float(number))
    elif kind is SankeyLayoutErrorKind.INVALID_DRAWABLE_AREA:
        arguments.update(axis=str(first), value=float(number))
    elif kind is SankeyLayoutErrorKind.EMPTY_NODE_NAME:
        arguments["index"] = int(index)
    elif kind is SankeyLayoutErrorKind.DUPLICATE_NODE_NAME:
        arguments.update(
            node_name=str(first),
            first_index=int(index),
            duplicate_index=int(other_index),
        )
    elif kind is SankeyLayoutErrorKind.UNKNOWN_LINK_ENDPOINT:
        arguments.update(endpoint=str(first), node_name=str(second), index=int(index))
    else:
        arguments.update(index=int(index), value=float(number))
    return SankeyLayoutError(kind, str(message), **arguments)


@dataclass(frozen=True)
class SankeyLayout:
    _width: float = field(default=928.0, repr=False)
    _height: float = field(default=600.0, repr=False)
    _margins: tuple[float, float, float, float] = field(
        default=(5.0, 1.0, 5.0, 1.0), repr=False
    )
    _extent: tuple[tuple[float, float], tuple[float, float]] | None = field(
        default=None, repr=False
    )
    _node_width: float = field(default=15.0, repr=False)
    _node_padding: float = field(default=10.0, repr=False)
    _iterations: int = field(default=6, repr=False)
    _node_align: SankeyNodeAlign = field(default=SankeyNodeAlign.JUSTIFY, repr=False)
    _link_sort: SankeyLinkSort = field(default=SankeyLinkSort.DEFAULT, repr=False)

    @classmethod
    def new(cls) -> "SankeyLayout":
        return cls()

    def width(self, width: float) -> "SankeyLayout":
        return replace(self, _width=float(width))

    def height(self, height: float) -> "SankeyLayout":
        return replace(self, _height=float(height))

    def margins(
        self, top: float, right: float, bottom: float, left: float
    ) -> "SankeyLayout":
        return replace(self, _margins=(float(top), float(right), float(bottom), float(left)))

    def extent(self, x0: float, y0: float, x1: float, y1: float) -> "SankeyLayout":
        return replace(self, _extent=((float(x0), float(y0)), (float(x1), float(y1))))

    def node_width(self, width: float) -> "SankeyLayout":
        return replace(self, _node_width=float(width))

    def node_padding(self, padding: float) -> "SankeyLayout":
        return replace(self, _node_padding=float(padding))

    def iterations(self, iterations: int) -> "SankeyLayout":
        if not isinstance(iterations, int) or isinstance(iterations, bool) or iterations < 0:
            raise ValueError("iterations must be a non-negative integer")
        return replace(self, _iterations=iterations)

    def node_align(self, align: SankeyNodeAlign) -> "SankeyLayout":
        return replace(self, _node_align=SankeyNodeAlign(align))

    def link_sort(self, strategy: SankeyLinkSort) -> "SankeyLayout":
        return replace(self, _link_sort=SankeyLinkSort(strategy))

    def link_sort_input_order(self) -> "SankeyLayout":
        return self.link_sort(SankeyLinkSort.INPUT_ORDER)

    def try_compute(
        self, node_names: Sequence[str], links: Sequence[SankeyLinkInput]
    ) -> SankeyResult:
        names = tuple(node_names)
        if not all(isinstance(name, str) for name in names):
            raise TypeError("node_names must contain only str values")
        native_links: list[tuple[str, str, float]] = []
        for index, link in enumerate(links):
            if not isinstance(link, SankeyLinkInput):
                raise TypeError(f"links[{index}] must be a SankeyLinkInput")
            native_links.append((link.source, link.target, float(link.value)))
        result, error = _sankey_layout(
            names,
            native_links,
            self._width,
            self._height,
            self._margins,
            self._extent,
            self._node_width,
            self._node_padding,
            self._iterations,
            self._node_align.value,
            self._link_sort is SankeyLinkSort.INPUT_ORDER,
        )
        if error is not None:
            raise _decode_sankey_error(tuple(error))
        if result is None:
            raise RuntimeError("native Sankey layout returned neither data nor an error")
        nodes, computed_links = result
        return SankeyResult(
            tuple(SankeyNode(*node) for node in nodes),
            tuple(SankeyLink(*link) for link in computed_links),
        )

    def compute(
        self, node_names: Sequence[str], links: Sequence[SankeyLinkInput]
    ) -> SankeyResult:
        return self.try_compute(node_names, links)


@dataclass(frozen=True, order=True)
class Tile:
    x: int
    y: int
    z: int


@dataclass(frozen=True)
class BrushSelection:
    x0: float
    y0: float
    x1: float
    y1: float

    @classmethod
    def new(cls, x0: float, y0: float, x1: float, y1: float) -> "BrushSelection":
        x0, y0, x1, y1 = float(x0), float(y0), float(x1), float(y1)
        if not all(math.isfinite(value) for value in (x0, y0, x1, y1)):
            raise ValueError("brush selection coordinates must be finite")
        return cls(
            x0 if x0 <= x1 else x1,
            y0 if y0 <= y1 else y1,
            x1 if x0 <= x1 else x0,
            y1 if y0 <= y1 else y0,
        )

    def width(self) -> float:
        return self.x1 - self.x0

    def height(self) -> float:
        return self.y1 - self.y0

    def is_trivial(self, min_size: float) -> bool:
        return self.width() < min_size or self.height() < min_size

    def to_domain(
        self, x_scale: "AxisScale", y_scale: "AxisScale"
    ) -> "DomainSelection":
        values = _brush_to_domain(
            (self.x0, self.y0, self.x1, self.y1),
            x_scale.kind.value,
            x_scale.domain,
            x_scale.range,
            x_scale.parameter,
            x_scale.clamped,
            x_scale.nice_count,
            y_scale.kind.value,
            y_scale.domain,
            y_scale.range,
            y_scale.parameter,
            y_scale.clamped,
            y_scale.nice_count,
        )
        return DomainSelection(*values)


@dataclass(frozen=True)
class DomainSelection:
    x0: float
    y0: float
    x1: float
    y1: float

    @classmethod
    def new(cls, x0: float, y0: float, x1: float, y1: float) -> "DomainSelection":
        x0, y0, x1, y1 = float(x0), float(y0), float(x1), float(y1)
        return cls(
            x0 if x0 <= x1 else x1,
            y0 if y0 <= y1 else y1,
            x1 if x0 <= x1 else x0,
            y1 if y0 <= y1 else y0,
        )


class BrushState:
    def __init__(self) -> None:
        if _NativeBrushState is None:
            raise RuntimeError("native extension is not installed")
        self._native = _NativeBrushState()

    @classmethod
    def new(cls) -> "BrushState":
        return cls()

    def start(self, x: float, y: float) -> None:
        self._native.start(float(x), float(y))

    def update(self, x: float, y: float) -> None:
        self._native.update(float(x), float(y))

    def end(self) -> BrushSelection | None:
        value = self._native.end()
        return None if value is None else BrushSelection(*value)

    def reset(self) -> None:
        self._native.reset()

    def is_active(self) -> bool:
        return bool(self._native.is_active())

    def current_selection(self) -> BrushSelection | None:
        value = self._native.current_selection()
        return None if value is None else BrushSelection(*value)


@dataclass(frozen=True)
class BrushConfig:
    fill_color: tuple[int, int, int, int] = (100, 150, 200, 80)
    stroke_color: tuple[int, int, int] = (70, 130, 180)
    stroke_width: float = 1.0
    min_size: float = 5.0


@dataclass(frozen=True)
class ZoomConfig:
    zoom_x: bool = True
    zoom_y: bool = True
    min_extent: float = 0.001
    max_extent: float = 100.0


class ZoomState:
    def __init__(
        self,
        x_min: float = 0.0,
        x_max: float = 1.0,
        y_min: float = 0.0,
        y_max: float = 1.0,
        *,
        log_x: bool = False,
        log_y: bool = False,
    ) -> None:
        if _NativeZoomState is None:
            raise RuntimeError("native extension is not installed")
        self._validate_domains(x_min, x_max, y_min, y_max)
        if log_x and x_min <= 0.0 or log_y and y_min <= 0.0:
            raise ValueError("log zoom domains must be positive")
        self._native = _NativeZoomState(x_min, x_max, y_min, y_max, log_x, log_y)

    @staticmethod
    def _validate_domains(x_min: float, x_max: float, y_min: float, y_max: float) -> None:
        if not all(math.isfinite(value) for value in (x_min, x_max, y_min, y_max)):
            raise ValueError("zoom domains must be finite")
        if x_min >= x_max or y_min >= y_max:
            raise ValueError("zoom domains must be strictly increasing")

    @classmethod
    def new(cls, x_min: float, x_max: float, y_min: float, y_max: float) -> "ZoomState":
        return cls(x_min, x_max, y_min, y_max)

    def _copy_with_native(self, native_state: object) -> "ZoomState":
        value = object.__new__(ZoomState)
        value._native = native_state
        return value

    def with_log_x(self, is_log: bool) -> "ZoomState":
        if not isinstance(is_log, bool):
            raise TypeError("is_log must be bool")
        if is_log and self.original_x_domain()[0] <= 0.0:
            raise ValueError("log zoom domains must be positive")
        return self._copy_with_native(self._native.with_log_x(is_log))

    def with_log_y(self, is_log: bool) -> "ZoomState":
        if not isinstance(is_log, bool):
            raise TypeError("is_log must be bool")
        if is_log and self.original_y_domain()[0] <= 0.0:
            raise ValueError("log zoom domains must be positive")
        return self._copy_with_native(self._native.with_log_y(is_log))

    def zoom_to(self, x_min: float, x_max: float, y_min: float, y_max: float) -> None:
        self._validate_domains(x_min, x_max, y_min, y_max)
        self._native.zoom_to(x_min, x_max, y_min, y_max)

    def set_viewport(self, x_min: float, x_max: float, y_min: float, y_max: float) -> None:
        self._validate_domains(x_min, x_max, y_min, y_max)
        self._native.set_viewport(x_min, x_max, y_min, y_max)

    def reset(self) -> None:
        self._native.reset()

    def zoom_back(self) -> bool:
        return bool(self._native.zoom_back())

    def is_zoomed(self) -> bool:
        return bool(self._native.is_zoomed())

    def x_domain(self) -> tuple[float, float]:
        return tuple(self._native.x_domain())

    def y_domain(self) -> tuple[float, float]:
        return tuple(self._native.y_domain())

    def original_x_domain(self) -> tuple[float, float]:
        return tuple(self._native.original_x_domain())

    def original_y_domain(self) -> tuple[float, float]:
        return tuple(self._native.original_y_domain())

    def zoom_level(self) -> int:
        return int(self._native.zoom_level())

    def set_original(self, x_min: float, x_max: float, y_min: float, y_max: float) -> None:
        self._validate_domains(x_min, x_max, y_min, y_max)
        self._native.set_original(x_min, x_max, y_min, y_max)


class DragErrorKind(str, Enum):
    NON_FINITE_COORDINATE = "non_finite_coordinate"
    INVALID_EXTENT = "invalid_extent"
    INVALID_CLICK_DISTANCE = "invalid_click_distance"
    ALREADY_ACTIVE = "already_active"
    INACTIVE = "inactive"
    POINTER_MISMATCH = "pointer_mismatch"


class DragError(ValueError):
    def __init__(
        self,
        kind: DragErrorKind,
        message: str,
        *,
        axis: str | None = None,
        value: float | None = None,
        reason: str | None = None,
        active: int | None = None,
        received: int | None = None,
    ) -> None:
        super().__init__(message)
        self.kind = kind
        self.axis = axis
        self.value = value
        self.reason = reason
        self.active = active
        self.received = received


@dataclass(frozen=True)
class DragPoint:
    x: float
    y: float

    @classmethod
    def try_new(cls, x: float, y: float) -> "DragPoint":
        x = float(x)
        y = float(y)
        if not math.isfinite(x):
            raise DragError(
                DragErrorKind.NON_FINITE_COORDINATE,
                f"non-finite x coordinate: {x}",
                axis="x",
                value=x,
            )
        if not math.isfinite(y):
            raise DragError(
                DragErrorKind.NON_FINITE_COORDINATE,
                f"non-finite y coordinate: {y}",
                axis="y",
                value=y,
            )
        return cls(x, y)

    def delta_from(self, other: "DragPoint") -> "DragDelta":
        return DragDelta(self.x - other.x, self.y - other.y)


@dataclass(frozen=True)
class DragDelta:
    dx: float
    dy: float

    def length_squared(self) -> float:
        return self.dx * self.dx + self.dy * self.dy

    def length(self) -> float:
        return math.sqrt(self.length_squared())


@dataclass(frozen=True)
class DragExtent:
    x0: float
    y0: float
    x1: float
    y1: float

    @classmethod
    def try_new(cls, x0: float, y0: float, x1: float, y1: float) -> "DragExtent":
        minimum = DragPoint.try_new(x0, y0)
        maximum = DragPoint.try_new(x1, y1)
        if minimum.x > maximum.x:
            raise DragError(
                DragErrorKind.INVALID_EXTENT,
                "x0 must be <= x1",
                reason="x0 must be <= x1",
            )
        if minimum.y > maximum.y:
            raise DragError(
                DragErrorKind.INVALID_EXTENT,
                "y0 must be <= y1",
                reason="y0 must be <= y1",
            )
        return cls(minimum.x, minimum.y, maximum.x, maximum.y)

    def clamp(self, point: DragPoint) -> DragPoint:
        return DragPoint(
            self.x0 if point.x < self.x0 else self.x1 if point.x > self.x1 else point.x,
            self.y0 if point.y < self.y0 else self.y1 if point.y > self.y1 else point.y,
        )


@dataclass(frozen=True)
class DragConfig:
    click_distance: float = 0.0
    extent: DragExtent | None = None

    def validate(self) -> "DragConfig":
        if not math.isfinite(self.click_distance) or self.click_distance < 0.0:
            raise DragError(
                DragErrorKind.INVALID_CLICK_DISTANCE,
                f"invalid click distance: {self.click_distance}",
                value=self.click_distance,
            )
        return self

    def with_click_distance(self, click_distance: float) -> "DragConfig":
        return replace(self, click_distance=float(click_distance)).validate()

    def with_extent(self, extent: DragExtent) -> "DragConfig":
        if not isinstance(extent, DragExtent):
            raise TypeError("extent must be a DragExtent")
        return replace(self, extent=extent)


class DragPhase(str, Enum):
    START = "start"
    DRAG = "drag"
    END = "end"
    CANCEL = "cancel"


@dataclass(frozen=True)
class DragUpdate:
    phase: DragPhase
    pointer_id: int
    start: DragPoint
    previous: DragPoint
    current: DragPoint
    delta: DragDelta
    total_delta: DragDelta
    distance: float
    exceeds_click_distance: bool


def _drag_update(value: tuple[object, ...]) -> DragUpdate:
    return DragUpdate(
        DragPhase(value[0]),
        int(value[1]),
        DragPoint(*value[2]),
        DragPoint(*value[3]),
        DragPoint(*value[4]),
        DragDelta(*value[5]),
        DragDelta(*value[6]),
        float(value[7]),
        bool(value[8]),
    )


def _drag_result(result: object, error: object) -> DragUpdate:
    if error is not None:
        index, axis, value, reason, active, received, message = error
        kind = tuple(DragErrorKind)[int(index)]
        raise DragError(
            kind,
            str(message),
            axis=str(axis) or None,
            value=float(value)
            if kind in (
                DragErrorKind.NON_FINITE_COORDINATE,
                DragErrorKind.INVALID_CLICK_DISTANCE,
            )
            else None,
            reason=str(reason) or None,
            active=int(active)
            if kind in (DragErrorKind.ALREADY_ACTIVE, DragErrorKind.POINTER_MISMATCH)
            else None,
            received=int(received) if kind is DragErrorKind.POINTER_MISMATCH else None,
        )
    if result is None:
        raise RuntimeError("native drag state returned neither an update nor an error")
    return _drag_update(result)


class DragState:
    def __init__(self, config: DragConfig | None = None) -> None:
        if _NativeDragState is None:
            raise RuntimeError("native extension is not installed")
        config = (config or DragConfig()).validate()
        extent = (
            None
            if config.extent is None
            else (config.extent.x0, config.extent.y0, config.extent.x1, config.extent.y1)
        )
        self._native = _NativeDragState(config.click_distance, extent)

    @classmethod
    def new(cls) -> "DragState":
        return cls()

    @classmethod
    def with_config(cls, config: DragConfig) -> "DragState":
        return cls(config)

    def config(self) -> DragConfig:
        click_distance, extent = self._native.config()
        return DragConfig(
            float(click_distance),
            None if extent is None else DragExtent(*extent),
        )

    def start(self, pointer_id: int, x: float, y: float) -> DragUpdate:
        return _drag_result(*self._native.start(pointer_id, x, y))

    def drag(self, pointer_id: int, x: float, y: float) -> DragUpdate:
        return _drag_result(*self._native.drag(pointer_id, x, y))

    def end(self, pointer_id: int, x: float, y: float) -> DragUpdate:
        return _drag_result(*self._native.end(pointer_id, x, y))

    def cancel(self, pointer_id: int) -> DragUpdate:
        return _drag_result(*self._native.cancel(pointer_id))

    def is_active(self) -> bool:
        return bool(self._native.is_active())

    def active_pointer_id(self) -> int | None:
        value = self._native.active_pointer_id()
        return None if value is None else int(value)

    def current_update(self) -> DragUpdate | None:
        value = self._native.current_update()
        return None if value is None else _drag_update(value)


class TimerState(str, Enum):
    ACTIVE = "active"
    STOPPED = "stopped"


class TimerCallbackError(RuntimeError):
    pass


class _TimerHandleBase:
    def __init__(self, resource: object) -> None:
        self._resource = resource
        self._closed = False

    def _require_open(self) -> object:
        if self._closed:
            raise RuntimeError("timer resource is closed")
        return self._resource

    def _raise_callback_error(self) -> None:
        error = self._require_open().take_callback_error()
        if error is not None:
            raise TimerCallbackError(error)

    def stop(self) -> None:
        if not self._closed:
            self._resource.stop()

    def is_stopped(self) -> bool:
        stopped = bool(self._require_open().is_stopped())
        self._raise_callback_error()
        return stopped

    def state(self) -> TimerState:
        return TimerState.STOPPED if self.is_stopped() else TimerState.ACTIVE

    def join(self) -> None:
        self._require_open().join()
        self._raise_callback_error()

    def try_join(self, timeout_ms: float) -> bool:
        completed = bool(self._require_open().try_join(float(timeout_ms)))
        if completed:
            self._raise_callback_error()
        return completed

    def close(self) -> None:
        if not self._closed:
            self._resource.stop()
            self._closed = True
            self._resource = None

    def __enter__(self) -> "_TimerHandleBase":
        self._require_open()
        return self

    def __exit__(self, *_args: object) -> None:
        self.close()


class Timer(_TimerHandleBase):
    def __init__(
        self,
        callback: Callable[[float], bool],
        delay: float | None = None,
        time: float | None = None,
    ) -> None:
        if _NativeTimerResource is None:
            raise RuntimeError("native extension is not installed")
        if not callable(callback):
            raise TypeError("callback must be callable")
        super().__init__(_NativeTimerResource("timer", callback, delay, time, None))

    @classmethod
    def new(
        cls,
        callback: Callable[[float], bool],
        delay: float | None = None,
        time: float | None = None,
    ) -> "Timer":
        return cls(callback, delay, time)

    def restart(
        self,
        callback: Callable[[float], bool],
        delay: float | None = None,
        time: float | None = None,
    ) -> None:
        if not callable(callback):
            raise TypeError("callback must be callable")
        self._require_open().restart(callback, delay, time)

    def id(self) -> int:
        return int(self._require_open().id())

    def delay(self) -> float:
        return float(self._require_open().delay())

    def start_time(self) -> float:
        return float(self._require_open().start_time())


class Interval(_TimerHandleBase):
    def __init__(
        self,
        callback: Callable[[float], bool],
        interval_ms: float,
        time: float | None = None,
    ) -> None:
        if _NativeTimerResource is None:
            raise RuntimeError("native extension is not installed")
        if not callable(callback):
            raise TypeError("callback must be callable")
        super().__init__(
            _NativeTimerResource("interval", callback, None, time, float(interval_ms))
        )

    @classmethod
    def new(
        cls,
        callback: Callable[[float], bool],
        interval_ms: float,
        time: float | None = None,
    ) -> "Interval":
        return cls(callback, interval_ms, time)


class Timeout(_TimerHandleBase):
    def __init__(
        self,
        callback: Callable[[float], object],
        delay: float,
        time: float | None = None,
    ) -> None:
        if _NativeTimerResource is None:
            raise RuntimeError("native extension is not installed")
        if not callable(callback):
            raise TypeError("callback must be callable")
        super().__init__(_NativeTimerResource("timeout", callback, float(delay), time, None))

    @classmethod
    def new(
        cls,
        callback: Callable[[float], object],
        delay: float,
        time: float | None = None,
    ) -> "Timeout":
        return cls(callback, delay, time)


def timer(
    callback: Callable[[float], bool],
    delay: float | None = None,
    time: float | None = None,
) -> Timer:
    return Timer(callback, delay, time)


def interval(
    callback: Callable[[float], bool],
    interval_ms: float,
    time: float | None = None,
) -> Interval:
    return Interval(callback, interval_ms, time)


def timeout(
    callback: Callable[[float], object],
    delay: float,
    time: float | None = None,
) -> Timeout:
    return Timeout(callback, delay, time)


def now() -> float:
    return float(_timer_now())


def set_now(value: float) -> None:
    _timer_set_now(float(value))


def timer_flush() -> None:
    _timer_flush()


class TransitionEase(str, Enum):
    LINEAR = "linear"
    QUAD_IN = "quad_in"
    QUAD_OUT = "quad_out"
    QUAD_IN_OUT = "quad_in_out"
    CUBIC_IN = "cubic_in"
    CUBIC_OUT = "cubic_out"
    CUBIC_IN_OUT = "cubic_in_out"
    SIN_IN = "sin_in"
    SIN_OUT = "sin_out"
    SIN_IN_OUT = "sin_in_out"
    EXP_IN = "exp_in"
    EXP_OUT = "exp_out"
    EXP_IN_OUT = "exp_in_out"
    CIRCLE_IN = "circle_in"
    CIRCLE_OUT = "circle_out"
    CIRCLE_IN_OUT = "circle_in_out"
    ELASTIC_IN = "elastic_in"
    ELASTIC_OUT = "elastic_out"
    ELASTIC_IN_OUT = "elastic_in_out"
    BACK_IN = "back_in"
    BACK_OUT = "back_out"
    BACK_IN_OUT = "back_in_out"
    BOUNCE_IN = "bounce_in"
    BOUNCE_OUT = "bounce_out"
    BOUNCE_IN_OUT = "bounce_in_out"


class TransitionState(str, Enum):
    PENDING = "pending"
    ACTIVE = "active"
    ENDED = "ended"
    INTERRUPTED = "interrupted"


@dataclass(frozen=True)
class TransitionConfig:
    duration: float = 250.0
    delay: float = 0.0
    ease: TransitionEase = TransitionEase.LINEAR
    name: str | None = None


@dataclass(frozen=True)
class Transition:
    _config: TransitionConfig = field(default_factory=TransitionConfig, repr=False)
    _start: float = field(default=0.0, repr=False)
    _end: float = field(default=1.0, repr=False)
    _on_start: Callable[[], None] | None = field(default=None, repr=False, compare=False)
    _on_end: Callable[[], None] | None = field(default=None, repr=False, compare=False)
    _on_interrupt: Callable[[], None] | None = field(default=None, repr=False, compare=False)

    @classmethod
    def new(cls) -> "Transition":
        return cls()

    def duration(self, milliseconds: float) -> "Transition":
        return replace(self, _config=replace(self._config, duration=float(milliseconds)))

    def delay(self, milliseconds: float) -> "Transition":
        return replace(self, _config=replace(self._config, delay=float(milliseconds)))

    def ease(self, ease: TransitionEase) -> "Transition":
        return replace(self, _config=replace(self._config, ease=TransitionEase(ease)))

    def name(self, name: str) -> "Transition":
        return replace(self, _config=replace(self._config, name=str(name)))

    def from_to(self, start: float, end: float) -> "Transition":
        return replace(self, _start=float(start), _end=float(end))

    def to(self, end: float) -> "Transition":
        return replace(self, _end=float(end))

    def on_start(self, callback: Callable[[], None]) -> "Transition":
        if not callable(callback):
            raise TypeError("callback must be callable")
        return replace(self, _on_start=callback)

    def on_end(self, callback: Callable[[], None]) -> "Transition":
        if not callable(callback):
            raise TypeError("callback must be callable")
        return replace(self, _on_end=callback)

    def on_interrupt(self, callback: Callable[[], None]) -> "Transition":
        if not callable(callback):
            raise TypeError("callback must be callable")
        return replace(self, _on_interrupt=callback)

    def start(self) -> "TransitionHandle":
        if _NativeTransitionState is None:
            raise RuntimeError("native extension is not installed")
        native_state = _NativeTransitionState(
            self._config.duration,
            self._config.delay,
            self._config.ease.value,
            self._config.name,
            self._start,
            self._end,
        )
        return TransitionHandle(
            native_state,
            self._on_start,
            self._on_end,
            self._on_interrupt,
        )


class TransitionHandle:
    def __init__(
        self,
        native_state: object,
        on_start: Callable[[], None] | None,
        on_end: Callable[[], None] | None,
        on_interrupt: Callable[[], None] | None,
    ) -> None:
        self._native_state = native_state
        self._on_start = on_start
        self._on_end = on_end
        self._on_interrupt = on_interrupt
        self._closed = False

    def _require_open(self) -> object:
        if self._closed:
            raise RuntimeError("transition handle is closed")
        return self._native_state

    def tick(self, delta_ms: float) -> float:
        native_state = self._require_open()
        value, before_value, after_value = native_state.tick(float(delta_ms))
        before = TransitionState(before_value)
        after = TransitionState(after_value)
        if before is TransitionState.PENDING and after is not TransitionState.PENDING:
            if self._on_start is not None:
                self._on_start()
        if after is TransitionState.ENDED and before is not TransitionState.ENDED:
            if self._on_end is not None:
                self._on_end()
        return float(value)

    def value(self) -> float:
        return float(self._require_open().value())

    def state(self) -> TransitionState:
        return TransitionState(self._require_open().state())

    def is_complete(self) -> bool:
        return bool(self._require_open().is_complete())

    def interrupt(self) -> None:
        before_value, after_value = self._require_open().interrupt()
        before = TransitionState(before_value)
        after = TransitionState(after_value)
        if after is TransitionState.INTERRUPTED and before is not after:
            if self._on_interrupt is not None:
                self._on_interrupt()

    def reset(self) -> None:
        self._require_open().reset()

    def close(self) -> None:
        if not self._closed:
            if not self.is_complete():
                self.interrupt()
            self._closed = True
            self._native_state = None

    def __enter__(self) -> "TransitionHandle":
        self._require_open()
        return self

    def __exit__(self, *_args: object) -> None:
        self.close()


class TransitionManager:
    def __init__(self) -> None:
        self._transitions: dict[str, TransitionHandle] = {}

    @classmethod
    def new(cls) -> "TransitionManager":
        return cls()

    def add(self, name: str, transition: Transition) -> None:
        if not isinstance(transition, Transition):
            raise TypeError("transition must be a Transition")
        key = str(name)
        previous = self._transitions.pop(key, None)
        if previous is not None:
            previous.interrupt()
            previous.close()
        self._transitions[key] = transition.name(key).start()

    def tick(self, delta_ms: float) -> tuple[tuple[str, float], ...]:
        results: list[tuple[str, float]] = []
        completed: list[str] = []
        for name, transition in self._transitions.items():
            results.append((name, transition.tick(delta_ms)))
            if transition.is_complete():
                completed.append(name)
        for name in completed:
            self._transitions.pop(name).close()
        return tuple(results)

    def get(self, name: str) -> float | None:
        transition = self._transitions.get(name)
        return None if transition is None else transition.value()

    def interrupt(self, name: str) -> None:
        transition = self._transitions.pop(name, None)
        if transition is not None:
            transition.interrupt()
            transition.close()

    def interrupt_all(self) -> None:
        for name in tuple(self._transitions):
            self.interrupt(name)

    def is_animating(self) -> bool:
        return any(
            transition.state() is TransitionState.ACTIVE
            for transition in self._transitions.values()
        )


@dataclass(frozen=True)
class Event:
    type_: str
    data: object | None = None

    @classmethod
    def new(cls, type_: str, data: object | None = None) -> "Event":
        return cls(str(type_), data)

    @classmethod
    def with_data(cls, type_: str, data: object) -> "Event":
        return cls(str(type_), data)


@dataclass(frozen=True)
class ListenerId:
    value: int


class Dispatcher:
    _next_listener_id = 1

    def __init__(self) -> None:
        self._listeners: list[tuple[ListenerId, str, Callable[[Event], None], bool]] = []

    @classmethod
    def new(cls) -> "Dispatcher":
        return cls()

    def _add(self, type_: str, callback: Callable[[Event], None], once: bool) -> ListenerId:
        if not callable(callback):
            raise TypeError("callback must be callable")
        listener_id = ListenerId(Dispatcher._next_listener_id)
        Dispatcher._next_listener_id += 1
        self._listeners.append((listener_id, str(type_), callback, once))
        return listener_id

    def on(self, type_: str, callback: Callable[[Event], None]) -> ListenerId:
        return self._add(type_, callback, False)

    def once(self, type_: str, callback: Callable[[Event], None]) -> ListenerId:
        return self._add(type_, callback, True)

    def off(self, listener_id: ListenerId) -> None:
        self._listeners = [entry for entry in self._listeners if entry[0] != listener_id]

    def off_all(self, type_: str) -> None:
        self._listeners = [entry for entry in self._listeners if entry[1] != type_]

    def dispatch(self, type_: str, data: object | None = None) -> None:
        event = Event.new(type_, data)
        remove: set[ListenerId] = set()
        for listener_id, event_type, callback, once in tuple(self._listeners):
            if event_type == type_:
                callback(event)
                if once:
                    remove.add(listener_id)
        if remove:
            self._listeners = [entry for entry in self._listeners if entry[0] not in remove]

    def dispatch_typed(self, type_: str, data: object) -> None:
        self.dispatch(type_, data)

    def has_listeners(self, type_: str) -> bool:
        return any(entry[1] == type_ for entry in self._listeners)

    def listener_count(self, type_: str) -> int:
        return len([entry for entry in self._listeners if entry[1] == type_])

    def total_listeners(self) -> int:
        return len(self._listeners)

    def event_types(self) -> tuple[str, ...]:
        return tuple(sorted({entry[1] for entry in self._listeners}))

    def clear(self) -> None:
        self._listeners.clear()


def dispatcher() -> Dispatcher:
    return Dispatcher.new()


class AxisScaleKind(str, Enum):
    LINEAR = "linear"
    LOG = "log"
    POW = "pow"
    SYMLOG = "symlog"


@dataclass(frozen=True)
class AxisScale:
    kind: AxisScaleKind = AxisScaleKind.LINEAR
    domain: tuple[float, float] = (0.0, 1.0)
    range: tuple[float, float] = (0.0, 1.0)
    parameter: float = 1.0
    clamped: bool = False
    nice_count: int | None = None

    @classmethod
    def linear(cls) -> "AxisScale":
        return cls()

    @classmethod
    def log(cls) -> "AxisScale":
        return cls(AxisScaleKind.LOG, (1.0, 10.0), parameter=10.0, clamped=True)

    @classmethod
    def pow(cls) -> "AxisScale":
        return cls(AxisScaleKind.POW)

    @classmethod
    def symlog(cls) -> "AxisScale":
        return cls(AxisScaleKind.SYMLOG)

    def with_domain(self, minimum: float, maximum: float) -> "AxisScale":
        return replace(self, domain=(float(minimum), float(maximum)))

    def with_range(self, minimum: float, maximum: float) -> "AxisScale":
        return replace(self, range=(float(minimum), float(maximum)))

    def range_normalized(self, maximum: float) -> "AxisScale":
        return self.with_range(0.0, maximum)

    def base(self, base: float) -> "AxisScale":
        if self.kind is not AxisScaleKind.LOG:
            raise TypeError("base is only valid for log scales")
        return replace(self, parameter=float(base))

    def exponent(self, exponent: float) -> "AxisScale":
        if self.kind is not AxisScaleKind.POW:
            raise TypeError("exponent is only valid for pow scales")
        return replace(self, parameter=float(exponent))

    def constant(self, constant: float) -> "AxisScale":
        if self.kind is not AxisScaleKind.SYMLOG:
            raise TypeError("constant is only valid for symlog scales")
        return replace(self, parameter=float(constant))

    def clamp(self, enabled: bool) -> "AxisScale":
        if not isinstance(enabled, bool):
            raise TypeError("enabled must be bool")
        return replace(self, clamped=enabled)

    def nice(self, count: int = 10) -> "AxisScale":
        if self.kind is AxisScaleKind.LOG:
            raise TypeError("nice is unavailable for log scales")
        if not isinstance(count, int) or isinstance(count, bool) or count <= 0:
            raise ValueError("count must be a positive integer")
        return replace(self, nice_count=count)


class AxisOrientation(str, Enum):
    TOP = "top"
    RIGHT = "right"
    BOTTOM = "bottom"
    LEFT = "left"

    def is_horizontal(self) -> bool:
        return self in (AxisOrientation.TOP, AxisOrientation.BOTTOM)

    def is_vertical(self) -> bool:
        return not self.is_horizontal()


@dataclass(frozen=True)
class AxisPoint:
    x: float
    y: float


@dataclass(frozen=True)
class AxisLine:
    start: AxisPoint
    end: AxisPoint


@dataclass(frozen=True)
class AxisTick:
    value: float
    position: float
    line: AxisLine
    label_position: AxisPoint | None
    label: str | None
    label_angle_degrees: float
    is_minor: bool


@dataclass(frozen=True)
class AxisTitle:
    text: str
    position: AxisPoint
    angle_degrees: float


@dataclass(frozen=True)
class AxisLayout:
    orientation: AxisOrientation
    size: float
    domain_line: AxisLine | None
    major_ticks: tuple[AxisTick, ...]
    minor_ticks: tuple[AxisTick, ...]
    title: AxisTitle | None

    def ticks(self) -> tuple[AxisTick, ...]:
        return (*self.major_ticks, *self.minor_ticks)


class AxisLayoutErrorKind(str, Enum):
    NON_FINITE_CONFIG = "non_finite_config"
    NEGATIVE_CONFIG = "negative_config"
    NON_FINITE_RANGE = "non_finite_range"
    NON_FINITE_TICK = "non_finite_tick"
    NON_FINITE_TICK_POSITION = "non_finite_tick_position"


class AxisLayoutError(ValueError):
    def __init__(
        self,
        kind: AxisLayoutErrorKind,
        message: str,
        *,
        field: str | None = None,
        value: float | None = None,
    ) -> None:
        super().__init__(message)
        self.kind = kind
        self.field = field
        self.value = value


def _axis_point(value: tuple[float, float]) -> AxisPoint:
    return AxisPoint(float(value[0]), float(value[1]))


def _axis_line(value: tuple[tuple[float, float], tuple[float, float]]) -> AxisLine:
    return AxisLine(_axis_point(value[0]), _axis_point(value[1]))


def _axis_tick(value: tuple[object, ...]) -> AxisTick:
    return AxisTick(
        float(value[0]),
        float(value[1]),
        _axis_line(value[2]),
        None if value[3] is None else _axis_point(value[3]),
        None if value[4] is None else str(value[4]),
        float(value[5]),
        bool(value[6]),
    )


@dataclass(frozen=True)
class AxisConfig:
    _orientation: AxisOrientation = field(default=AxisOrientation.BOTTOM, repr=False)
    _tick_count: int = field(default=10, repr=False)
    _tick_values: tuple[float, ...] | None = field(default=None, repr=False)
    _minor_tick_values: tuple[float, ...] | None = field(default=None, repr=False)
    _minor_tick_size: float = field(default=3.0, repr=False)
    _tick_size: float = field(default=6.0, repr=False)
    _tick_padding: float = field(default=4.0, repr=False)
    _label_font_size: float = field(default=10.0, repr=False)
    _formatter: Callable[[float], str] | None = field(default=None, repr=False, compare=False)
    _show_domain_line: bool = field(default=True, repr=False)
    _domain_line_width: float = field(default=1.0, repr=False)
    _title: str | None = field(default=None, repr=False)
    _title_font_size: float = field(default=12.0, repr=False)
    _title_padding: float = field(default=8.0, repr=False)
    _label_angle: float = field(default=0.0, repr=False)

    @classmethod
    def bottom(cls) -> "AxisConfig": return cls()
    @classmethod
    def top(cls) -> "AxisConfig": return cls(_orientation=AxisOrientation.TOP)
    @classmethod
    def left(cls) -> "AxisConfig": return cls(_orientation=AxisOrientation.LEFT)
    @classmethod
    def right(cls) -> "AxisConfig": return cls(_orientation=AxisOrientation.RIGHT)

    def with_ticks(self, count: int) -> "AxisConfig":
        if not isinstance(count, int) or isinstance(count, bool): raise TypeError("count must be int")
        return replace(self, _tick_count=count)
    def with_tick_values(self, values: Sequence[float]) -> "AxisConfig":
        return replace(self, _tick_values=tuple(float(value) for value in values))
    def with_minor_tick_values(self, values: Sequence[float]) -> "AxisConfig":
        return replace(self, _minor_tick_values=tuple(float(value) for value in values))
    def with_minor_tick_size(self, size: float) -> "AxisConfig": return replace(self, _minor_tick_size=float(size))
    def with_tick_size(self, size: float) -> "AxisConfig": return replace(self, _tick_size=float(size))
    def with_tick_padding(self, padding: float) -> "AxisConfig": return replace(self, _tick_padding=float(padding))
    def with_label_font_size(self, size: float) -> "AxisConfig": return replace(self, _label_font_size=float(size))
    def with_formatter(self, formatter: Callable[[float], str]) -> "AxisConfig":
        if not callable(formatter): raise TypeError("formatter must be callable")
        return replace(self, _formatter=formatter)
    def hide_domain_line(self) -> "AxisConfig": return replace(self, _show_domain_line=False)
    def with_domain_line_width(self, width: float) -> "AxisConfig": return replace(self, _domain_line_width=float(width))
    def with_title(self, title: str) -> "AxisConfig": return replace(self, _title=str(title))
    def with_title_font_size(self, size: float) -> "AxisConfig": return replace(self, _title_font_size=float(size))
    def with_title_padding(self, padding: float) -> "AxisConfig": return replace(self, _title_padding=float(padding))
    def with_label_angle(self, angle_degrees: float) -> "AxisConfig": return replace(self, _label_angle=float(angle_degrees))

    def total_size(self) -> float:
        title_space = self._title_padding + self._title_font_size if self._title is not None else 0.0
        if self._orientation.is_vertical(): return 60.0 + title_space
        angle = abs(self._label_angle) * math.pi / 180.0
        label_height = self._label_font_size if abs(self._label_angle) <= 0.1 else abs(math.sin(angle)) * self._label_font_size + abs(math.cos(angle)) * self._label_font_size
        return self._tick_size + self._tick_padding + label_height + title_space + 4.0

    def try_layout(self, scale: AxisScale, size: float) -> AxisLayout:
        if not isinstance(scale, AxisScale): raise TypeError("scale must be an AxisScale")
        result, error = _axis_layout(
            scale.kind.value, scale.domain, scale.range, scale.parameter, scale.clamped,
            scale.nice_count, self._orientation.value, self._tick_count, self._tick_values,
            self._minor_tick_values, self._minor_tick_size, self._tick_size,
            self._tick_padding, self._label_font_size, self._show_domain_line,
            self._domain_line_width, self._title, self._title_font_size,
            self._title_padding, self._label_angle, float(size),
        )
        if error is not None:
            index, error_field, error_value, message = error
            kind = tuple(AxisLayoutErrorKind)[int(index)]
            raise AxisLayoutError(kind, str(message), field=str(error_field) or None,
                                  value=float(error_value) if kind in (AxisLayoutErrorKind.NON_FINITE_TICK, AxisLayoutErrorKind.NON_FINITE_TICK_POSITION) else None)
        if result is None: raise RuntimeError("native axis layout returned neither a result nor an error")
        orientation, layout_size, domain_line, major, minor, title = result
        major_ticks = tuple(_axis_tick(tick) for tick in major)
        if self._formatter is not None:
            major_ticks = tuple(replace(tick, label=self._formatter(tick.value)) if tick.label is not None else tick for tick in major_ticks)
        return AxisLayout(
            AxisOrientation(orientation), float(layout_size),
            None if domain_line is None else _axis_line(domain_line), major_ticks,
            tuple(_axis_tick(tick) for tick in minor),
            None if title is None else AxisTitle(str(title[0]), _axis_point(title[1]), float(title[2])),
        )

    def layout(self, scale: AxisScale, size: float) -> AxisLayout:
        return self.try_layout(scale, size)


def axis_layout(scale: AxisScale, config: AxisConfig, size: float) -> AxisLayout:
    return config.try_layout(scale, size)


@dataclass(frozen=True)
class GridPoint:
    x: float
    y: float


@dataclass(frozen=True)
class GridLine:
    value: float
    start: GridPoint
    end: GridPoint


@dataclass(frozen=True)
class GridDot:
    x_value: float
    y_value: float
    center: GridPoint


@dataclass(frozen=True)
class GridLayout:
    width: float
    height: float
    vertical_lines: tuple[GridLine, ...]
    horizontal_lines: tuple[GridLine, ...]
    dots: tuple[GridDot, ...]

    def is_empty(self) -> bool:
        return not self.vertical_lines and not self.horizontal_lines and not self.dots


class GridLayoutErrorKind(str, Enum):
    NON_FINITE_SIZE = "non_finite_size"
    NEGATIVE_SIZE = "negative_size"
    NON_FINITE_CONFIG = "non_finite_config"
    NEGATIVE_CONFIG = "negative_config"
    INVALID_OPACITY = "invalid_opacity"
    NON_FINITE_RANGE = "non_finite_range"
    DEGENERATE_RANGE = "degenerate_range"
    NON_FINITE_TICK = "non_finite_tick"
    NON_FINITE_TICK_POSITION = "non_finite_tick_position"


class GridLayoutError(ValueError):
    def __init__(
        self,
        kind: GridLayoutErrorKind,
        message: str,
        *,
        field: str | None = None,
        axis: str | None = None,
        value: float | None = None,
    ) -> None:
        super().__init__(message)
        self.kind = kind
        self.field = field
        self.axis = axis
        self.value = value


@dataclass(frozen=True)
class GridConfig:
    _show_vertical_lines: bool = field(default=False, repr=False)
    _show_horizontal_lines: bool = field(default=False, repr=False)
    _show_dots: bool = field(default=True, repr=False)
    _line_width: float = field(default=1.0, repr=False)
    _dot_radius: float = field(default=2.0, repr=False)
    _line_opacity: float = field(default=0.2, repr=False)
    _dot_opacity: float = field(default=0.4, repr=False)
    _vertical_values: tuple[float, ...] | None = field(default=None, repr=False)
    _horizontal_values: tuple[float, ...] | None = field(default=None, repr=False)

    @classmethod
    def new(cls) -> "GridConfig":
        return cls()

    @classmethod
    def dots_only(cls) -> "GridConfig":
        return cls()

    @classmethod
    def with_lines(cls) -> "GridConfig":
        return cls(_show_vertical_lines=True, _show_horizontal_lines=True)

    @classmethod
    def lines_only(cls) -> "GridConfig":
        return cls(
            _show_vertical_lines=True,
            _show_horizontal_lines=True,
            _show_dots=False,
        )

    @staticmethod
    def _bool(name: str, value: bool) -> bool:
        if not isinstance(value, bool):
            raise TypeError(f"{name} must be bool")
        return value

    def with_vertical_lines(self, show: bool) -> "GridConfig":
        return replace(self, _show_vertical_lines=self._bool("show", show))

    def with_horizontal_lines(self, show: bool) -> "GridConfig":
        return replace(self, _show_horizontal_lines=self._bool("show", show))

    def with_dots(self, show: bool) -> "GridConfig":
        return replace(self, _show_dots=self._bool("show", show))

    def with_line_width(self, width: float) -> "GridConfig":
        return replace(self, _line_width=float(width))

    def with_dot_radius(self, radius: float) -> "GridConfig":
        return replace(self, _dot_radius=float(radius))

    def with_line_opacity(self, opacity: float) -> "GridConfig":
        value = float(opacity)
        if value < 0.0:
            value = 0.0
        elif value > 1.0:
            value = 1.0
        return replace(self, _line_opacity=value)

    def with_dot_opacity(self, opacity: float) -> "GridConfig":
        value = float(opacity)
        if value < 0.0:
            value = 0.0
        elif value > 1.0:
            value = 1.0
        return replace(self, _dot_opacity=value)

    def with_vertical_values(self, values: Sequence[float]) -> "GridConfig":
        return replace(self, _vertical_values=tuple(float(value) for value in values))

    def with_horizontal_values(self, values: Sequence[float]) -> "GridConfig":
        return replace(self, _horizontal_values=tuple(float(value) for value in values))

    def try_layout(
        self,
        x_scale: AxisScale,
        y_scale: AxisScale,
        width: float,
        height: float,
    ) -> GridLayout:
        if not isinstance(x_scale, AxisScale) or not isinstance(y_scale, AxisScale):
            raise TypeError("x_scale and y_scale must be AxisScale values")
        result, error = _grid_layout(
            x_scale.kind.value, x_scale.domain, x_scale.range, x_scale.parameter,
            x_scale.clamped, x_scale.nice_count,
            y_scale.kind.value, y_scale.domain, y_scale.range, y_scale.parameter,
            y_scale.clamped, y_scale.nice_count,
            self._show_vertical_lines, self._show_horizontal_lines, self._show_dots,
            self._line_width, self._dot_radius, self._line_opacity, self._dot_opacity,
            self._vertical_values, self._horizontal_values, float(width), float(height),
        )
        if error is not None:
            index, error_field, error_axis, error_value, message = error
            kind = tuple(GridLayoutErrorKind)[int(index)]
            raise GridLayoutError(
                kind,
                str(message),
                field=str(error_field) or None,
                axis=str(error_axis) or None,
                value=float(error_value)
                if kind in (
                    GridLayoutErrorKind.INVALID_OPACITY,
                    GridLayoutErrorKind.NON_FINITE_TICK,
                    GridLayoutErrorKind.NON_FINITE_TICK_POSITION,
                )
                else None,
            )
        if result is None:
            raise RuntimeError("native grid layout returned neither a result nor an error")
        layout_width, layout_height, vertical, horizontal, dots = result
        line = lambda value: GridLine(
            float(value[0]),
            GridPoint(float(value[1][0]), float(value[1][1])),
            GridPoint(float(value[2][0]), float(value[2][1])),
        )
        return GridLayout(
            float(layout_width),
            float(layout_height),
            tuple(line(value) for value in vertical),
            tuple(line(value) for value in horizontal),
            tuple(
                GridDot(
                    float(value[0]),
                    float(value[1]),
                    GridPoint(float(value[2][0]), float(value[2][1])),
                )
                for value in dots
            ),
        )

    def layout(
        self,
        x_scale: AxisScale,
        y_scale: AxisScale,
        width: float,
        height: float,
    ) -> GridLayout:
        return self.try_layout(x_scale, y_scale, width, height)


def grid_layout(
    x_scale: AxisScale,
    y_scale: AxisScale,
    config: GridConfig,
    width: float,
    height: float,
) -> GridLayout:
    return config.try_layout(x_scale, y_scale, width, height)


class LegendPosition(str, Enum):
    TOP_LEFT = "top_left"
    TOP_RIGHT = "top_right"
    BOTTOM_LEFT = "bottom_left"
    BOTTOM_RIGHT = "bottom_right"
    TOP = "top"
    BOTTOM = "bottom"
    LEFT = "left"
    RIGHT = "right"


class LegendOrientation(str, Enum):
    HORIZONTAL = "horizontal"
    VERTICAL = "vertical"


class LegendSymbol(str, Enum):
    CIRCLE = "circle"
    SQUARE = "square"
    LINE = "line"
    LINE_WITH_MARKER = "line_with_marker"
    DASHED_LINE = "dashed_line"
    NONE = "none"


@dataclass(frozen=True)
class LegendItem:
    _label: str
    _color: str
    _symbol: LegendSymbol = field(default=LegendSymbol.CIRCLE, repr=False)
    _data: str | None = field(default=None, repr=False)

    @classmethod
    def color(cls, label: str, color: str) -> "LegendItem":
        return cls(str(label), str(color))

    @classmethod
    def line(cls, label: str, color: str) -> "LegendItem":
        return cls(str(label), str(color), LegendSymbol.LINE)

    @classmethod
    def with_symbol(
        cls, label: str, color: str, symbol: LegendSymbol
    ) -> "LegendItem":
        return cls(str(label), str(color), LegendSymbol(symbol))

    def symbol(self, symbol: LegendSymbol) -> "LegendItem":
        return replace(self, _symbol=LegendSymbol(symbol))

    def data(self, data: str) -> "LegendItem":
        return replace(self, _data=str(data))


@dataclass(frozen=True)
class LegendPoint:
    x: float
    y: float


@dataclass(frozen=True)
class LegendRect:
    origin: LegendPoint
    width: float
    height: float


@dataclass(frozen=True)
class LegendTitleLayout:
    text: str
    bounds: LegendRect


@dataclass(frozen=True)
class LegendItemLayout:
    index: int
    row: int
    column: int
    label: str
    symbol: LegendSymbol
    item_bounds: LegendRect
    symbol_bounds: LegendRect
    label_bounds: LegendRect


@dataclass(frozen=True)
class LegendLayout:
    width: float
    height: float
    columns: int
    rows: int
    column_widths: tuple[float, ...]
    title: LegendTitleLayout | None
    items: tuple[LegendItemLayout, ...]

    def is_empty(self) -> bool:
        return self.title is None and not self.items


class LegendLayoutErrorKind(str, Enum):
    NON_FINITE_SIZE = "non_finite_size"
    NEGATIVE_SIZE = "negative_size"
    NON_FINITE_CONFIG = "non_finite_config"
    NEGATIVE_CONFIG = "negative_config"
    NON_POSITIVE_AVERAGE_CHAR_WIDTH = "non_positive_average_char_width"


class LegendLayoutError(ValueError):
    def __init__(
        self,
        kind: LegendLayoutErrorKind,
        message: str,
        *,
        field: str | None = None,
        value: float | None = None,
    ) -> None:
        super().__init__(message)
        self.kind = kind
        self.field = field
        self.value = value


def _legend_rect(value: tuple[float, float, float, float]) -> LegendRect:
    return LegendRect(LegendPoint(value[0], value[1]), value[2], value[3])


@dataclass(frozen=True)
class LegendConfig:
    _position: LegendPosition = field(default=LegendPosition.TOP_RIGHT, repr=False)
    _orientation: LegendOrientation = field(default=LegendOrientation.VERTICAL, repr=False)
    _title: str | None = field(default=None, repr=False)
    _items: tuple[LegendItem, ...] = field(default=(), repr=False)
    _symbol_size: float = field(default=12.0, repr=False)
    _item_spacing: float = field(default=8.0, repr=False)
    _padding: float = field(default=8.0, repr=False)
    _background: bool = field(default=True, repr=False)
    _background_color: str = field(default="#ffffff", repr=False)
    _border_width: float = field(default=1.0, repr=False)
    _border_color: str = field(default="#c8c8c8", repr=False)
    _font_size: float = field(default=12.0, repr=False)
    _max_width: float | None = field(default=None, repr=False)

    @classmethod
    def new(cls) -> "LegendConfig":
        return cls()

    def position(self, position: LegendPosition) -> "LegendConfig":
        return replace(self, _position=LegendPosition(position))

    def orientation(self, orientation: LegendOrientation) -> "LegendConfig":
        return replace(self, _orientation=LegendOrientation(orientation))

    def title(self, title: str) -> "LegendConfig":
        return replace(self, _title=str(title))

    def items(self, items: Sequence[LegendItem]) -> "LegendConfig":
        values = tuple(items)
        if not all(isinstance(item, LegendItem) for item in values):
            raise TypeError("items must contain only LegendItem values")
        return replace(self, _items=values)

    def add_item(self, item: LegendItem) -> "LegendConfig":
        if not isinstance(item, LegendItem):
            raise TypeError("item must be a LegendItem")
        return replace(self, _items=(*self._items, item))

    def symbol_size(self, size: float) -> "LegendConfig":
        return replace(self, _symbol_size=float(size))

    def item_spacing(self, spacing: float) -> "LegendConfig":
        return replace(self, _item_spacing=float(spacing))

    def padding(self, padding: float) -> "LegendConfig":
        return replace(self, _padding=float(padding))

    def background(self, enabled: bool) -> "LegendConfig":
        if not isinstance(enabled, bool):
            raise TypeError("enabled must be bool")
        return replace(self, _background=enabled)

    def background_color(self, color: str) -> "LegendConfig":
        return replace(self, _background_color=str(color))

    def border_width(self, width: float) -> "LegendConfig":
        return replace(self, _border_width=float(width))

    def border_color(self, color: str) -> "LegendConfig":
        return replace(self, _border_color=str(color))

    def font_size(self, size: float) -> "LegendConfig":
        return replace(self, _font_size=float(size))

    def max_width(self, width: float) -> "LegendConfig":
        return replace(self, _max_width=float(width))

    def _native_items(self) -> list[tuple[str, str, str, str | None]]:
        return [
            (item._label, item._color, item._symbol.value, item._data)
            for item in self._items
        ]

    def try_layout(
        self, available_width: float, *, avg_char_width: float | None = None
    ) -> LegendLayout:
        result, error = _legend_layout(
            self._position.value,
            self._orientation.value,
            self._title,
            self._native_items(),
            self._symbol_size,
            self._item_spacing,
            self._padding,
            self._background,
            self._background_color,
            self._border_width,
            self._border_color,
            self._font_size,
            self._max_width,
            float(available_width),
            None if avg_char_width is None else float(avg_char_width),
        )
        if error is not None:
            kind_index, error_field, error_value, message = error
            kind = tuple(LegendLayoutErrorKind)[int(kind_index)]
            raise LegendLayoutError(
                kind,
                str(message),
                field=str(error_field) or None,
                value=float(error_value)
                if kind is LegendLayoutErrorKind.NON_POSITIVE_AVERAGE_CHAR_WIDTH
                else None,
            )
        if result is None:
            raise RuntimeError("native legend layout returned neither a result nor an error")
        width, height, columns, rows, column_widths, title, items = result
        title_layout = (
            None
            if title is None
            else LegendTitleLayout(str(title[0]), _legend_rect(title[1]))
        )
        item_layouts = tuple(
            LegendItemLayout(
                int(item[0]),
                int(item[1]),
                int(item[2]),
                str(item[3]),
                LegendSymbol(item[4]),
                _legend_rect(item[5]),
                _legend_rect(item[6]),
                _legend_rect(item[7]),
            )
            for item in items
        )
        return LegendLayout(
            float(width),
            float(height),
            int(columns),
            int(rows),
            tuple(float(value) for value in column_widths),
            title_layout,
            item_layouts,
        )

    def layout(self, available_width: float) -> LegendLayout:
        return self.try_layout(available_width)

    def layout_with_char_width(
        self, available_width: float, avg_char_width: float
    ) -> LegendLayout:
        return self.try_layout(available_width, avg_char_width=avg_char_width)

    def estimate_dimensions(self, avg_char_width: float) -> tuple[float, float]:
        if not self._items:
            return (0.0, 0.0)
        title_height = self._font_size * 1.5 if self._title is not None else 0.0
        if self._orientation is LegendOrientation.VERTICAL:
            max_label_width = max(len(item._label) * avg_char_width for item in self._items)
            return (
                self._padding * 2.0 + self._symbol_size + 8.0 + max_label_width,
                self._padding * 2.0
                + title_height
                + len(self._items) * (self._symbol_size + self._item_spacing)
                - self._item_spacing,
            )
        total_width = sum(
            [
            self._symbol_size
            + 8.0
            + len(item._label) * avg_char_width
            + self._item_spacing
            for item in self._items
            ]
        )
        return (
            self._padding * 2.0 + total_width - self._item_spacing,
            self._padding * 2.0 + title_height + self._symbol_size,
        )

    def offset_from_corner(
        self,
        chart_width: float,
        chart_height: float,
        legend_width: float,
        legend_height: float,
        margin: float,
    ) -> tuple[float, float]:
        horizontal = {
            LegendPosition.TOP_LEFT: margin,
            LegendPosition.BOTTOM_LEFT: margin,
            LegendPosition.LEFT: margin,
            LegendPosition.TOP_RIGHT: chart_width - legend_width - margin,
            LegendPosition.BOTTOM_RIGHT: chart_width - legend_width - margin,
            LegendPosition.RIGHT: chart_width - legend_width - margin,
            LegendPosition.TOP: (chart_width - legend_width) / 2.0,
            LegendPosition.BOTTOM: (chart_width - legend_width) / 2.0,
        }
        vertical = {
            LegendPosition.TOP_LEFT: margin,
            LegendPosition.TOP_RIGHT: margin,
            LegendPosition.TOP: margin,
            LegendPosition.BOTTOM_LEFT: chart_height - legend_height - margin,
            LegendPosition.BOTTOM_RIGHT: chart_height - legend_height - margin,
            LegendPosition.BOTTOM: chart_height - legend_height - margin,
            LegendPosition.LEFT: (chart_height - legend_height) / 2.0,
            LegendPosition.RIGHT: (chart_height - legend_height) / 2.0,
        }
        return horizontal[self._position], vertical[self._position]


def legend_layout(config: LegendConfig, available_width: float) -> LegendLayout:
    return config.try_layout(available_width)


def legend_from_scale(
    scale: Callable[[float], str],
    ticks: Sequence[float],
    formatter: Callable[[float], str],
) -> tuple[LegendItem, ...]:
    return tuple(
        LegendItem.color(formatter(float(tick)), scale(float(tick))).symbol(
            LegendSymbol.SQUARE
        )
        for tick in ticks
    )


class TileErrorKind(str, Enum):
    NON_FINITE_SCALE = "non_finite_scale"
    NON_POSITIVE_SCALE = "non_positive_scale"
    NON_FINITE_TILE_SIZE = "non_finite_tile_size"
    NON_POSITIVE_TILE_SIZE = "non_positive_tile_size"
    NON_FINITE_TRANSLATE = "non_finite_translate"
    NON_FINITE_EXTENT = "non_finite_extent"
    INVALID_EXTENT = "invalid_extent"
    ZOOM_OUT_OF_RANGE = "zoom_out_of_range"
    TOO_MANY_TILES = "too_many_tiles"


class TileError(ValueError):
    def __init__(self, kind: TileErrorKind, message: str) -> None:
        super().__init__(message)
        self.kind = kind


@dataclass(frozen=True)
class TileSet:
    tiles: tuple[Tile, ...]
    zoom: int
    tile_screen_size: float
    origin: tuple[float, float]

    def tile_bounds(
        self, tile: Tile
    ) -> tuple[tuple[float, float], tuple[float, float]]:
        if not isinstance(tile, Tile):
            raise TypeError("tile must be a Tile")
        x0 = self.origin[0] + tile.x * self.tile_screen_size
        y0 = self.origin[1] + tile.y * self.tile_screen_size
        return (x0, y0), (x0 + self.tile_screen_size, y0 + self.tile_screen_size)

    def len(self) -> int:
        return len(self.tiles)

    def __len__(self) -> int:
        return len(self.tiles)

    def is_empty(self) -> bool:
        return not self.tiles


@dataclass(frozen=True)
class TileLayout:
    _extent: tuple[tuple[float, float], tuple[float, float]] = field(
        default=((0.0, 0.0), (960.0, 500.0)), repr=False
    )
    _scale: float = field(default=256.0, repr=False)
    _translate: tuple[float, float] = field(default=(480.0, 250.0), repr=False)
    _tile_size: float = field(default=256.0, repr=False)
    _zoom_delta: int = field(default=0, repr=False)
    _clamp_x: bool = field(default=True, repr=False)
    _clamp_y: bool = field(default=True, repr=False)

    @classmethod
    def new(cls) -> "TileLayout":
        return cls()

    def size(self, width: float, height: float) -> "TileLayout":
        return replace(self, _extent=((0.0, 0.0), (float(width), float(height))))

    def extent(
        self, extent: Sequence[Sequence[float]]
    ) -> "TileLayout":
        values = _geo_line_tuple(extent, "extent")
        if len(values) != 2:
            raise ValueError("extent must contain two points")
        return replace(self, _extent=(values[0], values[1]))

    def scale(self, scale: float) -> "TileLayout":
        return replace(self, _scale=float(scale))

    def translate(self, translate: Sequence[float]) -> "TileLayout":
        return replace(self, _translate=_geo_point_tuple(translate, "translate"))

    def tile_size(self, tile_size: float) -> "TileLayout":
        return replace(self, _tile_size=float(tile_size))

    def zoom_delta(self, zoom_delta: int) -> "TileLayout":
        if not isinstance(zoom_delta, int) or isinstance(zoom_delta, bool):
            raise TypeError("zoom_delta must be int")
        return replace(self, _zoom_delta=zoom_delta)

    def clamp(self, clamp_x: bool, clamp_y: bool) -> "TileLayout":
        return replace(self, _clamp_x=bool(clamp_x), _clamp_y=bool(clamp_y))

    def try_tiles(self) -> TileSet:
        result, error = _tile_layout(
            self._extent,
            self._scale,
            self._translate,
            self._tile_size,
            self._zoom_delta,
            self._clamp_x,
            self._clamp_y,
        )
        if error is not None:
            kind, message = error
            raise TileError(tuple(TileErrorKind)[int(kind)], str(message))
        if result is None:
            raise RuntimeError("native tile layout returned neither data nor an error")
        tiles, zoom, tile_screen_size, origin = result
        return TileSet(
            tuple(Tile(int(x), int(y), int(z)) for x, y, z in tiles),
            int(zoom),
            float(tile_screen_size),
            tuple(origin),
        )

    def tiles(self) -> TileSet:
        return self.try_tiles()


def tiles_for_viewport(
    width: float,
    height: float,
    scale: float,
    translate: Sequence[float],
) -> TileSet:
    return (
        TileLayout.new()
        .size(width, height)
        .scale(scale)
        .translate(translate)
        .try_tiles()
    )


@dataclass(frozen=True)
class Extent:
    x0: float
    y0: float
    x1: float
    y1: float

    def union(self, other: "Extent") -> "Extent":
        if not isinstance(other, Extent):
            raise TypeError("other must be an Extent")
        return Extent(
            self.x0 if self.x0 < other.x0 else other.x0,
            self.y0 if self.y0 < other.y0 else other.y0,
            self.x1 if self.x1 > other.x1 else other.x1,
            self.y1 if self.y1 > other.y1 else other.y1,
        )

    def width(self) -> float:
        return self.x1 - self.x0

    def height(self) -> float:
        return self.y1 - self.y0

    def contains(self, x: float, y: float) -> bool:
        return self.x0 <= x < self.x1 and self.y0 <= y < self.y1


@dataclass(frozen=True)
class Aggregate:
    mass: float
    x: float
    y: float

    def merge(self, other: "Aggregate") -> "Aggregate":
        if not isinstance(other, Aggregate):
            raise TypeError("other must be an Aggregate")
        mass = self.mass + other.mass
        if mass == 0.0:
            return Aggregate(0.0, 0.0, 0.0)
        return Aggregate(
            mass,
            (self.x * self.mass + other.x * other.mass) / mass,
            (self.y * self.mass + other.y * other.mass) / mass,
        )


@dataclass(frozen=True)
class QuadPoint:
    x: float
    y: float
    data: object
    next: "QuadPoint | None" = None


class QuadNodeKind(str, Enum):
    INTERNAL = "internal"
    LEAF = "leaf"


@dataclass(frozen=True)
class QuadNode:
    kind: QuadNodeKind
    point: QuadPoint | None
    aggregate: Aggregate | None


class QuadTreeError(ValueError):
    def __init__(self, message: str, *, index: int | None, x: float, y: float) -> None:
        super().__init__(message)
        self.index = index
        self.x = x
        self.y = y


def _quad_coordinate(value: object, path: str) -> float:
    try:
        return float(value)
    except (TypeError, ValueError) as error:
        raise QuadTreeError(
            f"{path} must be numeric", index=None, x=math.nan, y=math.nan
        ) from error


def _quad_error(index: int | None, x: float, y: float) -> QuadTreeError:
    location = "" if index is None else f" at index {index}"
    return QuadTreeError(
        f"quadtree point{location} has non-finite coordinates: ({x}, {y})",
        index=index,
        x=x,
        y=y,
    )


class QuadTree:
    def __init__(self) -> None:
        if _NativeQuadTreeIndex is None:
            raise RuntimeError("native extension is not installed")
        self._index = _NativeQuadTreeIndex()
        self._values: dict[int, object] = {}
        self._next_id = 0

    @classmethod
    def new(cls) -> "QuadTree":
        return cls()

    @classmethod
    def _from_data(
        cls,
        data: Sequence[object],
        x: Callable[[object], float],
        y: Callable[[object], float],
        *,
        checked: bool,
    ) -> "QuadTree":
        if not callable(x) or not callable(y):
            raise TypeError("x and y must be callable")
        tree = cls()
        points: list[tuple[float, float, int]] = []
        for index, value in enumerate(data):
            point_x = _quad_coordinate(x(value), f"data[{index}].x")
            point_y = _quad_coordinate(y(value), f"data[{index}].y")
            if not math.isfinite(point_x) or not math.isfinite(point_y):
                if checked:
                    raise _quad_error(index, point_x, point_y)
                continue
            identifier = tree._next_id
            tree._next_id += 1
            tree._values[identifier] = value
            points.append((point_x, point_y, identifier))
        tree._index.add_all(points)
        return tree

    @classmethod
    def from_data(
        cls,
        data: Sequence[object],
        x: Callable[[object], float],
        y: Callable[[object], float],
    ) -> "QuadTree":
        return cls._from_data(data, x, y, checked=False)

    @classmethod
    def try_from_data(
        cls,
        data: Sequence[object],
        x: Callable[[object], float],
        y: Callable[[object], float],
    ) -> "QuadTree":
        return cls._from_data(data, x, y, checked=True)

    def cover(self, x: float, y: float) -> None:
        point_x = _quad_coordinate(x, "x")
        point_y = _quad_coordinate(y, "y")
        if math.isfinite(point_x) and math.isfinite(point_y):
            self._index.cover(point_x, point_y)

    def add(self, x: float, y: float, data: object) -> "QuadTree":
        point_x = _quad_coordinate(x, "x")
        point_y = _quad_coordinate(y, "y")
        if not math.isfinite(point_x) or not math.isfinite(point_y):
            return self
        identifier = self._next_id
        self._next_id += 1
        self._index.add(point_x, point_y, identifier)
        self._values[identifier] = data
        return self

    def try_add(self, x: float, y: float, data: object) -> "QuadTree":
        point_x = _quad_coordinate(x, "x")
        point_y = _quad_coordinate(y, "y")
        if not math.isfinite(point_x) or not math.isfinite(point_y):
            raise _quad_error(None, point_x, point_y)
        return self.add(point_x, point_y, data)

    def add_all(
        self,
        data: Sequence[object],
        x: Callable[[object], float],
        y: Callable[[object], float],
    ) -> "QuadTree":
        for value in data:
            self.add(x(value), y(value), value)
        return self

    def try_add_all(
        self,
        data: Sequence[object],
        x: Callable[[object], float],
        y: Callable[[object], float],
    ) -> "QuadTree":
        prepared: list[tuple[float, float, object]] = []
        for index, value in enumerate(data):
            point_x = _quad_coordinate(x(value), f"data[{index}].x")
            point_y = _quad_coordinate(y(value), f"data[{index}].y")
            if not math.isfinite(point_x) or not math.isfinite(point_y):
                raise _quad_error(index, point_x, point_y)
            prepared.append((point_x, point_y, value))
        for point_x, point_y, value in prepared:
            self.add(point_x, point_y, value)
        return self

    def remove(self, x: float, y: float) -> bool:
        point_x = _quad_coordinate(x, "x")
        point_y = _quad_coordinate(y, "y")
        if not math.isfinite(point_x) or not math.isfinite(point_y):
            return False
        identifier = self._index.remove(point_x, point_y)
        if identifier is None:
            return False
        self._values.pop(int(identifier), None)
        return True

    def remove_all(self, predicate: Callable[[object, float, float], bool]) -> int:
        if not callable(predicate):
            raise TypeError("predicate must be callable")
        points = tuple(self._index.data())
        coordinates = [
            (float(x), float(y))
            for x, y, identifier in points
            if predicate(self._values[int(identifier)], float(x), float(y))
        ]
        removed = 0
        for x, y in coordinates:
            removed += int(self.remove(x, y))
        return removed

    def find(self, x: float, y: float, radius: float | None = None) -> object | None:
        identifier = self._index.find(float(x), float(y), radius)
        return None if identifier is None else self._values[int(identifier)]

    def find_all(self, x: float, y: float, radius: float) -> list[object]:
        return [
            self._values[int(identifier)]
            for identifier in self._index.find_all(float(x), float(y), float(radius))
        ]

    def data(self) -> list[tuple[float, float, object]]:
        return [
            (float(x), float(y), self._values[int(identifier)])
            for x, y, identifier in self._index.data()
        ]

    def size(self) -> int:
        return int(self._index.size())

    def is_empty(self) -> bool:
        return bool(self._index.is_empty())

    def extent(self) -> Extent | None:
        value = self._index.extent()
        return None if value is None else Extent(*value)

    def copy(self) -> "QuadTree":
        copied = object.__new__(QuadTree)
        copied._index = self._index.copy()
        copied._values = self._values.copy()
        copied._next_id = self._next_id
        return copied

    def compute_aggregates(self) -> None:
        self._index.compute_aggregates()

    def _decode_node(self, value: tuple[object, ...]) -> tuple[Extent, QuadNode]:
        x0, y0, x1, y1, kind, points, aggregate = value
        linked: QuadPoint | None = None
        for point_x, point_y, identifier in reversed(points):
            linked = QuadPoint(
                float(point_x),
                float(point_y),
                self._values[int(identifier)],
                linked,
            )
        decoded_aggregate = (
            None if aggregate is None else Aggregate(*tuple(aggregate))
        )
        return (
            Extent(float(x0), float(y0), float(x1), float(y1)),
            QuadNode(
                QuadNodeKind.LEAF if int(kind) == 1 else QuadNodeKind.INTERNAL,
                linked,
                decoded_aggregate,
            ),
        )

    def snapshots(self, *, after: bool = False) -> tuple[tuple[Extent, QuadNode], ...]:
        return tuple(
            self._decode_node(tuple(value)) for value in self._index.snapshots(after)
        )

    def visit(
        self, callback: Callable[[float, float, float, float, QuadNode], bool]
    ) -> None:
        if not callable(callback):
            raise TypeError("callback must be callable")

        def bridge(value: tuple[object, ...]) -> bool:
            extent, node = self._decode_node(tuple(value))
            return bool(callback(extent.x0, extent.y0, extent.x1, extent.y1, node))

        self._index.visit(bridge)

    def visit_after(
        self, callback: Callable[[float, float, float, float, QuadNode], object]
    ) -> None:
        if not callable(callback):
            raise TypeError("callback must be callable")

        def bridge(value: tuple[object, ...]) -> None:
            extent, node = self._decode_node(tuple(value))
            callback(extent.x0, extent.y0, extent.x1, extent.y1, node)

        self._index.visit_after(bridge)

    def visit_aggregate(
        self,
        callback: Callable[
            [float, float, float, float, QuadNode, Aggregate | None], bool
        ],
    ) -> None:
        if not callable(callback):
            raise TypeError("callback must be callable")

        def bridge(
            value: tuple[object, ...], aggregate: tuple[float, float, float] | None
        ) -> bool:
            extent, node = self._decode_node(tuple(value))
            decoded = None if aggregate is None else Aggregate(*aggregate)
            return bool(
                callback(extent.x0, extent.y0, extent.x1, extent.y1, node, decoded)
            )

        self._index.visit_aggregate(bridge)


@dataclass(frozen=True)
class GeoPath:
    _projection: Projection
    _digits_value: int = field(default=3, repr=False)
    _point_radius_value: float = field(default=4.5, repr=False)

    def __post_init__(self) -> None:
        if not isinstance(self._projection, Projection):
            raise TypeError("projection must be a Projection")
        if not 0 <= self._digits_value <= 15:
            raise ValueError("digits must be between 0 and 15")
        if not math.isfinite(self._point_radius_value) or self._point_radius_value < 0.0:
            raise ValueError("point_radius must be finite and non-negative")

    @classmethod
    def new(cls, projection: Projection) -> "GeoPath":
        return cls(projection)

    def digits(self, digits: int) -> "GeoPath":
        return replace(self, _digits_value=int(digits))

    def point_radius(self, radius: float) -> "GeoPath":
        return replace(self, _point_radius_value=float(radius))

    @property
    def digits_value(self) -> int:
        return self._digits_value

    @property
    def point_radius_value(self) -> float:
        return self._point_radius_value

    def projection(self) -> Projection:
        return self._projection

    def with_projection(self, projection: Projection) -> "GeoPath":
        return replace(self, _projection=projection)

    def _arguments(self) -> dict[str, object]:
        return {
            "digits": self._digits_value,
            "point_radius": self._point_radius_value,
            **self._projection._arguments(),
        }

    def render(self, geometry: GeoJsonGeometry) -> str:
        if not isinstance(geometry, GeoJsonGeometry):
            raise TypeError("geometry must be a GeoJsonGeometry")
        return str(
            _geo_path_render(
                geometry.kind.value,
                geometry.coordinates,
                self._projection._kind,
                **self._arguments(),
            )
        )

    def render_cow(self, geometry: GeoJsonGeometry) -> str:
        return self.render(geometry)

    def render_into(self, geometry: GeoJsonGeometry, buffer: object) -> None:
        write = getattr(buffer, "write", None)
        if not callable(write):
            raise TypeError("buffer must provide write(str)")
        write(self.render(geometry))

    def project_coords(
        self, coordinates: Sequence[Sequence[float]]
    ) -> tuple[tuple[float, float], ...]:
        points = _geo_line_tuple(coordinates, "coordinates")
        return tuple(
            tuple(point)
            for point in _geo_path_project_coords(
                points,
                self._projection._kind,
                **self._projection._arguments(),
            )
        )

    def bounds(
        self, geometry: GeoJsonGeometry
    ) -> tuple[tuple[float, float], tuple[float, float]]:
        if not isinstance(geometry, GeoJsonGeometry):
            raise TypeError("geometry must be a GeoJsonGeometry")
        bounds = _geo_path_bounds(
            geometry.kind.value,
            geometry.coordinates,
            self._projection._kind,
            **self._arguments(),
        )
        return tuple(bounds[0]), tuple(bounds[1])

    def centroid(self, geometry: GeoJsonGeometry) -> tuple[float, float]:
        if not isinstance(geometry, GeoJsonGeometry):
            raise TypeError("geometry must be a GeoJsonGeometry")
        return tuple(
            _geo_path_centroid(
                geometry.kind.value,
                geometry.coordinates,
                self._projection._kind,
                **self._arguments(),
            )
        )


class HistogramThreshold(str, Enum):
    STURGES = "sturges"
    FREEDMAN_DIACONIS = "freedman_diaconis"
    SCOTT = "scott"
    COUNT = "count"
    VALUES = "values"


@dataclass(frozen=True)
class HistogramBin:
    x0: float
    x1: float
    values: tuple[float, ...]

    @property
    def count(self) -> int:
        return len(self.values)

    def __len__(self) -> int:
        return len(self.values)

    @property
    def is_empty(self) -> bool:
        return not self.values


def histogram(
    data: Sequence[float],
    *,
    strategy: HistogramThreshold = HistogramThreshold.STURGES,
    count: int | None = None,
    thresholds: Sequence[float] | None = None,
    domain: tuple[float, float] | None = None,
) -> list[HistogramBin]:
    """Generate immutable bins with gpui-d3rs threshold semantics."""
    strategy = HistogramThreshold(strategy)
    if count is not None and (not isinstance(count, int) or isinstance(count, bool)):
        raise TypeError("histogram count must be an integer or None")
    result = _histogram(
        data,
        strategy=strategy.value,
        count=count,
        thresholds=thresholds,
        domain=domain,
    )
    return [HistogramBin(x0, x1, tuple(values)) for x0, x1, values in result]

__all__ = [
    "AVAILABLE",
    "abi3_minimum_python",
    "binary_search",
    "bisect_left",
    "bisect_right",
    "color_darken",
    "d3_color_rgb",
    "d3_color_from_hex",
    "d3_color_from_f32",
    "d3_color_from_hsl",
    "d3_color_transform",
    "d3_color_to_hex",
    "d3_color_luminance",
    "d3_color_to_lab",
    "d3_color_to_hcl",
    "d3_lab_create",
    "d3_lab_from_color",
    "d3_lab_to_color",
    "d3_lab_delta_e",
    "d3_lab_chroma",
    "d3_hcl_create",
    "d3_hcl_from_lab",
    "d3_hcl_from_color",
    "d3_hcl_to_lab",
    "d3_hcl_to_color",
    "d3_hcl_interpolate",
    "interpolate_hsl_value_new",
    "interpolate_hsl_value_from_color",
    "interpolate_hsl_value_to_color",
    "interpolate_cubehelix_value_new",
    "interpolate_cubehelix_value_from_color",
    "interpolate_cubehelix_value_to_color",
    "interpolate_cubehelix_default",
    "interpolate_cubehelix_custom",
    "d3_color_scheme",
    "d3_color_scheme_color",
    "d3_interpolate_colors",
    "d3_sequential_color",
    "d3_sequential_scheme_name",
    "d3_sequential_scale_get",
    "d3_sequential_scale_sample",
    "d3_diverging_scheme_name",
    "d3_diverging_scale_get",
    "d3_diverging_scale_sample",
    "color_lighten",
    "color_luminance",
    "clamp01",
    "cross",
    "cumsum",
    "deviation",
    "difference",
    "ease_back_in",
    "ease_back_in_out",
    "ease_back_in_out_with",
    "ease_back_in_with",
    "ease_back_out",
    "ease_back_out_with",
    "ease_bounce_in",
    "ease_bounce_in_out",
    "ease_bounce_out",
    "ease_circle_in",
    "ease_circle_in_out",
    "ease_circle_out",
    "ease_cubic_in",
    "ease_cubic_in_out",
    "ease_cubic_out",
    "ease_elastic_in",
    "ease_elastic_in_out",
    "ease_elastic_in_with",
    "ease_elastic_out",
    "ease_elastic_out_with",
    "ease_exp_in",
    "ease_exp_in_out",
    "ease_exp_out",
    "ease_linear",
    "ease_poly_in",
    "ease_poly_in_out",
    "ease_poly_out",
    "ease_quad_in",
    "ease_quad_in_out",
    "ease_quad_out",
    "ease_sin_in",
    "ease_sin_in_out",
    "ease_sin_out",
    "extent",
    "format",
    "format_locale",
    "format_locale_value",
    "format_prefix",
    "format_value",
    "FormatAlign",
    "FormatSign",
    "FormatSpecifier",
    "FormatType",
    "Locale",
    "DEFAULT_LOCALE",
    "parse_format_specifier",
    "prefix_exponent",
    "TimeInterval",
    "TimeFormat",
    "TimeFormatParts",
    "TimeScale",
    "Transform2D",
    "ZoomParams",
    "ZoomView",
    "zoom_duration",
    "SECOND",
    "MINUTE",
    "HOUR",
    "DAY",
    "WEEK",
    "time_second",
    "time_minute",
    "time_hour",
    "time_day",
    "time_week",
    "time_monday",
    "time_month",
    "time_year",
    "time_format",
    "time_format_value",
    "timestamp_from_millis",
    "millis_from_timestamp",
    "histogram",
    "HistogramBin",
    "HistogramThreshold",
    "Hierarchy",
    "LodErrorKind",
    "LodError",
    "LodBounds",
    "LodDensityGrid",
    "DensityPyramid",
    "m4_indices",
    "m4_point_indices",
    "HierarchyErrorKind",
    "HierarchyError",
    "HierarchyNode",
    "HierarchyNodeSnapshot",
    "HierarchyRect",
    "HierarchyCircle",
    "HierarchyPoint",
    "TreemapLayout",
    "PartitionLayout",
    "PackLayout",
    "TreeLayout",
    "ClusterLayout",
    "Contour",
    "ContourBand",
    "ContourGenerator",
    "ContourRing",
    "ContourRingError",
    "ContourSegment",
    "DensityEstimator",
    "DensityError",
    "DensityGrid",
    "DensityKernel",
    "contour",
    "contours",
    "contour_threshold_freedman_diaconis",
    "contour_threshold_scott",
    "contour_threshold_sturges",
    "density_2d",
    "try_density_2d",
    "epanechnikov_kernel",
    "gaussian_kernel",
    "Delaunay",
    "Voronoi",
    "polygon_area",
    "polygon_area_signed",
    "polygon_centroid",
    "polygon_contains",
    "polygon_hull",
    "polygon_length",
    "SimulationNode",
    "Simulation",
    "ForceCenter",
    "ForceX",
    "ForceY",
    "ForceRadial",
    "ForceCollide",
    "ForceManyBody",
    "ForceLink",
    "ForceError",
    "Point",
    "PathCommandKind",
    "PathCommand",
    "Path",
    "PathBuilder",
    "ShapePath",
    "ShapeGenerationError",
    "ArcDatum",
    "Arc",
    "arc_points",
    "try_arc_points",
    "SymbolType",
    "Symbol",
    "symbol_radius",
    "try_symbol_radius",
    "LinkDirection",
    "Link",
    "RadialLink",
    "link_horizontal",
    "try_link_horizontal",
    "link_vertical",
    "try_link_vertical",
    "link_step",
    "try_link_step",
    "link_radial",
    "try_link_radial",
    "PieSlice",
    "Pie",
    "pie",
    "try_pie",
    "donut",
    "try_donut",
    "half_pie",
    "try_half_pie",
    "StackLayoutError",
    "StackOrder",
    "StackOffset",
    "StackSeries",
    "Stack",
    "stack",
    "try_stack",
    "stack_expand",
    "try_stack_expand",
    "streamgraph",
    "try_streamgraph",
    "CurveKind",
    "Curve",
    "RadialGenerationError",
    "RadialPoint",
    "RadialLineConfig",
    "RadialAreaConfig",
    "radial_line",
    "try_radial_line",
    "radial_area",
    "try_radial_area",
    "polar_grid_circles",
    "try_polar_grid_circles",
    "polar_grid_rays",
    "try_polar_grid_rays",
    "AreaGenerationError",
    "Area",
    "SimpleArea",
    "area_points",
    "try_area_points",
    "ChordLayoutError",
    "ChordSort",
    "ChordSubgroup",
    "ChordGroup",
    "Chord",
    "ChordResult",
    "ChordLayout",
    "RibbonGenerator",
    "LcgRng",
    "RandomUniform",
    "RandomNormal",
    "RandomLogNormal",
    "RandomExponential",
    "RandomBernoulli",
    "RandomPoisson",
    "RandomIrwinHall",
    "RandomBates",
    "HALF_PI",
    "TAU",
    "EPSILON",
    "radians",
    "degrees",
    "geo_distance",
    "geo_length",
    "geo_interpolate",
    "geo_area",
    "geo_bounds",
    "geo_centroid",
    "geo_contains",
    "GraticuleConfig",
    "Graticule",
    "graticule10",
    "Rotation",
    "Versor",
    "Projection",
    "Mercator",
    "Equirectangular",
    "Orthographic",
    "Stereographic",
    "TransverseMercator",
    "ConicEqualArea",
    "Albers",
    "GeoJsonKind",
    "GeoJsonGeometry",
    "GeoStreamEventKind",
    "GeoStreamEvent",
    "GeoStream",
    "geo_stream_events",
    "stream_geojson",
    "TopoJsonError",
    "TopoJsonInvalidError",
    "TopoJsonBudgetError",
    "TopoJsonEmptyLandError",
    "TopoJsonBudget",
    "parse_land",
    "parse_land_with_budget",
    "AutoTypeKind",
    "AutoTyped",
    "auto_type",
    "auto_type_row",
    "auto_type_rows",
    "DsvParseErrorKind",
    "DsvBudgetResource",
    "DsvParseError",
    "DsvBudgetError",
    "DsvCancelledError",
    "DsvBudget",
    "ColumnPolicy",
    "CsvOptions",
    "DsvCancellationToken",
    "DsvParser",
    "parse_dsv",
    "parse_dsv_with_budget",
    "parse_dsv_lossy",
    "try_parse_dsv",
    "parse_csv",
    "parse_csv_with_budget",
    "parse_csv_with_budget_and_cancel",
    "parse_csv_lossy",
    "try_parse_csv",
    "parse_csv_with_options",
    "parse_csv_lossy_with_options",
    "try_parse_csv_with_options",
    "parse_tsv",
    "parse_tsv_with_budget",
    "parse_tsv_with_budget_and_cancel",
    "parse_tsv_lossy",
    "try_parse_tsv",
    "parse_tsv_with_options",
    "parse_tsv_lossy_with_options",
    "try_parse_tsv_with_options",
    "format_csv",
    "format_tsv",
    "MAX_TILE_ZOOM",
    "MAX_VISIBLE_TILES",
    "HexbinBin",
    "HexbinErrorKind",
    "HexbinError",
    "Hexbin",
    "SankeyNode",
    "SankeyLink",
    "SankeyLinkInput",
    "SankeyResult",
    "SankeyNodeAlign",
    "SankeyLinkSort",
    "SankeyLinkSortContext",
    "SankeyLayoutErrorKind",
    "SankeyLayoutError",
    "SankeyLayout",
    "BrushSelection",
    "DomainSelection",
    "BrushState",
    "BrushConfig",
    "ZoomState",
    "ZoomConfig",
    "DragErrorKind",
    "DragError",
    "DragPoint",
    "DragDelta",
    "DragExtent",
    "DragConfig",
    "DragPhase",
    "DragUpdate",
    "DragState",
    "TimerState",
    "TimerCallbackError",
    "Timer",
    "Interval",
    "Timeout",
    "timer",
    "interval",
    "timeout",
    "now",
    "set_now",
    "timer_flush",
    "TransitionEase",
    "TransitionState",
    "TransitionConfig",
    "Transition",
    "TransitionHandle",
    "TransitionManager",
    "Event",
    "ListenerId",
    "Dispatcher",
    "dispatcher",
    "AxisScaleKind",
    "AxisScale",
    "AxisOrientation",
    "AxisPoint",
    "AxisLine",
    "AxisTick",
    "AxisTitle",
    "AxisLayout",
    "AxisLayoutErrorKind",
    "AxisLayoutError",
    "AxisConfig",
    "axis_layout",
    "GridPoint",
    "GridLine",
    "GridDot",
    "GridLayout",
    "GridLayoutErrorKind",
    "GridLayoutError",
    "GridConfig",
    "grid_layout",
    "LegendPosition",
    "LegendOrientation",
    "LegendSymbol",
    "LegendItem",
    "LegendPoint",
    "LegendRect",
    "LegendTitleLayout",
    "LegendItemLayout",
    "LegendLayout",
    "LegendLayoutErrorKind",
    "LegendLayoutError",
    "LegendConfig",
    "legend_layout",
    "legend_from_scale",
    "Tile",
    "TileErrorKind",
    "TileError",
    "TileSet",
    "TileLayout",
    "tiles_for_viewport",
    "Extent",
    "Aggregate",
    "QuadPoint",
    "QuadNodeKind",
    "QuadNode",
    "QuadTreeError",
    "QuadTree",
    "GeoPath",
    "intersection",
    "interpolate_basis",
    "interpolate_basis_closed",
    "interpolate_clamped",
    "interpolate_cubehelix",
    "interpolate_cubehelix_long",
    "interpolate_date",
    "interpolate_discrete",
    "interpolate_ease",
    "interpolate_exp",
    "interpolate_hcl",
    "interpolate_hcl_long",
    "interpolate_hsl",
    "interpolate_hsl_long",
    "interpolate_lab",
    "interpolate_matrix",
    "interpolate_number",
    "interpolate_number_array",
    "interpolate_rgb",
    "interpolate_round",
    "interpolate_string",
    "interpolate_transform_css",
    "interpolate_transform",
    "interpolate_transform_svg",
    "interpolate_quantize",
    "interpolate_zoom_view",
    "interpolate_zoom_vector",
    "is_disjoint",
    "is_subset",
    "is_superset",
    "least_index",
    "linear_scale",
    "linear_scale_invert",
    "linear_scale_nice",
    "linear_scale_ticks",
    "log_scale",
    "log_scale_invert",
    "log_scale_ticks",
    "pow_scale",
    "pow_scale_invert",
    "pow_scale_nice",
    "pow_scale_ticks",
    "symlog_scale",
    "symlog_scale_invert",
    "symlog_scale_nice",
    "symlog_scale_ticks",
    "threshold_scale_index",
    "threshold_scale_invert_extent",
    "quantize_scale_index",
    "quantize_scale_thresholds",
    "quantize_scale_invert_extent",
    "quantile_scale_prepare",
    "quantile_scale_index",
    "quantile_scale_invert_extent",
    "band_scale_layout",
    "point_scale_layout",
    "log_ticks",
    "max",
    "max_index",
    "mean",
    "merge_sorted",
    "median",
    "min",
    "min_index",
    "nice",
    "nice_bin_edges",
    "nice_number",
    "scale_nice_number",
    "generate_linear_ticks",
    "generate_log_ticks",
    "pairs",
    "piecewise",
    "piecewise_domain",
    "quantile",
    "quantile_sorted",
    "quantize",
    "reverse",
    "shuffle",
    "shuffle_seeded",
    "sort",
    "sort_descending",
    "sum",
    "symmetric_difference",
    "tick_increment",
    "tick_step",
    "ticks",
    "ticks_interval",
    "time_ticks",
    "threshold_sturges",
    "union",
    "unique",
    "variance",
]
