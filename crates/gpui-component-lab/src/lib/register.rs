use super::component_story::ComponentStory;
use super::consts::UI_KIT_EXPORTED_COMPONENT_STORIES;
use super::consts::UI_KIT_SHOWCASE_STORIES;
use super::story_prop::StoryProp;
use super::story_registry::StoryRegistry;
use super::theme_preset::ThemePreset;
use super::types::StoryPropValue;
use super::viewport_preset::ViewportPreset;
use anyhow::Result;

pub fn register_ui_kit_stories(registry: &mut StoryRegistry) -> Result<()> {
    registry.register(
        ComponentStory::new(
            "ui-kit.button",
            "gpui-ui-kit",
            "Button",
            "Primary action button",
        )
        .props([
            StoryProp::new("label", "Label", StoryPropValue::Text("Save".into())),
            StoryProp::new(
                "variant",
                "Variant",
                StoryPropValue::Choice("primary".into()),
            )
            .options(["primary", "secondary", "destructive", "ghost", "outline"]),
            StoryProp::new("disabled", "Disabled", StoryPropValue::Bool(false)),
        ]),
    )?;
    registry.register(
        ComponentStory::new(
            "ui-kit.form",
            "gpui-ui-kit",
            "Form Controls",
            "Inputs, toggles, selects, and sliders",
        )
        .props([
            StoryProp::new("label", "Label", StoryPropValue::Text("Gain".into())),
            StoryProp::new("value", "Value", StoryPropValue::Number(0.5)),
        ]),
    )?;
    registry.register(
        ComponentStory::new(
            "ui-kit.status",
            "gpui-ui-kit",
            "Status Indicators",
            "Badges and progress indicators",
        )
        .props([
            StoryProp::new("label", "Label", StoryPropValue::Text("Ready".into())),
            StoryProp::new(
                "variant",
                "Variant",
                StoryPropValue::Choice("success".into()),
            )
            .options(["default", "primary", "success", "warning", "error", "info"]),
            StoryProp::new("value", "Progress", StoryPropValue::Number(0.72)),
        ]),
    )?;
    registry.register(
        ComponentStory::new(
            "ui-kit.navigation",
            "gpui-ui-kit",
            "Tabs",
            "Segmented navigation tabs",
        )
        .props([
            StoryProp::new("variant", "Variant", StoryPropValue::Choice("pills".into())).options([
                "underline",
                "enclosed",
                "pills",
                "vertical_card",
            ]),
            StoryProp::new("selected", "Selected", StoryPropValue::Number(1.0)),
        ]),
    )?;
    registry.register(
        ComponentStory::new(
            "ui-kit.feedback",
            "gpui-ui-kit",
            "Feedback",
            "Alerts and inline feedback states",
        )
        .props([
            StoryProp::new("variant", "Variant", StoryPropValue::Choice("info".into()))
                .options(["info", "success", "warning", "error"]),
            StoryProp::new(
                "message",
                "Message",
                StoryPropValue::Text("Design tokens validated".into()),
            ),
        ]),
    )?;
    registry.register(
        ComponentStory::new(
            "ui-kit.card",
            "gpui-ui-kit",
            "Card",
            "Header, content, and footer slots",
        )
        .props([
            StoryProp::new("title", "Title", StoryPropValue::Text("Preview".into())),
            StoryProp::new(
                "content",
                "Content",
                StoryPropValue::Text("Responsive component composition".into()),
            ),
        ]),
    )?;

    register_ui_kit_exported_component_stories(registry)?;
    register_ui_kit_showcase_stories(registry)
}

fn register_ui_kit_exported_component_stories(registry: &mut StoryRegistry) -> Result<()> {
    for (id, title, description) in UI_KIT_EXPORTED_COMPONENT_STORIES {
        if registry.story(id).is_some() {
            continue;
        }
        registry.register(
            ComponentStory::new(*id, "gpui-ui-kit", *title, *description).props([
                StoryProp::new("label", "Label", StoryPropValue::Text((*title).into())),
                StoryProp::new("value", "Value", StoryPropValue::Number(0.64)),
                StoryProp::new(
                    "variant",
                    "Variant",
                    StoryPropValue::Choice("default".into()),
                )
                .options([
                    "default",
                    "primary",
                    "secondary",
                    "success",
                    "warning",
                    "error",
                    "info",
                    "ghost",
                    "outline",
                ]),
                StoryProp::new("disabled", "Disabled", StoryPropValue::Bool(false)),
                StoryProp::new("selected", "Selected", StoryPropValue::Bool(true)),
                StoryProp::new("open", "Open", StoryPropValue::Bool(true)),
            ]),
        )?;
    }
    Ok(())
}

