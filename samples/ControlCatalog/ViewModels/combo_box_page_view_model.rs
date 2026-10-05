//! Port of `ViewModels/ComboBoxPageViewModel.cs`.

use ferroui_base::data::model::{BindableList, Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_controls::ItemsSource;
use mini_mvvm::ViewModelBase;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The view model of the combo box page.
pub struct ComboBoxPageViewModel {
    base: ViewModelBase,
    wrap_selection: Cell<bool>,
    text_value: RefCell<String>,
    selected_item: RefCell<Option<Rc<IdAndName>>>,
    values: RefCell<Rc<BindableList<Rc<IdAndName>>>>,
}

impl PartialEq for ComboBoxPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for ComboBoxPageViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl ComboBoxPageViewModel {
    pub fn new() -> Rc<ComboBoxPageViewModel> {
        let item = |id: &str, name: &str, search_text: &str| {
            let item = IdAndName::new();
            item.set_id(Some(id.to_string()));
            item.set_name(Some(name.to_string()));
            item.set_search_text(Some(search_text.to_string()));
            item
        };
        Rc::new(Self {
            base: ViewModelBase::new(),
            wrap_selection: Cell::new(false),
            text_value: RefCell::new(String::new()),
            selected_item: RefCell::new(None),
            values: RefCell::new(BindableList::new([
                item("Id 1", "Name 1", "A"),
                item("Id 2", "Name 2", "B"),
                item("Id 3", "Name 3", "C"),
                item("Id 4", "Name 4", "D"),
                item("Id 5", "Name 5", "E"),
            ])),
        })
    }

    pub fn wrap_selection(&self) -> bool {
        self.wrap_selection.get()
    }

    pub fn set_wrap_selection(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.wrap_selection, value, "WrapSelection");
    }

    pub fn text_value(&self) -> String {
        self.text_value.borrow().clone()
    }

    pub fn set_text_value(&self, value: String) {
        self.base.raise_and_set_if_changed(&self.text_value, value, "TextValue");
    }

    pub fn selected_item(&self) -> Option<Rc<IdAndName>> {
        self.selected_item.borrow().clone()
    }

    pub fn set_selected_item(&self, value: Option<Rc<IdAndName>>) {
        self.base.raise_and_set_if_changed(&self.selected_item, value, "SelectedItem");
    }

    pub fn values(&self) -> Rc<BindableList<Rc<IdAndName>>> {
        self.values.borrow().clone()
    }

    pub fn set_values(&self, value: Rc<BindableList<Rc<IdAndName>>>) {
        *self.values.borrow_mut() = value;
    }
}

ferro_markup_type!(class ComboBoxPageViewModel {
    this: Rc<ComboBoxPageViewModel>,
    handles: [ComboBoxPageViewModel, Rc<ComboBoxPageViewModel>, Option<Rc<ComboBoxPageViewModel>>],
    constructors: [() => ComboBoxPageViewModel::new],
    properties: [
        WrapSelection: bool {
            get: |this: &Rc<ComboBoxPageViewModel>| this.wrap_selection(),
            set: |this: &Rc<ComboBoxPageViewModel>, value: bool| this.set_wrap_selection(value)
        },
        TextValue: String {
            get: |this: &Rc<ComboBoxPageViewModel>| this.text_value(),
            set: |this: &Rc<ComboBoxPageViewModel>, value: String| this.set_text_value(value)
        },
        SelectedItem: Option<Rc<IdAndName>> {
            get: |this: &Rc<ComboBoxPageViewModel>| this.selected_item(),
            set: |this: &Rc<ComboBoxPageViewModel>, value: Option<Rc<IdAndName>>| this.set_selected_item(value)
        },
        // A list a binding delivers to an items source property.
        Values: ItemsSource { get: |this: &Rc<ComboBoxPageViewModel>| ItemsSource::from(this.values()) },
    ],
    notify_property_changed: ComboBoxPageViewModel,
});

/// An item of the combo boxes of the page.
#[derive(Default)]
pub struct IdAndName {
    id: RefCell<Option<String>>,
    name: RefCell<Option<String>>,
    search_text: RefCell<Option<String>>,
}

impl PartialEq for IdAndName {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl IdAndName {
    pub fn new() -> Rc<IdAndName> {
        Rc::new(Self::default())
    }

    pub fn id(&self) -> Option<String> {
        self.id.borrow().clone()
    }

    pub fn set_id(&self, value: Option<String>) {
        *self.id.borrow_mut() = value;
    }

    pub fn name(&self) -> Option<String> {
        self.name.borrow().clone()
    }

    pub fn set_name(&self, value: Option<String>) {
        *self.name.borrow_mut() = value;
    }

    pub fn search_text(&self) -> Option<String> {
        self.search_text.borrow().clone()
    }

    pub fn set_search_text(&self, value: Option<String>) {
        *self.search_text.borrow_mut() = value;
    }
}

ferro_markup_type!(class IdAndName {
    this: Rc<IdAndName>,
    handles: [IdAndName, Rc<IdAndName>, Option<Rc<IdAndName>>],
    constructors: [() => IdAndName::new],
    properties: [
        Id: Option<String> {
            get: |this: &Rc<IdAndName>| this.id(),
            set: |this: &Rc<IdAndName>, value: Option<String>| this.set_id(value)
        },
        Name: Option<String> {
            get: |this: &Rc<IdAndName>| this.name(),
            set: |this: &Rc<IdAndName>, value: Option<String>| this.set_name(value)
        },
        SearchText: Option<String> {
            get: |this: &Rc<IdAndName>| this.search_text(),
            set: |this: &Rc<IdAndName>, value: Option<String>| this.set_search_text(value)
        },
    ],
});


#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn the_values_are_the_five_items_of_the_page() {
        let view_model = ComboBoxPageViewModel::new();
        let values = view_model.values().items().to_vec();
        assert_eq!(5, values.len());
        assert_eq!(Some("Id 3".to_string()), values[2].id());
        assert_eq!(Some("Name 3".to_string()), values[2].name());
        assert_eq!(Some("C".to_string()), values[2].search_text());
    }

    #[test]
    fn the_setters_notify_when_the_value_changes() {
        let view_model = ComboBoxPageViewModel::new();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = seen.clone();
        view_model.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));

        view_model.set_wrap_selection(true);
        view_model.set_wrap_selection(true);
        view_model.set_text_value("abc".to_string());
        let item = view_model.values().items().get(0);
        view_model.set_selected_item(Some(item.clone()));
        view_model.set_selected_item(Some(item));
        view_model.set_selected_item(None);
        assert_eq!(vec!["WrapSelection", "TextValue", "SelectedItem", "SelectedItem"], *seen.borrow());
        assert_eq!("abc", view_model.text_value());
    }
}
