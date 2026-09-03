"""Typed requests for native gpui-d3rs algorithms."""
from __future__ import annotations
import builtins as _builtins

from dataclasses import dataclass
from enum import Enum
from functools import cmp_to_key as _cmp_to_key
from math import isfinite, isnan
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    Generic,
    Hashable,
    Protocol,
    Sequence,
    TypeVar,
    runtime_checkable,
)

from . import native as _native
from .commands import CommandResult, CommandStatus

if TYPE_CHECKING:
    from .app import SessionContext


Domain = tuple[float, float]
_T = TypeVar("_T")
_U = TypeVar("_U")
_K = TypeVar("_K", bound=Hashable)


@runtime_checkable
class Interpolate(Protocol[_T]):
    """Python protocol corresponding to d3rs' generic interpolation trait."""

    def interpolate(self, other: _T, t: float) -> _T: ...


@dataclass(frozen=True, init=False)
class ArrayInterpolator(Generic[_T]):
    """Immutable collection of independent scalar interpolation callables."""

    _interpolators: tuple[Callable[[float], _T], ...]

    def __init__(self, interpolators: Sequence[Callable[[float], _T]]) -> None:
        checked = tuple(interpolators)
        for index, interpolator in enumerate(checked):
            _require_callable(interpolator, f"interpolators[{index}]")
        object.__setattr__(self, "_interpolators", checked)

    def interpolate(self, t: float) -> list[_T]:
        return [interpolator(t) for interpolator in self._interpolators]


class Renderer2D(str, Enum):
    """High-level renderer preference shared by d3rs and gpui-px."""

    VELLO = "vello"
    LEGACY = "legacy"

    def is_vello(self) -> bool:
        return self is Renderer2D.VELLO


class VelloBackend(str, Enum):
    """Backend preference for Vello-backed two-dimensional rendering."""

    AUTO = "auto"
    CPU = "cpu"
    WGPU = "wgpu"


def _color_u8(value: int, name: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool):
        raise TypeError(f"{name} must be an integer")
    if not 0 <= value <= 255:
        raise ValueError(f"{name} must be in 0..=255")
    return value


@dataclass(frozen=True)
class D3Color:
    """Immutable native-backed RGBA color with normalized components."""

    r: float
    g: float
    b: float
    a: float = 1.0

    def __post_init__(self) -> None:
        for name in ("r", "g", "b", "a"):
            value = float(getattr(self, name))
            if not isfinite(value):
                raise ValueError(f"D3Color.{name} must be finite")
            object.__setattr__(self, name, value)

    @classmethod
    def _from_tuple(cls, value: Sequence[float]) -> "D3Color":
        return cls(float(value[0]), float(value[1]), float(value[2]), float(value[3]))

    def _tuple(self) -> tuple[float, float, float, float]:
        return self.r, self.g, self.b, self.a

    @classmethod
    def rgb(cls, r: int, g: int, b: int) -> "D3Color":
        return cls._from_tuple(
            _native.d3_color_rgb(
                _color_u8(r, "r"), _color_u8(g, "g"), _color_u8(b, "b")
            )
        )

    @classmethod
    def rgba(cls, r: int, g: int, b: int, a: int) -> "D3Color":
        return cls._from_tuple(
            _native.d3_color_rgb(
                _color_u8(r, "r"),
                _color_u8(g, "g"),
                _color_u8(b, "b"),
                _color_u8(a, "a"),
            )
        )

    @classmethod
    def from_hex(cls, hex: int) -> "D3Color":
        if not isinstance(hex, int) or isinstance(hex, bool):
            raise TypeError("hex must be an integer")
        if not 0 <= hex <= 0xFFFFFF:
            raise ValueError("hex must be in 0x000000..=0xffffff")
        return cls._from_tuple(_native.d3_color_from_hex(hex))

    @classmethod
    def from_rgb_f32(cls, r: float, g: float, b: float) -> "D3Color":
        return cls._from_tuple(_native.d3_color_from_f32(r, g, b))

    @classmethod
    def from_rgba_f32(cls, r: float, g: float, b: float, a: float) -> "D3Color":
        return cls._from_tuple(_native.d3_color_from_f32(r, g, b, a))

    def to_rgba(self) -> tuple[float, float, float, float]:
        return self._tuple()

    @classmethod
    def from_rgba(cls, rgba: Sequence[float]) -> "D3Color":
        if len(rgba) != 4:
            raise ValueError("rgba must contain exactly four components")
        return cls._from_tuple(_native.d3_color_from_f32(*rgba))

    def _transform(
        self, operation: str, value: float, other: "D3Color | None" = None
    ) -> "D3Color":
        return D3Color._from_tuple(
            _native.d3_color_transform(
                operation,
                self._tuple(),
                value,
                None if other is None else other._tuple(),
            )
        )

    def with_alpha(self, alpha: float) -> "D3Color":
        return self._transform("with_alpha", alpha)

    def interpolate(self, other: "D3Color", t: float) -> "D3Color":
        if not isinstance(other, D3Color):
            raise TypeError("other must be D3Color")
        return self._transform("interpolate", t, other)

    def lighten(self, amount: float) -> "D3Color":
        return self._transform("lighten", amount)

    def darken(self, amount: float) -> "D3Color":
        return self._transform("darken", amount)

    def to_hex(self) -> str:
        return _native.d3_color_to_hex(self._tuple(), False)

    def to_hex_alpha(self) -> str:
        return _native.d3_color_to_hex(self._tuple(), True)

    def luminance(self) -> float:
        return _native.d3_color_luminance(self._tuple())

    def brighter(self, k: float) -> "D3Color":
        return self._transform("brighter", k)

    def darker(self, k: float) -> "D3Color":
        return self._transform("darker", k)

    def with_opacity(self, opacity: float) -> "D3Color":
        return self._transform("with_opacity", opacity)

    def opacity(self) -> float:
        return self.a

    @classmethod
    def from_hsl(cls, h: float, s: float, l: float) -> "D3Color":
        return cls._from_tuple(_native.d3_color_from_hsl(h, s, l))

    def to_lab(self) -> "Lab":
        return Lab._from_tuple(_native.d3_color_to_lab(self._tuple()))

    @classmethod
    def from_lab(cls, l: float, a: float, b: float) -> "D3Color":
        return Lab.new(l, a, b).to_rgb()

    def to_hcl(self) -> "Hcl":
        return Hcl._from_tuple(_native.d3_color_to_hcl(self._tuple()))

    @classmethod
    def from_hcl(cls, h: float, c: float, l: float) -> "D3Color":
        return Hcl.new(h, c, l).to_rgb()


@dataclass(frozen=True)
class Hsl:
    """Immutable native-backed HSL interpolation color value."""

    h: float
    s: float
    l: float
    a: float = 1.0

    def __post_init__(self) -> None:
        for name in ("h", "s", "l", "a"):
            value = float(getattr(self, name))
            if not isfinite(value):
                raise ValueError(f"Hsl.{name} must be finite")
            object.__setattr__(self, name, value)

    @classmethod
    def _from_tuple(cls, value: Sequence[float]) -> "Hsl":
        return cls(float(value[0]), float(value[1]), float(value[2]), float(value[3]))

    def _tuple(self) -> tuple[float, float, float, float]:
        return self.h, self.s, self.l, self.a

    @classmethod
    def new(cls, h: float, s: float, l: float) -> "Hsl":
        return cls._from_tuple(_native.interpolate_hsl_value_new(h, s, l))

    @classmethod
    def from_rgb(cls, color: D3Color) -> "Hsl":
        if not isinstance(color, D3Color):
            raise TypeError("color must be D3Color")
        return cls._from_tuple(_native.interpolate_hsl_value_from_color(color._tuple()))

    def to_rgb(self) -> D3Color:
        return D3Color._from_tuple(_native.interpolate_hsl_value_to_color(self._tuple()))


@dataclass(frozen=True)
class Cubehelix:
    """Immutable native-backed perceptual Cubehelix color value."""

    h: float
    s: float
    l: float
    alpha: float = 1.0

    def __post_init__(self) -> None:
        for name in ("h", "s", "l", "alpha"):
            value = float(getattr(self, name))
            if not isfinite(value):
                raise ValueError(f"Cubehelix.{name} must be finite")
            object.__setattr__(self, name, value)

    @classmethod
    def _from_tuple(cls, value: Sequence[float]) -> "Cubehelix":
        return cls(float(value[0]), float(value[1]), float(value[2]), float(value[3]))

    def _tuple(self) -> tuple[float, float, float, float]:
        return self.h, self.s, self.l, self.alpha

    @classmethod
    def new(cls, h: float, s: float, l: float) -> "Cubehelix":
        return cls._from_tuple(_native.interpolate_cubehelix_value_new(h, s, l))

    @classmethod
    def from_rgb(cls, color: D3Color) -> "Cubehelix":
        if not isinstance(color, D3Color):
            raise TypeError("color must be D3Color")
        return cls._from_tuple(
            _native.interpolate_cubehelix_value_from_color(color._tuple())
        )

    def to_rgb(self) -> D3Color:
        return D3Color._from_tuple(
            _native.interpolate_cubehelix_value_to_color(self._tuple())
        )


def cubehelix_default(t: float) -> D3Color:
    return D3Color._from_tuple(_native.interpolate_cubehelix_default(t))


def cubehelix_custom(
    start: float, rotations: float, hue: float, gamma: float, t: float
) -> D3Color:
    return D3Color._from_tuple(
        _native.interpolate_cubehelix_custom(start, rotations, hue, gamma, t)
    )


@dataclass(frozen=True)
class Lab:
    """Immutable native-backed CIELAB color value."""

    l: float
    a: float
    b: float
    alpha: float = 1.0

    def __post_init__(self) -> None:
        for name in ("l", "a", "b", "alpha"):
            value = float(getattr(self, name))
            if not isfinite(value):
                raise ValueError(f"Lab.{name} must be finite")
            object.__setattr__(self, name, value)

    @classmethod
    def _from_tuple(cls, value: Sequence[float]) -> "Lab":
        return cls(float(value[0]), float(value[1]), float(value[2]), float(value[3]))

    def _tuple(self) -> tuple[float, float, float, float]:
        return self.l, self.a, self.b, self.alpha

    @classmethod
    def new(cls, l: float, a: float, b: float) -> "Lab":
        return cls._from_tuple(_native.d3_lab_create(l, a, b))

    @classmethod
    def with_alpha(cls, l: float, a: float, b: float, alpha: float) -> "Lab":
        return cls._from_tuple(_native.d3_lab_create(l, a, b, alpha))

    @classmethod
    def from_rgb(cls, color: D3Color) -> "Lab":
        if not isinstance(color, D3Color):
            raise TypeError("color must be D3Color")
        return cls._from_tuple(_native.d3_lab_from_color(color._tuple()))

    def to_rgb(self) -> D3Color:
        return D3Color._from_tuple(_native.d3_lab_to_color(self._tuple()))

    def delta_e(self, other: "Lab") -> float:
        if not isinstance(other, Lab):
            raise TypeError("other must be Lab")
        return _native.d3_lab_delta_e(self._tuple(), other._tuple())

    def chroma(self) -> float:
        return _native.d3_lab_chroma(self._tuple())


@dataclass(frozen=True)
class Hcl:
    """Immutable native-backed hue-chroma-luminance color value."""

    h: float
    c: float
    l: float
    alpha: float = 1.0

    def __post_init__(self) -> None:
        for name in ("h", "c", "l", "alpha"):
            value = float(getattr(self, name))
            if not isfinite(value):
                raise ValueError(f"Hcl.{name} must be finite")
            object.__setattr__(self, name, value)

    @classmethod
    def _from_tuple(cls, value: Sequence[float]) -> "Hcl":
        return cls(float(value[0]), float(value[1]), float(value[2]), float(value[3]))

    def _tuple(self) -> tuple[float, float, float, float]:
        return self.h, self.c, self.l, self.alpha

    @classmethod
    def new(cls, h: float, c: float, l: float) -> "Hcl":
        return cls._from_tuple(_native.d3_hcl_create(h, c, l))

    @classmethod
    def with_alpha(cls, h: float, c: float, l: float, alpha: float) -> "Hcl":
        return cls._from_tuple(_native.d3_hcl_create(h, c, l, alpha))

    @classmethod
    def from_lab(cls, lab: Lab) -> "Hcl":
        if not isinstance(lab, Lab):
            raise TypeError("lab must be Lab")
        return cls._from_tuple(_native.d3_hcl_from_lab(lab._tuple()))

    @classmethod
    def from_rgb(cls, color: D3Color) -> "Hcl":
        if not isinstance(color, D3Color):
            raise TypeError("color must be D3Color")
        return cls._from_tuple(_native.d3_hcl_from_color(color._tuple()))

    def to_lab(self) -> Lab:
        return Lab._from_tuple(_native.d3_hcl_to_lab(self._tuple()))

    def to_rgb(self) -> D3Color:
        return D3Color._from_tuple(_native.d3_hcl_to_color(self._tuple()))

    def interpolate(self, other: "Hcl", t: float) -> "Hcl":
        if not isinstance(other, Hcl):
            raise TypeError("other must be Hcl")
        return Hcl._from_tuple(
            _native.d3_hcl_interpolate(self._tuple(), other._tuple(), t, False)
        )

    def interpolate_long(self, other: "Hcl", t: float) -> "Hcl":
        if not isinstance(other, Hcl):
            raise TypeError("other must be Hcl")
        return Hcl._from_tuple(
            _native.d3_hcl_interpolate(self._tuple(), other._tuple(), t, True)
        )


@dataclass(frozen=True, init=False)
class ColorScheme:
    """Immutable native d3rs categorical color scheme."""

    _colors: tuple[D3Color, ...]

    def __init__(self, colors: Sequence[D3Color]) -> None:
        resolved = tuple(colors)
        if any(not isinstance(color, D3Color) for color in resolved):
            raise TypeError("colors must contain only D3Color values")
        object.__setattr__(self, "_colors", resolved)

    @classmethod
    def new(cls, colors: Sequence[D3Color]) -> "ColorScheme":
        return cls(colors)

    @classmethod
    def _named(cls, name: str) -> "ColorScheme":
        return cls(tuple(D3Color._from_tuple(value) for value in _native.d3_color_scheme(name)))

    @classmethod
    def category10(cls) -> "ColorScheme":
        return cls._named("category10")

    @classmethod
    def tableau10(cls) -> "ColorScheme":
        return cls._named("tableau10")

    @classmethod
    def pastel(cls) -> "ColorScheme":
        return cls._named("pastel")

    def color(self, index: int) -> D3Color:
        index = _scale_index(index, "color scheme")
        return D3Color._from_tuple(
            _native.d3_color_scheme_color(
                [color._tuple() for color in self._colors], index
            )
        )

    def len(self) -> int:
        return len(self._colors)

    def __len__(self) -> int:
        return self.len()

    def is_empty(self) -> bool:
        return not self._colors

    def colors(self) -> tuple[D3Color, ...]:
        return self._colors


