"""Declarative unstructured-mesh plot protocol helpers."""
from __future__ import annotations

from dataclasses import dataclass
import json
from math import isfinite, isnan, tau
from typing import Any, Sequence

from .resources import Resource

MESHPLOT_SCHEMA_VERSION = 1
MAX_INLINE_MESH_BYTES = 256 * 1024
_DEFAULT_INTERACTIONS = ("pan", "zoom", "inspect", "select", "reset", "fit")


def _array(value: Sequence[Any]) -> list[Any]:
    return [list(item) if isinstance(item, (tuple, list)) else item for item in value]


def _resource_handle(value: Any, name: str) -> None:
    if not isinstance(value, dict) or not str(value.get("resource_id", value.get("id", ""))).strip():
        raise ValueError(f"{name} resource handle requires a non-empty resource_id")
    generation = value.get("generation")
    if not isinstance(generation, int) or isinstance(generation, bool) or generation <= 0:
        raise ValueError(f"{name} resource generation must be positive")


def _normalize_axes(value: dict[str, Any] | None) -> dict[str, Any] | None:
    if value is None:
        return None
    if not isinstance(value, dict):
        raise ValueError("mesh plot axes must be a mapping")
    allowed = {
        "horizontal_label",
        "vertical_label",
        "unit",
        "x_range",
        "y_range",
        "show_grid",
    }
    unknown = set(value) - allowed
    if unknown:
        raise ValueError(f"mesh plot axes contain unsupported properties: {sorted(unknown)!r}")
    normalized: dict[str, Any] = {}
    for name in ("horizontal_label", "vertical_label", "unit"):
        if name in value:
            if not isinstance(value[name], str):
                raise ValueError(f"mesh plot axes {name} must be a string")
            normalized[name] = value[name]
    for name in ("x_range", "y_range"):
        if name not in value:
            continue
        range_value = value[name]
        if (
            not isinstance(range_value, (list, tuple))
            or len(range_value) != 2
            or any(
                isinstance(item, bool)
                or not isinstance(item, (int, float))
                or not isfinite(float(item))
                for item in range_value
            )
            or float(range_value[0]) >= float(range_value[1])
        ):
            raise ValueError(f"mesh plot axes {name} must contain two increasing finite values")
        normalized[name] = [float(range_value[0]), float(range_value[1])]
    if "show_grid" in value:
        if not isinstance(value["show_grid"], bool):
            raise ValueError("mesh plot axes show_grid must be boolean")
        normalized["show_grid"] = value["show_grid"]
    return normalized


def _normalize_colorbar(value: dict[str, Any] | None) -> dict[str, Any] | None:
    if value is None:
        return None
    if not isinstance(value, dict):
        raise ValueError("mesh plot colorbar must be a mapping")
    allowed = {"label", "unit", "scale", "range", "ticks", "orientation"}
    if set(value) - allowed:
        raise ValueError("mesh plot colorbar contains unsupported properties")
    label = value.get("label")
    if not isinstance(label, str) or not label.strip():
        raise ValueError("mesh plot colorbar label must be a non-empty string")
    unit = value.get("unit")
    if unit is not None and not isinstance(unit, str):
        raise ValueError("mesh plot colorbar unit must be a string or null")
    scale = value.get("scale", "viridis")
    if scale not in {"viridis", "plasma", "inferno", "magma", "heat", "coolwarm", "greys"}:
        raise ValueError("mesh plot colorbar scale is unsupported")
    orientation = value.get("orientation", "vertical")
    if orientation not in {"vertical", "horizontal"}:
        raise ValueError("mesh plot colorbar orientation is unsupported")
    ticks = value.get("ticks")
    if ticks is not None:
        if not isinstance(ticks, (list, tuple)) or not ticks:
            raise ValueError("mesh plot colorbar ticks must be a non-empty sequence")
        ticks = [float(item) for item in ticks]
        if not all(isfinite(item) for item in ticks) or any(a >= b for a, b in zip(ticks, ticks[1:])):
            raise ValueError("mesh plot colorbar ticks must be finite and increasing")
    return {
        "label": label,
        "unit": unit,
        "scale": scale,
        "range": value.get("range", "auto"),
        "ticks": ticks,
        "orientation": orientation,
    }


