//! `ReactiveUIBindingBuilderTests` (ReactiveUI.Binding.Tests, `Builder/ReactiveUIBindingBuilderTests.cs`).

use super::stubs::{contains, StubBindingTypeConverter, StubCommandBinder, StubFallbackConverter, StubSetMethodBindingConverter, TestModule};
use ferroui_reactiveui::*;
use ferroui_splat::{AppLocator, ReadonlyDependencyResolverExt};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The observers the core services register at least (`MinimumCoreObservableServices`).
const MINIMUM_CORE_OBSERVABLE_SERVICES: usize = 2;

#[derive(Default)]
struct ConfigTestViewModel;

#[derive(Default)]
struct ConfigTestView {
    view_model: RefCell<Option<Rc<ConfigTestViewModel>>>,
}
view_for!(ConfigTestView, ConfigTestViewModel, view_model);

/// A typed converter as the registries hold it.
fn typed(converter: &Rc<StubBindingTypeConverter>) -> Rc<dyn IBindingTypeConverter> {
    converter.clone()
}

#[test]
fn constructor_creates_converter_service() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();

    assert!(Rc::strong_count(builder.converter_service()) >= 1);
}

#[test]
fn constructor_registers_converter_service_instance() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let app = builder.build_app();

    let registered = app.current().and_then(|current| current.get_service::<ConverterService>());

    assert!(registered.is_some_and(|registered| Rc::ptr_eq(&registered, builder.converter_service())));
}

#[test]
fn with_core_services_registers_observation_services() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();

    let _ = builder.with_core_services();
    let _ = builder.build_app();

    let services = AppLocator::get_services::<dyn ICreatesObservableForProperty>();

    assert!(services.len() >= MINIMUM_CORE_OBSERVABLE_SERVICES);
}

#[test]
fn with_platform_module_registers_module() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let registered = Rc::new(Cell::new(false));

    let _ = builder.with_platform_module(TestModule::new(&registered));
    let _ = builder.build_app();

    assert!(registered.get());
}

#[test]
fn with_registration_executes_action() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let executed = Cell::new(false);

    let _ = builder.with_registration(|_| executed.set(true));

    assert!(executed.get());
}

#[test]
fn build_app_sets_global_converter_service() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let _ = builder.with_core_services();

    let instance = builder.build_app();

    assert!(instance.current().is_some());
    assert!(Rc::strong_count(&BindingConverters::current()) >= 1);
}

#[test]
fn with_core_services_called_twice_only_registers_once() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();

    let _ = builder.with_core_services();
    let _ = builder.with_core_services();
    let _ = builder.build_app();

    // Does not panic; the services are registered once.
    let services = AppLocator::get_services::<dyn ICreatesObservableForProperty>();
    assert!(services.len() >= MINIMUM_CORE_OBSERVABLE_SERVICES);
}

#[test]
fn fluent_api_supports_chaining_calls() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();

    let _ = builder.with_core_services();
    let result = builder.with_registration(|_| {});

    assert!(Rc::ptr_eq(&result, &builder));
}

#[test]
fn with_converter_registers_in_typed_converters() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let converter = StubBindingTypeConverter::new::<i32, bool>(|_| Some(Rc::new(true) as Rc<dyn Any>));

    let _ = builder.with_converter(typed(&converter));

    let resolved = builder.converter_service().typed_converters().try_get_converter(ConverterType::of::<i32>(), ConverterType::of::<bool>());
    assert!(resolved.is_some_and(|resolved| Rc::ptr_eq(&resolved, &typed(&converter))));
}

#[test]
fn with_converter_returns_self_for_chaining() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let converter = StubBindingTypeConverter::new::<i32, String>(|from| from.and_then(|from| from.downcast_ref::<i32>()).map(|from| Rc::new(from.to_string()) as Rc<dyn Any>));

    let result = builder.with_converter(converter);

    assert!(Rc::ptr_eq(&result, &builder));
}

#[test]
fn with_fallback_converter_registers_in_fallback_converters() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let converter: Rc<dyn IBindingFallbackConverter> = StubFallbackConverter::new(|_| Some(Rc::new("converted".to_owned()) as Rc<dyn Any>));

    let _ = builder.with_fallback_converter(converter.clone());

    let all_converters = builder.converter_service().fallback_converters().get_all_converters();
    assert!(contains(&all_converters, &converter));
}

#[test]
fn with_fallback_converter_returns_self_for_chaining() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let converter = StubFallbackConverter::new(|_| Some(Rc::new("converted".to_owned()) as Rc<dyn Any>));

    let result = builder.with_fallback_converter(converter);

    assert!(Rc::ptr_eq(&result, &builder));
}

