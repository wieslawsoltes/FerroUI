//! Upstream's label tests only cover the letter spacing property of the
//! templated control, which is not available yet; these cover the port.

use crate::test_support::{test_scope, TestRoot};
use crate::{Control, Label, StackPanel};
use ferroui_base::input::{
    AccessKeyEventArgs, AccessKeyPressedEventArgs, IKeyboardDevice, InputElement, KeyboardDevice,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{FerroLocator, Ref};
use std::rc::Rc;

/// Keeps a keyboard device registered until dropped: focus changes need one.
struct KeyboardScope {
    scope: Rc<dyn IDisposable>,
}

impl KeyboardScope {
    fn new() -> Self {
        let scope = FerroLocator::enter_scope();
        FerroLocator::current_mutable().bind::<dyn IKeyboardDevice>().to_constant(KeyboardDevice::new());
        Self { scope }
    }
}

impl Drop for KeyboardScope {
    fn drop(&mut self) {
        self.scope.dispose();
    }
}

fn label_with_target() -> (Ref<TestRoot>, Ref<Label>, Ref<Control>) {
    let target = Control::new();
    target.set_focusable(true);
    let label = Label::new();
    label.set_target(&target);

    let panel = StackPanel::new();
    panel.children().add(label.clone().upcast::<Control>());
    panel.children().add(target.clone());
    let root = TestRoot::with_child(&panel);

    (root, label, target)
}

#[test]
fn label_is_not_a_tab_stop_by_default() {
    let label = Label::new();

    assert!(!label.is_tab_stop());
    assert!(Control::new().is_tab_stop());
}

#[test]
fn access_key_pressed_is_answered_with_the_target() {
    let _scope = test_scope();
    let (_root, label, target) = label_with_target();

    let args = AccessKeyPressedEventArgs::new("a");
    label.raise_event(&args);

    let expected: Ref<InputElement> = target.upcast();
    assert_eq!(Some(expected), args.target());
    assert!(args.handled());
}

#[test]
fn access_key_pressed_is_not_answered_when_a_target_is_set() {
    let _scope = test_scope();
    let (_root, label, _target) = label_with_target();
    let other: Ref<InputElement> = Control::new().upcast();

    let args = AccessKeyPressedEventArgs::new("a");
    args.set_target(Some(other.clone()));
    label.raise_event(&args);

    assert_eq!(Some(other), args.target());
    assert!(!args.handled());
}

#[test]
fn access_key_focuses_the_target() {
    let _scope = test_scope();
    let _keyboard = KeyboardScope::new();
    let (root, label, target) = label_with_target();
    root.execute_initial_layout_pass();

    let args = AccessKeyEventArgs::new("a", false);
    label.raise_event(&args);

    assert!(target.is_focused());
    assert!(args.handled());
}

#[test]
fn access_key_without_target_is_not_handled() {
    let _scope = test_scope();
    let label = Label::new();
    let _root = TestRoot::with_child(&label);

    let args = AccessKeyEventArgs::new("a", false);
    label.raise_event(&args);

    assert!(!args.handled());
}
