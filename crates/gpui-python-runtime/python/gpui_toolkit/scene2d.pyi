from collections.abc import Mapping, Sequence
from typing import Any

Color = str | Sequence[float]

class Scene2DLinearGradient:
    angle_degrees: float
    from_color: Color
    to_color: Color
    def __init__(self, angle_degrees: float, from_color: Color, to_color: Color) -> None: ...
    def to_spec(self) -> dict[str, Any]: ...

Brush = Color | Scene2DLinearGradient

class Stroke:
    color: Color
    width: float
    def __init__(self, color: Color, width: float = ...) -> None: ...
    def to_spec(self) -> dict[str, Any]: ...

class Scene2DSemantic:
    role: str
    label: str
    value_text: str | None
    description: str | None
    selected: bool | None
    disabled: bool | None
    def __init__(self, role: str, label: str, value_text: str | None = ..., description: str | None = ..., selected: bool | None = ..., disabled: bool | None = ...) -> None: ...
    def to_spec(self) -> dict[str, Any]: ...

class Scene2DTransform:
    translate_x: float
    translate_y: float
    scale_x: float
    scale_y: float
    rotation_degrees: float
    shear_x: float
    def __init__(self, translate_x: float = ..., translate_y: float = ..., scale_x: float = ..., scale_y: float = ..., rotation_degrees: float = ..., shear_x: float = ...) -> None: ...
    def to_spec(self) -> dict[str, float]: ...

class Scene2DTransition:
    duration_ms: int
    easing: str
    completion_id: str | None
    animate_color: bool
    reveal_path: bool
    def __init__(self, duration_ms: int = ..., easing: str = ..., completion_id: str | None = ..., animate_color: bool = ..., reveal_path: bool = ...) -> None: ...
    def to_spec(self) -> dict[str, Any]: ...

class Scene2DClip:
    x: float
    y: float
    width: float
    height: float
    def __init__(self, x: float, y: float, width: float, height: float) -> None: ...
    def to_spec(self) -> dict[str, float]: ...

class Scene2DShadow:
    offset_x: float
    offset_y: float
    blur_radius: float
    color: Color
    def __init__(self, offset_x: float = ..., offset_y: float = ..., blur_radius: float = ..., color: Color = ...) -> None: ...
    def to_spec(self) -> dict[str, Any]: ...

class SceneRect:
    id: str
    x: float
    y: float
    width: float
    height: float
    fill: Brush | None
    stroke: Stroke | None
    hit_id: str | None
    semantic: Scene2DSemantic | None
    opacity: float
    transform: Scene2DTransform | None
    transition: Scene2DTransition | None
    clip: Scene2DClip | None
    shadow: Scene2DShadow | None
    hit_bounds: Scene2DClip | None
    def __init__(self, id: str, x: float, y: float, width: float, height: float, fill: Brush | None = ..., stroke: Stroke | None = ..., hit_id: str | None = ..., semantic: Scene2DSemantic | None = ..., opacity: float = ..., transform: Scene2DTransform | None = ..., transition: Scene2DTransition | None = ..., clip: Scene2DClip | None = ..., shadow: Scene2DShadow | None = ..., hit_bounds: Scene2DClip | None = ...) -> None: ...
    def to_spec(self) -> dict[str, Any]: ...

class SceneRoundedRect(SceneRect):
    radius: float
    def __init__(self, id: str, x: float, y: float, width: float, height: float, fill: Brush | None = ..., stroke: Stroke | None = ..., hit_id: str | None = ..., semantic: Scene2DSemantic | None = ..., opacity: float = ..., transform: Scene2DTransform | None = ..., transition: Scene2DTransition | None = ..., clip: Scene2DClip | None = ..., shadow: Scene2DShadow | None = ..., hit_bounds: Scene2DClip | None = ..., radius: float = ...) -> None: ...

class SceneCircle:
    id: str
    x: float
    y: float
    radius: float
    fill: Brush | None
    stroke: Stroke | None
    hit_id: str | None
    semantic: Scene2DSemantic | None
    opacity: float
    transform: Scene2DTransform | None
    transition: Scene2DTransition | None
    clip: Scene2DClip | None
    shadow: Scene2DShadow | None
    hit_bounds: Scene2DClip | None
    def __init__(self, id: str, x: float, y: float, radius: float, fill: Brush | None = ..., stroke: Stroke | None = ..., hit_id: str | None = ..., semantic: Scene2DSemantic | None = ..., opacity: float = ..., transform: Scene2DTransform | None = ..., transition: Scene2DTransition | None = ..., clip: Scene2DClip | None = ..., shadow: Scene2DShadow | None = ..., hit_bounds: Scene2DClip | None = ...) -> None: ...
    def to_spec(self) -> dict[str, Any]: ...

class SceneLine:
    id: str
    start: tuple[float, float]
    end: tuple[float, float]
    stroke: Stroke
    hit_id: str | None
    semantic: Scene2DSemantic | None
    opacity: float
    transform: Scene2DTransform | None
    transition: Scene2DTransition | None
    clip: Scene2DClip | None
    shadow: Scene2DShadow | None
    hit_bounds: Scene2DClip | None
    def __init__(self, id: str, start: tuple[float, float], end: tuple[float, float], stroke: Stroke, hit_id: str | None = ..., semantic: Scene2DSemantic | None = ..., opacity: float = ..., transform: Scene2DTransform | None = ..., transition: Scene2DTransition | None = ..., clip: Scene2DClip | None = ..., shadow: Scene2DShadow | None = ..., hit_bounds: Scene2DClip | None = ...) -> None: ...
    def to_spec(self) -> dict[str, Any]: ...

class PathCommand:
    type: str
    points: tuple[tuple[float, float], ...]
    def __init__(self, type: str, points: tuple[tuple[float, float], ...] = ...) -> None: ...
    def to_spec(self) -> dict[str, Any]: ...

