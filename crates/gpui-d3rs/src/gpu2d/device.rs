//! Shared wgpu device management
//!
//! Provides a global singleton for the wgpu device and queue to avoid
//! creating multiple GPU contexts when multiple charts exist.

use std::sync::Arc;
#[cfg(not(target_family = "wasm"))]
use std::sync::LazyLock;

/// Global GPU context shared across all Chart2D instances
pub struct Gpu2DContext {
    /// The wgpu device
    pub device: Arc<wgpu::Device>,
    /// The wgpu queue for submitting commands
    pub queue: Arc<wgpu::Queue>,
    /// Keep the instance alive to prevent TLS access-after-destruction on drop
    _instance: wgpu::Instance,
}

impl Gpu2DContext {
    /// Get the global GPU context singleton
    ///
    /// This lazily initializes the wgpu device on first access.
    pub fn global() -> &'static Self {
        Self::try_global().unwrap_or_else(|err| panic!("{err}"))
    }

    /// Try to get the global GPU context singleton.
    ///
    /// Missing adapters are stored as an error instead of panicking inside the
    /// static initializer, so tests and callers that tolerate missing GPUs do
    /// not poison the process-global context.
    #[cfg(not(target_family = "wasm"))]
    pub fn try_global() -> Result<&'static Self, &'static str> {
        static CONTEXT: LazyLock<Result<Gpu2DContext, String>> = LazyLock::new(|| {
            let (instance, device, queue) = pollster::block_on(create_device())?;
            Ok(Gpu2DContext {
                device: Arc::new(device),
                queue: Arc::new(queue),
                _instance: instance,
            })
        });
        match &*CONTEXT {
            Ok(context) => Ok(context),
            Err(err) => Err(err.as_str()),
        }
    }

    /// Try to get the global GPU context singleton (wasm variant).
    ///
    /// wgpu's WebGPU backend types are `!Send`/`!Sync` (they wrap `JsValue`),
    /// so a process-wide `static LazyLock` does not compile on wasm. The
    /// browser app runs on the single main thread, so the singleton lives in a
    /// `thread_local!` instead; the initialized context is intentionally
    /// leaked once to hand out the same `&'static Self` as the native path.
    ///
    /// Device creation is async: the first call kicks `create_device().await`
    /// via `wasm_bindgen_futures::spawn_local` and returns
    /// `Err("GPU device is initializing")` until it resolves. `pollster` is
    /// never used here — blocking the browser main thread throws
    /// `RuntimeError: Atomics.wait cannot be called in this context`.
    #[cfg(target_family = "wasm")]
    pub fn try_global() -> Result<&'static Self, &'static str> {
        wasm_state::kick();
        wasm_state::STATE.with(|state| match &*state.borrow() {
            wasm_state::State::Ready(context) => Ok(*context),
            wasm_state::State::Uninit | wasm_state::State::Initializing => {
                Err("GPU device is initializing")
            }
            // Leak the message so the cached error can satisfy 'static too.
            wasm_state::State::Failed(err) => Err(&*Box::leak(err.clone().into_boxed_str())),
        })
    }

    /// Returns true once the async device initialization has completed (wasm only).
    #[cfg(target_family = "wasm")]
    pub fn is_ready() -> bool {
        wasm_state::STATE.with(|state| matches!(*state.borrow(), wasm_state::State::Ready(_)))
    }

    /// Returns true when device initialization has permanently failed, so
    /// callers can stop scheduling repaints (wasm only).
    #[cfg(target_family = "wasm")]
    pub fn init_failed() -> bool {
        wasm_state::STATE.with(|state| matches!(*state.borrow(), wasm_state::State::Failed(_)))
    }

    /// Get a clone of the device Arc
    pub fn device(&self) -> Arc<wgpu::Device> {
        self.device.clone()
    }

    /// Get a clone of the queue Arc
    pub fn queue(&self) -> Arc<wgpu::Queue> {
        self.queue.clone()
    }
}

/// Async device-init state machine (wasm only): `create_device().await` is
/// driven by the browser event loop instead of blocking the main thread.
#[cfg(target_family = "wasm")]
mod wasm_state {
    use super::{Gpu2DContext, create_device};
    use std::cell::RefCell;

    pub enum State {
        Uninit,
        Initializing,
        Ready(&'static Gpu2DContext),
        Failed(String),
    }

    thread_local! {
        pub static STATE: RefCell<State> = const { RefCell::new(State::Uninit) };
    }

    /// Kick async init if Uninit; idempotent.
    pub fn kick() {
        STATE.with(|s| {
            let should_start = matches!(*s.borrow(), State::Uninit);
            if should_start {
                *s.borrow_mut() = State::Initializing;
                wasm_bindgen_futures::spawn_local(async {
                    let next = match create_device().await {
                        Ok((instance, device, queue)) => {
                            State::Ready(&*Box::leak(Box::new(Gpu2DContext {
                                device: std::sync::Arc::new(device),
                                queue: std::sync::Arc::new(queue),
                                _instance: instance,
                            })))
                        }
                        Err(err) => State::Failed(err),
                    };
                    STATE.with(|s| *s.borrow_mut() = next);
                });
            }
        });
    }
}

/// Create a new wgpu instance, device and queue
async fn create_device() -> Result<(wgpu::Instance, wgpu::Device, wgpu::Queue), String> {
    // Native probes every backend; the browser target is WebGPU-only.
    #[cfg(not(target_family = "wasm"))]
    let backends = wgpu::Backends::all();
    #[cfg(target_family = "wasm")]
    let backends = wgpu::Backends::BROWSER_WEBGPU;

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .map_err(|err| format!("Failed to find suitable GPU adapter: {err:?}"))?;

    // WebGPU exposes a smaller limit set than native defaults.
    #[cfg(not(target_family = "wasm"))]
    let required_limits = wgpu::Limits::default();
    #[cfg(target_family = "wasm")]
    let required_limits = wgpu::Limits::downlevel_defaults();

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("Chart2D Device"),
            required_features: wgpu::Features::empty(),
            required_limits,
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::default(),
        })
        .await
        .map_err(|err| format!("Failed to create device: {err:?}"))?;

    Ok((instance, device, queue))
}