def _validate_inline_geometry(
    positions: Sequence[Sequence[float]],
    triangles: Sequence[Sequence[int]],
    vertex_ids: Sequence[int] | None,
    cell_ids: Sequence[int] | None,
) -> None:
    if not positions:
        raise ValueError("mesh geometry positions must not be empty")
    for index, point in enumerate(positions):
        if len(point) != 3 or any(not isinstance(value, (int, float)) or not isfinite(float(value)) for value in point):
            raise ValueError(f"mesh geometry position {index} must contain three finite numbers")
    for index, triangle in enumerate(triangles):
        if len(triangle) != 3 or any(not isinstance(value, int) or isinstance(value, bool) for value in triangle):
            raise ValueError(f"mesh geometry triangle {index} must contain three integer indices")
        if any(value < 0 or value >= len(positions) for value in triangle):
            raise ValueError(f"mesh geometry triangle {index} references an invalid vertex")
    for name, values, expected in (("vertex_ids", vertex_ids, len(positions)), ("cell_ids", cell_ids, len(triangles))):
        if values is not None and (len(values) != expected or any(not isinstance(value, int) or isinstance(value, bool) for value in values)):
            raise ValueError(f"mesh geometry {name} must contain {expected} integer ids")


@dataclass(frozen=True)
class MeshGeometry:
    positions: Sequence[Sequence[float]]
    triangles: Sequence[Sequence[int]]
    id: str = "mesh"
    vertex_ids: Sequence[int] | None = None
    cell_ids: Sequence[int] | None = None
    resource_id: str | None = None
    generation: int | None = None
    positions_resource_id: str | None = None
    positions_generation: int | None = None
    positions_shape: tuple[int, ...] | None = None
    positions_dtype: str | None = None
    triangles_resource_id: str | None = None
    triangles_generation: int | None = None
    triangles_shape: tuple[int, ...] | None = None
    triangles_dtype: str | None = None
    vertex_ids_resource_id: str | None = None
    vertex_ids_generation: int | None = None
    vertex_ids_shape: tuple[int, ...] | None = None
    vertex_ids_dtype: str | None = None
    cell_ids_resource_id: str | None = None
    cell_ids_generation: int | None = None
    cell_ids_shape: tuple[int, ...] | None = None
    cell_ids_dtype: str | None = None

    def to_spec(self) -> dict[str, Any]:
        if not self.id.strip():
            raise ValueError("mesh geometry id must not be empty")
        if self.resource_id is not None:
            raise ValueError(
                "whole-geometry resource handles are unsupported; provide "
                "positions_resource_id and triangles_resource_id"
            )
        if self.positions_resource_id is not None or self.triangles_resource_id is not None:
            if (
                not self.positions_resource_id
                or not self.triangles_resource_id
                or self.positions_generation is None
                or self.positions_generation <= 0
                or self.triangles_generation is None
                or self.triangles_generation <= 0
            ):
                raise ValueError("resource-backed geometry requires positions and triangles resources")
            spec: dict[str, Any] = {
                "id": self.id,
                "positions": {
                    "resource_id": self.positions_resource_id,
                    "generation": self.positions_generation,
                    "dtype": self.positions_dtype or "f64le",
                },
                "triangles": {
                    "resource_id": self.triangles_resource_id,
                    "generation": self.triangles_generation,
                    "dtype": self.triangles_dtype or "u32le",
                },
            }
            if self.positions_shape is not None:
                spec["positions"].update(
                    {"kind": "array_data", "shape": list(self.positions_shape)}
                )
            if self.triangles_shape is not None:
                spec["triangles"].update(
                    {"kind": "array_data", "shape": list(self.triangles_shape)}
                )
        else:
            _validate_inline_geometry(self.positions, self.triangles, self.vertex_ids, self.cell_ids)
            spec = {
                "id": self.id, "positions": _array(self.positions),
                "triangles": _array(self.triangles), "dtype": "f64le",
            }
        if self.vertex_ids is not None:
            spec["vertex_ids"] = list(self.vertex_ids)
        elif self.vertex_ids_resource_id is not None:
            if not self.vertex_ids_resource_id.strip() or self.vertex_ids_generation is None or self.vertex_ids_generation <= 0:
                raise ValueError("resource-backed vertex ids require a positive generation")
            spec["vertex_ids"] = {
                "resource_id": self.vertex_ids_resource_id,
                "generation": self.vertex_ids_generation,
                "dtype": self.vertex_ids_dtype or "u64le",
            }
            if self.vertex_ids_shape is not None:
                spec["vertex_ids"].update(
                    {"kind": "array_data", "shape": list(self.vertex_ids_shape)}
                )
            _resource_handle(spec["vertex_ids"], "mesh geometry vertex_ids")
        if self.cell_ids is not None:
            spec["cell_ids"] = list(self.cell_ids)
        elif self.cell_ids_resource_id is not None:
            if not self.cell_ids_resource_id.strip() or self.cell_ids_generation is None or self.cell_ids_generation <= 0:
                raise ValueError("resource-backed cell ids require a positive generation")
            spec["cell_ids"] = {
                "resource_id": self.cell_ids_resource_id,
                "generation": self.cell_ids_generation,
                "dtype": self.cell_ids_dtype or "u64le",
            }
            if self.cell_ids_shape is not None:
                spec["cell_ids"].update(
                    {"kind": "array_data", "shape": list(self.cell_ids_shape)}
                )
            _resource_handle(spec["cell_ids"], "mesh geometry cell_ids")
        return spec


