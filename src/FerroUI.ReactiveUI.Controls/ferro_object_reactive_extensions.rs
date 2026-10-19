//! `FerroObjectReactiveExtensions` (`...ObjectReactiveExtensions.cs` of the integration): the
//! properties of a FerroUI object as signals, observers and observables at once.

use ferroui_base::data::{BindingPriority, BindingValue};
use ferroui_base::{BoxedValue, FerroObject, FerroObjectExtensions, FerroProperty, PropertyValue, Ref, StyledProperty};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::fmt;
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;

/// The properties of a FerroUI object as signals (`FerroObjectReactiveExtensions`): a signal
/// observes the property, as [`FerroObjectExtensions::get_observable`] does (the current value on
/// subscription, then each change), and sets the property to each value pushed into it.
///
/// The typed forms take a styled property; the `_untyped` forms take any registered property and
/// carry its values as [`BoxedValue`]s, which must hold the property's value type. The
/// `get_binding_subject` forms carry [`BindingValue`]s and set only the ones that hold a value.
/// Without a priority, values are set at [`BindingPriority::LocalValue`].
///
/// Deviation (DEVIATIONS.md, ReactiveUI controls): the overloads of `GetSubject` and
/// `GetBindingSubject` on a property and its typed form are named by their difference
/// (`_untyped`, `_with_priority`), and the signal is the `ISignal` of the `Primitives` flavour,
/// [`FerroPropertySignal`].
pub trait FerroObjectReactiveExtensions {
    /// A signal of `property` that sets it at [`BindingPriority::LocalValue`]
    /// (`GetSubject<T>(FerroProperty<T>)`).
    fn get_subject<T: PropertyValue>(&self, property: &'static StyledProperty<T>) -> Rc<FerroPropertySignal<T>>;

    /// A signal of `property` that sets it at `priority`
    /// (`GetSubject<T>(FerroProperty<T>, BindingPriority)`).
    fn get_subject_with_priority<T: PropertyValue>(
        &self,
        property: &'static StyledProperty<T>,
        priority: BindingPriority,
    ) -> Rc<FerroPropertySignal<T>>;

    /// An untyped signal of `property` that sets it at [`BindingPriority::LocalValue`]
    /// (`GetSubject(FerroProperty)`).
    fn get_subject_untyped(&self, property: &'static FerroProperty) -> Rc<FerroPropertySignal<BoxedValue>>;

    /// An untyped signal of `property` that sets it at `priority`
    /// (`GetSubject(FerroProperty, BindingPriority)`).
    fn get_subject_untyped_with_priority(
        &self,
        property: &'static FerroProperty,
        priority: BindingPriority,
    ) -> Rc<FerroPropertySignal<BoxedValue>>;

    /// A signal of the binding values of `property` that sets it at [`BindingPriority::LocalValue`]
    /// (`GetBindingSubject<T>(FerroProperty<T>)`).
    fn get_binding_subject<T: PropertyValue>(&self, property: &'static StyledProperty<T>) -> Rc<FerroPropertySignal<BindingValue<T>>>;

    /// A signal of the binding values of `property` that sets it at `priority`
    /// (`GetBindingSubject<T>(FerroProperty<T>, BindingPriority)`).
    fn get_binding_subject_with_priority<T: PropertyValue>(
        &self,
        property: &'static StyledProperty<T>,
        priority: BindingPriority,
    ) -> Rc<FerroPropertySignal<BindingValue<T>>>;

    /// An untyped signal of the binding values of `property` that sets it at
    /// [`BindingPriority::LocalValue`] (`GetBindingSubject(FerroProperty)`).
    fn get_binding_subject_untyped(&self, property: &'static FerroProperty) -> Rc<FerroPropertySignal<BindingValue<BoxedValue>>>;

    /// An untyped signal of the binding values of `property` that sets it at `priority`
    /// (`GetBindingSubject(FerroProperty, BindingPriority)`).
    fn get_binding_subject_untyped_with_priority(
        &self,
        property: &'static FerroProperty,
        priority: BindingPriority,
    ) -> Rc<FerroPropertySignal<BindingValue<BoxedValue>>>;
}

impl FerroObjectReactiveExtensions for FerroObject {
    fn get_subject<T: PropertyValue>(&self, property: &'static StyledProperty<T>) -> Rc<FerroPropertySignal<T>> {
        self.get_subject_with_priority(property, BindingPriority::LocalValue)
    }

    fn get_subject_with_priority<T: PropertyValue>(
        &self,
        property: &'static StyledProperty<T>,
        priority: BindingPriority,
    ) -> Rc<FerroPropertySignal<T>> {
        let target: Ref<FerroObject> = self.to_ref();
        FerroPropertySignal::new(
            move |value: T| {
                let _ = target.set_value_with_priority(property, value, priority);
            },
            self.get_observable(property),
        )
    }

    fn get_subject_untyped(&self, property: &'static FerroProperty) -> Rc<FerroPropertySignal<BoxedValue>> {
        self.get_subject_untyped_with_priority(property, BindingPriority::LocalValue)
    }

    fn get_subject_untyped_with_priority(
        &self,
        property: &'static FerroProperty,
        priority: BindingPriority,
    ) -> Rc<FerroPropertySignal<BoxedValue>> {
        let target: Ref<FerroObject> = self.to_ref();
        FerroPropertySignal::new(
            move |value: BoxedValue| {
                let _ = target.set_value_untyped(property, (*value).as_any(), priority);
            },
            self.get_observable_untyped(property),
        )
    }

    fn get_binding_subject<T: PropertyValue>(&self, property: &'static StyledProperty<T>) -> Rc<FerroPropertySignal<BindingValue<T>>> {
        self.get_binding_subject_with_priority(property, BindingPriority::LocalValue)
    }

    fn get_binding_subject_with_priority<T: PropertyValue>(
        &self,
        property: &'static StyledProperty<T>,
        priority: BindingPriority,
    ) -> Rc<FerroPropertySignal<BindingValue<T>>> {
        let target: Ref<FerroObject> = self.to_ref();
        FerroPropertySignal::new(
            move |value: BindingValue<T>| {
                if !value.has_value() {
                    return;
                }
                let _ = target.set_value_with_priority(property, value.value().clone(), priority);
            },
            self.get_binding_observable(property),
        )
    }

    fn get_binding_subject_untyped(&self, property: &'static FerroProperty) -> Rc<FerroPropertySignal<BindingValue<BoxedValue>>> {
        self.get_binding_subject_untyped_with_priority(property, BindingPriority::LocalValue)
    }

    fn get_binding_subject_untyped_with_priority(
        &self,
        property: &'static FerroProperty,
        priority: BindingPriority,
    ) -> Rc<FerroPropertySignal<BindingValue<BoxedValue>>> {
        let target: Ref<FerroObject> = self.to_ref();
        FerroPropertySignal::new(
            move |value: BindingValue<BoxedValue>| {
                if !value.has_value() {
                    return;
                }
                let boxed: &BoxedValue = value.value();
                let _ = target.set_value_untyped(property, (**boxed).as_any(), priority);
            },
            self.get_binding_observable_untyped(property),
        )
    }
}

/// A property of a FerroUI object as a signal: an observer whose values `on_next` handles, and the
/// observable of the property (`FerroPropertySignal<T>`).
///
/// It counts its observers ([`has_observers`](Self::has_observers)); it is disposed by its
/// completion, by an error or by [`dispose`](IDisposable::dispose), after which the values pushed
/// into it are dropped.
pub struct FerroPropertySignal<T> {
    on_next: Box<dyn Fn(T)>,
    observable: Rc<dyn IObservable<T>>,
    observer_count: Rc<Cell<usize>>,
    is_disposed: Cell<bool>,
}

impl<T> fmt::Debug for FerroPropertySignal<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FerroPropertySignal")
            .field("observer_count", &self.observer_count.get())
            .field("is_disposed", &self.is_disposed.get())
            .finish_non_exhaustive()
    }
}

impl<T: 'static> FerroPropertySignal<T> {
    /// The signal that hands the values pushed into it to `on_next` and whose observers subscribe
    /// to `observable` (`FerroPropertySignal<T>(Action<T>, IObservable<T>)`).
    pub fn new(on_next: impl Fn(T) + 'static, observable: Rc<dyn IObservable<T>>) -> Rc<Self> {
        Rc::new(Self { on_next: Box::new(on_next), observable, observer_count: Rc::new(Cell::new(0)), is_disposed: Cell::new(false) })
    }

    /// Whether the signal has been disposed (`IsDisposed`).
    pub fn is_disposed(&self) -> bool {
        self.is_disposed.get()
    }

    /// Whether observers are subscribed (`HasObservers`).
    pub fn has_observers(&self) -> bool {
        self.observer_count.get() > 0
    }
}

impl<T: 'static> IObserver<T> for FerroPropertySignal<T> {
    /// Hands `value` to the signal's action, unless the signal is disposed (`OnNext`).
    fn on_next(&self, value: T) {
        if self.is_disposed.get() {
            return;
        }
        (self.on_next)(value);
    }

