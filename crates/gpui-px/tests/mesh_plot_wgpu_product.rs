//! Product-level GPUI `MeshPlot` capture through the adapter-backed WGPU
//! headless renderer. The paired Metal test uses the same component tree so
//! axes, labels, custom mesh draws, and selected-cell overlays are not tested
//! only through low-level `gpui-d3rs` scenes.

#![cfg(all(target_os = "macos", feature = "native-qa", feature = "headless-qa"))]

use d3rs::mesh::{ScalarField, TriangleMesh};
use gpui::{
    AnyWindowHandle, AppContext, Context, HeadlessAppContext, InteractiveElement, IntoElement,
    ParentElement, PlatformTextSystem, Render, Styled, Window, div, px, size,
};
use gpui_apple::AppleTextSystem;
use gpui_px::{
    Axes2d, Colorbar, FieldInterpolation, MeshPlot, MeshPlotBackend, MeshPlotPick, MeshPlotView,
    MeshRenderMode, Wireframe, mesh_plot,
};
use gpui_wgpu::WgpuHeadlessRenderer;
use image::RgbaImage;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const WIDTH: f32 = 600.0;
const HEIGHT: f32 = 400.0;
const PRODUCT_CAPTURE_DIR_ENV: &str = "GPUI_MESH_PLOT_PRODUCT_CAPTURE_DIR";
const PRODUCT_SOURCE_REVISION_ENV: &str = "GPUI_MESH_PLOT_PRODUCT_SOURCE_REVISION";
const PRODUCT_SOURCE_DIRTY_ENV: &str = "GPUI_MESH_PLOT_PRODUCT_SOURCE_DIRTY";

#[derive(Clone, Copy)]
enum Backend {
    Metal,
    Wgpu,
}

struct ProductMeshPlotView {
    mesh: TriangleMesh,
    field: ScalarField,
    selected: bool,
    backend: MeshPlotBackend,
}

fn product_plot(
    mesh: TriangleMesh,
    field: ScalarField,
    selected: bool,
    backend: MeshPlotBackend,
) -> MeshPlot {
    let (interpolation, color_scale) = match field.association {
        d3rs::mesh::ScalarAssociation::Vertex => {
            (FieldInterpolation::Smooth, gpui_px::ColorScale::Viridis)
        }
        d3rs::mesh::ScalarAssociation::Cell => {
            (FieldInterpolation::Flat, gpui_px::ColorScale::Plasma)
        }
    };
    let mut plot = mesh_plot(mesh.clone())
        .plot_id("product-paired-capture")
        .renderer_backend(backend)
        .field(field.clone())
        .title("Pressure field")
        .size(WIDTH, HEIGHT)
        .view(MeshPlotView::Planar {
            horizontal: d3rs::mesh::CoordinateAxis::X,
            vertical: d3rs::mesh::CoordinateAxis::Y,
        })
        .mode(MeshRenderMode::ScalarFill { interpolation })
        .color_scale(color_scale)
        .wireframe(Wireframe::Overlay)
        .axes(Axes2d::equal_aspect().labels("x", "y").unit("m"))
        .colorbar(Colorbar::new("Pressure").unit("Pa"));
    if selected {
        plot = plot.selection(MeshPlotPick {
            plot_id: "product-paired-capture".into(),
            mesh_id: mesh.id.clone(),
            cell_index: 1,
            cell_id: Some(101),
            nearest_vertex_index: Some(3),
            vertex_id: Some(13),
            world_position: [0.25, 0.75, 0.0],
            displayed_value: Some(0.75),
            field_id: Some(field.id.clone()),
        });
    }
    plot
}

impl Render for ProductMeshPlotView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let plot = product_plot(
            self.mesh.clone(),
            self.field.clone(),
            self.selected,
            self.backend,
        );
        div()
            .id("product-paired-mesh-plot")
            .w(px(WIDTH))
            .h(px(HEIGHT))
            .child(plot.build().expect("product MeshPlot should build"))
    }
}

