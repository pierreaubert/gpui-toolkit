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


def form(
    *, id: str, children: Sequence[Any], label: str | None = None,
    errors: Sequence[dict[str, Any]] = (), **props: Any,
) -> Node:
    """Group controls and expose a focusable native validation summary."""
    return Node("form", {"id": id, "label": label, "errors": list(errors), **props}, children)


def button(
    label: str, *, id: str | None = None, action: str | None = None,
    selected: bool = False, **props: Any,
) -> Node:
    """Render a button; Python-directed actions require an application ID."""
    return Node("button", {
        "id": id, "label": label, "action": action, "selected": selected, **props,
    })


def badge(label: str, *, tone: str = "neutral", **props: Any) -> Node:
    return Node("badge", {"label": label, "tone": tone, **props})


def metric(label: str, value: str | int | float, **props: Any) -> Node:
    return Node("metric", {"label": label, "value": str(value), **props})


def progress(value: float, *, label: str | None = None, **props: Any) -> Node:
    return Node("progress", {"value": float(value), "label": label, **props})


def spinner(label: str | None = None, **props: Any) -> Node:
    return Node("spinner", {"label": label, **props})


def tabs(
    items: Sequence[str], *, active: int = 0, id: str | None = None,
    action: str | None = None, **props: Any,
) -> Node:
    return Node("tabs", {"id": id, "items": list(items), "active": int(active),
                         "action": action, **props})


def stepper(
    *, id: str, steps: Sequence[str], active: int = 0,
    disabled_steps: Sequence[int] = (), action: str | None = None, **props: Any,
) -> Node:
    """Render a bindable workflow stepper with stable action events."""
    return Node("stepper", {
        "id": id, "steps": list(steps), "active": int(active),
        "disabled_steps": [int(index) for index in disabled_steps], "action": action,
        **props,
    })


def accordion(
    *, id: str, items: Sequence[dict[str, Any] | tuple[str, str, Sequence[Any]]],
    expanded: Sequence[str] = (), multiple: bool = False, action: str | None = None,
    **props: Any,
) -> Node:
    """Render bindable accordion sections with application-defined item IDs."""
    normalized = []
    for item in items:
        if isinstance(item, dict):
            normalized.append({**item, "children": _children(item.get("children", ()))})
        else:
            item_id, title, children = item
            normalized.append({"id": item_id, "title": title, "children": _children(children)})
    return Node("accordion", {
        "id": id, "items": normalized, "expanded": list(expanded),
        "multiple": multiple, "action": action, **props,
    })


def list_editor(
    *, id: str, rows: Sequence[dict[str, Any]], label: str | None = None,
    add_action: str | None = None, remove_action: str | None = None,
    reorder_action: str | None = None, add_label: str | None = None,
    disabled: bool = False, **props: Any,
) -> Node:
    """Render stable, reorderable rows with add/remove structured actions."""
    return Node("list_editor", {
        "id": id, "label": label, "rows": list(rows),
        "add_action": add_action, "remove_action": remove_action,
        "reorder_action": reorder_action, "add_label": add_label,
        "disabled": disabled, **props,
    })


def text_input(
    *, id: str, value: str = "", label: str | None = None,
    placeholder: str | None = None, action: str | None = None,
    commit_action: str | None = None, selection_action: str | None = None,
    password: bool = False,
    disabled: bool = False, read_only: bool = False,
    required: bool = False, validation: dict[str, Any] | None = None,
    help: str | None = None, default_value: Any = None, visible: bool = True,
    width: float | None = None,
    **props: Any,
) -> Node:
    # Passwords are write-only UI state: never place a supplied initial value
    # in a serializable IR snapshot (including GPUI_TOOLKIT_DUMP_IR output).
    # The host receives the value only in a user-originated event payload.
    if password:
        value = ""
        default_value = None
    return Node("text_input", {
        "id": id, "value": value, "label": label, "placeholder": placeholder,
        "action": action, "commit_action": commit_action,
        "selection_action": selection_action, "password": password,
        "disabled": disabled, "read_only": read_only, "required": required,
        "validation": validation, "help": help, "default_value": default_value,
        "visible": visible, "width": width, **props,
    })


def number_input(
    *, id: str, value: float | int | str, label: str | None = None,
    unit: str | None = None, minimum: float | None = None,
    maximum: float | None = None, step: float | None = None,
    precision: int | None = None, action: str | None = None,
    commit_action: str | None = None, validation: dict[str, Any] | None = None,
    disabled: bool = False, read_only: bool = False, required: bool = False,
    help: str | None = None, default_value: Any = None, visible: bool = True,
    width: float | None = None,
    **props: Any,
) -> Node:
    return Node("number_input", {
        "id": id, "value": value, "label": label, "unit": unit,
        "min": minimum, "max": maximum, "step": step, "precision": precision,
        "action": action, "commit_action": commit_action, "validation": validation,
        "disabled": disabled, "read_only": read_only, "required": required,
        "help": help, "default_value": default_value, "visible": visible, "width": width,
        **props,
    })


def slider(
    *, id: str, value: float, minimum: float = 0.0, maximum: float = 1.0,
    label: str | None = None, step: float | None = None,
    action: str | None = None, commit_action: str | None = None,
    disabled: bool = False, show_value: bool = False, width: float | None = None,
    help: str | None = None, default_value: Any = None, visible: bool = True,
    **props: Any,
) -> Node:
    """Render a numeric slider with preview and release-commit actions."""
    return Node("slider", {
        "id": id, "value": float(value), "label": label,
        "min": float(minimum), "max": float(maximum), "step": step,
        "action": action, "commit_action": commit_action,
        "disabled": disabled, "show_value": show_value, "width": width,
        "help": help, "default_value": default_value, "visible": visible,
        **props,
    })


