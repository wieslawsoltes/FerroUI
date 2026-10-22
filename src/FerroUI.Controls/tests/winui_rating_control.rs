//! Rating contracts and real native template/input paths against the pinned WinUI source.

use ferroui_base::input::{
    IPointer, InputElement, Key, KeyEventArgs, KeyModifiers, MouseButton, Pointer,
    PointerPointProperties, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
};
use ferroui_base::layout::ILayoutManager;
use ferroui_base::media::{DrawingImage, FlowDirection, IImage, RectangleGeometry};
use ferroui_base::{Point, Ref};
use ferroui_controls::automation::peers::{AutomationControlType, ControlAutomationPeer};
use ferroui_controls::automation::provider::{IRangeValueProvider, IValueProvider};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{RatingControl, RatingItemFontInfo, RatingItemImageInfo};
use ferroui_controls::{Control, Grid, Image, StackPanel, TextBlock, Window};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn start() -> UnitTestApplicationScope {
    ferroui_controls::register_types();
    UnitTestApplication::start(TestServices::styled_window())
}

fn press(control: &RatingControl, key: Key) -> KeyEventArgs {
    let mut args = KeyEventArgs::new();
    args.key = key;
    args.set_routed_event(Some(InputElement::key_down_event()));
    control.raise_event(&args);
    args
}

fn panel(control: &RatingControl, foreground: bool) -> Ref<StackPanel> {
    control.apply_template();
    let name = if foreground {
        "RatingForegroundStackPanel"
    } else {
        "RatingBackgroundStackPanel"
    };
    control
        .get_template_descendants()
        .into_iter()
        .find_map(|child| {
            (child.name().as_deref() == Some(name))
                .then(|| child.cast::<StackPanel>())
                .flatten()
        })
        .unwrap()
}

fn item(panel: &StackPanel, index: usize) -> Ref<Control> {
    panel
        .children()
        .get(index)
        .cast::<Grid>()
        .unwrap()
        .children()
        .get(0)
}

#[test]
fn rating_defaults_have_distinct_unset_and_placeholder_domains() {
    let _app = start();
    let rating = RatingControl::new();
    assert_eq!(rating.value(), -1.0);
    assert_eq!(rating.placeholder_value(), -1.0);
    assert_eq!(rating.max_rating(), 5);
    assert_eq!(rating.initial_set_value(), 1);
    assert!(rating.is_clear_enabled());
    assert!(!rating.is_read_only());
    assert_eq!(rating.caption(), "");
    assert!(rating.focusable());
    rating.set_rating_value(0.2);
    rating.set_placeholder_value(0.2);
    assert_eq!(rating.value(), 1.0);
    assert_eq!(rating.placeholder_value(), 0.2);
    rating.set_placeholder_value(0.0);
    assert_eq!(rating.placeholder_value(), 0.0);
    rating.set_rating_value(-9.0);
    rating.set_placeholder_value(-9.0);
    assert_eq!(rating.value(), -1.0);
    assert_eq!(rating.placeholder_value(), -1.0);
}

#[test]
fn maximum_coercion_commits_valid_bound_before_values_and_does_not_restore_clamped_values() {
    let _app = start();
    let rating = RatingControl::new();
    rating.set_rating_value(4.5);
    rating.set_placeholder_value(3.5);
    rating.set_max_rating(-2);
    assert_eq!(rating.max_rating(), 1);
    assert_eq!(rating.value(), 1.0);
    assert_eq!(rating.placeholder_value(), 1.0);
    rating.set_max_rating(10);
    assert_eq!(rating.value(), 1.0);
    assert_eq!(rating.placeholder_value(), 1.0);
    rating.set_rating_value(f64::INFINITY);
    assert_eq!(rating.value(), 10.0);
    rating.set_rating_value(f64::NEG_INFINITY);
    assert_eq!(rating.value(), -1.0);
}

