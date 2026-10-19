//! `BuilderInstanceMixinsTests` (`ReactiveUI.Builder.Tests/Mixins/BuilderInstanceMixinsTests.cs`):
//! the services of the tests of `WithInstance` on the builder, and the helpers of the tests of
//! each arity.
//!
//! Upstream's helpers per arity (`RegisterServices09` to `16` here,
//! `InvokeWithInstance09` to `16` in `BuilderInstanceMixinsTests.HelpersInvoke.cs` and
//! `AssertSameReferences09` to `16` in `BuilderInstanceMixinsTests.HelpersAssert.cs`, which hold
//! no tests) are the macro [`builder_with_instance_tests!`], which writes the two tests of an
//! arity: the action receives the registered services, and it is skipped without a resolver.

/// Declares the marker services of the tests (`InstanceService01` to `InstanceService16`).
macro_rules! instance_services {
    ($($service:ident),+) => {
        $(
            #[doc = concat!("`", stringify!($service), "`.")]
            pub struct $service;
        )+
    };
}

instance_services!(
    InstanceService01,
    InstanceService02,
    InstanceService03,
    InstanceService04,
    InstanceService05,
    InstanceService06,
    InstanceService07,
    InstanceService08,
    InstanceService09,
    InstanceService10,
    InstanceService11,
    InstanceService12,
    InstanceService13,
    InstanceService14,
    InstanceService15,
    InstanceService16
);

/// Writes the two tests of `WithInstance` of one arity on the builder: `$invokes`
/// (`Builder_WithInstance_N_Types_invokes_action`), in which the action receives the services
/// registered in the resolver of the builder (`RegisterServicesNN`, `InvokeWithInstanceNN`,
/// `AssertSameReferencesNN`), and `$skips` (`Builder_WithInstance_N_Types_skips_when_null`), in
/// which a builder without a resolver of its application does not call the action.
///
/// Each service is given as `Type => value, captured`, the names of its registered value and of
/// the value the action received.
macro_rules! builder_with_instance_tests {
    ($invokes:ident, $skips:ident; $($service:ident => $value:ident, $captured:ident);+ $(;)?) => {
        #[test]
        fn $invokes() {
            builder_test(|| {
                let resolver = ::std::rc::Rc::new(::ferroui_splat::ModernDependencyResolver::new());
                let builder = resolver.create_reactive_ui_builder();
                $(
                    let $value = ::std::rc::Rc::new($service);
                    ::ferroui_splat::MutableDependencyResolverExt::register_constant(&*resolver, $value.clone());
                )+
                let _ = builder.with_core_services().build();

                let captured = ::std::cell::RefCell::new(None);
                let _ = builder.with_instance(|services: ($(Option<::std::rc::Rc<$service>>,)+)| {
                    *captured.borrow_mut() = Some(services);
                });

                let ($($captured,)+) = captured.into_inner().expect("the action runs");
                $(
                    assert!(::std::rc::Rc::ptr_eq(&$captured.expect("the service is resolved"), &$value));
                )+
            });
        }

        #[test]
        fn $skips() {
            builder_test(|| {
                let resolver = ::std::rc::Rc::new(::ferroui_splat::ModernDependencyResolver::new());
                let builder = ReactiveUIBuilder::new(resolver, None);
                let _ = builder.with_core_services();
                let invoked = ::std::cell::Cell::new(false);

                let _ = builder.with_instance(|_: ($(Option<::std::rc::Rc<$service>>,)+)| invoked.set(true));

                assert!(!invoked.get());
            });
        }
    };
}

pub(super) use builder_with_instance_tests;
