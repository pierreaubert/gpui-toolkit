"""Platform-neutral declarations for optional native GPUI adapters.

The objects in this module are deliberately values, never native views,
window pointers, Metal objects, or JNI handles.  A compatible bundled host
advertises enabled adapters through ``GPUI_TOOLKIT_PLATFORM_CAPABILITIES``.
"""

from __future__ import annotations

import os
import sys
from dataclasses import dataclass
from typing import Final


class UnsupportedCapability(RuntimeError):
    """Raised when an importable optional host service is unavailable."""

    def __init__(self, capability: str, platform: str, supported_platforms: tuple[str, ...]):
        self.capability = capability
        self.platform = platform
        self.supported_platforms = supported_platforms
        super().__init__(
            f"GPUI capability {capability!r} is unavailable on {platform}; "
            f"supported targets: {', '.join(supported_platforms)}"
        )


@dataclass(frozen=True)
class PlatformCapability:
    id: str
    supported_platforms: tuple[str, ...]
    description: str

    @property
    def available(self) -> bool:
        enabled = {
            item.strip()
            for item in os.environ.get("GPUI_TOOLKIT_PLATFORM_CAPABILITIES", "").split(",")
            if item.strip()
        }
        return self.id in enabled

    def require(self) -> "PlatformCapability":
        if not self.available:
            raise UnsupportedCapability(self.id, sys.platform, self.supported_platforms)
        return self


@dataclass(frozen=True)
class AuViewConfig:
    """Declarative Audio Unit embedding configuration; host-owned when active."""

    width: float = 800.0
    height: float = 600.0
    title: str = "GPUI Audio Unit"


@dataclass(frozen=True)
class MobileViewConfig:
    """Declarative mobile lifecycle/view configuration without native handles."""

    title: str = "GPUI Application"
    prefers_dark_mode: bool | None = None
    allows_momentum_scrolling: bool = True


AU_EMBEDDING: Final = PlatformCapability(
    "au_embedding", ("darwin",), "Embed a GPUI view in a macOS AUv3 host.",
)
IOS_HOST: Final = PlatformCapability(
    "ios_host", ("ios",), "Run the iOS GPUI platform adapter.",
)
ANDROID_HOST: Final = PlatformCapability(
    "android_host", ("android",), "Run the Android GPUI platform adapter.",
)
_CAPABILITIES: Final = {item.id: item for item in (AU_EMBEDDING, IOS_HOST, ANDROID_HOST)}


def capabilities() -> tuple[PlatformCapability, ...]:
    """Return every importable platform adapter, including unavailable ones."""
    return tuple(_CAPABILITIES.values())


def require_capability(capability: str) -> PlatformCapability:
    """Return an enabled capability or raise :class:`UnsupportedCapability`."""
    try:
        candidate = _CAPABILITIES[capability]
    except KeyError as error:
        raise ValueError(f"unknown GPUI platform capability: {capability}") from error
    return candidate.require()
