use d3rs::render2d::{Renderer2D, VelloBackend};
use gpui::prelude::*;
use gpui::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use super::potentiometer_size::PotentiometerSize;
use super::types::PotentiometerScale;

/// Cached geometry for a potentiometer's tick ring.
pub(super) struct PotentiometerTickGeometry {
    /// Major/minor tick line endpoints.
    pub ticks: Arc<[PotentiometerTickLine]>,
    /// Label text and anchor positions for major ticks.
    pub labels: Arc<[(SharedString, f32, f32)]>,
    pub major_tick_width: f32,
    pub minor_tick_width: f32,
    pub knob_offset_x: f32,
    pub knob_offset_y: f32,
}

pub(super) struct PotentiometerTickLine {
    pub is_major: bool,
    pub inner_x: f32,
    pub inner_y: f32,
    pub outer_x: f32,
    pub outer_y: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct GeometryCacheKey {
    min: i64,
    max: i64,
    scale: PotentiometerScale,
    size: PotentiometerSize,
    unit_hash: u64,
    start_deg: i32,
    sweep_deg: i32,
}

#[cfg(feature = "vello")]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct TickSceneCacheKey {
    ticks_hash: u64,
    major_tick_width: u32,
    minor_tick_width: u32,
    major_color: [u32; 4],
    minor_color: [u32; 4],
}

#[cfg(feature = "vello")]
impl TickSceneCacheKey {
    fn new(element: &PotentiometerTickLinesElement) -> Self {
        let rgba_bits = |color: Rgba| {
            [
                color.r.to_bits(),
                color.g.to_bits(),
                color.b.to_bits(),
                color.a.to_bits(),
            ]
        };
        Self {
            ticks_hash: tick_content_hash(&element.ticks),
            major_tick_width: element.major_tick_width.to_bits(),
            minor_tick_width: element.minor_tick_width.to_bits(),
            major_color: rgba_bits(element.major_tick_color),
            minor_color: rgba_bits(element.minor_tick_color),
        }
    }
}

#[cfg(feature = "vello")]
fn tick_content_hash(ticks: &[PotentiometerTickLine]) -> u64 {
    let mut hash = fxhash::hash64(&[]);
    for tick in ticks {
        hash = fxhash::hash64_with_seed(hash, &[u8::from(tick.is_major)]);
        hash = fxhash::hash64_with_seed(hash, &tick.inner_x.to_bits().to_ne_bytes());
        hash = fxhash::hash64_with_seed(hash, &tick.inner_y.to_bits().to_ne_bytes());
        hash = fxhash::hash64_with_seed(hash, &tick.outer_x.to_bits().to_ne_bytes());
        hash = fxhash::hash64_with_seed(hash, &tick.outer_y.to_bits().to_ne_bytes());
    }
    hash
}

impl GeometryCacheKey {
    fn new(
        min: f64,
        max: f64,
        scale: PotentiometerScale,
        size: PotentiometerSize,
        unit: &str,
        start_deg: f32,
        sweep_deg: f32,
    ) -> Self {
        Self {
            min: (min * 1_000_000.0).round() as i64,
            max: (max * 1_000_000.0).round() as i64,
            scale,
            size,
            unit_hash: fxhash::hash64(unit.as_bytes()),
            start_deg: (start_deg * 1_000.0).round() as i32,
            sweep_deg: (sweep_deg * 1_000.0).round() as i32,
        }
    }
}

thread_local! {
    static GEOMETRY_CACHE: RefCell<HashMap<GeometryCacheKey, Arc<PotentiometerTickGeometry>>> =
        RefCell::new(HashMap::new());
    #[cfg(feature = "vello")]
    static TICK_SCENE_CACHE: RefCell<HashMap<TickSceneCacheKey, Arc<d3rs::vello2d::ChartScene>>> =
        RefCell::new(HashMap::new());
}

const GEOMETRY_CACHE_CAPACITY: usize = 64;
#[cfg(feature = "vello")]
const TICK_SCENE_CACHE_CAPACITY: usize = 64;

