//! Port of `ViewModelBase.cs`.

use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use std::cell::{Cell, RefCell};

/// The base of a view model: the property change notification and the
/// helpers that raise it.
///
/// A view model embeds it (the base class of the managed original) and
/// forwards [`INotifyPropertyChanged`] to it:
///
/// ```ignore
/// pub struct Person { base: ViewModelBase, name: RefCell<String> }
///
/// impl INotifyPropertyChanged for Person {
///     fn property_changed(&self) -> &Event<str> { self.base.property_changed() }
/// }
///
/// impl Person {
///     pub fn set_name(&self, value: String) { self.base.raise_and_set_if_changed(&self.name, value, "Name"); }
/// }
/// ```
///
/// The name of the property is passed explicitly where the managed original
/// takes the name of the calling member.
#[derive(Default)]
pub struct ViewModelBase {
    property_changed: Event<str>,
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

    /// `RaiseAndSetIfChanged(ref field, value)` for a field of a type that
    /// is not `Copy`: stores `value` and raises the notification of
    /// `property_name` when it differs from the stored value. Returns
    /// whether it did.
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

    /// `RaisePropertyChanged(propertyName)`.
    pub fn raise_property_changed(&self, property_name: &str) {
        self.property_changed.raise(property_name);
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream library has no tests.
    use super::*;
    use std::rc::Rc;

    #[test]
    fn raise_and_set_if_changed_stores_and_notifies_once() {
        let base = ViewModelBase::new();
        let field = RefCell::new(String::from("a"));
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = seen.clone();
        base.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));

        assert!(base.raise_and_set_if_changed(&field, String::from("b"), "Name"));
        assert!(!base.raise_and_set_if_changed(&field, String::from("b"), "Name"));
        assert_eq!("b", *field.borrow());
        assert_eq!(vec!["Name".to_string()], *seen.borrow());
    }

    #[test]
    fn raise_and_set_if_changed_cell_stores_and_notifies_once() {
        let base = ViewModelBase::new();
        let field = Cell::new(1);
        let count = Rc::new(Cell::new(0));
        let sink = count.clone();
        base.property_changed().add(Rc::new(move |_: &str| sink.set(sink.get() + 1)));

        assert!(base.raise_and_set_if_changed_cell(&field, 2, "Count"));
        assert!(!base.raise_and_set_if_changed_cell(&field, 2, "Count"));
        assert_eq!(2, field.get());
        assert_eq!(1, count.get());
    }

    #[test]
    fn the_handler_reads_the_new_value() {
        let base = Rc::new(ViewModelBase::new());
        let field = Rc::new(RefCell::new(1));
        let seen = Rc::new(Cell::new(0));
        let (sink, source) = (seen.clone(), field.clone());
        base.property_changed().add(Rc::new(move |_: &str| sink.set(*source.borrow())));

        base.raise_and_set_if_changed(&field, 5, "Value");
        assert_eq!(5, seen.get());
    }
}
