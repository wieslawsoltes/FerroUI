//! Port of `Next_Skip_Button_When_Command_CanExecute_Is_False` of `Input/KeyboardNavigationTests_Tab.cs` (base
//! unit tests): the tree has buttons, one with a command, so the test lives with the controls. The other tests of
//! the file are in `input/input_tests.rs` of `ferroui-base`.

use crate::test_command::TestCommand;
use crate::test_support::TestRoot;
use crate::testing::{TestServices, UnitTestApplication};
use crate::{Button, StackPanel};
use ferroui_base::input::{
    InputElement, KeyboardNavigation, KeyboardNavigationHandler, KeyboardNavigationMode, NavigationDirection,
};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{IntoRef, Ref};
use std::cell::Cell;
use std::rc::Rc;

fn named_button(name: &str) -> Ref<Button> {
    let button = Button::new();
    button.set_name(Some(name.to_string()));
    button
}

#[test]
fn next_skip_button_when_command_can_execute_is_false() {
    let executed = Rc::new(Cell::new(false));

    let _app = UnitTestApplication::start(TestServices::styled_window());

    let current = named_button("Button1");
    let disabled = named_button("Button2");
    let flag = executed.clone();
    disabled.set_command(TestCommand::with_can_execute_and_execute(|_| false, move |_| flag.set(true)).as_command());
    let expected = named_button("Button3");

    let inner = StackPanel::new();
    inner.children().add(&current);
    inner.children().add(&disabled);
    inner.children().add(&expected);

    let top = StackPanel::new();
    KeyboardNavigation::set_tab_navigation(&top, KeyboardNavigationMode::Cycle);
    top.children().add(&inner);

    let _test_root = TestRoot::with_child(&top);

    top.apply_template();

    Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));

    let current: Ref<InputElement> = current.into_ref();
    let result = KeyboardNavigationHandler::get_next(&current, NavigationDirection::Next)
        .and_then(|element| element.cast::<Button>());

    assert_eq!(expected.name(), result.and_then(|button| button.name()));
    assert!(!executed.get());
}
