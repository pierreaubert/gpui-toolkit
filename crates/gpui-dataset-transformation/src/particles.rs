//! Time-dependent particle advection (pathlines) through a [`TimeSeries`].
//!
//! Seeds are integrated with the true velocity (midpoint RK2, physical
//! time steps), sampling the vector field of the two bracketing time steps
//! and blending in time. Each output frame holds a trail of the most
//! recent positions per particle as line cells, a vertex cell at the head,
//! and an `age` scalar field (0 at the head, 1 at the trail tail) so a
//! transfer function can fade old positions.

// Rust guideline compliant 2026-02-21

use crate::TransformError;
use crate::streamline::Tracer;
use gpui_mesh::{Association, CellType, Cells, DataSet, Field, TimeAxis, TimeSeries};

/// Options for [`particle_frames`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleOptions {
    /// Physical time step between frames (and per integration step).
    pub dt: f64,
    /// Number of frames to emit, starting at the series' first time.
    pub frames: usize,
    /// Maximum trail points kept per particle, including the head.
    pub trail: usize,
}

struct ParticleCells {
    types: Vec<CellType>,
    offsets: Vec<u64>,
    connectivity: Vec<u32>,
}

impl ParticleCells {
    fn new() -> Self {
        Self {
            types: Vec::new(),
            offsets: vec![0],
            connectivity: Vec::new(),
        }
    }
    fn push(&mut self, kind: CellType, nodes: &[u32]) {
        self.types.push(kind);
        self.connectivity.extend_from_slice(nodes);
        self.offsets.push(self.connectivity.len() as u64);
    }
    fn finish(self) -> Cells {
        Cells {
            types: self.types.into(),
            offsets: self.offsets.into(),
            connectivity: self.connectivity.into(),
        }
    }
}

impl Default for ParticleOptions {
    fn default() -> Self {
        // 120 frames at 24 fps is the 5-second movie format used by the
        // demos; trails of 24 points read as comet tails at that rate.
        Self {
            dt: 1.0 / 24.0,
            frames: 120,
            trail: 24,
        }
    }
}

/// Space-time velocity sampler over a series' steps.
///
/// Spatial sampling is barycentric inside each step's tetrahedralisation;
/// temporal sampling blends the two bracketing steps.
pub(crate) struct TimeSampler<'a> {
    axis: TimeAxis<'a>,
    tracers: Vec<Tracer<'a>>,
}

impl<'a> TimeSampler<'a> {
    pub(crate) fn new(series: &'a TimeSeries, field_name: &str) -> Result<Self, TransformError> {
        series.validate()?;
        let mut tracers = Vec::with_capacity(series.len());
        let mut units = None::<&Option<std::sync::Arc<str>>>;
        for step in &series.steps {
            let field = step
                .field(Association::Point, field_name)
                .ok_or_else(|| TransformError::FieldNotFound(field_name.to_string()))?;
            let vectors = field
                .as_vector()
                .ok_or_else(|| TransformError::WrongFieldShape {
                    field: field_name.to_string(),
                    reason: "time sampling requires a 3-vector field".into(),
                })?;
            if units.is_some_and(|expected| expected != &field.unit) {
                return Err(TransformError::WrongFieldShape {
                    field: field_name.into(),
                    reason: "time sampling requires consistent velocity units".into(),
                });
            }
            units = Some(&field.unit);
            let tracer = Tracer::new(step, vectors, field.valid.as_deref());
            if !tracer.has_volume() {
                return Err(TransformError::Degenerate(
                    "time step holds no volume cells".into(),
                ));
            }
            tracers.push(tracer);
        }
        Ok(Self {
            axis: series.time_axis()?,
            tracers,
        })
    }

    pub(crate) fn velocity_at(&self, p: [f64; 3], time: f64) -> Option<[f64; 3]> {
        if !time.is_finite() {
            return None;
        }
        let (lo, hi, frac) = self.axis.locate(time).ok()?;
        if frac == 1.0 {
            return self.tracers[hi].velocity(p);
        }
        let v_lo = self.tracers[lo].velocity(p)?;
        if lo == hi || frac == 0.0 {
            return Some(v_lo);
        }
        let v_hi = self.tracers[hi].velocity(p)?;
        Some([
            crate::convert::blend(v_lo[0], v_hi[0], frac),
            crate::convert::blend(v_lo[1], v_hi[1], frac),
            crate::convert::blend(v_lo[2], v_hi[2], frac),
        ])
    }