#[test]
fn programmatic_updates_are_silent_and_user_notifications_allow_reentrant_edits() {
    let _app = start();
    let rating = RatingControl::new();
    let observed = Rc::new(RefCell::new(Vec::new()));
    let log = observed.clone();
    let subscription = rating.value_changed(move |sender| {
        log.borrow_mut().push(sender.value());
        if sender.value() == 3.0 {
            sender.set_rating_value(4.0);
        }
    });
    rating.set_rating_value(2.0);
    rating.set_max_rating(4);
    rating.set_placeholder_value(1.5);
    assert!(observed.borrow().is_empty());
    assert!(press(&rating, Key::Right).handled());
    assert_eq!(*observed.borrow(), vec![3.0]);
    assert_eq!(rating.value(), 4.0);
    subscription.dispose();
    press(&rating, Key::Home);
    assert_eq!(*observed.borrow(), vec![3.0]);
    assert_eq!(rating.value(), -1.0);
}

#[test]
fn keyboard_honors_initial_value_fractional_steps_and_clear_policy() {
    let _app = start();
    let rating = RatingControl::new();
    rating.set_initial_set_value(3);
    press(&rating, Key::Left);
    assert_eq!(rating.value(), 3.0);
    rating.set_rating_value(3.75);
    press(&rating, Key::Down);
    assert_eq!(rating.value(), 3.0);
    rating.set_rating_value(3.75);
    press(&rating, Key::Up);
    assert_eq!(rating.value(), 4.0);
    press(&rating, Key::End);
    press(&rating, Key::Right);
    assert_eq!(rating.value(), 5.0);
    press(&rating, Key::Home);
    assert_eq!(rating.value(), -1.0);
    rating.set_is_clear_enabled(false);
    press(&rating, Key::End);
    press(&rating, Key::Home);
    assert_eq!(rating.value(), 1.0);
    press(&rating, Key::Left);
    assert_eq!(rating.value(), 1.0);
}

#[test]
fn keyboard_horizontal_direction_reverses_in_rtl_but_vertical_does_not() {
    let _app = start();
    let rating = RatingControl::new();
    rating.set_flow_direction(FlowDirection::RightToLeft);
    rating.set_rating_value(3.0);
    press(&rating, Key::Right);
    assert_eq!(rating.value(), 2.0);
    press(&rating, Key::Left);
    assert_eq!(rating.value(), 3.0);
    press(&rating, Key::Up);
    assert_eq!(rating.value(), 4.0);
    press(&rating, Key::Down);
    assert_eq!(rating.value(), 3.0);
}

#[test]
fn normalized_gamepad_engagement_commits_or_restores_the_pre_engagement_value() {
    use ferroui_base::input::FocusChangedEventArgs;
    use ferroui_base::input::KeyDeviceType;
    let _app = start();
    let rating = RatingControl::new();
    rating.set_initial_set_value(2);
    let gamepad = |key| {
        let mut args = KeyEventArgs::new();
        args.key = key;
        args.key_device_type = KeyDeviceType::Gamepad;
        args.set_routed_event(Some(InputElement::key_down_event()));
        rating.raise_event(&args);
        args
    };
    assert!(!gamepad(Key::Right).handled());
    assert_eq!(rating.value(), -1.0);
    assert!(gamepad(Key::Enter).handled());
    assert!(rating.is_focus_engaged());
    assert_eq!(rating.value(), 2.0);
    gamepad(Key::Right);
    assert_eq!(rating.value(), 3.0);
    gamepad(Key::Escape);
    assert_eq!(rating.value(), -1.0);
    assert!(!rating.is_focus_engaged());
    gamepad(Key::Enter);
    gamepad(Key::Right);
    gamepad(Key::Enter);
    assert_eq!(rating.value(), 3.0);
    assert!(!rating.is_focus_engaged());
    gamepad(Key::Enter);
    gamepad(Key::Right);
    let args = FocusChangedEventArgs::new(InputElement::lost_focus_event());
    rating.raise_event(&args);
    assert_eq!(rating.value(), 3.0);
    assert!(!rating.is_focus_engaged());
}

#[test]
fn read_only_and_disabled_block_user_edits_but_preserve_programmatic_assignment() {
    let _app = start();
    let rating = RatingControl::new();
    let count = Rc::new(Cell::new(0));
    let log = count.clone();
    rating.value_changed(move |_| log.set(log.get() + 1));
    rating.set_is_read_only(true);
    assert!(!press(&rating, Key::End).handled());
    rating.select_rating(2.0, true);
    assert_eq!(rating.value(), -1.0);
    rating.set_rating_value(3.0);
    assert_eq!(rating.value(), 3.0);
    rating.set_is_read_only(false);
    rating.set_is_enabled(false);
    assert!(!press(&rating, Key::Home).handled());
    assert_eq!(rating.value(), 3.0);
    assert_eq!(count.get(), 0);
}

