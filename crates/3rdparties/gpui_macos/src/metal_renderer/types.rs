use parking_lot::Mutex;
use std :: { sync :: Arc } ;
use super::MetalRenderer;
use super::instance_buffer_pool::InstanceBufferPool;

pub(crate) type PointF = gpui::Point<f32>;

pub(crate) type Context = Arc<Mutex<InstanceBufferPool>>;

pub(crate) type Renderer = MetalRenderer;

pub(crate) struct InstanceBuffer {
    pub(super) metal_buffer: metal::Buffer,
    pub(super) size: usize,
}

