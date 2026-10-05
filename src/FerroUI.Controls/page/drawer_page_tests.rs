//! The drawer page tests: the property round trip, logical children and
//! drawer event groups of the reference tests. The lifecycle, display mode
//! mapping, closing, length validation, escape key and system back button
//! groups are in `drawer_page_tests_lifecycle`; the icon, header and footer
//! template, swipe gesture and detachment groups are in
//! `drawer_page_tests_templates`.
//!
//! The upstream tests are grouped in nested classes; the groups are kept, in
//! order, as sections of the files.

use super::navigation_page_tests::{counter, flag, page_h, same};
use super::tabbed_page_tests::is_logical_child;
use super::{DrawerBehavior, DrawerLayoutBehavior, DrawerPage, DrawerPlacement};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate, IDataTemplate};
use crate::test_support::{boxed_str, string_of, test_scope, TestRoot};
use crate::{Border, Canvas, Control, Image, PathIcon, SplitViewDisplayMode, TextBlock};
use ferroui_base::input::{
    IPointer, KeyModifiers, Pointer, PointerPointProperties, PointerPressedEventArgs, PointerType, PointerUpdateKind,
    RawInputModifiers,
};
use ferroui_base::interactivity::Interactive;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{Brushes, Color, Colors, IBrush, SolidColorBrush};
use ferroui_base::{BoxedValue, ObjectType, Point, Ref, Thickness, Visual};
use std::cell::RefCell;
use std::rc::Rc;

// --- helpers shared by the drawer page test files ---

/// The control as an untyped value.
pub(super) fn boxed<T: ObjectType>(control: &Ref<T>) -> Option<BoxedValue>
where
    Ref<T>: ferroui_base::IntoRef<Control>,
{
    Some(Control::boxed(control.clone()))
}

/// The control held by an untyped value.
pub(super) fn control_of(value: &Option<BoxedValue>) -> Option<Ref<Control>> {
    value.as_ref().and_then(Control::from_boxed)
}

/// The string held by an untyped value.
pub(super) fn text_of(value: &Option<BoxedValue>) -> Option<String> {
    value.as_ref().and_then(string_of)
}

/// `Assert.Same(expected, actual)` for brushes, as a condition.
fn same_brush(expected: &Rc<dyn IBrush>, actual: &Option<Rc<dyn IBrush>>) -> bool {
    actual.as_ref().is_some_and(|actual| expected.reference_id() == actual.reference_id())
}

/// `Assert.Same(expected, actual)` for data templates, as a condition.
pub(super) fn same_template(expected: &Rc<dyn IDataTemplate>, actual: &Option<Rc<dyn IDataTemplate>>) -> bool {
    actual.as_ref().is_some_and(|actual| **actual == **expected)
}

pub(super) fn pointer_pressed_args(target: &Ref<Interactive>, position: Point) -> PointerPressedEventArgs {
    let pointer: Rc<dyn IPointer> = Pointer::new(Pointer::get_next_free_id(), PointerType::Touch, true);
    let visual: Ref<Visual> = target.clone().cast::<Visual>().expect("the target is a visual");
    PointerPressedEventArgs::new(
        target.clone(),
        pointer,
        &visual,
        position,
        1,
        PointerPointProperties::new(RawInputModifiers::LEFT_MOUSE_BUTTON, PointerUpdateKind::LeftButtonPressed),
        KeyModifiers::NONE,
        1,
    )
}

fn raise_pointer_pressed(target: &Ref<Interactive>) {
    let args = pointer_pressed_args(target, Point::default());

    target.raise_event(&args);
}

fn backdrop_only_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, scope| {
        let backdrop = Border::new();
        backdrop.set_name(Some("PART_Backdrop".to_string()));
        let canvas = Canvas::new();
        canvas.children().add(backdrop.register_in_name_scope(&**scope));
        canvas.upcast()
    })
}

/// `new DrawerPage { IsOpen = true }`.
fn open_drawer_page() -> Ref<DrawerPage> {
    let dp = DrawerPage::new();
    dp.set_is_open(true);
    dp
}

// --- PropertyRoundTrips ---

#[test]
fn is_open_toggle() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    dp.set_is_open(true);
    assert!(dp.is_open());
    dp.set_is_open(false);
    assert!(!dp.is_open());
}

#[test]
fn drawer_length_round_trips() {
    let _scope = test_scope();
    for length in [100.0, 280.0, 500.0] {
        let dp = DrawerPage::new();
        dp.set_drawer_length(length);
        assert_eq!(length, dp.drawer_length());
    }
}