class ScenePath:
    id: str
    commands: Sequence[PathCommand]
    fill: Brush | None
    stroke: Stroke | None
    hit_id: str | None
    semantic: Scene2DSemantic | None
    opacity: float
    transform: Scene2DTransform | None
    transition: Scene2DTransition | None
    clip: Scene2DClip | None
    shadow: Scene2DShadow | None
    hit_bounds: Scene2DClip | None
    def __init__(self, id: str, commands: Sequence[PathCommand], fill: Brush | None = ..., stroke: Stroke | None = ..., hit_id: str | None = ..., semantic: Scene2DSemantic | None = ..., opacity: float = ..., transform: Scene2DTransform | None = ..., transition: Scene2DTransition | None = ..., clip: Scene2DClip | None = ..., shadow: Scene2DShadow | None = ..., hit_bounds: Scene2DClip | None = ...) -> None: ...
    def to_spec(self) -> dict[str, Any]: ...

class SceneText:
    id: str
    x: float
    y: float
    content: str
    size: float
    color: Color
    font: str | None
    align: str
    hit_id: str | None
    semantic: Scene2DSemantic | None
    opacity: float
    transform: Scene2DTransform | None
    transition: Scene2DTransition | None
    clip: Scene2DClip | None
    shadow: Scene2DShadow | None
    hit_bounds: Scene2DClip | None
    def __init__(self, id: str, x: float, y: float, content: str, size: float, color: Color, font: str | None = ..., align: str = ..., hit_id: str | None = ..., semantic: Scene2DSemantic | None = ..., opacity: float = ..., transform: Scene2DTransform | None = ..., transition: Scene2DTransition | None = ..., clip: Scene2DClip | None = ..., shadow: Scene2DShadow | None = ..., hit_bounds: Scene2DClip | None = ...) -> None: ...
    def to_spec(self) -> dict[str, Any]: ...

class Scene2DGroup:
    id: str
    children: Sequence[Any]
    hit_id: str | None
    semantic: Scene2DSemantic | None
    opacity: float
    transform: Scene2DTransform | None
    transition: Scene2DTransition | None
    clip: Scene2DClip | None
    hit_bounds: Scene2DClip | None
    def __init__(self, id: str, children: Sequence[Any], hit_id: str | None = ..., semantic: Scene2DSemantic | None = ..., opacity: float = ..., transform: Scene2DTransform | None = ..., transition: Scene2DTransition | None = ..., clip: Scene2DClip | None = ..., hit_bounds: Scene2DClip | None = ...) -> None: ...
    def to_spec(self) -> dict[str, Any]: ...

class GridCell:
    row: int
    column: int
    index: int
    id: str
    def __init__(self, row: int, column: int, index: int, id: str) -> None: ...

class GridSpec:
    rows: int
    columns: int
    x: float
    y: float
    cell_width: float
    cell_height: float
    gap: float
    row_labels: Sequence[str]
    column_labels: Sequence[str]
    def __init__(self, rows: int, columns: int, x: float, y: float, cell_width: float, cell_height: float, gap: float = ..., row_labels: Sequence[str] = ..., column_labels: Sequence[str] = ...) -> None: ...
    def to_spec(self) -> dict[str, Any]: ...
    def cell_id(self, row: int, column: int) -> str: ...
    def cell_rect(self, row: int, column: int) -> tuple[float, float, float, float]: ...
    def cell_center(self, row: int, column: int) -> tuple[float, float]: ...
    def pick(self, x: float, y: float) -> GridCell | None: ...

class Scene2DInputConfig:
    pointer: bool
    continuous: bool
    capture: bool
    keyboard: bool
    def __init__(self, pointer: bool = ..., continuous: bool = ..., capture: bool = ..., keyboard: bool = ...) -> None: ...
    def to_spec(self) -> dict[str, Any]: ...

class Scene2D:
    id: str
    width: float
    height: float
    nodes: Sequence[Any]
    grid: GridSpec | None
    input: Scene2DInputConfig
    revision: int
    semantic: Scene2DSemantic | None
    background: Brush | None
    def __init__(self, id: str, width: float, height: float, nodes: Sequence[Any] = ..., grid: GridSpec | None = ..., input: Scene2DInputConfig = ..., revision: int = ..., semantic: Scene2DSemantic | None = ..., background: Brush | None = ...) -> None: ...
    def scene_spec(self) -> dict[str, Any]: ...
    def to_spec(self) -> dict[str, Any]: ...

class Scene2DPatch:
    id: str
    base_revision: int
    revision: int
    upsert: Sequence[Mapping[str, Any]]
    remove: Sequence[str]
    view_box: Mapping[str, float] | None
    grid: Mapping[str, Any] | None
    input: Mapping[str, Any] | None
    background: Mapping[str, Any] | None
    semantic: Mapping[str, Any] | None
    def __init__(self, id: str, base_revision: int, revision: int, upsert: Sequence[Mapping[str, Any]] = ..., remove: Sequence[str] = ..., view_box: Mapping[str, float] | None = ..., grid: Mapping[str, Any] | None = ..., input: Mapping[str, Any] = ..., background: Mapping[str, Any] | None = ..., semantic: Mapping[str, Any] | None = ...) -> None: ...
    @classmethod
    def between(cls, previous: Scene2D, current: Scene2D) -> Scene2DPatch: ...
    def to_op(self) -> dict[str, Any]: ...

def color_spec(value: Color) -> dict[str, float]: ...
def replace_op(scene: Scene2D) -> dict[str, Any]: ...
def patch_op(previous: Scene2D, current: Scene2D) -> dict[str, Any]: ...
def grid_cell_id(row: int, column: int) -> str: ...
