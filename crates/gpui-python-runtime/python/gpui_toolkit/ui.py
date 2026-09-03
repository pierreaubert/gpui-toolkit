"""Declarative UI helpers for the GPUI Python wrapper."""

from __future__ import annotations

from dataclasses import dataclass, field, replace
from enum import Enum
from typing import TYPE_CHECKING, Any, Iterable, Sequence


# V2 strict table declarations. Legacy ``table(..., **props)`` remains for v1.
class SelectionMode(str, Enum):
    NONE = "none"
    SINGLE = "single"
    MULTIPLE = "multiple"


@dataclass(frozen=True)
class Column:
    id: str
    _field: str | None = None
    _sortable: bool = False
    _min_width: float | None = None
    _template: Any = None

    def __post_init__(self) -> None:
        if not self.id:
            raise ValueError("column id must be non-empty")

    def field(self, value: str) -> "Column":
        if not value:
            raise ValueError("column field must be non-empty")
        return replace(self, _field=value)

    def sortable(self, value: bool = True) -> "Column":
        return replace(self, _sortable=bool(value))

    def min_width(self, value: float) -> "Column":
        if value <= 0:
            raise ValueError("column minimum width must be positive")
        return replace(self, _min_width=float(value))

    def template(self, value: Any) -> "Column":
        from .data import FieldRef
        if not isinstance(value, FieldRef) and not hasattr(value, "to_spec"):
            raise TypeError("column template must be a data.FieldRef or typed node")
        return replace(self, _template=value)

    def to_spec(self) -> dict[str, Any]:
        if self._field is None:
            raise ValueError(f"column {self.id!r} requires .field(...)")
        spec = {"id": self.id, "field": self._field, "sortable": self._sortable, "min_width": self._min_width}
        if self._template is not None:
            spec["template"] = self._template.to_spec()
        return spec


@dataclass(frozen=True)
class Table:
    id: str
    _data: Any = None
    _columns: tuple[Column, ...] = ()
    _selection_mode: SelectionMode = SelectionMode.NONE
    _row_height: float = 28.0
    _overscan: int = 8
    _selection_action: str | None = None

    def __post_init__(self) -> None:
        if not self.id:
            raise ValueError("table id must be non-empty")

    def data(self, source: Any) -> "Table":
        from .data import Dataset, DatasetView
        if not isinstance(source, (Dataset, DatasetView)):
            raise TypeError("Table.data requires a data.Dataset or data.DatasetView")
        return replace(self, _data=source)

    def column(self, value: Column) -> "Table":
        if not isinstance(value, Column):
            raise TypeError("Table.column requires a ui.Column")
        if any(column.id == value.id for column in self._columns):
            raise ValueError(f"duplicate table column {value.id!r}")
        return replace(self, _columns=self._columns + (value,))

    def selection_mode(self, value: SelectionMode) -> "Table":
        return replace(self, _selection_mode=SelectionMode(value))

    def virtualize(self, *, row_height: float, overscan: int = 8) -> "Table":
        if row_height <= 0 or overscan < 0:
            raise ValueError("row_height must be positive and overscan must be non-negative")
        return replace(self, _row_height=float(row_height), _overscan=int(overscan))

    def on_selection_change(self, action: str) -> "Table":
        if not action:
            raise ValueError("selection action must be non-empty")
        return replace(self, _selection_action=action)

    def to_spec(self) -> dict[str, Any]:
        if self._data is None:
            raise ValueError("table requires .data(...) before serialization")
        if not self._columns:
            raise ValueError("table requires at least one column")
        from .data import DatasetView
        dataset = self._data.dataset if isinstance(self._data, DatasetView) else self._data
        required = {column._field for column in self._columns if column._field is not None}
        if self._selection_mode is not SelectionMode.NONE and dataset.key is not None:
            required.add(dataset.key)
        if isinstance(self._data, DatasetView):
            self._data._validate_fields(required)
        else:
            dataset._validate_fields(required)
        if self._selection_mode is not SelectionMode.NONE and dataset.key is None:
            raise ValueError("table selection requires a dataset primary key")
        return {"kind": "table_v2", "id": self.id, "data": self._data.to_spec(), "columns": [column.to_spec() for column in self._columns], "selection_mode": self._selection_mode.value, "virtualize": {"row_height": self._row_height, "overscan": self._overscan}, "selection_action": self._selection_action}

from .commands import CommandResult, CommandStatus

if TYPE_CHECKING:
    from .app import SessionContext


