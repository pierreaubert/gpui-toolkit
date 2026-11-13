//! Movie export: PNG sequences and MP4 muxing.
//!
//! The portable artifact is a numbered PNG sequence: it works on every
//! host, including wasm, where each frame can be downloaded or fed to a
//! browser encoder. On native hosts the `mp4` feature additionally muxes
//! frames to H.264 MP4 through the system `ffmpeg`, located at runtime.
//!
//! Exports stream: frames are encoded and handed to the caller's sink (or
//! to `ffmpeg` through a bounded queue) one at a time, so peak memory is
//! a frame plus queue depth, not the whole movie. The collect-all
//! [`png_sequence`]/[`mux_mp4`] conveniences remain for small exports and
//! reject inputs past [`MAX_COLLECT_FRAMES`]/[`MAX_COLLECT_BYTES`].
//! `ffmpeg` stdout/stderr are drained concurrently with feeding stdin,
//! so frames larger than an OS pipe buffer cannot deadlock the mux.

// Rust guideline compliant 2026-02-21

use crate::Frame;

/// Failures reported by movie export.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MovieError {
    /// Frame timestamps must be strictly increasing.
    #[error("movie frames must have strictly increasing times")]
    NonIncreasingTimes,
    /// The requested encoder is unavailable or failed.
    #[error("encoder failed: {0}")]
    Encoder(String),
    /// A frame could not be encoded as PNG.
    #[error("png encoding failed: {0}")]
    Png(String),
    /// The export exceeds the collect-all small-export limit; use a
    /// streaming export instead.
    #[error(
        "export of {frames} frames exceeds the collect-all limit ({limit} frames / {limit_bytes} encoded bytes); use the streaming export"
    )]
    TooLarge {
        /// Frames in the rejected export.
        frames: usize,
        /// Maximum frames for collect-all export.
        limit: usize,
        /// Maximum encoded bytes for collect-all export.
        limit_bytes: usize,
    },
    /// The sink asked to stop; frames already delivered stay delivered.
    #[error("export cancelled by the sink after {delivered} frames")]
    Cancelled {
        /// Frames delivered before cancellation.
        delivered: usize,
    },
    /// Writing exported bytes to the caller's writer failed.
    #[error("cannot write export output: {0}")]
    Output(String),
}

/// Maximum frames the collect-all conveniences will hold in memory.
///
/// Larger exports must use [`write_png_sequence`] or [`mux_mp4_to`],
/// which stream with a bounded queue.
pub const MAX_COLLECT_FRAMES: usize = 128;

/// Maximum encoded bytes the collect-all conveniences will hold in memory.
pub const MAX_COLLECT_BYTES: usize = 64 * 1024 * 1024;

/// Encoded frames buffered between the producer and the `ffmpeg` feeder.
///
/// Bounds in-flight export memory independently of movie length; the
/// producer blocks (backpressure) when the encoder falls behind.
#[cfg(all(feature = "mp4", not(target_family = "wasm")))]
const FEED_QUEUE_FRAMES: usize = 4;

/// One frame of a movie: physical time plus its rendered image.
pub type TimedFrame = (f64, Frame);

/// Validates frame timing shared by every export path.
///
/// # Errors
///
/// Returns [`MovieError::NonIncreasingTimes`] for empty or unordered input.
pub fn validate_frames(frames: &[TimedFrame]) -> Result<(), MovieError> {
    if frames.is_empty() || frames.windows(2).any(|pair| pair[1].0 <= pair[0].0) {
        return Err(MovieError::NonIncreasingTimes);
    }
    Ok(())
}

/// Frame rate implied by frame times, for encoders that need a constant rate.
#[must_use]
pub fn frame_rate(frames: &[TimedFrame]) -> f64 {
    if frames.len() < 2 {
        return 30.0;
    }
    let span = frames[frames.len() - 1].0 - frames[0].0;
    if span <= 0.0 {
        return 30.0;
    }
    (frames.len() - 1) as f64 / span
}

