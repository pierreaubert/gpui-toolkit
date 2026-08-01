//! # d3-timer - Animation Timing Module
//!
//! This module provides efficient animation timing utilities, inspired by
//! D3.js's d3-timer module. Timers share one scheduler thread instead of
//! creating an operating-system thread per timer.
//!
//! Applications with a UI event loop can install a dispatcher with
//! [`set_ui_dispatcher`]. Timer callbacks are then enqueued on that loop and
//! the scheduler remains responsible only for deadlines and cancellation.

use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Condvar, Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant};

mod interval;
mod misc;
#[cfg(test)]
mod tests;
mod timeout;
mod types;

pub use interval::*;
pub use misc::*;
pub use timeout::*;
pub use types::*;

use misc::TIMER_ID_COUNTER;

type TimerCallback = Arc<Mutex<Box<dyn FnMut(f64) -> bool + Send>>>;
type Completion = Arc<(Mutex<bool>, Condvar)>;

/// A function that transfers a timer callback to an application's UI queue.
///
/// The dispatcher receives a one-shot job. It must enqueue the job and return
/// quickly; the job sends the callback result back to the shared timer
/// scheduler after it has run on the UI thread.
pub type TimerDispatcher = Arc<dyn Fn(Box<dyn FnOnce() + Send>) + Send + Sync + 'static>;

static UI_DISPATCHER: OnceLock<RwLock<Option<TimerDispatcher>>> = OnceLock::new();

/// Register the UI-thread callback dispatcher used by all subsequently fired
/// timers.
///
/// Passing a closure that forwards its argument to `cx.spawn`, an event queue,
/// or an equivalent UI executor makes timer callbacks run on that UI thread.
/// Call [`clear_ui_dispatcher`] when the UI loop is torn down. If no dispatcher
/// is installed, callbacks run on the shared scheduler thread.
pub fn set_ui_dispatcher<F>(dispatcher: F)
where
    F: Fn(Box<dyn FnOnce() + Send>) + Send + Sync + 'static,
{
    *UI_DISPATCHER
        .get_or_init(|| RwLock::new(None))
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Arc::new(dispatcher));
}

/// Remove the process-wide UI-thread callback dispatcher.
pub fn clear_ui_dispatcher() {
    if let Some(dispatcher) = UI_DISPATCHER.get() {
        *dispatcher
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
    }
}

fn ui_dispatcher() -> Option<TimerDispatcher> {
    UI_DISPATCHER.get().and_then(|dispatcher| {
        dispatcher
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    })
}

#[derive(Clone)]
struct TaskSpec {
    id: u64,
    callback: TimerCallback,
    stopped: Arc<std::sync::atomic::AtomicBool>,
    completion: Completion,
    start_time: f64,
    period: Duration,
}

struct ScheduledTask {
    task: TaskSpec,
    next_tick: Instant,
}

enum SchedulerCommand {
    Schedule(ScheduledTask),
    Cancel(u64),
    CallbackResult { task: TaskSpec, keep_running: bool },
}

struct Scheduler {
    sender: Sender<SchedulerCommand>,
}

static SCHEDULER: OnceLock<Scheduler> = OnceLock::new();

fn scheduler() -> &'static Scheduler {
    SCHEDULER.get_or_init(|| {
        let (sender, receiver) = mpsc::channel();
        let worker_sender = sender.clone();
        std::thread::Builder::new()
            .name("d3-timer-scheduler".to_string())
            .spawn(move || scheduler_loop(receiver, worker_sender))
            .expect("failed to start d3 timer scheduler");
        Scheduler { sender }
    })
}

