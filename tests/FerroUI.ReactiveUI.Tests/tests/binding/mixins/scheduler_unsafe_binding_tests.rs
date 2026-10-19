//! `SchedulerUnsafeBindingTests` (`ReactiveUI.Binding.Tests/Mixins/SchedulerUnsafeBindingTests.cs`).
//!
//! The `Unsafe` calls that name a scheduler are the `_on` and `_with` forms of the bindings; `None`
//! is upstream's `null` scheduler, which writes on the thread of the object written to. The
//! refusals of the stubs a generator displaces have no counterpart.

use crate::tests::binding::test_models::{DispatchStubView, DispatchStubViewModel};
use ferroui_reactiveui::*;
use ferroui_system::prop;
use std::any::Any;
use std::rc::Rc;

const BOUND_VALUE: &str = "bound";

const REVERSE_VALUE: &str = "reversed";

/// The suffix [`SuffixConverter`] appends.
const SUFFIX: &str = "-converted";

/// The affinity that wins over every built-in converter.
const WINNING_AFFINITY: i32 = 100;

/// Appends [`SUFFIX`] (`SuffixConverter`).
struct SuffixConverter;

impl BindingTypeConverter for SuffixConverter {
    type From = String;
    type To = String;

    fn affinity(&self) -> i32 {
        WINNING_AFFINITY
    }

    fn convert(&self, from: Option<&String>, _conversion_hint: Option<&dyn Any>) -> Option<String> {
        from.map(|text| format!("{text}{SUFFIX}"))
    }
}

/// Removes [`SUFFIX`] when the text ends with it (`StripSuffixConverter`).
struct StripSuffixConverter;

impl BindingTypeConverter for StripSuffixConverter {
    type From = String;
    type To = String;

    fn affinity(&self) -> i32 {
        WINNING_AFFINITY
    }

    fn convert(&self, from: Option<&String>, _conversion_hint: Option<&dyn Any>) -> Option<String> {
        from.map(|text| text.strip_suffix(SUFFIX).unwrap_or(text).to_string())
    }
}

fn suffix() -> Option<Rc<dyn IBindingTypeConverter>> {
    Some(Rc::new(SuffixConverter))
}

fn strip_suffix() -> Option<Rc<dyn IBindingTypeConverter>> {
    Some(Rc::new(StripSuffixConverter))
}

/// The source and the target of the tests of two objects.
fn source_and_target() -> (Rc<DispatchStubViewModel>, Rc<DispatchStubView>) {
    let source = DispatchStubViewModel::new();
    source.set_caption(BOUND_VALUE.to_string());
    (source, DispatchStubView::new())
}

/// The view model and the view holding it, of the view-first tests.
fn view_and_view_model() -> (Rc<DispatchStubViewModel>, Rc<DispatchStubView>) {
    let view_model = DispatchStubViewModel::new();
    view_model.set_caption(BOUND_VALUE.to_string());
    let view = DispatchStubView::new();
    IViewFor::set_view_model(&*view, Some(view_model.clone() as Rc<dyn Any>));
    (view_model, view)
}

#[test]
fn bind_one_way_unsafe_carries_the_source_value() {
    let (source, target) = source_and_target();

    let binding = source.bind_one_way_on(&target, prop!(DispatchStubViewModel => caption), settable_prop!(DispatchStubView => caption), None);

    assert_eq!(target.caption(), BOUND_VALUE);
    binding.dispose();
}

#[test]
fn bind_one_way_unsafe_with_a_conversion_applies_it() {
    let (source, target) = source_and_target();

    let binding = source.bind_one_way_select_on(
        &target,
        prop!(DispatchStubViewModel => caption),
        settable_prop!(DispatchStubView => caption),
        |value: String| format!("{value}!"),
        None,
    );

    assert_eq!(target.caption(), format!("{BOUND_VALUE}!"));
    binding.dispose();
}

#[test]
fn bind_one_way_unsafe_with_a_converter_applies_it() {
    let (source, target) = source_and_target();

    let binding = source.bind_one_way_with(&target, prop!(DispatchStubViewModel => caption), settable_prop!(DispatchStubView => caption), suffix(), None, None);

    assert_eq!(target.caption(), format!("{BOUND_VALUE}{SUFFIX}"));
    binding.dispose();
}

