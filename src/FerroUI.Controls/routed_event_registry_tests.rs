//! Port of the tests of `Interactivity/RoutedEventRegistryTests.cs` (base unit tests) that are about the click
//! event of a button, so they live with the controls. `Pointer_Events_Should_Be_Registered` is in
//! `interactivity/interactive_tests.rs` of `ferroui-base`.

use crate::{Button, ContentControl};
use ferroui_base::input::InputElement;
use ferroui_base::interactivity::RoutedEventRegistry;

#[test]
fn click_event_should_be_registered_on_button() {
    let expected_events = [Button::click_event().as_routed_event()];
    let registered_events = RoutedEventRegistry::instance().get_registered_for::<Button>();
    assert!(registered_events.iter().any(|e| expected_events.contains(e)));
}

#[test]
fn click_event_should_not_be_registered_on_content_control() {
    // force ContentControl type to be loaded
    let _ = ContentControl::new();
    let expected_events = [Button::click_event().as_routed_event()];
    let registered_events = RoutedEventRegistry::instance().get_registered_for::<ContentControl>();
    assert!(!registered_events.iter().any(|e| expected_events.contains(e)));
}

#[test]
fn input_element_events_should_not_be_registered_on_button() {
    // force Button type to be loaded
    let _ = Button::new();
    let expected_events = [
        InputElement::pointer_pressed_event().as_routed_event(),
        InputElement::pointer_released_event().as_routed_event(),
    ];
    let registered_events = RoutedEventRegistry::instance().get_registered_for::<Button>();
    assert!(!registered_events.iter().any(|e| expected_events.contains(e)));
}
