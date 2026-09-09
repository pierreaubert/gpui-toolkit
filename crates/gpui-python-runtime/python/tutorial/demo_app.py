"""First gpui-toolkit app: text, metrics, a px line chart, and a button.

Install only the wheel, then open a native window::

    pip install gpui-toolkit
    python demo_app.py
"""
from __future__ import annotations

from dataclasses import dataclass
import math

from gpui_toolkit import App, Event, SessionContext, data, px, section, ui


def build_dataset() -> data.Dataset:
    xs = [float(index) for index in range(72)]
    sine = [math.sin(x * 0.35) for x in xs]
    return data.Dataset.from_mapping(
        {
            "x": xs + xs,
            "y": sine + [0.0 for _ in xs],
            "series": ["Sine"] * len(xs) + ["Zero"] * len(xs),
            "color": ["#60a5fa"] * len(xs) + ["#22c55e"] * len(xs),
        },
        id="demo-wave",
    )


@dataclass
class DemoApp(App):
    clicks: int = 0

    def on_action(self, event: Event, context: SessionContext) -> None:
        if event.action != "demo_bump":
            return
        self.clicks += 1
        context.acknowledge(event)
        context.patch(
            [
                {
                    "op": "set",
                    "id": "demo-clicks",
                    "property": "value",
                    "value": self.clicks,
                }
            ],
            request_id=event.id,
        )


def build_app() -> DemoApp:
    wave = build_dataset()
    chart = (
        px.line("demo-line")
        .data(wave)
        .x("x")
        .y("y")
        .series("series")
        .color("color")
        .title("Sine wave")
        .x_label("Sample")
        .y_label("Amplitude")
    )
    app = DemoApp(
        title="GPUI Python Demo",
        sidebar_title="Python Demo",
        sidebar_subtitle="declared in Python, rendered natively",
        sections=[
            section(
                "overview",
                "Overview",
                ui.vstack(
                    [
                        ui.section_header(
                            "Hello from Python",
                            "This UI is declared in Python and rendered by Rust/GPUI.",
                        ),
                        ui.hstack(
                            [
                                ui.metric("Samples", 72, id="demo-samples"),
                                ui.metric("Clicks", 0, id="demo-clicks"),
                            ],
                            gap=12.0,
                        ),
                        ui.button(
                            "Click me",
                            id="demo-button",
                            action="demo_bump",
                        ),
                        ui.text(
                            "Press the button: on_action runs in Python and patches "
                            "the Clicks metric without rebuilding the window."
                        ),
                    ],
                    gap=16.0,
                ),
            ),
            section(
                "chart",
                "Chart",
                ui.vstack(
                    [
                        ui.section_header(
                            "Native chart",
                            "One Dataset resource drives the px line renderer.",
                        ),
                        chart,
                    ],
                    gap=16.0,
                ),
            ),
        ],
    )
    app.resources = (wave,)
    return app


if __name__ == "__main__":
    build_app().run()
