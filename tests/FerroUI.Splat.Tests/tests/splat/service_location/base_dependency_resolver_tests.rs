//! Ported from `Splat.Tests/ServiceLocation/BaseDependencyResolverTests.cs` of Splat.
//! Adapted: the abstract generic test class is the macro `base_dependency_resolver_tests!`,
//! which takes the expression that makes the resolver under test (`GetDependencyResolver()`) and
//! expands to a module `base_dependency_resolver_tests` with the tests, so that each resolver's
//! test file instantiates them with one line. The `[Before(Test)]`/`[After(Test)]` scope is an
//! `AppLocatorScope` made at the start of each test. None of the tests is overridden by a
//! resolver's tests file (the `FuncDependencyResolver` override is not ported), so the macro has no
//! arm that leaves tests out. The service types named by `typeof(ILogManager)` and
//! `typeof(string)` are the generic `dyn ILogManager` and `String`; `IsTypeOf<FuncLogManager>` and
//! `IsTypeOf<DefaultLogManager>` compare with the instances the factories return
//! (`IsSameReferenceAs`); `Register<IViewModelOne, ViewModelOne>()` is a factory of a
//! `ViewModelOne` coerced to `Rc<dyn IViewModelOne>`, and its `IsTypeOf<ViewModelOne>` compares
//! with that `ViewModelOne`. `RegisterAndTests` leaves out its `ArgumentNullException` assertion,
//! and its `RegisterAnd<IViewModelOne, ViewModelOne>`/`Type`-based links are the factory forms.
//! A `using var` subscription is disposed at the end of the test. The disposable test services
//! are registered with `register_constant_disposable` / `register_lazy_singleton_disposable`, the
//! forms whose values the resolver disposes (upstream disposes any `IDisposable` constant).
//! A thrown exception is a panic: `Throws<ObjectDisposedException>` is `panics`, `ThrowsNothing`
//! is `!panics`.
//!
//! Of the 73 tests of the base, 52 are ported (in the macro) and 21 are not (listed at the end of
//! this file).

use ferroui_system::IDisposable;
use std::cell::Cell;

/// A service that records its disposal (`DisposableTestService`).
#[derive(Default)]
pub(crate) struct DisposableTestService {
    is_disposed: Cell<bool>,
}

impl DisposableTestService {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// `IsDisposed`.
    pub(crate) fn is_disposed(&self) -> bool {
        self.is_disposed.get()
    }
}

impl IDisposable for DisposableTestService {
    fn dispose(&self) {
        self.is_disposed.set(true);
    }
}

/// A service whose disposal panics (`ThrowingDisposableService`).
pub(crate) struct ThrowingDisposableService;

impl IDisposable for ThrowingDisposableService {
    fn dispose(&self) {
        panic!("Disposal exception");
    }
}

