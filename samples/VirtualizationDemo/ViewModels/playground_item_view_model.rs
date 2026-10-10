//! Port of `ViewModels/PlaygroundItemViewModel.cs`.

use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use mini_mvvm::ViewModelBase;
use std::cell::RefCell;
use std::rc::Rc;

pub struct PlaygroundItemViewModel {
    base: ViewModelBase,
    header: RefCell<Option<String>>,
}

impl PartialEq for PlaygroundItemViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for PlaygroundItemViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl PlaygroundItemViewModel {
    /// `PlaygroundItemViewModel(int index)`.
    pub fn new(index: i32) -> Rc<PlaygroundItemViewModel> {
        Self::with_header(Some(format!("Item {index}")))
    }

    /// `PlaygroundItemViewModel(string? header)`.
    pub fn with_header(header: Option<String>) -> Rc<PlaygroundItemViewModel> {
        let this = Rc::new(Self { base: ViewModelBase::new(), header: RefCell::new(None) });
        this.set_header(header);
        this
    }

    pub fn header(&self) -> Option<String> {
        self.header.borrow().clone()
    }

    pub fn set_header(&self, value: Option<String>) {
        self.base.raise_and_set_if_changed(&self.header, value, "Header");
    }
}

ferro_markup_type!(class PlaygroundItemViewModel {
    this: Rc<PlaygroundItemViewModel>,
    handles: [PlaygroundItemViewModel, Rc<PlaygroundItemViewModel>, Option<Rc<PlaygroundItemViewModel>>],
    constructors: [
        (i32) => PlaygroundItemViewModel::new,
        (Option<String>) => PlaygroundItemViewModel::with_header,
    ],
    properties: [
        Header: Option<String> {
            get: |this: &Rc<PlaygroundItemViewModel>| this.header(),
            set: |this: &Rc<PlaygroundItemViewModel>, value: Option<String>| this.set_header(value)
        },
    ],
    notify_property_changed: PlaygroundItemViewModel,
});
