//! `BuilderMixinsTests` (`ReactiveUI.Tests/Builder/BuilderMixinsTests.cs`).
//!
//! `IsSameReferenceAs(builder)` compares the addresses of the builder that a method returns and of
//! the builder it was called on.

use ferroui_reactiveui::*;
use std::any::Any;
use std::marker::PhantomData;
use std::rc::Rc;

/// A converter of `F` to `T` that converts nothing (`StubTypedConverter<TFrom, TTo>`).
struct StubTypedConverter<F, T>(PhantomData<fn(F) -> T>);

impl<F, T> StubTypedConverter<F, T> {
    fn new() -> Self {
        Self(PhantomData)
    }
}

impl<F: 'static, T: 'static> BindingTypeConverter for StubTypedConverter<F, T> {
    type From = F;
    type To = T;

    fn affinity(&self) -> i32 {
        0
    }

    fn convert(&self, _from: Option<&F>, _conversion_hint: Option<&dyn Any>) -> Option<T> {
        None
    }
}

/// A fallback converter that converts nothing (`StubFallbackConverter`).
struct StubFallbackConverter;

impl IBindingFallbackConverter for StubFallbackConverter {
    fn get_affinity_for_objects(&self, _from_type: ConverterType, _to_type: ConverterType) -> i32 {
        0
    }

    fn try_convert(
        &self,
        _from_type: ConverterType,
        _from: &dyn Any,
        _to_type: ConverterType,
        _conversion_hint: Option<&dyn Any>,
    ) -> Option<Rc<dyn Any>> {
        None
    }
}

/// A set-method converter that returns the new value (`StubSetMethodConverter`).
struct StubSetMethodConverter;

impl ISetMethodBindingConverter for StubSetMethodConverter {
    fn get_affinity_for_objects(&self, _from_type: Option<ConverterType>, _to_type: Option<ConverterType>) -> i32 {
        0
    }

    fn perform_set(
        &self,
        _to_target: Option<Rc<dyn Any>>,
        new_value: Option<Rc<dyn Any>>,
        _arguments: Option<&[Option<Rc<dyn Any>>]>,
    ) -> Option<Rc<dyn Any>> {
        new_value
    }
}

/// `StubViewModel`.
#[derive(Default)]
struct StubViewModel {
    reactive: ReactiveObject,
}

reactive_object!(StubViewModel);

fn typed() -> Rc<StubTypedConverter<String, i32>> {
    Rc::new(StubTypedConverter::new())
}

#[test]
fn with_converter_overloads_return_same_builder() {
    let builder = RxAppBuilder::create_reactive_ui_builder();

    let after_typed = builder.with_converter_of::<String, i32>(typed());
    let after_interface = builder.with_converter(typed() as Rc<dyn IBindingTypeConverter>);
    let after_typed_factory = builder.with_converter_of_factory::<String, i32>(|| typed() as Rc<dyn IBindingTypeConverterOf<String, i32>>);
    let after_interface_factory = builder.with_converter_factory(|| typed() as Rc<dyn IBindingTypeConverter>);
    let after_many = builder.with_converters(&[typed() as Rc<dyn IBindingTypeConverter>, typed() as Rc<dyn IBindingTypeConverter>]);

    assert!(std::ptr::eq(after_typed, &builder));
    assert!(std::ptr::eq(after_interface, &builder));
    assert!(std::ptr::eq(after_typed_factory, &builder));
    assert!(std::ptr::eq(after_interface_factory, &builder));
    assert!(std::ptr::eq(after_many, &builder));
}

#[test]
fn with_fallback_converter_overloads_return_same_builder() {
    let builder = RxAppBuilder::create_reactive_ui_builder();

    let after_instance = builder.with_fallback_converter(Rc::new(StubFallbackConverter));
    let after_factory = builder.with_fallback_converter_factory(|| Rc::new(StubFallbackConverter) as Rc<dyn IBindingFallbackConverter>);

    assert!(std::ptr::eq(after_instance, &builder));
    assert!(std::ptr::eq(after_factory, &builder));
}

#[test]
fn with_set_method_converter_overloads_return_same_builder() {
    let builder = RxAppBuilder::create_reactive_ui_builder();

    let after_instance = builder.with_set_method_converter(Rc::new(StubSetMethodConverter));
    let after_factory = builder.with_set_method_converter_factory(|| Rc::new(StubSetMethodConverter) as Rc<dyn ISetMethodBindingConverter>);

    assert!(std::ptr::eq(after_instance, &builder));
    assert!(std::ptr::eq(after_factory, &builder));
}

#[test]
fn with_converters_from_returns_same_builder() {
    let builder = RxAppBuilder::create_reactive_ui_builder();

    let result = builder.with_converters_from(&ferroui_splat::ModernDependencyResolver::new());

    assert!(std::ptr::eq(result, &builder));
}

#[test]
fn with_message_bus_and_register_constant_view_model_return_same_builder() {
    let builder = RxAppBuilder::create_reactive_ui_builder();

    let after_bus = builder.with_message_bus();
    let after_view_model = builder.register_constant_view_model::<StubViewModel>();

    assert!(std::ptr::eq(after_bus, &builder));
    assert!(std::ptr::eq(after_view_model, &builder));
}