@dataclass(frozen=True)
class UiConformanceReport:
    schema_version: int
    report_type: str
    reviewed_on: str
    entry_count: int
    all_release_ready: bool
    markdown: str


@dataclass(frozen=True)
class UiReports:
    accessibility: UiConformanceReport
    focus: UiConformanceReport
    behavior: UiConformanceReport


def request_reports(context: "SessionContext", request_id: str) -> None:
    context.command(request_id, "ui.reports")


def reports_from_command(result: CommandResult) -> UiReports:
    if result.status is not CommandStatus.SUCCEEDED:
        raise RuntimeError(result.error or f"UI reports {result.status.value}")

    def decode(value: dict[str, Any]) -> UiConformanceReport:
        return UiConformanceReport(
            schema_version=int(value["schema_version"]),
            report_type=str(value["report_type"]),
            reviewed_on=str(value["reviewed_on"]),
            entry_count=int(value["entry_count"]),
            all_release_ready=bool(value["all_release_ready"]),
            markdown=str(value["markdown"]),
        )

    return UiReports(
        accessibility=decode(result.data["accessibility"]),
        focus=decode(result.data["focus"]),
        behavior=decode(result.data["behavior"]),
    )


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


def thinking_orb(
    state: str,
    *,
    id: str,
    size: float = 96.0,
    points_per_sphere: float = 256.0,
    speed: float = 0.5,
    dot_scale: float = 1.0,
    dot_color: str = "#60a5fa",
    paused: bool = False,
    aria_label: str | None = None,
    **props: Any,
) -> Node:
    """Render the native animated dotted-sphere status indicator."""
    return Node(
        "thinking_orb",
        {
            "id": id,
            "state": state,
            "size": float(size),
            "points_per_sphere": float(points_per_sphere),
            "speed": float(speed),
            "dot_scale": float(dot_scale),
            "dot_color": dot_color,
            "paused": paused,
            "aria_label": aria_label,
            **props,
        },
    )


def breadcrumbs(
    *, id: str, items: Sequence[dict[str, Any] | tuple[str, str]],
    separator: str = "slash", action: str | None = None, **props: Any,
) -> Node:
    """Render native breadcrumbs and emit a semantic item-id change event."""
    normalized = [
        item if isinstance(item, dict) else {"id": item[0], "label": item[1]}
        for item in items
    ]
    return Node("breadcrumbs", {
        "id": id, "items": normalized, "separator": separator, "action": action, **props,
    })


def alert(
    message: str, *, id: str, title: str | None = None, variant: str = "info",
    closeable: bool = False, action: str | None = None, **props: Any,
) -> Node:
    """Render a native alert; ``action`` receives its semantic close event."""
    return Node("alert", {
        "id": id, "message": message, "title": title, "variant": variant,
        "closeable": closeable, "action": action, **props,
    })


def toast(
    message: str, *, id: str, title: str | None = None, variant: str = "info",
    closeable: bool = True, duration_secs: float | None = 5.0,
    action: str | None = None, **props: Any,
) -> Node:
    """Render a native accessibility-announced toast with a close event."""
    return Node("toast", {
        "id": id, "message": message, "title": title, "variant": variant,
        "closeable": closeable, "duration_secs": duration_secs, "action": action, **props,
    })


def tooltip(
    child: Any, content: str, *, id: str, placement: str = "top", delay_ms: int = 200,
    show: bool | None = None, **props: Any,
) -> Node:
    """Wrap one node with native hover/focus tooltip behavior."""
    return Node("tooltip", {
        "id": id, "content": content, "placement": placement, "delay_ms": int(delay_ms),
        "show": show, "child": _spec(child), **props,
    })


def empty_state(
    title: str, *, description: str | None = None, icon: str | None = None,
    action: Any | None = None, **props: Any,
) -> Node:
    """Render the host-native empty-state layout with an optional action node."""
    return Node("empty_state", {
        "title": title, "description": description, "icon": icon,
        "action": None if action is None else _spec(action), **props,
    })


def dialog(
    *, id: str, content: Sequence[Any], title: str | None = None, footer: Sequence[Any] = (),
    size: str = "md", show_close_button: bool = True, close_on_backdrop: bool = True,
    close_action: str | None = None, **props: Any,
) -> Node:
    """Render a retained native modal dialog with typed content/footer slots."""
    return Node("dialog", {
        "id": id, "title": title, "size": size, "content": _children(content),
        "footer": _children(footer), "show_close_button": show_close_button,
        "close_on_backdrop": close_on_backdrop, "close_action": close_action, **props,
    })


