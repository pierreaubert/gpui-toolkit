"""Serializable keymap declarations compatible with host-side registration."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


class KeymapPreset(str, Enum):
    DEFAULT = "default"
    VIM = "vim"
    EMACS = "emacs"
    VSCODE = "vscode"

    @property
    def display_name(self) -> str:
        return {self.DEFAULT: "Default", self.VIM: "Vim", self.EMACS: "Emacs", self.VSCODE: "VSCode"}[self]


@dataclass(frozen=True)
class KeyBinding:
    command_id: str
    key: str
    description: str
    category: str = "General"
    when: str | None = None

    def __post_init__(self) -> None:
        if not self.command_id.strip() or not self.key.strip() or not self.description.strip():
            raise ValueError("key bindings require command_id, key, and description")

    def to_spec(self) -> dict[str, str | None]:
        return {
            "command_id": self.command_id,
            "key": self.key,
            "description": self.description,
            "category": self.category,
            "when": self.when,
        }


@dataclass(frozen=True)
class KeyConflict:
    key: str
    bindings: tuple[KeyBinding, ...]


class KeybindingRegistry:
    """Value registry; the native host later installs its serialized result."""

    def __init__(self) -> None:
        self._bindings: dict[KeymapPreset, list[KeyBinding]] = {preset: [] for preset in KeymapPreset}

    def register(self, binding: KeyBinding, preset: KeymapPreset = KeymapPreset.DEFAULT) -> None:
        self._bindings[preset].append(binding)

    def bindings(self, preset: KeymapPreset = KeymapPreset.DEFAULT) -> tuple[KeyBinding, ...]:
        return tuple(self._bindings[preset])

    def conflicts(self, preset: KeymapPreset = KeymapPreset.DEFAULT) -> tuple[KeyConflict, ...]:
        groups: dict[str, list[KeyBinding]] = {}
        for binding in self._bindings[preset]:
            groups.setdefault(binding.key.casefold(), []).append(binding)
        return tuple(
            KeyConflict(key=key, bindings=tuple(bindings))
            for key, bindings in groups.items()
            if len(bindings) > 1
        )

    def search(self, query: str, preset: KeymapPreset = KeymapPreset.DEFAULT) -> tuple[KeyBinding, ...]:
        needle = query.casefold().strip()
        if not needle:
            return self.bindings(preset)
        return tuple(
            binding for binding in self._bindings[preset]
            if needle in " ".join((binding.command_id, binding.key, binding.description, binding.category)).casefold()
        )

    def to_spec(self, preset: KeymapPreset = KeymapPreset.DEFAULT) -> list[dict[str, str | None]]:
        return [binding.to_spec() for binding in self._bindings[preset]]