    /// Disposes the signal and raises `error` again (`OnError`).
    // Deviation (DEVIATIONS.md, ReactiveUI controls): an error is never null; it is raised again as
    // a panic, the port's form of the exception upstream throws.
    fn on_error(&self, error: ObservableError) {
        self.is_disposed.set(true);
        panic!("{error}");
    }

    /// Disposes the signal (`OnCompleted`).
    fn on_completed(&self) {
        self.is_disposed.set(true);
    }
}

impl<T: 'static> IObservable<T> for FerroPropertySignal<T> {
    /// Subscribes `observer` to the property, counting it until the subscription is disposed; the
    /// count is restored when subscribing panics (`Subscribe`).
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        self.observer_count.set(self.observer_count.get() + 1);
        let subscribed = panic::catch_unwind(AssertUnwindSafe(|| self.observable.subscribe(observer)));
        let subscription = match subscribed {
            Ok(subscription) => subscription,
            Err(payload) => {
                self.observer_count.set(self.observer_count.get() - 1);
                panic::resume_unwind(payload);
            }
        };
        Rc::new(SignalSubscription { subscription, observer_count: self.observer_count.clone(), disposed: Cell::new(false) })
    }
}

impl<T: 'static> IDisposable for FerroPropertySignal<T> {
    /// Disposes the signal (`Dispose`, which completes it).
    fn dispose(&self) {
        self.on_completed();
    }
}

/// The subscription of an observer of a [`FerroPropertySignal`]: disposing it ends the
/// subscription to the property and uncounts the observer, once.
struct SignalSubscription {
    subscription: Rc<dyn IDisposable>,
    observer_count: Rc<Cell<usize>>,
    disposed: Cell<bool>,
}

impl IDisposable for SignalSubscription {
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        self.subscription.dispose();
        self.observer_count.set(self.observer_count.get().saturating_sub(1));
    }
}
