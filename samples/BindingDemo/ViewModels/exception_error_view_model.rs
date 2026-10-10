//! Port of `ViewModels/ExceptionErrorViewModel.cs`.

use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use mini_mvvm::ViewModelBase;
use std::cell::Cell;
use std::rc::Rc;

pub struct ExceptionErrorViewModel {
    base: ViewModelBase,
    less_than_10: Cell<i32>,
}

impl PartialEq for ExceptionErrorViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for ExceptionErrorViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl ExceptionErrorViewModel {
    pub fn new() -> Rc<ExceptionErrorViewModel> {
        Rc::new(Self { base: ViewModelBase::new(), less_than_10: Cell::new(0) })
    }

    pub fn less_than_10(&self) -> i32 {
        self.less_than_10.get()
    }

    /// The setter fails for a value of ten or more (the `ArgumentOutOfRangeException` of the
    /// managed original).
    pub fn set_less_than_10(&self, value: i32) -> Result<(), String> {
        if value < 10 {
            self.base.raise_and_set_if_changed_cell(&self.less_than_10, value, "LessThan10");
            Ok(())
        } else {
            Err("Value must be less than 10. (Parameter 'value')".to_string())
        }
    }
}

ferro_markup_type!(class ExceptionErrorViewModel {
    this: Rc<ExceptionErrorViewModel>,
    handles: [ExceptionErrorViewModel, Rc<ExceptionErrorViewModel>, Option<Rc<ExceptionErrorViewModel>>],
    constructors: [() => ExceptionErrorViewModel::new],
    properties: [
        LessThan10: i32 {
            get: |this: &Rc<ExceptionErrorViewModel>| this.less_than_10(),
            try_set: |this: &Rc<ExceptionErrorViewModel>, value: i32| this.set_less_than_10(value)
        },
    ],
    notify_property_changed: ExceptionErrorViewModel,
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn the_setter_rejects_a_value_of_ten_or_more() {
        let view_model = ExceptionErrorViewModel::new();
        assert_eq!(Ok(()), view_model.set_less_than_10(9));
        assert_eq!(9, view_model.less_than_10());
        assert_eq!(Err("Value must be less than 10. (Parameter 'value')".to_string()), view_model.set_less_than_10(10));
        assert_eq!(9, view_model.less_than_10());
    }
}
