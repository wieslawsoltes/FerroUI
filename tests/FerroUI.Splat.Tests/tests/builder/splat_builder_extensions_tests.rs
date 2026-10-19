//! Ported from `Splat.Builder.Tests/SplatBuilderExtensionsTests.cs` of Splat.
//! Adapted: the `[Before(Test)]`/`[After(Test)]` scope is an `AppBuilderScope` at the start of each
//! test; `CreateSplatBuilder(Action<IMutableDependencyResolver>)` is `create_splat_builder_with`,
//! and the service `string` is `String`. `IsNotNull` of a builder checks that it holds the resolver
//! it was created over.

use crate::tests::common::{same, AppBuilderScope};
use ferroui_splat::service_location::InternalLocator;
use ferroui_splat::{
    IAppBuilder, IAppInstance, MutableDependencyResolverExt, ReadonlyDependencyResolverExt,
    SplatBuilderExt,
};
use ferroui_system::IDisposable;
use std::rc::Rc;

const HELLO_VALUE: &str = "Hello";

#[test]
fn create_splat_builder_returns_app_builder() {
    let _scope = AppBuilderScope::new();
    let resolver = InternalLocator::new();
    let builder = resolver.current_mutable().create_splat_builder();
    assert!(same(&builder.current_mutable(), &resolver.current_mutable()));
    resolver.dispose();
}

#[test]
fn create_splat_builder_with_configure_action_returns_app_builder() {
    let _scope = AppBuilderScope::new();
    let resolver = InternalLocator::new();
    let builder = resolver
        .current_mutable()
        .create_splat_builder_with(|r| r.register::<String>(|| Some(Rc::new(HELLO_VALUE.to_owned()))));
    let instance = builder.build();
    assert!(same(&instance.current_mutable(), &resolver.current_mutable()));
    let hello = resolver.current().get_service::<String>();
    assert_eq!(hello.as_deref().map(String::as_str), Some(HELLO_VALUE));
    resolver.dispose();
}

// Not ported: ApplyThrowsOnNullModule - argument-null validation, which Rust's types make impossible
// Not ported: CreateSplatBuilderThrowsOnNullResolver - argument-null validation, which Rust's types make impossible
// Not ported: CreateSplatBuilderWithConfigureActionThrowsOnNullResolver - argument-null validation, which Rust's types make impossible
// Not ported: CreateSplatBuilderWithConfigureActionReturnsAppBuilderNonGeneric - the `Type`-based `Register(Func<object?>, Type)`, which is not ported
