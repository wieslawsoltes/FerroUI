//! Port of `ViewModels/MetricCardViewModel.cs`: the view model of a reusable metric card resolved by
//! contract.

use ferroui_base::ferro_markup_type;
use ferroui_reactiveui::{reactive_object, IReactiveObjectExt, ReactiveObject};
use std::cell::RefCell;
use std::rc::Rc;

/// The view model of a reusable metric card resolved by contract (`MetricCardViewModel`).
pub struct MetricCardViewModel {
    reactive: ReactiveObject,
    /// The card title (`Title`).
    title: String,
    /// The value text (`_value`).
    value: RefCell<String>,
    /// The caption text (`Caption`).
    caption: String,
}
reactive_object!(MetricCardViewModel);

impl PartialEq for MetricCardViewModel {
    /// The same card (reference equality, as for the objects of the reference).
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl MetricCardViewModel {
    /// A card (`MetricCardViewModel(string, string, string)`).
    pub fn new(title: &str, value: &str, caption: &str) -> Rc<Self> {
        Rc::new(Self { reactive: ReactiveObject::new(), title: title.to_owned(), value: RefCell::new(value.to_owned()), caption: caption.to_owned() })
    }

    /// The card title (`Title`).
    pub fn title(&self) -> String {
        self.title.clone()
    }

    /// The value text (`Value { get; }`).
    pub fn value(&self) -> String {
        self.value.borrow().clone()
    }

    /// Sets the value text (`Value { set; }`).
    pub fn set_value(&self, value: String) {
        let _ = self.raise_and_set_if_changed(&self.value, value, "Value");
    }

    /// The caption text (`Caption`).
    pub fn caption(&self) -> String {
        self.caption.clone()
    }
}

ferro_markup_type!(class MetricCardViewModel {
    this: Rc<MetricCardViewModel>,
    handles: [MetricCardViewModel, Rc<MetricCardViewModel>, Option<Rc<MetricCardViewModel>>],
    properties: [
        Title: String { get: |this: &Rc<MetricCardViewModel>| this.title() },
        Value: String { get: |this: &Rc<MetricCardViewModel>| this.value() },
        Caption: String { get: |this: &Rc<MetricCardViewModel>| this.caption() },
    ],
    notify_property_changed: MetricCardViewModel,
});
