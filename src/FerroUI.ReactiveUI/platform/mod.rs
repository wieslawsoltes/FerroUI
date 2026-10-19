//! The helpers that the platforms of ReactiveUI share (`Shared/Platform`,
//! `Shared/ViewContractObservableHelpers.cs`, `Shared/ViewHostResolution.cs`): small observables
//! and sinks without operator pipelines, the view contract of the view hosts, and their view
//! lookup.
//!
//! Upstream compiles these internal helpers into each platform assembly. They are not re-exported
//! at the root of the crate. `ChangeSetBinder` is in [`change_sets`](crate::change_sets).
//!
//! Deviation (DEVIATIONS.md, ReactiveUI): the internal helpers of the platforms are public, in this
//! module, since the integration of FerroUI's user interface is another crate.
//!
//! Not ported: `ItemsControlTemplateBinding`, which reads the `ItemsControl` of WPF and WinUI (a
//! platform of user interface).

mod combine_latest_sink;
mod from_event_observable;
mod idle_timeout_observable;
mod observe_on_observable;
mod start_with_observable;
mod view_contract_observable_helpers;
mod view_host_resolution;

pub use combine_latest_sink::CombineLatestSink;
pub use from_event_observable::{EventSink, FromEventObservable};
pub use idle_timeout_observable::IdleTimeoutObservable;
pub use observe_on_observable::ObserveOnObservable;
pub use start_with_observable::StartWithObservable;
pub use view_contract_observable_helpers::ViewContractObservableHelpers;
pub use view_host_resolution::{ResolveView, ViewHostResolution};