def interpolate_colors(colors: Sequence[D3Color], t: float) -> D3Color:
    resolved = tuple(colors)
    if any(not isinstance(color, D3Color) for color in resolved):
        raise TypeError("colors must contain only D3Color values")
    return D3Color._from_tuple(
        _native.d3_interpolate_colors([color._tuple() for color in resolved], t)
    )


def sequential_color(t: float) -> D3Color:
    return D3Color._from_tuple(_native.d3_sequential_color(t))


@dataclass(frozen=True, init=False)
class SequentialScale:
    """Immutable native HCL sequential color scale."""

    _colors: tuple[Hcl, ...] | None
    _name: str
    _scheme: str | None

    def __init__(self, colors: Sequence[Hcl], name: str) -> None:
        resolved = tuple(colors)
        if any(not isinstance(color, Hcl) for color in resolved):
            raise TypeError("colors must contain only Hcl values")
        if not isinstance(name, str):
            raise TypeError("name must be str")
        object.__setattr__(self, "_colors", resolved)
        object.__setattr__(self, "_name", name)
        object.__setattr__(self, "_scheme", None)

    @classmethod
    def new(cls, colors: Sequence[Hcl], name: str) -> "SequentialScale":
        return cls(colors, name)

    @classmethod
    def _named(cls, name: str) -> "SequentialScale":
        canonical = _native.d3_sequential_scheme_name(name)
        if canonical is None:
            raise ValueError(f"unknown sequential color scheme {name!r}")
        value = object.__new__(cls)
        object.__setattr__(value, "_colors", None)
        object.__setattr__(value, "_name", canonical)
        object.__setattr__(value, "_scheme", name)
        return value

    def _native_colors(self) -> list[tuple[float, float, float, float]] | None:
        if self._colors is None:
            return None
        return [color._tuple() for color in self._colors]

    def get(self, t: float) -> D3Color:
        return D3Color._from_tuple(
            _native.d3_sequential_scale_get(self._native_colors(), self._scheme, t)
        )

    def name(self) -> str:
        return self._name

    def sample(self, n: int) -> list[D3Color]:
        n = _scale_index(n, "sequential scale sample count")
        return [
            D3Color._from_tuple(value)
            for value in _native.d3_sequential_scale_sample(
                self._native_colors(), self._scheme, n
            )
        ]


class SequentialScheme:
    """Native named sequential chromatic schemes."""

    @staticmethod
    def blues() -> SequentialScale:
        return SequentialScale._named("Blues")

    @staticmethod
    def greens() -> SequentialScale:
        return SequentialScale._named("Greens")

    @staticmethod
    def reds() -> SequentialScale:
        return SequentialScale._named("Reds")

    @staticmethod
    def purples() -> SequentialScale:
        return SequentialScale._named("Purples")

    @staticmethod
    def oranges() -> SequentialScale:
        return SequentialScale._named("Oranges")

    @staticmethod
    def viridis() -> SequentialScale:
        return SequentialScale._named("Viridis")

    @staticmethod
    def magma() -> SequentialScale:
        return SequentialScale._named("Magma")

    @staticmethod
    def inferno() -> SequentialScale:
        return SequentialScale._named("Inferno")

    @staticmethod
    def plasma() -> SequentialScale:
        return SequentialScale._named("Plasma")

    @staticmethod
    def turbo() -> SequentialScale:
        return SequentialScale._named("Turbo")

    @staticmethod
    def bu_pu() -> SequentialScale:
        return SequentialScale._named("BuPu")

    @staticmethod
    def cubehelix() -> SequentialScale:
        return SequentialScale._named("Cubehelix")

    @staticmethod
    def get(name: str) -> SequentialScale | None:
        if not isinstance(name, str):
            raise TypeError("name must be str")
        if _native.d3_sequential_scheme_name(name) is None:
            return None
        return SequentialScale._named(name)


@dataclass(frozen=True, init=False)
class DivergingScale:
    """Immutable native HCL diverging color scale."""

    _negative: tuple[Hcl, ...] | None
    _neutral: Hcl | None
    _positive: tuple[Hcl, ...] | None
    _name: str
    _scheme: str | None

    def __init__(
        self,
        negative: Sequence[Hcl],
        neutral: Hcl,
        positive: Sequence[Hcl],
        name: str,
    ) -> None:
        negative_values = tuple(negative)
        positive_values = tuple(positive)
        if any(not isinstance(color, Hcl) for color in negative_values):
            raise TypeError("negative must contain only Hcl values")
        if not isinstance(neutral, Hcl):
            raise TypeError("neutral must be Hcl")
        if any(not isinstance(color, Hcl) for color in positive_values):
            raise TypeError("positive must contain only Hcl values")
        if not isinstance(name, str):
            raise TypeError("name must be str")
        object.__setattr__(self, "_negative", negative_values)
        object.__setattr__(self, "_neutral", neutral)
        object.__setattr__(self, "_positive", positive_values)
        object.__setattr__(self, "_name", name)
        object.__setattr__(self, "_scheme", None)

    @classmethod
    def new(
        cls,
        negative: Sequence[Hcl],
        neutral: Hcl,
        positive: Sequence[Hcl],
        name: str,
    ) -> "DivergingScale":
        return cls(negative, neutral, positive, name)

    @classmethod
    def _named(cls, name: str) -> "DivergingScale":
        canonical = _native.d3_diverging_scheme_name(name)
        if canonical is None:
            raise ValueError(f"unknown diverging color scheme {name!r}")
        value = object.__new__(cls)
        object.__setattr__(value, "_negative", None)
        object.__setattr__(value, "_neutral", None)
        object.__setattr__(value, "_positive", None)
        object.__setattr__(value, "_name", canonical)
        object.__setattr__(value, "_scheme", name)
        return value

    def _native_stops(
        self,
    ) -> tuple[
        list[tuple[float, float, float, float]] | None,
        tuple[float, float, float, float] | None,
        list[tuple[float, float, float, float]] | None,
    ]:
        if self._negative is None or self._neutral is None or self._positive is None:
            return None, None, None
        return (
            [color._tuple() for color in self._negative],
            self._neutral._tuple(),
            [color._tuple() for color in self._positive],
        )

    def get(self, t: float) -> D3Color:
        negative, neutral, positive = self._native_stops()
        return D3Color._from_tuple(
            _native.d3_diverging_scale_get(
                negative, neutral, positive, self._scheme, t
            )
        )

    def name(self) -> str:
        return self._name

    def sample(self, n: int) -> list[D3Color]:
        n = _scale_index(n, "diverging scale sample count")
        negative, neutral, positive = self._native_stops()
        return [
            D3Color._from_tuple(value)
            for value in _native.d3_diverging_scale_sample(
                negative, neutral, positive, self._scheme, n
            )
        ]


class DivergingScheme:
    """Native named diverging chromatic schemes."""

    @staticmethod
    def rd_bu() -> DivergingScale:
        return DivergingScale._named("RdBu")

    @staticmethod
    def rd_yl_bu() -> DivergingScale:
        return DivergingScale._named("RdYlBu")

    @staticmethod
    def rd_yl_gn() -> DivergingScale:
        return DivergingScale._named("RdYlGn")

    @staticmethod
    def pi_yg() -> DivergingScale:
        return DivergingScale._named("PiYG")

    @staticmethod
    def br_bg() -> DivergingScale:
        return DivergingScale._named("BrBG")

    @staticmethod
    def pu_or() -> DivergingScale:
        return DivergingScale._named("PuOr")

    @staticmethod
    def spectral() -> DivergingScale:
        return DivergingScale._named("Spectral")

    @staticmethod
    def get(name: str) -> DivergingScale | None:
        if not isinstance(name, str):
            raise TypeError("name must be str")
        if _native.d3_diverging_scheme_name(name) is None:
            return None
        return DivergingScale._named(name)


@runtime_checkable
class Scale(Protocol):
    """Structural protocol implemented by continuous numeric scale builders."""

    def scale(self, value: float) -> float: ...

    def invert(self, value: float) -> float | None: ...

    def ticks(self, count: int) -> list[float]: ...

    @property
    def domain_values(self) -> tuple[float, float]: ...

    @property
    def range_values(self) -> tuple[float, float]: ...


@dataclass(frozen=True, init=False)
class LinearScale:
    """Immutable builder for the complete stable d3rs linear scale."""

    _domain: tuple[float, float] = (0.0, 1.0)
    _range: tuple[float, float] = (0.0, 1.0)
    _clamp: bool = False

    def __init__(self) -> None:
        object.__setattr__(self, "_domain", (0.0, 1.0))
        object.__setattr__(self, "_range", (0.0, 1.0))
        object.__setattr__(self, "_clamp", False)

    def _updated(self, **changes: object) -> "LinearScale":
        updated = object.__new__(type(self))
        object.__setattr__(updated, "_domain", changes.get("_domain", self._domain))
        object.__setattr__(updated, "_range", changes.get("_range", self._range))
        object.__setattr__(updated, "_clamp", changes.get("_clamp", self._clamp))
        return updated

    def domain(self, min: float, max: float) -> "LinearScale":
        minimum, maximum = float(min), float(max)
        if not isfinite(minimum) or not isfinite(maximum):
            raise ValueError("linear scale domain endpoints must be finite")
        return self._updated(_domain=(minimum, maximum))

    def range(self, min: float, max: float) -> "LinearScale":
        minimum, maximum = float(min), float(max)
        if not isfinite(minimum) or not isfinite(maximum):
            raise ValueError("linear scale range endpoints must be finite")
        return self._updated(_range=(minimum, maximum))

    def range_normalized(self, max: float) -> "LinearScale":
        return self.range(0.0, max)

    def clamp(self, enabled: bool) -> "LinearScale":
        if not isinstance(enabled, bool):
            raise TypeError("linear scale clamp must be bool")
        return self._updated(_clamp=enabled)

    def nice(self, count: int | None = None) -> "LinearScale":
        if count is not None and (
            not isinstance(count, int) or isinstance(count, bool) or count <= 0
        ):
            raise ValueError("linear scale nice count must be a positive integer or None")
        return self._updated(_domain=_native.linear_scale_nice(self._domain, count))

    def copy(self) -> "LinearScale":
        return self

    def domain_min(self) -> float:
        return self._domain[0]

    def domain_max(self) -> float:
        return self._domain[1]

    def is_clamped(self) -> bool:
        return self._clamp

    def scale(self, value: float) -> float:
        return _native.linear_scale(
            value, domain=self._domain, range=self._range, clamp=self._clamp
        )

    def invert(self, value: float) -> float | None:
        return _native.linear_scale_invert(
            value, domain=self._domain, range=self._range, clamp=self._clamp
        )

    def ticks(self, count: int = 10) -> list[float]:
        if not isinstance(count, int) or isinstance(count, bool) or count < 0:
            raise ValueError("linear scale tick count must be a non-negative integer")
        return _native.linear_scale_ticks(self._domain, count)

    @property
    def domain_values(self) -> tuple[float, float]:
        return self._domain

    @property
    def range_values(self) -> tuple[float, float]:
        return self._range


@dataclass(frozen=True, init=False)
class LogScale:
    """Immutable builder for positive logarithmic d3rs scales."""

    _domain: tuple[float, float] = (1.0, 10.0)
    _range: tuple[float, float] = (0.0, 1.0)
    _base: float = 10.0
    _clamp: bool = True

    def __init__(self) -> None:
        object.__setattr__(self, "_domain", (1.0, 10.0))
        object.__setattr__(self, "_range", (0.0, 1.0))
        object.__setattr__(self, "_base", 10.0)
        object.__setattr__(self, "_clamp", True)

    def _updated(self, **changes: object) -> "LogScale":
        updated = object.__new__(type(self))
        object.__setattr__(updated, "_domain", changes.get("_domain", self._domain))
        object.__setattr__(updated, "_range", changes.get("_range", self._range))
        object.__setattr__(updated, "_base", changes.get("_base", self._base))
        object.__setattr__(updated, "_clamp", changes.get("_clamp", self._clamp))
        return updated

    def domain(self, min: float, max: float) -> "LogScale":
        minimum, maximum = float(min), float(max)
        if (
            not isfinite(minimum)
            or not isfinite(maximum)
            or minimum <= 0.0
            or maximum <= 0.0
            or minimum == maximum
        ):
            raise ValueError(
                "log scale domain endpoints must be finite, positive, and different"
            )
        return self._updated(_domain=(minimum, maximum))

    def range(self, min: float, max: float) -> "LogScale":
        minimum, maximum = float(min), float(max)
        if (
            not isfinite(minimum)
            or not isfinite(maximum)
            or minimum == maximum
        ):
            raise ValueError("log scale range endpoints must be finite and different")
        return self._updated(_range=(minimum, maximum))

    def range_normalized(self, max: float) -> "LogScale":
        return self.range(0.0, max)

    def base(self, base: float) -> "LogScale":
        resolved = float(base)
        if not isfinite(resolved) or resolved <= 0.0 or resolved == 1.0:
            raise ValueError("log scale base must be finite, positive, and different from 1")
        return self._updated(_base=resolved)

    def clamp(self, enabled: bool) -> "LogScale":
        if not isinstance(enabled, bool):
            raise TypeError("log scale clamp must be bool")
        return self._updated(_clamp=enabled)

    def is_clamped(self) -> bool:
        return self._clamp

    def scale(self, value: float) -> float:
        return _native.log_scale(
            value,
            domain=self._domain,
            range=self._range,
            base=self._base,
            clamp=self._clamp,
        )

    def invert(self, value: float) -> float | None:
        return _native.log_scale_invert(
            value,
            domain=self._domain,
            range=self._range,
            base=self._base,
            clamp=self._clamp,
        )

    def ticks(self, count: int = 10) -> list[float]:
        if not isinstance(count, int) or isinstance(count, bool) or count < 0:
            raise ValueError("log scale tick count must be a non-negative integer")
        return _native.log_scale_ticks(self._domain, count, self._base)

    @property
    def domain_values(self) -> tuple[float, float]:
        return self._domain

    @property
    def range_values(self) -> tuple[float, float]:
        return self._range