def confirm_dialog(
    *, id: str, message: str, title: str | None = None, variant: str = "default",
    confirm_label: str = "Confirm", cancel_label: str = "Cancel",
    confirm_action: str | None = None, cancel_action: str | None = None, **props: Any,
) -> Node:
    """Render a host-native confirmation dialog with semantic outcomes."""
    return Node("confirm_dialog", {
        "id": id, "message": message, "title": title, "variant": variant,
        "confirm_label": confirm_label, "cancel_label": cancel_label,
        "confirm_action": confirm_action, "cancel_action": cancel_action, **props,
    })


@dataclass(frozen=True)
class MenuItem:
    """A typed native-menu item, including nested submenus."""

    id: str = ""
    label: str = ""
    shortcut: str | None = None
    disabled: bool = False
    checkbox: bool = False
    checked: bool = False
    danger: bool = False
    separator: bool = False
    children: Sequence["MenuItem"] = ()

    @classmethod
    def divider(cls) -> "MenuItem":
        return cls(separator=True)

    def to_spec(self) -> dict[str, Any]:
        return {
            "id": self.id, "label": self.label, "shortcut": self.shortcut,
            "disabled": self.disabled, "checkbox": self.checkbox,
            "checked": self.checked, "danger": self.danger,
            "separator": self.separator, "children": _children(self.children),
        }


def context_menu(
    *, id: str, items: Sequence[MenuItem], position: tuple[float, float] = (0.0, 0.0),
    min_width: float = 180.0, focused_index: int | None = None,
    action: str | None = None, close_action: str | None = None,
    focus_action: str | None = None, **props: Any,
) -> Node:
    """Render a native contextual menu with semantic selection and close events."""
    return Node("context_menu", {
        "id": id, "items": _children(items), "position": [float(position[0]), float(position[1])],
        "min_width": float(min_width), "focused_index": focused_index, "action": action,
        "close_action": close_action, "focus_action": focus_action, **props,
    })


def menu(
    *, id: str, items: Sequence[MenuItem], min_width: float = 180.0,
    focused_index: int | None = None, action: str | None = None,
    close_action: str | None = None, focus_action: str | None = None, **props: Any,
) -> Node:
    """Render an inline native menu with keyboard selection semantics."""
    return Node("menu", {
        "id": id, "items": _children(items), "min_width": float(min_width),
        "focused_index": focused_index, "action": action, "close_action": close_action,
        "focus_action": focus_action, **props,
    })


@dataclass(frozen=True)
class MenuBarItem:
    """One typed top-level menu, with stable menu and item IDs."""

    id: str
    label: str
    items: Sequence[MenuItem] = ()

    def to_spec(self) -> dict[str, Any]:
        return {"id": self.id, "label": self.label, "items": _children(self.items)}


def menu_bar(
    *, id: str, items: Sequence[MenuBarItem], active_menu: str | None = None,
    action: str | None = None, toggle_action: str | None = None, **props: Any,
) -> Node:
    """Render a host-native menu bar and its retained active drop-down menu."""
    return Node("menu_bar", {
        "id": id, "items": _children(items), "active_menu": active_menu,
        "action": action, "toggle_action": toggle_action, **props,
    })


def popover(
    trigger: Any, *, id: str, content: Sequence[Any], placement: str = "bottom",
    width: float | None = None, show_backdrop: bool = True,
    close_action: str | None = None, **props: Any,
) -> Node:
    """Anchor native popover content to one retained trigger element."""
    return Node("popover", {
        "id": id, "trigger": _spec(trigger), "content": _children(content),
        "placement": placement, "width": width, "show_backdrop": show_backdrop,
        "close_action": close_action, **props,
    })


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


def mesh_plot(
    plot: Any, *, id: str | None = None, width: float | None = None,
    height: float | None = None, selection_action: str | None = None,
    export_action: str | None = None,
) -> Node:
    """Render a declarative mesh plot with selection and toolbar-export events."""
    spec = _spec(plot)
    if selection_action is None:
        selection_action = getattr(plot, "selection_action", None)
    if export_action is None:
        export_action = getattr(plot, "export_action", None)
    return Node("mesh_plot", {
        "id": id or spec.get("id", spec.get("geometry", {}).get("id", "mesh_plot")),
        "spec": spec, "width": width, "height": height,
        "selection_action": selection_action,
        "export_action": export_action,
    })