fn fixture() -> (TriangleMesh, ScalarField) {
    let mesh = TriangleMesh {
        id: "product-paired-mesh".into(),
        positions: Arc::from([
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ]),
        triangles: Arc::from([[0, 1, 2], [0, 2, 3]]),
        vertex_ids: Some(Arc::from([10, 11, 12, 13])),
        cell_ids: Some(Arc::from([100, 101])),
    };
    let field = ScalarField {
        id: "product-paired-field".into(),
        label: "Pressure".into(),
        unit: Some("Pa".into()),
        values: Arc::from([0.0, 0.5, 1.0, 0.25]),
        association: d3rs::mesh::ScalarAssociation::Vertex,
        valid: None,
    };
    (mesh, field)
}

fn text_system() -> Arc<dyn PlatformTextSystem> {
    // Use the native text backend without creating an AppKit platform on a
    // test worker. MacPlatform itself must be constructed on the main thread.
    Arc::new(AppleTextSystem::new())
}

fn capture(backend: Backend, selected: bool) -> Option<RgbaImage> {
    capture_with_field(backend, selected, None)
}

fn capture_with_field(
    backend: Backend,
    selected: bool,
    field_override: Option<ScalarField>,
) -> Option<RgbaImage> {
    capture_with_viewport(backend, selected, field_override, WIDTH)
}

fn capture_with_viewport(
    backend: Backend,
    selected: bool,
    field_override: Option<ScalarField>,
    viewport_width: f32,
) -> Option<RgbaImage> {
    let size = size(px(viewport_width), px(HEIGHT));
    let renderer_available = match backend {
        Backend::Metal => gpui_macos::metal_renderer::MetalHeadlessRenderer::try_new().is_some(),
        Backend::Wgpu => WgpuHeadlessRenderer::try_new(
            gpui::Size {
                width: gpui::DevicePixels(WIDTH as i32),
                height: gpui::DevicePixels(HEIGHT as i32),
            },
            false,
        )
        .is_some(),
    };
    if !renderer_available {
        return None;
    }
    let (mesh, field) = fixture();
    let field = field_override.unwrap_or(field);
    let mut cx = match backend {
        Backend::Metal => HeadlessAppContext::with_platform(text_system(), Arc::new(()), || {
            Ok(Some(Box::new(
                gpui_macos::metal_renderer::MetalHeadlessRenderer::new(),
            )))
        }),
        Backend::Wgpu => HeadlessAppContext::with_platform(text_system(), Arc::new(()), || {
            Ok(WgpuHeadlessRenderer::new(
                gpui::Size {
                    width: gpui::DevicePixels(WIDTH as i32),
                    height: gpui::DevicePixels(HEIGHT as i32),
                },
                false,
            )
            .ok()
            .map(|renderer| Box::new(renderer) as Box<dyn gpui::PlatformHeadlessRenderer>))
        }),
    };
    let window = cx
        .open_window(size, move |_window, app| {
            app.new(|_cx| ProductMeshPlotView {
                mesh,
                field,
                selected,
                backend: match backend {
                    Backend::Metal => MeshPlotBackend::Auto,
                    Backend::Wgpu => MeshPlotBackend::Wgpu,
                },
            })
        })
        .expect("open product MeshPlot capture window");
    let window: AnyWindowHandle = window.into();
    cx.update_window(window, |_, window, app| {
        let _ = window.draw(app);
    })
    .expect("draw product MeshPlot capture window");
    cx.run_until_parked();
    // WGPU atlas uploads are queued during the first scene build; render a
    // second settled frame before reading the target so the capture includes
    // GPUI image overlays such as the retained selection layer.
    cx.update_window(window, |_, window, app| {
        let _ = window.draw(app);
    })
    .expect("draw settled product MeshPlot capture window");
    cx.run_until_parked();
    let image = cx
        .capture_screenshot(window)
        .expect("capture product MeshPlot framebuffer");
    cx.update_window(window, |_, window, _app| window.remove_window())
        .expect("close product MeshPlot capture window");
    cx.run_until_parked();
    Some(image)
}

