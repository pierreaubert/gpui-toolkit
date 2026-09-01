"""Python-authored GPUI runtime showcase."""

from __future__ import annotations

import math
import time

from gpui_toolkit import App, Event, SessionContext, charts, scene3d as s3, section, ui

ORB_STATES = (
    "working",
    "searching",
    "solving",
    "listening",
    "connecting",
    "weaving",
    "composing",
    "breathing",
    "shaping",
)
ORB_BASE_SIZE = 96.0
ORB_COLOR = "#60a5fa"


class RuntimeShowcase(App):
    """A small live-session path exercised by the bundled native showcase."""

    def on_action(self, event: Event, context: SessionContext) -> None:
        orb_slider_actions = {
            "set_orb_density": ("orb-density", "points_per_sphere"),
            "set_orb_size": ("orb-size", "size"),
            "set_orb_dot_size": ("orb-dot-size", "dot_scale"),
            "set_orb_speed": ("orb-speed", "speed"),
        }
        if event.action in orb_slider_actions:
            control_id, property_name = orb_slider_actions[event.action]
            value = float(event.payload.get("value", 0.0))
            orb_value = ORB_BASE_SIZE * value if property_name == "size" else value
            ops = [
                {"op": "set", "id": control_id, "property": "value", "value": value}
            ]
            for state in ORB_STATES:
                ops.append(
                    {
                        "op": "set",
                        "id": f"thinking-orb-{state}",
                        "property": property_name,
                        "value": orb_value,
                    }
                )
                if property_name == "size":
                    ops.append(
                        {
                            "op": "set",
                            "id": f"thinking-orb-cell-{state}",
                            "property": "width",
                            "value": max(128.0, orb_value),
                        }
                    )
            context.acknowledge(event)
            context.patch(ops, request_id=event.id)
            return

        if event.action == "set_orb_color":
            color = str(event.payload.get("value", ORB_COLOR))
            ops = [
                {
                    "op": "set",
                    "id": "orb-dot-color",
                    "property": "value",
                    "value": color,
                }
            ]
            ops.extend(
                {
                    "op": "set",
                    "id": f"thinking-orb-{state}",
                    "property": "dot_color",
                    "value": color,
                }
                for state in ORB_STATES
            )
            context.acknowledge(event)
            context.patch(ops, request_id=event.id)
            return

        if event.action == "set_simulation_step":
            context.acknowledge(event)
            context.patch([
                {"op": "set", "id": "simulation-workflow", "property": "active",
                 "value": int(event.payload.get("index", 0))},
            ], request_id=event.id)
            return
        if event.action in {"preview_drive_level", "set_frequency_start", "set_speaker_model"}:
            context.acknowledge(event)
            return
        if event.action != "run-showcase-simulation":
            context.reject(event, "unknown_action", "This showcase action is not available.")
            return

        context.acknowledge(event)

        def simulate(token) -> None:
            for completed in range(1, 6):
                if token.cancelled:
                    context.job_log("showcase-simulation", "Simulation cancelled by user.", "warn")
                    return
                context.job(
                    "showcase-simulation", "running", completed=completed, total=5,
                    message=f"Solving frequency band {completed} of 5",
                )
                context.job_log("showcase-simulation", f"Completed band {completed}/5")
                time.sleep(0.08)
            context.patch([
                {"op": "set", "id": "simulation-result", "property": "value", "value": "Ready"},
            ], request_id=event.id)

        context.spawn_job("showcase-simulation", simulate, resource_tags=("simulation",))


