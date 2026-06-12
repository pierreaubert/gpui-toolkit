//! Native iOS integration metadata used by GPUI shells.
//!
//! These types are intentionally platform-agnostic so they can be tested on
//! non-iOS hosts while the Objective-C bridge maps them to UIKit at runtime.

/// UIKit size class for one axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SizeClass {
    Compact,
    Regular,
    #[default]
    Unspecified,
}

/// Layout mode derived from the current usable scene dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IosLayoutMode {
    PortraitLike,
    LandscapeLike,
}

/// Product layout class derived from usable scene metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IosSceneClass {
    Phone,
    IpadFullscreen,
    IpadCompactSplit,
    IpadStageManager,
    ExternalDisplay,
}

/// Dynamic Type category ordered from smallest to largest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum DynamicTypeCategory {
    ExtraSmall,
    Small,
    #[default]
    Medium,
    Large,
    ExtraLarge,
    ExtraExtraLarge,
    ExtraExtraExtraLarge,
    AccessibilityMedium,
    AccessibilityLarge,
    AccessibilityExtraLarge,
    AccessibilityExtraExtraLarge,
    AccessibilityExtraExtraExtraLarge,
}

impl DynamicTypeCategory {
    pub fn scale_factor(self) -> f32 {
        match self {
            Self::ExtraSmall => 0.82,
            Self::Small => 0.9,
            Self::Medium => 1.0,
            Self::Large => 1.08,
            Self::ExtraLarge => 1.18,
            Self::ExtraExtraLarge => 1.28,
            Self::ExtraExtraExtraLarge => 1.4,
            Self::AccessibilityMedium => 1.55,
            Self::AccessibilityLarge => 1.75,
            Self::AccessibilityExtraLarge => 1.95,
            Self::AccessibilityExtraExtraLarge => 2.15,
            Self::AccessibilityExtraExtraExtraLarge => 2.35,
        }
    }

    pub fn is_accessibility_size(self) -> bool {
        self >= Self::AccessibilityMedium
    }
}

/// Insets in UIKit order: top, left, bottom, right.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SafeAreaInsets {
    pub top: f32,
    pub left: f32,
    pub bottom: f32,
    pub right: f32,
}

impl SafeAreaInsets {
    pub const fn new(top: f32, left: f32, bottom: f32, right: f32) -> Self {
        Self {
            top,
            left,
            bottom,
            right,
        }
    }

    pub fn horizontal(self) -> f32 {
        self.left + self.right
    }

    pub fn vertical(self) -> f32 {
        self.top + self.bottom
    }
}

/// Scene metrics that layout code needs for iPad multitasking and previews.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IosSceneMetrics {
    pub width: f32,
    pub height: f32,
    pub scale_factor: f32,
    pub horizontal_size_class: SizeClass,
    pub vertical_size_class: SizeClass,
    pub dynamic_type: DynamicTypeCategory,
    pub safe_area: SafeAreaInsets,
    pub keyboard_height: f32,
}

impl IosSceneMetrics {
    pub fn content_size(self) -> (f32, f32) {
        (
            (self.width - self.safe_area.horizontal()).max(0.0),
            (self.height - self.safe_area.vertical() - self.keyboard_height).max(0.0),
        )
    }

    pub fn layout_mode(self) -> IosLayoutMode {
        let (width, height) = self.content_size();
        if width >= height {
            IosLayoutMode::LandscapeLike
        } else {
            IosLayoutMode::PortraitLike
        }
    }

    pub fn is_landscape_like(self) -> bool {
        self.layout_mode() == IosLayoutMode::LandscapeLike
    }

    pub fn is_portrait_like(self) -> bool {
        self.layout_mode() == IosLayoutMode::PortraitLike
    }

    pub fn is_split_view_like(self) -> bool {
        let (content_width, _) = self.content_size();
        self.horizontal_size_class == SizeClass::Compact || content_width < 760.0
    }

    pub fn scene_class(self) -> IosSceneClass {
        let (content_width, content_height) = self.content_size();
        if content_width >= 1600.0 || content_height >= 1600.0 {
            return IosSceneClass::ExternalDisplay;
        }
        if self.horizontal_size_class == SizeClass::Compact && content_width < 700.0 {
            return IosSceneClass::IpadCompactSplit;
        }
        if content_width < 760.0 || content_height < 560.0 {
            return IosSceneClass::IpadStageManager;
        }
        if self.horizontal_size_class == SizeClass::Compact
            && self.vertical_size_class == SizeClass::Regular
        {
            return IosSceneClass::Phone;
        }
        IosSceneClass::IpadFullscreen
    }

    pub fn validate(self) -> Result<(), String> {
        if !self.width.is_finite() || self.width <= 0.0 {
            return Err("scene width must be finite and positive".to_string());
        }
        if !self.height.is_finite() || self.height <= 0.0 {
            return Err("scene height must be finite and positive".to_string());
        }
        if !self.scale_factor.is_finite() || self.scale_factor <= 0.0 {
            return Err("scene scale factor must be finite and positive".to_string());
        }
        if self.keyboard_height < 0.0 {
            return Err("keyboard height must not be negative".to_string());
        }
        Ok(())
    }
}

