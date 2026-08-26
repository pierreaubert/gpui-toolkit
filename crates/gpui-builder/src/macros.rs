//! Declarative layout macros.

/// Build and solve a nested layout tree in one expression.
///
/// The macro expands to local borrowed `LayoutNode` arrays and immediately
/// passes the root to [`solve`](crate::solve), returning an
/// [`OwnedSolvedNode`](crate::OwnedSolvedNode). The owned result lets the
/// macro release its temporary child arrays immediately instead of leaking
/// declaration storage. For allocation-sensitive repeated solves, build a
/// borrowed tree explicitly and use [`RetainedLayoutSolver`](crate::RetainedLayoutSolver).
///
/// Node names are Rust identifiers and become layout ids via `stringify!`.
///
/// ```rust
/// use gpui_builder::{Axis, LayoutPreferences, Sizing, solve_layout};
///
/// let solved = solve_layout! {
///     width: 1000.0,
///     height: 800.0,
///     prefs: &LayoutPreferences::default(),
///     container root(Axis::Vertical, Sizing::flex(0.0)) {
///         slot header(Sizing::Fixed(40.0));
///         container content(Axis::Horizontal, Sizing::flex(0.0); divider_size = 6.0) {
///             slot sidebar(Sizing::fractional(0.25, 120.0);
///                 priority = 0.5,
///                 collapsible = true,
///                 collapse_label = "Sidebar"
///             );
///             slot main(Sizing::flex(240.0));
///         };
///         slot footer(Sizing::Fixed(80.0));
///     }
/// };
///
/// assert_eq!(solved.find("header").unwrap().height, 40.0);
/// ```
#[macro_export]
macro_rules! solve_layout {
    (
        width: $width:expr,
        height: $height:expr,
        prefs: $prefs:expr,
        container $id:ident($axis:expr, $sizing:expr $(; $($opts:tt)*)?) {
            $($children:tt)*
        }
        $(,)?
    ) => {{
        $crate::__gpui_builder_layout_declare_children! { $($children)* }
        let $id = {
            let mut __gpui_builder_children = ::std::vec::Vec::new();
            $crate::__gpui_builder_layout_push_children! {
                __gpui_builder_children,
                $($children)*
            }
            __gpui_builder_children.into_boxed_slice()
        };
        let __gpui_builder_root = $crate::__gpui_builder_layout_container_node!(
            $id,
            $axis,
            $sizing,
            &$id
            $(; $($opts)*)?
        );
        $crate::solve(&__gpui_builder_root, $width, $height, $prefs).into_owned()
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __gpui_builder_layout_declare_children {
    () => {};

    (
        slot $id:ident($sizing:expr $(; $($opts:tt)*)?);
        $($rest:tt)*
    ) => {
        $crate::__gpui_builder_layout_declare_children! { $($rest)* }
    };

    (
        container $id:ident($axis:expr, $sizing:expr $(; $($opts:tt)*)?) {
            $($children:tt)*
        };
        $($rest:tt)*
    ) => {
        $crate::__gpui_builder_layout_declare_children! { $($children)* }
        let $id = {
            let mut __gpui_builder_children = ::std::vec::Vec::new();
            $crate::__gpui_builder_layout_push_children! {
                __gpui_builder_children,
                $($children)*
            }
            __gpui_builder_children.into_boxed_slice()
        };
        $crate::__gpui_builder_layout_declare_children! { $($rest)* }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __gpui_builder_layout_push_children {
    ($children:ident,) => {};
    () => {};

    (
        $children:ident,
        slot $id:ident($sizing:expr $(; $($opts:tt)*)?);
        $($rest:tt)*
    ) => {
        $children.push($crate::__gpui_builder_layout_slot_node!($id, $sizing $(; $($opts)*)?));
        $crate::__gpui_builder_layout_push_children! { $children, $($rest)* }
    };

    (
        $children:ident,
        container $id:ident($axis:expr, $sizing:expr $(; $($opts:tt)*)?) {
            $($nested:tt)*
        };
        $($rest:tt)*
    ) => {
        $children.push($crate::__gpui_builder_layout_container_node!(
            $id,
            $axis,
            $sizing,
            &$id
            $(; $($opts)*)?
        ));
        $crate::__gpui_builder_layout_push_children! { $children, $($rest)* }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __gpui_builder_layout_slot_node {
    ($id:ident, $sizing:expr $(; $($opts:tt)*)?) => {{
        let mut __gpui_builder_slot = $crate::SlotNode {
            id: stringify!($id),
            sizing: $sizing,
            priority: 1.0,
            collapsible: false,
            display_tiers: &[],
            collapse_label: None,
        };
        $crate::__gpui_builder_layout_slot_options!(
            __gpui_builder_slot
            $(, $($opts)*)?
        );
        $crate::LayoutNode::Slot(__gpui_builder_slot)
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __gpui_builder_layout_container_node {
    ($id:ident, $axis:expr, $sizing:expr, $children:expr $(; $($opts:tt)*)?) => {{
        let mut __gpui_builder_container = $crate::ContainerNode {
            id: stringify!($id),
            axis: $axis,
            auto_axis: None,
            sizing: $sizing,
            children: $children,
            divider_size: 0.0,
        };
        $crate::__gpui_builder_layout_container_options!(
            __gpui_builder_container
            $(, $($opts)*)?
        );
        $crate::LayoutNode::Container(__gpui_builder_container)
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __gpui_builder_layout_slot_options {
    ($slot:ident) => {};

    ($slot:ident, priority = $value:expr $(, $($rest:tt)*)?) => {
        $slot.priority = $value;
        $crate::__gpui_builder_layout_slot_options!($slot $(, $($rest)*)?);
    };

    ($slot:ident, collapsible = $value:expr $(, $($rest:tt)*)?) => {
        $slot.collapsible = $value;
        $crate::__gpui_builder_layout_slot_options!($slot $(, $($rest)*)?);
    };

    ($slot:ident, collapse_label = $value:expr $(, $($rest:tt)*)?) => {
        $slot.collapse_label = Some($value);
        $crate::__gpui_builder_layout_slot_options!($slot $(, $($rest)*)?);
    };

    ($slot:ident, display_tiers = $value:expr $(, $($rest:tt)*)?) => {
        $slot.display_tiers = $value;
        $crate::__gpui_builder_layout_slot_options!($slot $(, $($rest)*)?);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __gpui_builder_layout_container_options {
    ($container:ident) => {};

    ($container:ident, auto_axis = $value:expr $(, $($rest:tt)*)?) => {
        $container.auto_axis = Some($value);
        $crate::__gpui_builder_layout_container_options!($container $(, $($rest)*)?);
    };

    ($container:ident, divider_size = $value:expr $(, $($rest:tt)*)?) => {
        $container.divider_size = $value;
        $crate::__gpui_builder_layout_container_options!($container $(, $($rest)*)?);
    };
}

#[cfg(test)]
mod tests {
    use crate::{
        Axis, ContainerNode, DisplayTier, LayoutNode, LayoutPreferences, Sizing, SlotNode, solve,
    };

    static RACK_TIERS: &[DisplayTier<'_>] = &[
        DisplayTier {
            name: "Full",
            min_size: 200.0,
        },
        DisplayTier {
            name: "Mini",
            min_size: 100.0,
        },
    ];

    #[test]
    fn solve_layout_macro_matches_explicit_nested_tree() {
        let content_children = [
            LayoutNode::Slot(SlotNode {
                id: "library",
                sizing: Sizing::fractional(0.30, 100.0),
                priority: 0.5,
                collapsible: true,
                display_tiers: &[],
                collapse_label: Some("Library"),
            }),
            LayoutNode::Slot(SlotNode {
                id: "queue",
                sizing: Sizing::flex(200.0),
                priority: 1.0,
                collapsible: false,
                display_tiers: &[],
                collapse_label: None,
            }),
            LayoutNode::Slot(SlotNode {
                id: "rack",
                sizing: Sizing::fractional(0.30, 0.0),
                priority: 0.3,
                collapsible: true,
                display_tiers: RACK_TIERS,
                collapse_label: Some("Rack"),
            }),
        ];
        let root_children = [
            LayoutNode::Slot(SlotNode {
                id: "header",
                sizing: Sizing::Fixed(40.0),
                priority: 1.0,
                collapsible: false,
                display_tiers: &[],
                collapse_label: None,
            }),
            LayoutNode::Container(ContainerNode {
                id: "content",
                axis: Axis::Horizontal,
                auto_axis: Some(1.0),
                sizing: Sizing::flex(0.0),
                children: &content_children,
                divider_size: 6.0,
            }),
            LayoutNode::Slot(SlotNode {
                id: "footer",
                sizing: Sizing::Fixed(100.0),
                priority: 1.0,
                collapsible: false,
                display_tiers: &[],
                collapse_label: None,
            }),
        ];
        let explicit_root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Vertical,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &root_children,
            divider_size: 0.0,
        });

        let prefs =
            LayoutPreferences::new(&[("library", Axis::Horizontal, 0.45)], &[("rack", true)]);

        let explicit = solve(&explicit_root, 1200.0, 800.0, &prefs);
        let macro_solved = crate::solve_layout! {
            width: 1200.0,
            height: 800.0,
            prefs: &prefs,
            container root(Axis::Vertical, Sizing::flex(0.0)) {
                slot header(Sizing::Fixed(40.0));
                container content(
                    Axis::Horizontal,
                    Sizing::flex(0.0);
                    auto_axis = 1.0,
                    divider_size = 6.0
                ) {
                    slot library(Sizing::fractional(0.30, 100.0);
                        priority = 0.5,
                        collapsible = true,
                        collapse_label = "Library"
                    );
                    slot queue(Sizing::flex(200.0));
                    slot rack(Sizing::fractional(0.30, 0.0);
                        priority = 0.3,
                        collapsible = true,
                        collapse_label = "Rack",
                        display_tiers = RACK_TIERS
                    );
                };
                slot footer(Sizing::Fixed(100.0));
            }
        };

        for id in [
            "root", "header", "content", "library", "queue", "rack", "footer",
        ] {
            let explicit = explicit.find(id).unwrap();
            let from_macro = macro_solved.find(id).unwrap();
            assert_eq!(from_macro.width, explicit.width, "width mismatch for {id}");
            assert_eq!(
                from_macro.height, explicit.height,
                "height mismatch for {id}"
            );
            assert_eq!(
                from_macro.visible, explicit.visible,
                "visibility mismatch for {id}"
            );
            assert_eq!(
                from_macro.active_tier.as_deref(), explicit.active_tier,
                "tier mismatch for {id}"
            );
            assert_eq!(
                from_macro.collapse_label.as_deref(), explicit.collapse_label,
                "collapse label mismatch for {id}"
            );
        }
    }

    #[test]
    fn solve_layout_macro_returns_owned_result_after_declarations_drop() {
        let solved = {
            let label = String::from("Temporary label");
            let prefs = LayoutPreferences::new(&[], &[("panel", true)]);
            solve_layout! {
                width: 200.0,
                height: 100.0,
                prefs: &prefs,
                container root(Axis::Horizontal, Sizing::flex(0.0)) {
                    slot panel(Sizing::flex(0.0); collapsible = true, collapse_label = label.as_str());
                }
            }
        };

        let panel = solved.find("panel").unwrap();
        assert!(!panel.visible);
        assert_eq!(panel.collapse_label.as_deref(), Some("Temporary label"));
    }
}