#[allow(clippy::too_many_arguments)]
/// Compute or retrieve cached tick geometry for the given parameters.
pub(super) fn get_tick_geometry(
    min: f64,
    max: f64,
    scale: PotentiometerScale,
    size: PotentiometerSize,
    unit: &SharedString,
    start_deg: f32,
    sweep_deg: f32,
    knob_size: f32,
    center: f32,
    horizontal_label_gutter: f32,
    vertical_label_gutter: f32,
    container_width: f32,
    container_height: f32,
    major_tick_outer_radius: f32,
    minor_tick_outer_radius: f32,
    tick_inner_radius: f32,
    label_radius: f32,
) -> Arc<PotentiometerTickGeometry> {
    let key = GeometryCacheKey::new(min, max, scale, size, unit, start_deg, sweep_deg);
    GEOMETRY_CACHE.with(|cache| {
        if let Some(geometry) = cache.borrow().get(&key) {
            return geometry.clone();
        }

        let geometry = Arc::new(build_tick_geometry(
            min,
            max,
            scale,
            size,
            unit,
            start_deg,
            sweep_deg,
            knob_size,
            center,
            horizontal_label_gutter,
            vertical_label_gutter,
            container_width,
            container_height,
            major_tick_outer_radius,
            minor_tick_outer_radius,
            tick_inner_radius,
            label_radius,
        ));
        let mut cache = cache.borrow_mut();
        if cache.len() >= GEOMETRY_CACHE_CAPACITY
            && let Some(evicted_key) = cache.keys().next().copied()
        {
            cache.remove(&evicted_key);
        }
        cache.insert(key, geometry.clone());
        geometry
    })
}

#[allow(clippy::too_many_arguments)]
fn build_tick_geometry(
    min: f64,
    max: f64,
    scale: PotentiometerScale,
    size: PotentiometerSize,
    unit: &SharedString,
    start_deg: f32,
    sweep_deg: f32,
    _knob_size: f32,
    center: f32,
    horizontal_label_gutter: f32,
    vertical_label_gutter: f32,
    _container_width: f32,
    _container_height: f32,
    major_tick_outer_radius: f32,
    minor_tick_outer_radius: f32,
    tick_inner_radius: f32,
    label_radius: f32,
) -> PotentiometerTickGeometry {
    let range = max - min;
    let is_large = matches!(size, PotentiometerSize::Lg);
    let divisors: &[i32] = if is_large { &[10, 5, 3, 2] } else { &[5, 3, 2] };

    let mut num_major_ticks = if is_large { 10 } else { 4 };
    for &div in divisors {
        if max.abs() < 0.0001 {
            continue;
        }
        let tick_interval = max / div as f64;
        if tick_interval.abs() < 0.0001 {
            continue;
        }
        let min_remainder = if min.abs() < 0.0001 {
            0.0
        } else {
            (min / tick_interval).fract().abs()
        };
        let min_aligned = min_remainder < 0.01 || (1.0 - min_remainder) < 0.01;
        if min_aligned {
            let tick_count = (range / tick_interval).round() as i32;
            if tick_count >= 2 && tick_count <= (if is_large { 10 } else { 6 }) {
                num_major_ticks = tick_count;
                break;
            }
        }
    }

    let minor_ticks_between = 4;
    let total_ticks = num_major_ticks * (minor_ticks_between + 1);
    let start_rad = start_deg.to_radians();
    let end_rad = (start_deg + sweep_deg).to_radians();
    let knob_offset_x = horizontal_label_gutter;
    let knob_offset_y = vertical_label_gutter;
    let major_tick_width = 3.0;
    let minor_tick_width = 1.5;

    let mut ticks = Vec::with_capacity(total_ticks as usize + 1);
    let mut labels = Vec::new();

    for i in 0..=total_ticks {
        let tick_normalized = i as f32 / total_ticks as f32;
        let tick_angle = start_rad + (end_rad - start_rad) * tick_normalized;
        let is_major = i % (minor_ticks_between + 1) == 0;

        let tick_outer_radius = if is_major {
            major_tick_outer_radius
        } else {
            minor_tick_outer_radius
        };

        let inner_x = knob_offset_x + center + tick_inner_radius * tick_angle.cos();
        let inner_y = knob_offset_y + center + tick_inner_radius * tick_angle.sin();
        let outer_x = knob_offset_x + center + tick_outer_radius * tick_angle.cos();
        let outer_y = knob_offset_y + center + tick_outer_radius * tick_angle.sin();

        ticks.push(PotentiometerTickLine {
            is_major,
            inner_x,
            inner_y,
            outer_x,
            outer_y,
        });

        if is_major {
            let tick_value = scale.normalized_to_value(tick_normalized as f64, min, max);
            let label_x = knob_offset_x + center + label_radius * tick_angle.cos();
            let label_y = knob_offset_y + center + label_radius * tick_angle.sin();

            let unit = unit.as_ref();
            let label_text: SharedString = if unit == "%" {
                format!("{:.0}", tick_normalized * 100.0).into()
            } else if unit == "Hz" {
                if tick_value >= 1000.0 {
                    format!("{:.0}k", tick_value / 1000.0).into()
                } else {
                    format!("{:.0}", tick_value).into()
                }
            } else if unit == "dB" {
                format!("{:.0}", tick_value).into()
            } else {
                format!("{:.1}", tick_value).into()
            };

            labels.push((label_text, label_x, label_y));
        }
    }

    PotentiometerTickGeometry {
        ticks: ticks.into(),
        labels: labels.into(),
        major_tick_width,
        minor_tick_width,
        knob_offset_x,
        knob_offset_y,
    }
}

