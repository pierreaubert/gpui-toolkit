//! Shared wgpu device management
//!
//! Provides a global singleton for the wgpu device and queue to avoid
//! creating multiple GPU contexts when multiple charts exist.

use std::sync::{Arc, LazyLock};

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

    /// Get a clone of the device Arc
    pub fn device(&self) -> Arc<wgpu::Device> {
        self.device.clone()
    }

    /// Get a clone of the queue Arc
    pub fn queue(&self) -> Arc<wgpu::Queue> {
        self.queue.clone()
    }
}

/// Create a new wgpu instance, device and queue
async fn create_device() -> Result<(wgpu::Instance, wgpu::Device, wgpu::Queue), String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
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

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("Chart2D Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::default(),
        })
        .await
        .map_err(|err| format!("Failed to create device: {err:?}"))?;

    Ok((instance, device, queue))
}
