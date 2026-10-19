//! `BuilderInstanceMixinsHappyPathTests` (`ReactiveUI.Builder.Tests/Mixins/BuilderInstanceMixinsHappyPathTests.cs`):
//! `WithInstance` of an application ([`ReactiveUIInstanceMixins::with_instance`]) over a mock
//! application whose resolver holds the services, arities 1 to 8.
//!
//! The macro [`with_instance_happy_path_tests!`] writes the two tests of an arity: the action
//! receives the resolved services and the application is returned, and the action is skipped when
//! the application has no resolver. Arities 9 to 16 are in
//! `builder_instance_mixins_happy_path_tests_high_arity`.

use super::super::executors::builder_test;
use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactiveui::*;
use ferroui_splat::service_location::{erase_service, ErasedService, ServiceType};
use ferroui_splat::{IAppInstance, IMutableDependencyResolver, IReadonlyDependencyResolver};
use std::any::TypeId;
use std::collections::HashMap;
use std::rc::Rc;

/// Declares the marker services of the tests (`Service1` to `Service16`).
macro_rules! services {
    ($($service:ident),+) => {
        $(
            #[doc = concat!("`", stringify!($service), "`.")]
            pub struct $service;
        )+
    };
}

services!(
    Service1, Service2, Service3, Service4, Service5, Service6, Service7, Service8, Service9, Service10, Service11, Service12,
    Service13, Service14, Service15, Service16
);

/// A resolver that hands out the services it was given, by their type (`MockDependencyResolver`).
pub struct MockDependencyResolver {
    services: HashMap<TypeId, ErasedService>,
}

impl IReadonlyDependencyResolver for MockDependencyResolver {
    fn get_service_erased(&self, service_type: ServiceType, _contract: Option<&str>) -> Option<ErasedService> {
        self.services.get(&service_type.id()).cloned()
    }

    fn get_services_erased(&self, service_type: ServiceType, contract: Option<&str>) -> Vec<ErasedService> {
        self.get_service_erased(service_type, contract).into_iter().collect()
    }
}

/// A service of the mock resolver: the service, under its type.
pub fn service<S: 'static>(value: &Rc<S>) -> (TypeId, ErasedService) {
    (TypeId::of::<S>(), erase_service(value.clone()))
}

/// An application whose resolver is a [`MockDependencyResolver`], or which has none
/// (`MockReactiveUIInstance`).
pub struct MockReactiveUIInstance {
    resolver: Option<Rc<MockDependencyResolver>>,
}

impl MockReactiveUIInstance {
    /// An application whose resolver holds `services` (`MockReactiveUIInstance(params object[])`).
    pub fn new(services: Vec<(TypeId, ErasedService)>) -> Self {
        Self { resolver: Some(Rc::new(MockDependencyResolver { services: services.into_iter().collect() })) }
    }

    /// An application without a resolver (`MockReactiveUIInstance(bool)` with `true`, the only
    /// form the tests use).
    pub fn with_null_current() -> Self {
        Self { resolver: None }
    }
}

impl IAppInstance for MockReactiveUIInstance {
    fn current(&self) -> Option<Rc<dyn IReadonlyDependencyResolver>> {
        self.resolver.clone().map(|resolver| resolver as Rc<dyn IReadonlyDependencyResolver>)
    }

    fn current_mutable(&self) -> Rc<dyn IMutableDependencyResolver> {
        panic!("NotSupportedException: the mock application has no mutable resolver");
    }
}

impl IReactiveUIInstance for MockReactiveUIInstance {
    fn main_thread_scheduler(&self) -> Option<Rc<dyn IScheduler>> {
        panic!("NotSupportedException: the mock application has no scheduler");
    }

    fn task_pool_scheduler(&self) -> Option<Rc<dyn IScheduler>> {
        panic!("NotSupportedException: the mock application has no scheduler");
    }
}

