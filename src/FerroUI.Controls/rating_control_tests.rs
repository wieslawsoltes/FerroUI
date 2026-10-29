//! Tests of `RatingControl` and the rating item infos (WinUI; docs/porting/winui-controls.md,
//! Input), adapted from the tests of #286 (`tests/winui_rating_control.rs`).

use crate::automation::peers::{AutomationControlType, ControlAutomationPeer};
use crate::automation::provider::{IRangeValueProvider, IValueProvider};
use crate::automation::{RangeValuePatternIdentifiers as Range, ValuePatternIdentifiers as Value};
use crate::input_test_support::{part, rating_control_template};
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{Control, Grid, Image, RatingControl, RatingItemFontInfo, RatingItemImageInfo, StackPanel, TextBlock, Window};
use ferroui_base::input::{
    IPointer, InputElement, Key, KeyEventArgs, KeyModifiers, MouseButton, Pointer, PointerCaptureLostEventArgs,
    PointerPointProperties, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
};
use ferroui_base::layout::ILayoutManager;
use ferroui_base::media::{DrawingImage, FlowDirection, IImage, RectangleGeometry};
use ferroui_base::{Point, Ref};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn start() -> UnitTestApplicationScope {
    crate::register_types();
    UnitTestApplication::start(TestServices::styled_window())
}

fn rating() -> Ref<RatingControl> {
    let rating = RatingControl::new();
    rating.set_template(Some(rating_control_template()));
    rating
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
    part::<StackPanel>(control, if foreground { "RatingForegroundStackPanel" } else { "RatingBackgroundStackPanel" })
}

fn item(panel: &StackPanel, index: usize) -> Ref<Control> {
    panel.children().get(index).cast::<Grid>().unwrap().children().get(0)
}

fn show(rating: &RatingControl) -> Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(rating.to_ref())));
    window.show();
    window.layout_manager().execute_layout_pass();
    window
}

#[test]
fn the_rating_and_the_placeholder_are_unset_by_default_and_coerced() {
    let _app = start();
    let rating = RatingControl::new();
    assert_eq!(-1.0, rating.value());
    assert_eq!(-1.0, rating.placeholder_value());
    assert_eq!(5, rating.max_rating());
    assert_eq!(1, rating.initial_set_value());
    assert!(rating.is_clear_enabled());
    assert!(!rating.is_read_only());
    assert_eq!("", rating.caption());
    assert!(rating.focusable());
    // CoerceValueBetweenMinAndMax and CoercePlaceholderValueBetweenMinAndMax.
    rating.set_rating_value(0.2);
    rating.set_placeholder_value(0.2);
    assert_eq!(1.0, rating.value());
    assert_eq!(0.2, rating.placeholder_value());
    rating.set_placeholder_value(0.0);
    assert_eq!(0.0, rating.placeholder_value());
    rating.set_rating_value(-9.0);
    rating.set_placeholder_value(-9.0);
    assert_eq!(-1.0, rating.value());
    assert_eq!(-1.0, rating.placeholder_value());
    // A NaN rating is not set.
    rating.set_rating_value(3.0);
    rating.set_rating_value(f64::NAN);
    assert_eq!(-1.0, rating.value());
}

#[test]
fn a_smaller_maximum_clamps_the_values_once() {
    let _app = start();
    let rating = RatingControl::new();
    rating.set_rating_value(4.5);
    rating.set_placeholder_value(3.5);
    rating.set_max_rating(-2);
    assert_eq!(1, rating.max_rating());
    assert_eq!(1.0, rating.value());
    assert_eq!(1.0, rating.placeholder_value());
    rating.set_max_rating(10);
    assert_eq!(1.0, rating.value());
    assert_eq!(1.0, rating.placeholder_value());
    rating.set_rating_value(f64::INFINITY);
    assert_eq!(10.0, rating.value());
    rating.set_rating_value(f64::NEG_INFINITY);
    assert_eq!(-1.0, rating.value());
}

#[test]
fn only_the_changes_of_the_user_raise_value_changed() {
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
    assert_eq!(vec![3.0], *observed.borrow());
    assert_eq!(4.0, rating.value());
    subscription.dispose();
    press(&rating, Key::Home);
    assert_eq!(vec![3.0], *observed.borrow());
    assert_eq!(-1.0, rating.value());
}

