"""Declarative UI helpers for the GPUI Python wrapper."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Iterable, Sequence


def _spec(value: Any) -> Any:
    if hasattr(value, "to_spec"):
        return value.to_spec()
    if isinstance(value, list | tuple):
        return [_spec(item) for item in value]
    return value


def _children(values: Iterable[Any] | None) -> list[dict[str, Any]]:
    return [_spec(value) for value in ([] if values is None else values)]


@dataclass(frozen=True)
class Node:
    kind: str
    props: dict[str, Any] = field(default_factory=dict)
    children: Sequence[Any] = field(default_factory=list)

    def to_spec(self) -> dict[str, Any]:
        spec = {"kind": self.kind, **self.props}
        if self.children:
            spec["children"] = _children(self.children)
        return spec


def vstack(children: Sequence[Any], *, gap: float | None = None, **props: Any) -> Node:
    return Node("vstack", {"gap": gap, **props}, children)


def hstack(children: Sequence[Any], *, gap: float | None = None, **props: Any) -> Node:
    return Node("hstack", {"gap": gap, **props}, children)


def wrap(children: Sequence[Any], *, gap: float | None = None, **props: Any) -> Node:
    return Node("wrap", {"gap": gap, **props}, children)


def heading(text: str, *, level: int = 1, **props: Any) -> Node:
    return Node("heading", {"text": text, "level": int(level), **props})


def text(value: str, *, tone: str = "primary", **props: Any) -> Node:
    return Node("text", {"text": value, "tone": tone, **props})


def code(value: str, **props: Any) -> Node:
    return Node("code", {"text": value, **props})


def section_header(title: str, subtitle: str = "", **props: Any) -> Node:
    return Node("section_header", {"title": title, "subtitle": subtitle, **props})


def card(children: Sequence[Any], *, title: str | None = None, **props: Any) -> Node:
    return Node("card", {"title": title, **props}, children)


def button(label: str, *, action: str | None = None, selected: bool = False, **props: Any) -> Node:
    return Node("button", {"label": label, "action": action, "selected": selected, **props})


def badge(label: str, *, tone: str = "neutral", **props: Any) -> Node:
    return Node("badge", {"label": label, "tone": tone, **props})


def metric(label: str, value: str | int | float, **props: Any) -> Node:
    return Node("metric", {"label": label, "value": str(value), **props})


def progress(value: float, *, label: str | None = None, **props: Any) -> Node:
    return Node("progress", {"value": float(value), "label": label, **props})


def spinner(label: str | None = None, **props: Any) -> Node:
    return Node("spinner", {"label": label, **props})


def tabs(items: Sequence[str], *, active: int = 0, **props: Any) -> Node:
    return Node("tabs", {"items": list(items), "active": int(active), **props})


def table(headers: Sequence[str], rows: Sequence[Sequence[Any]], **props: Any) -> Node:
    return Node(
        "table",
        {
            "headers": [str(header) for header in headers],
            "rows": [[str(cell) for cell in row] for row in rows],
            **props,
        },
    )


def divider(**props: Any) -> Node:
    return Node("divider", props)


def spacer(**props: Any) -> Node:
    return Node("spacer", props)


def scene3d(spec: Any, *, id: str | None = None, width: float | None = None, height: float | None = None) -> Node:
    scene_spec = _spec(spec)
    return Node(
        "scene3d",
        {
            "id": id or scene_spec.get("id", "scene3d"),
            "spec": scene_spec,
            "width": width,
            "height": height,
        },
    )
