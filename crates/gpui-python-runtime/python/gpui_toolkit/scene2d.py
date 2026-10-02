"""Typed declarations for small retained GPUI drawing surfaces.

Scene2D keeps game and diagram geometry as ordinary JSON values.  A surface is
owned and painted by the native host; Python supplies a complete initial scene
and can later publish revision checked object patches.
"""
from __future__ import annotations

from dataclasses import dataclass, field
import math
import re
import struct
from typing import Any, Mapping, Sequence


_UNSET = object()
_MAX_SCENE_NODES = 65_536
_MAX_GROUP_DEPTH = 64
_MAX_DIMENSION = 1_000_000.0
_MAX_TRANSFORM_SCALE = 1_000.0
_MAX_TRANSFORM_ROTATION = 36_000.0
_MAX_TRANSFORM_SHEAR = 1_000_000.0
_F32_MAX = 3.4028234663852886e38
_F32_EPSILON = 1.1920928955078125e-7


def _node_specs(nodes: Sequence[Any]) -> list[dict[str, Any]]:
    return [node.to_spec() for node in nodes]


def _walk_node_ids(nodes: Sequence[Mapping[str, Any]]) -> list[tuple[str, int]]:
    result: list[tuple[str, int]] = []

    def visit(
        items: Sequence[Mapping[str, Any]],
        group_depth: int,
        parent_transform: tuple[float, float, float, float, float, float],
    ) -> None:
        for node in items:
            if not isinstance(node, Mapping):
                raise ValueError("Scene2D nodes must be mappings")
            node_id = node.get("id")
            if not isinstance(node_id, str) or not node_id.strip():
                raise ValueError("Scene2D node id must not be empty")
            result.append((node_id, group_depth))
            kind = node.get("kind")
            if not isinstance(kind, Mapping):
                raise ValueError(f"Scene2D node {node_id!r} must have a typed kind")
            local_transform = _transform_from_spec(node.get("transform", _UNSET))
            composed_transform = _compose_transform(parent_transform, local_transform)
            for field_name in ("hit_bounds", "clip"):
                bounds = node.get(field_name)
                if bounds is not None:
                    _check_transformed_bounds(
                        _rect_from_spec(bounds, f"Scene2D {field_name}"),
                        composed_transform,
                        f"Scene2D transformed {field_name}",
                    )
            geometry = _kind_bounds(kind, node_id)
            if geometry is not None:
                _check_transformed_bounds(
                    geometry, composed_transform, "Scene2D transformed geometry"
                )
            if kind.get("type") == "group":
                if group_depth >= _MAX_GROUP_DEPTH:
                    raise ValueError(f"Scene2D group nesting exceeds {_MAX_GROUP_DEPTH}")
                children = kind.get("children", ())
                if not isinstance(children, Sequence) or isinstance(children, (str, bytes)):
                    raise ValueError("Scene2D group children must be a sequence")
                visit(children, group_depth + 1, composed_transform)
            if len(result) > _MAX_SCENE_NODES:
                raise ValueError(f"Scene2D node count exceeds {_MAX_SCENE_NODES}")

    visit(nodes, 0, (0.0, 0.0, 1.0, 1.0, 0.0, 0.0))
    return result


def _number(value: float, name: str, *, minimum: float | None = None) -> float:
    if isinstance(value, bool):
        raise TypeError(f"{name} must be a finite number")
    try:
        result = float(value)
    except (TypeError, ValueError, OverflowError) as error:
        raise TypeError(f"{name} must be a finite number") from error
    if not math.isfinite(result) or abs(result) > _F32_MAX:
        raise ValueError(f"{name} must fit in a finite f32")
    if minimum is not None and result < minimum:
        bound = "finite and non-negative" if minimum == 0 else "finite and positive"
        raise ValueError(f"{name} must be {bound}")
    return result


def _f32(value: Any, name: str) -> float:
    number = _number(value, name)
    try:
        rounded = struct.unpack("=f", struct.pack("=f", number))[0]
    except (OverflowError, struct.error) as error:
        raise ValueError(f"{name} must fit in a finite f32") from error
    if not math.isfinite(rounded):
        raise ValueError(f"{name} must fit in a finite f32")
    return rounded


def _f32_add(left: float, right: float, name: str) -> float:
    return _f32(left + right, name)


def _f32_sub(left: float, right: float, name: str) -> float:
    return _f32(left - right, name)


def _f32_mul(left: float, right: float, name: str) -> float:
    return _f32(left * right, name)


def _f32_div(left: float, right: float, name: str) -> float:
    if right == 0.0:
        raise ValueError(f"{name} must fit in a finite f32")
    return _f32(left / right, name)