@dataclass(frozen=True, init=False)
class PowScale:
    """Immutable builder for sign-preserving power scales."""

    _domain: tuple[float, float] = (0.0, 1.0)
    _range: tuple[float, float] = (0.0, 1.0)
    _exponent: float = 1.0
    _clamp: bool = False

    def __init__(self) -> None:
        object.__setattr__(self, "_domain", (0.0, 1.0))
        object.__setattr__(self, "_range", (0.0, 1.0))
        object.__setattr__(self, "_exponent", 1.0)
        object.__setattr__(self, "_clamp", False)

    def _updated(self, **changes: object) -> "PowScale":
        updated = object.__new__(type(self))
        object.__setattr__(updated, "_domain", changes.get("_domain", self._domain))
        object.__setattr__(updated, "_range", changes.get("_range", self._range))
        object.__setattr__(
            updated, "_exponent", changes.get("_exponent", self._exponent)
        )
        object.__setattr__(updated, "_clamp", changes.get("_clamp", self._clamp))
        return updated

    def domain(self, min: float, max: float) -> "PowScale":
        minimum, maximum = float(min), float(max)
        if not isfinite(minimum) or not isfinite(maximum):
            raise ValueError("power scale domain values must be finite")
        return self._updated(_domain=(minimum, maximum))

    def range(self, min: float, max: float) -> "PowScale":
        minimum, maximum = float(min), float(max)
        if not isfinite(minimum) or not isfinite(maximum):
            raise ValueError("power scale range values must be finite")
        return self._updated(_range=(minimum, maximum))

    def exponent(self, exp: float) -> "PowScale":
        resolved = float(exp)
        if not isfinite(resolved) or resolved <= 0.0:
            raise ValueError("power scale exponent must be positive and finite")
        return self._updated(_exponent=resolved)

    def clamp(self, enabled: bool) -> "PowScale":
        if not isinstance(enabled, bool):
            raise TypeError("power scale clamp must be bool")
        return self._updated(_clamp=enabled)

    def nice(self, count: int | None = None) -> "PowScale":
        if count is not None and (
            not isinstance(count, int) or isinstance(count, bool) or count <= 0
        ):
            raise ValueError("power scale nice count must be a positive integer or None")
        return self._updated(_domain=_native.pow_scale_nice(self._domain, count))

    def copy(self) -> "PowScale":
        return self

    def domain_min(self) -> float:
        return self._domain[0]

    def domain_max(self) -> float:
        return self._domain[1]

    def exponent_value(self) -> float:
        return self._exponent

    def is_clamped(self) -> bool:
        return self._clamp

    def scale(self, value: float) -> float:
        return _native.pow_scale(
            value,
            domain=self._domain,
            range=self._range,
            exponent=self._exponent,
            clamp=self._clamp,
        )

    def invert(self, value: float) -> float | None:
        return _native.pow_scale_invert(
            value,
            domain=self._domain,
            range=self._range,
            exponent=self._exponent,
            clamp=self._clamp,
        )

    def ticks(self, count: int = 10) -> list[float]:
        if not isinstance(count, int) or isinstance(count, bool) or count < 0:
            raise ValueError("power scale tick count must be a non-negative integer")
        return _native.pow_scale_ticks(self._domain, count)

    @property
    def domain_values(self) -> tuple[float, float]:
        return self._domain

    @property
    def range_values(self) -> tuple[float, float]:
        return self._range


SqrtScale = PowScale


def sqrt_scale() -> PowScale:
    """Create the native sqrt-scale configuration (power exponent 0.5)."""

    return PowScale().exponent(0.5)


@dataclass(frozen=True, init=False)
class SymlogScale:
    """Immutable symmetric-log scale supporting negative and zero values."""

    _domain: tuple[float, float] = (0.0, 1.0)
    _range: tuple[float, float] = (0.0, 1.0)
    _constant: float = 1.0
    _clamp: bool = False

    def __init__(self) -> None:
        object.__setattr__(self, "_domain", (0.0, 1.0))
        object.__setattr__(self, "_range", (0.0, 1.0))
        object.__setattr__(self, "_constant", 1.0)
        object.__setattr__(self, "_clamp", False)

    def _updated(self, **changes: object) -> "SymlogScale":
        updated = object.__new__(type(self))
        object.__setattr__(updated, "_domain", changes.get("_domain", self._domain))
        object.__setattr__(updated, "_range", changes.get("_range", self._range))
        object.__setattr__(
            updated, "_constant", changes.get("_constant", self._constant)
        )
        object.__setattr__(updated, "_clamp", changes.get("_clamp", self._clamp))
        return updated

    def domain(self, min: float, max: float) -> "SymlogScale":
        minimum, maximum = float(min), float(max)
        if not isfinite(minimum) or not isfinite(maximum):
            raise ValueError("symlog scale domain values must be finite")
        return self._updated(_domain=(minimum, maximum))

    def range(self, min: float, max: float) -> "SymlogScale":
        minimum, maximum = float(min), float(max)
        if not isfinite(minimum) or not isfinite(maximum):
            raise ValueError("symlog scale range values must be finite")
        return self._updated(_range=(minimum, maximum))

    def constant(self, c: float) -> "SymlogScale":
        resolved = float(c)
        if not isfinite(resolved) or resolved <= 0.0:
            raise ValueError("symlog scale constant must be positive and finite")
        return self._updated(_constant=resolved)

    def clamp(self, enabled: bool) -> "SymlogScale":
        if not isinstance(enabled, bool):
            raise TypeError("symlog scale clamp must be bool")
        return self._updated(_clamp=enabled)

    def nice(self, count: int | None = None) -> "SymlogScale":
        if count is not None and (
            not isinstance(count, int) or isinstance(count, bool) or count <= 0
        ):
            raise ValueError("symlog scale nice count must be a positive integer or None")
        return self._updated(_domain=_native.symlog_scale_nice(self._domain, count))

    def copy(self) -> "SymlogScale":
        return self

    def domain_min(self) -> float:
        return self._domain[0]

    def domain_max(self) -> float:
        return self._domain[1]

    def constant_value(self) -> float:
        return self._constant

    def is_clamped(self) -> bool:
        return self._clamp

    def scale(self, value: float) -> float:
        return _native.symlog_scale(
            value,
            domain=self._domain,
            range=self._range,
            constant=self._constant,
            clamp=self._clamp,
        )

    def invert(self, value: float) -> float | None:
        return _native.symlog_scale_invert(
            value,
            domain=self._domain,
            range=self._range,
            constant=self._constant,
            clamp=self._clamp,
        )

    def ticks(self, count: int = 10) -> list[float]:
        if not isinstance(count, int) or isinstance(count, bool) or count < 0:
            raise ValueError("symlog scale tick count must be a non-negative integer")
        return _native.symlog_scale_ticks(self._domain, count)

    @property
    def domain_values(self) -> tuple[float, float]:
        return self._domain

    @property
    def range_values(self) -> tuple[float, float]:
        return self._range


def _scale_index(index: int, name: str) -> int:
    if not isinstance(index, int) or isinstance(index, bool) or index < 0:
        raise ValueError(f"{name} index must be a non-negative integer")
    return index


@dataclass(frozen=True, init=False)
class ThresholdScale(Generic[_T]):
    """Immutable explicit-threshold scale with arbitrary Python range values."""

    _thresholds: tuple[float, ...] = ()
    _range_values: tuple[_T, ...] = ()

    def __init__(self) -> None:
        object.__setattr__(self, "_thresholds", ())
        object.__setattr__(self, "_range_values", ())

    @classmethod
    def with_range(cls, range: Sequence[_T]) -> "ThresholdScale[_T]":
        return cls().range(range)

    def _updated(self, **changes: object) -> "ThresholdScale[_T]":
        updated = object.__new__(type(self))
        object.__setattr__(
            updated, "_thresholds", changes.get("_thresholds", self._thresholds)
        )
        object.__setattr__(
            updated,
            "_range_values",
            changes.get("_range_values", self._range_values),
        )
        return updated

    def domain(self, thresholds: Sequence[float]) -> "ThresholdScale[_T]":
        resolved = tuple(float(value) for value in thresholds)
        if any(not isfinite(value) for value in resolved):
            raise ValueError("threshold scale thresholds must be finite")
        if any(left >= right for left, right in zip(resolved, resolved[1:])):
            raise ValueError("threshold scale thresholds must be strictly increasing")
        return self._updated(_thresholds=resolved)

    def range(self, values: Sequence[_U]) -> "ThresholdScale[_U]":
        return self._updated(_range_values=tuple(values))  # type: ignore[return-value]

    def thresholds(self) -> tuple[float, ...]:
        return self._thresholds

    def range_values(self) -> tuple[_T, ...]:
        return self._range_values

    def invert_extent(self, index: int) -> tuple[float, float] | None:
        return _native.threshold_scale_invert_extent(
            self._thresholds,
            len(self._range_values),
            _scale_index(index, "threshold scale"),
        )

    def copy(self) -> "ThresholdScale[_T]":
        return self

    def scale(self, value: float) -> _T:
        index = _native.threshold_scale_index(
            float(value), self._thresholds, len(self._range_values)
        )
        return self._range_values[index]

    def invert(self, _value: _T) -> None:
        return None

    def ticks(self, count: int = 10) -> list[float]:
        _scale_index(count, "threshold scale tick count")
        return list(self._thresholds)


@dataclass(frozen=True, init=False)
class QuantizeScale(Generic[_T]):
    """Immutable uniform continuous-to-discrete scale."""

    _domain: tuple[float, float] = (0.0, 1.0)
    _range_values: tuple[_T, ...] = ()

    def __init__(self) -> None:
        object.__setattr__(self, "_domain", (0.0, 1.0))
        object.__setattr__(self, "_range_values", ())

    @classmethod
    def with_range(cls, range: Sequence[_T]) -> "QuantizeScale[_T]":
        return cls().range(range)

    def _updated(self, **changes: object) -> "QuantizeScale[_T]":
        updated = object.__new__(type(self))
        object.__setattr__(updated, "_domain", changes.get("_domain", self._domain))
        object.__setattr__(
            updated,
            "_range_values",
            changes.get("_range_values", self._range_values),
        )
        return updated

    def domain(self, min: float, max: float) -> "QuantizeScale[_T]":
        minimum, maximum = float(min), float(max)
        if not isfinite(minimum) or not isfinite(maximum):
            raise ValueError("quantize scale domain values must be finite")
        return self._updated(_domain=(minimum, maximum))

    def range(self, values: Sequence[_U]) -> "QuantizeScale[_U]":
        return self._updated(_range_values=tuple(values))  # type: ignore[return-value]

    def domain_min(self) -> float:
        return self._domain[0]

    def domain_max(self) -> float:
        return self._domain[1]

    def range_values(self) -> tuple[_T, ...]:
        return self._range_values

    def thresholds(self) -> list[float]:
        return _native.quantize_scale_thresholds(
            self._domain, len(self._range_values)
        )

    def invert_extent(self, index: int) -> tuple[float, float] | None:
        return _native.quantize_scale_invert_extent(
            self._domain,
            len(self._range_values),
            _scale_index(index, "quantize scale"),
        )

    def copy(self) -> "QuantizeScale[_T]":
        return self

    def scale(self, value: float) -> _T:
        index = _native.quantize_scale_index(
            float(value), self._domain, len(self._range_values)
        )
        return self._range_values[index]

    def invert(self, _value: _T) -> None:
        return None

    def ticks(self, count: int = 10) -> list[float]:
        _scale_index(count, "quantize scale tick count")
        return self.thresholds()

    @property
    def domain_values(self) -> tuple[float, float]:
        return self._domain


@dataclass(frozen=True, init=False)
class QuantileScale(Generic[_T]):
    """Immutable sample-quantile scale with arbitrary Python range values."""

    _domain_samples: tuple[float, ...] = ()
    _range_values: tuple[_T, ...] = ()

    def __init__(self) -> None:
        object.__setattr__(self, "_domain_samples", ())
        object.__setattr__(self, "_range_values", ())

    @classmethod
    def with_range(cls, range: Sequence[_T]) -> "QuantileScale[_T]":
        return cls().range(range)

    def _updated(self, **changes: object) -> "QuantileScale[_T]":
        updated = object.__new__(type(self))
        object.__setattr__(
            updated,
            "_domain_samples",
            changes.get("_domain_samples", self._domain_samples),
        )
        object.__setattr__(
            updated,
            "_range_values",
            changes.get("_range_values", self._range_values),
        )
        return updated

    def domain(self, samples: Sequence[float]) -> "QuantileScale[_T]":
        resolved = tuple(float(value) for value in samples)
        if any(value in (float("inf"), float("-inf")) for value in resolved):
            raise ValueError("quantile scale samples must not be infinite")
        return self._updated(
            _domain_samples=tuple(sorted(value for value in resolved if not isnan(value)))
        )

    def range(self, values: Sequence[_U]) -> "QuantileScale[_U]":
        return self._updated(_range_values=tuple(values))  # type: ignore[return-value]

    def domain_samples(self) -> tuple[float, ...]:
        return self._domain_samples

    def range_values(self) -> tuple[_T, ...]:
        return self._range_values

    def quantiles(self) -> list[float]:
        _, quantiles = _native.quantile_scale_prepare(
            self._domain_samples, len(self._range_values)
        )
        return quantiles

    def invert_extent(self, index: int) -> tuple[float, float] | None:
        return _native.quantile_scale_invert_extent(
            self._domain_samples,
            len(self._range_values),
            _scale_index(index, "quantile scale"),
        )

    def copy(self) -> "QuantileScale[_T]":
        return self

    def scale(self, value: float) -> _T:
        index = _native.quantile_scale_index(
            float(value), self._domain_samples, len(self._range_values)
        )
        return self._range_values[index]

    def invert(self, _value: _T) -> None:
        return None

    def ticks(self, count: int = 10) -> list[float]:
        _scale_index(count, "quantile scale tick count")
        return self.quantiles()


_ORDINAL_UNSET = object()


def _categorical_domain(values: Sequence[_K], name: str) -> tuple[_K, ...]:
    resolved = tuple(values)
    for index, value in enumerate(resolved):
        try:
            hash(value)
        except TypeError as error:
            raise TypeError(f"{name}[{index}] must be hashable") from error
    return resolved


def _categorical_index(domain: tuple[_K, ...], value: _K) -> int | None:
    try:
        index_map = {item: index for index, item in enumerate(domain)}
        return index_map.get(value)
    except TypeError as error:
        raise TypeError("ordinal scale value must be hashable") from error


