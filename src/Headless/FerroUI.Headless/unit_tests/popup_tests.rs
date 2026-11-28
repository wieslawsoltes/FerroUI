//! Port of `PopupTests.cs` of the upstream unit test project of the
//! headless platform.

use super::test_application::{ferro_fact, HeadlessRuntimeOptions, TestApplication};
use crate::HeadlessWindowExtensions;
use ferroui_base::input::{MouseButton, RawInputModifiers};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{PixelPoint, Point, Ref, Thickness, Visual};
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

/// A border of 20 by 20 at the top left of its parent, moved by a margin.
fn placed_border(left: f64, top: f64, background: std::rc::Rc<dyn IBrush>) -> Ref<Border> {
    let border = sized_border(20.0, 20.0);
    border.set_horizontal_alignment(HorizontalAlignment::Left);
    border.set_vertical_alignment(VerticalAlignment::Top);
    border.set_margin(Thickness::new(left, top, 0.0, 0.0));
    border.set_background(Some(background));
    border
}

/// A popup of 20 by 20 below `target`.
fn popup_below(target: &Ref<Border>) -> Ref<Popup> {
    let popup = Popup::new();
    popup.set_placement_target(target.clone());
    popup.set_placement(PlacementMode::Bottom);
    popup.set_child(sized_border(20.0, 20.0));
    popup
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

fn popup_opening_sequence_tracks_hosted_open_close_reopen() {
    let target = sized_border(20.0, 20.0);
    let popup = popup_below(&target);
    assert_eq!(popup.opening_sequence(), None);
    // A request before attachment has no displayed host or opening sequence.
    popup.set_is_open(true);
    assert_eq!(popup.opening_sequence(), None);
    popup.set_is_open(false);
    let window = window_with(&target, &popup);
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    popup.open();
    let first = popup.opening_sequence().expect("hosted opening");
    popup.open();
    assert_eq!(popup.opening_sequence(), Some(first));
    popup.close();
    assert_eq!(popup.opening_sequence(), None);
    popup.open();
    assert!(popup.opening_sequence().unwrap() > first);
    popup.close();
    window.close();
}
ferro_fact!(popup_opening_sequence_tracks_hosted_open_close_reopen);

fn popup_opening_sequence_precedes_reentrant_opened_callback() {
    let target = sized_border(20.0, 20.0);
    let first = popup_below(&target);
    let second = popup_below(&target);
    let window = window_with(&target, &first);
    let panel = Control::from_boxed(&window.content().unwrap()).unwrap().cast::<Panel>().unwrap();
    panel.children().add(second.clone());
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    let second_weak = second.downgrade();
    let first_weak = first.downgrade();
    let subscription = first.opened(move || {
        let first = first_weak.upgrade().unwrap();
        let second = second_weak.upgrade().unwrap();
        let first_order = first.opening_sequence().expect("set before callback");
        second.open();
        assert!(second.opening_sequence().unwrap() > first_order);
    });
    first.open();
    subscription.dispose();
    second.close();
    first.close();
    window.close();
}
ferro_fact!(popup_opening_sequence_precedes_reentrant_opened_callback);

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
ferro_fact!(can_click_button_inside_platform_popup);

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
ferro_fact!(nested_popup_is_owned_by_parent_popup);

fn nested_popup_placement_respects_parent_popup_position() {
    let nested_target = placed_border(10.0, 5.0, Brushes::green());
    let nested_popup = popup_below(&nested_target);
    let target = placed_border(30.0, 40.0, Brushes::red());
    let popup = Popup::new();
    popup.set_placement_target(target.clone());
    popup.set_placement(PlacementMode::Bottom);
    let popup_panel = Panel::new();
    popup_panel.set_width(20.0);
    popup_panel.set_height(50.0);
    popup_panel.children().add(&nested_target);
    popup_panel.children().add(&nested_popup);
    popup.set_child(popup_panel);
    let window = window_with(&target, &popup);
    window.set_position(PixelPoint::new(100, 200));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);

    popup.open();
    Dispatcher::ui_thread().run_jobs(None);
    nested_popup.open();
    Dispatcher::ui_thread().run_jobs(None);

    let popup_root = get_popup_top_level(&popup).expect("the popup has a top level");
    let nested_root = get_popup_top_level(&nested_popup).expect("the nested popup has a top level");
    assert_eq!(target.point_to_screen(Point::new(0.0, 20.0)), popup_root.point_to_screen(Point::default()));
    assert_eq!(nested_target.point_to_screen(Point::new(0.0, 20.0)), nested_root.point_to_screen(Point::default()));

    window.close();
}
ferro_fact!(nested_popup_placement_respects_parent_popup_position);

fn popup_placement_respects_render_scaling() {
    let target = placed_border(30.0, 40.0, Brushes::red());
    let popup = popup_below(&target);
    let window = window_with(&target, &popup);
    window.set_position(PixelPoint::new(100, 200));
    window.show();
    window.set_render_scaling(2.0);
    Dispatcher::ui_thread().run_jobs(None);

    popup.open();
    Dispatcher::ui_thread().run_jobs(None);

    let popup_root = get_popup_top_level(&popup).expect("the popup has a top level");
    assert_eq!(target.point_to_screen(Point::new(0.0, 20.0)), popup_root.point_to_screen(Point::default()));
    assert_eq!(2.0, popup_root.render_scaling());

    window.close();
}
ferro_fact!(popup_placement_respects_render_scaling);

