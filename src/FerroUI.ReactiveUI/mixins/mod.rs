//! `Mixins` (`ReactiveUI.Shared/Mixins`): extension methods of observables.
//!
//! The other mixins of the folder (`ChangeSetMixins`, `DependencyResolverMixins`,
//! `MutableDependencyResolverExtensions`, `AutoPersistHelperMixins`, `BuilderMixins`) belong to
//! later waves.

pub(crate) mod observable_logging_mixins;
pub(crate) mod observable_mixins;
pub(crate) mod switch_subscribe_mixins;

pub use observable_logging_mixins::ObservableLoggingMixins;
pub use observable_mixins::{from_async_with_all_notifications, from_async_with_all_notifications_param, AsyncExecution, DeferredValueObservable, ObservableMixins};
pub use switch_subscribe_mixins::{SwitchSource, SwitchSubscribeMixins};
