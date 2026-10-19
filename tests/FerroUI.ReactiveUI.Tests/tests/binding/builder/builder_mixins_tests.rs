//! `BuilderMixinsTests` (ReactiveUI.Binding.Tests, `Builder/BuilderMixinsTests.cs`).
//!
//! The mixins continue a chain from a `&dyn IAppBuilder` (upstream's `((IAppBuilder)builder)`).

use super::stubs::{contains, StubBindingTypeConverter, StubFallbackConverter, StubSetMethodBindingConverter, TestModule};
use crate::mocks::panic_message;
use ferroui_reactiveui::*;
use ferroui_splat::{IAppBuilder, IAppInstance, IModule, IMutableDependencyResolver};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The value the stub converter of `FluentChaining_ThroughIAppBuilder_Works` gives
/// (`ConvertedValue`).
const CONVERTED_VALUE: i32 = 42;

#[derive(Default)]
struct MixinTestViewModel;

#[derive(Default)]
struct MixinTestView {
    view_model: RefCell<Option<Rc<MixinTestViewModel>>>,
}
view_for!(MixinTestView, MixinTestViewModel, view_model);

/// A Splat builder that is not a builder of ReactiveUI.Binding (`FakeAppBuilder`).
struct FakeAppBuilder;

impl IAppBuilder for FakeAppBuilder {
    fn build(&self) -> &dyn IAppInstance {
        panic!("NotSupportedException")
    }

    fn use_current_splat_locator(&self) -> &dyn IAppBuilder {
        self.with_core_services()
    }

    fn using_module(&self, _registration_module: Rc<dyn IModule>) -> &dyn IAppBuilder {
        self
    }

    fn with_core_services(&self) -> &dyn IAppBuilder {
        self
    }

    fn with_custom_registration(&self, _configure_action: Rc<dyn Fn(&dyn IMutableDependencyResolver)>) -> &dyn IAppBuilder {
        self
    }
}

/// The builder as a Splat builder (`(IAppBuilder)builder`).
fn app_builder(builder: &Rc<ReactiveUIBindingBuilder>) -> &dyn IAppBuilder {
    &**builder
}

fn int_to_bool() -> Rc<StubBindingTypeConverter> {
    StubBindingTypeConverter::new::<i32, bool>(|_| Some(Rc::new(true) as Rc<dyn Any>))
}

fn fallback(result: &'static str) -> Rc<dyn IBindingFallbackConverter> {
    StubFallbackConverter::new(move |_| Some(Rc::new(result.to_owned()) as Rc<dyn Any>))
}

#[test]
fn build_app_with_valid_builder_succeeds() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let _ = builder.with_core_services();

    let instance = app_builder(&builder).build_app();

    assert!(instance.current().is_some());
}

#[test]
fn build_app_with_non_reactive_ui_builder_throws_invalid_operation_exception() {
    let fake_builder: &dyn IAppBuilder = &FakeAppBuilder;

    let message = panic_message(|| {
        let _ = fake_builder.build_app();
    });

    assert!(message.contains("InvalidOperationException"));
}

#[test]
fn with_platform_module_delegates_to_builder() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let registered = Rc::new(Cell::new(false));

    let result = app_builder(&builder).with_platform_module(TestModule::new(&registered));

    let _ = builder.build_app();

    assert!(registered.get());
    assert!(Rc::ptr_eq(&result, &builder));
}

#[test]
fn with_platform_module_with_non_reactive_ui_builder_throws_invalid_operation_exception() {
    let fake_builder: &dyn IAppBuilder = &FakeAppBuilder;
    let registered = Rc::new(Cell::new(false));

    let message = panic_message(|| {
        let _ = fake_builder.with_platform_module(TestModule::new(&registered));
    });

    assert!(message.contains("InvalidOperationException"));
}

#[test]
fn with_registration_delegates_to_builder() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let executed = Cell::new(false);

    let result = app_builder(&builder).with_registration(|_| executed.set(true));

    assert!(executed.get());
    assert!(Rc::ptr_eq(&result, &builder));
}

#[test]
fn with_registration_with_non_reactive_ui_builder_throws_invalid_operation_exception() {
    let fake_builder: &dyn IAppBuilder = &FakeAppBuilder;

    let message = panic_message(|| {
        let _ = fake_builder.with_registration(|_| {});
    });

    assert!(message.contains("InvalidOperationException"));
}