#[test]
fn repeated_pointer_selection_clears_including_maximum_and_respects_clear_disabled() {
    let _app = start();
    let rating = RatingControl::new();
    rating.select_rating(5.0, true);
    rating.select_rating(5.0, true);
    assert_eq!(rating.value(), -1.0);
    rating.set_is_clear_enabled(false);
    rating.select_rating(5.0, true);
    rating.select_rating(5.0, true);
    assert_eq!(rating.value(), 5.0);
    rating.select_rating(0.0, true);
    assert_eq!(rating.value(), 1.0);
}

#[test]
fn template_clips_fractional_values_and_retains_native_item_identity_during_edits() {
    let _app = start();
    let rating = RatingControl::new();
    rating.set_placeholder_value(2.5);
    let foreground = panel(&rating, true);
    assert_eq!(foreground.children().count(), 5);
    let first = item(&foreground, 0);
    let width = |i| {
        item(&foreground, i)
            .clip()
            .unwrap()
            .cast::<RectangleGeometry>()
            .unwrap()
            .rect()
            .width
    };
    assert_eq!(width(0), 32.0);
    assert_eq!(width(1), 32.0);
    assert_eq!(width(2), 16.0);
    assert_eq!(width(3), 0.0);
    rating.set_rating_value(1.25);
    assert_eq!(item(&foreground, 0), first);
    assert_eq!(width(0), 32.0);
    assert_eq!(width(1), 8.0);
    assert_eq!(width(2), 0.0);
    rating.set_max_rating(3);
    assert_eq!(foreground.children().count(), 3);
}

#[test]
fn scoped_zero_item_spacing_is_respected_without_changing_other_controls() {
    let _app = start();
    let rating = RatingControl::new();
    rating
        .resources()
        .add_value("RatingControlItemSpacing", 0.0_f64);
    let other = RatingControl::new();
    assert_eq!(panel(&rating, false).spacing(), 0.0);
    assert_eq!(panel(&other, false).spacing(), 8.0);
}

#[test]
fn glyph_fallbacks_and_live_item_info_updates_use_the_original_chains() {
    let _app = start();
    let rating = RatingControl::new();
    let info = RatingItemFontInfo::new();
    info.set_glyph("S");
    info.set_placeholder_glyph("P");
    info.set_unset_glyph("U");
    rating.set_item_info(Some(info.clone().upcast()));
    rating.set_placeholder_value(3.5);
    let background = panel(&rating, false);
    let foreground = panel(&rating, true);
    let bg = item(&background, 0).cast::<TextBlock>().unwrap();
    let fg = item(&foreground, 0).cast::<TextBlock>().unwrap();
    assert_eq!(bg.text().as_deref(), Some("U"));
    assert_eq!(fg.text().as_deref(), Some("P"));
    info.set_placeholder_glyph("");
    assert_eq!(fg.text().as_deref(), Some("S"));
    rating.set_is_enabled(false);
    info.set_disabled_glyph("D");
    assert_eq!(fg.text().as_deref(), Some("D"));
    info.set_disabled_glyph("");
    assert_eq!(fg.text().as_deref(), Some("S"));
}

#[test]
fn image_info_uses_native_images_and_releases_replaced_descriptors() {
    let _app = start();
    let rating = RatingControl::new();
    let info = RatingItemImageInfo::new();
    let source: Rc<dyn IImage> = DrawingImage::new().into();
    let placeholder: Rc<dyn IImage> = DrawingImage::new().into();
    info.set_image(Some(source.clone()));
    info.set_placeholder_image(Some(placeholder.clone()));
    rating.set_item_info(Some(info.clone().upcast()));
    rating.set_placeholder_value(2.5);
    let foreground = panel(&rating, true);
    let image = item(&foreground, 0).cast::<Image>().unwrap();
    assert!(Rc::ptr_eq(&image.source().unwrap(), &placeholder));
    info.set_placeholder_image(None);
    assert!(Rc::ptr_eq(&image.source().unwrap(), &source));
    let weak = info.downgrade();
    rating.set_item_info(None);
    drop(info);
    assert!(weak.upgrade().is_none());
    assert_eq!(foreground.children().count(), 0);
}