fn changed_pixels(left: &RgbaImage, right: &RgbaImage) -> usize {
    left.pixels()
        .zip(right.pixels())
        .filter(|(left, right)| left.0 != right.0)
        .count()
}

fn changed_fraction(left: &RgbaImage, right: &RgbaImage) -> f64 {
    changed_pixels(left, right) as f64
        / f64::from(left.width().max(1))
        / f64::from(left.height().max(1))
}

fn json_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn source_provenance_json() -> String {
    let Some(revision) = env::var(PRODUCT_SOURCE_REVISION_ENV).ok() else {
        return String::new();
    };
    let dirty = env::var(PRODUCT_SOURCE_DIRTY_ENV).map_or(true, |value| {
        matches!(value.as_str(), "1" | "true" | "TRUE")
    });
    format!(
        ",\n  \"source_revision\": \"{}\",\n  \"source_dirty\": {}",
        json_string(&revision),
        dirty
    )
}

fn product_capture_dir() -> Option<PathBuf> {
    env::var_os(PRODUCT_CAPTURE_DIR_ENV).map(PathBuf::from)
}

fn write_product_skip(output_dir: &Path, reason: &str) -> Result<(), String> {
    fs::create_dir_all(output_dir).map_err(|error| error.to_string())?;
    let manifest = format!(
        "{{\n  \"schema_version\": 1,\n  \"report_type\": \"gpui-mesh-plot-product-capture\",\n  \"status\": \"skipped\",\n  \"reason\": \"{}\",\n  \"cases\": []{}\n}}\n",
        json_string(reason),
        source_provenance_json(),
    );
    fs::write(output_dir.join("manifest.json"), manifest).map_err(|error| error.to_string())
}

fn write_product_capture(
    output_dir: &Path,
    metal_plain: &RgbaImage,
    metal_selected: &RgbaImage,
    wgpu_plain: &RgbaImage,
    wgpu_selected: &RgbaImage,
) -> Result<(), String> {
    let metal_dir = output_dir.join("metal");
    let wgpu_dir = output_dir.join("wgpu");
    fs::create_dir_all(&metal_dir).map_err(|error| error.to_string())?;
    fs::create_dir_all(&wgpu_dir).map_err(|error| error.to_string())?;
    metal_plain
        .save(metal_dir.join("plain.png"))
        .map_err(|error| error.to_string())?;
    metal_selected
        .save(metal_dir.join("selected.png"))
        .map_err(|error| error.to_string())?;
    wgpu_plain
        .save(wgpu_dir.join("plain.png"))
        .map_err(|error| error.to_string())?;
    wgpu_selected
        .save(wgpu_dir.join("selected.png"))
        .map_err(|error| error.to_string())?;

    let width = metal_plain.width();
    let height = metal_plain.height();
    let metal_changed = changed_pixels(metal_plain, metal_selected);
    let wgpu_changed = changed_pixels(wgpu_plain, wgpu_selected);
    let metal_wgpu_axes_changed = changed_pixels(metal_plain, wgpu_plain);
    let metal_wgpu_selection_changed = changed_pixels(metal_selected, wgpu_selected);
    let manifest = format!(
        "{{\n  \"schema_version\": 1,\n  \"report_type\": \"gpui-mesh-plot-product-capture\",\n  \"status\": \"captured\",\n  \"logical_width\": {},\n  \"logical_height\": {},\n  \"width\": {},\n  \"height\": {},\n  \"composition\": {{\"axes_present\": true, \"axis_titles\": [\"x\", \"y\"], \"colorbar_present\": true, \"selection_annotation_contract\": true}},\n  \"paired_comparison\": {{\"axes\": {{\"comparison_id\": \"px.mesh_plot.product.axes\", \"metal_case\": \"metal-plain\", \"wgpu_case\": \"wgpu-plain\", \"changed_pixels\": {}, \"changed_fraction\": {}}}, \"selection\": {{\"comparison_id\": \"px.mesh_plot.product.selection\", \"metal_case\": \"metal-selected\", \"wgpu_case\": \"wgpu-selected\", \"changed_pixels\": {}, \"changed_fraction\": {}}}}},\n  \"cases\": [\n    {{\"id\": \"metal-plain\", \"renderer\": \"metal-headless\", \"state\": \"plain\", \"comparison_id\": \"px.mesh_plot.product.axes\", \"artifact_kind\": \"png\", \"path\": \"metal/plain.png\", \"axes_present\": true, \"selection_annotation\": false}},\n    {{\"id\": \"metal-selected\", \"renderer\": \"metal-headless\", \"state\": \"selected\", \"comparison_id\": \"px.mesh_plot.product.selection\", \"artifact_kind\": \"png\", \"path\": \"metal/selected.png\", \"axes_present\": true, \"selection_annotation\": true, \"changed_pixels_from_plain\": {}}},\n    {{\"id\": \"wgpu-plain\", \"renderer\": \"wgpu-headless\", \"state\": \"plain\", \"comparison_id\": \"px.mesh_plot.product.axes\", \"artifact_kind\": \"png\", \"path\": \"wgpu/plain.png\", \"axes_present\": true, \"selection_annotation\": false}},\n    {{\"id\": \"wgpu-selected\", \"renderer\": \"wgpu-headless\", \"state\": \"selected\", \"comparison_id\": \"px.mesh_plot.product.selection\", \"artifact_kind\": \"png\", \"path\": \"wgpu/selected.png\", \"axes_present\": true, \"selection_annotation\": true, \"changed_pixels_from_plain\": {}}}\n  ]{}\n}}\n",
        WIDTH as u32,
        HEIGHT as u32,
        width,
        height,
        metal_wgpu_axes_changed,
        changed_fraction(metal_plain, wgpu_plain),
        metal_wgpu_selection_changed,
        changed_fraction(metal_selected, wgpu_selected),
        metal_changed,
        wgpu_changed,
        source_provenance_json(),
    );
    fs::write(output_dir.join("manifest.json"), manifest).map_err(|error| error.to_string())
}