@dataclass(frozen=True)
class MeshScalarField:
    values: Sequence[float]
    association: str = "vertex"
    id: str = "field"
    label: str | None = None
    unit: str | None = None
    valid: Sequence[bool] | None = None
    resource_id: str | None = None
    generation: int | None = None
    shape: tuple[int, ...] | None = None
    dtype: str | None = None
    valid_resource_id: str | None = None
    valid_generation: int | None = None
    valid_shape: tuple[int, ...] | None = None
    valid_dtype: str | None = None

    def to_spec(self, missing_value_policy: str = "reject") -> dict[str, Any]:
        if not self.id.strip():
            raise ValueError("mesh field id must not be empty")
        valid: list[bool] | None = None
        if self.resource_id is not None:
            if not self.resource_id.strip() or self.generation is None or self.generation <= 0:
                raise ValueError("resource-backed field requires a positive generation")
            spec: dict[str, Any] = {
                "id": self.id, "resource_id": self.resource_id,
                "generation": self.generation, "association": self.association,
                "dtype": self.dtype or "f64le",
            }
            if self.shape is not None:
                spec.update({"kind": "array_data", "shape": list(self.shape)})
            _resource_handle(spec, "mesh field")
        else:
            values = list(self.values)
            if any(
                not isinstance(value, (int, float))
                or (not isfinite(float(value)) and not isnan(float(value)))
                for value in values
            ):
                raise ValueError("mesh field values must be finite numbers or NaN")
            valid = None if self.valid is None else list(self.valid)
            if valid is not None and (
                len(valid) != len(values)
                or any(not isinstance(value, bool) for value in valid)
            ):
                raise ValueError("mesh field valid mask must match values and contain booleans")
            for index, value in enumerate(values):
                if not isnan(float(value)):
                    continue
                explicitly_masked = valid is not None and not valid[index]
                if missing_value_policy != "mask_nan" and not explicitly_masked:
                    raise ValueError(
                        "NaN mesh field values require an explicit false validity entry "
                        "or missing_value_policy='mask_nan'"
                    )
                if valid is None:
                    valid = [True] * len(values)
                valid[index] = False
                values[index] = 0.0
            spec = {"id": self.id, "values": values, "dtype": "f64le", "association": self.association}
        if self.label is not None: spec["label"] = self.label
        if self.unit is not None: spec["unit"] = self.unit
        if self.resource_id is None and valid is not None:
            spec["valid"] = valid
        elif self.valid is not None:
            if len(self.valid) != len(self.values) or any(not isinstance(value, bool) for value in self.valid):
                raise ValueError("mesh field valid mask must match values and contain booleans")
            spec["valid"] = list(self.valid)
        elif self.valid_resource_id is not None:
            if not self.valid_resource_id.strip() or self.valid_generation is None or self.valid_generation <= 0:
                raise ValueError("resource-backed field mask requires a positive generation")
            spec["valid"] = {
                "resource_id": self.valid_resource_id,
                "generation": self.valid_generation,
                "dtype": self.valid_dtype or "bool_bytes",
            }
            if self.valid_shape is not None:
                spec["valid"].update(
                    {"kind": "array_data", "shape": list(self.valid_shape)}
                )
            _resource_handle(spec["valid"], "mesh field valid mask")
        return spec


