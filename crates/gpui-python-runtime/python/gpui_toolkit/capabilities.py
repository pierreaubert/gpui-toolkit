"""Generated capability descriptors; do not edit by hand."""
from __future__ import annotations
from dataclasses import dataclass

@dataclass(frozen=True)
class Capability:
    id: str
    disposition: str
    python_path: str

_ENTRIES = (
    ('gpui-python-runtime.app', 'direct', 'gpui_toolkit.App'),
    ('gpui-ui-kit.declarative', 'declarative', 'gpui_toolkit.ui'),
    ('gpui-ui-kit.navigation-and-feedback', 'declarative', 'gpui_toolkit.ui.breadcrumbs'),
    ('gpui-ui-kit.overlays', 'declarative', 'gpui_toolkit.ui.popover'),
    ('gpui-px.charts', 'declarative', 'gpui_toolkit.charts'),
    ('gpui-px.extended-chart-families', 'declarative', 'gpui_toolkit.charts.TreemapNode'),
    ('gpui-px.capability-and-visual-reports', 'command', 'gpui_toolkit.charts.request_reports'),
    ('gpui-d3rs.scene3d', 'declarative', 'gpui_toolkit.scene3d'),
    ('gpui-d3rs.zoom-command', 'command', 'gpui_toolkit.d3.ZoomRequest'),
    ('gpui-d3rs.array-search-quantile', 'command', 'gpui_toolkit.d3.ArrayRequest'),
    ('gpui-d3rs.scales', 'command', 'gpui_toolkit.d3.ScaleRequest'),
    ('gpui-d3rs.statistics-and-ticks', 'command', 'gpui_toolkit.d3.StatisticsRequest'),
    ('gpui-d3rs.parity-and-benchmark-reports', 'command', 'gpui_toolkit.d3.request_reports'),
    ('gpui-python-runtime.state', 'direct', 'gpui_toolkit.StateStore'),
    ('gpui-python-runtime.bindings', 'direct', 'gpui_toolkit.State'),
    ('gpui-python-runtime.resources', 'opaque', 'gpui_toolkit.resources.ResourceStore'),
    ('gpui-python-runtime.events', 'event', 'gpui_toolkit.events.Event'),
    ('gpui-ui-kit-macros.generated-behavior', 'non-consumer', 'gpui_toolkit.ui'),
    ('gpui-toolkit.aggregate-host', 'host-owned', 'gpui_toolkit'),
    ('gpui-python-runtime.host', 'command', 'gpui_toolkit.App.run'),
    ('gpui-au.platform', 'platform-unavailable', 'gpui_toolkit.platform.AU_EMBEDDING'),
    ('gpui-ios.platform', 'platform-unavailable', 'gpui_toolkit.platform.IOS_HOST'),
    ('gpui-android.platform', 'platform-unavailable', 'gpui_toolkit.platform.ANDROID_HOST'),
    ('gpui-keybinding.registry', 'direct', 'gpui_toolkit.keybindings.KeybindingRegistry'),
    ('gpui-design.tokens-and-reports', 'direct', 'gpui_toolkit.design.DesignReports'),
    ('gpui-pretext.requests-and-results', 'direct', 'gpui_toolkit.text.TextReports'),
    ('gpui-themes.selection-and-gallery', 'declarative', 'gpui_toolkit.themes.CommunityThemeImport'),
    ('gpui-component-lab.stories', 'declarative', 'gpui_toolkit.lab.ComponentStory'),
    ('gpui-design-tools.token-operations', 'command', 'gpui_toolkit.tooling.DesignTokenOperation'),
    ('gpui-builder.declarations', 'declarative', 'gpui_toolkit.layout.Container'),
    ('gpui-builder.solve-validation-inspection', 'command', 'gpui_toolkit.layout.solve'),
    ('gpui-builder.retained-snapshots-state-accessibility', 'command', 'gpui_toolkit.layout.solve_matrix'),
    ('gpui-builder.full-solver-and-inspection', 'command', 'gpui_toolkit.layout'),
    ('gpui-profiler.snapshots-and-budgets', 'direct', 'gpui_toolkit.profiler.AllocatorTelemetry'),
    ('gpui-scaffolder.commands', 'command', 'gpui_toolkit.scaffolder.ScaffoldOptions'),
    ('gpui-miniapp.configuration', 'command', 'gpui_toolkit.miniapp.MiniAppConfig'),
    ('gpui-audio-kit.declarations-and-reports', 'declarative', 'gpui_toolkit.audio.TickConfig'),
    ('gpui-audio-kit.native-controls', 'declarative', 'gpui_toolkit.audio.potentiometer'),
    ('gpui-audio-kit.bounded-binary-streams', 'host-owned', 'gpui_toolkit.audio.MeterStream'),
    ('gpui-audio-kit.accessibility-reports-and-tokens', 'command', 'gpui_toolkit.audio.AudioReports'),
    ('gpui-audio-kit.full-python-surface', 'direct', 'gpui_toolkit.audio'),
    ('gpui-toolkit.release-qa-snapshots', 'direct', 'gpui_toolkit.reports.ReleaseQaMatrix'),
    ('gpui-toolkit.aggregate-reports', 'direct', 'gpui_toolkit.reports.StabilityReport'),
    ('gpui-python-runtime.host-effects', 'command', 'gpui_toolkit.effects.ConfirmDialog'),
    ('gpui-python-runtime.commands', 'command', 'gpui_toolkit.commands.CommandResult'),
    ('gpui-ui-kit.accessibility-and-focus', 'declarative', 'gpui_toolkit.accessibility.AriaProps'),
    ('gpui-ui-kit.i18n', 'declarative', 'gpui_toolkit.i18n.TranslationCatalog'),
)

def capabilities() -> tuple[Capability, ...]:
    return tuple(Capability(*entry) for entry in _ENTRIES)