#[test]
fn compact_drawer_length_round_trips() {
    let _scope = test_scope();
    for length in [40.0, 56.0, 80.0] {
        let dp = DrawerPage::new();
        dp.set_compact_drawer_length(length);
        assert_eq!(length, dp.compact_drawer_length());
    }
}

#[test]
fn drawer_breakpoint_length_round_trips() {
    let _scope = test_scope();
    for width in [600.0, 800.0, 1200.0] {
        let dp = DrawerPage::new();
        dp.set_drawer_breakpoint_length(width);
        assert_eq!(width, dp.drawer_breakpoint_length());
    }
}

#[test]
fn is_gesture_enabled_round_trips() {
    let _scope = test_scope();
    for value in [true, false] {
        let dp = DrawerPage::new();
        dp.set_is_gesture_enabled(value);
        assert_eq!(value, dp.is_gesture_enabled());
    }
}

#[test]
fn drawer_behavior_round_trips() {
    let _scope = test_scope();
    for behavior in [DrawerBehavior::Auto, DrawerBehavior::Flyout, DrawerBehavior::Locked, DrawerBehavior::Disabled] {
        let dp = DrawerPage::new();
        dp.set_drawer_behavior(behavior);
        assert_eq!(behavior, dp.drawer_behavior());
    }
}

#[test]
fn drawer_layout_behavior_round_trips() {
    let _scope = test_scope();
    for behavior in [
        DrawerLayoutBehavior::Overlay,
        DrawerLayoutBehavior::Split,
        DrawerLayoutBehavior::CompactOverlay,
        DrawerLayoutBehavior::CompactInline,
    ] {
        let dp = DrawerPage::new();
        dp.set_drawer_layout_behavior(behavior);
        assert_eq!(behavior, dp.drawer_layout_behavior());
    }
}

#[test]
fn drawer_placement_round_trips() {
    let _scope = test_scope();
    for placement in [DrawerPlacement::Left, DrawerPlacement::Right, DrawerPlacement::Top, DrawerPlacement::Bottom] {
        let dp = DrawerPage::new();
        dp.set_drawer_placement(placement);
        assert_eq!(placement, dp.drawer_placement());
    }
}

#[test]
fn display_mode_round_trips() {
    let _scope = test_scope();
    for mode in [
        SplitViewDisplayMode::Overlay,
        SplitViewDisplayMode::CompactOverlay,
        SplitViewDisplayMode::Inline,
        SplitViewDisplayMode::CompactInline,
    ] {
        let dp = DrawerPage::new();
        dp.set_display_mode(mode);
        assert_eq!(mode, dp.display_mode());
    }
}

#[test]
fn horizontal_content_alignment_round_trips() {
    let _scope = test_scope();
    for value in [
        HorizontalAlignment::Left,
        HorizontalAlignment::Center,
        HorizontalAlignment::Right,
        HorizontalAlignment::Stretch,
    ] {
        let dp = DrawerPage::new();
        dp.set_horizontal_content_alignment(value);
        assert_eq!(value, dp.horizontal_content_alignment());
    }
}

#[test]
fn vertical_content_alignment_round_trips() {
    let _scope = test_scope();
    for value in
        [VerticalAlignment::Top, VerticalAlignment::Center, VerticalAlignment::Bottom, VerticalAlignment::Stretch]
    {
        let dp = DrawerPage::new();
        dp.set_vertical_content_alignment(value);
        assert_eq!(value, dp.vertical_content_alignment());
    }
}

#[test]
fn drawer_header_accepts_string() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    dp.set_drawer_header(boxed_str("My App"));
    assert_eq!(text_of(&dp.drawer_header()).as_deref(), Some("My App"));
}

#[test]
fn drawer_header_accepts_control() {
    let _scope = test_scope();
    let ctrl = TextBlock::new();
    ctrl.set_text(Some("My App"));
    let dp = DrawerPage::new();
    dp.set_drawer_header(boxed(&ctrl));
    assert!(same(&ctrl, &control_of(&dp.drawer_header())));
}

#[test]
fn drawer_footer_accepts_string() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    dp.set_drawer_footer(boxed_str("v2.0"));
    assert_eq!(text_of(&dp.drawer_footer()).as_deref(), Some("v2.0"));
}