@dataclass(frozen=True)
class MeshRevolve:
    radial: str = "x"
    axial: str = "z"
    start_angle: float = 0.0
    sweep_angle: float = tau
    segments: int = 64
    end_caps: bool = False

    def to_spec(self) -> dict[str, Any]:
        if self.radial not in {"x", "y", "z"} or self.axial not in {"x", "y", "z"}:
            raise ValueError("revolve radial and axial axes must be 'x', 'y', or 'z'")
        if self.radial == self.axial:
            raise ValueError("revolve radial and axial axes must be distinct")
        if not isfinite(float(self.start_angle)):
            raise ValueError("revolve start_angle must be finite")
        if not isfinite(float(self.sweep_angle)) or not 0.0 < float(self.sweep_angle) <= tau:
            raise ValueError("revolve sweep_angle must be in (0, 2*pi]")
        if not isinstance(self.segments, int) or isinstance(self.segments, bool) or self.segments < 3:
            raise ValueError("revolve segments must be an integer of at least 3")
        if not isinstance(self.end_caps, bool):
            raise ValueError("revolve end_caps must be boolean")
        return {
            "radial": self.radial,
            "axial": self.axial,
            "start_angle": float(self.start_angle),
            "sweep_angle": float(self.sweep_angle),
            "segments": self.segments,
            "end_caps": self.end_caps,
        }