def build_app() -> App:
    """Run the component catalog using only the public PyPI wheel API."""
    return RuntimeShowcase(
        title="UI Kit Showcase",
        sidebar_title="UI Kit Showcase",
        sidebar_subtitle="Python · gpui-toolkit wheel",
        sections=native_demo_sections(),
    )

    surface = build_surface_spec()
    lines = build_lines_spec()
    scatter_x, scatter_y = generate_scatter_data()
    line_x, line_y = generate_frequency_response()
    heatmap_size = 24
    heatmap_z = generate_heatmap_data(heatmap_size)

    return RuntimeShowcase(
        title="GPUI Python Runtime Showcase",
        sidebar_title="Python GPUI",
        sidebar_subtitle="Python app, Rust renderers",
        sections=[
            section("overview", "Overview", overview_section()),
            *component_sections(),
            section("thinking-orbs", "Thinking Orbs", thinking_orbs_section()),
            section(
                "charts",
                "gpui-px Charts",
                ui.vstack(
                    [
                        ui.section_header(
                            "gpui-px Charts",
                            "Wheel zoom, drag pan, double-click reset; keyboard +/− and arrows also work.",
                        ),
                        ui.wrap(
                            [
                                charts.scatter(
                                    "latency",
                                    scatter_x,
                                    scatter_y,
                                    title="Callback Latency",
                                    color="#1f77b4",
                                    point_radius=4.0,
                                ),
                                charts.line(
                                    "response",
                                    line_x,
                                    line_y,
                                    title="Frequency Response",
                                    color="#ff7f0e",
                                    x_log=True,
                                    stroke_width=2.0,
                                    x_label="Frequency (Hz)",
                                    y_label="Level (dB)",
                                    series=[
                                        charts.Series("measured", line_x, line_y, label="Measured", color="#ff7f0e"),
                                        charts.Series(
                                            "target",
                                            line_x,
                                            [0.0 for _ in line_x],
                                            label="Target",
                                            color="#22c55e",
                                        ),
                                    ],
                                ),
                                charts.bar(
                                    "scene-nodes",
                                    ["Surface", "Lines", "Mesh", "Light", "Callback"],
                                    [42.0, 31.0, 18.0, 8.0, 5.0],
                                    title="Scene Nodes",
                                    color="#2ca02c",
                                ),
                                charts.heatmap(
                                    "uploads",
                                    heatmap_z,
                                    heatmap_size,
                                    heatmap_size,
                                    title="Upload Activity",
                                    color_scale="viridis",
                                    x=[float(index) for index in range(heatmap_size)],
                                    y=[float(index) for index in range(heatmap_size)],
                                    color_label="Upload intensity",
                                    color_unit="a.u.",
                                    color_range=(0.0, 1.0),
                                    aspect_ratio=1.0,
                                ),
                            ],
                            gap=20.0,
                        ),
                    ],
                    gap=20.0,
                ),
            ),
            section(
                "surface",
                "3D Surface",
                ui.vstack(
                    [
                        ui.section_header("3D Surface", "A log-frequency surface declared in Python"),
                        ui.scene3d(surface, width=760.0, height=480.0),
                        ui.card(
                            [
                                ui.table(
                                    ["field", "value"],
                                    [
                                        ["id", surface.to_spec()["id"]],
                                        ["grid", "10 x 7"],
                                        ["camera", "orbit distance 3.8"],
                                        ["resource path", "Surface3DElement"],
                                    ],
                                )
                            ]
                        ),
                    ],
                    gap=20.0,
                ),
            ),
            section(
                "lines",
                "3D Lines",
                ui.vstack(
                    [
                        ui.section_header("3D Lines", "Line strips use the same retained orbit model"),
                        ui.scene3d(lines, width=700.0, height=440.0),
                        ui.card(
                            [
                                ui.table(
                                    ["field", "value"],
                                    [
                                        ["id", lines.to_spec()["id"]],
                                        ["strips", "helix + xyz axes"],
                                        ["resource path", "Lines3DElement"],
                                    ],
                                )
                            ]
                        ),
                    ],
                    gap=20.0,
                ),
            ),
            section(
                "scene-specs",
                "Scene Specs",
                ui.vstack(
                    [
                        ui.section_header("Scene Specs", "Stable ids drive retained GPU resources"),
                        ui.wrap(
                            [
                                ui.metric("Surface samples", len(surface.to_spec()["z"]["values"])),
                                ui.metric("Line points", 86),
                                ui.metric("Cache entries", 2),
                                ui.metric("Python calls while idle", 0),
                            ],
                            gap=16.0,
                        ),
                        ui.hstack(
                            [
                                ui.scene3d(surface, id="surface-preview", width=420.0, height=280.0),
                                ui.scene3d(lines, id="lines-preview", width=420.0, height=280.0),
                            ],
                            gap=20.0,
                        ),
                        ui.scene3d(build_mesh_scene(), id="mesh-scene-preview", width=420.0, height=180.0),
                    ],
                    gap=20.0,
                ),
            ),
        ],
    )


