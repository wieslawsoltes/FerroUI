//! `IReactiveObjectExtensions` (`ReactiveUI.Shared/ReactiveObject/IReactiveObjectExtensions.cs`):
//! the setters of notifying properties, the suppression and delay of the notifications, and the
//! observables of the changes of a reactive object.

use super::{IExtensionState, IReactiveObject};
use crate::{IReactiveNotifyPropertyChanged, ReactivePropertyChangedEventArgs};
use ferroui_system::component_model::BackingField;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// The extension methods of every reactive object (`IReactiveObjectExtensions`), implemented for
/// every `T: IReactiveObject`.
///
/// The raising of a single change, [`raise_property_changing`](IReactiveObject::raise_property_changing)
/// and [`raise_property_changed`](IReactiveObject::raise_property_changed), are methods of
/// [`IReactiveObject`] itself.
pub trait IReactiveObjectExt: IReactiveObject {
    /// The setter of a notifying property (`RaiseAndSetIfChanged`): unless `field` already holds
    /// a value equal to `value`, raises the changing notifications of `property_name`, stores the
    /// value, and raises the changed notifications. The field is a `RefCell`, or a `Cell` of a
    /// `Copy` value.
    ///
    /// Deviation (DEVIATIONS.md, ReactiveUI): it returns whether the value changed, where upstream
    /// returns the value; the property name is given, where upstream takes the name of the caller
    /// (`[CallerMemberName]`).
    fn raise_and_set_if_changed<V: PartialEq>(&self, field: &impl BackingField<V>, value: V, property_name: &str) -> bool {
        if field.equals(&value, &|current, value| current == value) {
            return false;
        }
        self.raise_property_changing(property_name);
        field.store(value);
        self.raise_property_changed(property_name);
        true
    }

    /// [`raise_and_set_if_changed`](Self::raise_and_set_if_changed) for a `Cell` field.
    fn raise_and_set_if_changed_cell<V: Copy + PartialEq>(&self, field: &Cell<V>, value: V, property_name: &str) -> bool {
        self.raise_and_set_if_changed(field, value, property_name)
    }

    /// Makes changes raise the `PropertyChanging` event (`SubscribePropertyChangingEvents`), for
    /// an object that implements [`IReactiveObject`] by hand.
    fn subscribe_property_changing_events(&self) {
        self.reactive_object().subscribe_property_changing_events();
    }

    /// Makes changes raise the `PropertyChanged` event (`SubscribePropertyChangedEvents`).
    fn subscribe_property_changed_events(&self) {
        self.reactive_object().subscribe_property_changed_events();
    }

    /// Stops all change notifications until the result is disposed
    /// (`SuppressChangeNotifications`); the calls nest.
    fn suppress_change_notifications(&self) -> Rc<dyn IDisposable> {
        self.reactive_object().suppress_change_notifications()
    }

    /// Whether change notifications are sent (`AreChangeNotificationsEnabled`).
    fn are_change_notifications_enabled(&self) -> bool {
        self.reactive_object().are_change_notifications_enabled()
    }

    /// Keeps the change notifications until the result is disposed, then sends the last one of
    /// each property (`DelayChangeNotifications`); the calls nest.
    fn delay_change_notifications(&self) -> Rc<dyn IDisposable> {
        self.reactive_object().delay_change_notifications()
    }

    /// Whether change notifications are delayed (`AreChangeNotificationsDelayed`).
    fn are_change_notifications_delayed(&self) -> bool {
        self.reactive_object().are_change_notifications_delayed()
    }
}

impl<T: IReactiveObject + ?Sized> IReactiveObjectExt for T {}

/// The observables of a reactive object that upstream's extension methods create at each call
/// (`GetChangingObservable`, `GetChangedObservable`, `GetThrownExceptionsObservable`). The
/// observables that [`IReactiveNotifyPropertyChanged`] gives are created once per object from
/// these.
pub struct IReactiveObjectExtensions;

impl IReactiveObjectExtensions {
    /// An observable of the properties of `source` that are about to change, with `source` as the
    /// sender (`GetChangingObservable`).
    pub fn get_changing_observable<T: IReactiveObject>(source: &Rc<T>) -> Rc<dyn IObservable<ReactivePropertyChangedEventArgs<Rc<T>>>> {
        let names = IExtensionState::changing(&**source.reactive_object().state());
        Rc::new(ChangeArgsObservable { source: names, sender: Rc::downgrade(source) })
    }

    /// An observable of the properties of `source` that changed, with `source` as the sender
    /// (`GetChangedObservable`).
    pub fn get_changed_observable<T: IReactiveObject>(source: &Rc<T>) -> Rc<dyn IObservable<ReactivePropertyChangedEventArgs<Rc<T>>>> {
        let names = IExtensionState::changed(&**source.reactive_object().state());
        Rc::new(ChangeArgsObservable { source: names, sender: Rc::downgrade(source) })
    }

    /// The errors of the subscribers of the change observables of `source`
    /// (`GetThrownExceptionsObservable`).
    pub fn get_thrown_exceptions_observable<T: IReactiveObject + ?Sized>(source: &T) -> Rc<dyn IObservable<ObservableError>> {
        IExtensionState::thrown_exceptions(&**source.reactive_object().state())
    }
}

/// The changes of an object as arguments that carry the object (`ChangeArgsCastObservable`).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the object is held weakly, so that a subscription does
/// not keep it alive (there is no garbage collector to break the cycle of the object, its
/// subjects and the subscription); a change raised while the object is being dropped is not
/// delivered.
struct ChangeArgsObservable<T> {
    source: Rc<dyn IObservable<Rc<str>>>,
    sender: Weak<T>,
}

impl<T: 'static> IObservable<ReactivePropertyChangedEventArgs<Rc<T>>> for ChangeArgsObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<ReactivePropertyChangedEventArgs<Rc<T>>>>) -> Rc<dyn IDisposable> {
        self.source.subscribe(Rc::new(ChangeArgsSink { downstream: observer, sender: self.sender.clone() }))
    }
}

/// Gives each name of a change its sender (`ChangeArgsCastObservable.Sink`).
struct ChangeArgsSink<T> {
    downstream: Rc<dyn IObserver<ReactivePropertyChangedEventArgs<Rc<T>>>>,
    sender: Weak<T>,
}

impl<T: 'static> IObserver<Rc<str>> for ChangeArgsSink<T> {
    fn on_next(&self, value: Rc<str>) {
        let Some(sender) = self.sender.upgrade() else {
            return;
        };
        self.downstream.on_next(ReactivePropertyChangedEventArgs { sender, property_name: value.to_string() });
    }

    fn on_error(&self, error: ObservableError) {
        self.downstream.on_error(error);
    }

    fn on_completed(&self) {
        self.downstream.on_completed();
    }
}

/// The observables of the changes of a reactive object, from its shared handle: `vm.changing()`,
/// `vm.changed()` (`ReactiveObject.Changing`, `ReactiveObject.Changed`). Each is created at the
/// first call and is the same observable afterwards.
impl<T: IReactiveObject> IReactiveNotifyPropertyChanged<Rc<T>> for Rc<T> {
    fn changing(&self) -> Rc<dyn IObservable<ReactivePropertyChangedEventArgs<Rc<T>>>> {
        self.reactive_object().changing_of(self)
    }

    fn changed(&self) -> Rc<dyn IObservable<ReactivePropertyChangedEventArgs<Rc<T>>>> {
        self.reactive_object().changed_of(self)
    }

    fn suppress_change_notifications(&self) -> Rc<dyn IDisposable> {
        self.reactive_object().suppress_change_notifications()
    }
}
