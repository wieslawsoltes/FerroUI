//! `ReactiveUIBuilderConverterTests` (`ReactiveUI.Builder.Tests/ReactiveUIBuilderConverterTests.cs`).
//!
//! `IsSameReferenceAs(builder)` compares the addresses of the builder a method returns and of the
//! builder it was called on.

use super::executors::builder_test;
use ferroui_reactiveui::*;
use ferroui_splat::ModernDependencyResolver;
use std::any::Any;
use std::rc::Rc;

/// A converter of `i32` to `String` (`TestTypedConverter`).
struct TestTypedConverter;

impl BindingTypeConverter for TestTypedConverter {
    type From = i32;
    type To = String;

    fn affinity(&self) -> i32 {
        1
    }

    fn convert(&self, from: Option<&i32>, _conversion_hint: Option<&dyn Any>) -> Option<String> {
        from.map(i32::to_string)
    }
}

/// A converter of `bool` to `String` written against the untyped interface (`TestBindingConverter`).
struct TestBindingConverter;

impl IBindingTypeConverter for TestBindingConverter {
    fn from_type(&self) -> ConverterType {
        ConverterType::of::<bool>()
    }

    fn to_type(&self) -> ConverterType {
        ConverterType::of::<String>()
    }

    fn get_affinity_for_objects(&self) -> i32 {
        1
    }

    fn try_convert_typed(&self, from: Option<&dyn Any>, _conversion_hint: Option<&dyn Any>) -> Option<Rc<dyn Any>> {
        let from = from?.downcast_ref::<bool>()?;
        Some(Rc::new(from.to_string()))
    }
}

/// A fallback converter that hands the value back (`TestFallbackConverter`).
struct TestFallbackConverter;

impl IBindingFallbackConverter for TestFallbackConverter {
    fn get_affinity_for_objects(&self, _from_type: ConverterType, _to_type: ConverterType) -> i32 {
        1
    }

    fn try_convert(
        &self,
        _from_type: ConverterType,
        from: &dyn Any,
        _to_type: ConverterType,
        _conversion_hint: Option<&dyn Any>,
    ) -> Option<Rc<dyn Any>> {
        // Upstream hands back the same object; an untyped value here is borrowed, so the
        // converter hands back a copy of the values it knows.
        from.downcast_ref::<String>().map(|text| Rc::new(text.clone()) as Rc<dyn Any>)
    }
}

#[test]
fn with_converter_typed_converter_returns_builder_for_chaining() {
    builder_test(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();
        let converter: Rc<dyn IBindingTypeConverterOf<i32, String>> = Rc::new(TestTypedConverter);

        let result = builder.with_converter_of(converter);

        assert!(std::ptr::eq(result, &builder));
    });
}

// Not ported: WithConverter_TypedConverter_WithNull_Throws - argument validation

#[test]
fn with_converter_generic_interface_returns_builder_for_chaining() {
    builder_test(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();
        let converter = Rc::new(TestBindingConverter);

        let result = builder.with_converter(converter);

        assert!(std::ptr::eq(result, &builder));
    });
}

// Not ported: WithConverter_GenericInterface_WithNull_Throws - argument validation

#[test]
fn with_converter_typed_factory_returns_builder_for_chaining() {
    builder_test(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();

        let result = builder.with_converter_of_factory(|| Rc::new(TestTypedConverter) as Rc<dyn IBindingTypeConverterOf<i32, String>>);

        assert!(std::ptr::eq(result, &builder));
    });
}

// Not ported: WithConverter_TypedFactory_WithNull_Throws - argument validation

#[test]
fn with_converter_interface_factory_returns_builder_for_chaining() {
    builder_test(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();

        let result = builder.with_converter_factory(|| Rc::new(TestBindingConverter) as Rc<dyn IBindingTypeConverter>);

        assert!(std::ptr::eq(result, &builder));
    });
}

// Not ported: WithConverter_InterfaceFactory_WithNull_Throws - argument validation
// Not ported: WithConvertersFrom_WithNull_Throws - argument validation

#[test]
fn with_converters_from_returns_builder_for_chaining() {
    builder_test(|| {
        let source_locator = ModernDependencyResolver::new();
        let builder = RxAppBuilder::create_reactive_ui_builder();

        let result = builder.with_converters_from(&source_locator);

        assert!(std::ptr::eq(result, &builder));
    });
}

#[test]
fn with_fallback_converter_instance_returns_builder_for_chaining() {
    builder_test(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();
        let converter = Rc::new(TestFallbackConverter);

        let result = builder.with_fallback_converter(converter);

        assert!(std::ptr::eq(result, &builder));
    });
}

// Not ported: WithFallbackConverter_Instance_WithNull_Throws - argument validation

#[test]
fn with_fallback_converter_factory_returns_builder_for_chaining() {
    builder_test(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();

        let result = builder.with_fallback_converter_factory(|| Rc::new(TestFallbackConverter) as Rc<dyn IBindingFallbackConverter>);

        assert!(std::ptr::eq(result, &builder));
    });
}

// Not ported: WithFallbackConverter_Factory_WithNull_Throws - argument validation

#[test]
fn with_fallback_converter_via_interface_typed_variable_does_not_recurse() {
    builder_test(|| {
        // Upstream's regression (issue 4293): the call through the interface recursed into the
        // mixin. The call here goes through the trait object.
        let concrete = RxAppBuilder::create_reactive_ui_builder();
        let builder: &dyn IReactiveUIBuilder = &concrete;
        let converter = Rc::new(TestFallbackConverter);

        let result = builder.with_fallback_converter(converter);

        assert!(std::ptr::addr_eq(result as *const dyn IReactiveUIBuilder, builder as *const dyn IReactiveUIBuilder));
    });
}
