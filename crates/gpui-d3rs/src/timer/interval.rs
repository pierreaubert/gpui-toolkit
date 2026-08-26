use super::Timer;
use super::misc::now;
use super::period_from_ms;
use std::time::Duration;

/// A repeating timer that fires at fixed intervals.
///
/// Unlike `Timer` which fires as fast as possible (approximately 60fps),
/// `Interval` fires at exactly the specified interval.
#[derive(Clone)]
pub struct Interval {
    pub(super) timer: Timer,
}

impl std::fmt::Debug for Interval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Interval")
            .field("timer", &self.timer)
            .finish()
    }
}

impl Interval {
    /// Creates a new interval timer that fires at fixed intervals.
    ///
    /// # Arguments
    ///
    /// * `callback` - Function called at each interval. Return `false` to stop.
    /// * `interval_ms` - Interval between callbacks in milliseconds
    /// * `time` - Optional start time
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use d3rs::timer::Interval;
    ///
    /// let interval = Interval::new(|elapsed| {
    ///     println!("Tick at {} ms", elapsed);
    ///     elapsed < 5000.0 // Run for 5 seconds
    /// }, 1000.0, None);
    /// ```
    pub fn new<F>(callback: F, interval_ms: f64, time: Option<f64>) -> Self
    where
        F: FnMut(f64) -> bool + Send + 'static,
    {
        let start_time = time.unwrap_or_else(now);
        Interval {
            timer: Timer::with_period(
                callback,
                Some(interval_ms),
                Some(start_time),
                period_from_ms(interval_ms, Duration::from_millis(1)),
            ),
        }
    }

    /// Stops the interval timer.
    pub fn stop(&self) {
        self.timer.stop();
    }

    /// Returns true if the interval has been stopped.
    pub fn is_stopped(&self) -> bool {
        self.timer.is_stopped()
    }

    /// Wait for the interval to complete (blocking).
    pub fn join(self) {
        self.timer.join();
    }

    /// Wait for interval completion for at most `timeout`.
    pub fn try_join(&self, timeout: Duration) -> bool {
        self.timer.try_join(timeout)
    }
}

/// Creates a repeating timer that fires at fixed intervals.
///
/// # Arguments
///
/// * `callback` - Function called at each interval. Return `false` to stop.
/// * `interval_ms` - Interval between callbacks in milliseconds
/// * `time` - Optional start time
///
/// # Example
///
/// ```rust,no_run
/// use d3rs::timer::interval;
///
/// let mut count = 0;
/// let t = interval(move |elapsed| {
///     count += 1;
///     println!("Tick {} at {} ms", count, elapsed);
///     count < 10 // Stop after 10 ticks
/// }, 100.0, None);
/// ```
pub fn interval<F>(callback: F, interval_ms: f64, time: Option<f64>) -> Interval
where
    F: FnMut(f64) -> bool + Send + 'static,
{
    Interval::new(callback, interval_ms, time)
}