def _transform_from_spec(value: Any) -> tuple[float, float, float, float, float, float]:
    if value is _UNSET:
        return (0.0, 0.0, 1.0, 1.0, 0.0, 0.0)
    if value is None:
        # Typed constructors omit identity transforms. Explicit null is not a
        # valid serde representation of the runtime's non-optional struct.
        raise ValueError("Scene2D transform must be an object")
    if not isinstance(value, Mapping):
        raise ValueError("Scene2D transform must be an object")
    required = ("translate_x", "translate_y", "scale_x", "scale_y", "rotation_degrees")
    if any(name not in value for name in required):
        raise ValueError("Scene2D transform is missing a required field")
    translate_x = _f32(value.get("translate_x", 0.0), "transform translate_x")
    translate_y = _f32(value.get("translate_y", 0.0), "transform translate_y")
    scale_x = _f32(value.get("scale_x", 1.0), "transform scale_x")
    scale_y = _f32(value.get("scale_y", 1.0), "transform scale_y")
    rotation = _f32(value.get("rotation_degrees", 0.0), "transform rotation")
    shear_x = _f32(value.get("shear_x", 0.0), "transform shear")
    if abs(translate_x) > _MAX_DIMENSION or abs(translate_y) > _MAX_DIMENSION:
        raise ValueError("Scene2D transform translations exceed the supported range")
    if not 0.0 <= scale_x <= _MAX_TRANSFORM_SCALE or not 0.0 <= scale_y <= _MAX_TRANSFORM_SCALE:
        raise ValueError("Scene2D transform scales exceed the supported range")
    if scale_x <= _F32_EPSILON or scale_y <= _F32_EPSILON:
        raise ValueError("Scene2D transform scales must be nonzero")
    if abs(rotation) > _MAX_TRANSFORM_ROTATION:
        raise ValueError("Scene2D transform rotation exceeds the supported range")
    if abs(shear_x) > _MAX_TRANSFORM_SHEAR:
        raise ValueError("Scene2D transform shear exceeds the supported range")
    return translate_x, translate_y, scale_x, scale_y, rotation, shear_x


def _transform_linear(
    transform: tuple[float, float, float, float, float, float],
) -> tuple[float, float, float, float]:
    _, _, scale_x, scale_y, rotation, shear_x = transform
    radians = _f32(math.radians(rotation), "composed transform rotation")
    sine = _f32(math.sin(radians), "composed transform coefficient")
    cosine = _f32(math.cos(radians), "composed transform coefficient")
    a = _f32_mul(cosine, scale_x, "composed transform coefficient")
    b = _f32_sub(
        _f32_mul(cosine, shear_x, "composed transform coefficient"),
        _f32_mul(sine, scale_y, "composed transform coefficient"),
        "composed transform coefficient",
    )
    c = _f32_mul(sine, scale_x, "composed transform coefficient")
    d = _f32_add(
        _f32_mul(sine, shear_x, "composed transform coefficient"),
        _f32_mul(cosine, scale_y, "composed transform coefficient"),
        "composed transform coefficient",
    )
    return a, b, c, d


def _compose_transform(
    parent: tuple[float, float, float, float, float, float],
    child: tuple[float, float, float, float, float, float],
) -> tuple[float, float, float, float, float, float]:
    pa, pb, pc, pd = _transform_linear(parent)
    ca, cb, cc, cd = _transform_linear(child)
    a = _f32_add(
        _f32_mul(pa, ca, "composed transform coefficient"),
        _f32_mul(pb, cc, "composed transform coefficient"),
        "composed transform coefficient",
    )
    b = _f32_add(
        _f32_mul(pa, cb, "composed transform coefficient"),
        _f32_mul(pb, cd, "composed transform coefficient"),
        "composed transform coefficient",
    )
    c = _f32_add(
        _f32_mul(pc, ca, "composed transform coefficient"),
        _f32_mul(pd, cc, "composed transform coefficient"),
        "composed transform coefficient",
    )
    d = _f32_add(
        _f32_mul(pc, cb, "composed transform coefficient"),
        _f32_mul(pd, cd, "composed transform coefficient"),
        "composed transform coefficient",
    )
    scale_x_squared = _f32_add(
        _f32_mul(a, a, "composed transform scale"),
        _f32_mul(c, c, "composed transform scale"),
        "composed transform scale",
    )
    scale_x = _f32(math.sqrt(scale_x_squared), "composed transform scale")
    angle = _f32(math.atan2(c, a), "composed transform rotation")
    rotation = _f32(math.degrees(angle), "composed transform rotation")
    shear = _f32_div(
        _f32_add(
            _f32_mul(a, b, "composed transform shear"),
            _f32_mul(c, d, "composed transform shear"),
            "composed transform shear",
        ),
        scale_x,
        "composed transform shear",
    )
    scale_y = _f32_div(
        _f32_sub(
            _f32_mul(a, d, "composed transform scale"),
            _f32_mul(b, c, "composed transform scale"),
            "composed transform scale",
        ),
        scale_x,
        "composed transform scale",
    )
    translate_x = _f32_add(
        _f32_add(
            _f32_mul(pa, child[0], "composed transform translation"),
            _f32_mul(pb, child[1], "composed transform translation"),
            "composed transform translation",
        ),
        parent[0],
        "composed transform translation",
    )
    translate_y = _f32_add(
        _f32_add(
            _f32_mul(pc, child[0], "composed transform translation"),
            _f32_mul(pd, child[1], "composed transform translation"),
            "composed transform translation",
        ),
        parent[1],
        "composed transform translation",
    )
    if scale_x <= _F32_EPSILON or abs(scale_y) <= _F32_EPSILON:
        raise ValueError("Scene2D composed transform must have nonzero scale")
    result = (translate_x, translate_y, scale_x, scale_y, rotation, shear)
    _transform_linear(result)
    return result