@dataclass(frozen=True, init=False)
class OrdinalScale(Generic[_K, _T]):
    """Immutable categorical-to-categorical scale."""

    _domain: tuple[_K, ...] = ()
    _range: tuple[_T, ...] = ()
    _unknown: object = _ORDINAL_UNSET

    def __init__(self) -> None:
        object.__setattr__(self, "_domain", ())
        object.__setattr__(self, "_range", ())
        object.__setattr__(self, "_unknown", _ORDINAL_UNSET)

    def _updated(self, **changes: object) -> "OrdinalScale[_K, _T]":
        updated = object.__new__(type(self))
        object.__setattr__(updated, "_domain", changes.get("_domain", self._domain))
        object.__setattr__(updated, "_range", changes.get("_range", self._range))
        object.__setattr__(updated, "_unknown", changes.get("_unknown", self._unknown))
        return updated

    def domain(self, domain: Sequence[_K]) -> "OrdinalScale[_K, _T]":
        return self._updated(_domain=_categorical_domain(domain, "ordinal domain"))

    def range(self, range: Sequence[_U]) -> "OrdinalScale[_K, _U]":
        return self._updated(_range=tuple(range))  # type: ignore[return-value]

    def unknown(self, unknown: _U) -> "OrdinalScale[_K, _U]":
        return self._updated(_unknown=unknown)  # type: ignore[return-value]

    def scale(self, value: _K) -> _T | None:
        index = _categorical_index(self._domain, value)
        if index is not None and self._range:
            return self._range[index % len(self._range)]
        if self._unknown is _ORDINAL_UNSET:
            return None
        return self._unknown  # type: ignore[return-value]

    def get_domain(self) -> tuple[_K, ...]:
        return self._domain

    def get_range(self) -> tuple[_T, ...]:
        return self._range


@dataclass(frozen=True, init=False)
class BandScale(Generic[_K]):
    """Immutable categorical band scale backed by native d3rs layout."""

    _domain: tuple[_K, ...] = ()
    _range: tuple[float, float] = (0.0, 1.0)
    _padding_inner: float = 0.0
    _padding_outer: float = 0.0
    _align: float = 0.5
    _round: bool = False

    def __init__(self) -> None:
        object.__setattr__(self, "_domain", ())
        object.__setattr__(self, "_range", (0.0, 1.0))
        object.__setattr__(self, "_padding_inner", 0.0)
        object.__setattr__(self, "_padding_outer", 0.0)
        object.__setattr__(self, "_align", 0.5)
        object.__setattr__(self, "_round", False)

    def _updated(self, **changes: object) -> "BandScale[_K]":
        updated = object.__new__(type(self))
        for name in (
            "_domain",
            "_range",
            "_padding_inner",
            "_padding_outer",
            "_align",
            "_round",
        ):
            object.__setattr__(updated, name, changes.get(name, getattr(self, name)))
        return updated

    def domain(self, domain: Sequence[_K]) -> "BandScale[_K]":
        return self._updated(_domain=_categorical_domain(domain, "band domain"))

    def range(self, start: float, end: float) -> "BandScale[_K]":
        resolved = (float(start), float(end))
        if any(not isfinite(value) for value in resolved):
            raise ValueError("band scale range values must be finite")
        return self._updated(_range=resolved)

    def padding_inner(self, padding: float) -> "BandScale[_K]":
        resolved = float(padding)
        if not isfinite(resolved):
            raise ValueError("band scale inner padding must be finite")
        return self._updated(_padding_inner=_builtins.max(0.0, _builtins.min(1.0, resolved)))

    def padding_outer(self, padding: float) -> "BandScale[_K]":
        resolved = float(padding)
        if not isfinite(resolved):
            raise ValueError("band scale outer padding must be finite")
        return self._updated(_padding_outer=_builtins.max(0.0, _builtins.min(1.0, resolved)))

    def padding(self, padding: float) -> "BandScale[_K]":
        return self.padding_inner(padding).padding_outer(padding)

    def align(self, align: float) -> "BandScale[_K]":
        resolved = float(align)
        if not isfinite(resolved):
            raise ValueError("band scale alignment must be finite")
        return self._updated(_align=_builtins.max(0.0, _builtins.min(1.0, resolved)))

    def round(self, round: bool) -> "BandScale[_K]":
        if not isinstance(round, bool):
            raise TypeError("band scale round must be bool")
        return self._updated(_round=round)

    def _layout(self) -> tuple[list[float], float, float]:
        return _native.band_scale_layout(
            len(self._domain),
            range=self._range,
            padding_inner=self._padding_inner,
            padding_outer=self._padding_outer,
            align=self._align,
            round=self._round,
        )

    def scale(self, value: _K) -> float | None:
        index = _categorical_index(self._domain, value)
        if index is None:
            return None
        positions, _, _ = self._layout()
        return positions[index]

    def bandwidth(self) -> float:
        return self._layout()[1]

    def step(self) -> float:
        return self._layout()[2]

    def get_domain(self) -> tuple[_K, ...]:
        return self._domain

    def get_range(self) -> tuple[float, float]:
        return self._range


@dataclass(frozen=True, init=False)
class PointScale(Generic[_K]):
    """Immutable categorical point scale backed by native d3rs layout."""

    _domain: tuple[_K, ...] = ()
    _range: tuple[float, float] = (0.0, 1.0)
    _padding: float = 0.0
    _align: float = 0.5
    _round: bool = False

    def __init__(self) -> None:
        object.__setattr__(self, "_domain", ())
        object.__setattr__(self, "_range", (0.0, 1.0))
        object.__setattr__(self, "_padding", 0.0)
        object.__setattr__(self, "_align", 0.5)
        object.__setattr__(self, "_round", False)

    def _updated(self, **changes: object) -> "PointScale[_K]":
        updated = object.__new__(type(self))
        for name in ("_domain", "_range", "_padding", "_align", "_round"):
            object.__setattr__(updated, name, changes.get(name, getattr(self, name)))
        return updated

    def domain(self, domain: Sequence[_K]) -> "PointScale[_K]":
        return self._updated(_domain=_categorical_domain(domain, "point domain"))

    def range(self, start: float, end: float) -> "PointScale[_K]":
        resolved = (float(start), float(end))
        if any(not isfinite(value) for value in resolved):
            raise ValueError("point scale range values must be finite")
        return self._updated(_range=resolved)

    def padding(self, padding: float) -> "PointScale[_K]":
        resolved = float(padding)
        if not isfinite(resolved):
            raise ValueError("point scale padding must be finite")
        return self._updated(_padding=_builtins.max(0.0, _builtins.min(1.0, resolved)))

    def align(self, align: float) -> "PointScale[_K]":
        resolved = float(align)
        if not isfinite(resolved):
            raise ValueError("point scale alignment must be finite")
        return self._updated(_align=_builtins.max(0.0, _builtins.min(1.0, resolved)))

    def round(self, round: bool) -> "PointScale[_K]":
        if not isinstance(round, bool):
            raise TypeError("point scale round must be bool")
        return self._updated(_round=round)

    def _layout(self) -> tuple[list[float], float]:
        return _native.point_scale_layout(
            len(self._domain),
            range=self._range,
            padding=self._padding,
            align=self._align,
            round=self._round,
        )

    def scale(self, value: _K) -> float | None:
        index = _categorical_index(self._domain, value)
        if index is None:
            return None
        positions, _ = self._layout()
        return positions[index]

    def step(self) -> float:
        return self._layout()[1]

    def get_domain(self) -> tuple[_K, ...]:
        return self._domain


def _require_callable(value: Any, name: str) -> None:
    if not callable(value):
        raise TypeError(f"{name} must be callable")


def _domain(value: Sequence[float], name: str) -> Domain:
    if len(value) != 2:
        raise ValueError(f"{name} must contain exactly two values")
    minimum, maximum = float(value[0]), float(value[1])
    if not isfinite(minimum) or not isfinite(maximum) or minimum >= maximum:
        raise ValueError(f"{name} must be finite and increasing")
    return minimum, maximum


class ZoomOperationKind(str, Enum):
    ZOOM_TO = "zoom_to"
    RESET = "reset"
    BACK = "back"


@dataclass(frozen=True)
class ZoomOperation:
    kind: ZoomOperationKind
    x: Domain | None = None
    y: Domain | None = None

    @classmethod
    def zoom_to(cls, x: Sequence[float], y: Sequence[float]) -> "ZoomOperation":
        return cls(ZoomOperationKind.ZOOM_TO, _domain(x, "x"), _domain(y, "y"))

    @classmethod
    def reset(cls) -> "ZoomOperation":
        return cls(ZoomOperationKind.RESET)

    @classmethod
    def back(cls) -> "ZoomOperation":
        return cls(ZoomOperationKind.BACK)

    def to_spec(self) -> dict[str, Any]:
        if self.kind is ZoomOperationKind.ZOOM_TO:
            if self.x is None or self.y is None:
                raise ValueError("zoom_to requires x and y domains")
            return {"kind": self.kind.value, "x": list(self.x), "y": list(self.y)}
        if self.x is not None or self.y is not None:
            raise ValueError(f"{self.kind.value} does not accept domains")
        return {"kind": self.kind.value}


@dataclass(frozen=True)
class ZoomRequest:
    original_x: Domain
    original_y: Domain
    operations: Sequence[ZoomOperation] = ()
    log_x: bool = False
    log_y: bool = False

    def __post_init__(self) -> None:
        object.__setattr__(self, "original_x", _domain(self.original_x, "original_x"))
        object.__setattr__(self, "original_y", _domain(self.original_y, "original_y"))

    def to_spec(self) -> dict[str, Any]:
        return {
            "original_x": list(self.original_x), "original_y": list(self.original_y),
            "log_x": self.log_x, "log_y": self.log_y,
            "operations": [operation.to_spec() for operation in self.operations],
        }

    def send(self, context: "SessionContext", request_id: str) -> None:
        """Run this request with Rust's ``ZoomState`` through the host."""
        context.command(request_id, "d3.zoom", **self.to_spec())


@dataclass(frozen=True)
class ZoomResult:
    x: Domain
    y: Domain
    zoomed: bool
    level: int
    back_results: tuple[bool, ...]

    @classmethod
    def from_command(cls, result: CommandResult) -> "ZoomResult":
        if result.status is not CommandStatus.SUCCEEDED:
            raise RuntimeError(result.error or f"d3 zoom command {result.status.value}")
        data = result.data
        return cls(
            _domain(data.get("x", ()), "x"), _domain(data.get("y", ()), "y"),
            bool(data.get("zoomed")), int(data.get("level", 0)),
            tuple(bool(value) for value in data.get("back_results", ())),
        )


class ArrayOperation(str, Enum):
    BISECT_LEFT = "bisect_left"
    BISECT_RIGHT = "bisect_right"
    QUANTILE = "quantile"


@dataclass(frozen=True)
class ArrayRequest:
    """Run one native d3-array search or quantile operation."""

    operation: ArrayOperation
    data: Sequence[float]
    value: float | None = None
    percentile: float | None = None

    def to_spec(self) -> dict[str, Any]:
        data = [float(item) for item in self.data]
        if any(not isfinite(item) for item in data):
            raise ValueError("d3 array data must be finite")
        result: dict[str, Any] = {"operation": self.operation.value, "data": data}
        if self.operation in {ArrayOperation.BISECT_LEFT, ArrayOperation.BISECT_RIGHT}:
            if self.value is None or not isfinite(float(self.value)):
                raise ValueError("bisect requires a finite value")
            result["value"] = float(self.value)
        elif self.percentile is None or not 0 <= float(self.percentile) <= 1:
            raise ValueError("quantile requires a percentile in [0, 1]")
        else:
            result["percentile"] = float(self.percentile)
        return result

    def send(self, context: "SessionContext", request_id: str) -> None:
        context.command(request_id, "d3.array", **self.to_spec())

    @staticmethod
    def value_from_command(result: CommandResult) -> float | int | None:
        if result.status is not CommandStatus.SUCCEEDED:
            raise RuntimeError(result.error or f"d3 array command {result.status.value}")
        value = result.data.get("value")
        if value is None or isinstance(value, (int, float)):
            return value
        raise ValueError("native d3 array result has an unexpected shape")


class StatisticsOperation(str, Enum):
    SUM = "sum"
    MEAN = "mean"
    MEDIAN = "median"
    VARIANCE = "variance"
    DEVIATION = "deviation"
    QUANTILE = "quantile"
    EXTENT = "extent"
    CUMSUM = "cumsum"


@dataclass(frozen=True)
class StatisticsRequest:
    operation: StatisticsOperation
    data: Sequence[float]
    percentile: float | None = None
    def send(self, context: "SessionContext", request_id: str) -> None:
        values = [float(value) for value in self.data]
        if any(not isfinite(value) for value in values): raise ValueError("statistics data must be finite")
        arguments: dict[str, Any] = {"operation": self.operation.value, "data": values}
        if self.operation is StatisticsOperation.QUANTILE:
            if self.percentile is None or not 0 <= self.percentile <= 1: raise ValueError("quantile requires percentile in [0, 1]")
            arguments["percentile"] = self.percentile
        context.command(request_id, "d3.statistics", **arguments)
    @staticmethod
    def value_from_command(result: CommandResult) -> float | list[float] | tuple[float, float] | None:
        if result.status is not CommandStatus.SUCCEEDED: raise RuntimeError(result.error or "d3 statistics failed")
        value = result.data.get("value")
        if isinstance(value, list) and len(value) == 2: return (float(value[0]), float(value[1]))
        if isinstance(value, list): return [float(item) for item in value]
        if value is None or isinstance(value, (int, float)): return None if value is None else float(value)
        raise ValueError("native d3 statistics result has an unexpected shape")


class TickOperation(str, Enum):
    TICKS = "ticks"
    STEP = "tick_step"
    INCREMENT = "tick_increment"
    NICE = "nice"
    TIME = "time_ticks"
    INTERVAL = "interval"
    LOG = "log"


@dataclass(frozen=True)
class TickRequest:
    operation: TickOperation
    start: float
    stop: float
    count: int = 10
    interval: float | None = None
    base: float = 10.0
    subdivisions: bool = True
    def send(self, context: "SessionContext", request_id: str) -> None:
        if not isfinite(self.start) or not isfinite(self.stop) or self.count < 0: raise ValueError("invalid tick range")
        context.command(request_id, "d3.ticks", operation=self.operation.value, start=self.start, stop=self.stop, count=self.count, interval=self.interval, base=self.base, subdivisions=self.subdivisions)
    @staticmethod
    def value_from_command(result: CommandResult) -> float | tuple[float, float] | list[float]:
        if result.status is not CommandStatus.SUCCEEDED: raise RuntimeError(result.error or "d3 ticks failed")
        value = result.data["value"]
        if isinstance(value, list) and len(value) == 2 and all(isinstance(item, (int, float)) for item in value): return (float(value[0]), float(value[1]))
        if isinstance(value, list): return [float(item) for item in value]
        return float(value)


class ScaleKind(str, Enum):
    LINEAR = "linear"
    LOG = "log"
    POWER = "power"
    SQRT = "sqrt"
    SYMLOG = "symlog"
    QUANTIZE = "quantize"
    QUANTILE = "quantile"
    THRESHOLD = "threshold"
    ORDINAL = "ordinal"
    BAND = "band"
    POINT = "point"