#[test]
fn drawer_footer_accepts_control() {
    let _scope = test_scope();
    let ctrl = TextBlock::new();
    ctrl.set_text(Some("Footer"));
    let dp = DrawerPage::new();
    dp.set_drawer_footer(boxed(&ctrl));
    assert!(same(&ctrl, &control_of(&dp.drawer_footer())));
}

#[test]
fn drawer_icon_accepts_control() {
    let _scope = test_scope();
    let icon = PathIcon::new();
    let dp = DrawerPage::new();
    dp.set_drawer_icon(boxed(&icon));
    assert!(same(&icon, &control_of(&dp.drawer_icon())));
}

#[test]
fn drawer_background_round_trips() {
    let _scope = test_scope();
    let brush: Rc<dyn IBrush> = SolidColorBrush::with_color(Colors::DODGER_BLUE).into();
    let dp = DrawerPage::new();
    dp.set_drawer_background(Some(brush.clone()));
    assert!(same_brush(&brush, &dp.drawer_background()));
}

#[test]
fn drawer_header_background_round_trips() {
    let _scope = test_scope();
    let brush: Rc<dyn IBrush> = SolidColorBrush::with_color(Colors::INDIGO).into();
    let dp = DrawerPage::new();
    dp.set_drawer_header_background(Some(brush.clone()));
    assert!(same_brush(&brush, &dp.drawer_header_background()));
}

#[test]
fn drawer_header_foreground_round_trips() {
    let _scope = test_scope();
    let brush: Rc<dyn IBrush> = Brushes::white();
    let dp = DrawerPage::new();
    dp.set_drawer_header_foreground(Some(brush.clone()));
    assert!(same_brush(&brush, &dp.drawer_header_foreground()));
}

#[test]
fn drawer_footer_background_round_trips() {
    let _scope = test_scope();
    let brush: Rc<dyn IBrush> = SolidColorBrush::with_color(Colors::DARK_GRAY).into();
    let dp = DrawerPage::new();
    dp.set_drawer_footer_background(Some(brush.clone()));
    assert!(same_brush(&brush, &dp.drawer_footer_background()));
}

#[test]
fn drawer_footer_foreground_round_trips() {
    let _scope = test_scope();
    let brush: Rc<dyn IBrush> = Brushes::light_gray();
    let dp = DrawerPage::new();
    dp.set_drawer_footer_foreground(Some(brush.clone()));
    assert!(same_brush(&brush, &dp.drawer_footer_foreground()));
}

#[test]
fn drawer_template_can_be_set_to_null() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    dp.set_drawer_template(None);
    assert!(dp.drawer_template().is_none());
}

#[test]
fn content_template_can_be_set_to_null() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    dp.set_content_template(None);
    assert!(dp.content_template().is_none());
}

#[test]
fn backdrop_brush_round_trips() {
    let _scope = test_scope();
    let brush: Rc<dyn IBrush> = SolidColorBrush::with_color(Color::from_argb(128, 0, 0, 0)).into();
    let dp = DrawerPage::new();
    dp.set_backdrop_brush(Some(brush.clone()));
    assert!(same_brush(&brush, &dp.backdrop_brush()));
}

#[test]
fn backdrop_brush_can_be_set_to_null() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    dp.set_backdrop_brush(Some(Brushes::black()));
    dp.set_backdrop_brush(None);
    assert!(dp.backdrop_brush().is_none());
}

#[test]
fn header_round_trips() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    dp.set_header(boxed_str("My Drawer Page"));
    assert_eq!(text_of(&dp.header()).as_deref(), Some("My Drawer Page"));
}

#[test]
fn icon_round_trips() {
    let _scope = test_scope();
    let icon = Image::new();
    let dp = DrawerPage::new();
    dp.set_icon(boxed(&icon));
    assert!(same(&icon, &control_of(&dp.icon())));
}

#[test]
fn safe_area_padding_round_trips() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    let padding = Thickness::new(10.0, 20.0, 10.0, 34.0);
    dp.set_safe_area_padding(padding);
    assert_eq!(padding, dp.safe_area_padding());
}

#[test]
fn drawer_behavior_disabled_prevents_is_open_set_to_true() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    dp.set_drawer_behavior(DrawerBehavior::Disabled);
    dp.set_is_open(true);
    assert!(!dp.is_open());
}

#[test]
fn drawer_accepts_string() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    dp.set_drawer(boxed_str("MenuContent"));
    assert_eq!(text_of(&dp.drawer()).as_deref(), Some("MenuContent"));
}