def _rect_from_spec(value: Any, name: str) -> tuple[float, float, float, float]:
    if not isinstance(value, Mapping):
        raise ValueError(f"{name} must be an object")
    x = _f32(value.get("x"), f"{name} x")
    y = _f32(value.get("y"), f"{name} y")
    width = _f32(value.get("width"), f"{name} width")
    height = _f32(value.get("height"), f"{name} height")
    return x, y, width, height


def _kind_bounds(kind: Mapping[str, Any], node_id: str) -> tuple[float, float, float, float] | None:
    kind_type = kind.get("type")
    if kind_type == "group":
        return None
    if kind_type in ("rect", "rounded_rect"):
        return _rect_from_spec(kind.get("rect"), f"Scene2D node {node_id!r} rect")
    if kind_type == "circle":
        center = kind.get("center")
        if not isinstance(center, Mapping):
            raise ValueError("Scene2D circle center must be an object")
        x = _f32(center.get("x"), "circle center x")
        y = _f32(center.get("y"), "circle center y")
        radius = _f32(kind.get("radius"), "circle radius")
        return (
            _f32_sub(x, radius, "circle bounds"),
            _f32_sub(y, radius, "circle bounds"),
            _f32_mul(radius, 2.0, "circle bounds"),
            _f32_mul(radius, 2.0, "circle bounds"),
        )
    if kind_type == "line":
        start = kind.get("start")
        end = kind.get("end")
        stroke = kind.get("stroke")
        if not isinstance(start, Mapping) or not isinstance(end, Mapping) or not isinstance(stroke, Mapping):
            raise ValueError("Scene2D line geometry must be objects")
        sx = _f32(start.get("x"), "line start x")
        sy = _f32(start.get("y"), "line start y")
        ex = _f32(end.get("x"), "line end x")
        ey = _f32(end.get("y"), "line end y")
        half_width = _f32_mul(_f32(stroke.get("width"), "line stroke width"), 0.5, "line bounds")
        return (
            _f32_sub(min(sx, ex), half_width, "line bounds"),
            _f32_sub(min(sy, ey), half_width, "line bounds"),
            _f32_add(abs(sx - ex), 2.0 * half_width, "line bounds"),
            _f32_add(abs(sy - ey), 2.0 * half_width, "line bounds"),
        )
    if kind_type == "path":
        commands = kind.get("commands", ())
        if not isinstance(commands, Sequence) or isinstance(commands, (str, bytes)):
            raise ValueError("Scene2D path commands must be a sequence")
        points: list[tuple[float, float]] = []
        for command in commands:
            if not isinstance(command, Mapping):
                raise ValueError("Scene2D path commands must be objects")
            for field_name in ("point", "control", "control_a", "control_b"):
                point = command.get(field_name)
                if point is not None:
                    if not isinstance(point, Mapping):
                        raise ValueError("Scene2D path points must be objects")
                    points.append((_f32(point.get("x"), "path x"), _f32(point.get("y"), "path y")))
        if not points:
            return None
        stroke = kind.get("stroke")
        padding = 0.0
        if stroke is not None:
            if not isinstance(stroke, Mapping):
                raise ValueError("Scene2D path stroke must be an object")
            padding = _f32_mul(_f32(stroke.get("width"), "path stroke width"), 0.5, "path bounds")
        min_x = min(point[0] for point in points)
        min_y = min(point[1] for point in points)
        max_x = max(point[0] for point in points)
        max_y = max(point[1] for point in points)
        return (
            _f32_sub(min_x, padding, "path bounds"),
            _f32_sub(min_y, padding, "path bounds"),
            _f32_add(_f32_sub(max_x, min_x, "path bounds"), 2.0 * padding, "path bounds"),
            _f32_add(_f32_sub(max_y, min_y, "path bounds"), 2.0 * padding, "path bounds"),
        )
    if kind_type == "text":
        origin = kind.get("origin")
        content = kind.get("content")
        if not isinstance(origin, Mapping) or not isinstance(content, str):
            raise ValueError("Scene2D text geometry is invalid")
        x = _f32(origin.get("x"), "text origin x")
        y = _f32(origin.get("y"), "text origin y")
        text_size = _f32(kind.get("size"), "text size")
        width = _f32_mul(_f32_mul(float(len(content)), text_size, "text bounds"), 0.62, "text bounds")
        height = _f32_mul(text_size, 1.4, "text bounds")
        return x, y, width, height
    raise ValueError(f"Scene2D node {node_id!r} has unsupported kind {kind_type!r}")


