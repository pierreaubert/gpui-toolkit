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
}

impl Gpu2DContext {
    /// Get the global GPU context singleton
    ///
    /// This lazily initializes the wgpu device on first access.
    pub fn global() -> &'static Self {
        static CONTEXT: LazyLock<Gpu2DContext> = LazyLock::new(|| {
            let (device, queue) = pollster::block_on(create_device());
            Gpu2DContext {
                device: Arc::new(device),
                queue: Arc::new(queue),
            }
        });
        &CONTEXT
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

/// Create a new wgpu device and queue
async fn create_device() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .expect("Failed to find suitable GPU adapter");

    adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("Chart2D Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
        })
        .await
        .expect("Failed to create device")
}