fn scheduler_loop(receiver: Receiver<SchedulerCommand>, sender: Sender<SchedulerCommand>) {
    let mut tasks = HashMap::<u64, ScheduledTask>::new();
    let mut due_ids = Vec::new();

    loop {
        let wait = tasks
            .values()
            .map(|task| task.next_tick.saturating_duration_since(Instant::now()))
            .min()
            .unwrap_or(Duration::from_secs(3600));

        match receiver.recv_timeout(wait) {
            Ok(command) => handle_command(command, &mut tasks),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        let now = Instant::now();
        due_ids.clear();
        due_ids.extend(
            tasks
                .iter()
                .filter_map(|(&id, task)| (task.next_tick <= now).then_some(id)),
        );

        for id in due_ids.drain(..) {
            let Some(task) = tasks.remove(&id) else {
                continue;
            };
            dispatch_due_task(task, &sender);
        }
    }

    for (_, task) in tasks {
        finish_task(&task.task);
    }
}

fn handle_command(command: SchedulerCommand, tasks: &mut HashMap<u64, ScheduledTask>) {
    match command {
        SchedulerCommand::Schedule(task) => {
            tasks.insert(task.task.id, task);
        }
        SchedulerCommand::Cancel(id) => {
            if let Some(task) = tasks.remove(&id) {
                finish_task(&task.task);
            }
        }
        SchedulerCommand::CallbackResult { task, keep_running } => {
            if keep_running && !task.stopped.load(std::sync::atomic::Ordering::Acquire) {
                tasks.insert(
                    task.id,
                    ScheduledTask {
                        next_tick: Instant::now() + task.period,
                        task,
                    },
                );
            } else {
                task.stopped
                    .store(true, std::sync::atomic::Ordering::Release);
                finish_task(&task);
            }
        }
    }
}

fn dispatch_due_task(task: ScheduledTask, sender: &Sender<SchedulerCommand>) {
    let task = task.task;
    if task.stopped.load(std::sync::atomic::Ordering::Acquire) {
        finish_task(&task);
        return;
    }

    if let Some(dispatcher) = ui_dispatcher() {
        let sender = sender.clone();
        let callback_task = task.clone();
        dispatcher(Box::new(move || {
            let keep_running = if callback_task
                .stopped
                .load(std::sync::atomic::Ordering::Acquire)
            {
                false
            } else {
                invoke_callback(&callback_task)
            };
            let _ = sender.send(SchedulerCommand::CallbackResult {
                task: callback_task,
                keep_running,
            });
        }));
    } else {
        let keep_running = invoke_callback(&task);
        if keep_running && !task.stopped.load(std::sync::atomic::Ordering::Acquire) {
            scheduler()
                .sender
                .send(SchedulerCommand::Schedule(ScheduledTask {
                    next_tick: Instant::now() + task.period,
                    task,
                }))
                .ok();
        } else {
            task.stopped
                .store(true, std::sync::atomic::Ordering::Release);
            finish_task(&task);
        }
    }
}

fn invoke_callback(task: &TaskSpec) -> bool {
    let elapsed = (now() - task.start_time).max(0.0);
    catch_unwind(AssertUnwindSafe(|| {
        let mut callback = task
            .callback
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        callback(elapsed)
    }))
    .unwrap_or(false)
}

fn finish_task(task: &TaskSpec) {
    let (lock, condvar) = &*task.completion;
    let mut done = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    *done = true;
    condvar.notify_all();
}

fn completion() -> Completion {
    Arc::new((Mutex::new(false), Condvar::new()))
}

fn period_from_ms(milliseconds: f64, default: Duration) -> Duration {
    if milliseconds.is_finite() && milliseconds > 0.0 {
        Duration::from_secs_f64(milliseconds / 1000.0).max(Duration::from_millis(1))
    } else {
        default
    }
}

/// A timer that invokes a callback repeatedly.
///
/// The callback receives the elapsed time since the timer was started. If the
/// callback returns `false`, the timer stops. All timers share one scheduler
/// thread; callback execution can be moved to a UI thread with
/// [`set_ui_dispatcher`].
#[derive(Clone)]
#[allow(clippy::type_complexity)]
pub struct Timer {
    id: u64,
    callback: TimerCallback,
    delay: f64,
    start_time: f64,
    stopped: Arc<std::sync::atomic::AtomicBool>,
    completion: Completion,
}

impl std::fmt::Debug for Timer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Timer")
            .field("id", &self.id)
            .field("delay", &self.delay)
            .field("start_time", &self.start_time)
            .field(
                "stopped",
                &self.stopped.load(std::sync::atomic::Ordering::Acquire),
            )
            .finish()
    }
}

impl Timer {
    /// Creates a new timer with the default approximately-60fps period.
    pub fn new<F>(callback: F, delay: Option<f64>, time: Option<f64>) -> Self
    where
        F: FnMut(f64) -> bool + Send + 'static,
    {
        Self::with_period(callback, delay, time, Duration::from_millis(16))
    }

    pub(super) fn with_period<F>(
        callback: F,
        delay: Option<f64>,
        time: Option<f64>,
        period: Duration,
    ) -> Self
    where
        F: FnMut(f64) -> bool + Send + 'static,
    {
        let id = TIMER_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let delay = delay.unwrap_or(0.0);
        let start_time = time.unwrap_or_else(now);
        let stopped = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let completion = completion();
        let callback = Arc::new(Mutex::new(
            Box::new(callback) as Box<dyn FnMut(f64) -> bool + Send>
        ));
        let timer = Self {
            id,
            callback: callback.clone(),
            delay,
            start_time,
            stopped: stopped.clone(),
            completion: completion.clone(),
        };

        let next_tick = Instant::now() + period_from_ms(delay, Duration::ZERO);
        scheduler()
            .sender
            .send(SchedulerCommand::Schedule(ScheduledTask {
                next_tick,
                task: TaskSpec {
                    id,
                    callback,
                    stopped,
                    completion,
                    start_time,
                    period,
                },
            }))
            .expect("d3 timer scheduler has stopped");
        timer
    }

    /// Stops the timer. Stopping is idempotent and does not block.
    pub fn stop(&self) {
        self.stopped
            .store(true, std::sync::atomic::Ordering::Release);
        finish_task(&TaskSpec {
            id: self.id,
            callback: self.callback.clone(),
            stopped: self.stopped.clone(),
            completion: self.completion.clone(),
            start_time: self.start_time,
            period: Duration::ZERO,
        });
        let _ = scheduler().sender.send(SchedulerCommand::Cancel(self.id));
    }

    /// Returns true if the timer has been stopped or completed.
    pub fn is_stopped(&self) -> bool {
        self.stopped.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Restarts the timer with a new callback.
    pub fn restart<F>(&mut self, callback: F, delay: Option<f64>, time: Option<f64>)
    where
        F: FnMut(f64) -> bool + Send + 'static,
    {
        self.stop();
        let new_timer = Timer::new(callback, delay, time);
        *self = new_timer;
    }

    /// Returns the timer's unique ID.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Returns the delay before the first callback.
    pub fn delay(&self) -> f64 {
        self.delay
    }

    /// Returns the start time used for elapsed calculation.
    pub fn start_time(&self) -> f64 {
        self.start_time
    }

    /// Waits for the timer to complete. This is useful in tests or when an
    /// application must ensure all callbacks have finished.
    pub fn join(self) {
        let (lock, condvar) = &*self.completion;
        let mut done = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        while !*done {
            done = condvar
                .wait(done)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }
}

/// Creates a new timer that invokes a callback repeatedly.
pub fn timer<F>(callback: F, delay: Option<f64>, time: Option<f64>) -> Timer
where
    F: FnMut(f64) -> bool + Send + 'static,
{
    Timer::new(callback, delay, time)
}
