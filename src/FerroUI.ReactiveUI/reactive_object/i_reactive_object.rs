//! `IReactiveObject` (`ReactiveUI.Core/ReactiveObject/IReactiveObject.cs`) and the state slot of
//! `IReactiveObjectStateSlot` (`ReactiveUI.Shared/ReactiveObject/IReactiveObjectStateSlot.cs`).

use crate::ReactiveObject;
use ferroui_splat::IEnableLogger;
use ferroui_system::component_model::{INotifyPropertyChanged, INotifyPropertyChanging};

/// A view model that notifies when its properties are changing and have changed
/// (`IReactiveObject`); [`reactive_object!`](crate::reactive_object) implements it for a type that
/// holds a [`ReactiveObject`].
///
/// The raising methods go through the state of the object: they follow its suppression and delay,
/// and feed both the events and the observables of the changes. Helpers outside the object, such
/// as the helper of a derived property, raise the changes of the object through them.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream's `IReactiveObjectStateSlot`, the optional slot
/// that holds the state on the object, is [`reactive_object`](Self::reactive_object): every
/// reactive object holds its state, since there is no table of states keyed by object
/// (`ConditionalWeakTable`).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream's `RaisePropertyChanging(PropertyChangingEventArgs)`
/// and `RaisePropertyChanged(PropertyChangedEventArgs)` invoke the events, and the extension
/// methods of the same names go through the state. The two cannot share a name in Rust (a call
/// would be ambiguous), so these methods are the extension methods, and the state raises the
/// events of the [`ReactiveObject`] itself.
pub trait IReactiveObject: INotifyPropertyChanged + INotifyPropertyChanging + IEnableLogger + 'static {
    /// The part that holds the events and the state of the change notifications
    /// (`IReactiveObjectStateSlot.GetReactiveStateSlot`).
    fn reactive_object(&self) -> &ReactiveObject;

    /// Raises the notifications of a property that is about to change: the `PropertyChanging`
    /// event and the `Changing` observable (`RaisePropertyChanging(string)`). For properties
    /// whose setter does more than [`raise_and_set_if_changed`](crate::IReactiveObjectExt::raise_and_set_if_changed).
    fn raise_property_changing(&self, property_name: &str) {
        self.reactive_object().raising_property_changing(property_name, std::any::type_name::<Self>());
    }

    /// Raises the notifications of a property that changed: the `PropertyChanged` event and the
    /// `Changed` observable (`RaisePropertyChanged(string)`).
    fn raise_property_changed(&self, property_name: &str) {
        self.reactive_object().raising_property_changed(property_name, std::any::type_name::<Self>());
    }
}