fn popup_follows_window_move() {
    let target = placed_border(30.0, 40.0, Brushes::red());
    let popup = popup_below(&target);
    let window = window_with(&target, &popup);
    window.set_position(PixelPoint::new(100, 200));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);

    popup.open();
    Dispatcher::ui_thread().run_jobs(None);

    window.set_position(PixelPoint::new(300, 400));
    Dispatcher::ui_thread().run_jobs(None);

    let popup_root = get_popup_top_level(&popup).expect("the popup has a top level");
    assert_eq!(PixelPoint::new(330, 460), popup_root.point_to_screen(Point::default()));

    window.close();
}
ferro_fact!(popup_follows_window_move);

fn window_position_is_fixed_when_positioning_is_not_supported() {
    let _scope = TestApplication::reconfigure(HeadlessRuntimeOptions {
        supports_window_positioning: Some(false),
        ..HeadlessRuntimeOptions::default()
    });

    let target = placed_border(30.0, 40.0, Brushes::red());
    let popup = popup_below(&target);
    let window = window_with(&target, &popup);
    window.set_position(PixelPoint::new(100, 200));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);

    assert_eq!(PixelPoint::ORIGIN, window.position());
    assert_eq!(PixelPoint::new(10, 20), window.point_to_screen(Point::new(10.0, 20.0)));

    popup.open();
    Dispatcher::ui_thread().run_jobs(None);

    let popup_root = get_popup_top_level(&popup).expect("the popup has a top level");
    assert_eq!(PixelPoint::new(30, 60), popup_root.point_to_screen(Point::default()));

    window.close();
}
ferro_fact!(window_position_is_fixed_when_positioning_is_not_supported);

fn desktop_coordinates_are_logical_when_enabled() {
    let _scope = TestApplication::reconfigure(HeadlessRuntimeOptions {
        use_logical_desktop_coordinates: Some(true),
        ..HeadlessRuntimeOptions::default()
    });

    let target = placed_border(30.0, 40.0, Brushes::red());
    let popup = popup_below(&target);
    let window = window_with(&target, &popup);
    window.set_position(PixelPoint::new(100, 200));
    window.show();
    window.set_render_scaling(2.0);
    Dispatcher::ui_thread().run_jobs(None);

    assert_eq!(2.0, window.render_scaling());
    assert_eq!(1.0, window.desktop_scaling());
    assert_eq!(PixelPoint::new(110, 220), window.point_to_screen(Point::new(10.0, 20.0)));
    assert_eq!(Point::new(10.0, 20.0), window.point_to_client(PixelPoint::new(110, 220)));

    popup.open();
    Dispatcher::ui_thread().run_jobs(None);

    let popup_root = get_popup_top_level(&popup).expect("the popup has a top level");
    assert_eq!(2.0, popup_root.render_scaling());
    assert_eq!(PixelPoint::new(130, 260), popup_root.point_to_screen(Point::default()));

    window.close();
}
ferro_fact!(desktop_coordinates_are_logical_when_enabled);

pub(crate) fn get_popup_top_level(popup: &Popup) -> Option<Ref<TopLevel>> {
    let child = popup.child();
    assert!(child.is_some());
    let child = child.unwrap();
    let visual: &Visual = &child;
    TopLevel::get_top_level(Some(visual))
}

fn ferro_precise_pointer_popup_preserves_fractional_physical_and_logical_desktop_coordinates() {
    use ferroui_base::input::{
        InputElement, KeyModifiers, Pointer, PointerEventArgs, PointerPointProperties, PointerType,
    };
    for logical in [false, true] {
        let _scope = TestApplication::reconfigure(HeadlessRuntimeOptions {
            use_logical_desktop_coordinates: Some(logical),
            ..HeadlessRuntimeOptions::default()
        });
        for scaling in [1.0, 1.5, 2.0] {
            let target = placed_border(30.0, 40.0, Brushes::red());
            let popup = popup_below(&target);
            let window = window_with(&target, &popup);
            window.set_position(PixelPoint::new(100, 200));
            window.show();
            window.set_render_scaling(scaling);
            Dispatcher::ui_thread().run_jobs(None);
            popup.open();
            Dispatcher::ui_thread().run_jobs(None);
            let popup_root = get_popup_top_level(&popup).expect("dedicated popup top level");
            let position = Point::new(10.25, 15.75);
            let event = PointerEventArgs::new(
                Some(InputElement::pointer_moved_event()),
                popup_root.clone(),
                Pointer::new(41, PointerType::Mouse, true),
                Some(&popup_root),
                position,
                0,
                PointerPointProperties::NONE,
                KeyModifiers::NONE,
            );
            let popup_origin = popup_root.point_to_screen(Point::default());
            let window_origin = window.point_to_screen(Point::default());
            let desktop_scaling = if logical { 1.0 } else { scaling };
            let expected = Point::new(
                position.x + f64::from(popup_origin.x - window_origin.x) / desktop_scaling,
                position.y + f64::from(popup_origin.y - window_origin.y) / desktop_scaling,
            );
            let actual = event
                .try_get_position_precise(Some(&window))
                .expect("backend continuous transform");
            assert!(
                (actual.x - expected.x).abs() < 1e-10 && (actual.y - expected.y).abs() < 1e-10,
                "logical={logical}, scaling={scaling}, {actual:?} != {expected:?}"
            );
            assert_eq!(
                event.try_get_position_precise(Some(&popup_root)),
                Some(position)
            );
            window.close();
        }
    }
}
ferro_fact!(
    ferro_precise_pointer_popup_preserves_fractional_physical_and_logical_desktop_coordinates
);
