use crate::primitives::{RangeBase, ScrollBar, ScrollBarVisibility, ScrollEventArgs, ScrollEventType, Thumb, Track};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::test_support::test_scope;
use crate::{Border, Control};
use ferroui_base::controls::NameScopeRef;
use ferroui_base::data::{BindingMode, IndexerBinding};
use ferroui_base::input::{
    ContextRequestedEventArgs, InputElement, Key, KeyEventArgs, KeyModifiers, Pointer, PointerPointProperties,
    PointerPressedEventArgs, PointerType, PointerUpdateKind, RawInputModifiers, VectorEventArgs,
};
use ferroui_base::layout::Orientation;
use ferroui_base::media::Brushes;
use ferroui_base::{FerroObject, FerroProperty, Point, Ref, Vector};
use std::cell::RefCell;
use std::rc::Rc;

fn template(control: &Ref<ScrollBar>, scope: &NameScopeRef) -> Ref<Control> {
    let source: Ref<FerroObject> = control.clone().upcast();
    let track = Track::new();
    track.set_name(Some("track".to_string()));

    let bind = |target: &'static FerroProperty, property: &'static FerroProperty, mode: BindingMode| {
        track.bind_binding(target, &IndexerBinding::new(source.clone(), property, mode));
    };
    bind(Track::minimum_property().as_property(), RangeBase::minimum_property().as_property(), BindingMode::OneWay);
    bind(Track::maximum_property().as_property(), RangeBase::maximum_property().as_property(), BindingMode::OneWay);
    bind(Track::value_property().as_property(), RangeBase::value_property().as_property(), BindingMode::TwoWay);
    bind(
        Track::viewport_size_property().as_property(),
        ScrollBar::viewport_size_property().as_property(),
        BindingMode::OneWay,
    );
    bind(
        Track::orientation_property().as_property(),
        ScrollBar::orientation_property().as_property(),
        BindingMode::OneWay,
    );

    let thumb = Thumb::new();
    thumb.set_template(Some(FuncControlTemplate::for_type::<Thumb>(thumb_template)));
    track.set_thumb(thumb);

    let border = Border::new();
    border.set_child(track.register_in_name_scope(&**scope));
    border.upcast()
}

fn thumb_template(_control: &Ref<Thumb>, _scope: &NameScopeRef) -> Ref<Control> {
    let border = Border::new();
    border.set_background(Some(Brushes::gray()));
    border.upcast()
}

fn templated_scroll_bar() -> Ref<ScrollBar> {
    let target = ScrollBar::new();
    target.set_template(Some(FuncControlTemplate::for_type::<ScrollBar>(template)));
    target
}

fn find_track(target: &ScrollBar) -> Ref<Track> {
    target
        .get_template_descendants()
        .into_iter()
        .filter_map(|x| x.cast::<Track>())
        .find(|x| x.name().as_deref() == Some("track"))
        .unwrap()
}

fn create_scroll_bar(
    orientation: Orientation,
    minimum: f64,
    maximum: f64,
    value: f64,
    small_change: f64,
    large_change: f64,
) -> Ref<ScrollBar> {
    let target = ScrollBar::new();
    target.set_orientation(orientation);
    target.set_template(Some(FuncControlTemplate::for_type::<ScrollBar>(template)));
    target.set_minimum(minimum);
    target.set_maximum(maximum);
    target.set_range_value(value);
    target.set_small_change(small_change);
    target.set_large_change(large_change);

    target.apply_template();
    target
}

fn create_default_scroll_bar(orientation: Orientation) -> Ref<ScrollBar> {
    create_scroll_bar(orientation, 0.0, 100.0, 0.0, 1.0, 10.0)
}

fn key_down(target: &ScrollBar, key: Key) {
    let mut e = KeyEventArgs::new();
    e.set_routed_event(Some(InputElement::key_down_event()));
    e.key = key;
    target.raise_event(&e);
}