/// Writes the two tests of `WithInstance` of one arity on a mock application: `$invokes`
/// (`WithInstance_N_Types_Invokes_Action_With_Resolved_Services`) and `$returns_early`
/// (`WithInstance_N_Types_Returns_Early_When_Current_Is_Null`). Each service is given as
/// `Type => value, captured`.
macro_rules! with_instance_happy_path_tests {
    ($invokes:ident, $returns_early:ident; $($service:ident => $value:ident, $captured:ident);+ $(;)?) => {
        #[test]
        fn $invokes() {
            builder_test(|| {
                $( let $value = ::std::rc::Rc::new($service); )+
                let mock_instance = MockReactiveUIInstance::new(vec![$(service(&$value)),+]);
                let captured = ::std::cell::RefCell::new(None);

                let result = mock_instance.with_instance(|services: ($(Option<::std::rc::Rc<$service>>,)+)| {
                    *captured.borrow_mut() = Some(services);
                });

                assert!(::std::ptr::eq(result, &mock_instance));
                let ($($captured,)+) = captured.into_inner().expect("the action runs");
                $(
                    assert!(::std::rc::Rc::ptr_eq(&$captured.expect("the service is resolved"), &$value));
                )+
            });
        }

        #[test]
        fn $returns_early() {
            builder_test(|| {
                let mock_instance = MockReactiveUIInstance::with_null_current();
                let action_invoked = ::std::cell::Cell::new(false);

                let result = mock_instance.with_instance(|_: ($(Option<::std::rc::Rc<$service>>,)+)| action_invoked.set(true));

                assert!(::std::ptr::eq(result, &mock_instance));
                assert!(!action_invoked.get());
            });
        }
    };
}

pub(super) use with_instance_happy_path_tests;

with_instance_happy_path_tests!(with_instance_1_type_invokes_action_with_resolved_services, with_instance_1_type_returns_early_when_current_is_null;
    Service1 => service1, captured1);

// Not ported: WithInstance_1_Type_Handles_Null_Action - argument validation

with_instance_happy_path_tests!(with_instance_2_types_invokes_action_with_resolved_services, with_instance_2_types_returns_early_when_current_is_null;
    Service1 => service1, captured1; Service2 => service2, captured2);

with_instance_happy_path_tests!(with_instance_3_types_invokes_action_with_resolved_services, with_instance_3_types_returns_early_when_current_is_null;
    Service1 => service1, captured1; Service2 => service2, captured2; Service3 => service3, captured3);

with_instance_happy_path_tests!(with_instance_4_types_invokes_action_with_resolved_services, with_instance_4_types_returns_early_when_current_is_null;
    Service1 => service1, captured1; Service2 => service2, captured2; Service3 => service3, captured3; Service4 => service4, captured4);

with_instance_happy_path_tests!(with_instance_5_types_invokes_action_with_resolved_services, with_instance_5_types_returns_early_when_current_is_null;
    Service1 => service1, captured1; Service2 => service2, captured2; Service3 => service3, captured3; Service4 => service4, captured4;
    Service5 => service5, captured5);

with_instance_happy_path_tests!(with_instance_6_types_invokes_action_with_resolved_services, with_instance_6_types_returns_early_when_current_is_null;
    Service1 => service1, captured1; Service2 => service2, captured2; Service3 => service3, captured3; Service4 => service4, captured4;
    Service5 => service5, captured5; Service6 => service6, captured6);

with_instance_happy_path_tests!(with_instance_7_types_invokes_action_with_resolved_services, with_instance_7_types_returns_early_when_current_is_null;
    Service1 => service1, captured1; Service2 => service2, captured2; Service3 => service3, captured3; Service4 => service4, captured4;
    Service5 => service5, captured5; Service6 => service6, captured6; Service7 => service7, captured7);

with_instance_happy_path_tests!(with_instance_8_types_invokes_action_with_resolved_services, with_instance_8_types_returns_early_when_current_is_null;
    Service1 => service1, captured1; Service2 => service2, captured2; Service3 => service3, captured3; Service4 => service4, captured4;
    Service5 => service5, captured5; Service6 => service6, captured6; Service7 => service7, captured7; Service8 => service8, captured8);
