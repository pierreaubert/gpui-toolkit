//! Time playback over immutable prepared sampling state.

// Rust guideline compliant 2026-02-21

use crate::SceneError;
use gpui_mesh::{DataSet, PreparedTimeSeries, TimeSeries};

/// Playback driven by host-supplied elapsed time.
///
/// The host owns the clock. Prepared source state and interpolation plans are
/// shared by clones; advancing does not rebuild them.
#[derive(Debug, Clone)]
pub struct Player {
    series: PreparedTimeSeries,
    /// Current finite physical time; sampling clamps it to the series range.
    pub time: f64,
    /// Whether elapsed time advances playback.
    pub playing: bool,
    /// Finite speed multiplier; negative values play backward.
    pub speed: f64,
    /// Whether playback wraps at either endpoint.
    pub looping: bool,
}

impl Player {
    /// Validates a series and creates paused playback at its first step.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid input or incompatible field shapes/units.
    pub fn new(series: TimeSeries) -> Result<Self, SceneError> {
        let series = series.prepare()?;
        let time = series.time_axis().range()[0];
        Ok(Self {
            series,
            time,
            playing: false,
            speed: 1.0,
            looping: true,
        })
    }

    /// Starts or resumes playback.
    pub fn play(&mut self) {
        self.playing = true;
    }

    /// Pauses playback.
    pub fn pause(&mut self) {
        self.playing = false;
    }

    /// Seeks finite physical time and reports whether the time changed.
    ///
    /// # Errors
    ///
    /// Returns an error for a nonfinite query without changing state.
    pub fn seek(&mut self, time: f64) -> Result<bool, SceneError> {
        if !time.is_finite() {
            return Err(SceneError::InvalidPlayback("seek time must be finite"));
        }
        let [first, last] = self.series.time_axis().range();
        let next = time.clamp(first, last);
        let changed = next != self.time;
        self.time = next;
        Ok(changed)
    }

    /// Advances playback and reports whether time or playing state changed.
    ///
    /// Negative elapsed time or speed reverses direction. Non-looping playback
    /// stops at either endpoint. A false result allows hosts to skip redraw.
    ///
    /// # Errors
    ///
    /// Returns an error for nonfinite controls or an unrepresentable looping
    /// increment. Errors preserve state.
    pub fn advance(&mut self, elapsed_seconds: f64) -> Result<bool, SceneError> {
        if !elapsed_seconds.is_finite() || !self.speed.is_finite() || !self.time.is_finite() {
            return Err(SceneError::InvalidPlayback(
                "time, elapsed seconds and speed must be finite",
            ));
        }
        if !self.playing {
            return Ok(false);
        }
        let [first, last] = self.series.time_axis().range();
        let previous = self.time;
        let was_playing = self.playing;
        let base = self.time.clamp(first, last);
        if first == last {
            self.time = first;
            self.playing = false;
            return Ok(previous != first || was_playing);
        }
        if elapsed_seconds == 0.0 || self.speed == 0.0 {
            self.time = base;
            return Ok(previous != base);
        }
        let mut next = elapsed_seconds.mul_add(self.speed, base);
        let forward = elapsed_seconds.is_sign_positive() == self.speed.is_sign_positive();
        if next < first || next > last {
            if self.looping {
                next = wrap_increment(base, elapsed_seconds, self.speed, [first, last])?;
            } else {
                next = next.clamp(first, last);
                self.playing = false;
            }
        } else if !self.looping && ((forward && next >= last) || (!forward && next <= first)) {
            self.playing = false;
        }
        self.time = next;
        Ok(previous != next || was_playing != self.playing)
    }

    /// Samples all point/cell field shapes at the current physical time.
    ///
    /// Uses the same named-field, nearest-step and validity semantics as
    /// [`TimeSeries::sample`], without revalidating source datasets per frame.
    ///
    /// # Errors
    ///
    /// Returns an error for a nonfinite public `time` value.
    pub fn current(&self) -> Result<DataSet, SceneError> {
        self.series.sample(self.time).map_err(Into::into)
    }