def overview_section() -> ui.Node:
    return ui.vstack(
        [
            ui.section_header("Python-authored Showcase", "The app shell, sections, charts, and 3D specs are Python data"),
            ui.wrap(
                [
            ui.metric("UI sections", 26),
                    ui.metric("Chart demos", 4),
                    ui.metric("3D specs", 3),
                    ui.metric("Raw wgpu exposed", 0),
                ],
                gap=16.0,
            ),
            ui.card(
                [
                    ui.heading("Runtime Boundary", level=2),
                    ui.text("Python declares stable ids, layout, chart data, and scene3d resources."),
                    ui.text("Rust owns GPUI, gpui-ui-kit rendering, gpui-px charts, retained 3D resources, and the native event loop."),
                    ui.hstack(
                        [
                            ui.badge("JSON UI IR", tone="accent"),
                            ui.badge("Retained 3D", tone="success"),
                            ui.badge("Native GPUI", tone="neutral"),
                        ],
                        gap=8.0,
                    ),
                ],
                width=760.0,
            ),
        ],
        gap=20.0,
    )


def thinking_orbs_section() -> ui.Node:
    controls = ui.vstack(
        [
            ui.slider(
                id="orb-density",
                label="Points per sphere",
                value=256.0,
                minimum=64.0,
                maximum=1024.0,
                step=1.0,
                action="set_orb_density",
                width=300.0,
            ),
            ui.slider(
                id="orb-size",
                label="Sphere size (×)",
                value=1.0,
                minimum=1.0,
                maximum=8.0,
                step=0.25,
                action="set_orb_size",
                width=300.0,
            ),
            ui.slider(
                id="orb-dot-size",
                label="Small dot size (×)",
                value=1.0,
                minimum=0.25,
                maximum=20.0,
                step=0.05,
                action="set_orb_dot_size",
                width=300.0,
            ),
            ui.slider(
                id="orb-speed",
                label="Animation speed (×)",
                value=0.5,
                minimum=0.05,
                maximum=2.0,
                step=0.05,
                action="set_orb_speed",
                width=300.0,
            ),
        ],
        gap=12.0,
        width=340.0,
    )
    color_picker = ui.color_picker(
        id="orb-dot-color",
        label="Small dot color",
        value=ORB_COLOR,
        action="set_orb_color",
        width=400.0,
    )
    orbs = [
        ui.vstack(
            [
                ui.text(state.capitalize()),
                ui.thinking_orb(
                    state,
                    id=f"thinking-orb-{state}",
                    size=ORB_BASE_SIZE,
                    points_per_sphere=256.0,
                    speed=0.5,
                    dot_scale=1.0,
                    dot_color=ORB_COLOR,
                    aria_label=f"{state.capitalize()} thinking orb",
                ),
            ],
            id=f"thinking-orb-cell-{state}",
            gap=8.0,
            width=128.0,
        )
        for state in ORB_STATES
    ]
    return ui.vstack(
        [
            ui.section_header(
                "Thinking Orbs",
                "All nine native status animations with shared appearance and speed controls.",
            ),
            ui.wrap([controls, color_picker], gap=20.0),
            ui.wrap(orbs, gap=16.0),
        ],
        gap=20.0,
    )


NATIVE_SECTION_ORDER = (
    ("buttons", "Buttons"), ("text", "Text"), ("badges", "Badges"),
    ("avatars", "Avatars"), ("form-controls", "Form Controls"),
    ("progress", "Progress"), ("alerts", "Alerts"), ("tabs", "Tabs"),
    ("cards", "Cards"), ("breadcrumbs", "Breadcrumbs"), ("spinners", "Spinners"),
    ("layout", "Layout"), ("icon-buttons", "Icon Buttons"), ("toasts", "Toasts"),
    ("dialog", "Dialog"), ("menu", "Menu"), ("table", "Table"),
    ("tooltips", "Tooltips"), ("accordion", "Accordion"), ("wizard", "Wizard"),
    ("workflow", "Workflow"), ("qr-code", "QR Code"),
    ("context-menu", "Context Menu"), ("popover", "Popover"),
    ("sidebar", "Sidebar"), ("status-bar", "Status Bar"),
    ("search-bar", "Search Bar"), ("keyboard-shortcut", "Keyboard Shortcuts"),
    ("empty-state", "Empty State"), ("confirm-dialog", "Confirm Dialog"),
    ("split-pane", "Split Pane"), ("image-view", "Image View"),
    ("settings-form", "Settings Form"), ("step-indicator", "Step Indicator"),
    ("loading-overlay", "Loading Overlay"), ("tag", "Tag"), ("toolbar", "Toolbar"),
    ("notification", "Notification"), ("tree-view", "Tree View"),
    ("drag-list", "Drag List"), ("command-palette", "Command Palette"),
    ("accessibility", "Accessibility"), ("audio-visuals", "Audio Visuals"),
    ("thinking-orbs", "Thinking Orbs"),
)