@dataclass(frozen=True)
class MeshPlotSpec:
    geometry: MeshGeometry
    id: str = "mesh_plot"
    revision: int = 0
    field: MeshScalarField | None = None
    view: str = "planar"
    mode: str = "mesh"
    color_scale: str = "viridis"
    # ``{"symmetric": {"center": 0.0, "extent": "auto"}}`` or a
    # positive numeric extent mirrors Rust's ColorRange::Symmetric.
    color_range: str | tuple[float, float] | dict[str, Any] = "auto"
    # ``mask_nan`` converts NaN field samples into explicit invalid-mask
    # entries; ``reject`` preserves the strict default validation.
    missing_value_policy: str = "reject"
    wireframe: bool = True
    title: str | None = None
    width: float | None = None
    height: float | None = None
    fill: bool = False
    min_width: float | None = None
    min_height: float | None = None
    aspect_ratio: float | None = None
    selection: dict[str, Any] | None = None
    camera: dict[str, Any] | None = None
    viewport: dict[str, Any] | None = None
    contour_levels: dict[str, Any] | None = None
    equal_aspect: bool = True
    axes: dict[str, Any] | None = None
    interactions: Sequence[str] = _DEFAULT_INTERACTIONS
    toolbar: bool = True
    hidden_toolbar_actions: Sequence[str] = ()
    colorbar: dict[str, Any] | None = None
    renderer_backend: str = "auto"
    revolve: MeshRevolve | None = None

    def to_spec(self) -> dict[str, Any]:
        if not self.id.strip():
            raise ValueError("mesh plot id must not be empty")
        if self.view not in {"planar", "axisymmetric_section", "axisymmetric_revolve", "surface3d"}:
            raise ValueError(f"unsupported mesh plot view: {self.view!r}")
        if self.mode not in {"mesh", "scalar_fill", "filled_contours", "isolines", "fill_and_isolines"}:
            raise ValueError(f"unsupported mesh plot mode: {self.mode!r}")
        if self.color_scale not in {"viridis", "plasma", "inferno", "magma", "cividis", "turbo", "coolwarm", "cool_warm"}:
            raise ValueError(f"unsupported mesh plot color scale: {self.color_scale!r}")
        if self.mode != "mesh" and self.field is None:
            raise ValueError(f"mesh plot mode {self.mode!r} requires a scalar field")
        if self.missing_value_policy not in {"reject", "mask_nan"}:
            raise ValueError("missing_value_policy must be 'reject' or 'mask_nan'")
        geometry = self.geometry.to_spec()
        field = None if self.field is None else self.field.to_spec(self.missing_value_policy)
        if field is not None and "values" in field:
            expected = len(geometry["positions"]) if field.get("association", "vertex") == "vertex" else len(geometry["triangles"])
            if len(field["values"]) != expected:
                raise ValueError(f"mesh {field.get('association', 'vertex')} field must contain {expected} values")
            if self.mode in {"filled_contours", "isolines", "fill_and_isolines"} and field.get("association", "vertex") != "vertex":
                raise ValueError("mesh plot contours require a vertex field")
        if self.color_range != "auto":
            if isinstance(self.color_range, tuple):
                if len(self.color_range) != 2 or not all(float(value) == float(value) and abs(float(value)) < float("inf") for value in self.color_range) or self.color_range[0] >= self.color_range[1]:
                    raise ValueError("color_range must be increasing finite values")
            elif isinstance(self.color_range, dict):
                symmetric = self.color_range.get("symmetric")
                if set(self.color_range) != {"symmetric"} or not isinstance(symmetric, dict) or set(symmetric) != {"center", "extent"}:
                    raise ValueError("symmetric color_range must be {'symmetric': {'center': number, 'extent': 'auto' or positive number}}")
                center, extent = symmetric["center"], symmetric["extent"]
                if not isinstance(center, (int, float)) or not isfinite(float(center)):
                    raise ValueError("symmetric color_range center must be finite")
                if extent != "auto" and (not isinstance(extent, (int, float)) or not isfinite(float(extent)) or float(extent) <= 0):
                    raise ValueError("symmetric color_range extent must be 'auto' or a positive finite number")
            else:
                raise ValueError("color_range must be 'auto', a (min, max) tuple, or a symmetric range mapping")
        if not isinstance(self.revision, int) or self.revision < 0:
            raise ValueError("revision must be a non-negative integer")
        if (self.width is None) != (self.height is None):
            raise ValueError("mesh plot fixed size requires both width and height")
        for name, value in (
            ("width", self.width), ("height", self.height),
            ("min_width", self.min_width), ("min_height", self.min_height),
            ("aspect_ratio", self.aspect_ratio),
        ):
            if value is not None and (not isfinite(float(value)) or float(value) <= 0.0):
                raise ValueError(f"mesh plot {name} must be finite and positive")
        if (self.min_width is None) != (self.min_height is None):
            raise ValueError("mesh plot minimum size requires both dimensions")
        if not isinstance(self.fill, bool):
            raise TypeError("mesh plot fill must be bool")
        if self.fill and self.width is not None:
            raise ValueError("mesh plot fill and fixed size are mutually exclusive")
        if self.revolve is not None and self.view != "axisymmetric_revolve":
            raise ValueError("revolve settings require view='axisymmetric_revolve'")
        if self.contour_levels is not None:
            if not isinstance(self.contour_levels, dict):
                raise ValueError("contour_levels must be a mapping")
            count = self.contour_levels.get("count")
            values = self.contour_levels.get("values")
            if count is not None and (not isinstance(count, int) or isinstance(count, bool) or count <= 0):
                raise ValueError("contour_levels.count must be positive")
            if values is not None and (len(values) < 2 or any(not isfinite(float(value)) for value in values) or any(values[index] >= values[index + 1] for index in range(len(values) - 1))):
                raise ValueError("contour_levels.values must be increasing finite values")
            if count is None and values is None:
                raise ValueError("contour_levels requires count or values")
        allowed_interactions = {"pan", "zoom", "inspect", "select", "reset", "fit"}
        if any(interaction not in allowed_interactions for interaction in self.interactions) or len(set(self.interactions)) != len(self.interactions):
            raise ValueError("mesh plot interactions contain an unsupported or duplicate value")
        if not isinstance(self.toolbar, bool):
            raise TypeError("mesh plot toolbar must be bool")
        if self.renderer_backend not in {"auto", "wgpu"}:
            raise ValueError(
                f"unsupported mesh plot renderer backend {self.renderer_backend!r}"
            )
        allowed_toolbar_actions = {
            "fit", "reset", "open_mode_menu", "toggle_wireframe",
            "reset_color_range", "open_view_menu", "export",
        }
        hidden_toolbar_actions = list(self.hidden_toolbar_actions)
        if any(action not in allowed_toolbar_actions for action in hidden_toolbar_actions) or len(
            set(hidden_toolbar_actions)
        ) != len(hidden_toolbar_actions):
            raise ValueError("mesh plot hidden toolbar actions are unsupported or duplicated")
        interactions = list(self.interactions)
        axes = _normalize_axes(self.axes)
        colorbar = _normalize_colorbar(self.colorbar)
        spec = {"kind": "mesh_plot", "schema_version": MESHPLOT_SCHEMA_VERSION, "id": self.id, "revision": self.revision, "geometry": geometry, "field": field, "view": self.view, "mode": self.mode, "color_scale": self.color_scale, "color_range": list(self.color_range) if isinstance(self.color_range, tuple) else self.color_range, "missing_value_policy": self.missing_value_policy, "wireframe": self.wireframe, "title": self.title, "width": self.width, "height": self.height, "selection": self.selection, "camera": self.camera, "viewport": self.viewport, "contour_levels": self.contour_levels, "equal_aspect": self.equal_aspect, "axes": axes, "interactions": interactions}
        spec["toolbar"] = self.toolbar
        spec["hidden_toolbar_actions"] = hidden_toolbar_actions
        spec["renderer_backend"] = self.renderer_backend
        spec["colorbar"] = colorbar
        spec["fill"] = self.fill
        spec["min_width"] = self.min_width
        spec["min_height"] = self.min_height
        spec["aspect_ratio"] = self.aspect_ratio
        if self.view == "axisymmetric_revolve":
            spec["revolve"] = (self.revolve or MeshRevolve()).to_spec()
        if "positions" in geometry and len(json.dumps(spec, separators=(",", ":")).encode("utf-8")) > MAX_INLINE_MESH_BYTES:
            raise ValueError(
                "inline mesh plot payload exceeds 256 KiB; use ResourceStore mesh handles"
            )
        return spec


