//! Ported from `Splat.Tests/LocatorSerialRegisterTests.cs` of Splat.
//! Adapted: the `[Before(Test)]`/`[After(Test)]` scope is an `InternalLocatorScope` at the start of
//! each test. The `Type`-based `GetService(typeof(ILogManager))` is the generic
//! `get_service::<dyn ILogManager>` (the type only names the service). `IsTypeOf<DebugLogger>` and
//! `IsTypeOf<DefaultLogManager>` have no form on `Rc<dyn ILogger>` and `Rc<dyn ILogManager>`, so
//! those tests check that the services are there. `RegisterConstantAnd<T>(value, contract)` is
//! `register_constant_and_value_with_contract`; the `RegisterAnd<IDummyInterface,
//! DummyObjectClass1>(contract)` and `Register<IDummyInterface, DummyObjectClass3>(contract)` of
//! `ModernDependencyResolver_UnregisterCurrent_WithValuesWorks` are factory closures that create the
//! implementation (`RegisterAnd<TService, TImplementation>` and `Register<TService,
//! TImplementation>` are not ported). `using var` is an explicit `dispose`, and `Contains` of a
//! list of services is the identity of the `Rc`.

use crate::tests::common::{
    same, DummyObjectClass1, DummyObjectClass2, DummyObjectClass3, IDummyInterface,
    InternalLocatorScope,
};
use ferroui_splat::service_location::InternalLocator;
use ferroui_splat::{
    DependencyResolverMixins, ILogManager, ILogger, Locator, ModernDependencyResolver,
    MutableDependencyResolverExt, ReadonlyDependencyResolverExt, ResolverMixins,
    ServiceLocationInitializationExt,
};
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

const REGISTERED_DUMMY_COUNT: usize = 3;

const REMAINING_DUMMY_COUNT: usize = 2;

#[test]
fn initialize_splat_registrations_not_empty_no_registrations() {
    let _scope = InternalLocatorScope::new();
    let test_locator = InternalLocator::new();
    test_locator.current_mutable().initialize_splat();
    let log_manager = test_locator.current().get_service::<dyn ILogManager>();
    let logger = test_locator.current().get_service::<dyn ILogger>();

    assert!(log_manager.is_some());
    assert!(logger.is_some());
    test_locator.dispose();
}

#[test]
fn initialize_splat_contract_registrations_null_no_registration() {
    let _scope = InternalLocatorScope::new();
    let test_locator = InternalLocator::new();
    let log_manager = test_locator.current().get_service_with_contract::<dyn ILogManager>(Some("test"));
    let logger = test_locator.current().get_service_with_contract::<dyn ILogger>(Some("test"));

    assert!(log_manager.is_none());
    assert!(logger.is_none());
    test_locator.dispose();
}

#[test]
fn initialize_splat_extension_methods_not_null() {
    let _scope = InternalLocatorScope::new();
    let test_locator = InternalLocator::new();
    test_locator.current_mutable().initialize_splat();
    let log_manager = test_locator.current().get_service::<dyn ILogManager>();
    let logger = test_locator.current().get_service::<dyn ILogger>();

    assert!(log_manager.is_some());
    assert!(logger.is_some());
    test_locator.dispose();
}

#[test]
fn without_suppress_notifications_happen() {
    let _scope = InternalLocatorScope::new();
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
    let _scope = InternalLocatorScope::new();
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
    let _scope = InternalLocatorScope::new();
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
    let _scope = InternalLocatorScope::new();
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
    let _scope = InternalLocatorScope::new();
    let current_mutable = ModernDependencyResolver::new();

    let dummy1 = Rc::new(DummyObjectClass1);
    let dummy2 = Rc::new(DummyObjectClass2);
    let dummy3 = Rc::new(DummyObjectClass3);

    let test_contracts = ["", "test"];

    for test_contract in test_contracts {
        current_mutable
            .register_constant_and_value_with_contract::<dyn IDummyInterface>(dummy1.clone(), test_contract)
            .register_constant_and_value_with_contract::<dyn IDummyInterface>(dummy2.clone(), test_contract)
            .register_constant_with_contract::<dyn IDummyInterface>(dummy3.clone(), Some(test_contract));
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
fn modern_dependency_resolver_unregister_all_no_values_works() {
    let _scope = InternalLocatorScope::new();
    let current_mutable = ModernDependencyResolver::new();

    let items = current_mutable.get_services::<dyn IDummyInterface>();

    assert!(items.is_empty());

    current_mutable.unregister_all::<dyn IDummyInterface>();

    let items = current_mutable.get_services::<dyn IDummyInterface>();

    assert!(items.is_empty());
    current_mutable.dispose();
}

#[test]
fn modern_dependency_resolver_constant_unregister_current_with_values_works() {
    let _scope = InternalLocatorScope::new();
    let dummy1 = Rc::new(DummyObjectClass1);
    let dummy2 = Rc::new(DummyObjectClass2);
    let dummy3 = Rc::new(DummyObjectClass3);

    let current_mutable = ModernDependencyResolver::new();

    let test_contracts = ["", "test"];

    for test_contract in test_contracts {
        current_mutable
            .register_constant_and_value_with_contract::<dyn IDummyInterface>(dummy1.clone(), test_contract)
            .register_constant_and_value_with_contract::<dyn IDummyInterface>(dummy2.clone(), test_contract)
            .register_constant_with_contract::<dyn IDummyInterface>(dummy3.clone(), Some(test_contract));
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
fn modern_dependency_resolver_unregister_current_with_values_works() {
    let _scope = InternalLocatorScope::new();
    let current_mutable = ModernDependencyResolver::new();

    let test_contracts = ["", "test"];

    for test_contract in test_contracts {
        current_mutable
            .register_and_factory_with_contract::<dyn IDummyInterface>(
                || Rc::new(DummyObjectClass1) as Rc<dyn IDummyInterface>,
                test_contract,
            )
            .register_and_factory_with_contract::<dyn IDummyInterface>(
                || Rc::new(DummyObjectClass2) as Rc<dyn IDummyInterface>,
                test_contract,
            )
            .register_with_contract::<dyn IDummyInterface>(
                || Some(Rc::new(DummyObjectClass3) as Rc<dyn IDummyInterface>),
                Some(test_contract),
            );
    }

    for test_contract in test_contracts {
        let items = current_mutable.get_services_with_contract::<dyn IDummyInterface>(Some(test_contract));

        assert_eq!(items.len(), REGISTERED_DUMMY_COUNT);

        current_mutable.unregister_current_with_contract::<dyn IDummyInterface>(Some(test_contract));

        let items = current_mutable.get_services_with_contract::<dyn IDummyInterface>(Some(test_contract));

        assert_eq!(items.len(), REMAINING_DUMMY_COUNT);
    }

    current_mutable.dispose();
}

#[test]
fn modern_dependency_resolver_unregister_current_no_values_works() {
    let _scope = InternalLocatorScope::new();
    let current_mutable = ModernDependencyResolver::new();
    let items = current_mutable.get_services::<dyn IDummyInterface>();

    assert!(items.is_empty());

    current_mutable.unregister_current::<dyn IDummyInterface>();

    let items = current_mutable.get_services::<dyn IDummyInterface>();

    assert!(items.is_empty());
    current_mutable.dispose();
}

// Not ported: FuncDependencyResolver_UnregisterAll - FuncDependencyResolver is not in ferroui-splat, and the `Type` its delegates receive is of the Type-based API
// Not ported: FuncDependencyResolver_UnregisterCurrent - FuncDependencyResolver is not in ferroui-splat, and the `Type` its delegates receive is of the Type-based API