    /// Midpoint-RK2 step of physical size `dt` at `time`; `None` outside.
    pub(crate) fn rk2(&self, p: [f64; 3], time: f64, dt: f64) -> Option<[f64; 3]> {
        if !dt.is_finite() {
            return None;
        }
        let v = self.velocity_at(p, time)?;
        let mid = [
            (0.5 * dt).mul_add(v[0], p[0]),
            (0.5 * dt).mul_add(v[1], p[1]),
            (0.5 * dt).mul_add(v[2], p[2]),
        ];
        let vm = self.velocity_at(mid, time + 0.5 * dt)?;
        let next = [
            dt.mul_add(vm[0], p[0]),
            dt.mul_add(vm[1], p[1]),
            dt.mul_add(vm[2], p[2]),
        ];
        self.velocity_at(next, time + dt)?;
        Some(next)
    }
}

/// Advects seed particles through a time series.
///
/// Particles that leave the volume freeze in place and stop growing their
/// trail. Times beyond the series end sample the last step (the series
/// clamps), so `frames * dt` may exceed the series span for hold-end
/// playback.
/// Missing spatial/temporal support freezes an existing trail. Seeds without
/// initial support are omitted. New positions require supported RK2 endpoints.
///
/// # Errors
///
/// Returns an error when the field is missing or not a vector field in any
/// step, when a step holds no volume cells, input/seeds are invalid, options or
/// frame times are nonfinite, or output point indices exceed the representation.
pub fn particle_frames(
    series: &TimeSeries,
    field_name: &str,
    seeds: &[[f64; 3]],
    options: ParticleOptions,
) -> Result<Vec<DataSet>, TransformError> {
    if !options.dt.is_finite() || options.dt <= 0.0 || options.frames == 0 || options.trail == 0 {
        return Err(TransformError::Degenerate(
            "particle options require dt > 0, frames > 0, trail > 0".into(),
        ));
    }
    crate::streamline::validate_seeds(seeds)?;
    let sampler = TimeSampler::new(series, field_name)?;

    let t0 = series.times[0];
    if !options
        .dt
        .mul_add((options.frames - 1) as f64, t0)
        .is_finite()
    {
        return Err(TransformError::Degenerate(
            "particle frame time is not representable".into(),
        ));
    }
    let mut alive: Vec<bool> = seeds
        .iter()
        .map(|&seed| sampler.velocity_at(seed, t0).is_some())
        .collect();
    let mut histories: Vec<Vec<[f64; 3]>> = seeds
        .iter()
        .zip(&alive)
        .map(|(s, alive)| if *alive { vec![*s] } else { Vec::new() })
        .collect();
    let mut frames = Vec::with_capacity(options.frames);
    for frame in 0..options.frames {
        let start_time = options.dt.mul_add(frame.saturating_sub(1) as f64, t0);
        if frame > 0 {
            for (history, is_alive) in histories.iter_mut().zip(alive.iter_mut()) {
                if !*is_alive {
                    continue;
                }
                let p = *history.last().unwrap_or(&[0.0; 3]);
                let Some(next) = sampler.rk2(p, start_time, options.dt) else {
                    *is_alive = false;
                    continue;
                };
                history.push(next);
                if history.len() > options.trail {
                    let excess = history.len() - options.trail;
                    history.drain(..excess);
                }
            }
        }
        frames.push(frame_dataset(series, frame, &histories)?);
    }
    Ok(frames)
}

