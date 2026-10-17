//! `Models/NotifyingBase.cs`: the property change notification of the core's models (columns,
//! rows, sources).
//!
//! The reference's base class becomes a field: a type holds a `NotifyingBase` and implements the
//! framework's `INotifyPropertyChanged` by returning its event.

use ferroui_base::data::model::Event;

/// The `PropertyChanged` event of a model and the helpers that raise it.
#[derive(Default)]
pub struct NotifyingBase {
    property_changed: Event<str>,
}

impl NotifyingBase {
    pub fn new() -> Self {
        Self::default()
    }

    /// The event.
    pub fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }

    /// Sets `field` to `value` and raises the change of `property_name` if the value changed
    /// (`RaiseAndSetIfChanged`). The field is set before the event is raised; the caller passes a
    /// field it does not hold borrowed.
    pub fn raise_and_set_if_changed<T: PartialEq>(
        &self,
        field: &std::cell::RefCell<T>,
        value: T,
        property_name: &str,
    ) -> bool {
        if *field.borrow() != value {
            *field.borrow_mut() = value;
            self.raise_property_changed(property_name);
            true
        } else {
            false
        }
    }

    /// [`raise_and_set_if_changed`](Self::raise_and_set_if_changed) for a `Cell` of a `Copy` value.
    pub fn raise_and_set_cell_if_changed<T: PartialEq + Copy>(
        &self,
        field: &std::cell::Cell<T>,
        value: T,
        property_name: &str,
    ) -> bool {
        if field.get() != value {
            field.set(value);
            self.raise_property_changed(property_name);
            true
        } else {
            false
        }
    }

    /// Raises the change of `property_name` (an empty name: every property).
    pub fn raise_property_changed(&self, property_name: &str) {
        self.property_changed.raise(property_name);
    }
}
