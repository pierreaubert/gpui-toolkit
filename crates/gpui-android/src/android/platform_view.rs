//! Android platform view implementation.
//!
//! Embeds native Android `View` instances in the GPUI render tree using
//! hybrid composition. Native views are added to the NativeActivity's
//! content view hierarchy and positioned to match GPUI element bounds.

use crate::platform_view::{
    PlatformView, PlatformViewBounds, PlatformViewFactory, PlatformViewId, PlatformViewParams,
};
use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
};

/// Encode creation parameters for the Java `GpuiPlatformView` factory.
///
/// Entries are sorted and delimited with `|`; a backslash escapes itself, the
/// key/value separator (`=`), and the entry separator (`|`). This retains the
/// legacy representation for ordinary values while making arbitrary strings
/// lossless for factories that parse the documented escape sequences.
fn encode_creation_params(params: &HashMap<String, String>) -> String {
    let mut entries: Vec<_> = params.iter().collect();
    entries.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));
    entries
        .into_iter()
        .map(|(key, value)| {
            format!(
                "{}={}",
                escape_creation_param(key),
                escape_creation_param(value)
            )
        })
        .collect::<Vec<_>>()
        .join("|")
}

/// Java helper class for platform-view operations.
///
/// Resolved through the cached JNI lookup (`find_cached_app_class`) so
/// per-frame bounds updates don't pay the Activity-classloader cost.
const PLATFORM_VIEW_HELPER: &str = "dev.gpui.mobile.GpuiPlatformView";

fn escape_creation_param(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('=', "\\=")
}

/// Android implementation of a platform view.
///
/// In hybrid composition mode, this creates a native Android `View` and
/// adds it to the Activity's window. The view is positioned absolutely
/// to match the GPUI element's screen coordinates.
pub struct AndroidPlatformView {
    id: PlatformViewId,
    view_type: String,
    /// JNI global reference to the Android View object.
    /// 0 means no view (disposed or not yet created).
    #[allow(dead_code)]
    java_view_ref: std::sync::atomic::AtomicUsize,
    disposed: AtomicBool,
    bounds: std::sync::Mutex<PlatformViewBounds>,
    /// Last visibility sent to Java. `set_visible` skips the JNI round-trip
    /// when the state is unchanged (layout passes repeat the call often).
    visible: AtomicBool,
}

impl AndroidPlatformView {
    /// Create a new Android platform view.
    ///
    /// This allocates the Rust-side state. The actual Android `View` is
    /// created via JNI in a separate step.
    pub fn new(view_type: &str, params: &PlatformViewParams) -> Result<Self, String> {
        let id = PlatformViewId::next();

        let view = Self {
            id,
            view_type: view_type.to_string(),
            java_view_ref: std::sync::atomic::AtomicUsize::new(0),
            disposed: AtomicBool::new(false),
            bounds: std::sync::Mutex::new(params.bounds),
            // New views start visible, matching the registry's tracking.
            visible: AtomicBool::new(true),
        };

        // Create the native Android view via JNI
        view.create_native_view(params)?;

        Ok(view)
    }

    /// Create the native Android View via JNI.
    fn create_native_view(&self, params: &PlatformViewParams) -> Result<(), String> {
        use super::jni::{
            JniExt, activity, find_cached_app_class, invalidate_app_class_cache, with_env,
        };
        use jni::objects::JValue;

        with_env(|env| {
            let helper_class = find_cached_app_class(env, PLATFORM_VIEW_HELPER)?;
            let view_type_jstr = env.new_string(&self.view_type).map_err(|e| e.to_string())?;
            let view_id = self.id.0 as i64;

            let bounds = self.bounds.lock().unwrap();
            let x = bounds.x;
            let y = bounds.y;
            let width = bounds.width;
            let height = bounds.height;
            drop(bounds);

            let creation_params_str = encode_creation_params(&params.creation_params);
            let creation_params_jstr = env.new_string(&creation_params_str).e()?;

            let act = activity(env)?;

            let result = env
                .call_static_method(
                    &helper_class,
                    jni::jni_str!("createView"),
                    jni::jni_sig!(
                        "(Landroid/app/Activity;Ljava/lang/String;JFFFFLjava/lang/String;)Z"
                    ),
                    &[
                        JValue::Object(&act),
                        JValue::Object(&view_type_jstr),
                        JValue::Long(view_id),
                        JValue::Float(x),
                        JValue::Float(y),
                        JValue::Float(width),
                        JValue::Float(height),
                        JValue::Object(&creation_params_jstr),
                    ],
                )
                .map_err(|e| {
                    env.exception_clear();
                    invalidate_app_class_cache(PLATFORM_VIEW_HELPER);
                    format!("createView JNI call failed: {}", e)
                })?;

            let success = result.z().unwrap_or(false);
            if !success {
                return Err("GpuiPlatformView.createView returned false".to_string());
            }

            log::info!(
                "AndroidPlatformView: created native view id={} type='{}'",
                self.id,
                self.view_type
            );
            Ok(())
        })
    }

    /// Update the native view's bounds via JNI.
    fn update_native_bounds(&self, bounds: &PlatformViewBounds) {
        use super::jni::{find_cached_app_class, invalidate_app_class_cache, with_env};
        use jni::objects::JValue;

        let view_id = self.id.0 as i64;
        let _ = with_env(|env| {
            let helper_class = find_cached_app_class(env, PLATFORM_VIEW_HELPER)?;
            let _ = env
                .call_static_method(
                    &helper_class,
                    jni::jni_str!("setBounds"),
                    jni::jni_sig!("(JFFFF)V"),
                    &[
                        JValue::Long(view_id),
                        JValue::Float(bounds.x),
                        JValue::Float(bounds.y),
                        JValue::Float(bounds.width),
                        JValue::Float(bounds.height),
                    ],
                )
                .map_err(|e| {
                    env.exception_clear();
                    invalidate_app_class_cache(PLATFORM_VIEW_HELPER);
                    format!("setBounds failed: {}", e)
                })?;
            Ok(())
        });
    }