fn frame_dataset(
    series: &TimeSeries,
    frame: usize,
    histories: &[Vec<[f64; 3]>],
) -> Result<DataSet, TransformError> {
    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut ages: Vec<f64> = Vec::new();
    let mut cells = ParticleCells::new();
    for history in histories {
        if history.is_empty() {
            continue;
        }
        let base = crate::geometry::output_base(positions.len(), history.len())?;
        let n = history.len();
        for (i, point) in history.iter().enumerate() {
            positions.push(*point);
            // Head (last point) has age 0, the oldest kept point age 1.
            ages.push(if n == 1 {
                0.0
            } else {
                1.0 - i as f64 / (n - 1) as f64
            });
        }
        for i in 0..n - 1 {
            cells.push(CellType::Line, &[base + i as u32, base + i as u32 + 1]);
        }
        cells.push(CellType::Vertex, &[base + (n - 1) as u32]);
    }
    Ok(DataSet::new(
        format!("{}-particles-{frame}", series.steps[0].id),
        positions,
        cells.finish(),
    )
    .with_field(Field::scalar("age", ages)))
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn uniform_series(frames: usize) -> TimeSeries {
        let mut steps = Vec::new();
        for _ in 0..frames {
            let ds = DataSet::structured_grid("g", [4, 4, 4], [0.0; 3], [1.0, 1.0, 1.0]).unwrap();
            let count = ds.point_count();
            steps.push(ds.with_field(Field::vector("v", vec![[1.0, 0.0, 0.0]; count])));
        }
        let times: Vec<f64> = (0..frames).map(|i| i as f64).collect();
        TimeSeries::new(steps, times).unwrap()
    }

    #[test]
    fn uniform_flow_moves_particles_at_unit_speed() {
        let series = uniform_series(2);
        let frames = particle_frames(
            &series,
            "v",
            &[[0.5, 1.5, 1.5]],
            ParticleOptions {
                dt: 0.5,
                frames: 3,
                trail: 8,
            },
        )
        .unwrap();
        assert_eq!(frames.len(), 3);
        // After two steps of dt=0.5 at speed 1, the head is at x = 1.5.
        let last = frames[2].positions.last().copied().unwrap();
        assert!((last[0] - 1.5).abs() < 1e-9, "head at {last:?}");
        let age = frames[2]
            .field(Association::Point, "age")
            .unwrap()
            .as_scalar()
            .unwrap();
        assert_eq!(age[0], 1.0);
        assert_eq!(*age.last().unwrap(), 0.0);
        assert_eq!(frames[2].validate(), Ok(()));
    }

    #[test]
    fn rotation_keeps_particles_on_their_circle() {
        // v = (-y, x, 0) around (1.5, 1.5): a particle at radius 1 keeps
        // its radius under RK2 to integration accuracy.
        let mut steps = Vec::new();
        for _ in 0..2 {
            let ds = DataSet::structured_grid("g", [7, 7, 3], [0.0; 3], [0.5, 0.5, 0.5]).unwrap();
            let vectors: Vec<[f64; 3]> = ds
                .positions
                .iter()
                .map(|p| [-(p[1] - 1.5), p[0] - 1.5, 0.0])
                .collect();
            steps.push(ds.with_field(Field::vector("v", vectors)));
        }
        let series = TimeSeries::new(steps, vec![0.0, 10.0]).unwrap();
        let frames = particle_frames(
            &series,
            "v",
            &[[2.5, 1.5, 0.5]],
            ParticleOptions {
                dt: 0.05,
                frames: 21,
                trail: 4,
            },
        )
        .unwrap();
        let head = frames[20].positions.last().copied().unwrap();
        let radius = ((head[0] - 1.5).powi(2) + (head[1] - 1.5).powi(2)).sqrt();
        assert!((radius - 1.0).abs() < 0.02, "radius {radius}");
        // Trail is capped at 4 points.
        assert_eq!(frames[20].point_count(), 4);
    }

    #[test]
    fn bad_options_and_missing_field_are_errors() {
        let series = uniform_series(1);
        assert!(
            particle_frames(
                &series,
                "v",
                &[[0.0; 3]],
                ParticleOptions {
                    dt: 0.0,
                    frames: 1,
                    trail: 1
                }
            )
            .unwrap_err()
            .to_string()
            .contains("dt > 0")
        );
        assert!(
            particle_frames(&series, "nope", &[[0.0; 3]], ParticleOptions::default())
                .unwrap_err()
                .to_string()
                .contains("nope")
        );
    }
}

/// Options for [`streak_frames`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StreakOptions {
    /// Physical time step between frames.
    pub dt: f64,
    /// Number of frames to emit.
    pub frames: usize,
    /// Release a new particle at each seed every this many frames.
    pub release_every: usize,
}