#[test]
fn content_accepts_string() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    dp.set_content(boxed_str("ContentValue"));
    assert_eq!(text_of(&dp.content()).as_deref(), Some("ContentValue"));
}

#[test]
fn drawer_accepts_content_page() {
    let _scope = test_scope();
    let page = page_h("Menu");
    let dp = DrawerPage::new();
    dp.set_drawer(boxed(&page));
    assert!(same(&page, &control_of(&dp.drawer())));
}

#[test]
fn content_accepts_content_page() {
    let _scope = test_scope();
    let page = page_h("Main");
    let dp = DrawerPage::new();
    dp.set_content(boxed(&page));
    assert!(same(&page, &control_of(&dp.content())));
}

// --- LogicalChildrenTests ---

#[test]
fn drawer_set_page_added_to_logical_children() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    let drawer = page_h("Menu");
    dp.set_drawer(boxed(&drawer));
    assert!(is_logical_child(&dp, &drawer));
}

#[test]
fn content_set_page_added_to_logical_children() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    let detail = page_h("Content");
    dp.set_content(boxed(&detail));
    assert!(is_logical_child(&dp, &detail));
}

#[test]
fn drawer_replaced_old_removed_new_added() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    let first = page_h("First");
    let second = page_h("Second");

    dp.set_drawer(boxed(&first));
    dp.set_drawer(boxed(&second));

    assert!(!is_logical_child(&dp, &first));
    assert!(is_logical_child(&dp, &second));
}

#[test]
fn content_replaced_old_removed_new_added() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    let first = page_h("First");
    let second = page_h("Second");

    dp.set_content(boxed(&first));
    dp.set_content(boxed(&second));

    assert!(!is_logical_child(&dp, &first));
    assert!(is_logical_child(&dp, &second));
}

#[test]
fn drawer_set_to_null_removed_from_logical_children() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    let drawer = page_h("Menu");
    dp.set_drawer(boxed(&drawer));
    dp.set_drawer(None);
    assert!(!is_logical_child(&dp, &drawer));
}

#[test]
fn content_set_to_null_removed_from_logical_children() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    let detail = page_h("Content");
    dp.set_content(boxed(&detail));
    dp.set_content(None);
    assert!(!is_logical_child(&dp, &detail));
}

#[test]
fn drawer_and_content_both_set_both_in_logical_children() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    let drawer = page_h("Menu");
    let detail = page_h("Home");
    dp.set_drawer(boxed(&drawer));
    dp.set_content(boxed(&detail));

    assert!(is_logical_child(&dp, &drawer));
    assert!(is_logical_child(&dp, &detail));
}

#[test]
fn drawer_multiple_replacements_only_last_in_logical_children() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    let first = page_h("1st");
    let second = page_h("2nd");
    let third = page_h("3rd");

    dp.set_drawer(boxed(&first));
    dp.set_drawer(boxed(&second));
    dp.set_drawer(boxed(&third));

    assert!(!is_logical_child(&dp, &first));
    assert!(!is_logical_child(&dp, &second));
    assert!(is_logical_child(&dp, &third));
}

// --- DrawerEventTests ---

#[test]
fn is_open_set_true_fires_opened() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    let fired = flag();
    let f = fired.clone();
    dp.opened(move |_, _| f.set(true));

    dp.set_is_open(true);

    assert!(fired.get());
}

#[test]
fn is_open_set_false_fires_closed() {
    let _scope = test_scope();
    let dp = open_drawer_page();
    let fired = flag();
    let f = fired.clone();
    dp.closed(move |_, _| f.set(true));

    dp.set_is_open(false);

    assert!(fired.get());
}

#[test]
fn is_open_set_false_fires_closing_before_closed() {
    let _scope = test_scope();
    let dp = open_drawer_page();
    let order = Rc::new(RefCell::new(Vec::new()));
    let o = order.clone();
    dp.closing(move |_, _| o.borrow_mut().push("Closing"));
    let o = order.clone();
    dp.closed(move |_, _| o.borrow_mut().push("Closed"));

    dp.set_is_open(false);

    assert_eq!(vec!["Closing", "Closed"], *order.borrow());
}

#[test]
fn closing_cancel_prevents_close() {
    let _scope = test_scope();
    let dp = open_drawer_page();
    dp.closing(|_, e| e.set_cancel(true));

    dp.set_is_open(false);

    assert!(dp.is_open());
}

