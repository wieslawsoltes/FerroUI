//! Ported from `Splat.Tests/LocatorTests.cs` of Splat.
//! Adapted: the `[Before(Test)]`/`[After(Test)]` scope is an `AppLocatorScope` at the start of each
//! test. Upstream's `new InternalLocator()` has the services of Splat because the static
//! constructor of `Locator`, run by an earlier test of the process, set the resolver initializer;
//! here the state is per thread, so the tests that expect those services use `Locator` first. The
//! `Type`-based `GetService(typeof(ILogManager))` is the generic `get_service::<dyn ILogManager>`
//! (the type only names the service). `IsTypeOf<DebugLogger>` and `IsTypeOf<DefaultLogManager>`
//! have no form on `Rc<dyn ILogger>` and `Rc<dyn ILogManager>`, so those tests check that the
//! services are there. A nullable `DummyObjectClass1?` is the service `DummyObjectClass1`, whose
//! factory returns `None` for `null` and `default`. `using var` is an explicit `dispose`, and
//! `Contains` of a list of services is the identity of the `Rc`.

use crate::tests::common::{
    panics, same, AppLocatorScope, DummyObjectClass1, DummyObjectClass2, DummyObjectClass3,
    IDummyInterface,
};
use ferroui_splat::service_location::InternalLocator;
use ferroui_splat::{
    DependencyResolverMixins, ILogManager, ILogger, Locator, ModernDependencyResolver,
    MutableDependencyResolverExt, ReadonlyDependencyResolverExt,
};
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

const REGISTERED_DUMMY_COUNT: usize = 3;

const REMAINING_DUMMY_COUNT: usize = 2;

#[test]
fn set_locator_replaces_current_resolver() {
    let _scope = AppLocatorScope::new();
    let resolver = Rc::new(ModernDependencyResolver::new());

    Locator::set_locator(resolver.clone());

    assert!(same(&Locator::get_locator(), &resolver));
}

#[test]
fn suppress_resolver_callback_changed_notifications_disables_notifications_until_disposed() {
    let _scope = AppLocatorScope::new();
    assert!(Locator::are_resolver_callback_changed_notifications_enabled());

    let suppression = Locator::suppress_resolver_callback_changed_notifications();
    assert!(!Locator::are_resolver_callback_changed_notifications_enabled());
    suppression.dispose();

    assert!(Locator::are_resolver_callback_changed_notifications_enabled());
}

#[test]
fn initialize_splat_registrations_not_empty_no_registrations() {
    let _scope = AppLocatorScope::new();
    // The first use of Locator on the thread sets the resolver initializer, as its static
    // constructor does upstream.
    let _ = Locator::current();

    let test_locator = InternalLocator::new();
    let log_manager = test_locator.current().get_service::<dyn ILogManager>();
    let logger = test_locator.current().get_service::<dyn ILogger>();

    assert!(log_manager.is_some());
    assert!(logger.is_some());
    test_locator.dispose();
}

#[test]
fn initialize_splat_contract_registrations_null_no_registration() {
    let _scope = AppLocatorScope::new();
    let _ = Locator::current();

    let test_locator = InternalLocator::new();
    let log_manager = test_locator.current().get_service_with_contract::<dyn ILogManager>(Some("test"));
    let logger = test_locator.current().get_service_with_contract::<dyn ILogger>(Some("test"));

    assert!(log_manager.is_none());
    assert!(logger.is_none());
    test_locator.dispose();
}

#[test]
fn initialize_splat_contract_registrations_extension_methods_null_no_registration() {
    let _scope = AppLocatorScope::new();
    let _ = Locator::current();

    let test_locator = InternalLocator::new();
    let log_manager = test_locator.current().get_service_with_contract::<dyn ILogManager>(Some("test"));
    let logger = test_locator.current().get_service_with_contract::<dyn ILogger>(Some("test"));

    assert!(log_manager.is_none());
    assert!(logger.is_none());
    test_locator.dispose();
}

#[test]
fn initialize_splat_extension_methods_not_null() {
    let _scope = AppLocatorScope::new();
    let _ = Locator::current();

    let test_locator = InternalLocator::new();
    let log_manager = test_locator.current().get_service::<dyn ILogManager>();
    let logger = test_locator.current().get_service::<dyn ILogger>();

    assert!(log_manager.is_some());
    assert!(logger.is_some());
    test_locator.dispose();
}