fn assert_rendered(image: &RgbaImage) {
    assert_eq!(image.width(), (WIDTH as u32) * 2);
    assert_eq!(image.height(), (HEIGHT as u32) * 2);
    let (min, max) = image
        .pixels()
        .map(|pixel| u16::from(pixel.0[0]) + u16::from(pixel.0[1]) + u16::from(pixel.0[2]))
        .fold((u16::MAX, 0), |(min, max), value| {
            (min.min(value), max.max(value))
        });
    assert!(max > min, "product MeshPlot framebuffer should not be flat");
}

fn assert_scalar_fill_present(image: &RgbaImage) {
    let center = image.get_pixel(image.width() / 2, image.height() / 2).0;
    assert!(
        center[0] < 240 || center[1] < 240 || center[2] < 240,
        "product MeshPlot scalar fill is missing at the plot center: {center:?}"
    );
}

fn non_black_pixels(
    image: &RgbaImage,
    x_range: std::ops::Range<u32>,
    y_range: std::ops::Range<u32>,
) -> usize {
    x_range
        .flat_map(|x| y_range.clone().map(move |y| image.get_pixel(x, y).0))
        .filter(|pixel| pixel[3] > 0 && pixel[0].max(pixel[1]).max(pixel[2]) > 0)
        .count()
}

fn assert_product_text_is_rasterized(image: &RgbaImage) {
    // The fixture's title is above the plot and its vertical axis title is to
    // the left of the plot. These regions exclude scalar pixels and tick
    // marks, so they specifically catch a missing WGPU glyph-atlas flush.
    let title_pixels = non_black_pixels(image, 300..900, 0..47);
    let y_axis_title_pixels = non_black_pixels(image, 0..150, 47..740);
    assert!(
        title_pixels > 50,
        "product MeshPlot title was not rasterized: {title_pixels} non-black pixels"
    );
    assert!(
        y_axis_title_pixels > 50,
        "product MeshPlot y-axis title was not rasterized: {y_axis_title_pixels} non-black pixels"
    );
}

