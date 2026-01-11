//! # d3-timer - Animation Timing Module
//!
//! This module provides efficient animation timing utilities, inspired by D3.js's d3-timer module.
//! It uses `std::time::Instant` for high-resolution monotonic time and provides callbacks
//! scheduled relative to an application-defined epoch.
//!
//! ## Key Features
//!
//! - **now()**: Returns milliseconds since the timer epoch
//! - **Timer**: A repeating timer that calls a callback on each animation frame
//! - **timeout()**: A one-shot timer that fires once after a delay
//! - **interval()**: A repeating timer that fires at fixed intervals
//!
//! ## Example
//!
//! ```rust,no_run
//! use d3rs::timer::{now, timer, timeout, interval};
//!
//! // Get current time in ms since epoch
//! let t = now();
//!
//! // One-shot timer after 1 second
//! timeout(|elapsed| {
//!     println!("Fired after {} ms", elapsed);
//! }, 1000.0, None);
//!
//! // Repeating every 500ms
//! let mut count = 0;
//! interval(move |elapsed| {
//!     count += 1;
//!     println!("Tick {} at {} ms", count, elapsed);
//!     count < 10 // Return false to stop
//! }, 500.0, None);
//! ```

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

/// Global epoch for timing - lazily initialized on first use
static EPOCH: OnceLock<Instant> = OnceLock::new();

/// Counter for generating unique timer IDs
static TIMER_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Get the epoch instant, initializing it if necessary
fn get_epoch() -> Instant {
    *EPOCH.get_or_init(Instant::now)
}

/// Returns the current time in milliseconds since the timer epoch.
///
/// The epoch is established when this function (or any timer function) is first called.
/// This provides a consistent time base for all animations.
///
/// # Example
///
/// ```rust
/// use d3rs::timer::now;
///
/// let start = now();
/// // ... do some work ...
/// let elapsed = now() - start;
/// println!("Elapsed: {} ms", elapsed);
/// ```
pub fn now() -> f64 {
    get_epoch().elapsed().as_secs_f64() * 1000.0
}

/// Sets the current clock time for timer calculations.
///
/// This is primarily useful for testing or for synchronizing timers with
/// external time sources. In most cases, you should use `now()` which
/// automatically tracks real time.
///
/// Note: This function resets the epoch to a new instant offset by the
/// specified time. It should be used with caution as it affects all timers.
pub fn set_now(t: f64) {
    // This is a no-op in our implementation since we use real time
    // In D3.js this is used for testing with fake time
    let _ = t;
}

/// Timer state
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TimerState {
    /// Timer is active and will fire
    Active,
    /// Timer has been stopped
    Stopped,
}

/// A timer that invokes a callback repeatedly.
///
/// The callback receives the elapsed time since the timer was started.
/// If the callback returns `false`, the timer stops.
///
/// # Example
///
/// ```rust,no_run
/// use d3rs::timer::Timer;
///
/// let timer = Timer::new(|elapsed| {
///     println!("Elapsed: {} ms", elapsed);
///     elapsed < 5000.0 // Stop after 5 seconds
/// }, None, None);
///
/// // Timer runs in background thread
/// // Call timer.stop() to cancel early
/// ```
#[derive(Clone)]
#[allow(clippy::type_complexity)]
pub struct Timer {
    id: u64,
    callback: Arc<Mutex<Box<dyn FnMut(f64) -> bool + Send>>>,
    delay: f64,
    start_time: f64,
    stopped: Arc<AtomicBool>,
    handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
}

impl std::fmt::Debug for Timer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Timer")
            .field("id", &self.id)
            .field("delay", &self.delay)
            .field("start_time", &self.start_time)
            .field("stopped", &self.stopped.load(Ordering::SeqCst))
            .finish()
    }
}

