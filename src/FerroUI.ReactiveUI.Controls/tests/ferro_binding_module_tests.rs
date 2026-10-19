//! `...BindingModuleTests` (the tests of ReactiveUI.Binding's integration).
//!
//! A registered service is told apart by its affinity, as the traits of the services are not
//! `Any`: the property observer answers the affinity of a registered property for a FerroUI
//! object, and the command binder the affinity of a command source for a button.

use super::binding_support::{start_app, TestObject};
use crate::{
    control_type, ferro_property_owner, FerroBindingModule, FerroCreatesCommandBinding, FerroObjectObservableForProperty,
    FerroViewThreadInvoker,
};
use ferroui_controls::Button;
use ferroui_reactiveui::{ICreatesCommandBinding, ICreatesObservableForProperty, IViewThreadInvoker};
use ferroui_splat::{IModule, ModernDependencyResolver, ReadonlyDependencyResolverExt};
use std::rc::Rc;

#[test]
fn configure_registers_the_property_observer() {
    let _app = start_app();
    let resolver = ModernDependencyResolver::new();

    FerroBindingModule::new().configure(&resolver);

    let owner = ferro_property_owner(&TestObject::new());
    let observers = resolver.get_services::<dyn ICreatesObservableForProperty>();
    assert!(observers
        .iter()
        .any(|observer| observer.get_affinity_for_object(&owner, "Text", false) == FerroObjectObservableForProperty::FERRO_PROPERTY_AFFINITY));
}

#[test]
fn configure_registers_the_command_binder() {
    let _app = start_app();
    let resolver = ModernDependencyResolver::new();

    FerroBindingModule::new().configure(&resolver);

    let button = control_type::<Button>();
    let binders = resolver.get_services::<dyn ICreatesCommandBinding>();
    assert!(binders
        .iter()
        .any(|binder| binder.get_affinity_for_object(button, false) == FerroCreatesCommandBinding::COMMAND_SOURCE_AFFINITY));
}

#[test]
fn configure_registers_the_shared_view_thread_invoker() {
    let resolver = ModernDependencyResolver::new();

    FerroBindingModule::new().configure(&resolver);

    let shared = FerroViewThreadInvoker::instance();
    let invokers = resolver.get_services::<dyn IViewThreadInvoker>();
    assert!(invokers.iter().any(|invoker| std::ptr::addr_eq(Rc::as_ptr(invoker), Rc::as_ptr(&shared))));
}

// Not ported: Configure_NullResolver_ThrowsArgumentNullException - argument validation