#[test]
fn default_glyph_descriptors_and_value_handlers_do_not_retain_controls() {
    let _app = start();
    let first = RatingControl::new();
    let second = RatingControl::new();
    assert_ne!(first.item_info(), second.item_info());
    let info = first
        .item_info()
        .unwrap()
        .cast::<RatingItemFontInfo>()
        .unwrap();
    info.set_glyph("X");
    assert_eq!(
        second
            .item_info()
            .unwrap()
            .cast::<RatingItemFontInfo>()
            .unwrap()
            .glyph(),
        "\u{e735}"
    );
    let weak = first.downgrade();
    let subscription = first.value_changed(|_| {});
    drop(first);
    assert!(weak.upgrade().is_none());
    subscription.dispose();
    info.set_glyph("Y");
}

#[test]
fn caption_is_native_text_and_empty_caption_does_not_reserve_spacing() {
    let _app = start();
    let rating = RatingControl::new();
    let window = Window::new();
    window.set_width(600.0);
    window.set_height(100.0);
    window.set_content(Some(Control::boxed(rating.clone())));
    window.show();
    // Child text/visibility invalidation is queued on the layout manager.
    // Repeating Measure on an unattached, valid ancestor does not drain that
    // queue and would continue to return the ancestor's cached DesiredSize.
    window.layout_manager().execute_layout_pass();
    let width_without_caption = rating.desired_size().width;
    assert_eq!(
        width_without_caption,
        panel(&rating, false).desired_size().width
    );
    let caption = rating
        .get_template_descendants()
        .into_iter()
        .find(|child| child.name().as_deref() == Some("Caption"))
        .unwrap()
        .cast::<TextBlock>()
        .unwrap();
    assert!(!caption.is_visible());

    rating.set_caption("Your rating");
    window.layout_manager().execute_layout_pass();
    assert!(caption.is_visible());
    assert_eq!(caption.text().as_deref(), Some("Your rating"));
    assert!(caption.desired_size().width > 0.0);
    let width_with_caption = rating.desired_size().width;
    assert!(width_with_caption > width_without_caption + caption.desired_size().width);

    // A visible-to-visible text change must also remeasure the ancestor.
    rating.set_caption("Your rating, based on this review");
    window.layout_manager().execute_layout_pass();
    assert!(rating.desired_size().width > width_with_caption);

    rating.set_caption("");
    window.layout_manager().execute_layout_pass();
    assert!(!caption.is_visible());
    assert_eq!(rating.desired_size().width, width_without_caption);
    window.close();
}

#[test]
fn automation_exposes_value_and_range_patterns_with_unset_zero_projection() {
    let _app = start();
    let rating = RatingControl::new();
    rating.set_caption("Overall rating");
    let peer = ControlAutomationPeer::create_peer_for_element(&rating);
    assert_eq!(peer.get_class_name(), "RatingControl");
    assert_eq!(
        peer.get_automation_control_type(),
        AutomationControlType::Slider
    );
    assert_eq!(peer.get_name(), "Overall rating");
    let range = peer.get_provider::<dyn IRangeValueProvider>().unwrap();
    let text = peer.get_provider::<dyn IValueProvider>().unwrap();
    assert_eq!(range.minimum(), 0.0);
    assert_eq!(range.maximum(), 5.0);
    assert_eq!(range.value(), 0.0);
    assert_eq!(range.small_change(), 1.0);
    assert_eq!(range.large_change(), 1.0);
    assert!(text.value().unwrap().contains("Unset"));
    rating.set_placeholder_value(0.5);
    assert!(text.value().unwrap().contains("0.5 of 5"));
    assert_eq!(range.value(), 0.0);
    rating.set_is_read_only(true);
    assert!(range.is_read_only());
    assert!(text.is_read_only());
    range.set_value(4.25).unwrap();
    assert_eq!(rating.value(), 4.25);
    text.set_value(Some("2.5")).unwrap();
    assert_eq!(rating.value(), 2.5);
    text.set_value(Some("invalid")).unwrap();
    assert_eq!(rating.value(), 2.5);
}