#[test]
fn with_converter_registers_converter_via_app_builder() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let converter: Rc<dyn IBindingTypeConverter> = int_to_bool();

    let result = app_builder(&builder).with_converter(converter.clone());

    let resolved = builder.converter_service().typed_converters().try_get_converter(ConverterType::of::<i32>(), ConverterType::of::<bool>());

    assert!(Rc::ptr_eq(&result, &builder));
    assert!(resolved.is_some_and(|resolved| Rc::ptr_eq(&resolved, &converter)));
}

#[test]
fn with_converter_with_non_reactive_ui_builder_throws_invalid_operation_exception() {
    let fake_builder: &dyn IAppBuilder = &FakeAppBuilder;
    let converter = int_to_bool();

    let message = panic_message(|| {
        let _ = fake_builder.with_converter(converter);
    });

    assert!(message.contains("InvalidOperationException"));
}

#[test]
fn with_fallback_converter_registers_converter_via_app_builder() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let converter = fallback("converted");

    let result = app_builder(&builder).with_fallback_converter(converter.clone());

    let all_converters = builder.converter_service().fallback_converters().get_all_converters();

    assert!(Rc::ptr_eq(&result, &builder));
    assert!(contains(&all_converters, &converter));
}

#[test]
fn with_fallback_converter_with_non_reactive_ui_builder_throws_invalid_operation_exception() {
    let fake_builder: &dyn IAppBuilder = &FakeAppBuilder;
    let converter = fallback("converted");

    let message = panic_message(|| {
        let _ = fake_builder.with_fallback_converter(converter);
    });

    assert!(message.contains("InvalidOperationException"));
}

#[test]
fn with_set_method_converter_registers_converter_via_app_builder() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let converter: Rc<dyn ISetMethodBindingConverter> = Rc::new(StubSetMethodBindingConverter);

    let result = app_builder(&builder).with_set_method_converter(converter.clone());

    let all_converters = builder.converter_service().set_method_converters().get_all_converters();

    assert!(Rc::ptr_eq(&result, &builder));
    assert!(contains(&all_converters, &converter));
}

#[test]
fn with_set_method_converter_with_non_reactive_ui_builder_throws_invalid_operation_exception() {
    let fake_builder: &dyn IAppBuilder = &FakeAppBuilder;

    let message = panic_message(|| {
        let _ = fake_builder.with_set_method_converter(Rc::new(StubSetMethodBindingConverter));
    });

    assert!(message.contains("InvalidOperationException"));
}

#[test]
fn fluent_chaining_through_i_app_builder_works() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let converter = StubBindingTypeConverter::new::<String, i32>(|_| Some(Rc::new(CONVERTED_VALUE) as Rc<dyn Any>));
    let fallback_converter = fallback("fallback");
    let set_converter: Rc<dyn ISetMethodBindingConverter> = Rc::new(StubSetMethodBindingConverter);

    let result = app_builder(&builder)
        .with_converter(converter)
        .with_fallback_converter(fallback_converter.clone())
        .with_set_method_converter(set_converter.clone());

    let service = builder.converter_service();
    assert!(Rc::ptr_eq(&result, &builder));
    assert!(service.typed_converters().try_get_converter(ConverterType::of::<String>(), ConverterType::of::<i32>()).is_some());
    assert!(contains(&service.fallback_converters().get_all_converters(), &fallback_converter));
    assert!(contains(&service.set_method_converters().get_all_converters(), &set_converter));
}

#[test]
fn configure_view_locator_delegates_to_builder() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();

    let _ = IAppBuilder::with_core_services(&*builder)
        .configure_view_locator(|mappings| {
            let _ = mappings.map::<MixinTestViewModel, MixinTestView>(None);
        })
        .build_app();

    let locator = ViewLocator::current();
    let result = locator.resolve_view_of(&Rc::new(MixinTestViewModel), None);

    assert!(result.is_some());
}

#[test]
fn configure_view_locator_with_non_reactive_ui_builder_throws() {
    let fake_builder: &dyn IAppBuilder = &FakeAppBuilder;

    let message = panic_message(|| {
        let _ = fake_builder.configure_view_locator(|_| {});
    });

    assert!(message.contains("InvalidOperationException"));
}