def _check_transformed_bounds(
    rect: tuple[float, float, float, float],
    transform: tuple[float, float, float, float, float, float],
    name: str,
) -> None:
    x, y, width, height = rect
    right = _f32_add(x, width, name)
    bottom = _f32_add(y, height, name)
    a, b, c, d = _transform_linear(transform)
    for corner_x, corner_y in ((x, y), (right, y), (x, bottom), (right, bottom)):
        _f32_add(
            _f32_add(_f32_mul(a, corner_x, name), _f32_mul(b, corner_y, name), name),
            transform[0],
            name,
        )
        _f32_add(
            _f32_add(_f32_mul(c, corner_x, name), _f32_mul(d, corner_y, name), name),
            transform[1],
            name,
        )


def _point(value: Sequence[float], name: str) -> dict[str, float]:
    if len(value) != 2:
        raise ValueError(f"{name} must contain exactly two coordinates")
    return {"x": _number(value[0], f"{name}.x"), "y": _number(value[1], f"{name}.y")}


def color_spec(value: str | Sequence[float]) -> dict[str, Any]:
    """Convert ``#rgb[a]`` / ``#rrggbb[aa]`` or normalized RGBA to wire form."""
    if isinstance(value, str):
        match = re.fullmatch(r"#?([0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})", value)
        if match is None:
            raise ValueError(f"invalid Scene2D color {value!r}")
        digits = match.group(1)
        if len(digits) in (3, 4):
            channels = [int(char * 2, 16) / 255.0 for char in digits]
        else:
            channels = [int(digits[index:index + 2], 16) / 255.0 for index in range(0, len(digits), 2)]
        if len(channels) == 3:
            channels.append(1.0)
    else:
        if len(value) not in (3, 4):
            raise ValueError("Scene2D colors need three or four normalized channels")
        channels = [_number(channel, "color channel", minimum=0.0) for channel in value]
        if any(channel > 1.0 for channel in channels):
            raise ValueError("Scene2D color channels must be between zero and one")
        if len(channels) == 3:
            channels.append(1.0)
    return {"r": channels[0], "g": channels[1], "b": channels[2], "a": channels[3]}


Color = str | Sequence[float]


@dataclass(frozen=True)
class Scene2DLinearGradient:
    """Linear paint brush accepted by filled Scene2D shapes and backgrounds."""

    angle_degrees: float
    from_color: Color
    to_color: Color

    def to_spec(self) -> dict[str, Any]:
        angle = _number(self.angle_degrees, "gradient angle")
        if not 0.0 <= angle <= 360.0:
            raise ValueError("gradient angle must be between zero and 360 degrees")
        return {
            "type": "linear_gradient",
            "angle_degrees": angle,
            "from": color_spec(self.from_color),
            "to": color_spec(self.to_color),
        }


Brush = Color | Scene2DLinearGradient


def _brush_spec(value: Brush | None) -> dict[str, Any] | None:
    if value is None:
        return None
    if isinstance(value, Scene2DLinearGradient):
        return value.to_spec()
    return {"type": "solid", "color": color_spec(value)}


@dataclass(frozen=True)
class Stroke:
    color: str | Sequence[float]
    width: float = 1.0

    def to_spec(self) -> dict[str, Any]:
        return {"width": _number(self.width, "stroke width", minimum=0.0), "color": color_spec(self.color)}


@dataclass(frozen=True)
class Scene2DSemantic:
    """Accessible description associated with a drawable hit target."""

    role: str
    label: str
    value_text: str | None = None
    description: str | None = None
    selected: bool | None = None
    disabled: bool | None = None

    def __post_init__(self) -> None:
        if self.role not in ("group", "grid", "row", "grid_cell", "button", "image", "status"):
            raise ValueError(f"unsupported Scene2D semantic role {self.role!r}")
        if not self.label.strip():
            raise ValueError("Scene2D semantic label must not be empty")

    def to_spec(self) -> dict[str, Any]:
        return {key: value for key, value in {
            "role": self.role, "label": self.label, "value_text": self.value_text,
            "description": self.description, "selected": self.selected,
            "disabled": self.disabled,
        }.items() if value is not None}


@dataclass(frozen=True)
class Scene2DTransform:
    translate_x: float = 0.0
    translate_y: float = 0.0
    scale_x: float = 1.0
    scale_y: float = 1.0
    rotation_degrees: float = 0.0
    shear_x: float = 0.0

    def to_spec(self) -> dict[str, float]:
        spec = {
            "translate_x": _number(self.translate_x, "transform translate_x"),
            "translate_y": _number(self.translate_y, "transform translate_y"),
            "scale_x": _number(self.scale_x, "transform scale_x"),
            "scale_y": _number(self.scale_y, "transform scale_y"),
            "rotation_degrees": _number(self.rotation_degrees, "transform rotation"),
            "shear_x": _number(self.shear_x, "transform shear"),
        }
        _transform_from_spec(spec)
        return spec


