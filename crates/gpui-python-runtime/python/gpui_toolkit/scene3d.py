"""Declarative 3D scene helpers for the GPUI Python wrapper."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Iterable, Mapping, Sequence


SCENE3D_SPEC_SCHEMA_VERSION = 1

Point3 = tuple[float, float, float]


def _point3(value: Sequence[float]) -> dict[str, float]:
    if len(value) != 3:
        raise ValueError("3D points must contain exactly three values")
    x, y, z = value
    return {"x": float(x), "y": float(y), "z": float(z)}


def _maybe_point3(value: Sequence[float] | None) -> dict[str, float]:
    return _point3((0.0, 0.0, 0.0) if value is None else value)


def _grid(z: Any) -> tuple[list[float], int, int]:
    if hasattr(z, "shape") and hasattr(z, "tolist"):
        shape = tuple(z.shape)
        if len(shape) != 2:
            raise ValueError("surface z arrays must be two-dimensional")
        rows = z.tolist()
    else:
        rows = z

    if not rows:
        raise ValueError("surface z grid must not be empty")

    height = len(rows)
    width = len(rows[0])
    if width == 0:
        raise ValueError("surface z rows must not be empty")

    values: list[float] = []
    for row in rows:
        if len(row) != width:
            raise ValueError("surface z rows must all have the same length")
        values.extend(float(value) for value in row)

    return values, width, height


def _axis(values: Sequence[float] | None) -> list[float] | None:
    if values is None:
        return None
    return [float(value) for value in values]


def _interactions(values: Iterable[str] | None) -> list[str]:
    if values is None:
        return []
    interactions = []
    for value in values:
        name = str(value).strip().lower().replace("-", "_")
        interactions.append("hit_test" if name == "hittest" else name)
    return interactions


def _colormap(value: str) -> str:
    name = value.strip().lower().replace("-", "_")
    return "cool_warm" if name == "coolwarm" else name


def _size(width: float | None, height: float | None) -> dict[str, float] | None:
    if width is None and height is None:
        return None
    if width is None or height is None:
        raise ValueError("width and height must be provided together")
    return {"width": float(width), "height": float(height)}


def _color(value: str | Point3 | None) -> dict[str, float] | None:
    if value is None:
        return None
    if isinstance(value, str):
        hex_value = value.strip()
        if not hex_value.startswith("#") or len(hex_value) not in (7, 9):
            raise ValueError("colors must use #rrggbb or #rrggbbaa")
        raw = int(hex_value[1:], 16)
        if len(hex_value) == 7:
            r = (raw >> 16) & 0xFF
            g = (raw >> 8) & 0xFF
            b = raw & 0xFF
            a = 0xFF
        else:
            r = (raw >> 24) & 0xFF
            g = (raw >> 16) & 0xFF
            b = (raw >> 8) & 0xFF
            a = raw & 0xFF
        return {
            "r": r / 255.0,
            "g": g / 255.0,
            "b": b / 255.0,
            "a": a / 255.0,
        }
    r, g, b = value
    return {"r": float(r), "g": float(g), "b": float(b), "a": 1.0}


@dataclass(frozen=True)
class OrbitCamera:
    distance: float = 3.5
    azimuth_deg: float = 45.0
    elevation_deg: float = 30.0
    target: Point3 = (0.0, 0.0, 0.0)
    fov_y_deg: float = 45.0
    near: float = 0.1
    far: float = 100.0

    def to_spec(self) -> dict[str, Any]:
        return {
            "kind": "orbit",
            "distance": float(self.distance),
            "azimuth_deg": float(self.azimuth_deg),
            "elevation_deg": float(self.elevation_deg),
            "target": _point3(self.target),
            "fov_y_deg": float(self.fov_y_deg),
            "near": float(self.near),
            "far": float(self.far),
        }


@dataclass(frozen=True)
class PerspectiveCamera:
    position: Point3 = (2.0, 2.0, 2.0)
    target: Point3 = (0.0, 0.0, 0.0)
    up: Point3 = (0.0, 1.0, 0.0)
    fov_y_deg: float = 45.0
    near: float = 0.1
    far: float = 100.0

    def to_spec(self) -> dict[str, Any]:
        return {
            "kind": "perspective",
            "position": _point3(self.position),
            "target": _point3(self.target),
            "up": _point3(self.up),
            "fov_y_deg": float(self.fov_y_deg),
            "near": float(self.near),
            "far": float(self.far),
        }


@dataclass(frozen=True)
class Material:
    color: str | Point3 = "#ffffff"
    opacity: float = 1.0

    def to_spec(self) -> dict[str, Any]:
        return {"color": _color(self.color), "opacity": float(self.opacity)}


@dataclass(frozen=True)
class Surface:
    id: str
    z: Any
    x: Sequence[float] | None = None
    y: Sequence[float] | None = None
    colormap: str = "viridis"
    wireframe: bool = False
    x_log: bool = False
    y_log: bool = False
    z_log: bool = False
    z_range: tuple[float, float] | None = None
    labels: Mapping[str, str] = field(default_factory=dict)
    camera: OrbitCamera | PerspectiveCamera | None = None
    interactions: Iterable[str] | None = None
    width: float | None = None
    height: float | None = None

    def to_spec(self) -> dict[str, Any]:
        values, grid_width, grid_height = _grid(self.z)
        labels = {
            "x": self.labels.get("x"),
            "y": self.labels.get("y"),
            "z": self.labels.get("z"),
            "title": self.labels.get("title"),
        }
        z_range = None
        if self.z_range is not None:
            z_range = {"min": float(self.z_range[0]), "max": float(self.z_range[1])}
        return {
            "schema_version": SCENE3D_SPEC_SCHEMA_VERSION,
            "kind": "surface",
            "id": self.id,
            "z": {"values": values, "width": grid_width, "height": grid_height},
            "x": _axis(self.x),
            "y": _axis(self.y),
            "colormap": _colormap(self.colormap),
            "wireframe": bool(self.wireframe),
            "x_log": bool(self.x_log),
            "y_log": bool(self.y_log),
            "z_log": bool(self.z_log),
            "z_range": z_range,
            "labels": labels,
            "camera": None if self.camera is None else self.camera.to_spec(),
            "interactions": _interactions(self.interactions),
            "size": _size(self.width, self.height),
        }


@dataclass(frozen=True)
class LineStrip:
    id: str
    points: Sequence[Point3]
    color: str | Point3 = "#ffffff"
    width: float = 1.5

    def to_spec(self) -> dict[str, Any]:
        return {
            "id": self.id,
            "points": [_point3(point) for point in self.points],
            "color": _color(self.color),
            "width": float(self.width),
        }


@dataclass(frozen=True)
class Lines:
    id: str
    strips: Sequence[LineStrip] = field(default_factory=list)
    background: str | Point3 | None = None
    camera: OrbitCamera | PerspectiveCamera | None = None
    interactions: Iterable[str] | None = None
    width: float | None = None
    height: float | None = None

    def to_spec(self) -> dict[str, Any]:
        return {
            "schema_version": SCENE3D_SPEC_SCHEMA_VERSION,
            "kind": "lines",
            "id": self.id,
            "strips": [strip.to_spec() for strip in self.strips],
            "segments": [],
            "background": _color(self.background),
            "camera": None if self.camera is None else self.camera.to_spec(),
            "interactions": _interactions(self.interactions),
            "size": _size(self.width, self.height),
        }


@dataclass(frozen=True)
class Mesh:
    id: str
    vertices: Sequence[Point3]
    indices: Sequence[int]
    material: Material = field(default_factory=Material)

    def to_spec(self) -> dict[str, Any]:
        return {
            "schema_version": SCENE3D_SPEC_SCHEMA_VERSION,
            "kind": "mesh",
            "id": self.id,
            "vertices": [_point3(vertex) for vertex in self.vertices],
            "indices": [int(index) for index in self.indices],
            "material": self.material.to_spec(),
        }


@dataclass(frozen=True)
class Light:
    id: str
    direction: Point3
    intensity: float = 1.0
    color: str | Point3 = "#ffffff"

    def to_spec(self) -> dict[str, Any]:
        return {
            "kind": "light",
            "id": self.id,
            "direction": _point3(self.direction),
            "intensity": float(self.intensity),
            "color": _color(self.color),
        }


@dataclass(frozen=True)
class Scene:
    id: str
    children: Sequence[Surface | Lines | Mesh | Light]
    camera: OrbitCamera | PerspectiveCamera = field(default_factory=OrbitCamera)
    interactions: Iterable[str] | None = None
    background: str | Point3 | None = None
    width: float | None = None
    height: float | None = None

    def to_spec(self) -> dict[str, Any]:
        return {
            "schema_version": SCENE3D_SPEC_SCHEMA_VERSION,
            "id": self.id,
            "camera": self.camera.to_spec(),
            "children": [child.to_spec() for child in self.children],
            "interactions": _interactions(self.interactions),
            "background": _color(self.background),
            "size": _size(self.width, self.height),
        }


def orbit(
    distance: float = 3.5,
    azimuth: float = 45.0,
    elevation: float = 30.0,
    target: Sequence[float] | None = None,
    *,
    fov_y: float = 45.0,
    near: float = 0.1,
    far: float = 100.0,
) -> OrbitCamera:
    return OrbitCamera(
        distance=float(distance),
        azimuth_deg=float(azimuth),
        elevation_deg=float(elevation),
        target=tuple(_maybe_point3(target).values()),  # type: ignore[arg-type]
        fov_y_deg=float(fov_y),
        near=float(near),
        far=float(far),
    )


def perspective(
    position: Sequence[float] = (2.0, 2.0, 2.0),
    target: Sequence[float] = (0.0, 0.0, 0.0),
    up: Sequence[float] = (0.0, 1.0, 0.0),
    *,
    fov_y: float = 45.0,
    near: float = 0.1,
    far: float = 100.0,
) -> PerspectiveCamera:
    return PerspectiveCamera(
        position=tuple(_point3(position).values()),  # type: ignore[arg-type]
        target=tuple(_point3(target).values()),  # type: ignore[arg-type]
        up=tuple(_point3(up).values()),  # type: ignore[arg-type]
        fov_y_deg=float(fov_y),
        near=float(near),
        far=float(far),
    )


def surface(id: str, z: Any, **kwargs: Any) -> Surface:
    return Surface(id=id, z=z, **kwargs)


def line_strip(id: str, points: Sequence[Point3], **kwargs: Any) -> LineStrip:
    return LineStrip(id=id, points=points, **kwargs)


def lines(id: str, strips: Sequence[LineStrip], **kwargs: Any) -> Lines:
    return Lines(id=id, strips=strips, **kwargs)


def mesh(
    id: str,
    vertices: Sequence[Point3],
    indices: Sequence[int],
    **kwargs: Any,
) -> Mesh:
    return Mesh(id=id, vertices=vertices, indices=indices, **kwargs)


def material(color: str | Point3 = "#ffffff", opacity: float = 1.0) -> Material:
    return Material(color=color, opacity=opacity)


def light(id: str, direction: Point3, **kwargs: Any) -> Light:
    return Light(id=id, direction=direction, **kwargs)


def scene(id: str, children: Sequence[Surface | Lines | Mesh | Light], **kwargs: Any) -> Scene:
    return Scene(id=id, children=children, **kwargs)