/// Custom element that paints all potentiometer tick lines in two batched paths.
pub(super) struct PotentiometerTickLinesElement {
    pub id: ElementId,
    pub container_width: f32,
    pub container_height: f32,
    pub major_tick_color: Rgba,
    pub minor_tick_color: Rgba,
    pub ticks: Arc<[PotentiometerTickLine]>,
    pub major_tick_width: f32,
    pub minor_tick_width: f32,
    #[cfg_attr(not(feature = "vello"), allow(dead_code))]
    pub renderer_2d: Renderer2D,
    #[cfg_attr(not(feature = "vello"), allow(dead_code))]
    pub vello_backend: VelloBackend,
    #[cfg(feature = "vello")]
    pub painter: d3rs::vello2d::VelloScenePainter,
}

impl IntoElement for PotentiometerTickLinesElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for PotentiometerTickLinesElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let layout_id = window.request_layout(
            Style {
                size: Size {
                    width: px(self.container_width).into(),
                    height: px(self.container_height).into(),
                },
                ..Default::default()
            },
            [],
            cx,
        );
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        let origin_x: f32 = bounds.origin.x.into();
        let origin_y: f32 = bounds.origin.y.into();

        let mut major_path = PathBuilder::fill();
        let mut minor_path = PathBuilder::fill();
        let mut has_major = false;
        let mut has_minor = false;

        #[cfg(feature = "vello")]
        if self.renderer_2d == Renderer2D::Vello {
            use d3rs::vello2d::kurbo::{BezPath, PathEl};
            use d3rs::vello2d::peniko::{Brush, Color};
            let scene_key = TickSceneCacheKey::new(self);
            if let Some(scene) =
                TICK_SCENE_CACHE.with(|cache| cache.borrow().get(&scene_key).cloned())
            {
                self.painter.set_backend(self.vello_backend);
                self.painter
                    .paint_retained(id, scene.as_ref(), bounds, window);
                return;
            }
            let mut scene = d3rs::vello2d::ChartScene::new();
            let mut major = BezPath::new();
            let mut minor = BezPath::new();
            for tick in self.ticks.iter() {
                let width = if tick.is_major {
                    self.major_tick_width
                } else {
                    self.minor_tick_width
                };
                let dx = tick.outer_x - tick.inner_x;
                let dy = tick.outer_y - tick.inner_y;
                let len = (dx * dx + dy * dy).sqrt().max(0.0001);
                let perp_x = -dy / len * width * 0.5;
                let perp_y = dx / len * width * 0.5;
                let points = [
                    (tick.inner_x + perp_x, tick.inner_y + perp_y),
                    (tick.outer_x + perp_x, tick.outer_y + perp_y),
                    (tick.outer_x - perp_x, tick.outer_y - perp_y),
                    (tick.inner_x - perp_x, tick.inner_y - perp_y),
                ];
                let path = if tick.is_major {
                    &mut major
                } else {
                    &mut minor
                };
                for (index, (x, y)) in points.iter().enumerate() {
                    let point = (*x as f64, *y as f64);
                    path.push(if index == 0 {
                        PathEl::MoveTo(point.into())
                    } else {
                        PathEl::LineTo(point.into())
                    });
                }
                path.push(PathEl::ClosePath);
            }
            if !major.is_empty() {
                let c = self.major_tick_color;
                scene.fill_path(major, Brush::Solid(Color::new([c.r, c.g, c.b, c.a])));
            }
            if !minor.is_empty() {
                let c = self.minor_tick_color;
                scene.fill_path(minor, Brush::Solid(Color::new([c.r, c.g, c.b, c.a])));
            }
            let scene = Arc::new(scene);
            TICK_SCENE_CACHE.with(|cache| {
                let mut cache = cache.borrow_mut();
                if cache.len() >= TICK_SCENE_CACHE_CAPACITY
                    && let Some(evicted_key) = cache.keys().next().copied()
                {
                    cache.remove(&evicted_key);
                }
                cache.insert(scene_key, Arc::clone(&scene));
            });
            self.painter.set_backend(self.vello_backend);
            self.painter
                .paint_retained(id, scene.as_ref(), bounds, window);
            return;
        }

        for tick in self.ticks.iter() {
            let (path, width, is_major) = if tick.is_major {
                (&mut major_path, self.major_tick_width, true)
            } else {
                (&mut minor_path, self.minor_tick_width, false)
            };

            let dx = tick.outer_x - tick.inner_x;
            let dy = tick.outer_y - tick.inner_y;
            let len = (dx * dx + dy * dy).sqrt().max(0.0001);
            let perp_x = -dy / len * (width / 2.0);
            let perp_y = dx / len * (width / 2.0);

            let p0 = point(
                px(origin_x + tick.inner_x + perp_x),
                px(origin_y + tick.inner_y + perp_y),
            );
            let p1 = point(
                px(origin_x + tick.outer_x + perp_x),
                px(origin_y + tick.outer_y + perp_y),
            );
            let p2 = point(
                px(origin_x + tick.outer_x - perp_x),
                px(origin_y + tick.outer_y - perp_y),
            );
            let p3 = point(
                px(origin_x + tick.inner_x - perp_x),
                px(origin_y + tick.inner_y - perp_y),
            );

            path.move_to(p0);
            path.line_to(p1);
            path.line_to(p2);
            path.line_to(p3);
            path.line_to(p0);

            if is_major {
                has_major = true;
            } else {
                has_minor = true;
            }
        }

        if has_major && let Ok(path) = major_path.build() {
            window.paint_path(path, self.major_tick_color);
        }
        if has_minor && let Ok(path) = minor_path.build() {
            window.paint_path(path, self.minor_tick_color);
        }
    }
}