@dataclass(frozen=True)
class Scene2DTransition:
    duration_ms: int = 160
    easing: str = "ease_out_cubic"
    completion_id: str | None = None
    animate_color: bool = False
    reveal_path: bool = False

    def to_spec(self) -> dict[str, Any]:
        if self.easing not in {"linear", "ease_out_quad", "ease_out_cubic",
                               "ease_in_out_cubic", "ease_out_back", "ease_out_bounce"}:
            raise ValueError(f"unsupported Scene2D transition easing {self.easing!r}")
        if (isinstance(self.duration_ms, bool) or not isinstance(self.duration_ms, int)
                or not 0 <= self.duration_ms <= 60_000):
            raise ValueError("transition duration must be an integer from zero to 60000 ms")
        if self.completion_id is not None and not self.completion_id.strip():
            raise ValueError("transition completion id must not be empty")
        return {"duration_ms": self.duration_ms, "easing": self.easing,
                "completion_id": self.completion_id,
                "animate_color": bool(self.animate_color),
                "reveal_path": bool(self.reveal_path)}


@dataclass(frozen=True)
class Scene2DClip:
    """Local rectangular clip on a drawable or inherited group subtree."""

    x: float
    y: float
    width: float
    height: float

    def to_spec(self) -> dict[str, float]:
        return {"x": _number(self.x, "clip x"), "y": _number(self.y, "clip y"),
                "width": _number(self.width, "clip width", minimum=0.0),
                "height": _number(self.height, "clip height", minimum=0.0)}


@dataclass(frozen=True)
class Scene2DShadow:
    """One bounded primitive shadow in local coordinates."""

    offset_x: float = 0.0
    offset_y: float = 0.0
    blur_radius: float = 0.0
    color: Color = (0.0, 0.0, 0.0, 0.25)

    def to_spec(self) -> dict[str, Any]:
        if _number(self.blur_radius, "shadow blur radius", minimum=0.0) > 128.0:
            raise ValueError("shadow blur radius must not exceed 128")
        return {"offset_x": _number(self.offset_x, "shadow offset_x"),
                "offset_y": _number(self.offset_y, "shadow offset_y"),
                "blur_radius": float(self.blur_radius),
                "color": color_spec(self.color)}


def _base_node(id: str, hit_id: str | None, semantic: Scene2DSemantic | None,
               kind: dict[str, Any], opacity: float = 1.0,
               transform: Scene2DTransform | None = None,
               transition: Scene2DTransition | None = None,
               clip: Scene2DClip | None = None,
               shadow: Scene2DShadow | None = None,
               hit_bounds: Scene2DClip | None = None) -> dict[str, Any]:
    if not id.strip():
        raise ValueError("Scene2D node id must not be empty")
    spec: dict[str, Any] = {"id": id, "hit_id": hit_id, "kind": kind}
    if semantic is not None:
        spec["semantic"] = semantic.to_spec()
    if opacity != 1.0:
        opacity_value = _number(opacity, "node opacity", minimum=0.0)
        if opacity_value > 1.0:
            raise ValueError("Scene2D opacity must be between zero and one")
        spec["opacity"] = opacity_value
    if transform is not None:
        spec["transform"] = transform.to_spec()
    if transition is not None:
        spec["transition"] = transition.to_spec()
    if clip is not None:
        spec["clip"] = clip.to_spec()
    if shadow is not None:
        spec["shadow"] = shadow.to_spec()
    if hit_bounds is not None:
        spec["hit_bounds"] = hit_bounds.to_spec()
    return spec


@dataclass(frozen=True)
class SceneRect:
    id: str
    x: float
    y: float
    width: float
    height: float
    fill: Brush | None = None
    stroke: Stroke | None = None
    hit_id: str | None = None
    semantic: Scene2DSemantic | None = None
    opacity: float = 1.0
    transform: Scene2DTransform | None = None
    transition: Scene2DTransition | None = None
    clip: Scene2DClip | None = None
    shadow: Scene2DShadow | None = None
    hit_bounds: Scene2DClip | None = None

    def to_spec(self) -> dict[str, Any]:
        kind: dict[str, Any] = {"type": "rect", "rect": {
            "x": _number(self.x, "rect x"), "y": _number(self.y, "rect y"),
            "width": _number(self.width, "rect width", minimum=0.0),
            "height": _number(self.height, "rect height", minimum=0.0),
        }}
        kind["fill"] = _brush_spec(self.fill)
        kind["stroke"] = None if self.stroke is None else self.stroke.to_spec()
        return _base_node(self.id, self.hit_id, self.semantic, kind,
                          self.opacity, self.transform, self.transition, self.clip, self.shadow,
                          self.hit_bounds)


@dataclass(frozen=True)
class SceneRoundedRect(SceneRect):
    radius: float = 8.0

    def to_spec(self) -> dict[str, Any]:
        rect = super().to_spec()
        kind = rect["kind"]
        kind["type"] = "rounded_rect"
        kind["radius"] = _number(self.radius, "corner radius", minimum=0.0)
        return rect


@dataclass(frozen=True)
class SceneCircle:
    id: str
    x: float
    y: float
    radius: float
    fill: Brush | None = None
    stroke: Stroke | None = None
    hit_id: str | None = None
    semantic: Scene2DSemantic | None = None
    opacity: float = 1.0
    transform: Scene2DTransform | None = None
    transition: Scene2DTransition | None = None
    clip: Scene2DClip | None = None
    shadow: Scene2DShadow | None = None
    hit_bounds: Scene2DClip | None = None

    def to_spec(self) -> dict[str, Any]:
        kind: dict[str, Any] = {
            "type": "circle", "center": _point((self.x, self.y), "circle center"),
            "radius": _number(self.radius, "circle radius", minimum=0.0),
            "fill": _brush_spec(self.fill),
            "stroke": None if self.stroke is None else self.stroke.to_spec(),
        }
        return _base_node(self.id, self.hit_id, self.semantic, kind,
                          self.opacity, self.transform, self.transition, self.clip, self.shadow,
                          self.hit_bounds)


