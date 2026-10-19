//! `ReactiveObject` (`ReactiveUI.Shared/ReactiveObject/ReactiveObject.cs`) and the macro
//! [`reactive_object!`](crate::reactive_object) that makes a type holding one a reactive object.

use super::extension_state::{ExtensionState, PropertyEvents};
use super::{IExtensionState, IReactiveObject, IReactiveObjectExtensions};
use crate::ReactivePropertyChangedEventArgs;
use ferroui_system::events::Event;
use ferroui_system::{IDisposable, IObservable, ObservableError};
use std::any::Any;
use std::cell::{OnceCell, RefCell};
use std::fmt;
use std::rc::Rc;

/// The part of a view model that makes it a reactive object (`ReactiveObject`): its
/// `PropertyChanging` and `PropertyChanged` events, and the state of its change notifications
/// (suppression, delay, the `Changing`, `Changed` and `ThrownExceptions` observables), created
/// when first needed.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream's base class is a part that a view model holds
/// as a field, named `reactive` by convention. The macro [`reactive_object!`](crate::reactive_object)
/// writes the traits of a reactive object for the type, and
/// [`reactive_properties!`](crate::reactive_properties) its notifying properties. The view model is
/// shared as an `Rc`, whose handle gives the observables with the sender
/// ([`IReactiveNotifyPropertyChanged`](crate::IReactiveNotifyPropertyChanged)).
///
/// ```
/// use ferroui_reactiveui::{reactive_object, IReactiveNotifyPropertyChanged, IReactiveObjectExt, ReactiveObject};
/// use ferroui_reactive::ObservableExt;
/// use std::cell::RefCell;
/// use std::rc::Rc;
///
/// #[derive(Default)]
/// struct Person {
///     reactive: ReactiveObject,
///     name: RefCell<String>,
/// }
/// reactive_object!(Person);
///
/// impl Person {
///     fn name(&self) -> String {
///         self.name.borrow().clone()
///     }
///     fn set_name(&self, value: String) {
///         self.raise_and_set_if_changed(&self.name, value, "Name");
///     }
/// }
///
/// let person = Rc::new(Person::default());
/// let names = Rc::new(RefCell::new(Vec::new()));
/// let seen = names.clone();
/// let _subscription = person.changed().subscribe_next(move |change| seen.borrow_mut().push(change.property_name));
/// person.set_name("Ada".into());
/// person.set_name("Ada".into());
/// assert_eq!(person.name(), "Ada");
/// assert_eq!(*names.borrow(), vec!["Name".to_owned()]);
/// ```
pub struct ReactiveObject {
    events: Rc<PropertyEvents>,
    /// `_reactiveStateSlot`: the state, created when first needed.
    state: OnceCell<Rc<ExtensionState>>,
    /// `_changing`: the `Changing` observable of the handle of the object, once asked for.
    changing: RefCell<Option<Rc<dyn Any>>>,
    /// `_changed`: the `Changed` observable of the handle of the object, once asked for.
    changed: RefCell<Option<Rc<dyn Any>>>,
    /// `_thrownExceptions`.
    thrown_exceptions: RefCell<Option<Rc<dyn IObservable<ObservableError>>>>,
}

impl Default for ReactiveObject {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for ReactiveObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReactiveObject")
            .field("property_changing_handlers", &self.events.property_changing.handler_count())
            .field("property_changed_handlers", &self.events.property_changed.handler_count())
            .field("change_notifications_enabled", &self.are_change_notifications_enabled())
            .finish()
    }
}

impl AsRef<ReactiveObject> for ReactiveObject {
    fn as_ref(&self) -> &ReactiveObject {
        self
    }
}

impl ReactiveObject {
    /// The part of a reactive object without handlers or state.
    pub fn new() -> Self {
        Self {
            events: Rc::new(PropertyEvents::new()),
            state: OnceCell::new(),
            changing: RefCell::new(None),
            changed: RefCell::new(None),
            thrown_exceptions: RefCell::new(None),
        }
    }

