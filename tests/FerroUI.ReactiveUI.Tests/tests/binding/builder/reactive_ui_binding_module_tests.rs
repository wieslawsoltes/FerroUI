//! `ReactiveUIBindingModuleTests` (ReactiveUI.Binding.Tests, `Builder/ReactiveUIBindingModuleTests.cs`).
//!
//! A registered observer is told apart by its affinity, as `dyn ICreatesObservableForProperty` is
//! not `Any`: the observer of `INotifyPropertyChanged` answers `EXPLICIT` for an object that
//! raises `PropertyChanged`, and the observer of plain objects answers `FALLBACK` for any object.

use ferroui_reactiveui::*;
use ferroui_splat::{IModule, ModernDependencyResolver, ReadonlyDependencyResolverExt};
use ferroui_system::component_model::{INotifyPropertyChanged, PropertyOwner};
use ferroui_system::events::Event;
use std::any::Any;
use std::rc::Rc;

/// The observers the module registers (`ExpectedObservableServices`).
const EXPECTED_OBSERVABLE_SERVICES: usize = 2;

/// An object that raises `PropertyChanged`.
struct Notifying {
    property_changed: Event<str>,
}

impl INotifyPropertyChanged for Notifying {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

#[test]
fn configure_registers_inpc_and_poco() {
    let resolver = Rc::new(ModernDependencyResolver::new());
    let module = ReactiveUIBindingModule;

    module.configure(&*resolver);

    let services = resolver.get_services::<dyn ICreatesObservableForProperty>();

    assert_eq!(services.len(), EXPECTED_OBSERVABLE_SERVICES);

    let notifying = Rc::new(Notifying { property_changed: Event::new() });
    let notifying_owner = PropertyOwner {
        object: Some(notifying.clone() as Rc<dyn Any>),
        notify: Some(notifying as Rc<dyn INotifyPropertyChanged>),
    };
    let plain_owner = PropertyOwner { object: Some(Rc::new(0_i32) as Rc<dyn Any>), notify: None };

    let has_inpc = services.iter().any(|service| service.get_affinity_for_object(&notifying_owner, "Name", false) == BindingAffinity::EXPLICIT);
    let has_poco = services.iter().any(|service| service.get_affinity_for_object(&plain_owner, "Name", false) == BindingAffinity::FALLBACK);

    assert!(has_inpc);
    assert!(has_poco);
}

#[test]
fn configure_no_command_binders() {
    let resolver = Rc::new(ModernDependencyResolver::new());
    let module = ReactiveUIBindingModule;

    module.configure(&*resolver);

    let binders = resolver.get_services::<dyn ICreatesCommandBinding>();

    assert!(binders.is_empty());
}

// Not ported: Configure_NullResolver_ThrowsArgumentNullException - argument validation