#[test]
fn closing_cancel_does_not_fire_closed() {
    let _scope = test_scope();
    let dp = open_drawer_page();
    dp.closing(|_, e| e.set_cancel(true));
    let closed_fired = flag();
    let f = closed_fired.clone();
    dp.closed(move |_, _| f.set(true));

    dp.set_is_open(false);

    assert!(!closed_fired.get());
}

#[test]
fn closing_cancel_does_not_fire_opened() {
    let _scope = test_scope();
    let dp = open_drawer_page();
    dp.closing(|_, e| e.set_cancel(true));
    let opened_fired = flag();
    let f = opened_fired.clone();
    dp.opened(move |_, _| f.set(true));

    dp.set_is_open(false);

    assert!(!opened_fired.get());
}

#[test]
fn is_open_already_true_set_true_does_not_fire_opened() {
    let _scope = test_scope();
    let dp = open_drawer_page();
    let fired = flag();
    let f = fired.clone();
    dp.opened(move |_, _| f.set(true));

    dp.set_is_open(true);

    assert!(!fired.get());
}

#[test]
fn is_open_already_false_set_false_does_not_fire_closing_or_closed() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    let closing_fired = flag();
    let closed_fired = flag();
    let f = closing_fired.clone();
    dp.closing(move |_, _| f.set(true));
    let f = closed_fired.clone();
    dp.closed(move |_, _| f.set(true));

    dp.set_is_open(false);

    assert!(!closing_fired.get());
    assert!(!closed_fired.get());
}

#[test]
fn is_open_set_true_does_not_fire_closing() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    let closing_fired = flag();
    let f = closing_fired.clone();
    dp.closing(move |_, _| f.set(true));

    dp.set_is_open(true);

    assert!(!closing_fired.get());
}

#[test]
fn drawer_behavior_locked_forces_is_open_true() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    dp.set_drawer_behavior(DrawerBehavior::Locked);
    assert!(dp.is_open());
}

#[test]
fn drawer_behavior_locked_while_closed_opens_without_firing_closing() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    let closing_fired = flag();
    let f = closing_fired.clone();
    dp.closing(move |_, _| f.set(true));

    dp.set_drawer_behavior(DrawerBehavior::Locked);

    assert!(dp.is_open());
    assert!(!closing_fired.get());
}

#[test]
fn closing_cancel_prevents_close_even_with_reentrant_is_open_false() {
    let _scope = test_scope();
    let dp = open_drawer_page();
    dp.closing(|_, e| e.set_cancel(true));

    let weak = dp.downgrade();
    let _subscription = dp.property_changed(move |e| {
        if e.property() == DrawerPage::is_open_property().as_property() {
            if let Some(dp) = weak.upgrade() {
                dp.set_current_value(DrawerPage::is_open_property(), false);
            }
        }
    });

    dp.set_is_open(false);

    assert!(dp.is_open());
}

#[test]
fn is_open_rapid_toggle_events_fired_exactly_once_per_change() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    let opened_count = counter();
    let closed_count = counter();
    let c = opened_count.clone();
    dp.opened(move |_, _| c.set(c.get() + 1));
    let c = closed_count.clone();
    dp.closed(move |_, _| c.set(c.get() + 1));

    for _ in 0..5 {
        dp.set_is_open(true);
        dp.set_is_open(false);
    }

    assert_eq!(5, opened_count.get());
    assert_eq!(5, closed_count.get());
    assert!(!dp.is_open());
}

#[test]
fn backdrop_press_with_canceled_close_fires_closing_once_when_template_was_applied_before_attach() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    dp.set_template(Some(backdrop_only_template()));
    dp.set_is_open(true);
    dp.set_backdrop_brush(Some(Brushes::black()));
    dp.set_display_mode(SplitViewDisplayMode::Overlay);

    dp.apply_template();

    let closing_count = counter();
    let c = closing_count.clone();
    dp.closing(move |_, e| {
        c.set(c.get() + 1);
        e.set_cancel(true);
    });

    let root = TestRoot::with_child(dp.clone());
    root.execute_initial_layout_pass();

    let backdrops: Vec<Ref<Border>> = dp
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<Border>())
        .filter(|border| border.name().as_deref() == Some("PART_Backdrop"))
        .collect();
    assert_eq!(1, backdrops.len());

    raise_pointer_pressed(&backdrops[0].clone().upcast());

    assert_eq!(1, closing_count.get());
    assert!(dp.is_open());
}