/// Streams frames as a numbered PNG sequence to a sink.
///
/// Each frame is encoded and handed to `sink(name, bytes)` in playback
/// order (`frame_000000.png`, and so on); at most one encoded frame is
/// alive at a time. The sink cancels the export by returning
/// [`MovieError::Cancelled`], which stops frame production immediately.
/// Returns the number of frames delivered.
///
/// # Errors
///
/// Returns an error for empty/non-increasing times (checked
/// incrementally, so a late disorder is reported after earlier frames
/// were delivered), a PNG encoding failure, or the sink's error.
#[cfg(feature = "png")]
pub fn write_png_sequence(
    frames: impl IntoIterator<Item = TimedFrame>,
    mut sink: impl FnMut(String, Vec<u8>) -> Result<(), MovieError>,
) -> Result<usize, MovieError> {
    let mut delivered = 0usize;
    let mut previous: Option<f64> = None;
    for (index, (time, frame)) in frames.into_iter().enumerate() {
        if previous.is_some_and(|previous| time <= previous) || !time.is_finite() {
            return Err(MovieError::NonIncreasingTimes);
        }
        previous = Some(time);
        let bytes = frame.to_png().map_err(MovieError::Png)?;
        sink(format!("frame_{index:06}.png"), bytes)?;
        delivered += 1;
    }
    if delivered == 0 {
        return Err(MovieError::NonIncreasingTimes);
    }
    Ok(delivered)
}

/// Encodes frames as a numbered PNG sequence: `frame_000000.png`, and so on.
///
/// Available under the `png` feature. File names sort lexicographically in
/// playback order, so any standard muxer or image viewer plays them back.
/// This collect-all convenience holds every encoded frame in memory and
/// rejects exports past [`MAX_COLLECT_FRAMES`]/[`MAX_COLLECT_BYTES`];
/// use [`write_png_sequence`] for larger movies.
///
/// # Errors
///
/// Returns an error for unordered times, a PNG encoding failure, or
/// [`MovieError::TooLarge`] past the small-export limit.
#[cfg(feature = "png")]
pub fn png_sequence(frames: &[TimedFrame]) -> Result<Vec<(String, Vec<u8>)>, MovieError> {
    validate_frames(frames)?;
    if frames.len() > MAX_COLLECT_FRAMES {
        return Err(MovieError::TooLarge {
            frames: frames.len(),
            limit: MAX_COLLECT_FRAMES,
            limit_bytes: MAX_COLLECT_BYTES,
        });
    }
    let mut out = Vec::with_capacity(frames.len());
    let mut total_bytes = 0usize;
    for (index, (_, frame)) in frames.iter().enumerate() {
        let bytes = frame.to_png().map_err(MovieError::Png)?;
        total_bytes = total_bytes.saturating_add(bytes.len());
        if total_bytes > MAX_COLLECT_BYTES {
            return Err(MovieError::TooLarge {
                frames: frames.len(),
                limit: MAX_COLLECT_FRAMES,
                limit_bytes: MAX_COLLECT_BYTES,
            });
        }
        out.push((format!("frame_{index:06}.png"), bytes));
    }
    Ok(out)
}

/// Muxes streaming frames to an H.264 MP4 written to `out`.
///
/// Frames are encoded one at a time and fed to `ffmpeg` through a
/// bounded queue ([`FEED_QUEUE_FRAMES`] encoded frames), while separate
/// threads drain `ffmpeg` stdout into `out` and stderr into a capped
/// buffer. Peak memory therefore stays frame-sized however long the
/// movie is, and frames larger than an OS pipe buffer cannot deadlock
/// against undrained output. Times are validated incrementally; on any
/// failure the encoder is killed and `out` may hold a partial stream.
/// Odd frame dimensions are padded to even sizes, as yuv420p requires.
///
/// # Errors
///
/// Returns an error for empty/non-increasing times, a non-positive or
/// non-finite `fps`, a PNG failure, [`MovieError::Encoder`] when
/// `ffmpeg` is missing or fails, or [`MovieError::Output`] when writing
/// to `out` fails. Unavailable on wasm, where [`write_png_sequence`] is
/// the export.
#[cfg(all(feature = "mp4", not(target_family = "wasm")))]
pub fn mux_mp4_to(
    frames: impl IntoIterator<Item = TimedFrame>,
    fps: f64,
    out: impl std::io::Write + Send,
) -> Result<(), MovieError> {
    mux_mp4_to_command("ffmpeg", frames, fps, out)
}