@dataclass(frozen=True)
class SceneLine:
    id: str
    start: tuple[float, float]
    end: tuple[float, float]
    stroke: Stroke
    hit_id: str | None = None
    semantic: Scene2DSemantic | None = None
    opacity: float = 1.0
    transform: Scene2DTransform | None = None
    transition: Scene2DTransition | None = None
    clip: Scene2DClip | None = None
    shadow: Scene2DShadow | None = None
    hit_bounds: Scene2DClip | None = None

    def to_spec(self) -> dict[str, Any]:
        kind = {"type": "line", "start": _point(self.start, "line start"),
                "end": _point(self.end, "line end"), "stroke": self.stroke.to_spec()}
        return _base_node(self.id, self.hit_id, self.semantic, kind,
                          self.opacity, self.transform, self.transition, self.clip, self.shadow,
                          self.hit_bounds)


@dataclass(frozen=True)
class PathCommand:
    """One move, line, quadratic, cubic, or close command in a vector path."""

    type: str
    points: tuple[tuple[float, float], ...] = ()

    def to_spec(self) -> dict[str, Any]:
        expected = {"move_to": 1, "line_to": 1, "quadratic_to": 2, "cubic_to": 3, "close": 0}
        if self.type not in expected or len(self.points) != expected[self.type]:
            raise ValueError(f"invalid Scene2D path command {self.type!r}")
        value: dict[str, Any] = {"type": self.type}
        if self.type in {"move_to", "line_to"}:
            value["point"] = _point(self.points[0], "path point")
        elif self.type == "quadratic_to":
            value["control"] = _point(self.points[0], "quadratic control")
            value["point"] = _point(self.points[1], "path point")
        elif self.type == "cubic_to":
            value["control_a"] = _point(self.points[0], "cubic control")
            value["control_b"] = _point(self.points[1], "cubic control")
            value["point"] = _point(self.points[2], "path point")
        return value


@dataclass(frozen=True)
class ScenePath:
    id: str
    commands: Sequence[PathCommand]
    fill: Brush | None = None
    stroke: Stroke | None = None
    hit_id: str | None = None
    semantic: Scene2DSemantic | None = None
    opacity: float = 1.0
    transform: Scene2DTransform | None = None
    transition: Scene2DTransition | None = None
    clip: Scene2DClip | None = None
    shadow: Scene2DShadow | None = None
    hit_bounds: Scene2DClip | None = None

    def to_spec(self) -> dict[str, Any]:
        if not self.commands:
            raise ValueError("Scene2D path requires at least one command")
        kind = {"type": "path", "commands": [command.to_spec() for command in self.commands],
                "fill": _brush_spec(self.fill),
                "stroke": None if self.stroke is None else self.stroke.to_spec()}
        return _base_node(self.id, self.hit_id, self.semantic, kind,
                          self.opacity, self.transform, self.transition, self.clip, self.shadow,
                          self.hit_bounds)


@dataclass(frozen=True)
class SceneText:
    id: str
    x: float
    y: float
    content: str
    size: float
    color: str | Sequence[float]
    font: str | None = None
    align: str = "left"
    hit_id: str | None = None
    semantic: Scene2DSemantic | None = None
    opacity: float = 1.0
    transform: Scene2DTransform | None = None
    transition: Scene2DTransition | None = None
    clip: Scene2DClip | None = None
    shadow: Scene2DShadow | None = None
    hit_bounds: Scene2DClip | None = None

    def to_spec(self) -> dict[str, Any]:
        if self.align not in ("left", "center", "right"):
            raise ValueError("Scene2D text align must be left, center, or right")
        content = str(self.content)
        if "\n" in content or "\r" in content:
            raise ValueError("Scene2D text content must be a single line")
        kind = {"type": "text", "origin": _point((self.x, self.y), "text origin"),
                "content": content, "size": _number(self.size, "text size", minimum=0.0),
                "color": color_spec(self.color), "font": self.font, "align": self.align}
        return _base_node(self.id, self.hit_id, self.semantic, kind,
                          self.opacity, self.transform, self.transition, self.clip, self.shadow,
                          self.hit_bounds)


@dataclass(frozen=True)
class Scene2DGroup:
    """Ordered child nodes sharing a transform, opacity, and optional clip."""

    id: str
    children: Sequence[Any]
    hit_id: str | None = None
    semantic: Scene2DSemantic | None = None
    opacity: float = 1.0
    transform: Scene2DTransform | None = None
    transition: Scene2DTransition | None = None
    clip: Scene2DClip | None = None
    hit_bounds: Scene2DClip | None = None

    def to_spec(self) -> dict[str, Any]:
        if not self.children:
            raise ValueError("Scene2D groups must contain at least one child")
        kind = {"type": "group", "children": [child.to_spec() for child in self.children]}
        return _base_node(self.id, self.hit_id, self.semantic, kind,
                          self.opacity, self.transform, self.transition, self.clip,
                          hit_bounds=self.hit_bounds)


