"""First gpui-toolkit app: metrics, a theme picker, and px charts.

Install only the wheel, then open a native window::

    pip install gpui-toolkit
    python demo_app.py

The app runs inside the native miniapp shell (``MiniAppConfig`` with
``with_theme=True``), so the host owns window chrome plus a shell theme
switcher. The Appearance picker below drives the app's own theme state
and echoes the choice through the Theme metric.
"""
from __future__ import annotations

from dataclasses import dataclass
import math

from gpui_toolkit import App, Event, SessionContext, data, px, section, ui
from gpui_toolkit.miniapp import MiniAppConfig

THEME_OPTIONS = (
    "dark",
    "light",
    "midnight",
    "forest",
    "black_and_white",
    "onyx",
    "carbon_white",
    "carbon_gray_10",
    "carbon_gray_90",
    "carbon_gray_100",
)
CLICK_GOAL = 10


def theme_label(theme: str) -> str:
    return theme.replace("_", " ").title()


def theme_from_shell_name(name: object) -> str | None:
    """Map the host shell theme display name back to a theme option id."""
    candidate = "_".join(str(name).lower().replace("&", "and").split())
    return candidate if candidate in THEME_OPTIONS else None


def wave_columns() -> dict[str, list]:
    xs = [float(index) for index in range(72)]
    sine = [math.sin(x * 0.35) for x in xs]
    cosine = [math.cos(x * 0.35) for x in xs]
    return {
        "x": xs + xs + xs,
        "y": sine + cosine + [0.0 for _ in xs],
        "series": ["Sine"] * len(xs) + ["Cosine"] * len(xs) + ["Zero"] * len(xs),
        "color": ["#60a5fa"] * len(xs) + ["#f59e0b"] * len(xs) + ["#22c55e"] * len(xs),
    }


def build_dataset() -> data.Dataset:
    return data.Dataset.from_mapping(wave_columns(), id="demo-wave")


def build_peaks_dataset() -> data.Dataset:
    columns = wave_columns()
    peaks: dict[str, float] = {}
    for series, value in zip(columns["series"], columns["y"]):
        peaks[series] = max(peaks.get(series, 0.0), abs(value))
    ordered = sorted(peaks.items(), key=lambda item: item[1], reverse=True)
    return data.Dataset.from_mapping(
        {
            "series": [series for series, _ in ordered],
            "peak": [peak for _, peak in ordered],
        },
        id="demo-peaks",
    )


@dataclass
class DemoApp(App):
    clicks: int = 0
    theme: str = "dark"

    def on_action(self, event: Event, context: SessionContext) -> None:
        if event.action == "demo_bump":
            self.clicks += 1
            context.acknowledge(event)
            context.patch(
                [
                    {
                        "op": "set",
                        "id": "demo-clicks",
                        "property": "value",
                        # Metric values are strings in the UI IR; sending a
                        # raw integer fails host validation and kills the
                        # session with an error screen.
                        "value": str(self.clicks),
                    },
                    {
                        "op": "set",
                        "id": "demo-progress",
                        "property": "value",
                        "value": min(self.clicks / CLICK_GOAL, 1.0),
                    },
                ],
                request_id=event.id,
            )
        elif event.action == "demo_theme":
            choice = (event.payload or {}).get("value", self.theme)
            if choice not in THEME_OPTIONS:
                return
            self.theme = choice
            context.acknowledge(event)
            context.patch(
                [
                    {
                        "op": "set",
                        "id": "demo-theme",
                        "property": "value",
                        "value": theme_label(choice),
                    }
                ],
                request_id=event.id,
            )
        elif event.action == "miniapp_theme_changed":
            # The shell theme switcher bypasses the picker: adopt the new
            # shell theme so app state stays consistent with the window,
            # and sync the picker display to match.
            choice = theme_from_shell_name((event.payload or {}).get("theme", ""))
            if choice is None:
                return
            self.theme = choice
            context.acknowledge(event)
            context.patch(
                [
                    {
                        "op": "set",
                        "id": "demo-theme",
                        "property": "value",
                        "value": theme_label(choice),
                    },
                    {
                        "op": "set",
                        "id": "demo-appearance",
                        "property": "value",
                        "value": choice,
                    },
                ],
                request_id=event.id,
            )


