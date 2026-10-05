use crate::i_clickable_control::as_clickable_control;
use crate::test_command::TestCommand;
use crate::test_support::{test_scope, TestRoot};
use crate::test_support_buttons::focus_scope;
use crate::{Control, SplitButton};
use ferroui_base::input::{InputElement, Key, KeyEventArgs};
use ferroui_base::BoxedValue;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn create_key_event(key: Key, down: bool) -> KeyEventArgs {
    let mut e = KeyEventArgs::new();
    e.key = key;
    if down {
        e.set_routed_event(Some(InputElement::key_down_event()));
    } else {
        e.set_routed_event(Some(InputElement::key_up_event()));
    }
    e
}

#[test]
fn split_button_command_parameter_does_not_change_while_execution() {
    let _scope = test_scope();
    let target = SplitButton::new();
    let initial: BoxedValue = Rc::new("A".to_string());
    let last_parameter: Rc<RefCell<Option<BoxedValue>>> = Rc::new(RefCell::new(Some(initial.clone())));
    let next = Cell::new(0_i32);

    let weak_target = target.downgrade();
    let last_in_can_execute = last_parameter.clone();
    let last_in_execute = last_parameter.clone();
    let command = TestCommand::with_can_execute_and_execute(
        move |parameter| {
            next.set(next.get() + 1);
            let value: BoxedValue = Rc::new(next.get());
            weak_target.upgrade().unwrap().set_command_parameter(Some(value));
            *last_in_can_execute.borrow_mut() = parameter.cloned();
            true
        },
        move |parameter| {
            assert!(*last_in_execute.borrow() == parameter.cloned());
        },
    );
    target.set_command_parameter(Some(initial));
    target.set_command(command.as_command());
    let _root = TestRoot::with_child(&target);

    as_clickable_control(&target).unwrap().raise_click();
}

/// A focusable control stands in for the text box of the reference test.
#[test]
fn should_not_fire_click_event_on_space_key_when_it_is_not_focus() {
    let _scope = test_scope();
    let _focus = focus_scope();
    let target = Control::new();
    target.set_focusable(true);
    let button = SplitButton::new();
    button.set_content(Some(Control::boxed(&target)));

    let window = TestRoot::with_child(&button);
    window.execute_initial_layout_pass();

    let raised = Rc::new(Cell::new(0));
    let counter = raised.clone();
    button.click(move |_, _| counter.set(counter.get() + 1));
    assert!(target.focus());
    target.raise_event(&create_key_event(Key::Space, true));
    target.raise_event(&create_key_event(Key::Space, false));
    assert_eq!(0, raised.get());
}
