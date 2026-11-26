//! Tests of `CommandBarFlyout` (the behaviour of `CommandBarFlyout.cpp` and of the WinUI Gallery
//! page CommandBarFlyout).

use super::{CommandBarFlyout, CommandBarFlyoutCommandBar};
use crate::flyouts::{FlyoutPresenter, FlyoutShowMode};
use crate::testing::{TestServices, UnitTestApplication};
use crate::{Button, CommandBarButton, CommandBarOverflowButtonVisibility, CommandBarToggleButton, Control, Window};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::threading::Dispatcher;
use ferroui_base::Ref;

fn button(label: &str) -> Ref<CommandBarButton> {
    let button = CommandBarButton::new();
    button.set_label(Some(label));
    button
}

fn flyout() -> Ref<CommandBarFlyout> {
    let flyout = CommandBarFlyout::new();
    for label in ["Share", "Save", "Delete"] {
        flyout.primary_commands().add(button(label).as_command_bar_element());
    }
    for label in ["Resize", "Move"] {
        flyout.secondary_commands().add(button(label).as_command_bar_element());
    }
    flyout
}

fn window() -> Ref<Window> {
    let window = Window::new();
    window.apply_template();
    window.show();
    window
}

fn bar_of(flyout: &CommandBarFlyout) -> Ref<CommandBarFlyoutCommandBar> {
    flyout.command_bar().expect("the command bar of the open flyout")
}

#[test]
fn the_presenter_is_a_bare_flyout_presenter_around_a_bar_of_the_commands() {
    let _app = UnitTestApplication::start(TestServices::focusable_window());
    let window = window();
    let flyout = flyout();
    flyout.show_at(&window);
    let presenter = flyout.popup().child().and_then(|child| child.cast::<FlyoutPresenter>()).expect("a presenter");
    assert_eq!(0.0, presenter.min_width());
    assert!(presenter.background().is_none());
    let bar = bar_of(&flyout);
    assert!(presenter
        .content()
        .as_ref()
        .and_then(Control::from_boxed)
        .is_some_and(|content| content == bar.clone().upcast::<Control>()));
    assert_eq!(3, bar.primary_commands().count());
    assert_eq!(2, bar.secondary_commands().count());
    assert!(bar.owning_flyout().is_some_and(|owner| owner == flyout));
    // A change of the flyout's lists reaches the bar.
    flyout.secondary_commands().add(button("Rotate").as_command_bar_element());
    assert_eq!(3, bar.secondary_commands().count());
    flyout.hide();
}

#[test]
fn a_standard_flyout_opens_its_bar_and_a_transient_one_does_not() {
    let _app = UnitTestApplication::start(TestServices::focusable_window());
    let window = window();
    let flyout = flyout();
    flyout.set_show_mode(FlyoutShowMode::Transient);
    flyout.show_at(&window);
    Dispatcher::ui_thread().run_jobs(None);
    let bar = bar_of(&flyout);
    assert!(!bar.is_open());
    assert_eq!(CommandBarOverflowButtonVisibility::Auto, bar.overflow_button_visibility());
    flyout.hide();
    assert!(!bar.is_open());

    flyout.set_show_mode(FlyoutShowMode::Standard);
    flyout.show_at(&window);
    Dispatcher::ui_thread().run_jobs(None);
    assert!(bar.is_open());
    flyout.hide();
    assert!(!bar.is_open());
}

#[test]
fn always_expanded_hides_the_more_button_and_shows_the_secondary_commands() {
    let _app = UnitTestApplication::start(TestServices::focusable_window());
    let window = window();
    let flyout = flyout();
    flyout.set_always_expanded(true);
    flyout.set_show_mode(FlyoutShowMode::Transient);
    flyout.show_at(&window);
    Dispatcher::ui_thread().run_jobs(None);
    let bar = bar_of(&flyout);
    assert_eq!(CommandBarOverflowButtonVisibility::Collapsed, bar.overflow_button_visibility());
    assert_eq!(FlyoutShowMode::Standard, flyout.show_mode());
    assert!(bar.is_open());
    flyout.hide();
}

#[test]
fn a_click_of_a_secondary_command_closes_the_flyout() {
    let _app = UnitTestApplication::start(TestServices::focusable_window());
    let window = window();
    let flyout = flyout();
    let toggle = CommandBarToggleButton::new();
    flyout.secondary_commands().add(toggle.as_command_bar_element());
    flyout.show_at(&window);
    let click = |button: &Button| button.raise_event(&RoutedEventArgs::with_event(&Button::click_event()));

    // A primary command leaves the flyout open.
    let primary = flyout.primary_commands().get(0).as_object().and_then(|o| o.to_ref().cast::<Button>()).unwrap();
    click(&primary);
    assert!(flyout.is_open());
    let secondary = flyout.secondary_commands().get(0).as_object().and_then(|o| o.to_ref().cast::<Button>()).unwrap();
    click(&secondary);
    assert!(!flyout.is_open());

    flyout.show_at(&window);
    click(&toggle);
    assert!(!flyout.is_open());
}