def select(
    *, id: str, value: Any, options: Sequence[tuple[Any, str] | dict[str, Any]],
    label: str | None = None, action: str | None = None, disabled: bool = False,
    help: str | None = None, default_value: Any = None, visible: bool = True,
    width: float | None = None,
    **props: Any,
) -> Node:
    normalized = [
        option if isinstance(option, dict) else {"value": option[0], "label": option[1]}
        for option in options
    ]
    return Node("select", {"id": id, "value": value, "options": normalized,
                           "label": label, "action": action, "disabled": disabled,
                           "help": help, "default_value": default_value, "visible": visible,
                           "width": width, **props})


def color_picker(
    *, id: str, value: str, label: str | None = None, action: str | None = None,
    disabled: bool = False, help: str | None = None, default_value: Any = None,
    visible: bool = True, width: float | None = None, **props: Any,
) -> Node:
    """Render the native RGB/HSL color picker for ``#RRGGBB``/``#RRGGBBAA``."""
    return Node("color_picker", {"id": id, "value": value, "label": label,
        "action": action, "disabled": disabled, "help": help,
        "default_value": default_value, "visible": visible, "width": width, **props})


def path_input(
    *, id: str, value: str = "", label: str | None = None,
    placeholder: str | None = None, mode: str = "open_file",
    filters: Sequence[tuple[str, Sequence[str]] | dict[str, Any]] = (),
    recent_values: Sequence[str] = (), must_exist: bool = False,
    action: str | None = None, commit_action: str | None = None,
    disabled: bool = False, read_only: bool = False, required: bool = False,
    validation: dict[str, Any] | None = None, help: str | None = None,
    default_value: Any = None, visible: bool = True, width: float | None = None,
    **props: Any,
) -> Node:
    """Render a native path editor with manual entry, browse, and recents.

    ``mode`` is ``open_file``, ``directory``, or ``save_file``. Filters are
    declarative ``(label, [extensions...])`` pairs so applications keep their
    file policy independent from the native host implementation.
    """
    normalized_filters = [
        item if isinstance(item, dict) else {"label": item[0], "extensions": list(item[1])}
        for item in filters
    ]
    return Node("path_input", {
        "id": id, "value": value, "label": label, "placeholder": placeholder,
        "mode": mode, "filters": normalized_filters,
        "recent_values": list(recent_values), "must_exist": must_exist,
        "action": action, "commit_action": commit_action, "disabled": disabled,
        "read_only": read_only, "required": required, "validation": validation,
        "help": help, "default_value": default_value, "visible": visible, "width": width,
        **props,
    })


def checkbox(
    *, id: str, value: bool | None, label: str, action: str | None = None,
    indeterminate: bool | None = None, help: str | None = None,
    default_value: Any = None, visible: bool = True, width: float | None = None,
    **props: Any,
) -> Node:
    indeterminate = value is None if indeterminate is None else indeterminate
    return Node("checkbox", {"id": id, "value": bool(value), "indeterminate": indeterminate,
                              "label": label, "action": action, "help": help,
                              "default_value": default_value, "visible": visible, "width": width, **props})


def toggle(*, id: str, value: bool, label: str, action: str | None = None,
           help: str | None = None, default_value: Any = None, visible: bool = True,
           width: float | None = None, **props: Any) -> Node:
    return Node("toggle", {"id": id, "value": bool(value), "label": label, "action": action,
                            "help": help, "default_value": default_value, "visible": visible,
                            "width": width, **props})


def table(
    headers: Sequence[str] = (), rows: Sequence[Sequence[Any]] = (), *,
    id: str | None = None, columns: Sequence[dict[str, Any] | tuple[str, str]] = (),
    typed_rows: Sequence[dict[str, Any]] = (), selected_row: str | None = None,
    selection_action: str | None = None, row_action: str | None = None,
    resize_action: str | None = None,
    sort_action: str | None = None, sort_column: str | None = None,
    sort_direction: str = "ascending",
    row_offset: int = 0, row_limit: int | None = None, **props: Any,
) -> Node:
    normalized_columns = [
        column if isinstance(column, dict) else {"id": column[0], "label": column[1]}
        for column in columns
    ]
    return Node(
        "table",
        {
            "id": id,
            "headers": [str(header) for header in headers],
            "rows": [[str(cell) for cell in row] for row in rows],
            "columns": normalized_columns,
            "typed_rows": list(typed_rows),
            "selected_row": selected_row,
            "selection_action": selection_action,
            "row_action": row_action,
            "resize_action": resize_action,
            "sort_action": sort_action,
            "sort_column": sort_column,
            "sort_direction": sort_direction,
            "row_offset": int(row_offset),
            "row_limit": row_limit,
            **props,
        },
    )


def divider(**props: Any) -> Node:
    return Node("divider", props)


def spacer(**props: Any) -> Node:
    return Node("spacer", props)


def scene3d(
    spec: Any, *, id: str | None = None, width: float | None = None,
    height: float | None = None, selection_action: str | None = None,
) -> Node:
    scene_spec = _spec(spec)
    return Node(
        "scene3d",
        {
            "id": id or scene_spec.get("id", "scene3d"),
            "spec": scene_spec,
            "width": width,
            "height": height,
            "selection_action": selection_action,
        },
    )