#[test]
fn with_set_method_converter_registers_in_set_method_converters() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let converter: Rc<dyn ISetMethodBindingConverter> = Rc::new(StubSetMethodBindingConverter);

    let _ = builder.with_set_method_converter(converter.clone());

    let all_converters = builder.converter_service().set_method_converters().get_all_converters();
    assert!(contains(&all_converters, &converter));
}

#[test]
fn with_set_method_converter_returns_self_for_chaining() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();

    let result = builder.with_set_method_converter(Rc::new(StubSetMethodBindingConverter));

    assert!(Rc::ptr_eq(&result, &builder));
}

#[test]
fn with_core_services_registers_default_converters() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();

    let _ = builder.with_core_services();

    // Spot check: the string converter converts a string to a string.
    let string_converter = builder.converter_service().typed_converters().try_get_converter(ConverterType::of::<String>(), ConverterType::of::<String>());
    assert!(string_converter.is_some());
}

#[test]
fn with_core_services_registers_integer_to_string_converter() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();

    let _ = builder.with_core_services();

    let converter = builder.converter_service().typed_converters().try_get_converter(ConverterType::of::<i32>(), ConverterType::of::<String>());
    assert!(converter.is_some());
}

#[test]
fn with_core_services_explicit_interface_returns_i_reactive_ui_binding_builder() {
    RxBindingBuilder::reset_for_testing();
    let builder: Rc<dyn IReactiveUIBindingBuilder> = RxBindingBuilder::create_reactive_ui_binding_builder();

    let result = builder.with_core_services();

    assert!(std::ptr::addr_eq(result as *const dyn ferroui_splat::IAppBuilder, Rc::as_ptr(&builder)));
}

#[test]
fn build_app_sets_global_converter_service_to_builder_instance() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let _ = builder.with_core_services();

    let _ = builder.build_app();

    assert!(Rc::ptr_eq(&BindingConverters::current(), builder.converter_service()));
}

#[test]
fn build_app_marks_as_initialized() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let _ = builder.with_core_services();

    let _ = builder.build_app();

    // Does not panic.
    RxBindingBuilder::ensure_initialized();
}

#[test]
fn multiple_converter_types_all_registered() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let typed_converter = StubBindingTypeConverter::new::<f64, String>(|from| from.and_then(|from| from.downcast_ref::<f64>()).map(|from| Rc::new(from.to_string()) as Rc<dyn Any>));
    let fallback: Rc<dyn IBindingFallbackConverter> = StubFallbackConverter::new(|_| Some(Rc::new("fallback".to_owned()) as Rc<dyn Any>));
    let set_method: Rc<dyn ISetMethodBindingConverter> = Rc::new(StubSetMethodBindingConverter);

    let _ = builder.with_converter(typed_converter);
    let _ = builder.with_fallback_converter(fallback.clone());
    let _ = builder.with_set_method_converter(set_method.clone());

    let service = builder.converter_service();
    assert!(service.typed_converters().try_get_converter(ConverterType::of::<f64>(), ConverterType::of::<String>()).is_some());
    assert!(contains(&service.fallback_converters().get_all_converters(), &fallback));
    assert!(contains(&service.set_method_converters().get_all_converters(), &set_method));
}

#[test]
fn with_command_binder_registers_in_splat() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let binder: Rc<dyn ICreatesCommandBinding> = Rc::new(StubCommandBinder);

    let _ = builder.with_command_binder(binder.clone());
    let _ = builder.build_app();

    let services = AppLocator::get_services::<dyn ICreatesCommandBinding>();
    assert!(contains(&services, &binder));
}

#[test]
fn with_command_binder_returns_self_for_chaining() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();

    let result = builder.with_command_binder(Rc::new(StubCommandBinder));

    assert!(Rc::ptr_eq(&result, &builder));
}

// Not ported: WithCommandBinder_NullBinder_Throws - argument validation

#[test]
fn configure_view_locator_registers_configured_locator() {
    RxBindingBuilder::reset_for_testing();
    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();

    let _ = builder.with_core_services();
    let _ = builder.configure_view_locator(|mappings| {
        let _ = mappings.map::<ConfigTestViewModel, ConfigTestView>(None);
    });
    let _ = builder.build_app();

    let locator = ViewLocator::current();
    let result = locator.resolve_view_of(&Rc::new(ConfigTestViewModel), None);

    assert!(result.is_some_and(|view| (view as Rc<dyn Any>).is::<ConfigTestView>()));
}
