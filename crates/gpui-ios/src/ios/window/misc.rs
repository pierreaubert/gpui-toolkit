use crate::native::{DynamicTypeCategory, IosSceneMetrics, SafeAreaInsets, SizeClass};
use objc::{class, msg_send, runtime::Object, sel, sel_impl};
use std::ffi::CStr;

#[link(name = "UIKit", kind = "framework")]
unsafe extern "C" {
    pub(super) static UIAccessibilityTraitButton: u64;
    pub(super) static UIAccessibilityTraitLink: u64;
    pub(super) static UIAccessibilityTraitHeader: u64;
    pub(super) static UIAccessibilityTraitSearchField: u64;
    pub(super) static UIAccessibilityTraitImage: u64;
    pub(super) static UIAccessibilityTraitSelected: u64;
    pub(super) static UIAccessibilityTraitStaticText: u64;
    pub(super) static UIAccessibilityTraitNotEnabled: u64;
    pub(super) static UIAccessibilityTraitUpdatesFrequently: u64;
    pub(super) static UIAccessibilityTraitAdjustable: u64;

    pub(super) static UIAccessibilityLayoutChangedNotification: u32;
    pub(super) static UIAccessibilityAnnouncementNotification: u32;

    pub(super) fn UIAccessibilityPostNotification(notification: u32, argument: *mut Object);
}

pub(super) fn size_class_from_uikit(value: i64) -> SizeClass {
    match value {
        1 => SizeClass::Compact,
        2 => SizeClass::Regular,
        _ => SizeClass::Unspecified,
    }
}

pub(super) fn dynamic_type_from_name(name: &str) -> DynamicTypeCategory {
    match name {
        "UICTContentSizeCategoryXS" | "UIContentSizeCategoryExtraSmall" => {
            DynamicTypeCategory::ExtraSmall
        }
        "UICTContentSizeCategoryS" | "UIContentSizeCategorySmall" => DynamicTypeCategory::Small,
        "UICTContentSizeCategoryL" | "UIContentSizeCategoryLarge" => DynamicTypeCategory::Large,
        "UICTContentSizeCategoryXL" | "UIContentSizeCategoryExtraLarge" => {
            DynamicTypeCategory::ExtraLarge
        }
        "UICTContentSizeCategoryXXL" | "UIContentSizeCategoryExtraExtraLarge" => {
            DynamicTypeCategory::ExtraExtraLarge
        }
        "UICTContentSizeCategoryXXXL" | "UIContentSizeCategoryExtraExtraExtraLarge" => {
            DynamicTypeCategory::ExtraExtraExtraLarge
        }
        "UICTContentSizeCategoryAccessibilityM" | "UIContentSizeCategoryAccessibilityMedium" => {
            DynamicTypeCategory::AccessibilityMedium
        }
        "UICTContentSizeCategoryAccessibilityL" | "UIContentSizeCategoryAccessibilityLarge" => {
            DynamicTypeCategory::AccessibilityLarge
        }
        "UICTContentSizeCategoryAccessibilityXL"
        | "UIContentSizeCategoryAccessibilityExtraLarge" => {
            DynamicTypeCategory::AccessibilityExtraLarge
        }
        "UICTContentSizeCategoryAccessibilityXXL"
        | "UIContentSizeCategoryAccessibilityExtraExtraLarge" => {
            DynamicTypeCategory::AccessibilityExtraExtraLarge
        }
        "UICTContentSizeCategoryAccessibilityXXXL"
        | "UIContentSizeCategoryAccessibilityExtraExtraExtraLarge" => {
            DynamicTypeCategory::AccessibilityExtraExtraExtraLarge
        }
        _ => DynamicTypeCategory::Medium,
    }
}

pub(super) fn ns_string_to_string(value: *mut Object) -> Option<String> {
    if value.is_null() {
        return None;
    }
    let utf8: *const i8 = unsafe { msg_send![value, UTF8String] };
    if utf8.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(utf8) }
        .to_str()
        .ok()
        .map(str::to_string)
}

pub(super) fn dispatch_accessibility_element_action(
    element: &Object,
    action: crate::accessibility::IosAccessibilityAction,
) -> bool {
    unsafe {
        // SAFETY: UIKit invokes these methods on a live GPUIAccessibilityElement.
        // `accessibilityIdentifier` is an NSString set by `refresh_accessibility`
        // when the element is rebuilt from the current snapshot.
        let identifier: *mut Object = msg_send![element, accessibilityIdentifier];
        let Some(id) = ns_string_to_string(identifier) else {
            return false;
        };
        crate::accessibility::dispatch_accessibility_action(&id, action)
    }
}

pub(super) unsafe fn view_safe_area_insets(view: *mut Object) -> SafeAreaInsets {
    if view.is_null() {
        return SafeAreaInsets::default();
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    struct UIEdgeInsets {
        top: f64,
        left: f64,
        bottom: f64,
        right: f64,
    }

    let insets: UIEdgeInsets = unsafe { msg_send![view, safeAreaInsets] };
    SafeAreaInsets::new(
        insets.top as f32,
        insets.left as f32,
        insets.bottom as f32,
        insets.right as f32,
    )
}

pub(super) unsafe fn query_scene_metrics(
    view: *mut Object,
    fallback_scale: f32,
) -> Option<IosSceneMetrics> {
    if view.is_null() {
        return None;
    }

    let bounds: core_graphics::geometry::CGRect = unsafe { msg_send![view, bounds] };
    let width = bounds.size.width as f32;
    let height = bounds.size.height as f32;
    if !width.is_finite() || width <= 0.0 || !height.is_finite() || height <= 0.0 {
        return None;
    }

    let window: *mut Object = unsafe { msg_send![view, window] };
    let screen: *mut Object = if window.is_null() {
        unsafe { msg_send![class!(UIScreen), mainScreen] }
    } else {
        unsafe { msg_send![window, screen] }
    };
    let scale_factor = if screen.is_null() {
        fallback_scale
    } else {
        let scale: core_graphics::base::CGFloat = unsafe { msg_send![screen, scale] };
        scale as f32
    };

    let trait_collection: *mut Object = unsafe { msg_send![view, traitCollection] };
    let (horizontal_size_class, vertical_size_class, dynamic_type) = if trait_collection.is_null() {
        (
            SizeClass::Unspecified,
            SizeClass::Unspecified,
            DynamicTypeCategory::Medium,
        )
    } else {
        let horizontal: i64 = unsafe { msg_send![trait_collection, horizontalSizeClass] };
        let vertical: i64 = unsafe { msg_send![trait_collection, verticalSizeClass] };
        let category: *mut Object =
            unsafe { msg_send![trait_collection, preferredContentSizeCategory] };
        let dynamic_type = ns_string_to_string(category)
            .as_deref()
            .map(dynamic_type_from_name)
            .unwrap_or_default();
        (
            size_class_from_uikit(horizontal),
            size_class_from_uikit(vertical),
            dynamic_type,
        )
    };

    let metrics = IosSceneMetrics {
        width,
        height,
        scale_factor,
        horizontal_size_class,
        vertical_size_class,
        dynamic_type,
        safe_area: unsafe { view_safe_area_insets(view) },
        keyboard_height: crate::keyboard_height(),
    };

    metrics.validate().ok()?;
    Some(metrics)
}