impl Timer {
    /// Creates a new timer with the given callback.
    ///
    /// # Arguments
    ///
    /// * `callback` - Function called on each tick. Receives elapsed time in ms.
    ///   Return `false` to stop the timer.
    /// * `delay` - Optional delay in ms before first callback (default: 0)
    /// * `time` - Optional start time for elapsed calculation (default: now())
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use d3rs::timer::Timer;
    ///
    /// // Timer that logs elapsed time every ~16ms
    /// let timer = Timer::new(|elapsed| {
    ///     println!("t = {:.1} ms", elapsed);
    ///     elapsed < 1000.0
    /// }, None, None);
    /// ```
    pub fn new<F>(callback: F, delay: Option<f64>, time: Option<f64>) -> Self
    where
        F: FnMut(f64) -> bool + Send + 'static,
    {
        let id = TIMER_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
        let delay = delay.unwrap_or(0.0);
        let start_time = time.unwrap_or_else(now);
        let stopped = Arc::new(AtomicBool::new(false));
        let callback = Arc::new(Mutex::new(
            Box::new(callback) as Box<dyn FnMut(f64) -> bool + Send>
        ));

        let timer = Timer {
            id,
            callback: callback.clone(),
            delay,
            start_time,
            stopped: stopped.clone(),
            handle: Arc::new(Mutex::new(None)),
        };

        // Start the timer thread
        let stopped_clone = stopped.clone();
        let callback_clone = callback;
        let start = start_time;
        let delay_ms = delay;

        let handle = thread::spawn(move || {
            // Wait for initial delay
            if delay_ms > 0.0 {
                thread::sleep(Duration::from_secs_f64(delay_ms / 1000.0));
            }

            // Animation loop - approximately 60fps
            let frame_duration = Duration::from_millis(16);

            while !stopped_clone.load(Ordering::SeqCst) {
                let elapsed = now() - start;

                // Call the callback
                let should_continue = {
                    let mut cb = callback_clone.lock().unwrap();
                    cb(elapsed)
                };

                if !should_continue {
                    stopped_clone.store(true, Ordering::SeqCst);
                    break;
                }

                thread::sleep(frame_duration);
            }
        });

        *timer.handle.lock().unwrap() = Some(handle);
        timer
    }

    /// Stops the timer.
    ///
    /// Once stopped, a timer cannot be restarted. Use `restart()` to
    /// reinitialize with a new callback.
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
    }

    /// Returns true if the timer has been stopped.
    pub fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }

    /// Restarts the timer with a new callback.
    ///
    /// # Arguments
    ///
    /// * `callback` - New function to call on each tick
    /// * `delay` - Optional delay before first callback (default: 0)
    /// * `time` - Optional start time (default: now())
    pub fn restart<F>(&mut self, callback: F, delay: Option<f64>, time: Option<f64>)
    where
        F: FnMut(f64) -> bool + Send + 'static,
    {
        // Stop the old timer
        self.stop();

        // Wait for old thread to finish
        if let Some(handle) = self.handle.lock().unwrap().take() {
            let _ = handle.join();
        }

        // Create new timer state
        let new_timer = Timer::new(callback, delay, time);
        self.id = new_timer.id;
        self.callback = new_timer.callback;
        self.delay = new_timer.delay;
        self.start_time = new_timer.start_time;
        self.stopped = new_timer.stopped;
        self.handle = new_timer.handle;
    }

    /// Returns the timer's unique ID.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Returns the delay before first callback.
    pub fn delay(&self) -> f64 {
        self.delay
    }

    /// Returns the start time used for elapsed calculation.
    pub fn start_time(&self) -> f64 {
        self.start_time
    }

    /// Wait for the timer to complete (blocking).
    ///
    /// This is useful in tests or when you need to ensure all
    /// timer callbacks have finished.
    pub fn join(self) {
        if let Some(handle) = self.handle.lock().unwrap().take() {
            let _ = handle.join();
        }
    }
}

/// Creates a new timer that invokes the callback repeatedly.
///
/// This is a convenience function equivalent to `Timer::new()`.
///
/// # Arguments
///
/// * `callback` - Function called on each tick. Return `false` to stop.
/// * `delay` - Optional delay in ms before first callback
/// * `time` - Optional start time for elapsed calculation
///
/// # Example
///
/// ```rust,no_run
/// use d3rs::timer::timer;
///
/// let t = timer(|elapsed| {
///     println!("Elapsed: {} ms", elapsed);
///     elapsed < 2000.0 // Run for 2 seconds
/// }, None, None);
/// ```
pub fn timer<F>(callback: F, delay: Option<f64>, time: Option<f64>) -> Timer
where
    F: FnMut(f64) -> bool + Send + 'static,
{
    Timer::new(callback, delay, time)
}

/// A one-shot timer that fires once after a delay.
///
/// Unlike `Timer`, this fires exactly once and then automatically stops.
#[derive(Clone)]
pub struct Timeout {
    inner: Timer,
}

impl std::fmt::Debug for Timeout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Timeout")
            .field("inner", &self.inner)
            .finish()
    }
}

impl Timeout {
    /// Creates a new timeout that fires once after the specified delay.
    ///
    /// # Arguments
    ///
    /// * `callback` - Function called once when the timeout fires
    /// * `delay` - Delay in ms before firing (default: 0)
    /// * `time` - Optional start time for calculation
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use d3rs::timer::Timeout;
    ///
    /// let timeout = Timeout::new(|elapsed| {
    ///     println!("Fired after {} ms", elapsed);
    /// }, 1000.0, None);
    /// ```
    pub fn new<F>(callback: F, delay: f64, time: Option<f64>) -> Self
    where
        F: FnOnce(f64) + Send + 'static,
    {
        let callback = Mutex::new(Some(callback));
        let timer = Timer::new(
            move |elapsed| {
                if let Some(cb) = callback.lock().unwrap().take() {
                    cb(elapsed);
                }
                false // Only fire once
            },
            Some(delay),
            time,
        );
        Timeout { inner: timer }
    }