fn create_context_requested(target: &Ref<ScrollBar>, pointer_type: PointerType) -> ContextRequestedEventArgs {
    let pointer = Pointer::new(Pointer::get_next_free_id(), pointer_type, true);
    let pointer_args = PointerPressedEventArgs::new(
        target.clone(),
        pointer,
        target,
        Point::default(),
        1,
        PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::Other),
        KeyModifiers::NONE,
        1,
    );

    ContextRequestedEventArgs::from_pointer_event_args(&pointer_args)
}

fn record_scroll(target: &ScrollBar) -> Rc<RefCell<Vec<ScrollEventArgs>>> {
    let events = Rc::new(RefCell::new(Vec::new()));
    let recorded = events.clone();
    target.scroll(move |e| recorded.borrow_mut().push(*e));
    events
}

#[test]
fn setting_value_should_update_track_value() {
    let _scope = test_scope();
    let target = templated_scroll_bar();

    target.apply_template();
    let track = find_track(&target);
    target.set_range_value(50.0);

    assert_eq!(50.0, track.value());
}

#[test]
fn setting_track_value_should_update_value() {
    let _scope = test_scope();
    let target = templated_scroll_bar();

    target.apply_template();
    let track = find_track(&target);
    track.set_range_value(50.0);

    assert_eq!(50.0, target.value());
}

#[test]
fn setting_track_value_after_setting_value_should_update_value() {
    let _scope = test_scope();
    let target = templated_scroll_bar();

    target.apply_template();

    let track = find_track(&target);
    target.set_range_value(25.0);
    track.set_range_value(50.0);

    assert_eq!(50.0, target.value());
}

#[test]
fn thumb_drag_delta_event_should_raise_scroll_event() {
    let _scope = test_scope();
    let target = templated_scroll_bar();

    target.apply_template();

    let track = find_track(&target);
    let events = record_scroll(&target);

    let mut ev = VectorEventArgs::new();
    ev.set_routed_event(Some(Thumb::drag_delta_event()));
    ev.vector = Vector::new(0.0, 0.0);

    track.thumb().unwrap().raise_event(&ev);

    assert_eq!(1, events.borrow().len());
    assert_eq!(ScrollEventType::ThumbTrack, events.borrow()[0].scroll_event_type());
}

#[test]
fn thumb_drag_complete_event_should_raise_scroll_event() {
    let _scope = test_scope();
    let target = templated_scroll_bar();

    target.apply_template();

    let track = find_track(&target);
    let events = record_scroll(&target);

    let mut ev = VectorEventArgs::new();
    ev.set_routed_event(Some(Thumb::drag_completed_event()));
    ev.vector = Vector::new(0.0, 0.0);

    track.thumb().unwrap().raise_event(&ev);

    assert_eq!(1, events.borrow().len());
    assert_eq!(ScrollEventType::EndScroll, events.borrow()[0].scroll_event_type());
}

#[test]
fn scroll_bar_can_auto_hide() {
    let _scope = test_scope();
    let target = ScrollBar::new();

    target.set_visibility(ScrollBarVisibility::Auto);
    target.set_viewport_size(1.0);
    target.set_maximum(0.0);

    assert!(!target.is_visible());
}

#[test]
fn scroll_bar_should_not_auto_hide_when_viewport_size_is_nan() {
    let _scope = test_scope();
    let target = ScrollBar::new();

    target.set_visibility(ScrollBarVisibility::Auto);
    target.set_minimum(0.0);
    target.set_maximum(100.0);
    target.set_viewport_size(f64::NAN);

    assert!(target.is_visible());
}

#[test]
fn scroll_bar_should_not_auto_hide_when_visibility_set_to_visible() {
    let _scope = test_scope();
    let target = ScrollBar::new();

    target.set_visibility(ScrollBarVisibility::Visible);
    target.set_minimum(0.0);
    target.set_maximum(100.0);
    target.set_viewport_size(100.0);

    assert!(target.is_visible());
}

