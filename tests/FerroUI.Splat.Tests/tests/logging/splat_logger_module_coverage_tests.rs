//! Ported from `Splat.Logging.Tests/SplatLoggerModuleCoverageTests.cs` of Splat.
//! Adapted: `configure` takes the resolver as a `&dyn IMutableDependencyResolver`.
//! `IsSameReferenceAs` is `same`.

use crate::tests::common::{same, TextLogger};
use ferroui_splat::{
    ILogger, IModule, ModernDependencyResolver, MutableDependencyResolverExt, ReadonlyDependencyResolverExt,
    SplatLoggerModule,
};
use std::rc::Rc;

#[test]
fn configure_on_empty_resolver_registers_logger() {
    let resolver = ModernDependencyResolver::new();

    SplatLoggerModule::new().configure(&resolver);

    assert!(resolver.has_registration::<dyn ILogger>());
}

#[test]
fn configure_does_not_replace_existing_logger() {
    let resolver = ModernDependencyResolver::new();
    let existing: Rc<dyn ILogger> = Rc::new(TextLogger::new());
    let registered = existing.clone();
    resolver.register::<dyn ILogger>(move || Some(registered.clone()));

    SplatLoggerModule::new().configure(&resolver);

    let logger = resolver.get_service::<dyn ILogger>().expect("a logger is registered");
    assert!(same(&logger, &existing));
}

// Not ported: Configure_On_Empty_Resolver_Registers_DebugLogger - IsTypeOf<DebugLogger> on a Rc<dyn ILogger>: nothing observable tells a DebugLogger apart (it writes to the standard error)
// Not ported: Configure_With_Null_Resolver_Throws - argument-null validation, which Rust's types make impossible
