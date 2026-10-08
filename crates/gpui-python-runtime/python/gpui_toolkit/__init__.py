"""Python declarations for GPUI Toolkit.

The Rust runtime consumes the dictionaries produced by these helpers and keeps
GPU resources private.
"""

from . import accessibility, audio, charts, d3, d3rs, data, design, effects, events, i18n, keybindings, lab, layout, meshplot, miniapp, native, platform, profiler, px, reports, resources, scaffolder, scene2d, scene3d, text, themes, tooling, ui
from .app import (
    App,
    CancellationToken,
    Event,
    MeshFrameAcknowledgement,
    ResourceBackpressureError,
    ResourceFrameAcknowledgement,
    Section,
    SessionContext,
    section,
)
from .events import (
    ChartSelection,
    ChartViewport,
    Click,
    Scene2DActivateInput,
    Scene2DEvent,
    Scene2DKeyInput,
    Scene2DLifecycleInput,
    Scene2DPointerInput,
    Scene2DTransitionCompleteInput,
    Scene2DTick,
    Selection,
    Viewport,
    ValueChange,
)
from .game_audio import CueBackend, GameCueAdapter
from .scene2d import (
    GridCell,
    GridSpec,
    PathCommand,
    Scene2D,
    SceneCircle,
    Scene2DInputConfig,
    Scene2DClip,
    Scene2DGroup,
    Scene2DLinearGradient,
    Scene2DPatch,
    ScenePath,
    Scene2DSemantic,
    Scene2DShadow,
    Scene2DTransform,
    Scene2DTransition,
    SceneRect,
    SceneRoundedRect,
    SceneText,
    SceneLine,
    Stroke,
    patch_op,
    replace_op,
)
from .capabilities import Capability, capabilities
from .state import Binding, Computed, State, StateError, StateStore, StoredState, ValidationResult, ValidationSeverity, application_data_dir
from .platform import UnsupportedCapability

__version__ = "0.9.33"

__all__ = [
    "App", "Binding", "CancellationToken", "Capability", "ChartSelection",
    "ChartViewport", "Click", "Computed", "Event", "GridCell", "GridSpec",
    "MeshFrameAcknowledgement", "PathCommand", "ResourceBackpressureError",
    "ResourceFrameAcknowledgement", "Scene2D", "Scene2DPatch",
    "Scene2DEvent", "Scene2DInputConfig", "Scene2DKeyInput", "Scene2DLinearGradient",
    "Scene2DActivateInput",
    "Scene2DLifecycleInput", "Scene2DPointerInput", "Scene2DSemantic",
    "Scene2DTransform", "Scene2DTransition", "Scene2DTransitionCompleteInput",
    "Scene2DClip", "Scene2DGroup", "Scene2DShadow",
    "Scene2DTick", "SceneCircle", "SceneLine", "ScenePath", "SceneRect",
    "SceneRoundedRect", "SceneText", "Section", "CueBackend", "GameCueAdapter",
    "Selection", "SessionContext", "State", "StateError", "StateStore",
    "StoredState", "Stroke", "UnsupportedCapability", "ValidationResult",
    "ValidationSeverity", "ValueChange", "Viewport", "__version__",
    "accessibility", "application_data_dir", "audio", "capabilities", "charts",
    "d3", "d3rs", "data", "design", "effects", "events", "i18n",
    "keybindings", "lab", "layout", "meshplot", "miniapp", "native",
    "patch_op", "platform", "profiler", "px", "replace_op", "reports",
    "resources", "scene2d", "scene3d", "section", "text", "themes", "tooling",
    "ui",
]
