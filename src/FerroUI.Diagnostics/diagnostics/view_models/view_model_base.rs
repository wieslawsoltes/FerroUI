//! Port of `Diagnostics/ViewModels/ViewModelBase.cs`.

use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The base of the view models of the developer tools: the property change
/// notification and the helpers that raise it.
///
/// A view model embeds it (the base class of the original) and forwards
/// [`INotifyPropertyChanged`] to it. The name of the property is passed
/// explicitly where the original takes the name of the calling member.
///
/// The original also records every addition and removal of a handler in a
/// list that nothing reads; the port leaves that list out.
#[derive(Default)]
pub struct ViewModelBase {
    property_changed: Event<str>,
    on_property_changed: RefCell<Option<Rc<dyn Fn(&str)>>>,
}

impl INotifyPropertyChanged for ViewModelBase {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl ViewModelBase {
    pub fn new() -> Self {
        Self::default()
    }

    /// The override of `protected virtual void OnPropertyChanged(PropertyChangedEventArgs e)`
    /// of a deriving view model: called with the name of the property before
    /// the handlers of [`property_changed`](INotifyPropertyChanged::property_changed).
    /// The base implementation does nothing. A deriving view model sets it
    /// once, when it is constructed (a closure over a weak reference to
    /// itself).
    pub fn set_on_property_changed(&self, handler: Option<Rc<dyn Fn(&str)>>) {
        *self.on_property_changed.borrow_mut() = handler;
    }

    /// `RaiseAndSetIfChanged(ref field, value)` for a field of a type that is
    /// not `Copy`: stores `value` and raises the notification of
    /// `property_name` when it differs from the stored value. Returns whether
    /// it did.
    pub fn raise_and_set_if_changed<T: PartialEq>(&self, field: &RefCell<T>, value: T, property_name: &str) -> bool {
        if *field.borrow() != value {
            // The borrow ends before the handlers run: they read the property.
            *field.borrow_mut() = value;
            self.raise_property_changed(property_name);
            return true;
        }

        false
    }

    /// `RaiseAndSetIfChanged(ref field, value)` for a `Copy` field.
    pub fn raise_and_set_if_changed_cell<T: PartialEq + Copy>(&self, field: &Cell<T>, value: T, property_name: &str) -> bool {
        if field.get() != value {
            field.set(value);
            self.raise_property_changed(property_name);
            return true;
        }

        false
    }

    /// `RaisePropertyChanged(propertyName)`: the override of
    /// `OnPropertyChanged`, then the handlers.
    pub fn raise_property_changed(&self, property_name: &str) {
        let on_property_changed = self.on_property_changed.borrow().clone();
        if let Some(on_property_changed) = on_property_changed {
            on_property_changed(property_name);
        }
        self.property_changed.raise(property_name);
    }
}