    /// Update the native view's visibility via JNI.
    fn update_native_visibility(&self, visible: bool) {
        use super::jni::{find_cached_app_class, invalidate_app_class_cache, with_env};
        use jni::objects::JValue;

        let view_id = self.id.0 as i64;
        let _ = with_env(|env| {
            let helper_class = find_cached_app_class(env, PLATFORM_VIEW_HELPER)?;
            let _ = env
                .call_static_method(
                    &helper_class,
                    jni::jni_str!("setVisible"),
                    jni::jni_sig!("(JZ)V"),
                    &[JValue::Long(view_id), JValue::Bool(visible)],
                )
                .map_err(|e| {
                    env.exception_clear();
                    invalidate_app_class_cache(PLATFORM_VIEW_HELPER);
                    format!("setVisible failed: {}", e)
                })?;
            Ok(())
        });
    }

    /// Remove and dispose the native view via JNI.
    fn dispose_native_view(&self) {
        use super::jni::{find_cached_app_class, invalidate_app_class_cache, with_env};
        use jni::objects::JValue;

        let view_id = self.id.0 as i64;
        let _ = with_env(|env| {
            let helper_class = find_cached_app_class(env, PLATFORM_VIEW_HELPER)?;
            let _ = env
                .call_static_method(
                    &helper_class,
                    jni::jni_str!("disposeView"),
                    jni::jni_sig!("(J)V"),
                    &[JValue::Long(view_id)],
                )
                .map_err(|e| {
                    env.exception_clear();
                    invalidate_app_class_cache(PLATFORM_VIEW_HELPER);
                    format!("disposeView failed: {}", e)
                })?;
            Ok(())
        });
    }
}

impl PlatformView for AndroidPlatformView {
    fn id(&self) -> PlatformViewId {
        self.id
    }

    fn view_type(&self) -> &str {
        &self.view_type
    }

    fn set_bounds(&self, bounds: PlatformViewBounds) {
        if self.disposed.load(Ordering::Relaxed) {
            return;
        }
        // Coalesce: layout emits bounds every frame during scroll; skip the
        // JNI round-trip when nothing changed.
        {
            let mut cached = self.bounds.lock().unwrap();
            if *cached == bounds {
                return;
            }
            *cached = bounds;
        }
        self.update_native_bounds(&bounds);
    }

    fn set_visible(&self, visible: bool) {
        if self.disposed.load(Ordering::Relaxed) {
            return;
        }
        // Coalesce redundant transitions: layout passes repeat the call with
        // the same state, and each one costs a JNI round-trip.
        if self.visible.swap(visible, Ordering::Relaxed) == visible {
            return;
        }
        self.update_native_visibility(visible);
    }

    fn set_z_index(&self, z_index: i32) {
        if self.disposed.load(Ordering::Relaxed) {
            return;
        }
        use super::jni::{find_cached_app_class, invalidate_app_class_cache, with_env};
        use jni::objects::JValue;

        let view_id = self.id.0 as i64;
        let _ = with_env(|env| {
            let helper_class = find_cached_app_class(env, PLATFORM_VIEW_HELPER)?;
            let _ = env
                .call_static_method(
                    &helper_class,
                    jni::jni_str!("setZIndex"),
                    jni::jni_sig!("(JI)V"),
                    &[JValue::Long(view_id), JValue::Int(z_index)],
                )
                .map_err(|e| {
                    env.exception_clear();
                    invalidate_app_class_cache(PLATFORM_VIEW_HELPER);
                    format!("setZIndex failed: {}", e)
                })?;
            Ok(())
        });
    }

    fn dispose(&self) {
        if self.disposed.swap(true, Ordering::Relaxed) {
            return; // Already disposed
        }
        self.dispose_native_view();
        log::info!("AndroidPlatformView: disposed view {}", self.id);
    }

    fn is_disposed(&self) -> bool {
        self.disposed.load(Ordering::Relaxed)
    }
}

/// Generic Android platform view factory.
///
/// The Java-side `GpuiPlatformView.createView` implementation receives
/// creation parameters as sorted `key=value` pairs joined by `|`. A backslash
/// escapes itself, `=`, and `|`; factories must unescape those sequences
/// before using the keys or values.
///
/// Delegates view creation to the Java-side `GpuiPlatformView` helper,
/// which handles the actual `View` instantiation based on the type string.
pub struct AndroidPlatformViewFactory {
    view_type: String,
}

impl AndroidPlatformViewFactory {
    pub fn new(view_type: &str) -> Self {
        Self {
            view_type: view_type.to_string(),
        }
    }
}

impl PlatformViewFactory for AndroidPlatformViewFactory {
    fn create(&self, params: &PlatformViewParams) -> Result<Box<dyn PlatformView>, String> {
        let view = AndroidPlatformView::new(&self.view_type, params)?;
        Ok(Box::new(view))
    }

    fn view_type(&self) -> &str {
        &self.view_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation_params_escape_separators_and_are_sorted() {
        let params = HashMap::from([
            ("path".to_string(), r"C:\temp".to_string()),
            ("a=key".to_string(), "value|one".to_string()),
        ]);

        assert_eq!(
            encode_creation_params(&params),
            r"a\=key=value\|one|path=C:\\temp"
        );
    }
}
