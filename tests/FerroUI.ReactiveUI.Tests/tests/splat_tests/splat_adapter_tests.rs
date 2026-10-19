//! `SplatAdapterTests` (`ReactiveUI.Splat.Tests/SplatAdapterTests.cs`).
//!
//! Upstream builds ReactiveUI over the Splat adapters of three .NET containers (DryIoc, Autofac,
//! Ninject) and checks that the container then holds the converter service, with its default
//! converters, and the default property observers. The containers are .NET libraries; each test
//! here gives the builder a Splat `ModernDependencyResolver` of its own, in the place of the
//! adapter, the way the test gives it (as the current locator, or to the builder directly), and
//! keeps the checks.
//!
//! The checks of the concrete types of upstream (`is StringConverter`, `is
//! INPCObservableForProperty`) are checks of what the services do: the typed converter of
//! `String` to `String` and of `object` to `bool`, and an observer with the affinity of the
//! observer of `INotifyPropertyChanged` objects, and one with the fallback affinity for any object.

use crate::mocks::NonReactiveInpcObject;
use ferroui_reactiveui::*;
use ferroui_splat::{AppLocator, ModernDependencyResolver, ReadonlyDependencyResolverExt};
use ferroui_system::component_model::{INotifyPropertyChanged, PropertyOwner};
use std::any::Any;
use std::rc::Rc;

/// The converter service in `resolver`, with its default converters.
fn assert_binding_converter_service(resolver: &ModernDependencyResolver) {
    let converters = resolver.get_service::<ConverterService>().expect("the builder registers its converter service");

    let string_converter = converters.resolve_converter(ConverterType::of::<String>(), ConverterType::of::<String>());
    let string_converter = string_converter.as_ref().and_then(|converter| converter.as_typed()).expect("a typed converter of strings");
    assert_eq!(string_converter.from_type(), ConverterType::of::<String>());
    assert_eq!(string_converter.to_type(), ConverterType::of::<String>());

    let equality_converter = converters.resolve_converter(ConverterType::of::<dyn Any>(), ConverterType::of::<bool>());
    let equality_converter = equality_converter.as_ref().and_then(|converter| converter.as_typed()).expect("a typed converter of objects to booleans");
    assert_eq!(equality_converter.from_type(), ConverterType::of::<dyn Any>());
    assert_eq!(equality_converter.to_type(), ConverterType::of::<bool>());
}

/// The default property observers in `resolver`.
fn assert_binding_property_observers(resolver: &ModernDependencyResolver) {
    let observers = resolver.get_services::<dyn ICreatesObservableForProperty>();
    assert!(!observers.is_empty());

    let inpc_object = NonReactiveInpcObject::new();
    let notifying_owner = PropertyOwner {
        object: Some(inpc_object.clone() as Rc<dyn Any>),
        notify: Some(inpc_object as Rc<dyn INotifyPropertyChanged>),
    };
    let plain_owner = PropertyOwner { object: Some(Rc::new(0_i32) as Rc<dyn Any>), notify: None };

    // `x is INPCObservableForProperty`.
    assert!(observers
        .iter()
        .any(|observer| observer.get_affinity_for_object(&notifying_owner, "Property", false) == BindingAffinity::EXPLICIT));
    // `x is POCOObservableForProperty`.
    assert!(observers
        .iter()
        .any(|observer| observer.get_affinity_for_object(&plain_owner, "Property", false) == BindingAffinity::FALLBACK));
}

/// `container.UseXDependencyResolver()` then `Locator.CurrentMutable.CreateReactiveUIBuilder()`:
/// the container becomes the current locator, which the builder is created over.
fn build_over_current_locator() -> Rc<ModernDependencyResolver> {
    let container = Rc::new(ModernDependencyResolver::new());
    AppLocator::set_locator(container.clone());

    let _ = AppLocator::current_mutable().create_reactive_ui_builder().with_core_services().build();
    container
}

/// `new AutofacDependencyResolver(builder).CreateReactiveUIBuilder()`: the builder is created
/// over the container itself.
fn build_over_resolver() -> Rc<ModernDependencyResolver> {
    let container = Rc::new(ModernDependencyResolver::new());

    let _ = container.create_reactive_ui_builder().with_core_services().build();
    container
}

#[test]
fn dry_ioc_dependency_resolver_should_register_binding_converter_service() {
    let container = build_over_current_locator();

    assert_binding_converter_service(&container);
}

#[test]
fn dry_ioc_dependency_resolver_should_register_binding_property_observers() {
    let container = build_over_current_locator();

    assert_binding_property_observers(&container);
}

#[test]
fn autofac_dependency_resolver_should_register_binding_converter_service() {
    let container = build_over_resolver();

    assert_binding_converter_service(&container);
}

#[test]
fn autofac_dependency_resolver_should_register_binding_property_observers() {
    let container = build_over_resolver();
    AppLocator::set_locator(container.clone());

    assert_binding_property_observers(&container);
}

#[test]
fn ninject_dependency_resolver_should_register_binding_converter_service() {
    let container = build_over_current_locator();

    assert_binding_converter_service(&container);
}

#[test]
fn ninject_dependency_resolver_should_register_binding_property_observers() {
    let container = build_over_current_locator();

    assert_binding_property_observers(&container);
}
