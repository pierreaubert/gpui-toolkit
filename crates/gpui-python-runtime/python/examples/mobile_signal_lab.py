"""A touch-friendly GPUI app that runs unchanged on desktop and mobile.

This example is the Phase 6 reference app for the embedded mobile hosts.
It deliberately stays inside the subset that is identical on desktop,
iOS, and Android:

- declarative sections, cards, metrics, buttons, and form controls;
- one revisioned ``Dataset`` feeding a native line chart;
- Python-owned state updated through ``on_action`` and small patches;
- no subprocesses, downloaded code, compiled Python dependencies, file
  pickers, or assumptions about a writable current directory.

Run it on the desktop with an installed wheel::

    python mobile_signal_lab.py

Or copy it to ``app/main_app.py`` in a project generated with::

    gpui-scaffolder my-app --template python-mobile
"""

from __future__ import annotations

import json
import math
import os
from collections.abc import Mapping
from typing import Any

from gpui_toolkit import App, Event, SessionContext, data, px, section, ui

SAMPLE_COUNT = 96
CLICK_GOAL = 12
WAVEFORMS = ("sine", "triangle", "square")
WAVEFORM_OPTIONS = [(waveform, waveform.title()) for waveform in WAVEFORMS]


def _unit_waveform(waveform: str, phase: float) -> float:
    """Return one sample in ``[-1.0, 1.0]`` for a supported waveform."""
    match waveform:
        case "sine":
            return math.sin(math.tau * phase)
        case "triangle":
            return 1.0 - 4.0 * abs(round(phase - 0.25) - (phase - 0.25))
        case "square":
            return 1.0 if phase - math.floor(phase) < 0.5 else -1.0
        case _:
            raise ValueError(
                f"unsupported waveform {waveform!r}; supported waveforms are "
                + ", ".join(WAVEFORMS)
            )


def waveform_columns(
    *,
    amplitude: float = 1.0,
    cycles: float = 1.5,
    waveform: str = "sine",
    samples: int = SAMPLE_COUNT,
) -> dict[str, list[Any]]:
    """Build deterministic chart columns for the current control state."""
    if samples < 2:
        raise ValueError("waveform requires at least two samples")
    if not math.isfinite(amplitude) or not 0.2 <= amplitude <= 1.5:
        raise ValueError("amplitude must be finite and between 0.2 and 1.5")
    if not math.isfinite(cycles) or not 0.5 <= cycles <= 4.0:
        raise ValueError("cycles must be finite and between 0.5 and 4.0")

    positions = [index / (samples - 1) for index in range(samples)]
    values = [
        amplitude * _unit_waveform(waveform, cycles * position)
        for position in positions
    ]
    return {
        "x": positions,
        "y": values,
        "series": ["Signal"] * samples,
        "color": ["#38bdf8"] * samples,
    }


def waveform_peak(columns: Mapping[str, Any]) -> float:
    """Return the largest absolute signal value in chart columns."""
    return max((abs(float(value)) for value in columns["y"]), default=0.0)


def build_wave_dataset(
    *,
    amplitude: float = 1.0,
    cycles: float = 1.5,
    waveform: str = "sine",
) -> data.Dataset:
    return data.Dataset.from_mapping(
        waveform_columns(amplitude=amplitude, cycles=cycles, waveform=waveform),
        id="mobile-wave",
    )


def _finite_number(value: Any) -> float | None:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return None
    number = float(value)
    return number if math.isfinite(number) else None


def _clean_display_name(value: Any) -> str:
    if not isinstance(value, str):
        return ""
    return " ".join(value.split())[:24]


