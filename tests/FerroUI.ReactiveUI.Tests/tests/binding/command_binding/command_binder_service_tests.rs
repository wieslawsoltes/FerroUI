//! `CommandBinderServiceTests` (ReactiveUI.Binding.Tests, `CommandBinding/CommandBinderServiceTests.cs`).

use ferroui_reactiveui::*;
use ferroui_system::{IDisposable, IObservable};
use std::any::{Any, TypeId};
use std::rc::Rc;

/// A control with an event, which no registered binder knows (`ClickableButton`).
struct ClickableButton;

/// An object with no event and no command property (`PlainObject`).
struct PlainObject;

/// A binder that claims every control with a high affinity and binds nothing (`HighAffinityBinder`).
struct HighAffinityBinder;

/// The affinity of [`HighAffinityBinder`] (`HighAffinity`).
const HIGH_AFFINITY: i32 = 10;

impl ICreatesCommandBinding for HighAffinityBinder {
    fn get_affinity_for_object(&self, _target_type: TypeId, _has_event_target: bool) -> i32 {
        HIGH_AFFINITY
    }

    fn bind_command_to_object(
        &self,
        _command: Option<Rc<dyn ICommand>>,
        _target: Option<Rc<dyn Any>>,
        _command_parameter: Rc<dyn IObservable<CommandParameter>>,
    ) -> Option<Rc<dyn IDisposable>> {
        None
    }

    fn bind_command_to_object_event(
        &self,
        _command: Option<Rc<dyn ICommand>>,
        _target: Option<Rc<dyn Any>>,
        _command_parameter: Rc<dyn IObservable<CommandParameter>>,
        _event_name: &str,
    ) -> Option<Rc<dyn IDisposable>> {
        None
    }

    fn bind_command_to_object_with_handlers(
        &self,
        _command: Option<Rc<dyn ICommand>>,
        _target: Option<Rc<dyn Any>>,
        _command_parameter: Rc<dyn IObservable<CommandParameter>>,
        _subscription: CommandEventSubscription,
    ) -> Option<Rc<dyn IDisposable>> {
        None
    }
}

#[test]
fn get_binder_core_services_only_returns_null() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let _ = builder.with_core_services();
    let _ = builder.build_app();

    let binder = CommandBinderService::get_binder::<ClickableButton>(false);

    assert!(binder.is_none());
}

#[test]
fn get_binder_no_matching_binders_returns_null() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let _ = builder.with_core_services();
    let _ = builder.build_app();

    // PlainObject has no default events and no Command/CommandParameter properties.
    let binder = CommandBinderService::get_binder::<PlainObject>(false);

    assert!(binder.is_none());
}

#[test]
fn get_binder_custom_binder_prefers_higher_affinity() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let _ = builder.with_core_services();

    let custom_binder: Rc<dyn ICreatesCommandBinding> = Rc::new(HighAffinityBinder);
    let _ = builder.with_command_binder(custom_binder.clone());
    let _ = builder.build_app();

    let binder = CommandBinderService::get_binder::<ClickableButton>(false);

    assert!(binder.is_some_and(|binder| Rc::ptr_eq(&binder, &custom_binder)));
}