/// [`mux_mp4_to`] with an explicit encoder program name.
#[cfg(all(feature = "mp4", not(target_family = "wasm")))]
fn mux_mp4_to_command(
    program: &str,
    frames: impl IntoIterator<Item = TimedFrame>,
    fps: f64,
    mut out: impl std::io::Write + Send,
) -> Result<(), MovieError> {
    use std::io::{Read, Write};
    use std::process::{Command, Stdio};
    use std::sync::mpsc::sync_channel;

    if !fps.is_finite() || fps <= 0.0 {
        return Err(MovieError::Encoder(format!("invalid frame rate {fps}")));
    }
    let mut child = Command::new(program)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "image2pipe",
            "-framerate",
            &format!("{fps}"),
            "-i",
            "pipe:0",
            "-vf",
            "pad=ceil(iw/2)*2:ceil(ih/2)*2",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-movflags",
            "frag_keyframe+empty_moov",
            "-f",
            "mp4",
            "pipe:1",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| MovieError::Encoder(format!("cannot start ffmpeg: {e}")))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| MovieError::Encoder("ffmpeg stdin unavailable".into()))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| MovieError::Encoder("ffmpeg stdout unavailable".into()))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| MovieError::Encoder("ffmpeg stderr unavailable".into()))?;

    let (feed_tx, feed_rx) = sync_channel::<Vec<u8>>(FEED_QUEUE_FRAMES);
    let pipeline = std::thread::scope(|scope| {
        // Drain stderr concurrently (capped retention, unbounded drain).
        let stderr_thread = scope.spawn(move || {
            let mut kept = Vec::new();
            let mut chunk = [0u8; 8192];
            loop {
                match stderr.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => {
                        if kept.len() < 64 * 1024 {
                            kept.extend_from_slice(&chunk[..read.min(64 * 1024 - kept.len())]);
                        }
                    }
                }
            }
            kept
        });
        // Drain stdout into the caller's writer concurrently.
        let stdout_thread = scope.spawn(move || {
            let mut prefix = [0u8; 8];
            let mut total = 0usize;
            let mut chunk = vec![0u8; 64 * 1024];
            loop {
                match stdout.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(read) => {
                        if total < prefix.len() {
                            let take = read.min(prefix.len() - total);
                            prefix[total..total + take].copy_from_slice(&chunk[..take]);
                        }
                        total += read;
                        if let Err(error) = out.write_all(&chunk[..read]) {
                            return Err(MovieError::Output(error.to_string()));
                        }
                    }
                    Err(error) => return Err(MovieError::Output(error.to_string())),
                }
            }
            out.flush()
                .map_err(|error| MovieError::Output(error.to_string()))?;
            Ok((total, prefix))
        });
        // Feed stdin from the bounded queue.
        let stdin_thread = scope.spawn(move || {
            while let Ok(png) = feed_rx.recv() {
                if stdin.write_all(&png).is_err() {
                    break;
                }
            }
            let _ = stdin.flush();
        });

        // Produce frames on this thread; the bounded channel applies
        // backpressure, and a send failure means the feeder died.
        let mut produced = 0usize;
        let mut previous: Option<f64> = None;
        let mut failure: Option<MovieError> = None;
        for (time, frame) in frames {
            if previous.is_some_and(|previous| time <= previous) || !time.is_finite() {
                failure = Some(MovieError::NonIncreasingTimes);
                break;
            }
            previous = Some(time);
            match frame.to_png().map_err(MovieError::Png) {
                Ok(png) => {
                    if feed_tx.send(png).is_err() {
                        failure = Some(MovieError::Encoder(
                            "ffmpeg stopped accepting frames".into(),
                        ));
                        break;
                    }
                    produced += 1;
                }
                Err(error) => {
                    failure = Some(error);
                    break;
                }
            }
        }
        if failure.is_none() && produced == 0 {
            failure = Some(MovieError::NonIncreasingTimes);
        }
        drop(feed_tx);
        if failure.is_some() {
            let _ = child.kill();
        }
        let stderr_bytes = stderr_thread.join().unwrap_or_default();
        let stdout_result = stdout_thread
            .join()
            .unwrap_or_else(|_| Err(MovieError::Output("output thread panicked".into())));
        let _ = stdin_thread.join();
        (failure, produced, stderr_bytes, stdout_result)
    });
    let (failure, _produced, stderr_bytes, stdout_result) = pipeline;
    let status = child
        .wait()
        .map_err(|e| MovieError::Encoder(format!("ffmpeg failed to finish: {e}")))?;
    if let Some(failure) = failure {
        return Err(failure);
    }
    let (total, prefix) = stdout_result?;
    if !status.success() {
        return Err(MovieError::Encoder(
            String::from_utf8_lossy(&stderr_bytes).trim().to_string(),
        ));
    }
    if total < 8 || &prefix[4..8] != b"ftyp" {
        return Err(MovieError::Encoder(
            "ffmpeg did not produce an MP4 stream".into(),
        ));
    }
    Ok(())
}