#[test]
fn the_keys_start_from_the_initial_value_step_whole_ratings_and_follow_is_clear_enabled() {
    let _app = start();
    let rating = RatingControl::new();
    rating.set_initial_set_value(3);
    press(&rating, Key::Left);
    assert_eq!(3.0, rating.value());
    rating.set_rating_value(3.75);
    press(&rating, Key::Down);
    assert_eq!(3.0, rating.value());
    rating.set_rating_value(3.75);
    press(&rating, Key::Up);
    assert_eq!(4.0, rating.value());
    press(&rating, Key::End);
    press(&rating, Key::Right);
    assert_eq!(5.0, rating.value());
    press(&rating, Key::Home);
    assert_eq!(-1.0, rating.value());
    rating.set_is_clear_enabled(false);
    press(&rating, Key::End);
    press(&rating, Key::Home);
    assert_eq!(1.0, rating.value());
    press(&rating, Key::Left);
    assert_eq!(1.0, rating.value());
}

#[test]
fn left_and_right_are_reversed_right_to_left_and_up_and_down_are_not() {
    let _app = start();
    let rating = RatingControl::new();
    rating.set_flow_direction(FlowDirection::RightToLeft);
    rating.set_rating_value(3.0);
    press(&rating, Key::Right);
    assert_eq!(2.0, rating.value());
    press(&rating, Key::Left);
    assert_eq!(3.0, rating.value());
    press(&rating, Key::Up);
    assert_eq!(4.0, rating.value());
    press(&rating, Key::Down);
    assert_eq!(3.0, rating.value());
}

#[test]
fn read_only_and_disabled_block_the_user_and_not_the_application() {
    let _app = start();
    let rating = RatingControl::new();
    let count = Rc::new(Cell::new(0));
    let log = count.clone();
    rating.value_changed(move |_| log.set(log.get() + 1));
    rating.set_is_read_only(true);
    assert!(!press(&rating, Key::End).handled());
    rating.set_rating_to(2.0, true);
    assert_eq!(-1.0, rating.value());
    rating.set_rating_value(3.0);
    assert_eq!(3.0, rating.value());
    rating.set_is_read_only(false);
    rating.set_is_enabled(false);
    assert!(!press(&rating, Key::Home).handled());
    assert_eq!(3.0, rating.value());
    assert_eq!(0, count.get());
}

#[test]
fn choosing_the_rating_again_clears_it_unless_clearing_is_disabled() {
    let _app = start();
    let rating = RatingControl::new();
    rating.set_rating_to(5.0, true);
    rating.set_rating_to(5.0, true);
    assert_eq!(-1.0, rating.value());
    rating.set_is_clear_enabled(false);
    rating.set_rating_to(5.0, true);
    rating.set_rating_to(5.0, true);
    assert_eq!(5.0, rating.value());
    rating.set_rating_to(0.0, true);
    assert_eq!(1.0, rating.value());
}

#[test]
fn the_foreground_items_are_clipped_to_the_value_and_kept_while_it_changes() {
    let _app = start();
    let rating = rating();
    rating.set_placeholder_value(2.5);
    let foreground = panel(&rating, true);
    assert_eq!(5, foreground.children().count());
    let first = item(&foreground, 0);
    let width = |i| item(&foreground, i).clip().unwrap().cast::<RectangleGeometry>().unwrap().rect().width;
    // The size of the items is the font size of the panels (RatingControlFontSizeForRendering).
    assert_eq!(32.0, width(0));
    assert_eq!(32.0, width(1));
    assert_eq!(16.0, width(2));
    assert_eq!(0.0, width(3));
    rating.set_rating_value(1.25);
    assert_eq!(first, item(&foreground, 0));
    assert_eq!(32.0, width(0));
    assert_eq!(8.0, width(1));
    assert_eq!(0.0, width(2));
    rating.set_max_rating(3);
    assert_eq!(3, foreground.children().count());
}

#[test]
fn the_states_are_pseudo_classes() {
    let _app = start();
    let rating = rating();
    rating.apply_template();
    assert!(!rating.classes().contains(":set"));
    assert!(!rating.classes().contains(":placeholder"));
    rating.set_placeholder_value(2.0);
    assert!(rating.classes().contains(":placeholder"));
    rating.set_rating_value(3.0);
    assert!(rating.classes().contains(":set"));
    assert!(!rating.classes().contains(":placeholder"));
}