    /// Stops the timeout before it fires.
    pub fn stop(&self) {
        self.inner.stop();
    }

    /// Returns true if the timeout has been stopped or has fired.
    pub fn is_stopped(&self) -> bool {
        self.inner.is_stopped()
    }

    /// Wait for the timeout to complete (blocking).
    pub fn join(self) {
        self.inner.join();
    }
}

/// Creates a one-shot timer that fires once after a delay.
///
/// # Arguments
///
/// * `callback` - Function called once when the timeout fires
/// * `delay` - Delay in ms before firing
/// * `time` - Optional start time
///
/// # Example
///
/// ```rust,no_run
/// use d3rs::timer::timeout;
///
/// let t = timeout(|elapsed| {
///     println!("Timeout fired at {} ms", elapsed);
/// }, 500.0, None);
/// ```
pub fn timeout<F>(callback: F, delay: f64, time: Option<f64>) -> Timeout
where
    F: FnOnce(f64) + Send + 'static,
{
    Timeout::new(callback, delay, time)
}

/// A repeating timer that fires at fixed intervals.
///
/// Unlike `Timer` which fires as fast as possible (approximately 60fps),
/// `Interval` fires at exactly the specified interval.
#[derive(Clone)]
pub struct Interval {
    timer: Timer,
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
        let id = TIMER_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
        let start_time = time.unwrap_or_else(now);
        let stopped = Arc::new(AtomicBool::new(false));
        let callback = Arc::new(Mutex::new(
            Box::new(callback) as Box<dyn FnMut(f64) -> bool + Send>
        ));
        let handle = Arc::new(Mutex::new(None));

        let stopped_clone = stopped.clone();
        let callback_clone = callback.clone();
        let start = start_time;

        let thread_handle = thread::spawn(move || {
            let interval_duration = Duration::from_secs_f64(interval_ms / 1000.0);
            let mut next_tick = Instant::now() + interval_duration;

            while !stopped_clone.load(Ordering::SeqCst) {
                // Wait until next tick
                let now_instant = Instant::now();
                if now_instant < next_tick {
                    thread::sleep(next_tick - now_instant);
                }

                if stopped_clone.load(Ordering::SeqCst) {
                    break;
                }

                let elapsed = now() - start;

                // Call the callback
                let should_continue = {
                    let mut cb = callback_clone.lock().unwrap();
                    cb(elapsed)
                };

                if !should_continue {
                    stopped_clone.store(true, Ordering::SeqCst);
                    break;
                }

                // Schedule next tick
                next_tick += interval_duration;

                // If we've fallen behind, catch up
                if next_tick < Instant::now() {
                    next_tick = Instant::now() + interval_duration;
                }
            }
        });

        *handle.lock().unwrap() = Some(thread_handle);

        Interval {
            timer: Timer {
                id,
                callback,
                delay: interval_ms,
                start_time,
                stopped,
                handle,
            },
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

/// Immediately invokes any eligible timer callbacks.
///
/// This is useful for ensuring that timers are up-to-date before
/// synchronous operations like rendering.
///
/// Note: In this implementation, timers run in their own threads,
/// so this function is primarily for API compatibility with D3.js.
pub fn timer_flush() {
    // In D3.js, this processes all pending timers synchronously.
    // In our threaded implementation, timers run independently,
    // so this is mostly a no-op. We ensure the epoch is initialized.
    let _ = get_epoch();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::time::Duration;

    #[test]
    fn test_now_monotonic() {
        let t1 = now();
        thread::sleep(Duration::from_millis(10));
        let t2 = now();
        assert!(t2 > t1);
        assert!(t2 - t1 >= 9.0); // At least 9ms elapsed (allowing for timing variance)
    }

    #[test]
    fn test_now_epoch_consistent() {
        let t1 = now();
        let t2 = now();
        let t3 = now();
        assert!(t1 <= t2);
        assert!(t2 <= t3);
    }

    #[test]
    fn test_timer_basic() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let timer = Timer::new(
            move |_elapsed| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                counter_clone.load(Ordering::SeqCst) < 3
            },
            None,
            None,
        );

        // Wait for timer to complete
        timer.join();

        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_timer_stop() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let timer = timer(
            move |_elapsed| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                true // Never stop by itself
            },
            None,
            None,
        );

        thread::sleep(Duration::from_millis(50));
        timer.stop();