#[test]
fn automation_notifications_report_coerced_values_and_current_text_with_correct_types() {
    use ferroui_controls::automation::{
        RangeValuePatternIdentifiers as Range, ValuePatternIdentifiers as Value,
    };
    let _app = start();
    let rating = RatingControl::new();
    let peer = ControlAutomationPeer::create_peer_for_element(&rating);
    let recorded = Rc::new(RefCell::new(Vec::new()));
    let log = recorded.clone();
    peer.property_changed(move |args| log.borrow_mut().push(args.clone()));
    rating.set_rating_value(99.0);
    let events = recorded.borrow();
    let values: Vec<_> = events
        .iter()
        .filter(|e| std::ptr::eq(e.property(), Range::value_property()))
        .collect();
    assert_eq!(values.len(), 1);
    assert_eq!(
        values[0].old_value().unwrap().downcast_ref::<f64>(),
        Some(&0.0)
    );
    assert_eq!(
        values[0].new_value().unwrap().downcast_ref::<f64>(),
        Some(&5.0)
    );
    let text = events
        .iter()
        .find(|e| std::ptr::eq(e.property(), Value::value_property()))
        .unwrap();
    assert_eq!(
        text.old_value().unwrap().downcast_ref::<String>().unwrap(),
        "Rating Unset"
    );
    assert_eq!(
        text.new_value().unwrap().downcast_ref::<String>().unwrap(),
        "Rating, 5 of 5"
    );
    drop(events);
    recorded.borrow_mut().clear();
    rating.set_max_rating(0);
    rating.set_is_read_only(true);
    let events = recorded.borrow();
    let maximum = events
        .iter()
        .find(|e| std::ptr::eq(e.property(), Range::maximum_property()))
        .unwrap();
    assert_eq!(
        maximum.old_value().unwrap().downcast_ref::<f64>(),
        Some(&5.0)
    );
    assert_eq!(
        maximum.new_value().unwrap().downcast_ref::<f64>(),
        Some(&1.0)
    );
    let readonly = events
        .iter()
        .find(|e| std::ptr::eq(e.property(), Value::is_read_only_property()))
        .unwrap();
    assert_eq!(
        readonly.new_value().unwrap().downcast_ref::<bool>(),
        Some(&true)
    );
}

fn pointer_press(rating: &RatingControl, window: &Window, pointer: Rc<dyn IPointer>, local: Point) {
    let background = panel(rating, false);
    let point = background.translate_point(local, window).unwrap();
    let args = PointerPressedEventArgs::new(
        rating.to_ref(),
        pointer,
        window,
        point,
        1,
        PointerPointProperties {
            is_left_button_pressed: true,
            ..PointerPointProperties::default()
        },
        KeyModifiers::NONE,
        1,
    );
    rating.raise_event(&args);
    assert!(args.handled());
}

fn pointer_release(
    rating: &RatingControl,
    window: &Window,
    pointer: Rc<dyn IPointer>,
    local: Point,
) {
    let background = panel(rating, false);
    let point = background.translate_point(local, window).unwrap();
    let args = PointerReleasedEventArgs::new(
        rating.to_ref(),
        pointer,
        window,
        point,
        2,
        PointerPointProperties::default(),
        KeyModifiers::NONE,
        MouseButton::Left,
    );
    rating.raise_event(&args);
    assert!(args.handled());
}

#[test]
fn native_pointer_capture_selects_clears_and_supports_drag_before_the_first_item() {
    let _app = start();
    let rating = RatingControl::new();
    let window = Window::new();
    window.set_content(Some(Control::boxed(rating.clone())));
    window.show();
    window.layout_manager().execute_layout_pass();
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let width = panel(&rating, false).bounds().width;
    let position = Point::new(width * 0.5, 16.0);
    pointer_press(&rating, &window, pointer.clone(), position);
    assert!(pointer.captured().is_some());
    pointer_release(&rating, &window, pointer.clone(), position);
    assert_eq!(rating.value(), 3.0);
    assert!(pointer.captured().is_none());
    pointer_press(&rating, &window, pointer.clone(), position);
    pointer_release(&rating, &window, pointer.clone(), position);
    assert_eq!(rating.value(), -1.0);
    rating.set_rating_value(4.0);
    pointer_press(&rating, &window, pointer.clone(), position);
    pointer_release(&rating, &window, pointer.clone(), Point::new(-20.0, 16.0));
    assert_eq!(rating.value(), -1.0);
    window.close();
}

