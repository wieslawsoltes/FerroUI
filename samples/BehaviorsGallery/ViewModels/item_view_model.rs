//! Port of `ViewModels/ItemViewModel.cs`.

use super::items::{items_source, ItemList};
use super::ViewModelBase;
use ferroui_base::ferro_markup_type;
use ferroui_controls::ItemsSource;
use ferroui_reactiveui::reactive_object;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// An item of the lists and trees of the pages: a text, its children and a colour.
pub struct ItemViewModel {
    base: ViewModelBase,
    value: RefCell<Option<String>>,
    items: RefCell<Option<ItemList>>,
    color: RefCell<Option<String>>,
}
reactive_object!(ItemViewModel, base);
super::reactive_accessors!(ItemViewModel {
    value, set_value, "Value": Option<String>;
    items, set_items, "Items": Option<ItemList>;
    color, set_color, "Color": Option<String>;
});

impl PartialEq for ItemViewModel {
    /// A view model is a reference type: two are equal when they are the same object.
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ItemViewModel {
    /// `ItemViewModel(string value, string color = "Black")`.
    pub fn new(value: &str, color: &str) -> Rc<Self> {
        Rc::new(Self {
            base: ViewModelBase::new(),
            value: RefCell::new(Some(value.to_string())),
            items: RefCell::new(None),
            color: RefCell::new(Some(color.to_string())),
        })
    }

    /// `new ItemViewModel(value)`: the colour is black.
    pub fn with_value(value: &str) -> Rc<Self> {
        Self::new(value, "Black")
    }
}

impl fmt::Display for ItemViewModel {
    /// `ToString()`: the value.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.value.borrow().as_deref().unwrap_or_default())
    }
}

ferro_markup_type!(class ItemViewModel {
    this: Rc<ItemViewModel>,
    handles: [ItemViewModel, Rc<ItemViewModel>, Option<Rc<ItemViewModel>>],
    // `new ItemViewModel(value, color)` of the documents (`<vm:ItemViewModel>` with `x:Arguments`).
    constructors: [
        (String, String) => |value: String, color: String| ItemViewModel::new(&value, &color),
    ],
    properties: [
        Value: Option<String> {
            get: |this: &Rc<ItemViewModel>| this.value(),
            set: |this: &Rc<ItemViewModel>, value: Option<String>| this.set_value(value)
        },
        Items: Option<ItemsSource> { get: |this: &Rc<ItemViewModel>| items_source(&this.items()) },
        Color: Option<String> {
            get: |this: &Rc<ItemViewModel>| this.color(),
            set: |this: &Rc<ItemViewModel>, value: Option<String>| this.set_color(value)
        },
    ],
    notify_property_changed: ItemViewModel,
});
