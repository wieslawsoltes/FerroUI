//! `ReactiveNotificationHelpers` (`ReactiveUI.Shared/ReactiveObject/ReactiveNotificationHelpers.cs`).

use super::{IReactiveObject, IReactiveObjectExt, IReactiveObjectExtensions};
use crate::ReactivePropertyChangedEventArgs;
use ferroui_system::events::Event;
use ferroui_system::{IObservable, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The `Changing` or `Changed` observable of a reactive object, as a cache holds it.
pub type ChangeObservable<T> = Rc<dyn IObservable<ReactivePropertyChangedEventArgs<Rc<T>>>>;

/// The notification state that the implementations of a reactive object share
/// (`ReactiveNotificationHelpers`): adding the handlers of the events, and the observables created
/// once and cached by the object. [`ReactiveObject`](crate::ReactiveObject) uses them, and so can
/// an object that implements [`IReactiveObject`] by hand.
pub struct ReactiveNotificationHelpers;

impl ReactiveNotificationHelpers {
    /// Adds a handler to the `PropertyChanging` event `handlers` of `source`, making its changes
    /// raise the event the first time (`AddPropertyChanging`); returns the token that removes the
    /// handler.
    pub fn add_property_changing<T: IReactiveObject + ?Sized>(
        source: &T,
        subscribed: &Cell<bool>,
        handlers: &Event<str>,
        handler: Rc<dyn Fn(&str)>,
    ) -> u64 {
        if !subscribed.get() {
            source.subscribe_property_changing_events();
            subscribed.set(true);
        }
        handlers.add(handler)
    }

    /// Adds a handler to the `PropertyChanged` event `handlers` of `source`, making its changes
    /// raise the event the first time (`AddPropertyChanged`); returns the token that removes the
    /// handler.
    pub fn add_property_changed<T: IReactiveObject + ?Sized>(
        source: &T,
        subscribed: &Cell<bool>,
        handlers: &Event<str>,
        handler: Rc<dyn Fn(&str)>,
    ) -> u64 {
        if !subscribed.get() {
            source.subscribe_property_changed_events();
            subscribed.set(true);
        }
        handlers.add(handler)
    }

    /// The `Changing` observable of `source`, created at the first call and kept in `observable`
    /// (`GetChanging`).
    pub fn get_changing<T: IReactiveObject>(source: &Rc<T>, observable: &RefCell<Option<ChangeObservable<T>>>) -> ChangeObservable<T> {
        if let Some(current) = observable.borrow().clone() {
            return current;
        }
        let created = IReactiveObjectExtensions::get_changing_observable(source);
        *observable.borrow_mut() = Some(created.clone());
        created
    }

    /// The `Changed` observable of `source`, created at the first call and kept in `observable`
    /// (`GetChanged`).
    pub fn get_changed<T: IReactiveObject>(source: &Rc<T>, observable: &RefCell<Option<ChangeObservable<T>>>) -> ChangeObservable<T> {
        if let Some(current) = observable.borrow().clone() {
            return current;
        }
        let created = IReactiveObjectExtensions::get_changed_observable(source);
        *observable.borrow_mut() = Some(created.clone());
        created
    }

    /// The `ThrownExceptions` observable of `source`, created at the first call and kept in
    /// `observable` (`GetThrownExceptions`).
    pub fn get_thrown_exceptions<T: IReactiveObject + ?Sized>(
        source: &T,
        observable: &RefCell<Option<Rc<dyn IObservable<ObservableError>>>>,
    ) -> Rc<dyn IObservable<ObservableError>> {
        if let Some(current) = observable.borrow().clone() {
            return current;
        }
        let created = IReactiveObjectExtensions::get_thrown_exceptions_observable(source);
        *observable.borrow_mut() = Some(created.clone());
        created
    }
}
