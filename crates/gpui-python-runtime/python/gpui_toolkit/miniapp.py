"""Host-backed configuration for the native ``gpui-miniapp`` shell."""
from __future__ import annotations
from dataclasses import dataclass
from typing import Any

_THEMES = frozenset({"dark", "light", "midnight", "forest", "black_and_white", "onyx", "carbon_white", "carbon_gray_10", "carbon_gray_90", "carbon_gray_100"})
_LANGUAGES = frozenset({"english", "french", "german", "spanish", "japanese"})

@dataclass(frozen=True)
class MiniAppConfig:
    title: str = "MiniApp"
    width: float = 900.0
    height: float = 700.0
    app_name: str | None = None
    scrollable: bool = True
    with_theme: bool = False
    with_i18n: bool = False
    initial_theme: str = "dark"
    initial_language: str = "english"
    def __post_init__(self) -> None:
        if not self.title.strip() or self.width <= 0 or self.height <= 0: raise ValueError("miniapp requires title and positive window dimensions")
        if self.app_name is not None and not self.app_name.strip(): raise ValueError("app_name cannot be blank")
        if self.initial_theme not in _THEMES or self.initial_language not in _LANGUAGES: raise ValueError("miniapp theme or language is unsupported by the native host")
    def to_spec(self) -> dict[str, Any]:
        return {"title": self.title, "width": self.width, "height": self.height, "app_name": self.app_name or self.title, "scrollable": self.scrollable, "with_theme": self.with_theme, "with_i18n": self.with_i18n, "initial_theme": self.initial_theme, "initial_language": self.initial_language}

@dataclass(frozen=True)
class MiniAppCommand:
    config: MiniAppConfig
    root_id: str
    def __post_init__(self) -> None:
        if not self.root_id.strip(): raise ValueError("miniapp command requires a root declaration id")
    def to_spec(self) -> dict[str, Any]: return {"command": "run_miniapp", "root_id": self.root_id, "config": self.config.to_spec()}