fn register_ui_kit_showcase_stories(registry: &mut StoryRegistry) -> Result<()> {
    for (id, title, description) in UI_KIT_SHOWCASE_STORIES {
        registry.register(ComponentStory::new(
            *id,
            "gpui-ui-kit",
            *title,
            *description,
        ))?;
    }
    Ok(())
}

pub fn register_px_stories(registry: &mut StoryRegistry) -> Result<()> {
    registry.register(
        ComponentStory::new("px.line", "gpui-px", "Line Chart", "Responsive line chart").props([
            StoryProp::new("series", "Series", StoryPropValue::Choice("sine".into()))
                .options(["sine", "sweep", "flat"]),
            StoryProp::new("fill", "Fill", StoryPropValue::Bool(true)),
        ]),
    )?;
    registry.register(
        ComponentStory::new(
            "px.bar",
            "gpui-px",
            "Bar Chart",
            "Responsive categorical bars",
        )
        .props([
            StoryProp::new("bars", "Bars", StoryPropValue::Number(8.0)),
            StoryProp::new("fill", "Fill", StoryPropValue::Bool(true)),
        ]),
    )?;
    registry.register(
        ComponentStory::new(
            "px.scatter",
            "gpui-px",
            "Scatter Chart",
            "Responsive point cloud chart",
        )
        .props([
            StoryProp::new("points", "Points", StoryPropValue::Number(48.0)),
            StoryProp::new("fill", "Fill", StoryPropValue::Bool(true)),
        ]),
    )?;
    registry.register(
        ComponentStory::new(
            "px.area",
            "gpui-px",
            "Area Chart",
            "Responsive filled area chart",
        )
        .props([
            StoryProp::new(
                "series",
                "Series",
                StoryPropValue::Choice("envelope".into()),
            )
            .options(["envelope", "decay", "baseline"]),
            StoryProp::new("fill", "Fill", StoryPropValue::Bool(true)),
        ]),
    )?;
    registry.register(
        ComponentStory::new(
            "px.heatmap",
            "gpui-px",
            "Heatmap",
            "Responsive scalar-field heatmap",
        )
        .props([
            StoryProp::new("size", "Grid Size", StoryPropValue::Number(18.0)),
            StoryProp::new(
                "scale",
                "Color Scale",
                StoryPropValue::Choice("viridis".into()),
            )
            .options(["viridis", "plasma", "inferno", "heat", "coolwarm", "greys"]),
            StoryProp::new("fill", "Fill", StoryPropValue::Bool(true)),
        ]),
    )?;
    registry.register(
        ComponentStory::new(
            "px.contour",
            "gpui-px",
            "Contour Chart",
            "Responsive filled contour bands",
        )
        .props([
            StoryProp::new("size", "Grid Size", StoryPropValue::Number(24.0)),
            StoryProp::new("fill", "Fill", StoryPropValue::Bool(true)),
        ]),
    )?;
    registry.register(
        ComponentStory::new(
            "px.isoline",
            "gpui-px",
            "Isoline Chart",
            "Responsive contour line chart",
        )
        .props([
            StoryProp::new("size", "Grid Size", StoryPropValue::Number(24.0)),
            StoryProp::new("fill", "Fill", StoryPropValue::Bool(true)),
        ]),
    )?;
    registry.register(
        ComponentStory::new("px.pie", "gpui-px", "Pie Chart", "Responsive pie chart").props([
            StoryProp::new("donut", "Donut", StoryPropValue::Bool(false)),
            StoryProp::new("slices", "Slices", StoryPropValue::Number(5.0)),
            StoryProp::new("fill", "Fill", StoryPropValue::Bool(true)),
        ]),
    )?;
    registry.register(
        ComponentStory::new(
            "px.donut",
            "gpui-px",
            "Donut Chart",
            "Responsive donut chart",
        )
        .props([
            StoryProp::new("slices", "Slices", StoryPropValue::Number(5.0)),
            StoryProp::new("fill", "Fill", StoryPropValue::Bool(true)),
        ]),
    )?;
    registry.register(
        ComponentStory::new(
            "px.boxplot",
            "gpui-px",
            "Box Plot",
            "Responsive grouped distribution chart",
        )
        .props([
            StoryProp::new("groups", "Groups", StoryPropValue::Number(5.0)),
            StoryProp::new("fill", "Fill", StoryPropValue::Bool(true)),
        ]),
    )?;
    registry.register(
        ComponentStory::new(
            "px.treemap",
            "gpui-px",
            "Treemap",
            "Responsive hierarchy chart",
        )
        .props([
            StoryProp::new(
                "tiling",
                "Tiling",
                StoryPropValue::Choice("squarify".into()),
            )
            .options(["squarify", "binary", "slice", "dice"]),
            StoryProp::new("fill", "Fill", StoryPropValue::Bool(true)),
        ]),
    )?;
    registry.register(
        ComponentStory::new(
            "px.surface3d",
            "gpui-px",
            "3D Surface",
            "Responsive GPU-backed 3D surface chart",
        )
        .props([
            StoryProp::new("size", "Grid Size", StoryPropValue::Number(22.0)),
            StoryProp::new(
                "colormap",
                "Colormap",
                StoryPropValue::Choice("viridis".into()),
            )
            .options(["viridis", "plasma", "inferno", "turbo", "coolwarm"]),
            StoryProp::new("wireframe", "Wireframe", StoryPropValue::Bool(false)),
            StoryProp::new("fill", "Fill", StoryPropValue::Bool(true)),
        ]),
    )?;
    registry.register(mesh_plot_story(
        "px.mesh_plot",
        "Mesh Plot",
        "Unstructured scalar triangle mesh with contour and selection states",
        true,
    ))?;
    for (id, title, description) in [
        (
            "px.mesh_plot.mesh_only",
            "Mesh Plot — Mesh only",
            "Deterministic unstructured triangle wireframe",
        ),
        (
            "px.mesh_plot.smooth_fill",
            "Mesh Plot — Smooth fill",
            "Vertex-associated scalar field with smooth interpolation",
        ),
        (
            "px.mesh_plot.flat_fill",
            "Mesh Plot — Flat fill",
            "Cell-associated scalar field with flat interpolation",
        ),
        (
            "px.mesh_plot.filled_contours",
            "Mesh Plot — Filled contours",
            "Unstructured marching-triangle filled contour bands",
        ),
        (
            "px.mesh_plot.isolines",
            "Mesh Plot — Isolines",
            "Unstructured marching-triangle isolines",
        ),
        (
            "px.mesh_plot.combined",
            "Mesh Plot — Combined",
            "Scalar fill with isolines and a wireframe overlay",
        ),
        (
            "px.mesh_plot.axisymmetric_section",
            "Mesh Plot — Axisymmetric section",
            "Deterministic radial/axial section of an annular profile",
        ),
        (
            "px.mesh_plot.revolve",
            "Mesh Plot — Revolve",
            "Retained 3D surface generated from an axisymmetric profile",
        ),
        (
            "px.mesh_plot.surface3d",
            "Mesh Plot — Surface 3D",
            "Small unstructured 3D scalar surface",
        ),
        (
            "px.mesh_plot.large_mesh",
            "Mesh Plot — Large mesh",
            "128×128 surface grid with 32,768 triangles",
        ),
        (
            "px.mesh_plot.picking",
            "Mesh Plot — Picking",
            "Known-cell selection and displayed-value annotation",
        ),
    ] {
        registry.register(mesh_plot_story(id, title, description, false))?;
    }
    Ok(())
}