/// High-level bridge capabilities that can be surfaced in debug tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeBridgeCapability {
    SwiftUiEmbedding,
    UiKitEmbedding,
    UiAccessibility,
    DynamicType,
    IpadMultitasking,
    PencilHover,
    WidgetSnapshots,
    InstrumentsSignposts,
}

/// Current native bridge status for docs, debug panels, and CI snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeBridgeReport {
    pub implemented: Vec<NativeBridgeCapability>,
    pub missing: Vec<NativeBridgeCapability>,
}

impl NativeBridgeReport {
    pub fn current() -> Self {
        Self::from_implemented(&[
            NativeBridgeCapability::SwiftUiEmbedding,
            NativeBridgeCapability::UiKitEmbedding,
            NativeBridgeCapability::UiAccessibility,
            NativeBridgeCapability::DynamicType,
            NativeBridgeCapability::IpadMultitasking,
            NativeBridgeCapability::PencilHover,
            NativeBridgeCapability::WidgetSnapshots,
            NativeBridgeCapability::InstrumentsSignposts,
        ])
    }

    pub fn from_implemented(implemented: &[NativeBridgeCapability]) -> Self {
        let all = [
            NativeBridgeCapability::SwiftUiEmbedding,
            NativeBridgeCapability::UiKitEmbedding,
            NativeBridgeCapability::UiAccessibility,
            NativeBridgeCapability::DynamicType,
            NativeBridgeCapability::IpadMultitasking,
            NativeBridgeCapability::PencilHover,
            NativeBridgeCapability::WidgetSnapshots,
            NativeBridgeCapability::InstrumentsSignposts,
        ];
        let missing = all
            .into_iter()
            .filter(|capability| !implemented.contains(capability))
            .collect();

        Self {
            implemented: implemented.to_vec(),
            missing,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.missing.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_metrics_account_for_safe_area_and_keyboard() {
        let metrics = IosSceneMetrics {
            width: 1024.0,
            height: 768.0,
            scale_factor: 2.0,
            horizontal_size_class: SizeClass::Regular,
            vertical_size_class: SizeClass::Regular,
            dynamic_type: DynamicTypeCategory::Large,
            safe_area: SafeAreaInsets::new(24.0, 10.0, 20.0, 10.0),
            keyboard_height: 200.0,
        };

        assert_eq!(metrics.content_size(), (1004.0, 524.0));
        assert!(metrics.is_landscape_like());
        assert!(metrics.validate().is_ok());
        assert!(DynamicTypeCategory::AccessibilityLarge.is_accessibility_size());
    }

    #[test]
    fn scene_metrics_detect_full_screen_ipad_modes() {
        let landscape = IosSceneMetrics {
            width: 1366.0,
            height: 1024.0,
            scale_factor: 2.0,
            horizontal_size_class: SizeClass::Regular,
            vertical_size_class: SizeClass::Regular,
            dynamic_type: DynamicTypeCategory::Medium,
            safe_area: SafeAreaInsets::new(24.0, 0.0, 20.0, 0.0),
            keyboard_height: 0.0,
        };
        let portrait = IosSceneMetrics {
            width: 1024.0,
            height: 1366.0,
            ..landscape
        };

        assert_eq!(landscape.layout_mode(), IosLayoutMode::LandscapeLike);
        assert!(!landscape.is_split_view_like());
        assert_eq!(portrait.layout_mode(), IosLayoutMode::PortraitLike);
        assert!(!portrait.is_split_view_like());
    }

    #[test]
    fn scene_metrics_detect_split_view_like_widths() {
        let split = IosSceneMetrics {
            width: 507.0,
            height: 1024.0,
            scale_factor: 2.0,
            horizontal_size_class: SizeClass::Compact,
            vertical_size_class: SizeClass::Regular,
            dynamic_type: DynamicTypeCategory::Medium,
            safe_area: SafeAreaInsets::new(24.0, 0.0, 20.0, 0.0),
            keyboard_height: 0.0,
        };
        let stage_manager_narrow = IosSceneMetrics {
            horizontal_size_class: SizeClass::Regular,
            ..split
        };

        assert!(split.is_portrait_like());
        assert!(split.is_split_view_like());
        assert!(stage_manager_narrow.is_split_view_like());
        assert_eq!(split.scene_class(), IosSceneClass::IpadCompactSplit);
        assert_eq!(
            stage_manager_narrow.scene_class(),
            IosSceneClass::IpadStageManager
        );
    }

    #[test]
    fn bridge_report_tracks_missing_capabilities() {
        let report = NativeBridgeReport::from_implemented(&[
            NativeBridgeCapability::UiKitEmbedding,
            NativeBridgeCapability::DynamicType,
        ]);

        assert!(!report.is_complete());
        assert!(
            report
                .missing
                .contains(&NativeBridgeCapability::UiAccessibility)
        );
        assert!(
            !report
                .missing
                .contains(&NativeBridgeCapability::DynamicType)
        );
    }

    #[test]
    fn current_bridge_report_is_complete() {
        assert!(NativeBridgeReport::current().is_complete());
    }
}