/// The tests of `BaseDependencyResolverTests<T>`, run on the resolver `$make` makes (an `Rc` of
/// the resolver, made anew for each test).
macro_rules! base_dependency_resolver_tests {
    ($make:expr) => {
        /// The tests inherited from `BaseDependencyResolverTests<T>`.
        mod base_dependency_resolver_tests {
            #[allow(unused_imports)]
            use super::*;
            use $crate::tests::common::{panics, same, AppLocatorScope, IViewModelOne, ViewModelOne};
            use $crate::tests::splat::service_location::base_dependency_resolver_tests::{
                DisposableTestService, ThrowingDisposableService,
            };
            use ::ferroui_splat::{
                AppLocator, DebugLogger, DefaultLogManager, FuncLogManager, IFullLogger, ILogManager,
                MutableDependencyResolverExt, ReadonlyDependencyResolverExt, ResolverMixins,
                WrappingFullLogger,
            };
            use ::ferroui_system::IDisposable;
            use ::std::cell::Cell;
            use ::std::rc::Rc;

            /// The contract used by tests that register a named service.
            const NAMED_CONTRACT: &str = "named";

            /// Contract name reused across the registration-chain tests.
            const SEVEN_CONTRACT: &str = "seven";

            /// The minimum number of registrations expected when a service is registered more
            /// than once.
            const EXPECTED_MULTIPLE_REGISTRATION_COUNT: usize = 2;

            /// The minimum number of registration callback invocations expected.
            const EXPECTED_CALLBACK_INVOCATION_COUNT: usize = 3;

            /// A new `DefaultLogManager` over the current locator, as a `dyn ILogManager`.
            fn default_log_manager() -> Option<Rc<dyn ILogManager>> {
                Some(Rc::new(DefaultLogManager::new(AppLocator::current())) as Rc<dyn ILogManager>)
            }

            /// A new `ViewModelOne`.
            fn view_model_one() -> Option<Rc<ViewModelOne>> {
                Some(Rc::new(ViewModelOne))
            }

            #[test]
            fn unregister_current_doesnt_throw_when_list_empty() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;

                resolver.register::<dyn ILogManager>(default_log_manager);
                resolver.register_with_contract::<dyn ILogManager>(default_log_manager, Some(NAMED_CONTRACT));

                assert!(!panics(|| {
                    resolver.unregister_current::<dyn ILogManager>();
                    resolver.unregister_current::<dyn ILogManager>();
                }));
            }

            #[test]
            fn unregister_current_remove_last() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let default_manager: Rc<dyn ILogManager> =
                    Rc::new(DefaultLogManager::new(AppLocator::current()));
                let func_manager: Rc<dyn ILogManager> = Rc::new(FuncLogManager::new(|_| {
                    Rc::new(WrappingFullLogger::new(Rc::new(DebugLogger::new()))) as Rc<dyn IFullLogger>
                }));

                let registered = default_manager.clone();
                resolver.register::<dyn ILogManager>(move || Some(registered.clone()));
                let registered = func_manager.clone();
                resolver.register::<dyn ILogManager>(move || Some(registered.clone()));
                resolver.register_with_contract::<dyn ILogManager>(default_log_manager, Some(NAMED_CONTRACT));

                let service = resolver.get_service::<dyn ILogManager>();
                assert!(service.is_some_and(|service| same(&service, &func_manager)));

                resolver.unregister_current::<dyn ILogManager>();

                let service = resolver.get_service::<dyn ILogManager>();
                assert!(service.is_some_and(|service| same(&service, &default_manager)));
            }

            #[test]
            fn unregister_current_by_name_doesnt_throw_when_list_empty() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let contract = NAMED_CONTRACT;

                resolver.register::<dyn ILogManager>(default_log_manager);
                resolver.register_with_contract::<dyn ILogManager>(default_log_manager, Some(contract));

                assert!(!panics(|| {
                    resolver.unregister_current_with_contract::<dyn ILogManager>(Some(contract));
                    resolver.unregister_current_with_contract::<dyn ILogManager>(Some(contract));
                }));
            }

            #[test]
            fn unregister_all_unregister_current_doesnt_throw_when_list_empty() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;

                resolver.register::<dyn ILogManager>(default_log_manager);
                resolver.register_with_contract::<dyn ILogManager>(default_log_manager, Some(NAMED_CONTRACT));

                assert!(!panics(|| {
                    resolver.unregister_all::<dyn ILogManager>();
                    resolver.unregister_current::<dyn ILogManager>();
                }));
            }

            #[test]
            fn unregister_all_by_contract_unregister_current_doesnt_throw_when_list_empty() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let contract = NAMED_CONTRACT;

                resolver.register::<dyn ILogManager>(default_log_manager);
                resolver.register_with_contract::<dyn ILogManager>(default_log_manager, Some(contract));

                assert!(!panics(|| {
                    resolver.unregister_all_with_contract::<dyn ILogManager>(Some(contract));
                    resolver.unregister_current_with_contract::<dyn ILogManager>(Some(contract));
                }));
            }

            #[test]
            fn get_services_should_never_return_null() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;

                // A `Vec` is never null; nothing is registered, so both lists are empty.
                assert!(resolver.get_services::<String>().is_empty());
                assert!(resolver.get_services_with_contract::<String>(Some("Landscape")).is_empty());
            }

            #[test]
            fn has_registration() {
                const CONTRACT_ONE: &str = "ContractOne";
                const CONTRACT_TWO: &str = "ContractTwo";
                let _scope = AppLocatorScope::new();
                let resolver = $make;

                assert!(!resolver.has_registration::<String>());
                assert!(!resolver.has_registration_with_contract::<String>(Some(CONTRACT_ONE)));
                assert!(!resolver.has_registration_with_contract::<String>(Some(CONTRACT_TWO)));

                resolver.register::<String>(|| Some(Rc::new("unnamed".to_owned())));
                assert!(resolver.has_registration::<String>());
                assert!(!resolver.has_registration_with_contract::<String>(Some(CONTRACT_ONE)));
                assert!(!resolver.has_registration_with_contract::<String>(Some(CONTRACT_TWO)));

                resolver.unregister_all::<String>();

                resolver.register_with_contract::<String>(|| Some(Rc::new(CONTRACT_ONE.to_owned())), Some(CONTRACT_ONE));
                assert!(!resolver.has_registration::<String>());
                assert!(resolver.has_registration_with_contract::<String>(Some(CONTRACT_ONE)));
                assert!(!resolver.has_registration_with_contract::<String>(Some(CONTRACT_TWO)));

                resolver.unregister_all_with_contract::<String>(Some(CONTRACT_ONE));

                resolver.register_with_contract::<String>(|| Some(Rc::new(CONTRACT_TWO.to_owned())), Some(CONTRACT_TWO));
                assert!(!resolver.has_registration::<String>());
                assert!(!resolver.has_registration_with_contract::<String>(Some(CONTRACT_ONE)));
                assert!(resolver.has_registration_with_contract::<String>(Some(CONTRACT_TWO)));
            }

            #[test]
            fn register_and_tests() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;

                resolver
                    .register_and_with_contract::<ViewModelOne>("one")
                    .register_and_factory_with_contract::<dyn IViewModelOne>(
                        || Rc::new(ViewModelOne) as Rc<dyn IViewModelOne>,
                        "two",
                    )
                    .register_and_factory_with_contract::<DefaultLogManager>(
                        || Rc::new(DefaultLogManager::new(AppLocator::current())),
                        "three",
                    )
                    .register_and_factory_with_contract::<dyn IViewModelOne>(
                        || Rc::new(ViewModelOne) as Rc<dyn IViewModelOne>,
                        "four",
                    )
                    .register_constant_and_with_contract::<ViewModelOne>("five")
                    .register_constant_and_value_with_contract::<ViewModelOne>(Rc::new(ViewModelOne), "six")
                    .register_lazy_singleton_and_factory_with_contract::<dyn ILogManager>(
                        || Rc::new(DefaultLogManager::new(AppLocator::current())) as Rc<dyn ILogManager>,
                        SEVEN_CONTRACT,
                    )
                    .register_lazy_singleton_and_with_contract::<ViewModelOne>("eight")
                    .register_lazy_singleton_and_factory_with_contract::<DefaultLogManager>(
                        || Rc::new(DefaultLogManager::new(AppLocator::current())),
                        SEVEN_CONTRACT,
                    )
                    .register::<dyn IViewModelOne>(|| Some(Rc::new(ViewModelOne) as Rc<dyn IViewModelOne>));
            }

            #[test]
            fn get_service_generic_without_contract_returns_service() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                resolver.register_constant(Rc::new(ViewModelOne));

                let result = resolver.get_service::<ViewModelOne>();

                assert!(result.is_some());
            }

            #[test]
            fn get_service_generic_with_contract_returns_service() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let contract = "test";
                resolver.register_constant_with_contract(Rc::new(ViewModelOne), Some(contract));

                let result = resolver.get_service_with_contract::<ViewModelOne>(Some(contract));

                assert!(result.is_some());
            }

            #[test]
            fn get_service_generic_unregistered_returns_null() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;

                let result = resolver.get_service::<ViewModelOne>();

                assert!(result.is_none());
            }

            #[test]
            fn get_services_generic_returns_all_services() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                resolver.register::<ViewModelOne>(view_model_one);
                resolver.register::<ViewModelOne>(view_model_one);

                let results = resolver.get_services::<ViewModelOne>();

                assert!(results.len() >= EXPECTED_MULTIPLE_REGISTRATION_COUNT);
            }

            #[test]
            fn get_services_generic_with_contract_returns_contract_services() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let contract = "test";
                resolver.register::<ViewModelOne>(view_model_one);
                resolver.register_with_contract::<ViewModelOne>(view_model_one, Some(contract));

                let results = resolver.get_services_with_contract::<ViewModelOne>(Some(contract));

                assert!(!results.is_empty());
            }

            #[test]
            fn has_registration_generic_without_contract_returns_true_when_registered() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                resolver.register_constant(Rc::new(ViewModelOne));

                let result = resolver.has_registration::<ViewModelOne>();

                assert!(result);
            }

            #[test]
            fn has_registration_generic_with_contract_returns_true_when_registered() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let contract = "test";
                resolver.register_constant_with_contract(Rc::new(ViewModelOne), Some(contract));

                let result = resolver.has_registration_with_contract::<ViewModelOne>(Some(contract));

                assert!(result);
            }

            #[test]
            fn has_registration_generic_not_registered_returns_false() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;

                let result = resolver.has_registration::<ViewModelOne>();

                assert!(!result);
            }

            #[test]
            fn register_generic_with_factory_registers_service() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                resolver.register::<ViewModelOne>(view_model_one);

                let result = resolver.get_service::<ViewModelOne>();

                assert!(result.is_some());
            }

            #[test]
            fn register_generic_with_factory_and_contract_registers_service() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let contract = "test";
                resolver.register_with_contract::<ViewModelOne>(view_model_one, Some(contract));

                let result = resolver.get_service_with_contract::<ViewModelOne>(Some(contract));

                assert!(result.is_some());
            }

            #[test]
            fn register_generic_with_service_and_implementation_registers_service() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let implementation = Rc::new(ViewModelOne);
                let registered = implementation.clone();
                resolver.register::<dyn IViewModelOne>(move || Some(registered.clone() as Rc<dyn IViewModelOne>));

                let result = resolver.get_service::<dyn IViewModelOne>();

                assert!(result.is_some());
                assert!(result.is_some_and(|result| same(&result, &implementation)));
            }

            #[test]
            fn register_generic_with_service_implementation_and_contract_registers_service() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let contract = "test";
                let implementation = Rc::new(ViewModelOne);
                let registered = implementation.clone();
                resolver.register_with_contract::<dyn IViewModelOne>(
                    move || Some(registered.clone() as Rc<dyn IViewModelOne>),
                    Some(contract),
                );

                let result = resolver.get_service_with_contract::<dyn IViewModelOne>(Some(contract));

                assert!(result.is_some());
                assert!(result.is_some_and(|result| same(&result, &implementation)));
            }

            #[test]
            fn register_constant_generic_creates_singleton() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let instance = Rc::new(ViewModelOne);
                resolver.register_constant(instance.clone());

                let result1 = resolver.get_service::<ViewModelOne>();
                let result2 = resolver.get_service::<ViewModelOne>();

                assert!(result1.is_some_and(|result| same(&result, &instance)));
                assert!(result2.is_some_and(|result| same(&result, &instance)));
            }

            #[test]
            fn register_constant_generic_with_contract_creates_singleton() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let contract = "test";
                let instance = Rc::new(ViewModelOne);
                resolver.register_constant_with_contract(instance.clone(), Some(contract));

                let result = resolver.get_service_with_contract::<ViewModelOne>(Some(contract));

                assert!(result.is_some_and(|result| same(&result, &instance)));
            }

            #[test]
            fn register_lazy_singleton_generic_creates_singleton_on_first_access() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let call_count = Rc::new(Cell::new(0));

                let counter = call_count.clone();
                resolver.register_lazy_singleton::<ViewModelOne>(move || {
                    counter.set(counter.get() + 1);
                    Some(Rc::new(ViewModelOne))
                });

                assert_eq!(call_count.get(), 0); // Not called yet

                let result1 = resolver.get_service::<ViewModelOne>();
                assert_eq!(call_count.get(), 1); // Called once
                assert!(result1.is_some());

                let result2 = resolver.get_service::<ViewModelOne>();
                assert_eq!(call_count.get(), 1); // Not called again

                assert!(matches!((&result1, &result2), (Some(a), Some(b)) if same(a, b)));
            }

            #[test]
            fn register_lazy_singleton_generic_with_contract_creates_singleton() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let contract = "test";

                resolver.register_lazy_singleton_with_contract::<ViewModelOne>(view_model_one, Some(contract));

                let result1 = resolver.get_service_with_contract::<ViewModelOne>(Some(contract));
                let result2 = resolver.get_service_with_contract::<ViewModelOne>(Some(contract));

                assert!(result1.is_some());
                assert!(matches!((&result1, &result2), (Some(a), Some(b)) if same(a, b)));
            }

            #[test]
            fn unregister_current_generic_removes_last_registration() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let instance1 = Rc::new(ViewModelOne);
                let instance2 = Rc::new(ViewModelOne);

                resolver.register_constant(instance1.clone());
                resolver.register_constant(instance2.clone());

                let before_unregister = resolver.get_service::<ViewModelOne>();
                assert!(before_unregister.is_some_and(|service| same(&service, &instance2)));

                resolver.unregister_current::<ViewModelOne>();

                let after_unregister = resolver.get_service::<ViewModelOne>();
                assert!(after_unregister.is_some_and(|service| same(&service, &instance1)));
            }

            #[test]
            fn unregister_current_generic_with_contract_removes_registration() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let contract = "test";
                resolver.register_constant_with_contract(Rc::new(ViewModelOne), Some(contract));

                assert!(resolver.has_registration_with_contract::<ViewModelOne>(Some(contract)));

                resolver.unregister_current_with_contract::<ViewModelOne>(Some(contract));

                assert!(!resolver.has_registration_with_contract::<ViewModelOne>(Some(contract)));
            }

            #[test]
            fn unregister_all_generic_removes_all_registrations() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                resolver.register_constant(Rc::new(ViewModelOne));
                resolver.register::<ViewModelOne>(view_model_one);

                assert!(resolver.has_registration::<ViewModelOne>());

                resolver.unregister_all::<ViewModelOne>();

                assert!(!resolver.has_registration::<ViewModelOne>());
            }

            #[test]
            fn unregister_all_generic_with_contract_removes_all_contract_registrations() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let contract = "test";
                resolver.register_constant_with_contract(Rc::new(ViewModelOne), Some(contract));
                resolver.register_with_contract::<ViewModelOne>(view_model_one, Some(contract));

                assert!(resolver.has_registration_with_contract::<ViewModelOne>(Some(contract)));

                resolver.unregister_all_with_contract::<ViewModelOne>(Some(contract));

                assert!(!resolver.has_registration_with_contract::<ViewModelOne>(Some(contract)));
            }

            #[test]
            fn service_registration_callback_generic_invoked_when_service_registered() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let callback_invoked = Rc::new(Cell::new(false));

                let invoked = callback_invoked.clone();
                let subscription = resolver.service_registration_callback::<ViewModelOne>(move |_| invoked.set(true));

                assert!(!callback_invoked.get());

                resolver.register::<ViewModelOne>(view_model_one);

                assert!(callback_invoked.get());
                subscription.dispose();
            }

            #[test]
            fn service_registration_callback_generic_with_existing_registration_invokes_immediately() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                resolver.register::<ViewModelOne>(view_model_one);

                let callback_invoked = Rc::new(Cell::new(false));

                let invoked = callback_invoked.clone();
                let subscription = resolver.service_registration_callback::<ViewModelOne>(move |_| invoked.set(true));

                assert!(callback_invoked.get());
                subscription.dispose();
            }

            #[test]
            fn service_registration_callback_generic_with_contract_invoked_when_service_registered() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let contract = "test";
                let callback_invoked = Rc::new(Cell::new(false));

                let invoked = callback_invoked.clone();
                let subscription = resolver
                    .service_registration_callback_with_contract::<ViewModelOne>(Some(contract), move |_| invoked.set(true));

                assert!(!callback_invoked.get());

                resolver.register_with_contract::<ViewModelOne>(view_model_one, Some(contract));

                assert!(callback_invoked.get());
                subscription.dispose();
            }

            #[test]
            fn service_registration_callback_disposal_stops_receiving_notifications() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let callback_count = Rc::new(Cell::new(0));

                let counter = callback_count.clone();
                let subscription =
                    resolver.service_registration_callback::<ViewModelOne>(move |_| counter.set(counter.get() + 1));

                resolver.register::<ViewModelOne>(view_model_one);
                assert_eq!(callback_count.get(), 1);

                subscription.dispose();

                resolver.register::<ViewModelOne>(view_model_one);
                assert_eq!(callback_count.get(), 1); // Should not increment
            }

            #[test]
            fn dispose_double_dispose_does_not_throw() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;

                assert!(!panics(|| {
                    resolver.dispose();
                    resolver.dispose();
                }));
            }

            #[test]
            fn dispose_disposes_registered_services() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let disposable_service = Rc::new(DisposableTestService::new());

                resolver.register_constant_disposable(disposable_service.clone());

                assert!(!disposable_service.is_disposed());

                resolver.dispose();

                assert!(disposable_service.is_disposed());
            }

            #[test]
            fn dispose_with_lazy_singleton_does_not_create_if_not_accessed() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let factory_called = Rc::new(Cell::new(false));

                let called = factory_called.clone();
                resolver.register_lazy_singleton_disposable::<DisposableTestService>(move || {
                    called.set(true);
                    Some(Rc::new(DisposableTestService::new()))
                });

                assert!(!factory_called.get());

                resolver.dispose();

                // Factory should not be called during disposal if lazy value was never accessed
                assert!(!factory_called.get());
            }

            #[test]
            fn dispose_invokes_callbacks() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let callback_invoked = Rc::new(Cell::new(false));

                let invoked = callback_invoked.clone();
                let _ = resolver.service_registration_callback::<ViewModelOne>(move |_| invoked.set(true));

                resolver.dispose();

                assert!(callback_invoked.get());
            }

            #[test]
            fn get_service_generic_with_null_contract_delegates_to_non_contract() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let instance = Rc::new(ViewModelOne);
                resolver.register_constant(instance.clone());

                let result = resolver.get_service_with_contract::<ViewModelOne>(None);

                assert!(result.is_some_and(|result| same(&result, &instance)));
            }

            #[test]
            fn get_services_generic_with_null_contract_delegates_to_non_contract() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                resolver.register::<ViewModelOne>(view_model_one);

                let results = resolver.get_services_with_contract::<ViewModelOne>(None);

                assert!(!results.is_empty());
            }

            #[test]
            fn has_registration_generic_with_null_contract_delegates_to_non_contract() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                resolver.register_constant(Rc::new(ViewModelOne));

                let result = resolver.has_registration_with_contract::<ViewModelOne>(None);

                assert!(result);
            }

            #[test]
            fn has_registration_generic_with_contract_returns_false_when_empty() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;

                let result = resolver.has_registration_with_contract::<ViewModelOne>(Some("test"));

                assert!(!result);
            }

            #[test]
            fn register_generic_with_null_contract_delegates_to_non_contract() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                resolver.register_with_contract::<ViewModelOne>(view_model_one, None);

                let result = resolver.get_service::<ViewModelOne>();

                assert!(result.is_some());
            }

            #[test]
            fn register_generic_with_service_implementation_and_null_contract_delegates_to_non_contract() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let implementation = Rc::new(ViewModelOne);
                let registered = implementation.clone();
                resolver.register_with_contract::<dyn IViewModelOne>(
                    move || Some(registered.clone() as Rc<dyn IViewModelOne>),
                    None,
                );

                let result = resolver.get_service::<dyn IViewModelOne>();

                assert!(result.is_some());
                assert!(result.is_some_and(|result| same(&result, &implementation)));
            }

            #[test]
            fn register_constant_generic_with_null_contract_delegates_to_non_contract() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let instance = Rc::new(ViewModelOne);
                resolver.register_constant_with_contract(instance.clone(), None);

                let result = resolver.get_service::<ViewModelOne>();

                assert!(result.is_some_and(|result| same(&result, &instance)));
            }

            #[test]
            fn register_lazy_singleton_generic_with_null_contract_delegates_to_non_contract() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                resolver.register_lazy_singleton_with_contract::<ViewModelOne>(view_model_one, None);

                let result1 = resolver.get_service::<ViewModelOne>();
                let result2 = resolver.get_service::<ViewModelOne>();

                assert!(result1.is_some());
                assert!(matches!((&result1, &result2), (Some(a), Some(b)) if same(a, b)));
            }

            #[test]
            fn unregister_current_generic_with_null_contract_delegates_to_non_contract() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let instance1 = Rc::new(ViewModelOne);
                let instance2 = Rc::new(ViewModelOne);

                resolver.register_constant(instance1.clone());
                resolver.register_constant(instance2.clone());

                resolver.unregister_current_with_contract::<ViewModelOne>(None);

                let result = resolver.get_service::<ViewModelOne>();
                assert!(result.is_some_and(|result| same(&result, &instance1)));
            }

            #[test]
            fn unregister_all_generic_with_null_contract_delegates_to_non_contract() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                resolver.register_constant(Rc::new(ViewModelOne));
                resolver.register::<ViewModelOne>(view_model_one);

                resolver.unregister_all_with_contract::<ViewModelOne>(None);

                assert!(!resolver.has_registration::<ViewModelOne>());
            }

            #[test]
            fn service_registration_callback_generic_invokes_for_each_existing_registration() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                resolver.register::<ViewModelOne>(view_model_one);
                resolver.register::<ViewModelOne>(view_model_one);
                resolver.register::<ViewModelOne>(view_model_one);

                let callback_count = Rc::new(Cell::new(0_usize));

                let counter = callback_count.clone();
                let subscription =
                    resolver.service_registration_callback::<ViewModelOne>(move |_| counter.set(counter.get() + 1));

                assert!(callback_count.get() >= EXPECTED_CALLBACK_INVOCATION_COUNT);
                subscription.dispose();
            }

            #[test]
            fn dispose_suppresses_exceptions_from_callbacks() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;

                let _ = resolver.service_registration_callback::<ViewModelOne>(|_| panic!("Test exception"));

                assert!(!panics(|| resolver.dispose()));
            }

            #[test]
            fn dispose_suppresses_exceptions_from_service_disposal() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let throwing_service = Rc::new(ThrowingDisposableService);

                resolver.register_constant_disposable(throwing_service);

                assert!(!panics(|| resolver.dispose()));
            }

            #[test]
            fn register_after_dispose_does_not_invoke_callbacks() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let callback_count = Rc::new(Cell::new(0));

                let counter = callback_count.clone();
                let _ = resolver.service_registration_callback::<ViewModelOne>(move |_| counter.set(counter.get() + 1));

                resolver.dispose();

                let count_after_dispose = callback_count.get();

                assert!(panics(|| resolver.register::<ViewModelOne>(view_model_one)));

                assert_eq!(callback_count.get(), count_after_dispose);
            }

            #[test]
            fn dispose_with_accessed_lazy_singleton_disposes_value() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let disposable_service = Rc::new(DisposableTestService::new());

                let registered = disposable_service.clone();
                resolver.register_lazy_singleton_disposable::<DisposableTestService>(move || Some(registered.clone()));

                let retrieved = resolver.get_service::<DisposableTestService>();
                assert!(retrieved.is_some_and(|retrieved| same(&retrieved, &disposable_service)));
                assert!(!disposable_service.is_disposed());

                resolver.dispose();

                assert!(disposable_service.is_disposed());
            }

            #[test]
            fn dispose_with_transient_services_does_not_create_instances() {
                let _scope = AppLocatorScope::new();
                let resolver = $make;
                let transient_created_count = Rc::new(Cell::new(0));
                let disposable_created_count = Rc::new(Cell::new(0));

                // Register transient non-disposable services
                let counter = transient_created_count.clone();
                resolver.register::<ViewModelOne>(move || {
                    counter.set(counter.get() + 1);
                    Some(Rc::new(ViewModelOne))
                });

                // Register transient disposable services
                let counter = disposable_created_count.clone();
                resolver.register::<DisposableTestService>(move || {
                    counter.set(counter.get() + 1);
                    Some(Rc::new(DisposableTestService::new()))
                });

                // Get some instances to verify registration works
                let _ = resolver.get_service::<ViewModelOne>();
                let _ = resolver.get_service::<DisposableTestService>();

                assert_eq!(transient_created_count.get(), 1);
                assert_eq!(disposable_created_count.get(), 1);

                // Dispose the resolver - should NOT create any new transient instances
                resolver.dispose();

                // Counts should remain the same - no new instances created during disposal
                assert_eq!(transient_created_count.get(), 1);
                assert_eq!(disposable_created_count.get(), 1);
            }
        }
    };
}

