"""Minimal app fixture for the embedded-session integration test."""

from gpui_toolkit.app import App, Event, SessionContext, section


class EmbeddedFixtureApp(App):
    def on_action(self, event: Event, context: SessionContext) -> None:
        context.patch(
            [{"op": "set", "id": "label", "property": "text", "value": "tapped"}],
            request_id=event.id,
        )


def build_app() -> EmbeddedFixtureApp:
    return EmbeddedFixtureApp(
        title="Embedded Fixture",
        sections=[section("main", "Main", {"kind": "text", "id": "label", "text": "hi"})],
        serial_reducer=True,
    )