@dataclass(frozen=True)
class ScaleRequest:
    kind: ScaleKind
    domain: Sequence[float] | Sequence[str]
    range: Sequence[float] | Sequence[str]
    values: Sequence[float] | Sequence[str]
    clamp: bool = False
    base: float = 10.0
    exponent: float = 1.0
    constant: float = 1.0
    tick_count: int = 10
    padding_inner: float = 0.0
    padding_outer: float = 0.0
    align: float = 0.5
    round: bool = False
    def send(self, context: "SessionContext", request_id: str) -> None:
        categorical = self.kind in {ScaleKind.ORDINAL, ScaleKind.BAND, ScaleKind.POINT}
        discrete_range = self.kind in {ScaleKind.QUANTIZE, ScaleKind.QUANTILE, ScaleKind.THRESHOLD, ScaleKind.ORDINAL}
        domain = [str(value) for value in self.domain] if categorical else [float(value) for value in self.domain]
        values = [str(value) for value in self.values] if categorical else [float(value) for value in self.values]
        scale_range = [str(value) for value in self.range] if discrete_range else [float(value) for value in self.range]
        context.command(request_id, "d3.scale", kind=self.kind.value, domain=domain, range=scale_range, values=values, clamp=self.clamp, base=self.base, exponent=self.exponent, constant=self.constant, tick_count=self.tick_count, padding_inner=self.padding_inner, padding_outer=self.padding_outer, align=self.align, round=self.round)


@dataclass(frozen=True)
class ScaleOutput:
    values: tuple[float | str | None, ...]
    ticks: tuple[float, ...] = ()
    thresholds: tuple[float, ...] = ()
    bandwidth: float | None = None
    step: float | None = None
    @classmethod
    def from_command(cls, result: CommandResult) -> "ScaleOutput":
        if result.status is not CommandStatus.SUCCEEDED: raise RuntimeError(result.error or "d3 scale failed")
        output = result.data["output"]
        return cls(tuple(None if value is None else value for value in output["values"]), tuple(float(value) for value in output.get("ticks", ())), tuple(float(value) for value in output.get("thresholds", ())), None if output.get("bandwidth") is None else float(output["bandwidth"]), None if output.get("step") is None else float(output["step"]))


@dataclass(frozen=True)
class D3ParityEntry:
    id: str
    d3_area: str
    gpui_d3rs_modules: str
    status: str
    evidence: str
    release_requirement: str

@dataclass(frozen=True)
class D3BenchmarkCase:
    id: str
    module: str
    bench_target: str
    benchmark_group: str
    benchmark_id: str
    dataset_scale: str
    evidence: str

@dataclass(frozen=True)
class D3Reports:
    parity_entries: tuple[D3ParityEntry, ...]
    parity_markdown: str
    benchmark_cases: tuple[D3BenchmarkCase, ...]
    benchmark_markdown: str


class D3BridgeKind(str, Enum):
    DIRECT_COMMAND = "direct_command"
    CHART_SPEC = "chart_spec"
    SCENE_SPEC = "scene_spec"
    HOST_INTERACTION = "host_interaction"
    NON_CONSUMER = "non_consumer"


class AlgorithmOperation(str, Enum):
    COLOR_INTERPOLATE = "color_interpolate"
    COLOR_CONVERT = "color_convert"
    FORMAT = "format"
    FORMAT_PREFIX = "format_prefix"
    TIME_INTERVAL = "time_interval"
    TIME_SCALE = "time_scale"
    CSV_PARSE = "csv_parse"
    DSV_PARSE = "dsv_parse"
    DSV_FORMAT = "dsv_format"
    INTERPOLATE_NUMBER = "interpolate_number"
    INTERPOLATE_ARRAY = "interpolate_array"
    INTERPOLATE_STRING = "interpolate_string"
    INTERPOLATE_TRANSFORM_CSS = "interpolate_transform_css"
    INTERPOLATE_TRANSFORM_SVG = "interpolate_transform_svg"
    INTERPOLATE_ZOOM = "interpolate_zoom"
    EASE = "ease"
    SELECTION_JOIN = "selection_join"
    BRUSH_GESTURE = "brush_gesture"
    DRAG_GESTURE = "drag_gesture"
    TRANSITION_SAMPLE = "transition_sample"
    POLYGON = "polygon"
    DELAUNAY = "delaunay"
    HEXBIN = "hexbin"
    TILES = "tiles"
    CHORD = "chord"
    SANKEY = "sankey"
    FORCE = "force"
    HIERARCHY_TREEMAP = "hierarchy_treemap"
    GEO = "geo"
    QUADTREE = "quadtree"
    CONTOUR = "contour"
    LOD_M4 = "lod_m4"
    RANDOM_UNIFORM = "random_uniform"
    RANDOM = "random"
    SHUFFLE = "shuffle"


class EaseKind(str, Enum):
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

    def apply(self, t: float) -> float:
        return ease(self, t)


class RandomKind(str, Enum):
    UNIFORM = "uniform"
    NORMAL = "normal"
    LOG_NORMAL = "log_normal"
    EXPONENTIAL = "exponential"
    BERNOULLI = "bernoulli"
    POISSON = "poisson"
    IRWIN_HALL = "irwin_hall"
    BATES = "bates"


@dataclass(frozen=True)
class SankeyLinkSpec:
    source: str
    target: str
    value: float

    def to_spec(self) -> dict[str, str | float]:
        return {"source": self.source, "target": self.target, "value": self.value}


@dataclass(frozen=True)
class HierarchySpec:
    name: str
    value: float
    children: tuple["HierarchySpec", ...] = ()

    def to_spec(self) -> dict[str, Any]:
        return {
            "name": self.name,
            "value": self.value,
            "children": [child.to_spec() for child in self.children],
        }


@dataclass(frozen=True)
class AlgorithmRequest:
    operation: AlgorithmOperation
    arguments: dict[str, Any]

    @classmethod
    def easing(cls, kind: EaseKind, values: Sequence[float]) -> "AlgorithmRequest":
        return cls(AlgorithmOperation.EASE, {"kind": kind.value, "values": list(values)})

    @classmethod
    def random(
        cls,
        kind: RandomKind,
        *,
        count: int,
        seed: int,
        **parameters: float | int,
    ) -> "AlgorithmRequest":
        if count < 0:
            raise ValueError("random sample count must be non-negative")
        return cls(
            AlgorithmOperation.RANDOM,
            {"kind": kind.value, "count": count, "seed": seed, **parameters},
        )

    @classmethod
    def dsv_parse(cls, input: str, delimiter: str = ",") -> "AlgorithmRequest":
        if len(delimiter) != 1:
            raise ValueError("DSV delimiter must be one character")
        return cls(AlgorithmOperation.DSV_PARSE, {"input": input, "delimiter": delimiter})

    @classmethod
    def dsv_format(
        cls,
        rows: Sequence[dict[str, Any]],
        columns: Sequence[str],
        delimiter: str = ",",
    ) -> "AlgorithmRequest":
        if delimiter not in {",", "\t"}:
            raise ValueError("DSV formatting supports comma or tab delimiters")
        return cls(
            AlgorithmOperation.DSV_FORMAT,
            {"rows": list(rows), "columns": list(columns), "delimiter": delimiter},
        )

    @classmethod
    def selection_join(
        cls, old_keys: Sequence[str], new_keys: Sequence[str]
    ) -> "AlgorithmRequest":
        return cls(
            AlgorithmOperation.SELECTION_JOIN,
            {"old_keys": list(old_keys), "new_keys": list(new_keys)},
        )

    @classmethod
    def brush_gesture(
        cls, points: Sequence[tuple[float, float]], *, min_size: float = 0.0
    ) -> "AlgorithmRequest":
        if len(points) < 2:
            raise ValueError("brush gesture requires at least two points")
        return cls(
            AlgorithmOperation.BRUSH_GESTURE,
            {"points": [list(point) for point in points], "min_size": min_size},
        )

    @classmethod
    def drag_gesture(
        cls,
        points: Sequence[tuple[float, float]],
        *,
        pointer_id: int = 1,
        click_distance: float = 0.0,
    ) -> "AlgorithmRequest":
        if len(points) < 2:
            raise ValueError("drag gesture requires at least two points")
        return cls(
            AlgorithmOperation.DRAG_GESTURE,
            {
                "points": [list(point) for point in points],
                "pointer_id": pointer_id,
                "click_distance": click_distance,
            },
        )

    @classmethod
    def transition_sample(
        cls,
        start: float,
        end: float,
        duration_ms: float,
        delta_ms: Sequence[float],
        *,
        delay_ms: float = 0.0,
        easing: EaseKind = EaseKind.LINEAR,
    ) -> "AlgorithmRequest":
        return cls(
            AlgorithmOperation.TRANSITION_SAMPLE,
            {
                "start": start,
                "end": end,
                "duration_ms": duration_ms,
                "delay_ms": delay_ms,
                "delta_ms": list(delta_ms),
                "kind": easing.value,
            },
        )

    @classmethod
    def polygon(
        cls,
        points: Sequence[tuple[float, float]],
        *,
        contains: tuple[float, float] | None = None,
    ) -> "AlgorithmRequest":
        arguments: dict[str, Any] = {"points": [list(point) for point in points]}
        if contains is not None:
            arguments["contains"] = [list(contains)]
        return cls(AlgorithmOperation.POLYGON, arguments)

    @classmethod
    def delaunay(
        cls,
        points: Sequence[tuple[float, float]],
        *,
        find: tuple[float, float] | None = None,
    ) -> "AlgorithmRequest":
        arguments: dict[str, Any] = {"points": [list(point) for point in points]}
        if find is not None:
            arguments["find"] = [list(find)]
        return cls(AlgorithmOperation.DELAUNAY, arguments)

    @classmethod
    def hexbin(
        cls, points: Sequence[tuple[float, float]], *, radius: float
    ) -> "AlgorithmRequest":
        return cls(
            AlgorithmOperation.HEXBIN,
            {"points": [list(point) for point in points], "radius": radius},
        )

    @classmethod
    def tiles(
        cls,
        width: float,
        height: float,
        scale: float,
        translate: tuple[float, float],
    ) -> "AlgorithmRequest":
        return cls(
            AlgorithmOperation.TILES,
            {
                "width": width,
                "height": height,
                "scale": scale,
                "translate": list(translate),
            },
        )

    @classmethod
    def chord(
        cls, matrix: Sequence[Sequence[float]], *, pad_angle: float = 0.0
    ) -> "AlgorithmRequest":
        return cls(
            AlgorithmOperation.CHORD,
            {"matrix": [list(row) for row in matrix], "pad_angle": pad_angle},
        )

    @classmethod
    def sankey(
        cls,
        nodes: Sequence[str],
        links: Sequence[SankeyLinkSpec],
        *,
        width: float = 960.0,
        height: float = 500.0,
        node_width: float = 24.0,
        node_padding: float = 8.0,
        iterations: int = 6,
    ) -> "AlgorithmRequest":
        return cls(
            AlgorithmOperation.SANKEY,
            {
                "nodes": list(nodes),
                "links": [link.to_spec() for link in links],
                "width": width,
                "height": height,
                "node_width": node_width,
                "node_padding": node_padding,
                "iterations": iterations,
            },
        )

    @classmethod
    def force(
        cls,
        points: Sequence[tuple[float, float]],
        *,
        center: tuple[float, float] = (0.0, 0.0),
        strength: float = -30.0,
        ticks: int = 1,
    ) -> "AlgorithmRequest":
        if ticks < 0:
            raise ValueError("force tick count must be non-negative")
        return cls(
            AlgorithmOperation.FORCE,
            {
                "points": [list(point) for point in points],
                "center": list(center),
                "strength": strength,
                "ticks": ticks,
            },
        )

    @classmethod
    def hierarchy_treemap(
        cls,
        root: HierarchySpec,
        *,
        size: tuple[float, float],
        padding: float = 0.0,
    ) -> "AlgorithmRequest":
        return cls(
            AlgorithmOperation.HIERARCHY_TREEMAP,
            {"root": root.to_spec(), "size": list(size), "padding": padding},
        )

    @classmethod
    def geo(
        cls,
        coordinates: Sequence[tuple[float, float]],
        *,
        contains: tuple[float, float] | None = None,
    ) -> "AlgorithmRequest":
        arguments: dict[str, Any] = {
            "coordinates": [list(point) for point in coordinates]
        }
        if contains is not None:
            arguments["contains"] = [list(contains)]
        return cls(AlgorithmOperation.GEO, arguments)

    @classmethod
    def quadtree(
        cls,
        points: Sequence[tuple[float, float]],
        *,
        find: tuple[float, float] | None = None,
        radius: float | None = None,
    ) -> "AlgorithmRequest":
        arguments: dict[str, Any] = {"points": [list(point) for point in points]}
        if find is not None:
            arguments["find"] = [list(find)]
        if radius is not None:
            arguments["radius"] = radius
        return cls(AlgorithmOperation.QUADTREE, arguments)

    @classmethod
    def contour(
        cls,
        values: Sequence[float],
        *,
        width: int,
        height: int,
        threshold: float,
    ) -> "AlgorithmRequest":
        if len(values) != width * height:
            raise ValueError("contour values must match width times height")
        return cls(
            AlgorithmOperation.CONTOUR,
            {
                "values": list(values),
                "width": width,
                "height": height,
                "threshold": threshold,
            },
        )

    @classmethod
    def lod_m4(
        cls,
        x: Sequence[float],
        y: Sequence[float],
        *,
        x0: float,
        x1: float,
        columns: int,
    ) -> "AlgorithmRequest":
        if len(x) != len(y):
            raise ValueError("LOD x and y must have equal lengths")
        return cls(
            AlgorithmOperation.LOD_M4,
            {"x": list(x), "y": list(y), "x0": x0, "x1": x1, "columns": columns},
        )

    def send(self, context: "SessionContext", request_id: str) -> None:
        context.command(
            request_id,
            "d3.algorithms",
            operation=self.operation.value,
            **self.arguments,
        )


@dataclass(frozen=True)
class AlgorithmResult:
    operation: AlgorithmOperation
    value: Any

    @classmethod
    def from_command(cls, result: CommandResult) -> "AlgorithmResult":
        if result.status is not CommandStatus.SUCCEEDED:
            raise RuntimeError(result.error or f"D3 algorithm {result.status.value}")
        return cls(AlgorithmOperation(str(result.data["operation"])), result.data.get("value"))


@dataclass(frozen=True)
class D3ModuleBridge:
    module: str
    bridge: D3BridgeKind
    python_path: str
    evidence: str