#[test]
fn the_glyphs_fall_back_as_in_winui_and_follow_the_item_info() {
    let _app = start();
    let rating = rating();
    let info = RatingItemFontInfo::new();
    info.set_glyph("S");
    info.set_placeholder_glyph("P");
    info.set_unset_glyph("U");
    rating.set_item_info(Some(info.clone().upcast()));
    rating.set_placeholder_value(3.5);
    let background = panel(&rating, false);
    let foreground = panel(&rating, true);
    let unset = item(&background, 0).cast::<TextBlock>().unwrap();
    let set = item(&foreground, 0).cast::<TextBlock>().unwrap();
    assert_eq!(Some("U"), unset.text().as_deref());
    assert_eq!(Some("P"), set.text().as_deref());
    info.set_placeholder_glyph("");
    assert_eq!(Some("S"), set.text().as_deref());
    rating.set_is_enabled(false);
    info.set_disabled_glyph("D");
    assert_eq!(Some("D"), set.text().as_deref());
    info.set_disabled_glyph("");
    assert_eq!(Some("S"), set.text().as_deref());
}

#[test]
fn an_image_info_draws_images_and_a_replaced_info_is_released() {
    let _app = start();
    let rating = rating();
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
    assert_eq!(0, foreground.children().count());
}

#[test]
fn each_control_has_its_own_default_glyphs_and_handlers_do_not_keep_it_alive() {
    let _app = start();
    let first = RatingControl::new();
    let second = RatingControl::new();
    assert_ne!(first.item_info(), second.item_info());
    let info = first.item_info().unwrap().cast::<RatingItemFontInfo>().unwrap();
    assert_eq!("\u{E735}", info.glyph());
    assert_eq!("\u{E734}", info.unset_glyph());
    info.set_glyph("X");
    assert_eq!("\u{E735}", second.item_info().unwrap().cast::<RatingItemFontInfo>().unwrap().glyph());
    let weak = first.downgrade();
    let subscription = first.value_changed(|_| {});
    drop(first);
    assert!(weak.upgrade().is_none());
    subscription.dispose();
    info.set_glyph("Y");
}

#[test]
fn the_peer_has_the_value_and_range_value_patterns_with_an_unset_rating_of_zero() {
    let _app = start();
    let rating = RatingControl::new();
    rating.set_caption("Overall rating");
    let peer = ControlAutomationPeer::create_peer_for_element(&rating);
    assert_eq!("RatingControl", peer.get_class_name());
    assert_eq!(AutomationControlType::Slider, peer.get_automation_control_type());
    assert_eq!("Overall rating", peer.get_name());
    let range = peer.get_provider::<dyn IRangeValueProvider>().unwrap();
    let text = peer.get_provider::<dyn IValueProvider>().unwrap();
    assert_eq!(0.0, range.minimum());
    assert_eq!(5.0, range.maximum());
    assert_eq!(0.0, range.value());
    assert_eq!(1.0, range.small_change());
    assert_eq!(1.0, range.large_change());
    assert_eq!(Some("Rating Unset"), text.value().as_deref());
    rating.set_placeholder_value(0.5);
    assert_eq!(Some("Community Rating, 0.5 of 5"), text.value().as_deref());
    assert_eq!(0.0, range.value());
    rating.set_is_read_only(true);
    assert!(range.is_read_only());
    assert!(text.is_read_only());
    range.set_value(4.25).unwrap();
    assert_eq!(4.25, rating.value());
    text.set_value(Some("2.5")).unwrap();
    assert_eq!(2.5, rating.value());
    text.set_value(Some("invalid")).unwrap();
    assert_eq!(2.5, rating.value());
}

#[test]
fn the_peer_reports_the_coerced_values_and_the_text_of_the_rating() {
    let _app = start();
    let rating = RatingControl::new();
    let peer = ControlAutomationPeer::create_peer_for_element(&rating);
    let recorded = Rc::new(RefCell::new(Vec::new()));
    let log = recorded.clone();
    peer.property_changed(move |args| log.borrow_mut().push(args.clone()));
    rating.set_rating_value(99.0);
    {
        let events = recorded.borrow();
        let values: Vec<_> = events.iter().filter(|e| std::ptr::eq(e.property(), Range::value_property())).collect();
        assert_eq!(1, values.len());
        assert_eq!(Some(&0.0), values[0].old_value().unwrap().downcast_ref::<f64>());
        assert_eq!(Some(&5.0), values[0].new_value().unwrap().downcast_ref::<f64>());
        let text = events.iter().find(|e| std::ptr::eq(e.property(), Value::value_property())).unwrap();
        assert_eq!("Rating Unset", text.old_value().unwrap().downcast_ref::<String>().unwrap());
        assert_eq!("Rating, 5 of 5", text.new_value().unwrap().downcast_ref::<String>().unwrap());
    }
    recorded.borrow_mut().clear();
    rating.set_max_rating(0);
    rating.set_is_read_only(true);
    let events = recorded.borrow();
    let maximum = events.iter().find(|e| std::ptr::eq(e.property(), Range::maximum_property())).unwrap();
    assert_eq!(Some(&5.0), maximum.old_value().unwrap().downcast_ref::<f64>());
    assert_eq!(Some(&1.0), maximum.new_value().unwrap().downcast_ref::<f64>());
    let read_only = events.iter().find(|e| std::ptr::eq(e.property(), Value::is_read_only_property())).unwrap();
    assert_eq!(Some(&true), read_only.new_value().unwrap().downcast_ref::<bool>());
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
        PointerPointProperties { is_left_button_pressed: true, ..PointerPointProperties::default() },
        KeyModifiers::NONE,
        1,
    );
    rating.raise_event(&args);
    assert!(args.handled());
}

