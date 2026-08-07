"""Theme-selection declarations; live palette application remains host-owned."""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import TYPE_CHECKING

from .commands import CommandResult, CommandStatus

if TYPE_CHECKING:
    from .app import SessionContext


class ThemeAppearance(str, Enum):
    LIGHT = "light"
    DARK = "dark"


class AccessibilityPalette(str, Enum):
    STANDARD = "standard"
    HIGH_CONTRAST = "high_contrast"
    PROTANOPIA = "protanopia"
    DEUTERANOPIA = "deuteranopia"
    TRITANOPIA = "tritanopia"


class BuiltInThemePreset(str, Enum):
    DARK = "dark"
    LIGHT = "light"
    HIGH_CONTRAST = "high_contrast"
    NORD = "nord"
    DRACULA = "dracula"
    PROTANOPIA = "protanopia"
    DEUTERANOPIA = "deuteranopia"
    TRITANOPIA = "tritanopia"


class ThemeTransitionEasing(str, Enum):
    LINEAR = "linear"
    EASE_OUT = "ease_out"
    EASE_IN_OUT = "ease_in_out"


@dataclass(frozen=True)
class TimeOfDay:
    hour: int
    minute: int

    def __post_init__(self) -> None:
        if not 0 <= self.hour < 24 or not 0 <= self.minute < 60:
            raise ValueError("time of day must be within 00:00–23:59")

    @property
    def minutes_after_midnight(self) -> int:
        return self.hour * 60 + self.minute


@dataclass(frozen=True)
class ThemeSchedule:
    light_start: TimeOfDay = field(default_factory=lambda: TimeOfDay(7, 0))
    dark_start: TimeOfDay = field(default_factory=lambda: TimeOfDay(18, 0))

    def resolve_at_minutes(self, minutes_after_midnight: int) -> ThemeAppearance:
        minute = minutes_after_midnight % (24 * 60)
        light, dark = self.light_start.minutes_after_midnight, self.dark_start.minutes_after_midnight
        if light == dark:
            return ThemeAppearance.DARK
        is_light = light <= minute < dark if light < dark else minute >= light or minute < dark
        return ThemeAppearance.LIGHT if is_light else ThemeAppearance.DARK


@dataclass(frozen=True)
class ThemeModePreference:
    """A serializable mode selection equivalent to Rust's tagged enum."""

    mode: str = "follow_system"
    schedule: ThemeSchedule | None = None

    def __post_init__(self) -> None:
        valid = {"follow_system", "light", "dark", "scheduled"}
        if self.mode not in valid or (self.mode == "scheduled") != (self.schedule is not None):
            raise ValueError("scheduled mode requires a schedule; other modes must not have one")

    def resolve(self, system_appearance: ThemeAppearance, minutes_after_midnight: int) -> ThemeAppearance:
        if self.mode == "follow_system": return system_appearance
        if self.mode == "light": return ThemeAppearance.LIGHT
        if self.mode == "dark": return ThemeAppearance.DARK
        assert self.schedule is not None
        return self.schedule.resolve_at_minutes(minutes_after_midnight)


@dataclass(frozen=True)
class ThemeTransition:
    duration_ms: int = 220
    easing: ThemeTransitionEasing = ThemeTransitionEasing.EASE_OUT
    cross_fade: bool = True

    def __post_init__(self) -> None:
        if not 0 <= self.duration_ms <= 65535:
            raise ValueError("theme transition duration must fit u16")

    def effective_duration_ms(self, reduce_motion: bool) -> int:
        return 0 if reduce_motion else self.duration_ms

    @classmethod
    def disabled(cls) -> "ThemeTransition":
        return cls(0, ThemeTransitionEasing.LINEAR, False)


@dataclass(frozen=True)
class ThemeGalleryEntry:
    id: str
    display_name: str
    tags: tuple[str, ...]
    accessibility: AccessibilityPalette
    appearance: ThemeAppearance

    def __post_init__(self) -> None:
        if not self.id or not self.display_name:
            raise ValueError("theme gallery entries require id and display_name")


@dataclass(frozen=True)
class ThemeGallery:
    entries: tuple[ThemeGalleryEntry, ...]

    def by_id(self, theme_id: str) -> ThemeGalleryEntry | None:
        return next((entry for entry in self.entries if entry.id == theme_id), None)


def request_gallery(context: "SessionContext", request_id: str) -> None:
    context.command(request_id, "themes.gallery")


def gallery_from_command(result: CommandResult) -> ThemeGallery:
    if result.status is not CommandStatus.SUCCEEDED:
        raise RuntimeError(result.error or f"theme gallery {result.status.value}")
    return ThemeGallery(
        tuple(
            ThemeGalleryEntry(
                id=str(entry["id"]),
                display_name=str(entry["display_name"]),
                tags=tuple(str(tag) for tag in entry.get("tags", ())),
                accessibility=AccessibilityPalette(str(entry["accessibility"])),
                appearance=ThemeAppearance(str(entry["appearance"])),
            )
            for entry in result.data.get("entries", ())
        )
    )


@dataclass(frozen=True)
class ActiveTheme:
    """A community theme validated and applied by the native host."""

    entry: ThemeGalleryEntry
    active: bool = True


@dataclass(frozen=True)
class CommunityThemeImport:
    """A JSON community bundle validated by the native theme crate."""

    json: str

    def __post_init__(self) -> None:
        if not self.json.strip():
            raise ValueError("community theme JSON cannot be empty")

    def validate(self, context: "SessionContext", request_id: str) -> None:
        context.command(request_id, "themes.community_validate", input=self.json)

    def activate(self, context: "SessionContext", request_id: str) -> None:
        """Validate then apply this theme to the host-owned live palette."""
        context.command(request_id, "themes.community_activate", input=self.json)

    @staticmethod
    def gallery_entry_from_command(result: CommandResult) -> ThemeGalleryEntry:
        if result.status is not CommandStatus.SUCCEEDED:
            raise RuntimeError(result.error or f"community theme validation {result.status.value}")
        try:
            return ThemeGalleryEntry(
                str(result.data["id"]), str(result.data["display_name"]),
                tuple(str(item) for item in result.data["tags"]),
                AccessibilityPalette(str(result.data["accessibility"])),
                ThemeAppearance(str(result.data["appearance"])),
            )
        except (KeyError, TypeError, ValueError) as error:
            raise ValueError("native community theme result has an invalid gallery entry") from error

    @staticmethod
    def active_theme_from_command(result: CommandResult) -> ActiveTheme:
        entry = CommunityThemeImport.gallery_entry_from_command(result)
        return ActiveTheme(entry, bool(result.data.get("active", False)))