    /// The event raised when a property is about to change (`PropertyChanging`). It is raised
    /// once it has a handler, and follows the suppression and the delay of the notifications.
    pub fn property_changing(&self) -> &Event<str> {
        &self.events.property_changing
    }

    /// The event raised when a property has changed (`PropertyChanged`).
    pub fn property_changed(&self) -> &Event<str> {
        &self.events.property_changed
    }

    /// The state of the change notifications, created at the first call (`GetState`, through
    /// `IReactiveObjectStateSlot.GetReactiveStateSlot`).
    pub(crate) fn state(&self) -> &Rc<ExtensionState> {
        self.state.get_or_init(|| ExtensionState::new(self.events.clone()))
    }

    /// The state of the change notifications, created at the first call (`GetState`).
    pub fn extension_state(&self) -> Rc<dyn IExtensionState> {
        self.state().clone()
    }

    /// Raises the changing notifications of `property_name` for an object of the type
    /// `sender_type` (`RaisingPropertyChanging`).
    pub(crate) fn raising_property_changing(&self, property_name: &str, sender_type: &'static str) {
        let state = self.state();
        state.set_sender_type(sender_type);
        state.raise_changing(property_name);
    }

    /// Raises the changed notifications of `property_name` for an object of the type
    /// `sender_type` (`RaisingPropertyChanged`).
    pub(crate) fn raising_property_changed(&self, property_name: &str, sender_type: &'static str) {
        let state = self.state();
        state.set_sender_type(sender_type);
        state.raise_changed(property_name);
    }

    /// Makes changes raise the `PropertyChanging` event (`SubscribePropertyChangingEvents`). A
    /// change does it by itself when the event has a handler.
    pub fn subscribe_property_changing_events(&self) {
        self.state().subscribe_changing();
    }

    /// Makes changes raise the `PropertyChanged` event (`SubscribePropertyChangedEvents`). A
    /// change does it by itself when the event has a handler.
    pub fn subscribe_property_changed_events(&self) {
        self.state().subscribe_changed();
    }

    /// Stops all change notifications, the events and the observables, until the result is
    /// disposed (`SuppressChangeNotifications`). The calls nest: the notifications resume when
    /// every result is disposed.
    pub fn suppress_change_notifications(&self) -> Rc<dyn IDisposable> {
        self.state().suppress()
    }

    /// Whether change notifications are sent (`AreChangeNotificationsEnabled`).
    pub fn are_change_notifications_enabled(&self) -> bool {
        match self.state.get() {
            Some(state) => state.notifications_enabled(),
            None => true,
        }
    }

    /// Keeps the change notifications until the result is disposed, then sends the last one of
    /// each property, in the order of those last notifications (`DelayChangeNotifications`). The
    /// calls nest.
    pub fn delay_change_notifications(&self) -> Rc<dyn IDisposable> {
        self.state().delay()
    }

    /// Whether change notifications are delayed (`AreChangeNotificationsDelayed`).
    pub fn are_change_notifications_delayed(&self) -> bool {
        match self.state.get() {
            Some(state) => state.are_change_notifications_delayed(),
            None => false,
        }
    }

    /// The errors of the subscribers of the change observables (`ThrownExceptions`); the same
    /// observable at every call.
    pub fn thrown_exceptions(&self) -> Rc<dyn IObservable<ObservableError>> {
        if let Some(cached) = self.thrown_exceptions.borrow().clone() {
            return cached;
        }
        let created = self.state().thrown_exceptions();
        *self.thrown_exceptions.borrow_mut() = Some(created.clone());
        created
    }

    /// The `Changing` observable of `source`, whose part this is; the same observable at every
    /// call (`Changing`).
    pub(crate) fn changing_of<T: IReactiveObject>(&self, source: &Rc<T>) -> Rc<dyn IObservable<ReactivePropertyChangedEventArgs<Rc<T>>>> {
        Self::cached(&self.changing, || IReactiveObjectExtensions::get_changing_observable(source))
    }

