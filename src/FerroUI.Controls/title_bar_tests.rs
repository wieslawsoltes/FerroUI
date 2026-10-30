//! Tests of [`TitleBar`], after WinUI's `TitleBarTests` (`controls/dev/TitleBar/APITests` of
//! microsoft-ui-xaml at `7e4b0c2`): the defaults, the events of the buttons, the states and the
//! window.

use crate::testing::{TestServices, UnitTestApplication};
use crate::{Button, Control, TitleBar};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn the_defaults_are_those_of_winui() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let bar = TitleBar::new();
    assert_eq!("", bar.title());
    assert_eq!("", bar.subtitle());
    assert!(bar.icon_source().is_none());
    assert!(bar.left_header().is_none() && bar.content().is_none() && bar.right_header().is_none());
    assert!(!bar.is_back_button_visible());
    assert!(bar.is_back_button_enabled());
    assert!(!bar.is_pane_toggle_button_visible());
    assert!(!bar.auto_refresh_drag_regions());
    assert!(bar.template_settings().icon_element().is_none());
}

#[test]
fn the_states_follow_the_properties() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let bar = TitleBar::new();
    let classes = bar.pseudo_classes();
    assert!(!classes.contains(":back-button-visible") && !classes.contains(":expanded-height"));
    bar.set_is_back_button_visible(true);
    assert!(classes.contains(":back-button-visible"));
    assert!(classes.contains(":negative-inset-spacing"), "back without toggle");
    bar.set_is_pane_toggle_button_visible(true);
    assert!(classes.contains(":pane-toggle-button-visible"));
    assert!(!classes.contains(":negative-inset-spacing"), "back with toggle");
    bar.set_title("App");
    assert!(classes.contains(":title-text-visible"));
    bar.set_content(Some(Button::new().upcast::<Control>()));
    assert!(classes.contains(":content-visible") && classes.contains(":expanded-height"));
    bar.set_content(None);
    assert!(!classes.contains(":content-visible") && !classes.contains(":expanded-height"));
    assert!(!classes.contains(":deactivated"), "a bar outside a window is not deactivated");
}

#[test]
fn the_buttons_raise_their_requests() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let bar = TitleBar::new();
    let back = Rc::new(Cell::new(0));
    let toggle = Rc::new(Cell::new(0));
    let counter = back.clone();
    let _a = bar.back_requested(move |_| counter.set(counter.get() + 1));
    let counter = toggle.clone();
    let _b = bar.pane_toggle_requested(move |_| counter.set(counter.get() + 1));
    bar.raise_back_requested_for_tests();
    bar.raise_pane_toggle_requested_for_tests();
    assert_eq!((1, 1), (back.get(), toggle.get()));
}

#[test]
fn the_icon_source_gives_the_icon_element() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let bar = TitleBar::new();
    let source = crate::SymbolIconSource::new();
    bar.set_icon_source(Some(source.upcast()));
    assert!(bar.template_settings().icon_element().is_some());
    assert!(bar.pseudo_classes().contains(":icon-visible"));
    bar.set_icon_source(None);
    assert!(bar.template_settings().icon_element().is_none());
}

#[test]
fn the_title_is_the_title_of_the_window_while_the_bar_shows_it() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = crate::Window::new();
    window.set_title(Some("Window".to_string()));
    let bar = TitleBar::new();
    bar.set_title("App");
    window.set_content(Some(Control::boxed(&bar)));
    window.show();
    assert_eq!(Some("App".to_string()), window.title());
    window.set_content(None);
    assert_eq!(Some("Window".to_string()), window.title(), "the window gets its title back");
    window.close();
}

#[test]
fn the_bar_drags_the_window_and_its_buttons_and_content_take_the_input() {
    use crate::chrome::WindowDecorationProperties;
    use ferroui_base::input::WindowDecorationsElementRole as Role;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let bar = TitleBar::new();
    let search = crate::TextBox::new();
    let panel = crate::StackPanel::new();
    let label = crate::TextBlock::new();
    panel.children().add(label.clone());
    panel.children().add(search.clone());
    bar.set_content(Some(panel.clone().upcast()));
    let window = crate::Window::new();
    window.set_content(Some(Control::boxed(&bar)));
    window.show();
    assert_eq!(Role::TitleBar, WindowDecorationProperties::get_element_role(&bar));
    assert_eq!(Role::User, WindowDecorationProperties::get_element_role(&search), "a control of the content");
    assert_eq!(Role::None, WindowDecorationProperties::get_element_role(&label), "text drags the window");
    // IsDragRegion true on the text box: it drags the window too (the test theme has no template
    // for the bar, so the content is not below it and the bar is asked to look again).
    TitleBar::set_is_drag_region(&search, Some(true));
    bar.recompute_drag_regions();
    assert_eq!(Role::None, WindowDecorationProperties::get_element_role(&search));
    window.close();
}
