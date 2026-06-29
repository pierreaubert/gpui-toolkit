use super::Timer;
use std::sync::Mutex;

/// A one-shot timer that fires once after a delay.
///
/// Unlike `Timer`, this fires exactly once and then automatically stops.
#[derive(Clone)]
pub struct Timeout {
    pub(super) inner: Timer,
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