#[test]
fn changing_read_only_during_capture_cancels_without_committing_a_preview() {
    let _app = start();
    let rating = RatingControl::new();
    let window = Window::new();
    window.set_content(Some(Control::boxed(rating.clone())));
    window.show();
    window.layout_manager().execute_layout_pass();
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Touch, true);
    pointer_press(&rating, &window, pointer.clone(), Point::new(30.0, 16.0));
    rating.set_is_read_only(true);
    assert!(pointer.captured().is_none());
    assert_eq!(rating.value(), -1.0);
    window.close();
}

#[test]
fn rtl_pointer_coordinates_follow_the_native_mirror_once() {
    let _app = start();
    let rating = RatingControl::new();
    rating.set_flow_direction(FlowDirection::RightToLeft);
    let window = Window::new();
    window.set_content(Some(Control::boxed(rating.clone())));
    window.show();
    window.layout_manager().execute_layout_pass();
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let background = panel(&rating, false);
    // The panel's first logical item is physically on the right in RTL.
    let position = Point::new(background.bounds().width * 0.1, 16.0);
    pointer_press(&rating, &window, pointer.clone(), position);
    pointer_release(&rating, &window, pointer.clone(), position);
    assert_eq!(rating.value(), 1.0);
    window.close();
}

#[test]
fn another_pointer_losing_capture_and_secondary_release_do_not_commit_active_drag() {
    use ferroui_base::input::PointerCaptureLostEventArgs;
    let _app = start();
    let rating = RatingControl::new();
    let window = Window::new();
    window.set_content(Some(Control::boxed(rating.clone())));
    window.show();
    window.layout_manager().execute_layout_pass();
    let active = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let other = Pointer::new(Pointer::get_next_free_id(), PointerType::Touch, false);
    pointer_press(&rating, &window, active.clone(), Point::new(30.0, 16.0));
    rating.raise_event(&PointerCaptureLostEventArgs::new(rating.clone(), other));
    assert!(active.captured().is_some());
    let point = panel(&rating, false)
        .translate_point(Point::new(30.0, 16.0), &window)
        .unwrap();
    let secondary = PointerReleasedEventArgs::new(
        rating.clone(),
        active.clone(),
        &window,
        point,
        2,
        PointerPointProperties {
            is_left_button_pressed: true,
            ..PointerPointProperties::default()
        },
        KeyModifiers::NONE,
        MouseButton::Right,
    );
    rating.raise_event(&secondary);
    assert!(!secondary.handled());
    assert_eq!(rating.value(), -1.0);
    assert!(active.captured().is_some());
    pointer_release(&rating, &window, active.clone(), Point::new(30.0, 16.0));
    assert_eq!(rating.value(), 2.0);
    window.close();
}

#[test]
fn canceled_capture_acquisition_leaves_no_active_drag_or_value_edit() {
    let _app = start();
    let rating = RatingControl::new();
    let window = Window::new();
    window.set_content(Some(Control::boxed(rating.clone())));
    window.show();
    window.layout_manager().execute_layout_pass();
    rating.add_handler(InputElement::pointer_capture_changing_event(), |_, args| {
        if args.new_value().is_some() {
            args.set_handled(true);
        }
    });
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let point = panel(&rating, false)
        .translate_point(Point::new(30.0, 16.0), &window)
        .unwrap();
    let pressed = PointerPressedEventArgs::new(
        rating.clone(),
        pointer.clone(),
        &window,
        point,
        1,
        PointerPointProperties {
            is_left_button_pressed: true,
            ..PointerPointProperties::default()
        },
        KeyModifiers::NONE,
        1,
    );
    rating.raise_event(&pressed);
    assert!(!pressed.handled());
    assert!(pointer.captured().is_none());
    let released = PointerReleasedEventArgs::new(
        rating.clone(),
        pointer.clone(),
        &window,
        point,
        2,
        PointerPointProperties::default(),
        KeyModifiers::NONE,
        MouseButton::Left,
    );
    rating.raise_event(&released);
    assert!(!released.handled());
    assert_eq!(rating.value(), -1.0);
    window.close();
}