@dataclass(frozen=True)
class GridCell:
    row: int
    column: int
    index: int
    id: str


@dataclass(frozen=True)
class GridSpec:
    rows: int
    columns: int
    x: float
    y: float
    cell_width: float
    cell_height: float
    gap: float = 0.0
    row_labels: Sequence[str] = ()
    column_labels: Sequence[str] = ()

    def __post_init__(self) -> None:
        if (isinstance(self.rows, bool) or not isinstance(self.rows, int)
                or isinstance(self.columns, bool) or not isinstance(self.columns, int)
                or self.rows <= 0 or self.columns <= 0):
            raise ValueError("grid rows and columns must be positive")
        for name in ("x", "y"):
            _number(getattr(self, name), f"grid {name}")
        for name in ("cell_width", "cell_height"):
            _number(getattr(self, name), f"grid {name}", minimum=0.0)
            if getattr(self, name) == 0:
                raise ValueError(f"grid {name} must be positive")
        _number(self.gap, "grid gap", minimum=0.0)
        if len(self.row_labels) > self.rows or len(self.column_labels) > self.columns:
            raise ValueError("grid labels cannot exceed their row or column count")
        if any(not isinstance(label, str) or len(label) > 1_048_576
               for label in (*self.row_labels, *self.column_labels)):
            raise ValueError("grid labels must be strings no longer than 1 MiB")

    def to_spec(self) -> dict[str, Any]:
        return {"rows": self.rows, "columns": self.columns,
                "x": float(self.x), "y": float(self.y),
                "cell_width": float(self.cell_width), "cell_height": float(self.cell_height),
                "gap": float(self.gap),
                "row_labels": list(self.row_labels),
                "column_labels": list(self.column_labels)}

    def cell_id(self, row: int, column: int) -> str:
        self._check(row, column)
        return f"r{row}c{column}"

    def cell_rect(self, row: int, column: int) -> tuple[float, float, float, float]:
        self._check(row, column)
        return (self.x + column * (self.cell_width + self.gap),
                self.y + row * (self.cell_height + self.gap),
                self.cell_width, self.cell_height)

    def cell_center(self, row: int, column: int) -> tuple[float, float]:
        x, y, width, height = self.cell_rect(row, column)
        return (x + width / 2, y + height / 2)

    def pick(self, x: float, y: float) -> GridCell | None:
        x = _number(x, "pointer x") - self.x
        y = _number(y, "pointer y") - self.y
        column = int(x // (self.cell_width + self.gap))
        row = int(y // (self.cell_height + self.gap))
        if row < 0 or row >= self.rows or column < 0 or column >= self.columns:
            return None
        local_x = x - column * (self.cell_width + self.gap)
        local_y = y - row * (self.cell_height + self.gap)
        if local_x >= self.cell_width or local_y >= self.cell_height:
            return None
        return GridCell(row, column, row * self.columns + column, self.cell_id(row, column))

    def _check(self, row: int, column: int) -> None:
        if not (0 <= row < self.rows and 0 <= column < self.columns):
            raise IndexError("grid cell is outside the declared grid")


@dataclass(frozen=True)
class Scene2DInputConfig:
    pointer: bool = True
    continuous: bool = False
    capture: bool = True
    keyboard: bool = True

    def to_spec(self) -> dict[str, Any]:
        return {"pointer": bool(self.pointer), "continuous": bool(self.continuous),
                "capture": bool(self.capture), "keyboard": bool(self.keyboard)}


@dataclass(frozen=True)
class Scene2D:
    id: str
    width: float
    height: float
    nodes: Sequence[Any] = ()
    grid: GridSpec | None = None
    input: Scene2DInputConfig = field(default_factory=Scene2DInputConfig)
    revision: int = 1
    semantic: Scene2DSemantic | None = None
    background: Brush | None = None

    def __post_init__(self) -> None:
        if not self.id.strip():
            raise ValueError("Scene2D surface id must not be empty")
        if isinstance(self.revision, bool) or not isinstance(self.revision, int) or self.revision <= 0:
            raise ValueError("Scene2D revision must be positive")
        _number(self.width, "Scene2D width", minimum=0.0)
        _number(self.height, "Scene2D height", minimum=0.0)
        if self.width == 0 or self.height == 0:
            raise ValueError("Scene2D view bounds must be positive")
        specs = _node_specs(self.nodes)
        node_ids = [node_id for node_id, _ in _walk_node_ids(specs)]
        if len(node_ids) != len(set(node_ids)):
            raise ValueError("Scene2D node ids must be unique")

    def scene_spec(self) -> dict[str, Any]:
        view_box = {"x": 0.0, "y": 0.0, "width": float(self.width), "height": float(self.height)}
        scene: dict[str, Any] = {
            "version": 1, "revision": int(self.revision), "view_box": view_box,
            "nodes": _node_specs(self.nodes),
            "grid": None if self.grid is None else self.grid.to_spec(),
            "input": self.input.to_spec(),
            "background": _brush_spec(self.background),
            "semantic": None if self.semantic is None else self.semantic.to_spec(),
        }
        return scene

    def to_spec(self) -> dict[str, Any]:
        # App sections use UiNode's shared `kind` tag; keep this declaration
        # consistent with the Rust UiNode enum rather than the nested scene
        # primitive `type` tags.
        return {"kind": "scene2d", "id": self.id, "scene": self.scene_spec()}


@dataclass(frozen=True)
class Scene2DPatch:
    id: str
    base_revision: int
    revision: int
    upsert: Sequence[Mapping[str, Any]] = ()
    remove: Sequence[str] = ()
    view_box: Mapping[str, float] | None = None
    # ``grid=None`` is a real replacement that removes a grid. The private
    # sentinel distinguishes that from a patch that leaves the grid untouched.
    grid: Mapping[str, Any] | None | object = _UNSET
    input: Mapping[str, Any] | object = _UNSET
    background: Mapping[str, Any] | None | object = _UNSET
    semantic: Mapping[str, Any] | None | object = _UNSET

    def __post_init__(self) -> None:
        if (not self.id.strip() or isinstance(self.base_revision, bool)
                or not isinstance(self.base_revision, int) or self.base_revision <= 0
                or isinstance(self.revision, bool) or not isinstance(self.revision, int)
                or self.revision != self.base_revision + 1):
            raise ValueError("Scene2D patches require a surface id and consecutive positive revisions")
        upsert_ids = [node_id for node_id, _ in _walk_node_ids(self.upsert)]
        if (any(not isinstance(node_id, str) or not node_id.strip()
                for node_id in upsert_ids)
                or len(upsert_ids) != len(set(upsert_ids))):
            raise ValueError("Scene2D patch upsert IDs, including group descendants, must be unique")
        if (any(not isinstance(node_id, str) or not node_id.strip()
                for node_id in self.remove)
                or len(self.remove) != len(set(self.remove))):
            raise ValueError("Scene2D patch remove ids must be non-empty and unique")
        if set(upsert_ids) & set(self.remove):
            raise ValueError("a Scene2D patch cannot upsert and remove the same id")

    @classmethod
    def between(cls, previous: Scene2D, current: Scene2D) -> "Scene2DPatch":
        if previous.id != current.id:
            raise ValueError("Scene2D patch surfaces must have the same id")
        if current.revision != previous.revision + 1:
            raise ValueError("Scene2D patch scenes must have consecutive revisions")
        old = {node.to_spec()["id"]: node.to_spec() for node in previous.nodes}
        new = {node.to_spec()["id"]: node.to_spec() for node in current.nodes}
        upsert = [value for node_id, value in new.items() if old.get(node_id) != value]
        remove = [node_id for node_id in old if node_id not in new]
        old_scene, new_scene = previous.scene_spec(), current.scene_spec()
        return cls(
            id=current.id, base_revision=previous.revision, revision=current.revision,
            upsert=upsert, remove=remove,
            view_box=new_scene["view_box"] if old_scene["view_box"] != new_scene["view_box"] else None,
            grid=new_scene["grid"] if old_scene["grid"] != new_scene["grid"] else _UNSET,
            input=new_scene["input"] if old_scene["input"] != new_scene["input"] else _UNSET,
            background=(new_scene["background"]
                        if old_scene["background"] != new_scene["background"] else _UNSET),
            semantic=(new_scene["semantic"]
                      if old_scene["semantic"] != new_scene["semantic"] else _UNSET),
        )

    def to_op(self) -> dict[str, Any]:
        op: dict[str, Any] = {"base_revision": self.base_revision, "revision": self.revision,
                              "upsert": [dict(node) for node in self.upsert], "remove": list(self.remove)}
        if self.view_box is not None:
            op["view_box"] = dict(self.view_box)
        if self.grid is not _UNSET:
            op["grid"] = None if self.grid is None else dict(self.grid)
        if self.input is not _UNSET:
            op["input"] = dict(self.input)
        if self.background is not _UNSET:
            op["background"] = None if self.background is None else dict(self.background)
        if self.semantic is not _UNSET:
            op["semantic"] = None if self.semantic is None else dict(self.semantic)
        return {"op": "scene2d_patch", "id": self.id, "patch": op}


def replace_op(scene: Scene2D) -> dict[str, Any]:
    """Build an atomic full-scene replacement PatchOp."""
    return {"op": "scene2d_replace", "id": scene.id, "scene": scene.scene_spec()}


def patch_op(previous: Scene2D, current: Scene2D) -> dict[str, Any]:
    """Build a stable-ID diff PatchOp between consecutive scene revisions."""
    return Scene2DPatch.between(previous, current).to_op()


def grid_cell_id(row: int, column: int) -> str:
    """Return the canonical host grid-cell ID used by Scene2D events."""
    if row < 0 or column < 0:
        raise ValueError("grid row and column must be non-negative")
    return f"r{row}c{column}"
