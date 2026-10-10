//! Port of `ViewModels/ExpanderPageViewModel.cs`.

use super::ExpanderItemViewModel;
use ferroui_base::collections::FerroList;
use ferroui_base::ferro_markup_type;
use std::cell::RefCell;
use std::rc::Rc;

ferroui_controls::ferro_markup_list!(pub ExpanderItemList: Rc<ExpanderItemViewModel>);

pub struct ExpanderPageViewModel {
    items: RefCell<FerroList<Rc<ExpanderItemViewModel>>>,
}

impl PartialEq for ExpanderPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ExpanderPageViewModel {
    pub fn new() -> Rc<ExpanderPageViewModel> {
        Rc::new(Self {
            items: RefCell::new(FerroList::from_items((0..100).map(|x| {
                let item = ExpanderItemViewModel::new();
                item.set_header(Some(format!("Item {x}")));
                item
            }))),
        })
    }

    pub fn items(&self) -> FerroList<Rc<ExpanderItemViewModel>> {
        self.items.borrow().clone()
    }

    pub fn set_items(&self, value: FerroList<Rc<ExpanderItemViewModel>>) {
        *self.items.borrow_mut() = value;
    }
}

ferro_markup_type!(class ExpanderPageViewModel {
    this: Rc<ExpanderPageViewModel>,
    handles: [ExpanderPageViewModel, Rc<ExpanderPageViewModel>, Option<Rc<ExpanderPageViewModel>>],
    constructors: [() => ExpanderPageViewModel::new],
    properties: [
        Items: FerroList<Rc<ExpanderItemViewModel>> {
            get: |this: &Rc<ExpanderPageViewModel>| this.items(),
            set: |this: &Rc<ExpanderPageViewModel>, value: FerroList<Rc<ExpanderItemViewModel>>| this.set_items(value)
        },
    ],
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn the_view_model_has_a_hundred_collapsed_items() {
        let view_model = ExpanderPageViewModel::new();
        assert_eq!(100, view_model.items().count());
        assert_eq!(Some(String::from("Item 0")), view_model.items().get(0).header());
        assert_eq!(Some(String::from("Item 99")), view_model.items().get(99).header());
        assert!(view_model.items().to_vec().iter().all(|item| !item.is_expanded()));
    }
}
