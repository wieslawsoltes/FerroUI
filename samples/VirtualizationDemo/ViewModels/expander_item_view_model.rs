//! Port of `ViewModels/ExpanderItemViewModel.cs`.

use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use mini_mvvm::ViewModelBase;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct ExpanderItemViewModel {
    base: ViewModelBase,
    header: RefCell<Option<String>>,
    is_expanded: Cell<bool>,
}

impl PartialEq for ExpanderItemViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for ExpanderItemViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl ExpanderItemViewModel {
    pub fn new() -> Rc<ExpanderItemViewModel> {
        Rc::new(Self { base: ViewModelBase::new(), header: RefCell::new(None), is_expanded: Cell::new(false) })
    }

    pub fn header(&self) -> Option<String> {
        self.header.borrow().clone()
    }

    pub fn set_header(&self, value: Option<String>) {
        self.base.raise_and_set_if_changed(&self.header, value, "Header");
    }

    pub fn is_expanded(&self) -> bool {
        self.is_expanded.get()
    }

    pub fn set_is_expanded(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.is_expanded, value, "IsExpanded");
    }
}

ferro_markup_type!(class ExpanderItemViewModel {
    this: Rc<ExpanderItemViewModel>,
    handles: [ExpanderItemViewModel, Rc<ExpanderItemViewModel>, Option<Rc<ExpanderItemViewModel>>],
    constructors: [() => ExpanderItemViewModel::new],
    properties: [
        Header: Option<String> {
            get: |this: &Rc<ExpanderItemViewModel>| this.header(),
            set: |this: &Rc<ExpanderItemViewModel>, value: Option<String>| this.set_header(value)
        },
        IsExpanded: bool {
            get: |this: &Rc<ExpanderItemViewModel>| this.is_expanded(),
            set: |this: &Rc<ExpanderItemViewModel>, value: bool| this.set_is_expanded(value)
        },
    ],
    notify_property_changed: ExpanderItemViewModel,
});
