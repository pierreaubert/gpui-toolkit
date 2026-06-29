//! iOS Window implementation using UIWindow and UIViewController.
//!
//! iOS windows are fundamentally different from desktop windows:
//! - Always fullscreen (or split-screen on iPad)
//! - No title bar or window chrome
//! - Touch-based input
//! - Safe area insets for notch/home indicator
//!
//! The window is backed by a UIWindow containing a UIViewController
//! whose view hosts a CAMetalLayer. Rendering is performed by
//! `gpui_wgpu::WgpuRenderer` which drives wgpu over the Metal backend.

use super::IosDisplay;
use super::events::*;
use crate::momentum::{MomentumScroller, VelocityTracker};
use crate::native::{DynamicTypeCategory, IosSceneMetrics, SafeAreaInsets, SizeClass};
use crate::platform_view::NativePlatformViewHost;
use gpui::{
    AnyWindowHandle, AtlasKey, AtlasTextureId, AtlasTextureKind, AtlasTile, Bounds, Capslock,
    DevicePixels, DispatchEventResult, GpuSpecs, Modifiers, Pixels, PlatformAtlas, PlatformDisplay,
    PlatformInput, PlatformInputHandler, PlatformWindow, Point, PromptButton, PromptLevel,
    RequestFrameOptions, Scene, Size, TileId, WindowAppearance, WindowBackgroundAppearance,
    WindowBounds, WindowControlArea, WindowParams, point, px, size,
};
use gpui_wgpu::{WgpuContext, WgpuRenderer, WgpuSurfaceConfig, wgpu};
use objc::{
    Message, class,
    declare::ClassDecl,
    msg_send,
    runtime::{BOOL, Class, Object, Sel, YES},
    sel, sel_impl,
};
use parking_lot::Mutex;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, UiKitDisplayHandle, UiKitWindowHandle};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    ffi::{CStr, c_void},
    ptr::{self, NonNull},
    rc::Rc,
    sync::Arc,
};

mod accessibility;
mod consts;
mod fallback_atlas;
mod handle;
mod ios_raw_handles;
mod ios_window;
mod misc;
mod register;
mod types;

pub use consts::*;
pub use ios_raw_handles::*;
pub use ios_window::*;
pub use misc::*;
