//! Port of `ViewModels/MenuItemViewModel.cs`.

use ferroui_base::data::model::BindableList;
use ferroui_base::input::ICommand;
use ferroui_base::{ferro_markup_type, BoxedValue};
use ferroui_controls::ItemsSource;
use std::cell::RefCell;
use std::rc::Rc;

/// A menu item of a generated menu.
#[derive(Default)]
pub struct MenuItemViewModel {
    header: RefCell<Option<String>>,
    command: RefCell<Option<Rc<dyn ICommand>>>,
    command_parameter: RefCell<Option<BoxedValue>>,
    items: RefCell<Option<Rc<BindableList<Rc<MenuItemViewModel>>>>>,
}

impl PartialEq for MenuItemViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl MenuItemViewModel {
    pub fn new() -> Rc<MenuItemViewModel> {
        Rc::new(Self::default())
    }

    pub fn header(&self) -> Option<String> {
        self.header.borrow().clone()
    }

    pub fn set_header(&self, value: Option<String>) {
        *self.header.borrow_mut() = value;
    }

    pub fn with_header(self: Rc<Self>, value: &str) -> Rc<Self> {
        self.set_header(Some(value.to_string()));
        self
    }

    pub fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.command.borrow().clone()
    }

    pub fn set_command(&self, value: Option<Rc<dyn ICommand>>) {
        *self.command.borrow_mut() = value;
    }

    pub fn with_command(self: Rc<Self>, value: Rc<dyn ICommand>) -> Rc<Self> {
        self.set_command(Some(value));
        self
    }

    pub fn command_parameter(&self) -> Option<BoxedValue> {
        self.command_parameter.borrow().clone()
    }

    pub fn set_command_parameter(&self, value: Option<BoxedValue>) {
        *self.command_parameter.borrow_mut() = value;
    }

    pub fn with_command_parameter(self: Rc<Self>, value: BoxedValue) -> Rc<Self> {
        self.set_command_parameter(Some(value));
        self
    }

    pub fn items(&self) -> Option<Rc<BindableList<Rc<MenuItemViewModel>>>> {
        self.items.borrow().clone()
    }

    pub fn set_items(&self, value: Option<Rc<BindableList<Rc<MenuItemViewModel>>>>) {
        *self.items.borrow_mut() = value;
    }

    pub fn with_items(self: Rc<Self>, value: Rc<BindableList<Rc<MenuItemViewModel>>>) -> Rc<Self> {
        self.set_items(Some(value));
        self
    }
}

ferro_markup_type!(class MenuItemViewModel {
    this: Rc<MenuItemViewModel>,
    handles: [MenuItemViewModel, Rc<MenuItemViewModel>, Option<Rc<MenuItemViewModel>>],
    constructors: [() => MenuItemViewModel::new],
    properties: [
        Header: Option<String> {
            get: |this: &Rc<MenuItemViewModel>| this.header(),
            set: |this: &Rc<MenuItemViewModel>, value: Option<String>| this.set_header(value)
        },
        Command: Option<Rc<dyn ICommand>> {
            get: |this: &Rc<MenuItemViewModel>| this.command(),
            set: |this: &Rc<MenuItemViewModel>, value: Option<Rc<dyn ICommand>>| this.set_command(value)
        },
        CommandParameter: Option<BoxedValue> {
            get: |this: &Rc<MenuItemViewModel>| this.command_parameter(),
            set: |this: &Rc<MenuItemViewModel>, value: Option<BoxedValue>| this.set_command_parameter(value)
        },
        // A list a binding delivers to an items source property.
        Items: Option<ItemsSource> { get: |this: &Rc<MenuItemViewModel>| this.items().map(ItemsSource::from) },
    ],
});

