//! The test files of the integration's test projects that are not ported, with the reason. The
//! names drop the name of the reference framework, as the file names of the port do.
//!
//! The dependency injection adapters of the integration (Autofac, DryIoc, Ninject,
//! Microsoft.Extensions.DependencyInjection) are not ported (the owner tracks dependency injection
//! in issue 251), so neither are their tests:
//!
//! - Not ported: `MixinsAutofacTests` (the main test project) - DI container.
//! - Not ported: `MixinsNinjectTests` (the main test project) - DI container.
//! - Not ported: `DependencyInjectionTests` (the main test project) - DI container.
//! - Not ported: `UseReactiveUIWithDIContainerTests` (the main test project) - DI container.
//! - Not ported: `AutofacIsolatedTestExecutor` (the main test project, the executor of the Autofac tests) - DI container.
//! - Not ported: `ReactiveDependencyInjectionMixinsCoverageTests` (the `.Reactive.Tests` project)
//!   - DI container.
//! - Not ported: `MixinsDryIocTests`, `MixinsDryIocMoreTests`
//!   (the DryIoc test project) - DI container.
//! - Not ported: `MixinsMicrosoftTests`, `MixinsMicrosoftMoreTests`
//!   (the Microsoft test project) - DI container.
//!
//! The files of the `.Reactive.Tests` project that the views share with the main test project
//! compile the same tests in the System.Reactive flavour, which is the only flavour of the port:
//!
//! - `AutoSuspendHelperShutdownTests` (the `.Reactive.Tests` project) is the file of the
//!   main project but for its namespace; it is ported once (`auto_suspend_helper_shutdown_tests`).
//! - `ReactiveUserControlActivationTests` (the `.Reactive.Tests` project) is the file of the
//!   main project in the System.Reactive flavour; it is ported once
//!   (`reactive_user_control_activation_tests`).
//!
//! The infrastructure of the test projects is replaced by the unit test application of the
//! framework (`view_support`): `AssemblyHooks`, `TestExecutor`, `TestSession`,
//! `ControlledLifetimeFixture` (ported as `view_support::ControlledLifetime`), `Mocks/` (the demo
//! application, its window and its views, which no ported test uses).