def geometry(positions: Sequence[Sequence[float]], triangles: Sequence[Sequence[int]], *, id: str = "mesh", vertex_ids: Sequence[int] | None = None, cell_ids: Sequence[int] | None = None, resource_id: str | None = None, generation: int | None = None) -> MeshGeometry:
    return MeshGeometry(positions, triangles, id, vertex_ids, cell_ids, resource_id, generation)


def resource_geometry(resource_id: str, generation: int, *, id: str = "mesh", vertex_ids: Sequence[int] | None = None, cell_ids: Sequence[int] | None = None, triangles_resource_id: str | None = None, triangles_generation: int | None = None, vertex_ids_resource_id: str | None = None, vertex_ids_generation: int | None = None, cell_ids_resource_id: str | None = None, cell_ids_generation: int | None = None) -> MeshGeometry:
    """Reference geometry sent through :class:`ResourceStore` mesh frames.

    Native rendering requires explicit position and triangle resources because
    positions and indices have different portable dtypes. Prefer
    :func:`resource_geometry_from_resources` for resources returned by
    :class:`ResourceStore`.
    """
    if triangles_resource_id is None:
        raise ValueError(
            "resource_geometry requires triangles_resource_id and "
            "triangles_generation; use resource_geometry_from_resources"
        )
    return MeshGeometry(
        positions=(), triangles=(), id=id, vertex_ids=vertex_ids, cell_ids=cell_ids,
        positions_resource_id=resource_id, positions_generation=generation,
        triangles_resource_id=triangles_resource_id,
        triangles_generation=triangles_generation,
        vertex_ids_resource_id=vertex_ids_resource_id,
        vertex_ids_generation=vertex_ids_generation,
        cell_ids_resource_id=cell_ids_resource_id,
        cell_ids_generation=cell_ids_generation,
    )


