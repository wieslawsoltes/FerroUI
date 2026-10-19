//! `BindingHooksTests` (`ReactiveUI.Binding.Tests/Bindings/BindingHooksTests.cs`).
//!
//! The registrations are per thread, and each test runs on a thread of its own; the tests still
//! clear what they register, as upstream's do.

use ferroui_reactiveui::*;
use ferroui_splat::AppLocator;
use std::any::Any;
use std::rc::Rc;

/// A hook that answers the same way every time (`StubHook`).
struct StubHook(bool);

impl IPropertyBindingHook for StubHook {
    fn execute_hook(
        &self,
        _source: Option<&Rc<dyn Any>>,
        _target: &Rc<dyn Any>,
        _get_current_view_model_properties: &dyn Fn() -> BindingHookChanges,
        _get_current_view_properties: &dyn Fn() -> BindingHookChanges,
        _direction: BindingDirection,
    ) -> bool {
        self.0
    }
}

/// An empty set of observed changes, which is what a hook is handed for a bound object (`NoChanges`).
fn no_changes() -> BindingHookChanges {
    Vec::new()
}

/// A bound object (`new object()`).
fn object() -> Rc<dyn Any> {
    Rc::new(())
}

/// Clears every registration and the cached set, so that a test starts from nothing registered
/// (`ResetHooks`).
fn reset_hooks() {
    AppLocator::unregister_all::<dyn IPropertyBindingHook>();
    BindingHooks::refresh();
}

fn register_hook(allow: bool) {
    AppLocator::register::<dyn IPropertyBindingHook>(move || Some(Rc::new(StubHook(allow)) as Rc<dyn IPropertyBindingHook>));
}

fn should_bind(direction: BindingDirection) -> bool {
    BindingHooks::should_bind(Some(&object()), &object(), &no_changes, &no_changes, direction)
}

#[test]
fn any_with_no_hook_registered_is_false() {
    reset_hooks();

    assert!(!BindingHooks::any());
}

#[test]
fn should_bind_with_no_hook_registered_allows_the_binding() {
    reset_hooks();

    assert!(should_bind(BindingDirection::OneWay));
}

#[test]
fn any_after_registering_a_hook_is_true() {
    reset_hooks();
    register_hook(true);
    BindingHooks::refresh();

    assert!(BindingHooks::any());
    reset_hooks();
}

#[test]
fn should_bind_when_the_hook_approves_allows_the_binding() {
    reset_hooks();
    register_hook(true);
    BindingHooks::refresh();

    assert!(should_bind(BindingDirection::TwoWay));
    reset_hooks();
}

#[test]
fn should_bind_when_the_hook_refuses_cancels_the_binding() {
    reset_hooks();
    register_hook(false);
    BindingHooks::refresh();

    assert!(!should_bind(BindingDirection::OneWay));
    reset_hooks();
}

#[test]
fn should_bind_when_one_of_several_hooks_refuses_cancels_the_binding() {
    reset_hooks();
    register_hook(true);
    register_hook(false);
    BindingHooks::refresh();

    assert!(!should_bind(BindingDirection::OneWay));
    reset_hooks();
}

// Not ported: ShouldBind_NullTarget_Throws - argument validation

// Not ported: ShouldBind_NullSourceReader_Throws - argument validation

// Not ported: ShouldBind_NullTargetReader_Throws - argument validation
