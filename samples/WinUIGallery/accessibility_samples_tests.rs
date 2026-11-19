//! The AccessibilityKeyboard page: the source's tab orders, arrow keys, accelerators and access
//! keys, with the handlers of `accessibility_samples.rs`.

use crate::accessibility_samples::ACCELERATOR_COLORS;
use crate::{examples, tests::start};
use ferroui_base::input::{
    InputElement, KeyboardNavigationHandler, NavigationDirection, XYFocus, XYFocusNavigationModes,
};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::{IntoRef, Ref};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{Button, Control, StackPanel, Window};

fn page(id: &str) -> Ref<Control> {
    examples::create(examples::find(id).unwrap()).unwrap_or_else(|error| panic!("{id}: {error}"))
}

fn next(element: &Ref<Button>) -> Option<String> {
    let element: Ref<InputElement> = element.clone().into_ref();
    KeyboardNavigationHandler::get_next(&element, NavigationDirection::Next)
        .and_then(|next| next.name())
}

#[test]
fn accessibility_keyboard_tab_orders_skip_what_the_source_skips() {
    let _application = start();
    let root = page("AccessibilityKeyboard");
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    window.update_layout();
    // The automatic tab order: the XAML order, without the text and the disabled button.
    assert_eq!(next(&root.get_control::<Button>("AutomaticFirst")).as_deref(), Some("AutomaticSecond"));
    assert_eq!(next(&root.get_control::<Button>("AutomaticSecond")).as_deref(), Some("AutomaticThird"));
    // The manual tab order: TabIndex 1, 2 and 3, and the button that is not a tab stop.
    assert_eq!(next(&root.get_control::<Button>("FirstStop")).as_deref(), Some("SecondStop"));
    assert_eq!(next(&root.get_control::<Button>("SecondStop")).as_deref(), Some("ThirdStop"));
    assert!(!root.get_control::<Button>("NotAStop").is_tab_stop());
    window.close();
}

#[test]
fn accessibility_keyboard_arrow_keys_accelerators_and_access_keys_keep_the_source() {
    use ferroui_base::media::IBrush;
    use std::rc::Rc;
    fn color(brush: Option<Rc<dyn IBrush>>) -> Option<ferroui_base::media::Color> {
        brush.and_then(|brush| brush.as_solid_color_brush().map(|brush| brush.color()))
    }
    let _application = start();
    let root = page("AccessibilityKeyboard");
    // XYFocusKeyboardNavigation of the potato buttons.
    let buttons = root.get_control::<StackPanel>("PotatoButtons");
    assert_eq!(XYFocus::get_navigation_modes(&buttons), XYFocusNavigationModes::ENABLED);
    // The accelerators: the buttons' HotKeys and AcceleratorKeys, and the Click handlers color
    // the rectangle.
    for (name, expected) in ACCELERATOR_COLORS {
        let button = root.get_control::<Button>(name);
        assert!(button.hot_key().is_some(), "{name}");
        assert!(AutomationProperties::get_accelerator_key(&button).is_some(), "{name}");
        button.raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
        assert_eq!(color(root.get_control::<Rectangle>("ColorRectangle").fill()), Some(expected), "{name}");
    }
    // The access keys keep the source's keys for assistive technology.
    let menu = root.get_control::<ferroui_controls::Menu>("AccessKeyMenu");
    let keys: Vec<Option<String>> = menu
        .get_logical_descendants()
        .filter_map(|item| item.cast::<ferroui_controls::MenuItem>())
        .map(|item| AutomationProperties::get_access_key(&item))
        .collect();
    assert_eq!(keys.len(), 12);
    assert!(keys.iter().all(Option::is_some));
}
