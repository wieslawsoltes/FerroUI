use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use std::cell::{Cell, RefCell};

/// The base of the view models of the dialogs: the property change
/// notification and the helpers that raise it.
///
/// A view model embeds it (the base class of the managed original) and
/// forwards [`INotifyPropertyChanged`] to it. The name of the property is
/// passed explicitly where the managed original takes the name of the
/// calling member.
#[derive(Default)]
pub struct FerroDialogsInternalViewModelBase {
    property_changed: Event<str>,
}

impl INotifyPropertyChanged for FerroDialogsInternalViewModelBase {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl FerroDialogsInternalViewModelBase {
    pub fn new() -> Self {
        Self::default()
    }

    /// `RaiseAndSetIfChanged(ref field, value)` for a field of a type that
    /// is not `Copy`: stores `value` and raises the notification of
    /// `property_name` when it differs from the stored value. Returns
    /// whether it did.
    pub fn raise_and_set_if_changed<T: PartialEq>(&self, field: &RefCell<T>, value: T, property_name: &str) -> bool {
        if *field.borrow() != value {
            // The borrow ends before the handlers run: they read the property.
            *field.borrow_mut() = value;
            self.property_changed.raise(property_name);
            return true;
        }

        false
    }

    /// `RaiseAndSetIfChanged(ref field, value)` for a `Copy` field.
    pub fn raise_and_set_if_changed_cell<T: PartialEq + Copy>(&self, field: &Cell<T>, value: T, property_name: &str) -> bool {
        if field.get() != value {
            field.set(value);
            self.property_changed.raise(property_name);
            return true;
        }

        false
    }

    /// `RaisePropertyChanged(propertyName)`.
    pub fn raise_property_changed(&self, property_name: &str) {
        self.property_changed.raise(property_name);
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the upstream project has no tests.
    use super::*;
    use std::rc::Rc;

    fn record(base: &FerroDialogsInternalViewModelBase) -> Rc<RefCell<Vec<String>>> {
        let raised = Rc::new(RefCell::new(Vec::new()));
        let sink = raised.clone();
        base.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));
        raised
    }

    #[test]
    fn raise_and_set_if_changed_stores_and_notifies_only_on_change() {
        let base = FerroDialogsInternalViewModelBase::new();
        let raised = record(&base);
        let field = RefCell::new(String::from("a"));
        let count = Cell::new(1);

        assert!(!base.raise_and_set_if_changed(&field, String::from("a"), "Text"));
        assert!(base.raise_and_set_if_changed(&field, String::from("b"), "Text"));
        assert!(!base.raise_and_set_if_changed_cell(&count, 1, "Count"));
        assert!(base.raise_and_set_if_changed_cell(&count, 2, "Count"));
        base.raise_property_changed("Other");

        assert_eq!("b", *field.borrow());
        assert_eq!(2, count.get());
        assert_eq!(vec!["Text", "Count", "Other"], *raised.borrow());
    }
}
