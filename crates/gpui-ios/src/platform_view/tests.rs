use super::PlatformView;
use super::platform_view_accessibility::PlatformViewAccessibility;
use super::platform_view_factory::PlatformViewFactory;
use super::platform_view_id::PlatformViewId;
use super::platform_view_kind::PlatformViewKind;
use super::platform_view_registry::PlatformViewRegistry;
use super::types::PlatformViewBounds;
use super::types::PlatformViewParams;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

struct TestView {
    id: PlatformViewId,
    disposed: Mutex<bool>,
}

impl PlatformView for TestView {
    fn id(&self) -> PlatformViewId {
        self.id
    }

    fn view_type(&self) -> &str {
        "test"
    }

    fn set_bounds(&self, _bounds: PlatformViewBounds) {}

    fn set_visible(&self, _visible: bool) {}

    fn set_z_index(&self, _z_index: i32) {}

    fn dispose(&self) {
        *self.disposed.lock().unwrap() = true;
    }

    fn is_disposed(&self) -> bool {
        *self.disposed.lock().unwrap()
    }
}

struct TestFactory;

impl PlatformViewFactory for TestFactory {
    fn create(&self, _params: &PlatformViewParams) -> Result<Box<dyn PlatformView>, String> {
        Ok(Box::new(TestView {
            id: PlatformViewId::next(),
            disposed: Mutex::new(false),
        }))
    }

    fn view_type(&self) -> &str {
        "test"
    }

    fn kind(&self) -> PlatformViewKind {
        PlatformViewKind::UiKit
    }
}

#[test]
fn registry_creates_tracks_and_sorts_views() {
    let registry = PlatformViewRegistry::new();
    registry.register_factory(Box::new(TestFactory));

    let view = registry
        .create_view(
            "test",
            &PlatformViewParams {
                bounds: PlatformViewBounds {
                    x: 10.0,
                    y: 20.0,
                    width: 30.0,
                    height: 40.0,
                },
                creation_params: HashMap::new(),
                accessibility: PlatformViewAccessibility::named("Native picker"),
            },
        )
        .unwrap();

    assert_eq!(registry.view_count(), 1);
    assert!(registry.hit_test(25.0, 35.0));
    registry.update_view_visibility(view.id(), false);
    assert!(!registry.hit_test(25.0, 35.0));

    let snapshot = registry.view_snapshot();
    assert_eq!(snapshot[0].kind, PlatformViewKind::UiKit);
    assert!(snapshot[0].accessibility.is_exposed());
}

#[test]
fn platform_view_kind_parses_common_spellings() {
    assert_eq!(
        "swift-ui".parse::<PlatformViewKind>(),
        Ok(PlatformViewKind::SwiftUi)
    );
    assert_eq!(PlatformViewKind::WebView.as_str(), "webview");
    assert!("unknown".parse::<PlatformViewKind>().is_err());
}

#[test]
fn view_snapshot_returns_cached_arc() {
    let registry = PlatformViewRegistry::new();
    registry.register_factory(Box::new(TestFactory));

    let view = registry
        .create_view(
            "test",
            &PlatformViewParams {
                bounds: PlatformViewBounds {
                    x: 10.0,
                    y: 20.0,
                    width: 30.0,
                    height: 40.0,
                },
                creation_params: HashMap::new(),
                accessibility: PlatformViewAccessibility::named("Native picker"),
            },
        )
        .unwrap();

    let snapshot1 = registry.view_snapshot();
    let snapshot2 = registry.view_snapshot();
    assert!(Arc::ptr_eq(&snapshot1, &snapshot2));

    registry.update_view_visibility(view.id(), false);
    let snapshot3 = registry.view_snapshot();
    assert!(!Arc::ptr_eq(&snapshot1, &snapshot3));
}
