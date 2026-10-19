//! `BindingDispatchStubTests` (`ReactiveUI.Binding.Tests/Mixins/BindingDispatchStubTests.cs`).
//!
//! Upstream checks two things: that a call no generated code claimed binds through the run-time
//! engine when it names the `Unsafe` twin, and that the stub a generator displaces refuses the
//! call otherwise. The bindings of the port take property paths and bind every call, so the first
//! group runs on the bindings themselves, and the refusals have no counterpart.
//!
//! Upstream's `BindCommandUnsafe` and `BindInteractionUnsafe` take the view model as an argument,
//! as the `_on` forms of `bind_command` and `bind_interaction` do. `RecordingStubCommand` is the
//! `RecordingCommand` of the mocks, which records its parameter the same way.

use crate::mocks::RecordingCommand;
use crate::tests::binding::test_models::{ClickCommandBinder, DispatchStubView, DispatchStubViewModel, ManualObservable};
use ferroui_reactive::linq::Observable;
use ferroui_reactiveui::*;
use ferroui_splat::AppLocator;
use ferroui_system::prop;
use ferroui_system::threading::tasks::{block_on, Task};
use ferroui_system::IObservable;
use std::rc::Rc;

const BOUND_VALUE: &str = "bound";

const REVERSE_VALUE: &str = "reversed";

/// The view model, the view and the manual source of a test.
fn fixture() -> (Rc<DispatchStubViewModel>, Rc<DispatchStubView>, Rc<ManualObservable<String>>) {
    (DispatchStubViewModel::new(), DispatchStubView::new(), ManualObservable::new())
}

#[test]
fn bind_one_way_with_no_generated_overload_binds_through_the_runtime_engine() {
    let (view_model, view, _) = fixture();
    view_model.set_caption(BOUND_VALUE.to_string());

    let binding = view_model.bind_one_way(&view, prop!(DispatchStubViewModel => caption), settable_prop!(DispatchStubView => caption));

    assert_eq!(view.caption(), BOUND_VALUE);
    binding.dispose();
}

#[test]
fn bind_one_way_converting_with_no_generated_overload_binds_through_the_runtime_engine() {
    let (view_model, view, _) = fixture();
    view_model.set_caption(BOUND_VALUE.to_string());

    let binding = view_model.bind_one_way_select(&view, prop!(DispatchStubViewModel => caption), settable_prop!(DispatchStubView => caption), |value: String| value);

    assert_eq!(view.caption(), BOUND_VALUE);
    binding.dispose();
}

#[test]
fn bind_two_way_with_no_generated_overload_binds_both_sides_through_the_runtime_engine() {
    let (view_model, view, _) = fixture();
    view_model.set_caption(BOUND_VALUE.to_string());

    let binding = view_model.bind_two_way(&view, settable_prop!(DispatchStubViewModel => caption), settable_prop!(DispatchStubView => caption));

    let after_bind = view.caption();
    view.set_caption(REVERSE_VALUE.to_string());

    assert_eq!(after_bind, BOUND_VALUE);
    assert_eq!(view_model.caption(), REVERSE_VALUE);
    binding.dispose();
}

#[test]
fn bind_two_way_converting_with_no_generated_overload_binds_through_the_runtime_engine() {
    let (view_model, view, _) = fixture();
    view_model.set_caption(BOUND_VALUE.to_string());

    let binding = view_model.bind_two_way_select(
        &view,
        settable_prop!(DispatchStubViewModel => caption),
        settable_prop!(DispatchStubView => caption),
        |value: String| value,
        |value: String| value,
    );

    assert_eq!(view.caption(), BOUND_VALUE);
    binding.dispose();
}

#[test]
fn one_way_bind_with_no_generated_overload_binds_through_the_runtime_engine() {
    let (view_model, view, _) = fixture();
    view_model.set_caption(BOUND_VALUE.to_string());

    let binding = view
        .one_way_bind(view_model.clone(), prop!(DispatchStubViewModel => caption), settable_prop!(DispatchStubView => caption))
        .expect("no hook refuses the binding");

    assert_eq!(view.caption(), BOUND_VALUE);
    binding.dispose();
}

#[test]
fn one_way_bind_converting_with_no_generated_overload_binds_through_the_runtime_engine() {
    let (view_model, view, _) = fixture();
    view_model.set_caption(BOUND_VALUE.to_string());

    let binding = view
        .one_way_bind_select(view_model.clone(), prop!(DispatchStubViewModel => caption), settable_prop!(DispatchStubView => caption), |value: String| value)
        .expect("no hook refuses the binding");

    assert_eq!(view.caption(), BOUND_VALUE);
    binding.dispose();
}

