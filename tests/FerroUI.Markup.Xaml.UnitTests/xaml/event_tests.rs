//! Port of `Xaml/EventTests.cs`.

use ferroui_base::input::{
    InputElement, KeyModifiers, Pointer, PointerEventArgs, PointerPointProperties, PointerType, TappedEventArgs,
};
use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs};
use ferroui_base::Point;
use ferroui_controls::Button;

use crate::support::app::xaml_test_base;
use crate::support::helpers::{assert_throws_xaml_diagnostic, boxed};
use crate::support::loader::{load_with_root, try_load_with_root};
use crate::support::xaml::event_tests::{MyButton, MyPanel};

#[test]
fn event_is_assigned() {
    let _base = xaml_test_base();
    let xaml = "<Button xmlns='https://github.com/ferroui' Click='OnClick'/>";
    let target = MyButton::new();

    load_with_root(xaml, None, boxed(target.clone()));

    target.raise_event(&RoutedEventArgs::with_event(Button::click_event()));

    assert!(target.was_clicked());
}

#[test]
fn attached_event_is_assigned() {
    let _base = xaml_test_base();
    let xaml = "<Button xmlns='https://github.com/ferroui' InputElement.Tapped='OnTapped'/>";
    let target = MyButton::new();

    load_with_root(xaml, None, boxed(target.clone()));

    target.raise_event(&RoutedEventArgs::with_event(InputElement::tapped_event()));

    assert!(target.was_tapped());
}

#[test]
fn attached_event_is_assigned_generic() {
    let _base = xaml_test_base();
    let xaml = "<Panel xmlns='https://github.com/ferroui'><Grid DoubleTapped='OnTapped'><Button Name='target'/></Grid></Panel>";
    let host = MyPanel::new();

    load_with_root(xaml, None, boxed(host.clone()));

    let target = host.find_control::<Button>("target");

    let target = target.expect("the control 'target' is found");

    // The managed original passes no pointer event (`null!`); the arguments
    // of a tap cannot be created without one here.
    let pointer_event = PointerEventArgs::new(
        None::<&RoutedEvent<PointerEventArgs>>,
        &target,
        Pointer::new(0, PointerType::Mouse, true),
        None,
        Point::default(),
        0,
        PointerPointProperties::default(),
        KeyModifiers::NONE,
    );
    target.raise_event(&TappedEventArgs::new(Some(InputElement::double_tapped_event()), &pointer_event));

    assert!(host.was_tapped());
}

#[test]
fn exception_is_thrown_if_event_not_found() {
    let _base = xaml_test_base();
    let xaml = "<Button xmlns='https://github.com/ferroui' Click='NotFound'/>";
    let target = MyButton::new();

    assert_throws_xaml_diagnostic(
        try_load_with_root(xaml, None, boxed(target.clone())),
        "FRN3000",
        "Unable to find suitable setter or adder for property Click of type FerroUI.Markup.Xaml.UnitTests:FerroUI.Markup.Xaml.UnitTests.Xaml.MyButton for argument System.Runtime:System.String, available setter parameter lists are:\nSystem.EventHandler`1[FerroUI.Interactivity.RoutedEventArgs] Line 1, position 44.",
    );
}

#[test]
fn attached_event_routed_event_handler() {
    let _base = xaml_test_base();
    let xaml = "<Panel xmlns='https://github.com/ferroui' Button.Click='OnClick'><Button Name='target'/></Panel>";
    let host = MyPanel::new();

    load_with_root(xaml, None, boxed(host.clone()));

    let target = host.get_control::<Button>("target");
    target.raise_event(&RoutedEventArgs::with_event(Button::click_event()));

    assert!(host.was_clicked());
}