fn assert_opaque_background(image: &RgbaImage) {
    assert_eq!(
        image.get_pixel(0, 0).0[3],
        255,
        "opaque product capture must clear its background with alpha 255"
    );
}

#[test]
fn product_mesh_plot_axes_and_selection_render_through_metal_and_wgpu() {
    let output_dir = product_capture_dir();
    let Some(metal_plain) = capture(Backend::Metal, false) else {
        if let Some(output_dir) = output_dir.as_deref() {
            write_product_skip(output_dir, "no usable Metal adapter")
                .expect("write product skip manifest");
        }
        return;
    };
    let Some(metal_selected) = capture(Backend::Metal, true) else {
        if let Some(output_dir) = output_dir.as_deref() {
            write_product_skip(output_dir, "no usable Metal adapter")
                .expect("write product skip manifest");
        }
        return;
    };
    let Some(wgpu_plain) = capture(Backend::Wgpu, false) else {
        if let Some(output_dir) = output_dir.as_deref() {
            write_product_skip(output_dir, "no usable WGPU adapter")
                .expect("write product skip manifest");
        }
        return;
    };
    let Some(wgpu_selected) = capture(Backend::Wgpu, true) else {
        if let Some(output_dir) = output_dir.as_deref() {
            write_product_skip(output_dir, "no usable WGPU adapter")
                .expect("write product skip manifest");
        }
        return;
    };

    for image in [&metal_plain, &metal_selected, &wgpu_plain, &wgpu_selected] {
        assert_rendered(image);
    }
    assert_scalar_fill_present(&metal_plain);
    assert_scalar_fill_present(&wgpu_plain);
    for image in [&metal_plain, &metal_selected, &wgpu_plain, &wgpu_selected] {
        assert_product_text_is_rasterized(image);
        assert_opaque_background(image);
    }
    let (mesh, field) = fixture();
    let svg = product_plot(mesh, field, true, MeshPlotBackend::Auto)
        .to_svg()
        .expect("product MeshPlot SVG should build");
    assert!(svg.contains("class=\"gpui-px-mesh-axis\""));
    assert!(svg.contains("class=\"gpui-px-mesh-axis-title\""));
    assert!(svg.contains("data-selected=\"true\""));
    assert!(changed_pixels(&metal_plain, &metal_selected) > 0);
    assert!(changed_pixels(&wgpu_plain, &wgpu_selected) > 0);
    // The adapter pair is intentionally not required to be pixel-identical:
    // GPUI text atlas rasterization is backend-specific. Both screenshots
    // must nevertheless contain the same high-level product composition.
    assert!(changed_pixels(&metal_selected, &wgpu_selected) > 0);

    // Backend agreement alone cannot detect a shared colormap defect.
    mesh_gpu_colormaps_match_cpu_palettes();
    flat_cell_fill_preserves_palette_colors();
    partially_offscreen_metal_mesh_preserves_projection();

    if let Some(output_dir) = output_dir.as_deref() {
        write_product_capture(
            output_dir,
            &metal_plain,
            &metal_selected,
            &wgpu_plain,
            &wgpu_selected,
        )
        .expect("write product MeshPlot capture artifacts");
    }
}