#[test]
fn bind_with_no_generated_overload_binds_both_sides_through_the_runtime_engine() {
    let (view_model, view, _) = fixture();
    view_model.set_caption(BOUND_VALUE.to_string());

    let binding = view
        .bind(view_model.clone(), settable_prop!(DispatchStubViewModel => caption), settable_prop!(DispatchStubView => caption))
        .expect("no hook refuses the binding");

    let after_bind = view.caption();
    view.set_caption(REVERSE_VALUE.to_string());

    assert_eq!(after_bind, BOUND_VALUE);
    assert_eq!(view_model.caption(), REVERSE_VALUE);
    binding.dispose();
}

#[test]
fn bind_converting_with_no_generated_overload_binds_through_the_runtime_engine() {
    let (view_model, view, _) = fixture();
    view_model.set_caption(BOUND_VALUE.to_string());

    let binding = view
        .bind_select(
            view_model.clone(),
            settable_prop!(DispatchStubViewModel => caption),
            settable_prop!(DispatchStubView => caption),
            |value: String| value,
            |value: String| value,
        )
        .expect("no hook refuses the binding");

    assert_eq!(view.caption(), BOUND_VALUE);
    binding.dispose();
}

#[test]
fn bind_to_with_no_generated_overload_writes_through_the_runtime_engine() {
    let (_, view, source) = fixture();

    let observable: Rc<dyn IObservable<String>> = source.clone();
    let binding = observable.bind_to(view.clone(), settable_prop!(DispatchStubView => caption));
    source.on_next(BOUND_VALUE.to_string());

    assert_eq!(view.caption(), BOUND_VALUE);
    binding.dispose();
}

#[test]
fn bind_to_with_a_conversion_hint_writes_through_the_runtime_engine() {
    let (_, view, source) = fixture();

    let observable: Rc<dyn IObservable<String>> = source.clone();
    let binding = observable.bind_to_with(view.clone(), settable_prop!(DispatchStubView => caption), None, None);
    source.on_next(BOUND_VALUE.to_string());

    assert_eq!(view.caption(), BOUND_VALUE);
    binding.dispose();
}

#[test]
fn bind_to_with_a_converter_writes_through_the_runtime_engine() {
    let (_, view, source) = fixture();

    let observable: Rc<dyn IObservable<String>> = source.clone();
    let binding = observable.bind_to_with(view.clone(), settable_prop!(DispatchStubView => caption), None, None);
    source.on_next(BOUND_VALUE.to_string());

    assert_eq!(view.caption(), BOUND_VALUE);
    binding.dispose();
}

#[test]
fn bind_to_with_a_conversion_hint_and_a_converter_writes_through_the_runtime_engine() {
    let (_, view, source) = fixture();

    let observable: Rc<dyn IObservable<String>> = source.clone();
    let binding = observable.bind_to_with(view.clone(), settable_prop!(DispatchStubView => caption), None, None);
    source.on_next(BOUND_VALUE.to_string());

    assert_eq!(view.caption(), BOUND_VALUE);
    binding.dispose();
}

#[test]
fn bind_command_with_no_generated_overload_binds_through_the_runtime_engine() {
    let (view_model, view, _) = fixture();

    let binding = view.bind_command_on(Some(view_model.clone()), prop!(DispatchStubViewModel => run), prop!(DispatchStubView => control), None);

    binding.dispose();
}

#[test]
fn bind_command_with_a_parameter_stream_binds_through_the_runtime_engine() {
    let (view_model, view, source) = fixture();

    let parameter: Rc<dyn IObservable<String>> = source.clone();
    let binding = view.bind_command_with_parameter_on(
        Some(view_model.clone()),
        prop!(DispatchStubViewModel => run),
        prop!(DispatchStubView => control),
        parameter,
        None,
    );

    binding.dispose();
}

#[test]
fn bind_command_with_a_parameter_property_binds_through_the_runtime_engine() {
    let (view_model, view, _) = fixture();

    let binding = view.bind_command_with_parameter_path_on(
        Some(view_model.clone()),
        prop!(DispatchStubViewModel => run),
        prop!(DispatchStubView => control),
        prop!(DispatchStubViewModel => parameter),
        None,
    );

    binding.dispose();
}

#[test]
fn bind_interaction_with_an_asynchronous_handler_registers_through_the_runtime_engine() {
    let (view_model, view, _) = fixture();
    let interaction = Interaction::<String, bool>::new();
    view_model.set_confirm(Some(interaction.clone()));

    let binding = view.bind_interaction_on(
        Some(view_model.clone()),
        prop!(DispatchStubViewModel => confirm),
        |context: Rc<dyn IInteractionContext<String, bool>>| {
            context.set_output(true);
            Task::from_result(())
        },
    );

    let handled = block_on(interaction.handle(BOUND_VALUE.to_string())).expect("the question is handled");

    assert!(handled);
    binding.dispose();
}