#[test]
fn scroll_bar_should_hide_when_visibility_set_to_hidden() {
    let _scope = test_scope();
    let target = ScrollBar::new();

    target.set_visibility(ScrollBarVisibility::Hidden);
    target.set_minimum(0.0);
    target.set_maximum(100.0);
    target.set_viewport_size(10.0);

    assert!(!target.is_visible());
}

fn orientation_should_set_matching_pseudo_class(orientation: Orientation) {
    let _scope = test_scope();
    let target = ScrollBar::new();
    target.set_orientation(orientation);

    assert_eq!(orientation == Orientation::Vertical, target.classes().contains(":vertical"));
    assert_eq!(orientation == Orientation::Horizontal, target.classes().contains(":horizontal"));
}

#[test]
fn orientation_should_set_matching_pseudo_class_vertical() {
    orientation_should_set_matching_pseudo_class(Orientation::Vertical);
}

#[test]
fn orientation_should_set_matching_pseudo_class_horizontal() {
    orientation_should_set_matching_pseudo_class(Orientation::Horizontal);
}

#[test]
fn scroll_to_home_should_set_value_to_minimum() {
    let _scope = test_scope();
    let target = create_scroll_bar(Orientation::Vertical, 10.0, 100.0, 50.0, 1.0, 10.0);

    target.scroll_to_home();

    assert_eq!(10.0, target.value());
}

#[test]
fn scroll_to_end_should_set_value_to_maximum() {
    let _scope = test_scope();
    let target = create_scroll_bar(Orientation::Vertical, 0.0, 90.0, 50.0, 1.0, 10.0);

    target.scroll_to_end();

    assert_eq!(90.0, target.value());
}

#[test]
fn page_up_should_decrease_value_by_large_change() {
    let _scope = test_scope();
    let target = create_scroll_bar(Orientation::Vertical, 0.0, 100.0, 50.0, 1.0, 10.0);

    target.page_up();

    assert_eq!(40.0, target.value());
}

#[test]
fn page_down_should_increase_value_by_large_change() {
    let _scope = test_scope();
    let target = create_scroll_bar(Orientation::Vertical, 0.0, 100.0, 50.0, 1.0, 10.0);

    target.page_down();

    assert_eq!(60.0, target.value());
}

#[test]
fn page_left_should_decrease_value_by_large_change() {
    let _scope = test_scope();
    let target = create_scroll_bar(Orientation::Horizontal, 0.0, 100.0, 50.0, 1.0, 10.0);

    target.page_left();

    assert_eq!(40.0, target.value());
}

#[test]
fn page_right_should_increase_value_by_large_change() {
    let _scope = test_scope();
    let target = create_scroll_bar(Orientation::Horizontal, 0.0, 100.0, 50.0, 1.0, 10.0);

    target.page_right();

    assert_eq!(60.0, target.value());
}

#[test]
fn line_up_should_decrease_value_by_small_change() {
    let _scope = test_scope();
    let target = create_scroll_bar(Orientation::Vertical, 0.0, 100.0, 50.0, 5.0, 10.0);

    target.line_up();

    assert_eq!(45.0, target.value());
}

#[test]
fn line_down_should_increase_value_by_small_change() {
    let _scope = test_scope();
    let target = create_scroll_bar(Orientation::Vertical, 0.0, 100.0, 50.0, 5.0, 10.0);

    target.line_down();

    assert_eq!(55.0, target.value());
}

#[test]
fn line_left_should_decrease_value_by_small_change() {
    let _scope = test_scope();
    let target = create_scroll_bar(Orientation::Horizontal, 0.0, 100.0, 50.0, 5.0, 10.0);

    target.line_left();

    assert_eq!(45.0, target.value());
}

#[test]
fn line_right_should_increase_value_by_small_change() {
    let _scope = test_scope();
    let target = create_scroll_bar(Orientation::Horizontal, 0.0, 100.0, 50.0, 5.0, 10.0);

    target.line_right();

    assert_eq!(55.0, target.value());
}