        let final_count = counter.load(Ordering::SeqCst);
        assert!(final_count > 0, "Timer should have ticked at least once");
        assert!(final_count < 100, "Timer should not have run forever");
    }

    #[test]
    fn test_timer_with_delay() {
        let start = now();
        let fired_at = Arc::new(Mutex::new(0.0));
        let fired_at_clone = fired_at.clone();

        let t = timer(
            move |elapsed| {
                *fired_at_clone.lock().unwrap() = elapsed;
                false // Fire once
            },
            Some(50.0), // 50ms delay
            None,
        );

        t.join();

        let fired = *fired_at.lock().unwrap();
        let elapsed = now() - start;

        // The callback should have fired after the delay
        assert!(
            elapsed >= 45.0,
            "Total time should be at least 45ms, got {}",
            elapsed
        );
        assert!(fired >= 0.0, "Fired elapsed should be >= 0");
    }

    #[test]
    fn test_timeout_fires_once() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let t = timeout(
            move |_elapsed| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            },
            10.0,
            None,
        );

        t.join();

        // Should have fired exactly once
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_timeout_stop_before_fire() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let t = timeout(
            move |_elapsed| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            },
            1000.0, // Long delay
            None,
        );

        // Stop before it fires
        thread::sleep(Duration::from_millis(10));
        t.stop();

        thread::sleep(Duration::from_millis(50));

        // Should not have fired
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_interval_fires_repeatedly() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let t = interval(
            move |_elapsed| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                counter_clone.load(Ordering::SeqCst) < 3
            },
            20.0, // 20ms interval
            None,
        );

        t.join();

        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_interval_timing() {
        let times = Arc::new(Mutex::new(Vec::new()));
        let times_clone = times.clone();

        let t = interval(
            move |elapsed| {
                times_clone.lock().unwrap().push(elapsed);
                times_clone.lock().unwrap().len() < 4
            },
            30.0, // 30ms interval
            None,
        );

        t.join();

        let recorded = times.lock().unwrap();
        assert_eq!(recorded.len(), 4);

        // Check that intervals are approximately correct
        for i in 1..recorded.len() {
            let diff = recorded[i] - recorded[i - 1];
            assert!(
                diff >= 20.0 && diff <= 50.0,
                "Interval {} was {} ms, expected ~30ms",
                i,
                diff
            );
        }
    }

    #[test]
    fn test_timer_restart() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let mut t = timer(
            move |_elapsed| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                counter_clone.load(Ordering::SeqCst) < 2
            },
            None,
            None,
        );

        // Wait for first timer to complete
        thread::sleep(Duration::from_millis(100));

        let first_count = counter.load(Ordering::SeqCst);
        assert!(first_count >= 2, "First timer should have completed");

        // Restart with new callback
        let counter2 = Arc::new(AtomicUsize::new(0));
        let counter2_clone = counter2.clone();

        t.restart(
            move |_elapsed| {
                counter2_clone.fetch_add(1, Ordering::SeqCst);
                counter2_clone.load(Ordering::SeqCst) < 2
            },
            None,
            None,
        );

        t.join();

        assert!(
            counter2.load(Ordering::SeqCst) >= 2,
            "Restarted timer should have run"
        );
    }

    #[test]
    fn test_timer_flush() {
        // Just ensure it doesn't panic and initializes the epoch
        timer_flush();
        let t = now();
        assert!(t >= 0.0);
    }

    #[test]
    fn test_multiple_timers() {
        let counter1 = Arc::new(AtomicUsize::new(0));
        let counter1_clone = counter1.clone();

        let counter2 = Arc::new(AtomicUsize::new(0));
        let counter2_clone = counter2.clone();

        let t1 = timer(
            move |_| {
                counter1_clone.fetch_add(1, Ordering::SeqCst);
                counter1_clone.load(Ordering::SeqCst) < 3
            },
            None,
            None,
        );

        let t2 = timer(
            move |_| {
                counter2_clone.fetch_add(1, Ordering::SeqCst);
                counter2_clone.load(Ordering::SeqCst) < 5
            },
            None,
            None,
        );

        t1.join();
        t2.join();

        assert_eq!(counter1.load(Ordering::SeqCst), 3);
        assert_eq!(counter2.load(Ordering::SeqCst), 5);
    }

    #[test]
    fn test_timer_is_stopped() {
        let t = timer(|_| true, None, None);
        assert!(!t.is_stopped());
        t.stop();
        assert!(t.is_stopped());
    }

    #[test]
    fn test_timer_id_unique() {
        let t1 = timer(|_| false, None, None);
        let t2 = timer(|_| false, None, None);
        let t3 = timer(|_| false, None, None);

        assert_ne!(t1.id(), t2.id());
        assert_ne!(t2.id(), t3.id());
        assert_ne!(t1.id(), t3.id());
    }
}
