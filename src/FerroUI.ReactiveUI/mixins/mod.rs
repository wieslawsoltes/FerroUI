//! `Mixins` (`ReactiveUI.Shared/Mixins`): the extension methods of observables (`ObservableMixins`,
//! `SwitchSubscribeMixins`, the logging mixins) and AutoPersist.
//!
//! `ChangeSetMixins` is in [`change_sets`](crate::change_sets), the builder and resolver mixins in
//! [`builder`](crate::builder) and [`registration`](crate::registration).

pub(crate) mod observable_logging_mixins;
pub(crate) mod observable_mixins;
pub(crate) mod switch_subscribe_mixins;

pub use observable_logging_mixins::ObservableLoggingMixins;
pub use observable_mixins::{from_async_with_all_notifications, from_async_with_all_notifications_param, AsyncExecution, DeferredValueObservable, ObservableMixins};
pub use switch_subscribe_mixins::{SwitchSource, SwitchSubscribeMixins};
pub(crate) mod auto_persist_helper_mixins;
pub(crate) mod auto_persist_helper_mixins_metadata;
pub(crate) mod auto_persist_helper_mixins_persistence;
pub use auto_persist_helper_mixins::{AutoPersistCollectionMixins, AutoPersistHelperMixins, DataContract};
pub use auto_persist_helper_mixins_metadata::AutoPersistMetadataMixins;
pub use auto_persist_helper_mixins_persistence::{AutoPersistCollectionMetadataMixins, AutoPersistMetadata, MetadataProvider, PersistFunction};