def native_demo_sections() -> list:
    """Python-wheel implementation of the native showcase's complete catalog."""
    declared = {item.id: item.content for item in component_sections()}
    tooltip = declared.pop("tooltip")
    declared["tooltips"] = tooltip
    declared["thinking-orbs"] = ui.vstack(
        [
            ui.section_header(
                "Thinking Orbs",
                "Status animations composed from primitives supported by the published wheel host.",
            ),
            ui.wrap(
                [
                    ui.card([ui.text(state.capitalize()), ui.spinner(state.capitalize())], width=160.0)
                    for state in ORB_STATES
                ],
                gap=16.0,
            ),
        ],
        gap=20.0,
    )
    fallback = {
        "avatars": ui.hstack([ui.card([ui.heading("AP", level=2), ui.text("Ada Parker")]), ui.card([ui.heading("ML", level=2), ui.text("Morgan Lee")])], gap=16.0),
        "cards": ui.wrap([ui.card([ui.heading("Project", level=2), ui.text("A flexible content surface.")]), ui.card([ui.metric("Measurements", 24)])], gap=16.0),
        "layout": ui.vstack([ui.hstack([ui.badge("Start"), ui.badge("Center"), ui.badge("End")], gap=12.0), ui.divider(), ui.text("VStack, HStack, Wrap, spacer, and divider are all wheel primitives.")], gap=16.0),
        "icon-buttons": ui.hstack([ui.button("＋"), ui.button("⌕"), ui.button("⚙")], gap=12.0),
        "toasts": ui.vstack([ui.toast(id="showcase-toast", title="Saved", message="The project was saved successfully.", variant="success", duration_secs=None)], gap=12.0),
        "wizard": ui.vstack([ui.stepper(id="showcase-wizard", steps=["Choose", "Configure", "Finish"], active=1), ui.card([ui.heading("Configure", level=2), ui.text("A Python-authored wizard step.")])], gap=16.0),
        "qr-code": ui.card([ui.heading("QR Code", level=2), ui.code("https://gpui.rs/showcase")]),
        "sidebar": ui.hstack([ui.card([ui.heading("Navigation", level=2), ui.text("Overview"), ui.text("Components"), ui.text("Settings")], width=180.0), ui.card([ui.heading("Content", level=2), ui.text("Sidebar layout preview")])], gap=16.0),
        "status-bar": ui.hstack([ui.text("Ready", tone="secondary"), ui.spacer(), ui.text("Python wheel", tone="secondary")], gap=12.0),
        "search-bar": ui.text_input(id="showcase-search", label="Search", placeholder="Search components…", width=420.0),
        "keyboard-shortcut": ui.wrap([ui.code("⌘ K"), ui.code("⌘ ⇧ P"), ui.code("Esc")], gap=12.0),
        "split-pane": ui.hstack([ui.card([ui.text("Left pane")], width=280.0), ui.divider(), ui.card([ui.text("Right pane")], width=280.0)], gap=12.0),
        "image-view": ui.empty_state("Image preview", description="Image rendering is represented by a wheel-authored empty state."),
        "settings-form": ui.form(id="showcase-settings", children=[ui.text_input(id="settings-name", label="Name", value="Default"), ui.select(id="settings-theme", label="Theme", value="dark", options=[("dark", "Dark"), ("light", "Light")]), ui.toggle(id="settings-sync", label="Sync settings", value=True)]),
        "loading-overlay": ui.card([ui.spinner("Loading component preview"), ui.text("The overlay state blocks its content while work completes.")]),
        "tag": ui.wrap([ui.badge("Audio", tone="accent"), ui.badge("GPU", tone="success"), ui.badge("Python")], gap=10.0),
        "toolbar": ui.hstack([ui.button("New"), ui.button("Open"), ui.divider(), ui.button("Share")], gap=10.0),
        "notification": ui.alert("Three tasks completed.", id="showcase-notification", title="Notification", variant="info"),
        "tree-view": ui.accordion(id="showcase-tree", expanded=["src"], items=[("src", "src", [ui.text("showcase.py"), ui.text("components.py")]), ("tests", "tests", [ui.text("test_showcase.py")])]),
        "drag-list": ui.list_editor(id="showcase-drag-list", label="Queue", rows=[{"id": "one", "label": "First"}, {"id": "two", "label": "Second"}]),
        "command-palette": ui.vstack([ui.text_input(id="showcase-command", placeholder="Type a command…", value="", width=440.0), ui.menu(id="showcase-command-menu", items=[ui.MenuItem(id="open", label="Open file"), ui.MenuItem(id="theme", label="Change theme")])], gap=12.0),
        "accessibility": ui.card([ui.heading("Accessible by default", level=2), ui.text("The wheel sends semantic labels and structured events to the native host.")]),
        "audio-visuals": ui.vstack([ui.metric("Peak", "-6.2 dB"), ui.progress(0.72, label="Output level")], gap=16.0),
    }
    sections = []
    for section_id, label in NATIVE_SECTION_ORDER:
        content = declared.get(section_id, fallback.get(section_id))
        if content is None:
            content = ui.card([ui.text(f"{label} is demonstrated by this Python wheel showcase.")])
        if section_id not in declared:
            content = ui.vstack([ui.section_header(label, f"Python implementation of the {label} showcase demo."), content], gap=20.0)
        sections.append(section(section_id, label, content))
    return sections