@dataclass(frozen=True)
class D3ModuleCatalog:
    modules: tuple[D3ModuleBridge, ...]

    def by_name(self, module: str) -> D3ModuleBridge | None:
        return next((entry for entry in self.modules if entry.module == module), None)


def request_module_catalog(context: "SessionContext", request_id: str) -> None:
    context.command(request_id, "d3.modules")


def module_catalog_from_command(result: CommandResult) -> D3ModuleCatalog:
    if result.status is not CommandStatus.SUCCEEDED:
        raise RuntimeError(result.error or f"D3 module catalog {result.status.value}")
    return D3ModuleCatalog(
        tuple(
            D3ModuleBridge(
                module=str(entry["module"]),
                bridge=D3BridgeKind(str(entry["bridge"])),
                python_path=str(entry["python_path"]),
                evidence=str(entry["evidence"]),
            )
            for entry in result.data.get("modules", ())
        )
    )

def request_reports(context: "SessionContext", request_id: str) -> None:
    context.command(request_id, "d3.reports")

def reports_from_command(result: CommandResult) -> D3Reports:
    if result.status is not CommandStatus.SUCCEEDED: raise RuntimeError(result.error or "d3 reports failed")
    parity, benchmark = result.data["parity"], result.data["benchmark"]
    return D3Reports(
        tuple(D3ParityEntry(str(value["id"]), str(value["d3_area"]), str(value["gpui_d3rs_modules"]), str(value["status"]), str(value["evidence"]), str(value["release_requirement"])) for value in parity["entries"]),
        str(parity["markdown"]),
        tuple(D3BenchmarkCase(str(value["id"]), str(value["module"]), str(value["bench_target"]), str(value["benchmark_group"]), str(value["benchmark_id"]), str(value["dataset_scale"]), str(value["evidence"])) for value in benchmark["cases"]),
        str(benchmark["markdown"]),
    )


# Pure d3-array computations run in-process through the abi3 extension. The
# request classes remain available when an application intentionally delegates
# work to the GPUI host.
HistogramBin = _native.HistogramBin
HistogramThreshold = _native.HistogramThreshold
Hierarchy = _native.Hierarchy
HierarchyErrorKind = _native.HierarchyErrorKind
HierarchyError = _native.HierarchyError
HierarchyNode = _native.HierarchyNode
HierarchyNodeSnapshot = _native.HierarchyNodeSnapshot
HierarchyRect = _native.HierarchyRect
HierarchyCircle = _native.HierarchyCircle
HierarchyPoint = _native.HierarchyPoint
TreemapLayout = _native.TreemapLayout
PartitionLayout = _native.PartitionLayout
PackLayout = _native.PackLayout
TreeLayout = _native.TreeLayout
ClusterLayout = _native.ClusterLayout
Contour = _native.Contour
ContourBand = _native.ContourBand
ContourGenerator = _native.ContourGenerator
ContourRing = _native.ContourRing
ContourRingError = _native.ContourRingError
ContourSegment = _native.ContourSegment
DensityEstimator = _native.DensityEstimator
DensityError = _native.DensityError
DensityGrid = _native.DensityGrid
LodDensityGrid = _native.LodDensityGrid
DensityKernel = _native.DensityKernel
contour = _native.contour
contours = _native.contours
contour_threshold_freedman_diaconis = _native.contour_threshold_freedman_diaconis
contour_threshold_scott = _native.contour_threshold_scott
contour_threshold_sturges = _native.contour_threshold_sturges
density_2d = _native.density_2d
try_density_2d = _native.try_density_2d
epanechnikov_kernel = _native.epanechnikov_kernel
gaussian_kernel = _native.gaussian_kernel
Delaunay = _native.Delaunay
Voronoi = _native.Voronoi
DelaunayError = _native.DelaunayError
DelaunayErrorKind = _native.DelaunayErrorKind
polygon_area = _native.polygon_area
polygon_area_signed = _native.polygon_area_signed
polygon_centroid = _native.polygon_centroid
polygon_contains = _native.polygon_contains
polygon_hull = _native.polygon_hull
polygon_length = _native.polygon_length
SimulationNode = _native.SimulationNode
Simulation = _native.Simulation
ForceCenter = _native.ForceCenter
ForceX = _native.ForceX
ForceY = _native.ForceY
ForceRadial = _native.ForceRadial
ForceCollide = _native.ForceCollide
ForceManyBody = _native.ForceManyBody
ForceLink = _native.ForceLink
ForceError = _native.ForceError
Point = _native.Point
PathCommandKind = _native.PathCommandKind
PathCommand = _native.PathCommand
Path = _native.Path
PathBuilder = _native.PathBuilder
ShapePath = _native.ShapePath
ShapeGenerationError = _native.ShapeGenerationError
ArcGenerationError = _native.ArcGenerationError
ArcGenerationErrorKind = _native.ArcGenerationErrorKind
ArcDatum = _native.ArcDatum
Arc = _native.Arc
arc_points = _native.arc_points
try_arc_points = _native.try_arc_points
SymbolType = _native.SymbolType
SymbolGenerationError = _native.SymbolGenerationError
SymbolGenerationErrorKind = _native.SymbolGenerationErrorKind
Symbol = _native.Symbol
symbol_radius = _native.symbol_radius
try_symbol_radius = _native.try_symbol_radius
LinkDirection = _native.LinkDirection
LinkGenerationError = _native.LinkGenerationError
LinkGenerationErrorKind = _native.LinkGenerationErrorKind
Link = _native.Link
RadialLink = _native.RadialLink
link_horizontal = _native.link_horizontal
try_link_horizontal = _native.try_link_horizontal
link_vertical = _native.link_vertical
try_link_vertical = _native.try_link_vertical
link_step = _native.link_step
try_link_step = _native.try_link_step
link_radial = _native.link_radial
try_link_radial = _native.try_link_radial
PieSlice = _native.PieSlice
PieLayoutError = _native.PieLayoutError
PieLayoutErrorKind = _native.PieLayoutErrorKind
Pie = _native.Pie
pie = _native.pie
try_pie = _native.try_pie
donut = _native.donut
try_donut = _native.try_donut
half_pie = _native.half_pie
try_half_pie = _native.try_half_pie
StackLayoutError = _native.StackLayoutError
StackLayoutErrorKind = _native.StackLayoutErrorKind
StackOrder = _native.StackOrder
StackOffset = _native.StackOffset
StackSeries = _native.StackSeries
Stack = _native.Stack
stack = _native.stack
try_stack = _native.try_stack
stack_expand = _native.stack_expand
try_stack_expand = _native.try_stack_expand
streamgraph = _native.streamgraph
try_streamgraph = _native.try_streamgraph
CurveKind = _native.CurveKind
Curve = _native.Curve
RadialGenerationErrorKind = _native.RadialGenerationErrorKind
RadialGenerationError = _native.RadialGenerationError
RadialPointField = _native.RadialPointField
RadialPoint = _native.RadialPoint
RadialLineConfig = _native.RadialLineConfig
RadialAreaConfig = _native.RadialAreaConfig
radial_line = _native.radial_line
try_radial_line = _native.try_radial_line
radial_area = _native.radial_area
try_radial_area = _native.try_radial_area
polar_grid_circles = _native.polar_grid_circles
try_polar_grid_circles = _native.try_polar_grid_circles
polar_grid_rays = _native.polar_grid_rays
try_polar_grid_rays = _native.try_polar_grid_rays
AreaGenerationError = _native.AreaGenerationError
Area = _native.Area
SimpleArea = _native.SimpleArea
area_points = _native.area_points
try_area_points = _native.try_area_points
ChordLayoutError = _native.ChordLayoutError
ChordLayoutErrorKind = _native.ChordLayoutErrorKind
ChordSort = _native.ChordSort
ChordSubgroup = _native.ChordSubgroup
ChordGroup = _native.ChordGroup
Chord = _native.Chord
ChordResult = _native.ChordResult
ChordLayout = _native.ChordLayout
RibbonGenerator = _native.RibbonGenerator
LcgRng = _native.LcgRng
RandomUniform = _native.RandomUniform
RandomNormal = _native.RandomNormal
RandomLogNormal = _native.RandomLogNormal
RandomExponential = _native.RandomExponential
RandomBernoulli = _native.RandomBernoulli
RandomPoisson = _native.RandomPoisson
RandomIrwinHall = _native.RandomIrwinHall
RandomBates = _native.RandomBates
HALF_PI = _native.HALF_PI
TAU = _native.TAU
EPSILON = _native.EPSILON
radians = _native.radians
degrees = _native.degrees
geo_distance = _native.geo_distance
geo_length = _native.geo_length
geo_interpolate = _native.geo_interpolate
geo_area = _native.geo_area
geo_bounds = _native.geo_bounds
geo_centroid = _native.geo_centroid
geo_contains = _native.geo_contains
GraticuleConfig = _native.GraticuleConfig
Graticule = _native.Graticule
graticule10 = _native.graticule10
Rotation = _native.Rotation
Versor = _native.Versor
Projection = _native.Projection
Mercator = _native.Mercator
Equirectangular = _native.Equirectangular
Orthographic = _native.Orthographic
Stereographic = _native.Stereographic
TransverseMercator = _native.TransverseMercator
ConicEqualArea = _native.ConicEqualArea
Albers = _native.Albers
GeoJsonKind = _native.GeoJsonKind
GeoJsonGeometry = _native.GeoJsonGeometry
GeoStreamEventKind = _native.GeoStreamEventKind
GeoStreamEvent = _native.GeoStreamEvent
GeoStream = _native.GeoStream
geo_stream_events = _native.geo_stream_events
stream_geojson = _native.stream_geojson
TopoJsonError = _native.TopoJsonError
TopoJsonInvalidError = _native.TopoJsonInvalidError
TopoJsonBudgetError = _native.TopoJsonBudgetError
TopoJsonEmptyLandError = _native.TopoJsonEmptyLandError
TopoJsonBudget = _native.TopoJsonBudget
parse_land = _native.parse_land
parse_land_with_budget = _native.parse_land_with_budget
AutoTypeKind = _native.AutoTypeKind
AutoTyped = _native.AutoTyped
auto_type = _native.auto_type
auto_type_row = _native.auto_type_row
auto_type_rows = _native.auto_type_rows
DsvParseErrorKind = _native.DsvParseErrorKind
DsvBudgetResource = _native.DsvBudgetResource
DsvParseError = _native.DsvParseError
DsvBudgetError = _native.DsvBudgetError
DsvCancelledError = _native.DsvCancelledError
DsvBudget = _native.DsvBudget
ColumnPolicy = _native.ColumnPolicy
CsvOptions = _native.CsvOptions
DsvCancellationToken = _native.DsvCancellationToken
DsvParser = _native.DsvParser
parse_dsv = _native.parse_dsv
parse_dsv_with_budget = _native.parse_dsv_with_budget
parse_dsv_lossy = _native.parse_dsv_lossy
try_parse_dsv = _native.try_parse_dsv
parse_csv = _native.parse_csv
parse_csv_with_budget = _native.parse_csv_with_budget
parse_csv_with_budget_and_cancel = _native.parse_csv_with_budget_and_cancel
parse_csv_lossy = _native.parse_csv_lossy
try_parse_csv = _native.try_parse_csv
parse_csv_with_options = _native.parse_csv_with_options
parse_csv_lossy_with_options = _native.parse_csv_lossy_with_options
try_parse_csv_with_options = _native.try_parse_csv_with_options
parse_tsv = _native.parse_tsv
parse_tsv_with_budget = _native.parse_tsv_with_budget
parse_tsv_with_budget_and_cancel = _native.parse_tsv_with_budget_and_cancel
parse_tsv_lossy = _native.parse_tsv_lossy
try_parse_tsv = _native.try_parse_tsv
parse_tsv_with_options = _native.parse_tsv_with_options
parse_tsv_lossy_with_options = _native.parse_tsv_lossy_with_options
try_parse_tsv_with_options = _native.try_parse_tsv_with_options
format_csv = _native.format_csv
format_tsv = _native.format_tsv
MAX_TILE_ZOOM = _native.MAX_TILE_ZOOM
MAX_VISIBLE_TILES = _native.MAX_VISIBLE_TILES
HexbinBin = _native.HexbinBin
HexbinErrorKind = _native.HexbinErrorKind
HexbinError = _native.HexbinError
Hexbin = _native.Hexbin
SankeyNode = _native.SankeyNode
SankeyLink = _native.SankeyLink
SankeyLinkInput = _native.SankeyLinkInput
SankeyResult = _native.SankeyResult
SankeyNodeAlign = _native.SankeyNodeAlign
SankeyLinkSort = _native.SankeyLinkSort
SankeyLinkSortContext = _native.SankeyLinkSortContext
SankeyLayoutErrorKind = _native.SankeyLayoutErrorKind
SankeyLayoutError = _native.SankeyLayoutError
SankeyLayout = _native.SankeyLayout
BrushSelection = _native.BrushSelection
DomainSelection = _native.DomainSelection
BrushState = _native.BrushState
BrushConfig = _native.BrushConfig
ZoomState = _native.ZoomState
ZoomConfig = _native.ZoomConfig
DragErrorKind = _native.DragErrorKind
DragError = _native.DragError
DragPoint = _native.DragPoint
DragDelta = _native.DragDelta
DragExtent = _native.DragExtent
DragConfig = _native.DragConfig
DragPhase = _native.DragPhase
DragUpdate = _native.DragUpdate
DragState = _native.DragState
TimerState = _native.TimerState
TimerDispatcher = _native.TimerDispatcher
set_ui_dispatcher = _native.set_ui_dispatcher
clear_ui_dispatcher = _native.clear_ui_dispatcher
TimerCallbackError = _native.TimerCallbackError
Timer = _native.Timer
Interval = _native.Interval
Timeout = _native.Timeout
timer = _native.timer
interval = _native.interval
timeout = _native.timeout
now = _native.now
set_now = _native.set_now
timer_flush = _native.timer_flush
TransitionEase = _native.TransitionEase
TransitionState = _native.TransitionState
TransitionConfig = _native.TransitionConfig
Transition = _native.Transition
TransitionHandle = _native.TransitionHandle
TransitionManager = _native.TransitionManager
Event = _native.Event
ListenerId = _native.ListenerId
Dispatcher = _native.Dispatcher
dispatcher = _native.dispatcher
AxisScaleKind = _native.AxisScaleKind
AxisScale = _native.AxisScale
AxisOrientation = _native.AxisOrientation
AxisPoint = _native.AxisPoint
AxisLine = _native.AxisLine
AxisTick = _native.AxisTick
AxisTitle = _native.AxisTitle
AxisLayout = _native.AxisLayout
AxisLayoutErrorKind = _native.AxisLayoutErrorKind
AxisLayoutError = _native.AxisLayoutError
AxisConfig = _native.AxisConfig
axis_layout = _native.axis_layout
AxisRgba = _native.AxisRgba
AxisTheme = _native.AxisTheme
DefaultAxisTheme = _native.DefaultAxisTheme
render_axis = _native.render_axis
GridPoint = _native.GridPoint
GridLine = _native.GridLine
GridDot = _native.GridDot
GridLayout = _native.GridLayout
GridLayoutErrorKind = _native.GridLayoutErrorKind
GridLayoutError = _native.GridLayoutError
GridConfig = _native.GridConfig
grid_layout = _native.grid_layout
render_grid = _native.render_grid
LegendPosition = _native.LegendPosition
LegendOrientation = _native.LegendOrientation
LegendSymbol = _native.LegendSymbol
LegendItem = _native.LegendItem
LegendPoint = _native.LegendPoint
LegendRect = _native.LegendRect
LegendTitleLayout = _native.LegendTitleLayout
LegendItemLayout = _native.LegendItemLayout
LegendLayout = _native.LegendLayout
LegendLayoutErrorKind = _native.LegendLayoutErrorKind
LegendLayoutError = _native.LegendLayoutError
LegendConfig = _native.LegendConfig
legend_layout = _native.legend_layout
legend_from_scale = _native.legend_from_scale
render_legend = _native.render_legend
Tile = _native.Tile
TileErrorKind = _native.TileErrorKind
TileError = _native.TileError
TileSet = _native.TileSet
TileLayout = _native.TileLayout
tiles_for_viewport = _native.tiles_for_viewport
Extent = _native.Extent
Aggregate = _native.Aggregate
QuadPoint = _native.QuadPoint
QuadNodeKind = _native.QuadNodeKind
QuadNode = _native.QuadNode
QuadTreeError = _native.QuadTreeError
QuadTree = _native.QuadTree
GeoPath = _native.GeoPath
FormatAlign = _native.FormatAlign
FormatSign = _native.FormatSign
FormatSpecifier = _native.FormatSpecifier
FormatType = _native.FormatType
Locale = _native.Locale
DEFAULT_LOCALE = _native.DEFAULT_LOCALE
format = _native.format
format_locale = _native.format_locale
format_locale_value = _native.format_locale_value
format_prefix = _native.format_prefix
format_value = _native.format_value
parse_format_specifier = _native.parse_format_specifier
prefix_exponent = _native.prefix_exponent
TimeInterval = _native.TimeInterval
TimeFormat = _native.TimeFormat
TimeFormatParts = _native.TimeFormatParts
TimeScale = _native.TimeScale
SECOND = _native.SECOND
MINUTE = _native.MINUTE
HOUR = _native.HOUR
DAY = _native.DAY
WEEK = _native.WEEK
time_second = _native.time_second
time_minute = _native.time_minute
time_hour = _native.time_hour
time_day = _native.time_day
time_week = _native.time_week
time_monday = _native.time_monday
time_month = _native.time_month
time_year = _native.time_year
time_format = _native.time_format
time_format_value = _native.time_format_value
timestamp_from_millis = _native.timestamp_from_millis
millis_from_timestamp = _native.millis_from_timestamp
def clamp01(t: float) -> float:
    return _native.clamp01(t)