    /// Renders evenly spaced frames over the complete time range.
    ///
    /// # Errors
    ///
    /// Returns an error for zero frame count or failed sampling/rendering.
    /// This method collects frames in memory.
    pub fn movie_frames(
        &self,
        scene_for: impl Fn(DataSet) -> crate::Scene,
        camera: &crate::Camera,
        width: u32,
        height: u32,
        frames: usize,
    ) -> Result<Vec<(f64, crate::Frame)>, SceneError> {
        if frames == 0 {
            return Err(SceneError::InvalidPlayback(
                "movie frame count must be positive",
            ));
        }
        let mut out = Vec::with_capacity(frames);
        for index in 0..frames {
            let fraction = if frames == 1 {
                0.0
            } else {
                index as f64 / (frames - 1) as f64
            };
            let time = self.series.time_axis().at_fraction(fraction)?;
            let scene = scene_for(self.series.sample(time)?);
            out.push((time, crate::render(&scene, camera, width, height)?));
        }
        Ok(out)
    }
}

fn wrap_increment(
    base: f64,
    elapsed: f64,
    speed: f64,
    [first, last]: [f64; 2],
) -> Result<f64, SceneError> {
    // Scaling an overflowing finite endpoint span retains its interval. Ordinary
    // playback stays in native units, preserving small representable increments.
    let scale = if (last - first).is_finite() { 1.0 } else { 0.5 };
    let start = first * scale;
    let width = last * scale - start;
    let offset = (base * scale - start).clamp(0.0, width);
    let delta = (elapsed * scale) * speed;
    if !delta.is_finite() {
        return Err(SceneError::InvalidPlayback(
            "looping increment is not representable",
        ));
    }
    let remainder = delta % width;
    let wrapped = if remainder >= 0.0 {
        let gap = width - offset;
        if remainder >= gap {
            remainder - gap
        } else {
            offset + remainder
        }
    } else {
        let magnitude = -remainder;
        if magnitude > offset {
            width - (magnitude - offset)
        } else {
            offset - magnitude
        }
    };
    Ok(((start + wrapped) / scale).clamp(first, last))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_mesh::{Association, CellType, Cells, Field};

    fn step(z: f64, p: f64) -> DataSet {
        DataSet::new(
            "s",
            vec![[0.0, 0.0, z], [1.0, 0.0, z], [0.0, 1.0, z]],
            Cells::from_cells(&[(CellType::Triangle, &[0, 1, 2][..])]),
        )
        .with_field(Field::scalar("p", vec![p; 3]))
    }

    #[test]
    fn interpolates_positions_and_fields() {
        let series =
            TimeSeries::new(vec![step(0.0, 0.0), step(2.0, 10.0)], vec![0.0, 2.0]).unwrap();
        let mut player = Player::new(series).unwrap();
        player.seek(1.0).unwrap();
        let current = player.current().unwrap();
        assert_eq!(current.positions[0], [0.0, 0.0, 1.0]);
        assert_eq!(
            current
                .field(Association::Point, "p")
                .unwrap()
                .as_scalar()
                .unwrap(),
            &[5.0, 5.0, 5.0]
        );
    }

    #[test]
    fn advance_wraps_when_looping() {
        let series =
            TimeSeries::new(vec![step(0.0, 0.0), step(2.0, 10.0)], vec![0.0, 2.0]).unwrap();
        let mut player = Player::new(series).unwrap();
        player.play();
        player.advance(3.0).unwrap();
        assert_eq!(player.time, 1.0);
        assert!(player.playing);
    }

    #[test]
    fn movie_frames_cover_the_series() {
        let series =
            TimeSeries::new(vec![step(0.0, 0.0), step(2.0, 10.0)], vec![0.0, 2.0]).unwrap();
        let player = Player::new(series).unwrap();
        let camera = crate::Camera::default();
        let frames = player
            .movie_frames(
                |ds| {
                    crate::Scene::new().with_actor(
                        crate::Actor::new(ds)
                            .coloured_by("p", crate::TransferFunction::viridis([0.0, 10.0])),
                    )
                },
                &camera,
                16,
                16,
                3,
            )
            .unwrap();
        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0].0, 0.0);
        assert_eq!(frames[2].0, 2.0);
    }
}
