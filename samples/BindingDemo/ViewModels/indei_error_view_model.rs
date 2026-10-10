//! Port of `ViewModels/IndeiErrorViewModel.cs`.

use ferroui_base::data::model::{Event, INotifyDataErrorInfo, INotifyPropertyChanged};
use ferroui_base::reactive::AnonymousObserver;
use ferroui_base::{ferro_markup_type, BoxedValue};
use mini_mvvm::{PropertyChangedExtensions, ViewModelBase};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct IndeiErrorViewModel {
    base: ViewModelBase,
    maximum: Cell<i32>,
    value: Cell<i32>,
    value_error: RefCell<Option<String>>,
    errors_changed: Event<str>,
}

impl PartialEq for IndeiErrorViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for IndeiErrorViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl IndeiErrorViewModel {
    pub fn new() -> Rc<IndeiErrorViewModel> {
        let this = Rc::new(Self {
            base: ViewModelBase::new(),
            maximum: Cell::new(10),
            value: Cell::new(0),
            value_error: RefCell::new(None),
            errors_changed: Event::new(),
        });

        // The managed original captures the view model in the subscription to its own
        // properties: a cycle its collector frees. Here the subscription holds it weakly.
        let weak = Rc::downgrade(&this);
        PropertyChangedExtensions::when_any_value2_tuple(
            &this,
            ("Maximum", |x: &IndeiErrorViewModel| x.maximum()),
            ("Value", |x: &IndeiErrorViewModel| x.value()),
        )
        .subscribe(Rc::new(AnonymousObserver::new(move |_: (i32, i32)| {
            if let Some(this) = weak.upgrade() {
                this.update_errors();
            }
        })));

        this
    }

    pub fn maximum(&self) -> i32 {
        self.maximum.get()
    }

    pub fn set_maximum(&self, value: i32) {
        self.base.raise_and_set_if_changed_cell(&self.maximum, value, "Maximum");
    }

    pub fn value(&self) -> i32 {
        self.value.get()
    }

    pub fn set_value(&self, value: i32) {
        self.base.raise_and_set_if_changed_cell(&self.value, value, "Value");
    }

    fn update_errors(&self) {
        if self.value() <= self.maximum() {
            if self.value_error.borrow().is_some() {
                *self.value_error.borrow_mut() = None;
                self.errors_changed.raise("Value");
            }
        } else if self.value_error.borrow().is_none() {
            *self.value_error.borrow_mut() = Some(String::from("Value must be less than Maximum"));
            self.errors_changed.raise("Value");
        }
    }
}

impl INotifyDataErrorInfo for IndeiErrorViewModel {
    /// # Panics
    /// Always: the managed original throws `NotImplementedException`. Nothing of the
    /// framework reads the property.
    fn has_errors(&self) -> bool {
        panic!("The method or operation is not implemented.")
    }

    fn get_errors(&self, property_name: Option<&str>) -> Vec<BoxedValue> {
        match property_name {
            // The managed original returns an array of one element, which is null while the
            // value has no error; the validation of a binding leaves out the nulls of the
            // enumeration, and the errors of this contract have no null.
            Some("Value") => self.value_error.borrow().iter().map(|error| Rc::new(error.clone()) as BoxedValue).collect(),
            _ => Vec::new(),
        }
    }

    fn errors_changed(&self) -> &Event<str> {
        &self.errors_changed
    }
}

ferro_markup_type!(class IndeiErrorViewModel {
    this: Rc<IndeiErrorViewModel>,
    handles: [IndeiErrorViewModel, Rc<IndeiErrorViewModel>, Option<Rc<IndeiErrorViewModel>>],
    constructors: [() => IndeiErrorViewModel::new],
    properties: [
        Maximum: i32 {
            get: |this: &Rc<IndeiErrorViewModel>| this.maximum(),
            set: |this: &Rc<IndeiErrorViewModel>, value: i32| this.set_maximum(value)
        },
        Value: i32 {
            get: |this: &Rc<IndeiErrorViewModel>| this.value(),
            set: |this: &Rc<IndeiErrorViewModel>, value: i32| this.set_value(value)
        },
    ],
    notify_property_changed: IndeiErrorViewModel,
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    fn errors(view_model: &IndeiErrorViewModel, property: &str) -> Vec<String> {
        view_model.get_errors(Some(property)).iter().filter_map(|error| error.downcast_ref::<String>().cloned()).collect()
    }

    #[test]
    fn a_value_above_the_maximum_is_an_error_until_it_is_not() {
        let view_model = IndeiErrorViewModel::new();
        let changes = Rc::new(RefCell::new(Vec::new()));
        let sink = changes.clone();
        view_model.errors_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));

        assert!(errors(&view_model, "Value").is_empty());
        view_model.set_value(11);
        assert_eq!(vec!["Value must be less than Maximum".to_string()], errors(&view_model, "Value"));
        assert!(errors(&view_model, "Maximum").is_empty());
        view_model.set_value(12);
        assert_eq!(vec!["Value".to_string()], *changes.borrow());

        view_model.set_maximum(20);
        assert!(errors(&view_model, "Value").is_empty());
        assert_eq!(vec!["Value".to_string(), "Value".to_string()], *changes.borrow());
    }
}