def interpolate_clamped(a: float, b: float, t: float) -> float:
    return _native.interpolate_clamped(a, b, t)


def interpolate_basis(values: Sequence[float], t: float) -> float:
    return _native.interpolate_basis(values, t)


def interpolate_basis_closed(values: Sequence[float], t: float) -> float:
    return _native.interpolate_basis_closed(values, t)


def interpolate_exp(a: float, b: float, t: float) -> float:
    return _native.interpolate_exp(a, b, t)


def interpolate_discrete(values: Sequence[float], t: float) -> float:
    return _native.interpolate_discrete(values, t)


def interpolate_quantize(a: float, b: float, levels: int, t: float) -> float:
    return _native.interpolate_quantize(a, b, levels, t)
def piecewise(values: Sequence[float], t: float) -> float:
    return _native.piecewise(values, t)


def piecewise_with(
    values: Sequence[_T], interpolator: Callable[[_T, _T, float], _T], t: float
) -> _T:
    """Evaluate a piecewise interpolation with a caller-owned pure callback."""

    if not values:
        raise ValueError("piecewise_with requires at least one value")
    _require_callable(interpolator, "interpolator")
    if len(values) == 1:
        return values[0]
    clamped = clamp01(float(t))
    scaled = clamped * (len(values) - 1)
    index = _builtins.min(int(scaled), len(values) - 2)
    return interpolator(values[index], values[index + 1], scaled - index)


def piecewise_domain(
    positions: Sequence[float], values: Sequence[float], t: float
) -> float:
    return _native.piecewise_domain(positions, values, t)


def quantize(values: Sequence[float], t: float) -> float:
    return _native.quantize(values, t)


def interpolate_array(a: Sequence[_T], b: Sequence[_T], t: float) -> list[_T]:
    """Interpolate matching generic values up to the shorter input length."""

    parameter = float(t)
    if not isfinite(parameter):
        raise ValueError("t must be finite")
    result: list[_T] = []
    for index, (left, right) in enumerate(zip(a, b)):
        if type(left) is int and type(right) is int:
            result.append(  # type: ignore[arg-type]
                _builtins.round(left + (right - left) * parameter)
            )
        elif isinstance(left, (int, float)) and isinstance(right, (int, float)):
            result.append(left + (right - left) * parameter)  # type: ignore[arg-type]
        elif isinstance(left, Interpolate):
            result.append(left.interpolate(right, parameter))
        else:
            raise TypeError(
                f"a[{index}] must be numeric or implement Interpolate"
            )
    return result


def interpolate_matrix(
    a: Sequence[Sequence[float]], b: Sequence[Sequence[float]], t: float
) -> list[list[float]]:
    return _native.interpolate_matrix(a, b, t)


def interpolate_zoom_vector(
    a: tuple[float, float, float], b: tuple[float, float, float], t: float
) -> tuple[float, float, float]:
    return _native.interpolate_zoom_vector(a, b, t)


def interpolate_string(a: str, b: str, t: float) -> str:
    return _native.interpolate_string(a, b, t)


def interpolate_transform_css(a: str, b: str, t: float) -> str:
    return _native.interpolate_transform_css(a, b, t)


def interpolate_date(a: float, b: float, t: float) -> float:
    return _native.interpolate_date(a, b, t)
Transform2D = _native.Transform2D


def interpolate_transform(a: Transform2D, b: Transform2D, t: float) -> Transform2D:
    return _native.interpolate_transform(a, b, t)


def interpolate_transform_svg(
    a: Sequence[float], b: Sequence[float], t: float
) -> tuple[float, float, float, float, float, float]:
    return _native.interpolate_transform_svg(a, b, t)


ZoomParams = _native.ZoomParams
ZoomView = _native.ZoomView


def interpolate_zoom_view(
    a: ZoomView, b: ZoomView, t: float, *, params: ZoomParams | None = None
) -> ZoomView:
    return _native.interpolate_zoom_view(a, b, t, params=params)


def interpolate_zoom_with_params(
    a: ZoomView, b: ZoomView, params: ZoomParams, t: float
) -> ZoomView:
    return _native.interpolate_zoom_view(a, b, t, params=params)


def zoom_duration(
    a: ZoomView, b: ZoomView, *, params: ZoomParams | None = None
) -> float:
    return _native.zoom_duration(a, b, params=params)


def zoom_duration_with_rho(a: ZoomView, b: ZoomView, rho: float) -> float:
    return _native.zoom_duration(a, b, params=ZoomParams(rho))


LodErrorKind = _native.LodErrorKind
LodError = _native.LodError
LodBounds = _native.LodBounds
DensityGrid = _native.DensityGrid
DensityPyramid = _native.DensityPyramid


def m4_indices(
    x: Sequence[float],
    y: Sequence[float],
    x0: float,
    x1: float,
    columns: int,
) -> list[int]:
    return _native.m4_indices(x, y, x0, x1, columns)


def m4_point_indices(
    points: Sequence[tuple[float, float]], columns: int
) -> list[int]:
    return _native.m4_point_indices(points, columns)


# Complete d3-ease surface executes in-process through the abi3 extension.
def ease_linear(t: float) -> float:
    return _native.ease_linear(t)


def ease_quad_in(t: float) -> float:
    return _native.ease_quad_in(t)


def ease_quad_out(t: float) -> float:
    return _native.ease_quad_out(t)


def ease_quad_in_out(t: float) -> float:
    return _native.ease_quad_in_out(t)


def ease_cubic_in(t: float) -> float:
    return _native.ease_cubic_in(t)


def ease_cubic_out(t: float) -> float:
    return _native.ease_cubic_out(t)


def ease_cubic_in_out(t: float) -> float:
    return _native.ease_cubic_in_out(t)


def ease_poly_in(exponent: float, t: float) -> float:
    return _native.ease_poly_in(exponent, t)


def ease_poly_out(exponent: float, t: float) -> float:
    return _native.ease_poly_out(exponent, t)


def ease_poly_in_out(exponent: float, t: float) -> float:
    return _native.ease_poly_in_out(exponent, t)


def ease_sin_in(t: float) -> float:
    return _native.ease_sin_in(t)


def ease_sin_out(t: float) -> float:
    return _native.ease_sin_out(t)


def ease_sin_in_out(t: float) -> float:
    return _native.ease_sin_in_out(t)


def ease_exp_in(t: float) -> float:
    return _native.ease_exp_in(t)


def ease_exp_out(t: float) -> float:
    return _native.ease_exp_out(t)


def ease_exp_in_out(t: float) -> float:
    return _native.ease_exp_in_out(t)


def ease_circle_in(t: float) -> float:
    return _native.ease_circle_in(t)


def ease_circle_out(t: float) -> float:
    return _native.ease_circle_out(t)


def ease_circle_in_out(t: float) -> float:
    return _native.ease_circle_in_out(t)


def ease_elastic_in_with(amplitude: float, period: float, t: float) -> float:
    return _native.ease_elastic_in_with(amplitude, period, t)


def ease_elastic_out_with(amplitude: float, period: float, t: float) -> float:
    return _native.ease_elastic_out_with(amplitude, period, t)


def ease_elastic_in(t: float) -> float:
    return _native.ease_elastic_in(t)


def ease_elastic_out(t: float) -> float:
    return _native.ease_elastic_out(t)


def ease_elastic_in_out(t: float) -> float:
    return _native.ease_elastic_in_out(t)


def ease_back_in_with(overshoot: float, t: float) -> float:
    return _native.ease_back_in_with(overshoot, t)


def ease_back_out_with(overshoot: float, t: float) -> float:
    return _native.ease_back_out_with(overshoot, t)


def ease_back_in_out_with(overshoot: float, t: float) -> float:
    return _native.ease_back_in_out_with(overshoot, t)


def ease_back_in(t: float) -> float:
    return _native.ease_back_in(t)


def ease_back_out(t: float) -> float:
    return _native.ease_back_out(t)


def ease_back_in_out(t: float) -> float:
    return _native.ease_back_in_out(t)


def ease_bounce_out(t: float) -> float:
    return _native.ease_bounce_out(t)


def ease_bounce_in(t: float) -> float:
    return _native.ease_bounce_in(t)


def ease_bounce_in_out(t: float) -> float:
    return _native.ease_bounce_in_out(t)

_EASE_FUNCTIONS = {
    EaseKind.LINEAR: ease_linear,
    EaseKind.QUAD_IN: ease_quad_in,
    EaseKind.QUAD_OUT: ease_quad_out,
    EaseKind.QUAD_IN_OUT: ease_quad_in_out,
    EaseKind.CUBIC_IN: ease_cubic_in,
    EaseKind.CUBIC_OUT: ease_cubic_out,
    EaseKind.CUBIC_IN_OUT: ease_cubic_in_out,
    EaseKind.SIN_IN: ease_sin_in,
    EaseKind.SIN_OUT: ease_sin_out,
    EaseKind.SIN_IN_OUT: ease_sin_in_out,
    EaseKind.EXP_IN: ease_exp_in,
    EaseKind.EXP_OUT: ease_exp_out,
    EaseKind.EXP_IN_OUT: ease_exp_in_out,
    EaseKind.CIRCLE_IN: ease_circle_in,
    EaseKind.CIRCLE_OUT: ease_circle_out,
    EaseKind.CIRCLE_IN_OUT: ease_circle_in_out,
    EaseKind.ELASTIC_IN: ease_elastic_in,
    EaseKind.ELASTIC_OUT: ease_elastic_out,
    EaseKind.ELASTIC_IN_OUT: ease_elastic_in_out,
    EaseKind.BACK_IN: ease_back_in,
    EaseKind.BACK_OUT: ease_back_out,
    EaseKind.BACK_IN_OUT: ease_back_in_out,
    EaseKind.BOUNCE_IN: ease_bounce_in,
    EaseKind.BOUNCE_OUT: ease_bounce_out,
    EaseKind.BOUNCE_IN_OUT: ease_bounce_in_out,
}


def ease(kind: EaseKind, t: float) -> float:
    """Apply a typed default d3 easing strategy in the Python process."""
    return _EASE_FUNCTIONS[EaseKind(kind)](t)


def interpolate_ease(a: float, b: float, kind: EaseKind, t: float) -> float:
    """Interpolate numeric endpoints with a supported typed easing strategy."""
    return _native.interpolate_ease(a, b, EaseKind(kind), t)


ThresholdStrategy = _native.HistogramThreshold


@dataclass(frozen=True)
class Bin(Generic[_T]):
    x0: float
    x1: float
    values: tuple[_T, ...]

    def __len__(self) -> int:
        return len(self.values)

    def is_empty(self) -> bool:
        return not self.values