#[test]
fn without_suppress_notifications_happen() {
    let _scope = AppLocatorScope::new();
    let test_locator = InternalLocator::new();
    let original_locator = test_locator.internal();

    let number_notifications = Rc::new(Cell::new(0));
    let counter = number_notifications.clone();

    let _subscription = test_locator.register_resolver_callback_changed(Rc::new(move || counter.set(counter.get() + 1)));

    test_locator.set_locator(Rc::new(ModernDependencyResolver::new()));
    test_locator.set_locator(Rc::new(ModernDependencyResolver::new()));

    // 1 for the callback being immediately called when registered, plus 2 for the two set_locator
    // changes.
    const IMMEDIATE_CALLBACK_COUNT: i32 = 1;
    const SET_LOCATOR_CHANGE_COUNT: i32 = 2;
    assert_eq!(number_notifications.get(), IMMEDIATE_CALLBACK_COUNT + SET_LOCATOR_CHANGE_COUNT);

    test_locator.set_locator(original_locator);
}

#[test]
fn with_suppression_notifications_dont_happen() {
    let _scope = AppLocatorScope::new();
    let test_locator = InternalLocator::new();
    let original_locator = test_locator.internal();

    let suppression = test_locator.suppress_resolver_callback_changed_notifications();
    let number_notifications = Rc::new(Cell::new(0));
    let counter = number_notifications.clone();

    let _subscription = test_locator.register_resolver_callback_changed(Rc::new(move || counter.set(counter.get() + 1)));

    test_locator.set_locator(Rc::new(ModernDependencyResolver::new()));
    test_locator.set_locator(Rc::new(ModernDependencyResolver::new()));

    assert_eq!(number_notifications.get(), 0);

    test_locator.set_locator(original_locator);
    suppression.dispose();
}

#[test]
fn with_resolver_notifications_dont_happen() {
    let _scope = AppLocatorScope::new();
    let number_notifications = Rc::new(Cell::new(0));
    let counter = number_notifications.clone();

    let test_locator = InternalLocator::new();
    let _subscription = test_locator.register_resolver_callback_changed(Rc::new(move || counter.set(counter.get() + 1)));

    let outer_resolver = test_locator.internal().with_resolver();
    let inner_resolver = test_locator.internal().with_resolver();
    inner_resolver.dispose();
    outer_resolver.dispose();

    // 1 due to the fact the callback is called when we register.
    assert_eq!(number_notifications.get(), 1);
}

#[test]
fn with_resolver_notifications_not_suppressed_happen() {
    let _scope = AppLocatorScope::new();
    let number_notifications = Rc::new(Cell::new(0));
    let counter = number_notifications.clone();

    let _subscription = Locator::register_resolver_callback_changed(move || counter.set(counter.get() + 1));

    let outer_resolver = Locator::get_locator().with_resolver_suppressing(false);
    let inner_resolver = Locator::get_locator().with_resolver_suppressing(false);
    inner_resolver.dispose();
    outer_resolver.dispose();

    // 1 for the callback being called immediately when we register, then each
    // with_resolver_suppressing(false) scope fires twice (once for the change to the resolver and
    // once for the change back).
    const IMMEDIATE_CALLBACK_COUNT: i32 = 1;
    const NOTIFICATIONS_PER_SCOPE: i32 = 2;
    const SCOPE_COUNT: i32 = 2;
    assert_eq!(number_notifications.get(), IMMEDIATE_CALLBACK_COUNT + (NOTIFICATIONS_PER_SCOPE * SCOPE_COUNT));
}

#[test]
fn modern_dependency_resolver_unregister_all_with_values_works() {
    let _scope = AppLocatorScope::new();
    let current_mutable = ModernDependencyResolver::new();

    let dummy1 = Rc::new(DummyObjectClass1);
    let dummy2 = Rc::new(DummyObjectClass2);
    let dummy3 = Rc::new(DummyObjectClass3);

    let test_contracts = ["", "test"];

    for test_contract in test_contracts {
        current_mutable.register_constant_with_contract::<dyn IDummyInterface>(dummy1.clone(), Some(test_contract));
        current_mutable.register_constant_with_contract::<dyn IDummyInterface>(dummy2.clone(), Some(test_contract));
        current_mutable.register_constant_with_contract::<dyn IDummyInterface>(dummy3.clone(), Some(test_contract));
    }

    for test_contract in test_contracts {
        let items = current_mutable.get_services_with_contract::<dyn IDummyInterface>(Some(test_contract));

        assert_eq!(items.len(), REGISTERED_DUMMY_COUNT);
        assert!(items.iter().any(|item| same(item, &dummy1)));
        assert!(items.iter().any(|item| same(item, &dummy2)));
        assert!(items.iter().any(|item| same(item, &dummy3)));

        current_mutable.unregister_all_with_contract::<dyn IDummyInterface>(Some(test_contract));

        let items = current_mutable.get_services_with_contract::<dyn IDummyInterface>(Some(test_contract));

        assert!(items.is_empty());
    }

    current_mutable.dispose();
}