def component_sections() -> list:
    """The Python equivalents of the native GPUI component catalog.

    Keep the identifiers aligned with ``gpui-showcase`` so the two sidebars
    can be compared directly.  Python-only rendering demos remain in their
    own sections below (charts and the retained 3D scenes).
    """
    menu_items = [
        ui.MenuItem(id="new", label="New window", shortcut="Cmd+N"),
        ui.MenuItem(id="save", label="Save", shortcut="Cmd+S"),
        ui.MenuItem.divider(),
        ui.MenuItem(id="quit", label="Quit", shortcut="Cmd+Q", danger=True),
    ]
    return [
        section("buttons", "Buttons", ui.vstack([
            ui.section_header("Buttons", "Primary, secondary, selected, and disabled actions."),
            ui.hstack([ui.button("Primary", selected=True), ui.button("Secondary"),
                       ui.button("Disabled", disabled=True)], gap=12.0),
        ], gap=20.0)),
        section("text", "Text", ui.vstack([
            ui.section_header("Text", "Typography primitives used throughout a GPUI application."),
            ui.heading("A clear hierarchy", level=2),
            ui.text("Body copy provides context for the control or content beside it."),
            ui.text("Secondary text de-emphasizes supporting information.", tone="secondary"),
            ui.code("let renderer = PythonRuntime::new();", language="rust"),
        ], gap=14.0)),
        section("badges", "Badges", ui.vstack([
            ui.section_header("Badges", "Compact status labels."),
            ui.hstack([ui.badge("Ready", tone="success"), ui.badge("Preview", tone="accent"),
                       ui.badge("Offline", tone="neutral")], gap=10.0),
        ], gap=20.0)),
        section("form-controls", "Form Controls", ui.vstack([
            ui.section_header("Form Controls", "Inputs, selection controls, and validation-friendly metadata."),
            ui.wrap([
                ui.text_input(id="showcase-name", label="Project name", placeholder="Untitled project", width=300.0),
                ui.number_input(id="showcase-rate", label="Sample rate", value=48_000, minimum=8_000, maximum=192_000, step=1_000, width=220.0),
                ui.slider(id="showcase-mix", label="Mix", value=0.65, show_value=True, width=260.0),
                ui.select(id="showcase-preset", label="Preset", value="balanced", options=[("balanced", "Balanced"), ("fast", "Fast"), ("quality", "Quality")], width=220.0),
                ui.checkbox(id="showcase-enabled", value=True, label="Enable processing"),
                ui.toggle(id="showcase-monitor", value=False, label="Monitor output"),
            ], gap=18.0),
        ], gap=20.0)),
        section("breadcrumbs", "Breadcrumbs", ui.vstack([
            ui.section_header("Breadcrumbs", "Navigation context for nested content."),
            ui.breadcrumbs(id="showcase-breadcrumbs", items=[("workspace", "Workspace"), ("demos", "Demos"), ("python", "Python runtime")]),
        ], gap=20.0)),
        section("menu", "Menu", ui.vstack([
            ui.section_header("Menu", "Keyboard-aware application and contextual menu items."),
            ui.menu_bar(id="showcase-menu-bar", items=[ui.MenuBarItem(id="file", label="File", items=menu_items), ui.MenuBarItem(id="edit", label="Edit", items=[ui.MenuItem(id="undo", label="Undo", shortcut="Cmd+Z")])]),
            ui.menu(id="showcase-menu", items=menu_items),
        ], gap=16.0)),
        section("tabs", "Tabs", ui.vstack([
            ui.section_header("Tabs", "Switch between peer views."),
            ui.tabs(["Overview", "Details", "History"], id="showcase-tabs", active=0),
            ui.card([ui.text("The active tab is declared by the Python app.")]),
        ], gap=20.0)),
        section("alerts", "Alerts", ui.vstack([
            ui.section_header("Alerts", "Persistent inline feedback."),
            ui.alert("The analysis needs a calibration file before it can run.", id="showcase-warning", title="Configuration needed", variant="warning"),
            ui.alert("All renderer capabilities are available.", id="showcase-success", variant="success"),
        ], gap=14.0)),
        section("progress", "Progress", ui.vstack([
            ui.section_header("Progress", "Long-running work with a concrete completion state."),
            ui.progress(0.68, label="Preparing geometry"),
        ], gap=20.0)),
        section("spinners", "Spinners", ui.vstack([
            ui.section_header("Spinners", "Indeterminate loading feedback."),
            ui.hstack([ui.spinner("Loading preview"), ui.spinner("Compiling shader")], gap=24.0),
        ], gap=20.0)),
        section("table", "Table", ui.vstack([
            ui.section_header("Table", "Structured, sortable data."),
            ui.table(id="showcase-table", columns=[{"id": "name", "label": "Name", "sortable": True}, {"id": "status", "label": "Status", "sortable": True}], typed_rows=[{"id": "renderer", "cells": ["Renderer", "Ready"]}, {"id": "session", "cells": ["Session", "Connected"]}], sort_column="name"),
        ], gap=20.0)),
        section("accordion", "Accordion", ui.vstack([
            ui.section_header("Accordion", "Progressive disclosure for related settings."),
            ui.accordion(id="showcase-accordion", expanded=["advanced"], items=[("advanced", "Advanced options", [ui.text("These settings are available without leaving the page.")]), ("diagnostics", "Diagnostics", [ui.text("No warnings reported.")])]),
        ], gap=20.0)),
        section("step-indicator", "Step Indicator", ui.vstack([
            ui.section_header("Step Indicator", "A compact multi-step progress display."),
            ui.stepper(id="showcase-stepper", steps=["Source", "Analyze", "Export"], active=1),
        ], gap=20.0)),
        section("empty-state", "Empty State", ui.vstack([
            ui.section_header("Empty State", "A useful zero-content state with a clear next action."),
            ui.empty_state("No measurements yet", description="Add a capture to begin analysis.", action=ui.button("Add measurement")),
        ], gap=20.0)),
        section("tooltip", "Tooltips", ui.vstack([
            ui.section_header("Tooltips", "Hover or focus assistance for concise controls."),
            ui.tooltip(ui.button("Hover me"), "This action opens the analyzer.", id="showcase-tooltip"),
        ], gap=20.0)),
        section("popover", "Popover", ui.vstack([
            ui.section_header("Popover", "An anchored, non-modal detail panel."),
            ui.popover(ui.button("Inspect settings"), id="showcase-popover", content=[ui.heading("Renderer settings", level=3), ui.text("The native host owns popover positioning.")]),
        ], gap=20.0)),
        section("dialog", "Dialog", ui.vstack([
            ui.section_header("Dialog", "A modal surface declared through the Python IR."),
            ui.dialog(id="showcase-dialog", title="Renderer details", content=[ui.text("This dialog is rendered by gpui-ui-kit.")], footer=[ui.button("Close")]),
        ], gap=20.0)),
        section("confirm-dialog", "Confirm Dialog", ui.vstack([
            ui.section_header("Confirm Dialog", "A focused destructive-action confirmation."),
            ui.confirm_dialog(id="showcase-confirm", title="Discard changes?", message="Unsaved renderer settings will be lost.", variant="warning", confirm_label="Discard"),
        ], gap=20.0)),
        section("context-menu", "Context Menu", ui.vstack([
            ui.section_header("Context Menu", "A contextual command list."),
            ui.context_menu(id="showcase-context-menu", items=menu_items, position=(24.0, 24.0)),
        ], gap=20.0)),
        section("workflow", "Workflow", workflow_section()),
    ]