pub(crate) use base_dependency_resolver_tests;

// Not ported: NullResolverTests - argument-null validation (a null resolver), which Rust's types make impossible
// Not ported: ServiceRegistrationCallback_NonGeneric_InvokedWhenServiceRegistered - the `Type`-based (non-generic) API, which is not ported
// Not ported: ServiceRegistrationCallback_NonGeneric_WithContract_InvokedWhenServiceRegistered - the `Type`-based (non-generic) API, which is not ported
// Not ported: ServiceRegistrationCallback_NullCallback_Throws - argument-null validation (a null callback), which Rust's types make impossible
// Not ported: GetService_NonGeneric_WithContract_ReturnsNullWhenEmpty - the `Type`-based (non-generic) API, which is not ported
// Not ported: GetServices_NonGeneric_WithNullContract_DelegatesToNonContract - the `Type`-based (non-generic) API, which is not ported
// Not ported: GetServices_NonGeneric_WithContract_ReturnsEmptyWhenNoRegistrations - the `Type`-based (non-generic) API, which is not ported
// Not ported: HasRegistration_NonGeneric_WithNullContract_DelegatesToNonContract - the `Type`-based (non-generic) API, which is not ported
// Not ported: Register_NonGeneric_WithNullContract_DelegatesToNonContract - the `Type`-based (non-generic) API, which is not ported
// Not ported: UnregisterCurrent_NonGeneric_WithNullContract_DelegatesToNonContract - the `Type`-based (non-generic) API, which is not ported
// Not ported: UnregisterAll_NonGeneric_WithNullContract_DelegatesToNonContract - the `Type`-based (non-generic) API, which is not ported
// Not ported: GetServices_Generic_CombinesGenericAndNonGenericResults - mixes generic and non-generic registrations; the non-generic registry is not ported
// Not ported: GetServices_Generic_WithContract_CombinesGenericAndNonGenericResults - mixes generic and non-generic registrations; the non-generic registry is not ported
// Not ported: GetService_NonGeneric_UnwrapsLazySingleton - the `Type`-based (non-generic) API, which is not ported
// Not ported: GetService_NonGeneric_WithContract_UnwrapsLazySingleton - the `Type`-based (non-generic) API, which is not ported
// Not ported: GetService_NonGeneric_WithNullType_HandlesNullServiceType - the null service type (NullServiceType), which is not ported
// Not ported: GetServices_NonGeneric_WithNullType_HandlesNullServiceType - the null service type (NullServiceType), which is not ported
// Not ported: HasRegistration_NonGeneric_WithNullType_HandlesNullServiceType - the null service type (NullServiceType), which is not ported
// Not ported: UnregisterCurrent_NonGeneric_WithNullType_HandlesNullServiceType - the null service type (NullServiceType), which is not ported
// Not ported: UnregisterAll_NonGeneric_WithNullType_HandlesNullServiceType - the null service type (NullServiceType), which is not ported
// Not ported: Dispose_WhileLazySingletonUnderConstruction_DisposesServiceAndThrowsException - real threads and concurrency (a lazy singleton under construction on another thread)