// Small fallback hasher so we don't add a new dependency.
mod fxhash {
    pub fn hash64(bytes: &[u8]) -> u64 {
        let hash: u64 = 0xcbf29ce484222325;
        hash64_with_seed(hash, bytes)
    }

    pub fn hash64_with_seed(mut hash: u64, bytes: &[u8]) -> u64 {
        for &b in bytes {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GEOMETRY_CACHE, GEOMETRY_CACHE_CAPACITY, PotentiometerScale, PotentiometerSize,
        get_tick_geometry,
    };
    use gpui::SharedString;

    #[test]
    fn tick_geometry_is_cached() {
        let unit: SharedString = "Hz".into();
        let geom_a = get_tick_geometry(
            20.0,
            20000.0,
            PotentiometerScale::Logarithmic,
            PotentiometerSize::Md,
            &unit,
            135.0,
            270.0,
            60.0,
            30.0,
            44.0,
            22.0,
            148.0,
            104.0,
            38.0,
            35.0,
            30.0,
            46.0,
        );
        let geom_b = get_tick_geometry(
            20.0,
            20000.0,
            PotentiometerScale::Logarithmic,
            PotentiometerSize::Md,
            &unit,
            135.0,
            270.0,
            60.0,
            30.0,
            44.0,
            22.0,
            148.0,
            104.0,
            38.0,
            35.0,
            30.0,
            46.0,
        );
        assert!(std::sync::Arc::ptr_eq(&geom_a, &geom_b));
        assert!(!geom_a.labels.is_empty());
    }

    #[test]
    fn geometry_cache_is_bounded() {
        let unit: SharedString = "Hz".into();
        for index in 0..=(GEOMETRY_CACHE_CAPACITY as u32 + 1) {
            let min = 20.0 + index as f64;
            get_tick_geometry(
                min,
                20_000.0,
                PotentiometerScale::Logarithmic,
                PotentiometerSize::Md,
                &unit,
                135.0,
                270.0,
                60.0,
                30.0,
                44.0,
                22.0,
                148.0,
                104.0,
                38.0,
                35.0,
                30.0,
                46.0,
            );
        }
        GEOMETRY_CACHE.with(|cache| assert!(cache.borrow().len() <= GEOMETRY_CACHE_CAPACITY));
    }

    #[cfg(feature = "vello")]
    #[test]
    fn equivalent_tick_geometry_has_the_same_scene_cache_hash() {
        use super::{PotentiometerTickLine, tick_content_hash};
        use std::sync::Arc;

        let first: Arc<[PotentiometerTickLine]> = Arc::from([PotentiometerTickLine {
            is_major: true,
            inner_x: 1.0,
            inner_y: 2.0,
            outer_x: 3.0,
            outer_y: 4.0,
        }]);
        let second: Arc<[PotentiometerTickLine]> = Arc::from([PotentiometerTickLine {
            is_major: true,
            inner_x: 1.0,
            inner_y: 2.0,
            outer_x: 3.0,
            outer_y: 4.0,
        }]);

        assert_ne!(first.as_ptr(), second.as_ptr());
        assert_eq!(tick_content_hash(&first), tick_content_hash(&second));
    }
}
