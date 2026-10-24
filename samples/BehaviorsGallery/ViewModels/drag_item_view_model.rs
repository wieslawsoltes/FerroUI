//! Port of `ViewModels/DragItemViewModel.cs`.

use super::ViewModelBase;
use ferroui_base::ferro_markup_type;
use ferroui_reactiveui::reactive_object;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// An item of the drag and drop lists: a title.
#[derive(Default)]
pub struct DragItemViewModel {
    base: ViewModelBase,
    title: RefCell<Option<String>>,
}
reactive_object!(DragItemViewModel, base);
super::reactive_accessors!(DragItemViewModel {
    title, set_title, "Title": Option<String>;
});

impl PartialEq for DragItemViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl DragItemViewModel {
    /// `new DragItemViewModel { Title = title }`.
    pub fn new(title: &str) -> Rc<Self> {
        Rc::new(Self {
            base: ViewModelBase::new(),
            title: RefCell::new(Some(title.to_string())),
        })
    }
}

impl fmt::Display for DragItemViewModel {
    /// `ToString()`: the title.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.title.borrow().as_deref().unwrap_or_default())
    }
}

ferro_markup_type!(class DragItemViewModel {
    this: Rc<DragItemViewModel>,
    handles: [DragItemViewModel, Rc<DragItemViewModel>, Option<Rc<DragItemViewModel>>],
    constructors: [() => || Rc::new(DragItemViewModel::default())],
    properties: [
        Title: Option<String> {
            get: |this: &Rc<DragItemViewModel>| this.title(),
            set: |this: &Rc<DragItemViewModel>, value: Option<String>| this.set_title(value)
        },
    ],
    notify_property_changed: DragItemViewModel,
});
