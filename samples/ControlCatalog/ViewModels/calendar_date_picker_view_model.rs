//! Port of `ViewModels/CalenderDatePickerViewModel.cs` (upstream e61863dff7): the page has a view
//! model of its own, so its bindings no longer depend on the data context of the main window.
//!
//! Not ported, because what it names does not exist in the framework yet: the `[Required]`
//! validation attribute of `ValidatedDateExample` (data annotations).

use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_base::utilities::DateTime;
use mini_mvvm::ViewModelBase;
use std::cell::Cell;
use std::rc::Rc;

/// The view model of the calendar date picker page.
pub struct CalendarDatePickerViewModel {
    base: ViewModelBase,
    validated_date_example: Cell<Option<DateTime>>,
}

impl PartialEq for CalendarDatePickerViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for CalendarDatePickerViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl CalendarDatePickerViewModel {
    pub fn new() -> Rc<CalendarDatePickerViewModel> {
        Rc::new(Self { base: ViewModelBase::new(), validated_date_example: Cell::new(None) })
    }

    /// A required date which demonstrates validation for the date picker.
    pub fn validated_date_example(&self) -> Option<DateTime> {
        self.validated_date_example.get()
    }

    pub fn set_validated_date_example(&self, value: Option<DateTime>) {
        self.base.raise_and_set_if_changed_cell(&self.validated_date_example, value, "ValidatedDateExample");
    }
}

ferro_markup_type!(class CalendarDatePickerViewModel {
    this: Rc<CalendarDatePickerViewModel>,
    handles: [CalendarDatePickerViewModel, Rc<CalendarDatePickerViewModel>, Option<Rc<CalendarDatePickerViewModel>>],
    constructors: [() => CalendarDatePickerViewModel::new],
    properties: [
        ValidatedDateExample: Option<DateTime> {
            get: |this: &Rc<CalendarDatePickerViewModel>| this.validated_date_example(),
            set: |this: &Rc<CalendarDatePickerViewModel>, value: Option<DateTime>| {
                this.set_validated_date_example(value)
            }
        },
    ],
    notify_property_changed: CalendarDatePickerViewModel,
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn the_validated_date_raises_its_change() {
        let view_model = CalendarDatePickerViewModel::new();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = seen.clone();
        view_model.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));
        view_model.set_validated_date_example(Some(DateTime::today()));
        view_model.set_validated_date_example(Some(DateTime::today()));
        assert_eq!(vec!["ValidatedDateExample"], *seen.borrow());
    }
}