@dataclass(frozen=True, init=False)
class BinGenerator(Generic[_T]):
    _accessor: Callable[[_T], float] = float
    _domain: tuple[float, float] | None = None
    _strategy: _native.HistogramThreshold = _native.HistogramThreshold.STURGES
    _count: int | None = None
    _threshold_values: tuple[float, ...] | None = None

    def __init__(self) -> None:
        object.__setattr__(self, "_accessor", float)
        object.__setattr__(self, "_domain", None)
        object.__setattr__(self, "_strategy", _native.HistogramThreshold.STURGES)
        object.__setattr__(self, "_count", None)
        object.__setattr__(self, "_threshold_values", None)

    def _updated(self, **changes: Any) -> "BinGenerator[_T]":
        updated = object.__new__(type(self))
        for name in ("_accessor", "_domain", "_strategy", "_count", "_threshold_values"):
            object.__setattr__(updated, name, changes.get(name, getattr(self, name)))
        return updated

    def value(self, accessor: Callable[[_T], float]) -> "BinGenerator[_T]":
        _require_callable(accessor, "accessor")
        return self._updated(_accessor=accessor)

    def domain(self, minimum: float, maximum: float) -> "BinGenerator[_T]":
        minimum = float(minimum)
        maximum = float(maximum)
        if not isfinite(minimum) or not isfinite(maximum) or minimum > maximum:
            raise ValueError("bin domain must contain finite increasing endpoints")
        return self._updated(_domain=(minimum, maximum))

    def thresholds_count(self, count: int) -> "BinGenerator[_T]":
        if not isinstance(count, int) or isinstance(count, bool) or count <= 0:
            raise ValueError("bin threshold count must be a positive integer")
        return self._updated(
            _strategy=_native.HistogramThreshold.COUNT,
            _count=count,
            _threshold_values=None,
        )

    def thresholds(self, values: Sequence[float]) -> "BinGenerator[_T]":
        thresholds = tuple(float(value) for value in values)
        if any(not isfinite(value) for value in thresholds):
            raise ValueError("bin thresholds must be finite")
        if any(left >= right for left, right in zip(thresholds, thresholds[1:])):
            raise ValueError("bin thresholds must be strictly increasing")
        return self._updated(
            _strategy=_native.HistogramThreshold.VALUES,
            _count=None,
            _threshold_values=thresholds,
        )

    def thresholds_sturges(self) -> "BinGenerator[_T]":
        return self._updated(
            _strategy=_native.HistogramThreshold.STURGES,
            _count=None,
            _threshold_values=None,
        )

    def thresholds_freedman_diaconis(self) -> "BinGenerator[_T]":
        return self._updated(
            _strategy=_native.HistogramThreshold.FREEDMAN_DIACONIS,
            _count=None,
            _threshold_values=None,
        )

    def thresholds_scott(self) -> "BinGenerator[_T]":
        return self._updated(
            _strategy=_native.HistogramThreshold.SCOTT,
            _count=None,
            _threshold_values=None,
        )

    def generate(self, data: Sequence[_T]) -> list[Bin[_T]]:
        materialized = tuple(data)
        if not materialized:
            return []
        values = [float(self._accessor(item)) for item in materialized]
        native_bins = _native.histogram(
            values,
            strategy=self._strategy,
            count=self._count,
            thresholds=self._threshold_values,
            domain=self._domain,
        )
        grouped: list[list[_T]] = [[] for _ in native_bins]
        for value, item in zip(values, materialized):
            for position, native_bin in enumerate(native_bins):
                if value < native_bin.x1 or position == len(native_bins) - 1:
                    grouped[position].append(item)
                    break
        return [
            Bin(native_bin.x0, native_bin.x1, tuple(items))
            for native_bin, items in zip(native_bins, grouped)
        ]


class Bisector(Generic[_T]):
    def __init__(self, accessor: Callable[[_T], float]) -> None:
        _require_callable(accessor, "accessor")
        self._accessor = accessor

    def left(self, data: Sequence[_T], value: float) -> int:
        low = 0
        high = len(data)
        while low < high:
            middle = low + (high - low) // 2
            if self._accessor(data[middle]) < value:
                low = middle + 1
            else:
                high = middle
        return low

    def right(self, data: Sequence[_T], value: float) -> int:
        low = 0
        high = len(data)
        while low < high:
            middle = low + (high - low) // 2
            if self._accessor(data[middle]) <= value:
                low = middle + 1
            else:
                high = middle
        return low

    def center(self, data: Sequence[_T], value: float) -> _T | None:
        if not data:
            return None
        position = self.left(data, value)
        if position == 0:
            return data[0]
        if position >= len(data):
            return data[-1]
        left = data[position - 1]
        right = data[position]
        if value - self._accessor(left) <= self._accessor(right) - value:
            return left
        return right


def histogram(
    data: Sequence[float],
    *,
    strategy: _native.HistogramThreshold = _native.HistogramThreshold.STURGES,
    count: int | None = None,
    thresholds: Sequence[float] | None = None,
    domain: tuple[float, float] | None = None,
) -> list[_native.HistogramBin]:
    return _native.histogram(
        data,
        strategy=strategy,
        count=count,
        thresholds=thresholds,
        domain=domain,
    )


def threshold_sturges(size: int) -> int:
    return _native.threshold_sturges(size)


def nice_bin_edges(minimum: float, maximum: float, count: int) -> list[float]:
    return _native.nice_bin_edges(minimum, maximum, count)


def bin(data: Sequence[float], count: int) -> list[Bin[float]]:
    """Bin numeric values with the Rust convenience function's count policy."""
    return BinGenerator[float]().thresholds_count(count).generate(data)


def bisect(data: Sequence[_T], value: _T) -> int:
    """Generic rightmost insertion point, matching ``d3rs::array::bisect``."""
    low = 0
    high = len(data)
    while low < high:
        middle = low + (high - low) // 2
        if data[middle] <= value:
            low = middle + 1
        else:
            high = middle
    return low


def bisect_left_f64(data: Sequence[float], value: float) -> int:
    return _native.bisect_left(data, value)


def bisect_right_f64(data: Sequence[float], value: float) -> int:
    return _native.bisect_right(data, value)


def count(data: Sequence[_T], predicate: Callable[[_T], bool]) -> int:
    _require_callable(predicate, "predicate")
    return _builtins.sum(1 for item in data if predicate(item))


def min_by(data: Sequence[_T], compare: Callable[[_T, _T], int]) -> _T | None:
    _require_callable(compare, "compare")
    return _builtins.min(data, key=_cmp_to_key(compare), default=None)


def max_by(data: Sequence[_T], compare: Callable[[_T, _T], int]) -> _T | None:
    _require_callable(compare, "compare")
    return _builtins.max(data, key=_cmp_to_key(compare), default=None)


def extent_by(
    data: Sequence[_T], compare: Callable[[_T, _T], int]
) -> tuple[_T, _T] | None:
    _require_callable(compare, "compare")
    if not data:
        return None
    key = _cmp_to_key(compare)
    return _builtins.min(data, key=key), _builtins.max(data, key=key)


def mean_by(data: Sequence[_T], accessor: Callable[[_T], float]) -> float | None:
    _require_callable(accessor, "accessor")
    if not data:
        return None
    return _builtins.sum(float(accessor(item)) for item in data) / len(data)


def filter(data: Sequence[_T], predicate: Callable[[_T], bool]) -> list[_T]:
    _require_callable(predicate, "predicate")
    return [item for item in data if predicate(item)]


def map(data: Sequence[_T], function: Callable[[_T], _U]) -> list[_U]:
    _require_callable(function, "function")
    return [function(item) for item in data]


def reduce(
    data: Sequence[_T], initial: _U, function: Callable[[_U, _T], _U]
) -> _U:
    _require_callable(function, "function")
    result = initial
    for item in data:
        result = function(result, item)
    return result


def group(data: Sequence[_T], key: Callable[[_T], _K]) -> dict[_K, list[_T]]:
    _require_callable(key, "key")
    result: dict[_K, list[_T]] = {}
    for item in data:
        result.setdefault(key(item), []).append(item)
    return result


def rollup(
    data: Sequence[_T],
    key: Callable[[_T], _K],
    reducer: Callable[[Sequence[_T]], _U],
) -> dict[_K, _U]:
    _require_callable(key, "key")
    _require_callable(reducer, "reducer")
    return {group_key: reducer(values) for group_key, values in group(data, key).items()}


def index(data: Sequence[_T], key: Callable[[_T], _K]) -> dict[_K, _T]:
    _require_callable(key, "key")
    return {key(item): item for item in data}


def sort_by(data: Sequence[_T], key: Callable[[_T], Any]) -> list[_T]:
    _require_callable(key, "key")
    return sorted(data, key=key)


def sort_by_desc(data: Sequence[_T], key: Callable[[_T], Any]) -> list[_T]:
    _require_callable(key, "key")
    return sorted(data, key=key, reverse=True)


def nice_number(range: float, round: bool) -> float:
    return _native.nice_number(range, round)


def scale_nice_number(range: float, round: bool) -> float:
    return _native.scale_nice_number(range, round)


def generate_linear_ticks(min: float, max: float, count: int) -> list[float]:
    return _native.generate_linear_ticks(min, max, count)


def generate_log_ticks(
    min: float,
    max: float,
    base: float,
    subdivisions: bool,
) -> list[float]:
    return _native.generate_log_ticks(min, max, base, subdivisions)


def reverse(data: Sequence[float]) -> list[float]:
    return _native.reverse(data)


def shuffle_seeded(data: Sequence[float], seed: int) -> list[float]:
    return _native.shuffle_seeded(data, seed)


def shuffle(
    rng: LcgRng | Sequence[object], data: Sequence[object] | None = None
) -> list[object]:
    return _native.shuffle(rng, data)


def shuffle_in_place(rng: LcgRng, data: list[object]) -> None:
    _native.shuffle_in_place(rng, data)


def pairs(data: Sequence[float]) -> list[tuple[float, float]]:
    return _native.pairs(data)


def cross(
    left: Sequence[float], right: Sequence[float]
) -> list[tuple[float, float]]:
    return _native.cross(left, right)


def unique(data: Sequence[float]) -> list[float]:
    return _native.unique(data)


def sort(data: Sequence[float]) -> list[float]:
    return _native.sort(data)


def sort_descending(data: Sequence[float]) -> list[float]:
    return _native.sort_descending(data)


def merge_sorted(slices: Sequence[Sequence[float]]) -> list[float]:
    return _native.merge_sorted(slices)


def binary_search(data: Sequence[float], value: float) -> int | None:
    return _native.binary_search(data, value)


def difference(left: Sequence[float], right: Sequence[float]) -> list[float]:
    return _native.difference(left, right)


def intersection(left: Sequence[float], right: Sequence[float]) -> list[float]:
    return _native.intersection(left, right)


def union(left: Sequence[float], right: Sequence[float]) -> list[float]:
    return _native.union(left, right)


def symmetric_difference(
    left: Sequence[float], right: Sequence[float]
) -> list[float]:
    return _native.symmetric_difference(left, right)


def is_subset(left: Sequence[float], right: Sequence[float]) -> bool:
    return _native.is_subset(left, right)


def is_superset(left: Sequence[float], right: Sequence[float]) -> bool:
    return _native.is_superset(left, right)


def is_disjoint(left: Sequence[float], right: Sequence[float]) -> bool:
    return _native.is_disjoint(left, right)


def bisect_left(data: Sequence[float], value: float) -> int:
    return _native.bisect_left(data, value)


def bisect_right(data: Sequence[float], value: float) -> int:
    return _native.bisect_right(data, value)


def least_index(data: Sequence[float], value: float) -> int | None:
    return _native.least_index(data, value)


def quantile(data: Sequence[float], percentile: float) -> float | None:
    return _native.quantile(data, percentile)


def quantile_sorted(data: Sequence[float], percentile: float) -> float | None:
    return _native.quantile_sorted(data, percentile)


def min(data: Sequence[float]) -> float | None:
    return _native.min(data)


def max(data: Sequence[float]) -> float | None:
    return _native.max(data)


def min_index(data: Sequence[float]) -> int | None:
    return _native.min_index(data)


def max_index(data: Sequence[float]) -> int | None:
    return _native.max_index(data)


def sum(data: Sequence[float]) -> float:
    return _native.sum(data)


def mean(data: Sequence[float]) -> float | None:
    return _native.mean(data)


def median(data: Sequence[float]) -> float | None:
    return _native.median(data)


def variance(data: Sequence[float]) -> float | None:
    return _native.variance(data)


def deviation(data: Sequence[float]) -> float | None:
    return _native.deviation(data)


def extent(data: Sequence[float]) -> tuple[float, float] | None:
    return _native.extent(data)


def cumsum(data: Sequence[float]) -> list[float]:
    return _native.cumsum(data)


def ticks(start: float, stop: float, count: int = 10) -> list[float]:
    return _native.ticks(start, stop, count)


def tick_step(start: float, stop: float, count: int = 10) -> float:
    return _native.tick_step(start, stop, count)


def tick_increment(start: float, stop: float, count: int = 10) -> float:
    return _native.tick_increment(start, stop, count)


def nice(start: float, stop: float, count: int = 10) -> tuple[float, float]:
    return _native.nice(start, stop, count)


def ticks_interval(start: float, stop: float, interval: float) -> list[float]:
    return _native.ticks_interval(start, stop, interval)


def log_ticks(
    start: float,
    stop: float,
    *,
    base: float = 10.0,
    subdivisions: bool = True,
) -> list[float]:
    return _native.log_ticks(start, stop, base=base, subdivisions=subdivisions)


def time_ticks(start: float, stop: float, count: int = 10) -> list[float]:
    return _native.time_ticks(start, stop, count)


def interpolate_number(a: float, b: float, t: float) -> float:
    return _native.interpolate_number(a, b, t)


def interpolate_f32(a: float, b: float, t: float) -> float:
    """Evaluate d3rs' f32 interpolator using Python's native float value."""

    return _native.interpolate_number(a, b, t)


def lerp(a: float, b: float, t: float) -> float:
    """Evaluate the generic d3rs linear interpolation operation."""

    return _native.interpolate_number(a, b, t)


def interpolate_round(a: int, b: int, t: float) -> int:
    return _native.interpolate_round(a, b, t)


def interpolate_number_array(
    a: Sequence[float], b: Sequence[float], t: float
) -> list[float]:
    return _native.interpolate_number_array(a, b, t)


def interpolate_rgb(a: str, b: str, t: float) -> str:
    return _native.interpolate_rgb(a, b, t)


def interpolate_hsl(a: str, b: str, t: float) -> str:
    return _native.interpolate_hsl(a, b, t)


def interpolate_hsl_long(a: str, b: str, t: float) -> str:
    return _native.interpolate_hsl_long(a, b, t)


def interpolate_lab(a: str, b: str, t: float) -> str:
    return _native.interpolate_lab(a, b, t)


def interpolate_hcl(a: str, b: str, t: float) -> str:
    return _native.interpolate_hcl(a, b, t)


def interpolate_hcl_long(a: str, b: str, t: float) -> str:
    return _native.interpolate_hcl_long(a, b, t)


def interpolate_cubehelix(a: str, b: str, t: float) -> str:
    return _native.interpolate_cubehelix(a, b, t)


def interpolate_cubehelix_long(a: str, b: str, t: float) -> str:
    return _native.interpolate_cubehelix_long(a, b, t)


def color_luminance(value: str) -> float:
    return _native.color_luminance(value)


def color_lighten(value: str, amount: float) -> str:
    return _native.color_lighten(value, amount)


def color_darken(value: str, amount: float) -> str:
    return _native.color_darken(value, amount)