/// Computes streaklines: particles released continuously from each seed.
///
/// At every frame, the particles released from one seed form a line
/// ordered by release time — the dye line of the classic streakline
/// visualisation. An `age` scalar (seconds since release) is attached.
/// Newly released particles stay at their seed until the next frame. Missing
/// space/time support removes a particle; unsupported releases are omitted.
///
/// # Errors
///
/// Returns an error when the field is missing, a step has no volume
/// cells, input/seeds/options are invalid, frame times are nonfinite, or output
/// point indices exceed the model representation.
pub fn streak_frames(
    series: &TimeSeries,
    field_name: &str,
    seeds: &[[f64; 3]],
    options: StreakOptions,
) -> Result<Vec<DataSet>, TransformError> {
    if !options.dt.is_finite()
        || options.dt <= 0.0
        || options.frames == 0
        || options.release_every == 0
    {
        return Err(TransformError::Degenerate(
            "streak options require dt > 0, frames > 0, release_every > 0".into(),
        ));
    }
    crate::streamline::validate_seeds(seeds)?;
    let sampler = TimeSampler::new(series, field_name)?;
    let t0 = series.times[0];
    if !options
        .dt
        .mul_add((options.frames - 1) as f64, t0)
        .is_finite()
    {
        return Err(TransformError::Degenerate(
            "streak frame time is not representable".into(),
        ));
    }
    // (seed index, release frame, position, alive)
    let mut particles: Vec<(usize, usize, [f64; 3], bool)> = Vec::new();
    let mut frames = Vec::with_capacity(options.frames);
    for frame in 0..options.frames {
        let time = options.dt.mul_add(frame as f64, t0);
        let start_time = options.dt.mul_add(frame.saturating_sub(1) as f64, t0);
        if frame % options.release_every == 0 {
            for (seed, &position) in seeds.iter().enumerate() {
                if sampler.velocity_at(position, time).is_some() {
                    particles.push((seed, frame, position, true));
                }
            }
        }
        if frame > 0 {
            for particle in &mut particles {
                if !particle.3 || particle.1 == frame {
                    continue;
                }
                match sampler.rk2(particle.2, start_time, options.dt) {
                    Some(next) => {
                        particle.2 = next;
                    }
                    _ => particle.3 = false,
                }
            }
        }
        particles.retain(|particle| particle.3);
        // Build one polyline per seed, oldest release first.
        let mut positions: Vec<[f64; 3]> = Vec::new();
        let mut ages: Vec<f64> = Vec::new();
        let mut cells = ParticleCells::new();
        for seed in 0..seeds.len() {
            let line = particles
                .iter()
                .filter(|p| p.0 == seed && p.3)
                .map(|p| (p.1, p.2));
            for (i, (release, position)) in line.enumerate() {
                let id = crate::geometry::output_base(positions.len(), 1)?;
                positions.push(position);
                ages.push((frame - release) as f64 * options.dt);
                if i > 0 {
                    cells.push(CellType::Line, &[id - 1, id]);
                }
            }
        }
        let mut age = Field::scalar("age", ages);
        age.unit = Some("s".into());
        let age = crate::validity::finish(age, |_| true);
        frames.push(
            DataSet::new(
                format!("{}-streak-{frame}", series.steps[0].id),
                positions,
                cells.finish(),
            )
            .with_field(age),
        );
    }
    Ok(frames)
}

#[cfg(test)]
mod streak_tests {
    use super::*;

    #[test]
    fn streakline_in_uniform_flow_is_a_growing_line() {
        let series = super::tests::uniform_series(2);
        let frames = streak_frames(
            &series,
            "v",
            &[[0.2, 1.5, 1.5]],
            StreakOptions {
                dt: 0.25,
                frames: 5,
                release_every: 1,
            },
        )
        .unwrap();
        let last = &frames[4];
        // Five releases, all still inside the [0,3]^3 domain.
        assert_eq!(last.point_count(), 5);
        assert_eq!(last.cell_count(), 4);
        // Oldest particle travelled 4 * 0.25 * 1 = 1.0 in +x.
        let oldest_x = last.positions.iter().map(|p| p[0]).fold(0.0, f64::max);
        assert!((oldest_x - 1.2).abs() < 1e-9, "oldest at {oldest_x}");
    }
}