/// Compare actual GPU pixels with the CPU palette, independently of image baselines.
#[test]
fn mesh_gpu_colormaps_match_cpu_palettes() {
    use d3rs::mesh::gpu::{
        GeometryRevision, MeshColorConfig, MeshSceneState, render_offscreen_wgpu,
    };
    use d3rs::mesh::{MeshTopology, prepare_upload};
    use gpui_px::ColorScale;

    let (mut mesh, _) = fixture();
    mesh.positions = Arc::from([
        [-1.0, -1.0, 0.0],
        [1.0, -1.0, 0.0],
        [1.0, 1.0, 0.0],
        [-1.0, 1.0, 0.0],
    ]);
    let topology = MeshTopology::build(&mesh.triangles);
    for scale in [ColorScale::Viridis, ColorScale::Plasma, ColorScale::Inferno] {
        for value in [1.0_f32, 0.0, 0.25, 0.5, 0.75] {
            let mut upload = prepare_upload(&mesh, &topology);
            upload.values_f32 = Some(vec![value; mesh.positions.len()]);
            let scene = MeshSceneState {
                geometry_rev: GeometryRevision(1),
                upload: Some(upload),
                color: MeshColorConfig {
                    colormap: scale.to_colormap_index(),
                    range: [0.0, 1.0],
                    wireframe: false,
                    unlit: true,
                    ..MeshColorConfig::default()
                },
                ..MeshSceneState::default()
            };
            let image = render_offscreen_wgpu(&scene, 64, 64)
                .expect("native QA requires an adapter and a valid mesh shader");
            let pixel = image.get_pixel(32, 32).0;
            assert_eq!(pixel[3], 255, "palette probe must hit the opaque mesh");
            let expected = scale.map(f64::from(value));
            for (channel, reference) in [expected.r, expected.g, expected.b].into_iter().enumerate()
            {
                // RGBA8 conversion and raster interpolation can round by one level.
                let error = (f32::from(pixel[channel]) - reference * 255.0).abs();
                assert!(
                    error <= 1.0,
                    "palette {} at {value}, channel {channel}: pixel {pixel:?}, expected {expected:?}",
                    scale.to_colormap_index(),
                );
            }
        }
    }
}

/// Flat cells retain their palette colors when isolines are disabled.
#[test]
fn flat_cell_fill_preserves_palette_colors() {
    for backend in [Backend::Metal, Backend::Wgpu] {
        let (_, mut field) = fixture();
        field.association = d3rs::mesh::ScalarAssociation::Cell;
        field.values = Arc::from([0.25, 1.25]);
        let image = capture_with_field(backend, false, Some(field))
            .expect("native QA requires the Metal and WGPU adapters");
        // Each triangle occupies far more than 2% of the frame. Requiring
        // this area avoids mistaking a few colorbar pixels for a filled cell.
        let minimum = (u64::from(image.width()) * u64::from(image.height())) / 50;
        for value in [0.0, 1.0] {
            let color = gpui_px::ColorScale::Plasma.map(value);
            let reference = [color.r, color.g, color.b];
            let matching = image
                .pixels()
                .filter(|pixel| {
                    pixel.0[3] == 255
                        && reference.iter().enumerate().all(|(channel, expected)| {
                            (f32::from(pixel.0[channel]) - expected * 255.0).abs() <= 1.0
                        })
                })
                .count();
            assert!(
                u64::try_from(matching).expect("pixel count fits u64") > minimum,
                "flat-cell endpoint {value} has only {matching} palette pixels; expected more than {minimum}"
            );
        }
    }
}

#[test]
fn partially_offscreen_metal_mesh_preserves_projection() {
    let full =
        capture(Backend::Metal, true).expect("Metal adapter required for clipping regression");
    let clipped = capture_with_viewport(Backend::Metal, true, None, WIDTH * 0.5)
        .expect("Metal adapter required for clipping regression");
    assert_eq!(full.height(), clipped.height());
    assert_eq!(full.width(), clipped.width() * 2);
    // The fixed-size chart must be cropped, including its selection overlay,
    // without changing the projection of the visible portion of the mesh.
    let differing = clipped
        .enumerate_pixels()
        .filter(|(x, y, pixel)| {
            let reference = full.get_pixel(*x, *y);
            pixel
                .0
                .iter()
                .zip(reference.0)
                .any(|(actual, expected)| actual.abs_diff(expected) > 1)
        })
        .count();
    let pixel_count = (clipped.width() * clipped.height()) as usize;
    assert!(
        differing < pixel_count / 1000,
        "clipping changed {differing} of {pixel_count} visible pixels"
    );
}