#[test]
fn register_and_resolve_a_nullable_type_with_value() {
    let _scope = AppLocatorScope::new();
    Locator::current_mutable().register::<DummyObjectClass1>(|| Some(Rc::new(DummyObjectClass1)));
    let doc = Locator::current().get_service::<DummyObjectClass1>();
    assert!(doc.is_some());
}

#[test]
fn register_and_resolve_a_nullable_type_with_null() {
    let _scope = AppLocatorScope::new();
    Locator::current_mutable().register::<DummyObjectClass1>(|| None);
    let doc = Locator::current().get_service::<DummyObjectClass1>();
    assert!(doc.is_none());
}

#[test]
fn register_and_resolve_a_nullable_type_with_value_locator_disposed() {
    let _scope = AppLocatorScope::new();
    let current_mutable = ModernDependencyResolver::new();
    current_mutable.register::<DummyObjectClass1>(|| Some(Rc::new(DummyObjectClass1)));
    current_mutable.dispose();

    assert!(panics(|| {
        let _ = current_mutable.get_service::<DummyObjectClass1>();
    }));
}

#[test]
fn register_and_resolve_a_nullable_type_with_default() {
    let _scope = AppLocatorScope::new();
    Locator::current_mutable().register::<DummyObjectClass1>(Option::default);
    let doc = Locator::current().get_service::<DummyObjectClass1>();
    assert!(doc.is_none());
}

#[test]
fn register_and_resolve_a_nullable_type_with_nulled_instance() {
    let _scope = AppLocatorScope::new();
    let dummy: Option<Rc<DummyObjectClass1>> = None;
    Locator::current_mutable().register::<DummyObjectClass1>(move || dummy.clone());
    let doc = Locator::current().get_service::<DummyObjectClass1>();
    assert!(doc.is_none());
}

#[test]
fn modern_dependency_resolver_unregister_all_no_values_works() {
    let _scope = AppLocatorScope::new();
    let current_mutable = ModernDependencyResolver::new();

    let items = current_mutable.get_services::<dyn IDummyInterface>();

    assert!(items.is_empty());

    current_mutable.unregister_all::<dyn IDummyInterface>();

    let items = current_mutable.get_services::<dyn IDummyInterface>();

    assert!(items.is_empty());
    current_mutable.dispose();
}

#[test]
fn modern_dependency_resolver_unregister_current_with_values_works() {
    let _scope = AppLocatorScope::new();
    let dummy1 = Rc::new(DummyObjectClass1);
    let dummy2 = Rc::new(DummyObjectClass2);
    let dummy3 = Rc::new(DummyObjectClass3);

    let current_mutable = ModernDependencyResolver::new();

    let test_contracts = ["", "test"];

    for test_contract in test_contracts {
        current_mutable.register_constant_with_contract::<dyn IDummyInterface>(dummy1.clone(), Some(test_contract));
        current_mutable.register_constant_with_contract::<dyn IDummyInterface>(dummy2.clone(), Some(test_contract));
        current_mutable.register_constant_with_contract::<dyn IDummyInterface>(dummy3.clone(), Some(test_contract));
    }

    for test_contract in test_contracts {
        let items = current_mutable.get_services_with_contract::<dyn IDummyInterface>(Some(test_contract));

        assert_eq!(items.len(), REGISTERED_DUMMY_COUNT);
        assert!(items.iter().any(|item| same(item, &dummy1)));
        assert!(items.iter().any(|item| same(item, &dummy2)));
        assert!(items.iter().any(|item| same(item, &dummy3)));

        current_mutable.unregister_current_with_contract::<dyn IDummyInterface>(Some(test_contract));

        let items = current_mutable.get_services_with_contract::<dyn IDummyInterface>(Some(test_contract));

        assert_eq!(items.len(), REMAINING_DUMMY_COUNT);
        assert!(items.iter().any(|item| same(item, &dummy1)));
        assert!(items.iter().any(|item| same(item, &dummy2)));
    }

    current_mutable.dispose();
}

#[test]
fn modern_dependency_resolver_unregister_current_no_values_works() {
    let _scope = AppLocatorScope::new();
    let current_mutable = ModernDependencyResolver::new();
    let items = current_mutable.get_services::<dyn IDummyInterface>();

    assert!(items.is_empty());

    current_mutable.unregister_current::<dyn IDummyInterface>();

    let items = current_mutable.get_services::<dyn IDummyInterface>();

    assert!(items.is_empty());
    current_mutable.dispose();
}

// Not ported: Can_Register_And_Resolve_Null_Types - registrations with a null service type (the Type-based API and NullServiceType), not ported
// Not ported: FuncDependencyResolver_UnregisterAll - FuncDependencyResolver is not in ferroui-splat, and the `Type` its delegates receive is of the Type-based API
// Not ported: FuncDependencyResolver_UnregisterCurrent - FuncDependencyResolver is not in ferroui-splat, and the `Type` its delegates receive is of the Type-based API