    /// The `Changed` observable of `source`, whose part this is; the same observable at every
    /// call (`Changed`).
    pub(crate) fn changed_of<T: IReactiveObject>(&self, source: &Rc<T>) -> Rc<dyn IObservable<ReactivePropertyChangedEventArgs<Rc<T>>>> {
        Self::cached(&self.changed, || IReactiveObjectExtensions::get_changed_observable(source))
    }

    fn cached<O: Clone + 'static>(cache: &RefCell<Option<Rc<dyn Any>>>, create: impl FnOnce() -> O) -> O {
        let hit = cache.borrow().as_ref().and_then(|cached| cached.downcast_ref::<O>().cloned());
        if let Some(hit) = hit {
            return hit;
        }
        let created = create();
        *cache.borrow_mut() = Some(Rc::new(created.clone()));
        created
    }
}

/// Makes a type that holds a [`ReactiveObject`] a reactive object: writes its
/// `INotifyPropertyChanged` (with `as_notify_property_changing`), `INotifyPropertyChanging`,
/// `IEnableLogger`, [`IReactiveObject`](crate::IReactiveObject) and
/// [`IHandleObservableErrors`](crate::IHandleObservableErrors).
///
/// `reactive_object!(Type)` uses the field `reactive`; `reactive_object!(Type, field)` names the
/// field. The field is a [`ReactiveObject`] or a [`ReactiveRecord`](crate::ReactiveRecord). The type
/// is not generic.
///
/// The impls cannot be blanket impls over a trait: Rust's orphan rule forbids implementing the
/// traits of `ferroui-system` for every type that holds the part, so a macro writes them, as a
/// derived base class gets them upstream.
///
/// ```
/// use ferroui_reactiveui::{reactive_object, IReactiveObject, ReactiveObject};
/// use ferroui_system::component_model::INotifyPropertyChanged;
/// use std::cell::RefCell;
/// use std::rc::Rc;
///
/// struct Model {
///     state: ReactiveObject,
/// }
/// reactive_object!(Model, state);
///
/// let model = Model { state: ReactiveObject::new() };
/// let raised = Rc::new(RefCell::new(Vec::<String>::new()));
/// let log = raised.clone();
/// model.property_changed().add(Rc::new(move |name: &str| log.borrow_mut().push(name.to_owned())));
/// model.raise_property_changed("Title");
/// assert_eq!(*raised.borrow(), vec!["Title".to_owned()]);
/// assert!(model.as_notify_property_changing().is_some());
/// ```
#[macro_export]
macro_rules! reactive_object {
    ($type_:ty) => {
        $crate::reactive_object!($type_, reactive);
    };
    ($type_:ty, $field:ident) => {
        impl $crate::__private::INotifyPropertyChanged for $type_ {
            fn property_changed(&self) -> &$crate::__private::Event<str> {
                ::core::convert::AsRef::<$crate::ReactiveObject>::as_ref(&self.$field).property_changed()
            }

            fn as_notify_property_changing(&self) -> ::core::option::Option<&dyn $crate::__private::INotifyPropertyChanging> {
                ::core::option::Option::Some(self)
            }
        }

        impl $crate::__private::INotifyPropertyChanging for $type_ {
            fn property_changing(&self) -> &$crate::__private::Event<str> {
                ::core::convert::AsRef::<$crate::ReactiveObject>::as_ref(&self.$field).property_changing()
            }
        }

        impl $crate::__private::IEnableLogger for $type_ {}

        impl $crate::IReactiveObject for $type_ {
            fn reactive_object(&self) -> &$crate::ReactiveObject {
                ::core::convert::AsRef::<$crate::ReactiveObject>::as_ref(&self.$field)
            }
        }

        impl $crate::IHandleObservableErrors for $type_ {
            fn thrown_exceptions(
                &self,
            ) -> ::std::rc::Rc<dyn $crate::__private::IObservable<$crate::__private::ObservableError>> {
                ::core::convert::AsRef::<$crate::ReactiveObject>::as_ref(&self.$field).thrown_exceptions()
            }
        }
    };
}
