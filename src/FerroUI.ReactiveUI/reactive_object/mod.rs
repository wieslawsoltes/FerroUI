//! `ReactiveObject` (`ReactiveUI.Core/ReactiveObject`, `ReactiveUI.Shared/ReactiveObject`): the
//! base of view models, which notifies the changes of its properties as events and observables,
//! with the suppression and delay of the notifications, and the macros that write a view model's
//! reactive object and properties.

mod delayable_notification_signal;
mod extension_state;
mod i_extension_state;
mod i_reactive_object;
mod i_reactive_object_extensions;
#[allow(clippy::module_inception)]
mod reactive_object;
mod reactive_notification_helpers;
mod reactive_properties;
mod reactive_record;

pub use i_extension_state::IExtensionState;
pub use i_reactive_object::IReactiveObject;
pub use i_reactive_object_extensions::{IReactiveObjectExt, IReactiveObjectExtensions};
pub use reactive_notification_helpers::{ChangeObservable, ReactiveNotificationHelpers};
pub use reactive_object::ReactiveObject;
pub use reactive_record::ReactiveRecord;

/// The observables of the changes of a reactive object from its shared handle (`vm.changing()`,
/// `vm.changed()`): [`IReactiveNotifyPropertyChanged`](crate::IReactiveNotifyPropertyChanged),
/// which every `Rc<T>` with `T: IReactiveObject` implements, under the name that says so. The
/// errors of the subscribers are `vm.thrown_exceptions()`, through
/// [`IHandleObservableErrors`](crate::IHandleObservableErrors).
pub use crate::interfaces::IReactiveNotifyPropertyChanged as ReactiveObjectRcExt;
