//! Port of `PopupTests.cs` of the upstream unit test project of the
//! headless platform.

use super::test_application::ferro_fact;
use crate::HeadlessWindowExtensions;
use ferroui_base::input::{MouseButton, RawInputModifiers};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::Brushes;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{PixelPoint, Point, Ref, Visual};
use ferroui_controls::primitives::{Popup, PopupRoot};
use ferroui_controls::{Border, Button, Control, Panel, PlacementMode, TopLevel, Window};
use std::cell::Cell;
use std::rc::Rc;

fn same_top_level(expected: &Option<Ref<TopLevel>>, actual: &Option<Ref<TopLevel>>) -> bool {
    match (expected, actual) {
        (Some(expected), Some(actual)) => expected.ptr_eq(actual),
        (None, None) => true,
        _ => false,
    }
}

/// A window of 100 by 100 whose content is a panel with `target` and `popup`.
fn window_with(target: &Ref<Border>, popup: &Ref<Popup>) -> Ref<Window> {
    let panel = Panel::new();
    panel.children().add(target);
    panel.children().add(popup);
    let window = Window::new();
    window.set_width(100.0);
    window.set_height(100.0);
    window.set_content(Some(Control::boxed(panel)));
    window
}

fn sized_border(width: f64, height: f64) -> Ref<Border> {
    let border = Border::new();
    border.set_width(width);
    border.set_height(height);
    border
}

fn point_to_screen_respects_window_position() {
    let window = Window::new();
    window.set_width(100.0);
    window.set_height(100.0);
    window.set_position(PixelPoint::new(100, 200));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);

    assert_eq!(PixelPoint::new(110, 220), window.point_to_screen(Point::new(10.0, 20.0)));
    assert_eq!(Point::new(10.0, 20.0), window.point_to_client(PixelPoint::new(110, 220)));

    window.close();
}
ferro_fact!(point_to_screen_respects_window_position);

fn popup_uses_dedicated_top_level() {
    let target = Border::new();
    target.set_background(Some(Brushes::red()));
    let popup = Popup::new();
    popup.set_placement_target(target.clone());
    popup.set_child(sized_border(20.0, 20.0));
    let window = window_with(&target, &popup);
    window.show();
    Dispatcher::ui_thread().run_jobs(None);

    assert!(!popup.is_open());
    assert!(!popup.is_using_overlay_layer());
    assert!(get_popup_top_level(&popup).is_none());

    popup.open();
    Dispatcher::ui_thread().run_jobs(None);

    assert!(popup.is_open());
    assert!(!popup.is_using_overlay_layer());
    assert!(get_popup_top_level(&popup).is_some_and(|top_level| top_level.is::<PopupRoot>()));

    window.close();

    assert!(!popup.is_open());
    assert!(!popup.is_using_overlay_layer());
    assert!(get_popup_top_level(&popup).is_none());
}
ferro_fact!(popup_uses_dedicated_top_level);

fn popup_placement_respects_window_position() {
    let target = sized_border(20.0, 20.0);
    target.set_horizontal_alignment(HorizontalAlignment::Left);
    target.set_vertical_alignment(VerticalAlignment::Top);
    target.set_background(Some(Brushes::red()));
    let popup = Popup::new();
    popup.set_placement_target(target.clone());
    popup.set_placement(PlacementMode::Bottom);
    popup.set_child(sized_border(20.0, 20.0));
    let window = window_with(&target, &popup);
    window.set_position(PixelPoint::new(100, 200));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);

    popup.open();
    Dispatcher::ui_thread().run_jobs(None);

    let popup_root = get_popup_top_level(&popup);
    assert!(popup_root.is_some());
    let popup_root = popup_root.unwrap();

    let expected = target.point_to_screen(Point::new(0.0, target.bounds().height));
    assert_eq!(expected, popup_root.point_to_screen(Point::default()));

    window.close();
}
ferro_fact!(popup_placement_respects_window_position);

fn can_click_button_inside_platform_popup() {
    let click_count = Rc::new(Cell::new(0));
    let button = Button::new();
    button.set_width(80.0);
    button.set_height(30.0);
    button.click({
        let click_count = click_count.clone();
        move |_, _| click_count.set(click_count.get() + 1)
    });

    let target = Border::new();
    target.set_background(Some(Brushes::red()));
    let popup = Popup::new();
    popup.set_placement_target(target.clone());
    popup.set_child(button.clone());
    let window = window_with(&target, &popup);
    window.show();
    Dispatcher::ui_thread().run_jobs(None);

    popup.open();
    Dispatcher::ui_thread().run_jobs(None);

    let popup_root = get_popup_top_level(&popup);
    assert!(popup_root.is_some());
    let popup_root = popup_root.unwrap();

    popup_root.mouse_down(Point::new(40.0, 15.0), MouseButton::Left, RawInputModifiers::NONE);
    popup_root.mouse_up(Point::new(40.0, 15.0), MouseButton::Left, RawInputModifiers::NONE);

    assert_eq!(1, click_count.get());

    window.close();
}
ferro_fact!(can_click_button_inside_platform_popup, shows_a_window);

fn nested_popup_is_owned_by_parent_popup() {
    let nested_target = sized_border(20.0, 20.0);
    nested_target.set_background(Some(Brushes::green()));
    let nested_popup = Popup::new();
    nested_popup.set_placement_target(nested_target.clone());
    nested_popup.set_child(sized_border(10.0, 10.0));
    let target = Border::new();
    target.set_background(Some(Brushes::red()));
    let popup = Popup::new();
    popup.set_placement_target(target.clone());
    let popup_panel = Panel::new();
    popup_panel.children().add(&nested_target);
    popup_panel.children().add(&nested_popup);
    popup.set_child(popup_panel);
    let window = window_with(&target, &popup);
    window.show();
    Dispatcher::ui_thread().run_jobs(None);

    popup.open();
    Dispatcher::ui_thread().run_jobs(None);
    nested_popup.open();
    Dispatcher::ui_thread().run_jobs(None);

    assert_eq!(1, window.opened_popups().len());
    assert!(popup.ptr_eq(&window.opened_popups()[0]));

    assert_eq!(1, popup.opened_popups().len());
    assert!(nested_popup.ptr_eq(&popup.opened_popups()[0]));
    assert_eq!(0, nested_popup.opened_popups().len());

    // The nested popup is hosted in the parent popup's own top level.
    let nested_target_visual: &Visual = &nested_target;
    assert!(same_top_level(&get_popup_top_level(&popup), &TopLevel::get_top_level(Some(nested_target_visual))));

    nested_popup.close();
    Dispatcher::ui_thread().run_jobs(None);

    assert_eq!(0, popup.opened_popups().len());
    assert_eq!(1, window.opened_popups().len());

    popup.close();
    Dispatcher::ui_thread().run_jobs(None);

    assert_eq!(0, window.opened_popups().len());

    window.close();
}
// The test closes a popup, a top-level of its own, and goes on: the difference of
// `second_window_tests` within one test. It fails in most runs, depending on when the render
// timer ticks.
ferro_fact!(
    #[ignore = "after the nested popup has closed the render loop panics with `a server object id was reused while still alive` (server_compositor.rs), as for a second window (second_window_tests.rs): the test fails with that panic in most runs"]
    nested_popup_is_owned_by_parent_popup
);

pub(crate) fn get_popup_top_level(popup: &Popup) -> Option<Ref<TopLevel>> {
    let child = popup.child();
    assert!(child.is_some());
    let child = child.unwrap();
    let visual: &Visual = &child;
    TopLevel::get_top_level(Some(visual))
}
