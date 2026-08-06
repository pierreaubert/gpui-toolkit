"""Serializable ``gpui-component-lab`` story declarations for native previews."""
from __future__ import annotations
from dataclasses import dataclass, field
from typing import Any

StoryValue = bool | float | str

@dataclass(frozen=True)
class StoryProp:
    name: str
    label: str
    value: StoryValue
    value_type: str
    options: tuple[str, ...] = ()

    def __post_init__(self) -> None:
        if self.value_type not in {"bool", "number", "text", "choice", "color"}:
            raise ValueError("invalid component-lab property type")
        if not self.name or not self.label:
            raise ValueError("story properties require name and label")
        if self.value_type == "choice" and not self.options:
            raise ValueError("choice properties require options")

    def to_spec(self) -> dict[str, Any]:
        return {"name": self.name, "label": self.label, "value": {"type": self.value_type, "value": self.value}, "options": list(self.options)}

@dataclass(frozen=True)
class ViewportPreset:
    id: str
    label: str
    width: float
    height: float
    def __post_init__(self) -> None:
        if not self.id or not self.label or self.width <= 0 or self.height <= 0:
            raise ValueError("viewports require identifiers and positive dimensions")

@dataclass(frozen=True)
class ThemePreset:
    id: str
    label: str
    design: str
    reduced_motion: bool = False

@dataclass(frozen=True)
class MotionPreset:
    id: str
    label: str
    reduced_motion: bool = False

@dataclass(frozen=True)
class ComponentStory:
    id: str
    crate_name: str
    title: str
    description: str
    props: tuple[StoryProp, ...] = ()
    viewports: tuple[ViewportPreset, ...] = ()
    themes: tuple[ThemePreset, ...] = ()
    motions: tuple[MotionPreset, ...] = ()
    metadata: tuple[tuple[str, str], ...] = ()

    def __post_init__(self) -> None:
        if not all((self.id, self.crate_name, self.title)):
            raise ValueError("component stories require id, crate_name, and title")
        if len({prop.name for prop in self.props}) != len(self.props):
            raise ValueError("component story property names must be unique")

    def to_spec(self) -> dict[str, Any]:
        return {"id": self.id, "crate_name": self.crate_name, "title": self.title, "description": self.description, "props": [prop.to_spec() for prop in self.props], "viewports": [item.__dict__.copy() for item in self.viewports], "themes": [item.__dict__.copy() for item in self.themes], "motions": [item.__dict__.copy() for item in self.motions], "metadata": [{"key": key, "value": value} for key, value in self.metadata]}