def resource_geometry_from_resources(
    positions: Resource,
    triangles: Resource,
    *,
    id: str = "mesh",
    vertex_ids: Sequence[int] | None = None,
    cell_ids: Sequence[int] | None = None,
    vertex_ids_resource: Resource | None = None,
    cell_ids_resource: Resource | None = None,
) -> MeshGeometry:
    """Create geometry backed by separate position and index resources."""
    return resource_geometry(
        positions.id,
        positions.generation,
        id=id,
        vertex_ids=vertex_ids,
        cell_ids=cell_ids,
        triangles_resource_id=triangles.id,
        triangles_generation=triangles.generation,
        vertex_ids_resource_id=None if vertex_ids_resource is None else vertex_ids_resource.id,
        vertex_ids_generation=None if vertex_ids_resource is None else vertex_ids_resource.generation,
        cell_ids_resource_id=None if cell_ids_resource is None else cell_ids_resource.id,
        cell_ids_generation=None if cell_ids_resource is None else cell_ids_resource.generation,
    )


def scalar_field(values: Sequence[float], *, association: str = "vertex", id: str = "field", label: str | None = None, unit: str | None = None, valid: Sequence[bool] | None = None) -> MeshScalarField:
    if association not in {"vertex", "cell"}: raise ValueError("association must be 'vertex' or 'cell'")
    if not id.strip(): raise ValueError("mesh field id must not be empty")
    values = tuple(values)
    if any(
        not isinstance(value, (int, float))
        or (not isfinite(float(value)) and not isnan(float(value)))
        for value in values
    ):
        raise ValueError("mesh field values must be finite numbers or NaN")
    if valid is not None and len(valid) != len(values): raise ValueError("mesh field valid mask length must match values")
    return MeshScalarField(values, association, id, label, unit, valid)


def revolve(*, radial: str = "x", axial: str = "z", start_angle: float = 0.0, sweep_angle: float = tau, segments: int = 64, end_caps: bool = False) -> MeshRevolve:
    return MeshRevolve(radial, axial, start_angle, sweep_angle, segments, end_caps)


def resource_field(resource_id: str, generation: int, *, association: str = "vertex", id: str = "field", label: str | None = None, unit: str | None = None, valid_resource_id: str | None = None, valid_generation: int | None = None) -> MeshScalarField:
    """Reference a scalar field sent through :class:`ResourceStore` frames."""
    if association not in {"vertex", "cell"}:
        raise ValueError("association must be 'vertex' or 'cell'")
    return MeshScalarField(
        values=(), association=association, id=id, label=label, unit=unit,
        resource_id=resource_id, generation=generation,
        valid_resource_id=valid_resource_id, valid_generation=valid_generation,
    )


def plot(
    mesh: MeshGeometry | None = None,
    field: MeshScalarField | None = None,
    *,
    id: str = "mesh_plot",
    geometry: MeshGeometry | None = None,
    revision: int = 0,
    view: str = "planar",
    mode: str = "mesh",
    color_scale: str = "viridis",
    color_range: str | tuple[float, float] | dict[str, Any] = "auto",
    missing_value_policy: str = "reject",
    wireframe: bool = True,
    title: str | None = None,
    width: float | None = None,
    height: float | None = None,
    selection: dict[str, Any] | None = None,
    camera: dict[str, Any] | None = None,
    viewport: dict[str, Any] | None = None,
    contour_levels: dict[str, Any] | None = None,
    equal_aspect: bool = True,
    axes: dict[str, Any] | None = None,
    interactions: Sequence[str] = _DEFAULT_INTERACTIONS,
    revolve: MeshRevolve | None = None,
) -> MeshPlotSpec:
    """Build a plot; ``geometry=`` is accepted alongside the short positional form."""
    geometry = geometry or mesh
    if geometry is None:
        raise TypeError("meshplot.plot requires geometry")
    return MeshPlotSpec(
        geometry=geometry,
        id=geometry.id if id == "mesh_plot" else id,
        field=field,
        revision=revision,
        view=view,
        mode=mode,
        color_scale=color_scale,
        color_range=color_range,
        missing_value_policy=missing_value_policy,
        wireframe=wireframe,
        title=title,
        width=width,
        height=height,
        selection=selection,
        camera=camera,
        viewport=viewport,
        contour_levels=contour_levels,
        equal_aspect=equal_aspect,
        axes=axes,
        interactions=interactions,
        revolve=revolve,
    )