fn mesh_plot_story(id: &str, title: &str, description: &str, expose_mode: bool) -> ComponentStory {
    let props = if expose_mode {
        vec![
            StoryProp::new("mode", "Mode", StoryPropValue::Choice("combined".into())).options([
                "mesh",
                "smooth_fill",
                "flat_fill",
                "filled_contours",
                "isolines",
                "combined",
            ]),
            StoryProp::new("wireframe", "Wireframe", StoryPropValue::Bool(true)),
            StoryProp::new("fill", "Fill", StoryPropValue::Bool(true)),
        ]
    } else {
        vec![
            StoryProp::new("wireframe", "Wireframe", StoryPropValue::Bool(true)),
            StoryProp::new("fill", "Fill", StoryPropValue::Bool(true)),
        ]
    };

    let mut story = ComponentStory::new(id, "gpui-px", title, description).props(props);
    // Keep MeshPlot's release captures aligned with the chart visual QA
    // contract: dashboard, panel, and mobile widths crossed with three
    // stable color-scheme identifiers.
    story.viewports = vec![
        ViewportPreset::new("dashboard-wide", "Dashboard wide", 1280.0, 760.0),
        ViewportPreset::new("panel-compact", "Panel compact", 720.0, 520.0),
        ViewportPreset::new("mobile-card", "Mobile card", 390.0, 640.0),
    ];
    story.themes = vec![
        ThemePreset::new("light", "Light", "neutral", false),
        ThemePreset::new("dark", "Dark", "apple_hig", false),
        ThemePreset::new("high_contrast", "High contrast", "material3", false),
    ];
    story
}

