"""Declarative gpui-px chart helpers for the GPUI Python wrapper."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Sequence


@dataclass(frozen=True)
class Chart:
    chart: str
    id: str
    title: str
    x: Sequence[float] | None = None
    y: Sequence[float] | None = None
    categories: Sequence[str] | None = None
    values: Sequence[float] | None = None
    z: Sequence[float] | None = None
    width_count: int | None = None
    height_count: int | None = None
    color: str | None = None
    color_scale: str = "viridis"
    x_log: bool = False
    y_log: bool = False
    width: float = 360.0
    height: float = 260.0
    point_radius: float = 4.0
    stroke_width: float = 2.0

    def to_spec(self) -> dict[str, Any]:
        return {
            "kind": "chart",
            "chart": self.chart,
            "id": self.id,
            "title": self.title,
            "x": None if self.x is None else [float(value) for value in self.x],
            "y": None if self.y is None else [float(value) for value in self.y],
            "categories": None if self.categories is None else [str(value) for value in self.categories],
            "values": None if self.values is None else [float(value) for value in self.values],
            "z": None if self.z is None else [float(value) for value in self.z],
            "width_count": self.width_count,
            "height_count": self.height_count,
            "color": self.color,
            "color_scale": self.color_scale,
            "x_log": bool(self.x_log),
            "y_log": bool(self.y_log),
            "width": float(self.width),
            "height": float(self.height),
            "point_radius": float(self.point_radius),
            "stroke_width": float(self.stroke_width),
        }


def scatter(id: str, x: Sequence[float], y: Sequence[float], *, title: str = "", **kwargs: Any) -> Chart:
    return Chart("scatter", id=id, title=title, x=x, y=y, **kwargs)


def line(id: str, x: Sequence[float], y: Sequence[float], *, title: str = "", **kwargs: Any) -> Chart:
    return Chart("line", id=id, title=title, x=x, y=y, **kwargs)


def bar(
    id: str,
    categories: Sequence[str],
    values: Sequence[float],
    *,
    title: str = "",
    **kwargs: Any,
) -> Chart:
    return Chart("bar", id=id, title=title, categories=categories, values=values, **kwargs)


def heatmap(
    id: str,
    z: Sequence[float],
    width_count: int,
    height_count: int,
    *,
    title: str = "",
    **kwargs: Any,
) -> Chart:
    return Chart(
        "heatmap",
        id=id,
        title=title,
        z=z,
        width_count=int(width_count),
        height_count=int(height_count),
        **kwargs,
    )