def workflow_section() -> ui.Node:
    return ui.vstack(
        [
            ui.section_header("UI Kit", "Python helpers cover the showcase component set"),
            ui.wrap(
                [
                    ui.card(
                        [
                            ui.heading("Actions", level=2),
                            ui.hstack(
                                [
                                    ui.button("Primary", selected=True),
                                    ui.button("Secondary"),
                                    ui.button("Disabled", disabled=True),
                                ],
                                gap=8.0,
                            ),
                        ],
                        width=360.0,
                    ),
                    ui.card(
                        [
                            ui.heading("Status", level=2),
                            ui.hstack(
                                [
                                    ui.badge("Ready", tone="success"),
                                    ui.badge("Preview", tone="accent"),
                                    ui.badge("Static", tone="neutral"),
                                ],
                                gap=8.0,
                            ),
                            ui.progress(0.68, label="Bridge coverage"),
                            ui.spinner("Renderer warm"),
                        ],
                        width=360.0,
                    ),
                    ui.card(
                        [
                            ui.heading("Navigation", level=2),
                            ui.tabs(["Layout", "Controls", "Data"], active=1),
                            ui.text("Tabs are represented in Python and styled by the host."),
                        ],
                        width=360.0,
                    ),
                    ui.card(
                        [
                            ui.heading("Simulation inputs", level=2),
                            ui.stepper(
                                id="simulation-workflow",
                                steps=["Model", "Solve", "Review"],
                                active=0,
                                action="set_simulation_step",
                            ),
                            ui.number_input(
                                id="frequency-start",
                                label="Start frequency",
                                value="20e",  # Deliberately intermediate text while editing.
                                unit="Hz",
                                commit_action="set_frequency_start",
                                validation={"severity": "error", "message": "Complete the exponent before running."},
                            ),
                            ui.slider(
                                id="drive-level",
                                label="Drive level",
                                value=0.65,
                                minimum=0.0,
                                maximum=1.0,
                                step=0.01,
                                show_value=True,
                                action="preview_drive_level",
                                commit_action="commit_drive_level",
                            ),
                            ui.accordion(
                                id="advanced-solver",
                                expanded=["tolerance"],
                                action="set_advanced_solver",
                                items=[(
                                    "tolerance", "Advanced solver settings", [
                                        ui.text("Changes are validated by the Python session.", tone="secondary"),
                                    ],
                                )],
                            ),
                            ui.list_editor(
                                id="evaluation-frequencies",
                                label="Evaluation frequencies",
                                rows=[
                                    {"id": "frequency-100", "label": "100 Hz", "value": 100.0},
                                    {"id": "frequency-1000", "label": "1 kHz", "value": 1000.0},
                                ],
                                add_action="add_evaluation_frequency",
                                remove_action="remove_evaluation_frequency",
                                reorder_action="reorder_evaluation_frequency",
                            ),
                            ui.path_input(
                                id="speaker-model",
                                label="Speaker model",
                                placeholder="Choose an .mlg or .json model",
                                value="",
                                filters=[("Speaker models", ["mlg", "json"])],
                                recent_values=["/models/reference.mlg", "/models/calibrated.json"],
                                must_exist=True,
                                commit_action="set_speaker_model",
                            ),
                            ui.checkbox(id="field-map", value=None, label="Generate field map"),
                            ui.button(
                                "Run showcase simulation", id="run-showcase-simulation",
                                action="run-showcase-simulation", selected=True,
                            ),
                            ui.metric("Latest result", "Not run", id="simulation-result"),
                        ],
                        width=420.0,
                    ),
                    ui.card(
                        [
                            ui.heading("Data", level=2),
                            ui.table(
                                id="runtime-capabilities",
                                columns=[
                                    {"id": "component", "label": "Component", "sortable": True, "width": 150},
                                    {"id": "state", "label": "State", "sortable": True, "width": 130},
                                ],
                                typed_rows=[
                                    {"id": "buttons", "cells": ["Buttons", "wrapped"]},
                                    {"id": "charts", "cells": ["Charts", "native"]},
                                    {"id": "scene3d", "cells": ["Scene3D", "retained"]},
                                ],
                                sort_action="sort_runtime_capabilities",
                            ),
                        ],
                        width=360.0,
                    ),
                ],
                gap=20.0,
            ),
        ],
        gap=20.0,
    )