#[test]
fn bind_two_way_unsafe_carries_both_directions() {
    let (source, target) = source_and_target();

    let binding = source.bind_two_way_on(&target, settable_prop!(DispatchStubViewModel => caption), settable_prop!(DispatchStubView => caption), None);
    let after_bind = target.caption();
    target.set_caption(REVERSE_VALUE.to_string());

    assert_eq!(after_bind, BOUND_VALUE);
    assert_eq!(source.caption(), REVERSE_VALUE);
    binding.dispose();
}

#[test]
fn bind_two_way_unsafe_with_conversions_applies_them() {
    let (source, target) = source_and_target();

    let binding = source.bind_two_way_select_on(
        &target,
        settable_prop!(DispatchStubViewModel => caption),
        settable_prop!(DispatchStubView => caption),
        |value: String| value,
        |value: String| value,
        None,
    );

    assert_eq!(target.caption(), BOUND_VALUE);
    binding.dispose();
}

#[test]
fn bind_two_way_unsafe_with_converters_applies_them() {
    let (source, target) = source_and_target();

    let binding = source.bind_two_way_with(
        &target,
        settable_prop!(DispatchStubViewModel => caption),
        settable_prop!(DispatchStubView => caption),
        suffix(),
        suffix(),
        None,
        None,
    );

    assert_eq!(target.caption(), format!("{BOUND_VALUE}{SUFFIX}"));
    binding.dispose();
}

#[test]
fn one_way_bind_unsafe_with_a_selector_projects_through_it() {
    let (view_model, view) = view_and_view_model();

    let binding = view
        .one_way_bind_select_on(view_model.clone(), prop!(DispatchStubViewModel => caption), settable_prop!(DispatchStubView => caption), |value: String| format!("{value}!"), None)
        .expect("no hook refuses the binding");

    assert_eq!(view.caption(), format!("{BOUND_VALUE}!"));
    binding.dispose();
}

#[test]
fn one_way_bind_unsafe_with_a_converter_applies_it() {
    let (view_model, view) = view_and_view_model();

    let binding = view
        .one_way_bind_with(view_model.clone(), prop!(DispatchStubViewModel => caption), settable_prop!(DispatchStubView => caption), suffix(), None, None)
        .expect("no hook refuses the binding");

    assert_eq!(view.caption(), format!("{BOUND_VALUE}{SUFFIX}"));
    binding.dispose();
}

#[test]
fn bind_unsafe_carries_both_directions() {
    let (view_model, view) = view_and_view_model();

    let binding = view
        .bind_select_on(
            view_model.clone(),
            settable_prop!(DispatchStubViewModel => caption),
            settable_prop!(DispatchStubView => caption),
            |value: String| value,
            |value: String| value,
            None,
        )
        .expect("no hook refuses the binding");

    let after_bind = view.caption();
    view.set_caption(REVERSE_VALUE.to_string());

    assert_eq!(after_bind, BOUND_VALUE);
    assert_eq!(view_model.caption(), REVERSE_VALUE);
    binding.dispose();
}

#[test]
fn bind_unsafe_with_converters_applies_them() {
    let (view_model, view) = view_and_view_model();

    let binding = view
        .bind_with(view_model.clone(), settable_prop!(DispatchStubViewModel => caption), settable_prop!(DispatchStubView => caption), suffix(), suffix(), None, None)
        .expect("no hook refuses the binding");

    assert_eq!(view.caption(), format!("{BOUND_VALUE}{SUFFIX}"));
    binding.dispose();
}

#[test]
fn bind_unsafe_with_converters_converts_the_view_write() {
    let (view_model, view) = view_and_view_model();

    let binding = view
        .bind_with(
            view_model.clone(),
            settable_prop!(DispatchStubViewModel => caption),
            settable_prop!(DispatchStubView => caption),
            suffix(),
            strip_suffix(),
            None,
            None,
        )
        .expect("no hook refuses the binding");

    view.set_caption(format!("{REVERSE_VALUE}{SUFFIX}"));

    assert_eq!(view_model.caption(), REVERSE_VALUE);
    binding.dispose();
}

// Not ported: BindOneWay_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: BindOneWay_ConvertingWithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: BindOneWay_WithAConverterAndNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: BindTwoWay_WithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: BindTwoWay_ConvertingWithNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: BindTwoWay_WithConvertersAndNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: OneWayBind_WithASelectorAndNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: OneWayBind_WithAConverterAndNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: Bind_WithConversionsAndNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
// Not ported: Bind_WithConvertersAndNoGeneratedOverload_NamesTheUnsafeTwin - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