/// Muxes frames to an H.264 MP4 (yuv420p, fragmented for streaming).
///
/// Frames are piped to `ffmpeg` at the rate implied by their times and
/// the MP4 bytes are returned in memory; nothing touches the filesystem.
/// This collect-all convenience rejects exports past
/// [`MAX_COLLECT_FRAMES`]; use [`mux_mp4_to`] for larger movies.
///
/// # Errors
///
/// Returns [`MovieError::Encoder`] when `ffmpeg` is missing or fails, or
/// [`MovieError::TooLarge`] past the small-export limit. This function is
/// unavailable on wasm, where [`png_sequence`] is the export.
#[cfg(all(feature = "mp4", not(target_family = "wasm")))]
pub fn mux_mp4(frames: &[TimedFrame]) -> Result<Vec<u8>, MovieError> {
    validate_frames(frames)?;
    if frames.len() > MAX_COLLECT_FRAMES {
        return Err(MovieError::TooLarge {
            frames: frames.len(),
            limit: MAX_COLLECT_FRAMES,
            limit_bytes: MAX_COLLECT_BYTES,
        });
    }
    let fps = frame_rate(frames);
    let mut bytes = Vec::new();
    mux_mp4_to(frames.iter().cloned(), fps, &mut bytes)?;
    if bytes.len() > MAX_COLLECT_BYTES {
        return Err(MovieError::TooLarge {
            frames: frames.len(),
            limit: MAX_COLLECT_FRAMES,
            limit_bytes: MAX_COLLECT_BYTES,
        });
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(value: u8) -> Frame {
        Frame {
            width: 16,
            height: 16,
            rgba: vec![value, value, value, 255]
                .into_iter()
                .cycle()
                .take(16 * 16 * 4)
                .collect(),
        }
    }

    #[test]
    fn frame_rate_comes_from_times() {
        let frames = vec![(0.0, frame(0)), (0.5, frame(1)), (1.0, frame(2))];
        assert_eq!(frame_rate(&frames), 2.0);
        assert_eq!(validate_frames(&frames), Ok(()));
        assert_eq!(
            validate_frames(&[(1.0, frame(0)), (0.5, frame(1))]),
            Err(MovieError::NonIncreasingTimes)
        );
    }

    #[cfg(feature = "png")]
    #[test]
    fn png_sequence_names_sort_in_playback_order() {
        let frames = vec![(0.0, frame(0)), (0.5, frame(90)), (1.0, frame(180))];
        let sequence = png_sequence(&frames).unwrap();
        assert_eq!(sequence.len(), 3);
        assert_eq!(sequence[0].0, "frame_000000.png");
        assert_eq!(sequence[2].0, "frame_000002.png");
        assert!(sequence.iter().all(|(_, bytes)| &bytes[..4] == b"\x89PNG"));
    }

    #[cfg(feature = "png")]
    #[test]
    fn streaming_png_delivers_in_order_and_cancels() {
        use std::cell::Cell;
        let produced = Cell::new(0usize);
        let frames = std::iter::from_fn(|| {
            let index = produced.get();
            produced.set(index + 1);
            (index < 10).then(|| (index as f64 * 0.1, frame(index as u8)))
        });
        let mut names = Vec::new();
        let error = write_png_sequence(frames, |name, bytes| {
            assert_eq!(&bytes[..4], b"\x89PNG");
            names.push(name);
            if names.len() == 3 {
                return Err(MovieError::Cancelled { delivered: 3 });
            }
            Ok(())
        })
        .unwrap_err();
        assert_eq!(error, MovieError::Cancelled { delivered: 3 });
        assert_eq!(names.len(), 3);
        assert_eq!(names[2], "frame_000002.png");
        assert_eq!(produced.get(), 3, "cancellation stops frame production");

        let mut count = 0;
        let delivered = write_png_sequence([(0.0, frame(0)), (0.5, frame(1))], |_, _| {
            count += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!((delivered, count), (2, 2));
        assert_eq!(
            write_png_sequence([(1.0, frame(0)), (0.5, frame(1))], |_, _| Ok(())),
            Err(MovieError::NonIncreasingTimes)
        );
        assert_eq!(
            write_png_sequence([], |_, _| Ok(())),
            Err(MovieError::NonIncreasingTimes)
        );
    }

    #[cfg(feature = "png")]
    #[test]
    fn streaming_export_of_300_frames_never_holds_more_than_one_frame() {
        use std::cell::Cell;
        // Peak-memory contract: the producer is never more than the
        // in-flight frame ahead of the sink, at any movie length.
        let produced = Cell::new(0usize);
        let frames = std::iter::from_fn(|| {
            let index = produced.get();
            produced.set(index + 1);
            (index < 300).then(|| (index as f64 * 0.033, frame((index % 251) as u8)))
        });
        let mut delivered = 0usize;
        let total = write_png_sequence(frames, |_, bytes| {
            assert!(
                produced.get() - delivered <= 1,
                "producer ran ahead: produced {} delivered {delivered}",
                produced.get()
            );
            assert!(!bytes.is_empty());
            delivered += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!((total, delivered), (300, 300));
    }

    #[cfg(feature = "png")]
    #[test]
    fn collect_all_png_rejects_past_the_small_export_limit() {
        let frames: Vec<TimedFrame> = (0..=MAX_COLLECT_FRAMES)
            .map(|index| (index as f64 * 0.1, frame(0)))
            .collect();
        assert!(matches!(
            png_sequence(&frames),
            Err(MovieError::TooLarge { .. })
        ));
    }

    /// Whether `ffmpeg` must be present for this test run.
    #[cfg(all(feature = "mp4", not(target_family = "wasm")))]
    fn ffmpeg_required() -> bool {
        std::env::var_os("GPUI_REQUIRE_FFMPEG").is_some()
    }

    /// True only for the one tolerated absence: no ffmpeg on a host
    /// that did not opt into GPUI_REQUIRE_FFMPEG=1.
    #[cfg(all(feature = "mp4", not(target_family = "wasm")))]
    fn absence_tolerated(error: &MovieError) -> bool {
        match error {
            MovieError::Encoder(message) if message.contains("cannot start ffmpeg") => {
                !ffmpeg_required()
            }
            _ => false,
        }
    }

    #[cfg(all(feature = "mp4", not(target_family = "wasm")))]
    #[test]
    fn muxes_a_real_mp4_when_ffmpeg_is_available() {
        let frames = vec![(0.0, frame(10)), (0.25, frame(90)), (0.5, frame(170))];
        match mux_mp4(&frames) {
            Ok(bytes) => {
                assert!(&bytes[4..8] == b"ftyp", "missing MP4 signature");
                assert!(bytes.len() > 200, "suspiciously small MP4: {}", bytes.len());
            }
            Err(error) if absence_tolerated(&error) => {}
            Err(other) => panic!("unexpected mux failure: {other}"),
        }
    }

    #[cfg(all(feature = "mp4", not(target_family = "wasm")))]
    #[test]
    fn missing_encoder_is_a_typed_error() {
        let mut out = Vec::new();
        let error = mux_mp4_to_command(
            "gpui-ffmpeg-that-does-not-exist",
            [(0.0, frame(0)), (0.5, frame(1))],
            30.0,
            &mut out,
        )
        .unwrap_err();
        assert!(
            matches!(&error, MovieError::Encoder(message) if message.contains("cannot start ffmpeg")),
            "got {error}"
        );
    }

    #[cfg(all(feature = "mp4", not(target_family = "wasm")))]
    #[test]
    fn large_frames_stream_through_pipes_without_deadlock() {
        // Incompressible 512x512 frames: each PNG is far larger than an
        // OS pipe buffer (64 KiB), and the MP4 output is larger still.
        // The pre-streaming mux wrote all stdin before reading stdout
        // and deadlocked on exactly this shape.
        fn noise_frame(seed: u32) -> Frame {
            let mut state = seed.wrapping_mul(2_654_435_761).wrapping_add(1);
            let mut rgba = vec![0u8; 512 * 512 * 4];
            for pixel in rgba.as_chunks_mut::<4>().0 {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                pixel.copy_from_slice(&state.to_le_bytes());
                pixel[3] = 255;
            }
            Frame {
                width: 512,
                height: 512,
                rgba,
            }
        }
        let frames = (0..12).map(|index| (index as f64 * 0.1, noise_frame(index + 1)));
        let mut out = Vec::new();
        match mux_mp4_to(frames, 10.0, &mut out) {
            Ok(()) => {
                assert_eq!(&out[4..8], b"ftyp", "missing MP4 signature");
                assert!(
                    out.len() > 64 * 1024,
                    "output must exceed a pipe buffer to prove the drain: {}",
                    out.len()
                );
            }
            Err(error) if absence_tolerated(&error) => {}
            Err(other) => panic!("unexpected mux failure: {other}"),
        }
    }
}