def build_surface_spec() -> s3.Surface:
    freqs = [20.0, 40.0, 80.0, 160.0, 315.0, 630.0, 1250.0, 2500.0, 5000.0, 10000.0]
    angles = [-90.0, -60.0, -30.0, 0.0, 30.0, 60.0, 90.0]
    z: list[list[float]] = []

    for angle in angles:
        angle_weight = abs(angle) / 90.0
        row: list[float] = []
        for freq in freqs:
            octave = math.log2(freq / 1000.0)
            on_axis_ripple = 2.0 * math.sin(octave * 2.4)
            off_axis_rolloff = -9.0 * angle_weight * max(0.0, math.log10(freq / 1000.0))
            row.append(on_axis_ripple + off_axis_rolloff)
        z.append(row)

    return s3.surface(
        "dispersion",
        z=z,
        x=freqs,
        y=angles,
        colormap="turbo",
        x_log=True,
        z_range=(-12.0, 4.0),
        labels={"x": "Frequency (Hz)", "y": "Angle (deg)", "z": "Level (dB)"},
        camera=s3.orbit(distance=3.8, azimuth=58.0, elevation=28.0),
        interactions=["orbit", "pan", "zoom", "reset"],
    )


def build_lines_spec() -> s3.Lines:
    helix = []
    for index in range(80):
        t = index / 79.0
        angle = t * 2.5 * math.tau
        radius = 0.7 + 0.2 * math.sin(t * math.tau)
        helix.append((radius * math.cos(angle), (t - 0.5) * 1.8, radius * math.sin(angle)))

    return s3.lines(
        "orbit-lines",
        strips=[
            s3.line_strip("helix", helix, color="#7dd3fc", width=2.5),
            s3.line_strip("x-axis", [(-1.2, 0.0, 0.0), (1.2, 0.0, 0.0)], color="#ef4444"),
            s3.line_strip("y-axis", [(0.0, -1.0, 0.0), (0.0, 1.0, 0.0)], color="#22c55e"),
            s3.line_strip("z-axis", [(0.0, 0.0, -1.2), (0.0, 0.0, 1.2)], color="#3b82f6"),
        ],
        background="#0b1020",
        camera=s3.orbit(distance=4.2, azimuth=42.0, elevation=24.0),
        interactions=["orbit", "pan", "zoom", "reset"],
    )


