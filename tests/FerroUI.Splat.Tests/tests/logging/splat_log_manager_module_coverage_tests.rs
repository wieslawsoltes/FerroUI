//! Ported from `Splat.Logging.Tests/SplatLogManagerModuleCoverageTests.cs` of Splat.
//! Adapted: the resolver is a `Rc<ModernDependencyResolver>`, handed to the module as the
//! `Rc<dyn IReadonlyDependencyResolver>` it reads the loggers from and to `configure` as the
//! `&dyn IMutableDependencyResolver` it registers into. The resolver registers one shared
//! `TextLogger` (upstream a new one per resolution). `IsTypeOf<DefaultLogManager>` is not checkable
//! on a `Rc<dyn ILogManager>`: the test checks the behaviour of `DefaultLogManager` instead, whose
//! loggers write to the `ILogger` of the resolver with the short name of the type in front.

use crate::tests::common::{same, IMockLogTarget, TextLogger};
use ferroui_splat::{
    FuncLogManager, IFullLogger, ILogManager, ILogger, IModule, LogLevel, ModernDependencyResolver,
    MutableDependencyResolverExt, ReadonlyDependencyResolverExt, SplatLogManagerModule, WrappingFullLogger,
};
use std::rc::Rc;

/// The type a logger is asked for.
#[allow(dead_code)] // only named as a type
struct SplatLogManagerModuleCoverageTests;

fn resolver_with_logger(text_logger: &Rc<TextLogger>) -> Rc<ModernDependencyResolver> {
    let resolver = Rc::new(ModernDependencyResolver::new());
    let text_logger = text_logger.clone();
    resolver.register::<dyn ILogger>(move || Some(text_logger.clone() as Rc<dyn ILogger>));
    resolver
}

#[test]
fn configure_on_empty_resolver_registers_log_manager() {
    let resolver = resolver_with_logger(&Rc::new(TextLogger::new()));

    SplatLogManagerModule::new(resolver.clone()).configure(&*resolver);

    assert!(resolver.has_registration::<dyn ILogManager>());
}

#[test]
fn configure_on_empty_resolver_registers_default_log_manager() {
    let text_logger = Rc::new(TextLogger::new());
    let resolver = resolver_with_logger(&text_logger);

    SplatLogManagerModule::new(resolver.clone()).configure(&*resolver);

    let log_manager = resolver.get_service::<dyn ILogManager>().expect("a log manager is registered");
    log_manager
        .get_logger(std::any::type_name::<SplatLogManagerModuleCoverageTests>())
        .info("message");
    assert_eq!(
        text_logger.logs(),
        vec![(LogLevel::Info, "SplatLogManagerModuleCoverageTests: message".to_owned())]
    );
}

#[test]
fn configure_does_not_replace_existing_log_manager() {
    let resolver = Rc::new(ModernDependencyResolver::new());
    let existing: Rc<dyn ILogManager> = Rc::new(FuncLogManager::new(|_| {
        Rc::new(WrappingFullLogger::new(Rc::new(TextLogger::new()))) as Rc<dyn IFullLogger>
    }));
    let registered = existing.clone();
    resolver.register::<dyn ILogManager>(move || Some(registered.clone()));

    SplatLogManagerModule::new(resolver.clone()).configure(&*resolver);

    let log_manager = resolver.get_service::<dyn ILogManager>().expect("a log manager is registered");
    assert!(same(&log_manager, &existing));
}

// Not ported: Configure_With_Null_Resolver_Throws - argument-null validation, which Rust's types make impossible