fn pointer_release(rating: &RatingControl, window: &Window, pointer: Rc<dyn IPointer>, local: Point) {
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
fn the_pointer_sets_and_clears_the_rating_and_a_drag_before_the_first_item_clears_it() {
    let _app = start();
    let rating = rating();
    let window = show(&rating);
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let width = panel(&rating, false).bounds().width;
    let position = Point::new(width * 0.5, 16.0);
    pointer_press(&rating, &window, pointer.clone(), position);
    assert!(pointer.captured().is_some());
    pointer_release(&rating, &window, pointer.clone(), position);
    assert_eq!(3.0, rating.value());
    assert!(pointer.captured().is_none());
    pointer_press(&rating, &window, pointer.clone(), position);
    pointer_release(&rating, &window, pointer.clone(), position);
    assert_eq!(-1.0, rating.value());
    rating.set_rating_value(4.0);
    pointer_press(&rating, &window, pointer.clone(), position);
    pointer_release(&rating, &window, pointer.clone(), Point::new(-20.0, 16.0));
    assert_eq!(-1.0, rating.value());
    window.close();
}

#[test]
fn read_only_during_a_capture_cancels_it_without_a_change() {
    let _app = start();
    let rating = rating();
    let window = show(&rating);
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Touch, true);
    pointer_press(&rating, &window, pointer.clone(), Point::new(30.0, 16.0));
    rating.set_is_read_only(true);
    assert!(pointer.captured().is_none());
    assert_eq!(-1.0, rating.value());
    window.close();
}

#[test]
fn right_to_left_coordinates_are_mirrored_once() {
    let _app = start();
    let rating = rating();
    rating.set_flow_direction(FlowDirection::RightToLeft);
    let window = show(&rating);
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let background = panel(&rating, false);
    // The first item of the panel is at its right in a right-to-left flow.
    let position = Point::new(background.bounds().width * 0.1, 16.0);
    pointer_press(&rating, &window, pointer.clone(), position);
    pointer_release(&rating, &window, pointer.clone(), position);
    assert_eq!(1.0, rating.value());
    window.close();
}

#[test]
fn another_pointer_and_another_button_do_not_end_the_drag() {
    let _app = start();
    let rating = rating();
    let window = show(&rating);
    let active = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let other = Pointer::new(Pointer::get_next_free_id(), PointerType::Touch, false);
    pointer_press(&rating, &window, active.clone(), Point::new(30.0, 16.0));
    rating.raise_event(&PointerCaptureLostEventArgs::new(rating.clone(), other));
    assert!(active.captured().is_some());
    let point = panel(&rating, false).translate_point(Point::new(30.0, 16.0), &window).unwrap();
    let secondary = PointerReleasedEventArgs::new(
        rating.clone(),
        active.clone(),
        &window,
        point,
        2,
        PointerPointProperties { is_left_button_pressed: true, ..PointerPointProperties::default() },
        KeyModifiers::NONE,
        MouseButton::Right,
    );
    rating.raise_event(&secondary);
    assert!(!secondary.handled());
    assert_eq!(-1.0, rating.value());
    assert!(active.captured().is_some());
    pointer_release(&rating, &window, active.clone(), Point::new(30.0, 16.0));
    assert_eq!(2.0, rating.value());
    window.close();
}

#[test]
fn a_refused_capture_leaves_no_drag_and_no_change() {
    let _app = start();
    let rating = rating();
    let window = show(&rating);
    rating.add_handler(InputElement::pointer_capture_changing_event(), |_, args| {
        if args.new_value().is_some() {
            args.set_handled(true);
        }
    });
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let point = panel(&rating, false).translate_point(Point::new(30.0, 16.0), &window).unwrap();
    let pressed = PointerPressedEventArgs::new(
        rating.clone(),
        pointer.clone(),
        &window,
        point,
        1,
        PointerPointProperties { is_left_button_pressed: true, ..PointerPointProperties::default() },
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
    assert_eq!(-1.0, rating.value());
    window.close();
}