def build_mesh_scene() -> s3.Scene:
    return s3.scene(
        "speaker-model",
        camera=s3.orbit(distance=3.5, azimuth=45.0, elevation=30.0),
        children=[
            s3.mesh(
                "speaker",
                vertices=[(-0.6, -0.5, 0.0), (0.6, -0.5, 0.0), (0.0, 0.7, 0.0)],
                indices=[0, 1, 2],
                material=s3.material("#88ccff", opacity=0.82),
                scalar_values=[0.1, 0.65, 1.0],
                scalar_location="vertex",
                colormap="turbo",
                scalar_range=(0.0, 1.0),
                scalar_label="Normalized displacement",
            ),
            s3.light("key", direction=(1.0, -2.0, -1.0), intensity=1.3),
        ],
        background="#111827",
    )


def generate_scatter_data() -> tuple[list[float], list[float]]:
    x: list[float] = []
    y: list[float] = []
    for index in range(80):
        t = index / 79.0
        x.append(t * 100.0)
        y.append(20.0 + 28.0 * t + 8.0 * math.sin(t * math.tau * 3.0))
    return x, y


def generate_frequency_response() -> tuple[list[float], list[float]]:
    x: list[float] = []
    y: list[float] = []
    for index in range(72):
        freq = 20.0 * 10 ** (index / 23.0)
        bass_shelf = -5.0 * (120.0 - freq) / 100.0 if freq < 120.0 else 0.0
        treble = -4.0 * (freq - 6000.0) / 14000.0 if freq > 6000.0 else 0.0
        x.append(freq)
        y.append(bass_shelf + treble + 1.2 * math.sin(math.log2(freq / 1000.0) * 3.0))
    return x, y


def generate_heatmap_data(size: int) -> list[float]:
    values: list[float] = []
    for y in range(size):
        for x in range(size):
            nx = x / (size - 1) * 2.0 - 1.0
            ny = y / (size - 1) * 2.0 - 1.0
            left = math.exp(-((nx + 0.35) ** 2 + (ny - 0.2) ** 2) * 8.0)
            right = 0.7 * math.exp(-((nx - 0.4) ** 2 + (ny + 0.25) ** 2) * 18.0)
            values.append(left + right)
    return values


if __name__ == "__main__":
    build_app().run()
