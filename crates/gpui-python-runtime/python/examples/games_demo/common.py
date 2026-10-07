"""Shared patch/IR helpers and board palettes for the classic-games demo.

Loaded by ``games_demo.py`` via ``games_demo_common``; the application shell lives in that entry file.
"""

from __future__ import annotations

from gpui_toolkit import ui
from gpui_toolkit.scene2d import (
    GridSpec,
    Scene2D,
    Scene2DInputConfig,
    Scene2DSemantic,
    SceneRoundedRect,
    SceneText,
    Stroke,
)


# ---------------------------------------------------------------------------
# Small patch/IR helpers
# ---------------------------------------------------------------------------

def _set(node_id: str, prop: str, value: Any) -> dict[str, Any]:
    """Build a ``set`` patch op (the property must exist in the live IR)."""
    return {"op": "set", "id": node_id, "property": prop, "value": value}


def _replace(node_id: str, node: ui.Node) -> dict[str, Any]:
    """Build a ``replace`` patch op for whole-board rebuilds."""
    return {"op": "replace", "id": node_id, "node": node.to_spec()}


def _cell_button(label: str, cell_id: str, action: str, *, selected: bool = False,
                 disabled: bool = False) -> ui.Node:
    """Board button with every patchable property declared up front."""
    return ui.button(label, id=cell_id, action=action, selected=selected,
                     disabled=disabled)


def _status_badge(badge_id: str, label: str, tone: str) -> ui.Node:
    return ui.badge(label, tone=tone, id=badge_id)


DARK_GAME_COLORS = {
    "board": "#172235", "cell": "#202D42", "cell_alt": "#25344B",
    "grid": "#364963", "text": "#F2F6FC", "muted": "#AAB8CC",
    "mint": "#71E0BF", "mint_dim": "#2F786E", "gold": "#FFD36F",
    "danger": "#FF6D7D", "paper": "#F4F1E8", "ink": "#253044",
    "fixed_text": "#F4F1E8", "major_grid": "#D8E1EC",
    "selection": "#314658", "match": "#29424C",
}
LIGHT_GAME_COLORS = {
    "board": "#E8EEF4", "cell": "#FFFFFF", "cell_alt": "#F3F6F9",
    "grid": "#A9B8C7", "text": "#172638", "muted": "#526579",
    "mint": "#087C68", "mint_dim": "#B9E7DA", "gold": "#B87A00",
    "danger": "#B3263B", "paper": "#FFFFFF", "ink": "#172638",
    "fixed_text": "#172638", "major_grid": "#64798B",
    "selection": "#C8ECE3", "match": "#DAEFE9",
}
GAME_PALETTES = {"dark": DARK_GAME_COLORS, "light": LIGHT_GAME_COLORS}
# Kept as the default palette for callers that imported the original constant.
GAME_COLORS = DARK_GAME_COLORS

QUEENS_REGION_COLORS = ("#F1A5A0", "#F7C49A", "#E8D184", "#9ED0B1",
                        "#91C3DB", "#B7A4D8", "#D29BBE", "#B8C7D4")
TETRIS_COLORS = {"I": "#69D7E8", "O": "#F2D56C", "T": "#BA9BFF",
                 "S": "#81DBA1", "Z": "#F47E8A", "J": "#8EABFF", "L": "#FFAE70"}
GAME_CUE_IDS = ("place", "error", "rotate", "lock", "clear", "win")


class _PaletteAwareGame:
    """Per-game board colors, independent of the native window theme."""

    palette: str

    @property
    def colors(self) -> dict[str, str]:
        return GAME_PALETTES[self.palette]


def _scene_cell(prefix: str, grid: GridSpec, row: int, column: int, *,
                fill: str, label: str, value: str = "", selected: bool = False,
                disabled: bool = False, stroke: str | None = None,
                stroke_width: float = 1.0, radius: float = 7.0) -> SceneRoundedRect:
    x, y, width, height = grid.cell_rect(row, column)
    return SceneRoundedRect(
        f"{prefix}-cell-{row}-{column}", x, y, width, height,
        fill=fill,
        stroke=None if stroke is None else Stroke(stroke, stroke_width),
        hit_id=f"{prefix}-cell-{row}-{column}", radius=radius,
        semantic=Scene2DSemantic(
            "grid_cell", label, value_text=value or None,
            selected=selected, disabled=disabled,
        ),
    )


def _scene_text(id: str, center: tuple[float, float], text: str, size: float,
                color: str, *, align: str = "center",
                semantic: Scene2DSemantic | None = None,
                transition: Scene2DTransition | None = None) -> SceneText:
    # Scene2D uses a top-left text origin and a line box of 1.25× the font size.
    return SceneText(id, center[0], center[1] - size * 0.625, text, size, color,
                     align=align, semantic=semantic, transition=transition)


def _grid_surface(surface_id: str, rows: int, columns: int, cell: float,
                  gap: float, padding: float, nodes: list[Any], *,
                  keyboard: bool = False, continuous: bool = False,
                  label: str) -> Scene2D:
    width = padding * 2 + columns * cell + max(0, columns - 1) * gap
    height = padding * 2 + rows * cell + max(0, rows - 1) * gap
    grid = GridSpec(rows, columns, padding, padding, cell, cell, gap)
    return Scene2D(
        surface_id, width, height, nodes, grid,
        Scene2DInputConfig(pointer=True, continuous=continuous, capture=True,
                           keyboard=keyboard),
        semantic=Scene2DSemantic(
            "grid", label, description=f"{rows} rows by {columns} columns"),
    )

