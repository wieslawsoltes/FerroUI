//! ferroui-splat
//!
//! A port of Splat (<https://github.com/reactiveui/splat>) to Rust: the service locator that
//! ReactiveUI and applications find their services through, the dependency resolvers behind it,
//! logging, the detection of a unit test runner, feature usage tracking, and the application
//! builder with its modules. It depends on nothing of the user interface of FerroUI.
//!
//! The modules follow the projects of Splat:
//!
//! - [`service_location`]: Splat.Core's service location: the resolver interfaces
//!   ([`IReadonlyDependencyResolver`], [`IMutableDependencyResolver`], [`IDependencyResolver`]),
//!   the resolvers ([`ModernDependencyResolver`], [`GlobalGenericFirstDependencyResolver`],
//!   [`InstanceGenericFirstDependencyResolver`]), [`AppLocator`] and Splat's [`Locator`].
//! - [`builder`]: Splat.Core's `IAppBuilder`, `IAppInstance` and `IModule`, and Splat.Builder
//!   ([`AppBuilder`]).
//! - [`logging`]: Splat.Logging.
//! - [`mode_detection`]: Splat's mode detection ([`ModeDetector`]).
//! - [`application_performance_monitoring`]: Splat's feature usage tracking.
//!
//! Services are keyed by the type of the service, which may be a trait object
//! (`dyn IViewLocator`), and are handed out as `Rc<S>`. Everything is single-threaded and
//! reference counted, as the object model of FerroUI is: the locator, the resolvers and the
//! global state of Splat are per thread (`docs/porting/reactive.md`, section 5.1).

pub mod application_performance_monitoring;
pub mod builder;
mod disposables;
pub mod logging;
pub mod mode_detection;
pub mod service_location;

pub use application_performance_monitoring::{
    DefaultFeatureUsageTrackingManager, DefaultFeatureUsageTrackingSession,
    EnableFeatureUsageTrackingExt, FeatureUsageTrackingSessionExt, FuncFeatureUsageTrackingManager,
    Guid, IEnableFeatureUsageTracking, IFeatureUsageTrackingManager, IFeatureUsageTrackingSession,
    IFeatureUsageTrackingSessionOf, IViewTracking,
};
pub use builder::{AppBuilder, IAppBuilder, IAppInstance, IModule, ModuleExt, SplatBuilderExt};
pub use logging::{
    ActionLogger, ConsoleLogger, DebugLogger, DefaultLogManager, FullLoggerExt, FuncLogManager,
    IAllocationFreeErrorLogger, IAllocationFreeLogger, IEnableLogger, IFullLogger, ILogManager,
    ILogger, IStaticFullLogger, LogHost, LogLevel, LogManagerExt, LoggingException,
    MemoizingMRUCache, NullLogger, SplatLogManagerModule, SplatLoggerModule, StaticFullLogger,
    StaticFullLoggerExt, WrappingFullLogger, WrappingLogLevelLogger, WrappingPrefixLogger,
};
pub use mode_detection::{DefaultModeDetector, IModeDetector, Mode, ModeDetector};
pub use service_location::{
    AppLocator, DependencyResolverMixins, ErasedService, GlobalGenericFirstDependencyResolver,
    IDependencyResolver, IMutableDependencyResolver, IReadonlyDependencyResolver,
    InstanceGenericFirstDependencyResolver, LazyDisposal, Locator, ModernDependencyResolver,
    MutableDependencyResolverExt, ReadonlyDependencyResolverExt, RegistrationCallback,
    ResolverMixins, ServiceFactory, ServiceLocationInitializationExt, ServiceType,
};

#[cfg(test)]
mod tests;