/// Shared vello raster-backend selector for audio stories: `auto` probes
/// custom-draw support, `cpu` forces `vello_cpu`, `gpu` forces the wgpu
/// custom draw (live output only where wgpu draws dispatch).
fn backend_prop() -> StoryProp {
    StoryProp::new(
        "backend",
        "Backend",
        StoryPropValue::Choice("auto".into()),
    )
    .options(["auto", "cpu", "gpu"])
}

pub fn register_audio_kit_stories(registry: &mut StoryRegistry) -> Result<()> {
    registry.register(
        ComponentStory::new(
            "audio-kit.potentiometer",
            "gpui-audio-kit",
            "Potentiometer",
            "Rotary audio parameter control",
        )
        .props([
            StoryProp::new("label", "Label", StoryPropValue::Text("Frequency".into())),
            StoryProp::new("value", "Value", StoryPropValue::Number(1000.0)),
            StoryProp::new(
                "scale",
                "Scale",
                StoryPropValue::Choice("logarithmic".into()),
            )
            .options(["linear", "logarithmic"]),
            backend_prop(),
        ]),
    )?;
    registry.register(
        ComponentStory::new(
            "audio-kit.vertical-slider",
            "gpui-audio-kit",
            "Vertical Slider",
            "Vertical audio parameter fader",
        )
        .props([
            StoryProp::new("label", "Label", StoryPropValue::Text("Gain".into())),
            StoryProp::new("value", "Value", StoryPropValue::Number(-6.0)),
            StoryProp::new("min", "Min", StoryPropValue::Number(-60.0)),
            StoryProp::new("max", "Max", StoryPropValue::Number(6.0)),
            StoryProp::new("peak", "Peak", StoryPropValue::Number(-1.5)),
            StoryProp::new("ticks", "Ticks", StoryPropValue::Bool(true)),
            StoryProp::new("scale", "Scale", StoryPropValue::Choice("linear".into()))
                .options(["linear", "logarithmic"]),
        ]),
    )?;
    registry.register(
        ComponentStory::new(
            "audio-kit.volume-knob",
            "gpui-audio-kit",
            "Volume Knob",
            "Circular volume control with mute state",
        )
        .props([
            StoryProp::new("label", "Label", StoryPropValue::Text("Output".into())),
            StoryProp::new("value", "Value", StoryPropValue::Number(0.72)),
            StoryProp::new("muted", "Muted", StoryPropValue::Bool(false)),
            backend_prop(),
        ]),
    )?;
    registry.register(
        ComponentStory::new(
            "audio-kit.meter",
            "gpui-audio-kit",
            "Level Meter",
            "Peak and level metering",
        )
        .props([
            StoryProp::new("level_db", "Level", StoryPropValue::Number(-12.0)),
            StoryProp::new("peak_db", "Peak", StoryPropValue::Number(-3.0)),
            backend_prop(),
        ]),
    )?;
    registry.register(
        ComponentStory::new(
            "audio-kit.horizontal-meter",
            "gpui-audio-kit",
            "Horizontal Meter",
            "Tick-aligned horizontal audio meter bar",
        )
        .props([
            StoryProp::new("label", "Label", StoryPropValue::Text("LUFS".into())),
            StoryProp::new("value", "Value", StoryPropValue::Number(-18.0)),
            StoryProp::new("gradient", "Gradient", StoryPropValue::Bool(true)),
            StoryProp::new("kind", "Scale", StoryPropValue::Choice("lufs".into())).options([
                "lufs",
                "stereo_width",
                "peak_spread",
            ]),
        ]),
    )?;
    registry.register(
        ComponentStory::new(
            "audio-kit.spectrum",
            "gpui-audio-kit",
            "Spectrum",
            "Spectrum analyzer element",
        )
        .props([
            StoryProp::new("bins", "Bins", StoryPropValue::Number(64.0)),
            backend_prop(),
        ]),
    )?;
    registry.register(
        ComponentStory::new(
            "audio-kit.backend-compare",
            "gpui-audio-kit",
            "Backend Compare",
            "CPU vs GPU vello rasterization snapshots with pixel diff",
        )
        .props([
            StoryProp::new(
                "preset",
                "Scene",
                StoryPropValue::Choice("knob".into()),
            )
            .options(["knob", "spectrum", "strokes"]),
            StoryProp::new("scale", "Scale", StoryPropValue::Choice("2x".into()))
                .options(["1x", "2x"]),
        ]),
    )?;
    registry.register(
        ComponentStory::new(
            "audio-kit.spectrum-axis",
            "gpui-audio-kit",
            "Spectrum Axes",
            "Reusable logarithmic frequency and dB axes",
        )
        .props([
            StoryProp::new("min_freq", "Min Hz", StoryPropValue::Number(20.0)),
            StoryProp::new("max_freq", "Max Hz", StoryPropValue::Number(20_000.0)),
        ]),
    )
}
