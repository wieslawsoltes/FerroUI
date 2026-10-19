//! `AbstractNotifyPropertyChanged` (`Binding/AbstractNotifyPropertyChanged.cs`): the base of the
//! objects that raise `PropertyChanged`, as a part that a type embeds.

use ferroui_reactive::disposables::Disposable;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// A backing field that [`AbstractNotifyPropertyChanged::set_and_raise`] reads and assigns: a
/// `Cell` for a value that is `Copy`, a `RefCell` for the others (`ref T backingField`).
pub trait BackingField<T> {
    /// Whether the value of the field equals `value` by `comparer`.
    fn equals(&self, value: &T, comparer: &dyn Fn(&T, &T) -> bool) -> bool;
    /// Assigns `value` to the field.
    fn store(&self, value: T);
}

impl<T: Copy> BackingField<T> for Cell<T> {
    fn equals(&self, value: &T, comparer: &dyn Fn(&T, &T) -> bool) -> bool {
        comparer(&self.get(), value)
    }

    fn store(&self, value: T) {
        self.set(value);
    }
}

impl<T> BackingField<T> for RefCell<T> {
    fn equals(&self, value: &T, comparer: &dyn Fn(&T, &T) -> bool) -> bool {
        comparer(&self.borrow(), value)
    }

    fn store(&self, value: T) {
        *self.borrow_mut() = value;
    }
}

struct Inner {
    property_changed: Event<str>,
    is_suspended: Cell<bool>,
}

/// The base of an object that notifies the changes of its properties
/// (`AbstractNotifyPropertyChanged`).
///
/// Deviation (DEVIATIONS.md, DynamicData): upstream's abstract class is a part that a type embeds;
/// the type implements `INotifyPropertyChanged` by returning [`property_changed`](Self::property_changed),
/// and its setters call [`set_and_raise`](Self::set_and_raise) with the property name that
/// `[CallerMemberName]` gives upstream (the PascalCase name of the getter, which `prop!` derives).
///
/// ```
/// use ferroui_dynamic_data::binding::AbstractNotifyPropertyChanged;
/// use ferroui_system::component_model::INotifyPropertyChanged;
/// use ferroui_system::events::Event;
/// use std::cell::Cell;
///
/// #[derive(Default)]
/// struct Item { notify: AbstractNotifyPropertyChanged, age: Cell<i32> }
/// impl Item {
///     fn age(&self) -> i32 { self.age.get() }
///     fn set_age(&self, value: i32) { self.notify.set_and_raise(&self.age, value, "Age") }
/// }
/// impl INotifyPropertyChanged for Item {
///     fn property_changed(&self) -> &Event<str> { self.notify.property_changed() }
/// }
///
/// let item = Item::default();
/// item.set_age(3);
/// assert_eq!(item.age(), 3);
/// ```
pub struct AbstractNotifyPropertyChanged {
    inner: Rc<Inner>,
}

impl Default for AbstractNotifyPropertyChanged {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for AbstractNotifyPropertyChanged {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AbstractNotifyPropertyChanged")
            .field("handlers", &self.inner.property_changed.handler_count())
            .field("is_suspended", &self.inner.is_suspended.get())
            .finish()
    }
}

impl AbstractNotifyPropertyChanged {
    /// A part without handlers.
    pub fn new() -> Self {
        Self { inner: Rc::new(Inner { property_changed: Event::new(), is_suspended: Cell::new(false) }) }
    }

    /// The event raised when a property value has changed (`PropertyChanged`).
    pub fn property_changed(&self) -> &Event<str> {
        &self.inner.property_changed
    }

    /// Raises `PropertyChanged` for `property_name`; an empty name means every property
    /// (`OnPropertyChanged(string? propertyName)`, where `null` is every property). Nothing is
    /// raised while the notifications are suspended.
    pub fn on_property_changed(&self, property_name: &str) {
        Self::raise(&self.inner, property_name);
    }

    fn raise(inner: &Inner, property_name: &str) {
        if inner.is_suspended.get() {
            return;
        }
        inner.property_changed.raise(property_name);
    }

    /// Assigns `value` to `field` and raises `PropertyChanged` for `property_name`, unless the
    /// field already holds an equal value (`SetAndRaise<T>(ref T, T, string?)`).
    pub fn set_and_raise<T: PartialEq>(&self, field: &impl BackingField<T>, value: T, property_name: &str) {
        self.set_and_raise_with(field, value, |current, value| current == value, property_name);
    }

    /// [`set_and_raise`](Self::set_and_raise) with an equality comparer
    /// (`SetAndRaise<T>(ref T, T, IEqualityComparer<T>?, string?)`).
    pub fn set_and_raise_with<T>(&self, field: &impl BackingField<T>, value: T, comparer: impl Fn(&T, &T) -> bool, property_name: &str) {
        if field.equals(&value, &comparer) {
            return;
        }
        field.store(value);
        self.on_property_changed(property_name);
    }

    /// Suspends the notifications until the result is disposed; then, when
    /// `invoke_property_changed_event_when_finished` is true, raises `PropertyChanged` for every
    /// property (`SuspendNotifications(bool invokePropertyChangeEventWhenDisposed = true)`).
    ///
    /// Deviation (DEVIATIONS.md, DynamicData): the reference checkout no longer has this member;
    /// the earlier API marked it obsolete ("This never worked properly in the first place"). It is
    /// ported from that API: the suspension is one flag, so the first disposal of nested
    /// suspensions ends it.
    pub fn suspend_notifications(&self, invoke_property_changed_event_when_finished: bool) -> Rc<dyn IDisposable> {
        self.inner.is_suspended.set(true);
        let inner = self.inner.clone();
        Disposable::create(move || {
            inner.is_suspended.set(false);
            if invoke_property_changed_event_when_finished {
                Self::raise(&inner, "");
            }
        })
    }
}

impl INotifyPropertyChanged for AbstractNotifyPropertyChanged {
    fn property_changed(&self) -> &Event<str> {
        &self.inner.property_changed
    }
}