class MobileSignalLabApp(App):
    """Small stateful app designed around touch-sized controls."""

    def __init__(self) -> None:
        self.clicks = 0
        self.display_name = "Mobile Demo"
        self.waveform = "sine"
        self.amplitude = 1.0
        self.cycles = 1.5
        self.show_peak = True
        self.notifications = False
        self.wave = build_wave_dataset(
            amplitude=self.amplitude,
            cycles=self.cycles,
            waveform=self.waveform,
        )
        super().__init__(
            title="Mobile Signal Lab",
            sidebar_title="Signal Lab",
            sidebar_subtitle="Python on desktop, iOS, and Android",
            width=430.0,
            height=860.0,
            sections=[],
            resources=(self.wave,),
            serial_reducer=True,
        )
        self.sections = [
            section("overview", "Overview", self._overview()),
            section("signal", "Signal", self._signal()),
            section("settings", "Settings", self._settings()),
        ]

    def to_spec(self) -> dict[str, Any]:
        # Rebuild from current state so full snapshots (first frame, or
        # a host reattaching after Android activity recreation) never
        # show values captured at construction time.
        self.sections = [
            section("overview", "Overview", self._overview()),
            section("signal", "Signal", self._signal()),
            section("settings", "Settings", self._settings()),
        ]
        return super().to_spec()

    def _overview(self) -> ui.Node:
        return ui.vstack(
            [
                ui.section_header(
                    "Hello from Python",
                    "One Python app, rendered natively on every platform.",
                ),
                ui.hstack(
                    [
                        ui.metric("Samples", SAMPLE_COUNT, id="sample-count"),
                        ui.metric("Clicks", self.clicks, id="click-count"),
                        ui.metric("Waveform", self.waveform.title(), id="waveform-name"),
                    ],
                    gap=12.0,
                ),
                ui.card(
                    [
                        ui.text(
                            "The button below is intentionally large enough "
                            "for touch. Its action runs here in Python.",
                            tone="secondary",
                        ),
                        ui.button(
                            "Tap to increment",
                            id="increment-button",
                            action="increment",
                        ),
                        ui.progress(
                            min(self.clicks / CLICK_GOAL, 1.0),
                            label="Click goal",
                            id="click-progress",
                        ),
                    ],
                    title="Try the action loop",
                ),
            ],
            gap=16.0,
        )

    def _signal(self) -> ui.Node:
        chart = (
            px.line("mobile-wave-chart")
            .data(self.wave)
            .x("x")
            .y("y")
            .series("series")
            .color("color")
            .title("Generated signal")
            .x_label("Phase")
            .y_label("Amplitude")
            .fill()
            .min_size(320.0, 280.0)
        )
        return ui.vstack(
            [
                ui.section_header(
                    "Signal controls",
                    "Adjust the controls; Python replaces one dataset generation.",
                ),
                chart,
                ui.hstack(
                    [
                        ui.metric("Peak", "1.00", id="peak"),
                        ui.metric("Amplitude", "1.00", id="amplitude-value"),
                        ui.metric("Cycles", "1.50", id="cycles-value"),
                    ],
                    gap=12.0,
                ),
                ui.card(
                    [
                        ui.select(
                            id="waveform",
                            label="Waveform",
                            value=self.waveform,
                            options=WAVEFORM_OPTIONS,
                            action="set_waveform",
                        ),
                        ui.slider(
                            id="amplitude",
                            label="Amplitude",
                            value=self.amplitude,
                            minimum=0.2,
                            maximum=1.5,
                            step=0.05,
                            action="preview_amplitude",
                            commit_action="set_amplitude",
                            show_value=True,
                        ),
                        ui.slider(
                            id="cycles",
                            label="Cycles",
                            value=self.cycles,
                            minimum=0.5,
                            maximum=4.0,
                            step=0.25,
                            action="preview_cycles",
                            commit_action="set_cycles",
                            show_value=True,
                        ),
                        ui.checkbox(
                            id="show-peak",
                            label="Calculate peak value",
                            value=self.show_peak,
                            action="set_show_peak",
                        ),
                    ],
                    title="Generator",
                ),
            ],
            gap=16.0,
        )

    def _settings(self) -> ui.Node:
        return ui.vstack(
            [
                ui.section_header(
                    "Settings",
                    "Form state stays in Python and is patched like every other node.",
                ),
                ui.card(
                    [
                        ui.text_input(
                            id="display-name",
                            label="Display name",
                            value=self.display_name,
                            placeholder="Name shown in the greeting",
                            commit_action="set_display_name",
                            help="Committing the field updates the greeting below.",
                        ),
                        ui.toggle(
                            id="notifications",
                            label="Enable demo notifications",
                            value=self.notifications,
                            action="set_notifications",
                        ),
                        ui.text(
                            f"Hello, {self.display_name}.",
                            id="greeting",
                        ),
                        ui.text(
                            "Notifications are off.",
                            tone="secondary",
                            id="notification-status",
                        ),
                    ],
                    title="Preferences",
                ),
                ui.card(
                    [
                        ui.text(
                            "This demo intentionally uses no subprocesses, "
                            "downloaded code, compiled third-party wheels, "
                            "or writes outside the Python session.",
                            tone="secondary",
                        ),
                    ],
                    title="Mobile contract",
                ),
            ],
            gap=16.0,
        )

    def _peak_text(self, columns: Mapping[str, Any]) -> str:
        return f"{waveform_peak(columns):.2f}" if self.show_peak else "—"

    def _waveform_ops(self, columns: Mapping[str, Any]) -> list[dict[str, Any]]:
        return [
            {
                "op": "set",
                "id": "waveform-name",
                "property": "value",
                "value": self.waveform.title(),
            },
            {
                "op": "set",
                "id": "peak",
                "property": "value",
                "value": self._peak_text(columns),
            },
            {
                "op": "set",
                "id": "amplitude-value",
                "property": "value",
                "value": f"{self.amplitude:.2f}",
            },
            {
                "op": "set",
                "id": "cycles-value",
                "property": "value",
                "value": f"{self.cycles:.2f}",
            },
            {
                "op": "set",
                "id": "waveform",
                "property": "value",
                "value": self.waveform,
            },
            {
                "op": "set",
                "id": "amplitude",
                "property": "value",
                "value": self.amplitude,
            },
            {
                "op": "set",
                "id": "cycles",
                "property": "value",
                "value": self.cycles,
            },
            {
                "op": "set",
                "id": "show-peak",
                "property": "value",
                "value": self.show_peak,
            },
        ]

    def _publish_waveform(
        self,
        event: Event,
        context: SessionContext,
        *,
        commit: bool,
    ) -> None:
        try:
            columns = waveform_columns(
                amplitude=self.amplitude,
                cycles=self.cycles,
                waveform=self.waveform,
            )
        except ValueError as error:
            context.reject(event, "invalid_signal", str(error))
            return

        context.acknowledge(event)
        if commit:
            self.wave.replace(columns)
        context.patch(self._waveform_ops(columns), request_id=event.id)

    def on_action(self, event: Event, context: SessionContext) -> None:
        payload = event.payload or {}

        if event.action == "increment":
            self.clicks += 1
            context.acknowledge(event)
            context.patch(
                [
                    {
                        "op": "set",
                        "id": "click-count",
                        "property": "value",
                        "value": str(self.clicks),
                    },
                    {
                        "op": "set",
                        "id": "click-progress",
                        "property": "value",
                        "value": min(self.clicks / CLICK_GOAL, 1.0),
                    },
                ],
                request_id=event.id,
            )
            return

        if event.action == "set_waveform":
            waveform = payload.get("value")
            if waveform not in WAVEFORMS:
                context.reject(
                    event,
                    "invalid_waveform",
                    f"waveform must be one of: {', '.join(WAVEFORMS)}",
                )
                return
            self.waveform = str(waveform)
            self._publish_waveform(event, context, commit=True)
            return

        if event.action in {
            "preview_amplitude",
            "set_amplitude",
            "preview_cycles",
            "set_cycles",
        }:
            value = _finite_number(payload.get("value"))
            if value is None:
                context.reject(event, "invalid_number", "signal value must be finite")
                return
            if event.action.endswith("amplitude"):
                self.amplitude = min(max(value, 0.2), 1.5)
            else:
                self.cycles = min(max(value, 0.5), 4.0)
            self._publish_waveform(
                event,
                context,
                commit=event.action.startswith("set_"),
            )
            return

        if event.action == "set_show_peak":
            value = payload.get("value")
            if not isinstance(value, bool):
                context.reject(event, "invalid_boolean", "peak preference must be boolean")
                return
            self.show_peak = value
            columns = waveform_columns(
                amplitude=self.amplitude,
                cycles=self.cycles,
                waveform=self.waveform,
            )
            context.acknowledge(event)
            context.patch(self._waveform_ops(columns), request_id=event.id)
            return

        if event.action == "set_display_name":
            clean_name = _clean_display_name(payload.get("value"))
            self.display_name = clean_name or "Mobile Demo"
            context.acknowledge(event)
            context.patch(
                [
                    {
                        "op": "set",
                        "id": "display-name",
                        "property": "value",
                        "value": self.display_name,
                    },
                    {
                        "op": "set",
                        "id": "greeting",
                        "property": "text",
                        "value": f"Hello, {self.display_name}.",
                    },
                ],
                request_id=event.id,
            )
            return

        if event.action == "set_notifications":
            value = payload.get("value")
            if not isinstance(value, bool):
                context.reject(event, "invalid_boolean", "notification preference must be boolean")
                return
            self.notifications = value
            status = (
                "Demo notifications are on. This example does not send any."
                if value
                else "Notifications are off."
            )
            context.acknowledge(event)
            context.patch(
                [
                    {
                        "op": "set",
                        "id": "notifications",
                        "property": "value",
                        "value": value,
                    },
                    {
                        "op": "set",
                        "id": "notification-status",
                        "property": "text",
                        "value": status,
                    },
                ],
                request_id=event.id,
            )
            return

        context.reject(
            event,
            "unknown_action",
            f"Mobile Signal Lab has no action named {event.action!r}.",
        )


def build_app() -> MobileSignalLabApp:
    return MobileSignalLabApp()


def main() -> None:
    # Running the file opens a window: App.run() serves the host session
    # when GPUI_TOOLKIT_SESSION=1 and otherwise execs the bundled native
    # host. Only GPUI_TOOLKIT_DUMP_IR=1 prints the JSON spec instead.
    app = build_app()
    if os.environ.get("GPUI_TOOLKIT_DUMP_IR") == "1":
        print(json.dumps(app.to_spec(), indent=2))
    else:
        app.run()


if __name__ == "__main__":
    main()