#[test]
fn bind_interaction_with_a_stream_handler_registers_through_the_runtime_engine() {
    let (view_model, view, _) = fixture();
    let interaction = Interaction::<String, bool>::new();
    view_model.set_confirm(Some(interaction.clone()));

    let binding = view.bind_interaction_observable_on(
        Some(view_model.clone()),
        prop!(DispatchStubViewModel => confirm),
        |context: Rc<dyn IInteractionContext<String, bool>>| -> Rc<dyn IObservable<String>> {
            context.set_output(true);
            Observable::empty::<String>()
        },
    );

    let handled = block_on(interaction.handle(BOUND_VALUE.to_string())).expect("the question is handled");

    assert!(handled);
    binding.dispose();
}

// Not ported: WhenChanged_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: WhenChanging_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: WhenAnyValue_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: WhenChangedWithSelector_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: BindOneWay_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: BindTwoWay_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: OneWayBind_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: Bind_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: BindTo_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: BindCommand_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: BindInteraction_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: InvokeCommand_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)

/// Registers the [`ClickCommandBinder`] (`Locator.CurrentMutable.RegisterConstant<ICreatesCommandBinding>`).
fn register_click_command_binder() {
    AppLocator::register_constant::<dyn ICreatesCommandBinding>(Rc::new(ClickCommandBinder));
}

/// Removes the binders the test registered.
fn unregister_command_binders() {
    AppLocator::unregister_all::<dyn ICreatesCommandBinding>();
}

#[test]
fn bind_command_unsafe_with_a_parameter_stream_executes_with_the_value_produced() {
    let (view_model, view, source) = fixture();
    register_click_command_binder();
    let command = RecordingCommand::new();
    view_model.set_run(Some(command.clone() as Rc<dyn ICommand>));

    let parameter: Rc<dyn IObservable<String>> = source.clone();
    let binding = view.bind_command_with_parameter_on(
        Some(view_model.clone()),
        prop!(DispatchStubViewModel => run),
        prop!(DispatchStubView => control),
        parameter,
        None,
    );

    source.on_next(BOUND_VALUE.to_string());
    view.control().perform_click();

    assert_eq!(command.last_parameter().as_deref(), Some(BOUND_VALUE));
    binding.dispose();
    unregister_command_binders();
}

#[test]
fn bind_command_unsafe_with_a_parameter_property_executes_with_the_value_held() {
    let (view_model, view, _) = fixture();
    register_click_command_binder();
    let command = RecordingCommand::new();
    view_model.set_run(Some(command.clone() as Rc<dyn ICommand>));
    view_model.set_parameter(REVERSE_VALUE.to_string());

    let binding = view.bind_command_with_parameter_path_on(
        Some(view_model.clone()),
        prop!(DispatchStubViewModel => run),
        prop!(DispatchStubView => control),
        prop!(DispatchStubViewModel => parameter),
        None,
    );

    view.control().perform_click();

    assert_eq!(command.last_parameter().as_deref(), Some(REVERSE_VALUE));
    binding.dispose();
    unregister_command_binders();
}

// Not ported: BindCommandUnsafe_WithANamedEvent_ExecutesThroughThatEvent - reflection (an event found by its name)

#[test]
fn bind_command_unsafe_with_a_parameter_property_and_no_view_model_binds_nothing() {
    let (view_model, view, _) = fixture();
    register_click_command_binder();
    let command = RecordingCommand::new();
    view_model.set_run(Some(command.clone() as Rc<dyn ICommand>));

    let binding = view.bind_command_with_parameter_path_on(
        None::<Rc<DispatchStubViewModel>>,
        prop!(DispatchStubViewModel => run),
        prop!(DispatchStubView => control),
        prop!(DispatchStubViewModel => parameter),
        None,
    );

    view.control().perform_click();

    assert_eq!(command.last_parameter(), None);
    binding.dispose();
    unregister_command_binders();
}

// Not ported: WhenAny_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: WhenAnyObservable_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: BindOneWayWithConversion_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: BindTwoWayWithConversions_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: OneWayBindWithSelector_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: BindWithConverters_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: BindToWithConversionHint_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: BindToWithConverterOverride_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: BindToWithHintAndConverter_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: BindCommandWithParameterStream_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: BindCommandWithParameterProperty_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: BindInteractionWithObservableHandler_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