#[test]
fn scroll_methods_should_respect_value_bounds() {
    let _scope = test_scope();
    let target = create_scroll_bar(Orientation::Vertical, 20.0, 80.0, 25.0, 10.0, 10.0);

    target.line_up();
    assert_eq!(20.0, target.value()); // Should not go below Minimum

    target.set_range_value(75.0);
    target.line_down();
    assert_eq!(80.0, target.value()); // Should not go above Maximum
}

#[test]
fn scroll_methods_should_raise_scroll_event() {
    let _scope = test_scope();
    let target = create_scroll_bar(Orientation::Vertical, 0.0, 100.0, 50.0, 5.0, 10.0);
    let events = record_scroll(&target);

    target.line_down();
    target.page_down();
    target.scroll_to_home();
    target.scroll_to_end();

    let events: Vec<ScrollEventType> = events.borrow().iter().map(|e| e.scroll_event_type()).collect();
    assert_eq!(
        vec![
            ScrollEventType::SmallIncrement,
            ScrollEventType::LargeIncrement,
            ScrollEventType::LargeDecrement,
            ScrollEventType::LargeIncrement,
        ],
        events
    );
}

#[test]
fn scroll_here_should_set_value_within_bounds() {
    let _scope = test_scope();
    let target = create_scroll_bar(Orientation::Vertical, 0.0, 100.0, 0.0, 1.0, 10.0);

    // No panic even though no pointer position was recorded yet.
    target.scroll_here();

    assert!(target.value() >= target.minimum() && target.value() <= target.maximum());
}

fn context_requested_should_be_handled_for_touch_or_pen_input(pointer_type: PointerType) {
    let _scope = test_scope();
    let target = create_default_scroll_bar(Orientation::Vertical);

    let args = create_context_requested(&target, pointer_type);
    target.raise_event(&args);

    assert!(args.handled());
}

#[test]
fn context_requested_should_be_handled_for_touch_or_pen_input_touch() {
    context_requested_should_be_handled_for_touch_or_pen_input(PointerType::Touch);
}

#[test]
fn context_requested_should_be_handled_for_touch_or_pen_input_pen() {
    context_requested_should_be_handled_for_touch_or_pen_input(PointerType::Pen);
}

#[test]
fn context_requested_should_not_be_handled_for_mouse_input() {
    let _scope = test_scope();
    let target = create_default_scroll_bar(Orientation::Vertical);

    let args = create_context_requested(&target, PointerType::Mouse);
    target.raise_event(&args);

    assert!(!args.handled());
}

#[test]
fn focus_key_input_should_scroll() {
    let _scope = test_scope();
    // Vertical scrollbar
    let target = create_default_scroll_bar(Orientation::Vertical);

    // Page down and page up
    key_down(&target, Key::PageDown);
    assert_eq!(target.large_change(), target.value());
    key_down(&target, Key::PageUp);
    assert_eq!(0.0, target.value());

    // Small scrolling with arrow keys
    key_down(&target, Key::Down);
    assert_eq!(target.small_change(), target.value());
    key_down(&target, Key::Up);
    assert_eq!(0.0, target.value());

    // Vertical scrollbars shouldn't scroll for left and right keys
    key_down(&target, Key::Right);
    assert_eq!(0.0, target.value());
    key_down(&target, Key::Left);
    assert_eq!(0.0, target.value());

    // Horizontal scrollbar
    let target = create_default_scroll_bar(Orientation::Horizontal);

    // Small scrolling with arrow keys
    key_down(&target, Key::Right);
    assert_eq!(target.small_change(), target.value());
    key_down(&target, Key::Left);
    assert_eq!(0.0, target.value());

    // Horizontal scrollbars shouldn't scroll for up and down keys
    key_down(&target, Key::Down);
    assert_eq!(0.0, target.value());
    key_down(&target, Key::Up);
    assert_eq!(0.0, target.value());
}