def build_overview(app: DemoApp) -> ui.Node:
    return ui.vstack(
        [
            ui.section_header(
                "Hello from Python",
                "Declared in Python, themed by the miniapp shell, rendered natively.",
            ),
            ui.hstack(
                [
                    ui.badge("tutorial", tone="neutral"),
                    ui.badge("miniapp shell", tone="accent"),
                    ui.badge("themes on", tone="success"),
                ],
                gap=8.0,
            ),
            ui.hstack(
                [
                    ui.metric("Samples", len(wave_columns()["x"]), id="demo-samples"),
                    ui.metric("Clicks", app.clicks, id="demo-clicks"),
                    ui.metric("Theme", theme_label(app.theme), id="demo-theme"),
                ],
                gap=12.0,
            ),
            ui.card(
                [
                    ui.text(
                        "Pick an app theme. The shell switcher handles window "
                        "chrome; this picker drives the app's own theme state.",
                        tone="secondary",
                    ),
                    ui.select(
                        id="demo-appearance",
                        label="Appearance",
                        value=app.theme,
                        options=[(theme, theme_label(theme)) for theme in THEME_OPTIONS],
                        # The `theme:` prefix asks the host to switch its
                        # ThemeState on change; the stripped `demo_theme`
                        # action still reaches on_action for app state.
                        action="theme:demo_theme",
                    ),
                ],
                title="Appearance",
            ),
            ui.button(
                "Click me",
                id="demo-button",
                action="demo_bump",
            ),
            ui.progress(0.0, label="Click goal", id="demo-progress"),
            ui.divider(),
            ui.text(
                "Press the button: on_action runs in Python and patches "
                "the Clicks metric and the goal bar without rebuilding the window.",
                tone="secondary",
            ),
        ],
        gap=16.0,
    )


def build_charts(wave: data.Dataset, peaks: data.Dataset) -> ui.Node:
    chart = (
        px.line("demo-line")
        .data(wave)
        .x("x")
        .y("y")
        .series("series")
        .color("color")
        .title("Waveforms")
        .x_label("Sample")
        .y_label("Amplitude")
        .fill()
        .min_size(940.0, 380.0)
        # Retained host interaction: wheel zoom, drag pan, double-click reset.
        # Viewport events are informational; the demo keeps no viewport state.
        .on_viewport_change("demo_viewport")
    )
    summary = (
        px.bar("demo-peaks")
        .data(peaks)
        .x("series")
        .y("peak")
        .title("Peak amplitude by series")
        .bar_gap(0.35)
        .fill()
        .min_size(940.0, 340.0)
        # Same retained interaction as the line chart; the host slices the
        # visible categories from the zoomed x domain (category index space).
        .on_viewport_change("demo_viewport")
    )
    return ui.vstack(
        [
            ui.section_header(
                "Native charts",
                "One wave dataset plus a peaks summary drive the px renderers.",
            ),
            chart,
            summary,
            ui.text(
                "Both charts read Arrow IPC published from Python; switch "
                "the shell theme and the data stays put.",
                tone="secondary",
            ),
        ],
        gap=16.0,
    )


def build_app() -> DemoApp:
    wave = build_dataset()
    peaks = build_peaks_dataset()
    app = DemoApp(
        title="GPUI Python Demo",
        sidebar_title="Python Demo",
        sidebar_subtitle="declared in Python, rendered natively",
        miniapp=MiniAppConfig(
            title="GPUI Python Demo",
            width=1280.0,
            height=860.0,
            with_theme=True,
            initial_theme="dark",
        ),
        sections=[],
    )
    app.sections = [
        section("overview", "Overview", build_overview(app)),
        section("chart", "Chart", build_charts(wave, peaks)),
    ]
    app.resources = (wave, peaks)
    return app


if __name__ == "__main__":
    build_app().run()
