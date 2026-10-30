// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{collections::ReadOnlyListHandle, BoxedValue};
use ferroui_reactiveui::ICommand;
use std::{cell::RefCell, rc::Rc};
/// Original command bar item.
pub struct DockCommandBarItem {
    header: RefCell<Option<BoxedValue>>,
    icon: RefCell<Option<BoxedValue>>,
    command: RefCell<Option<Rc<dyn ICommand>>>,
    command_parameter: RefCell<Option<BoxedValue>>,
    items: RefCell<Option<ReadOnlyListHandle<Rc<DockCommandBarItem>>>>,
    group_id: RefCell<Option<String>>,
    order: RefCell<i32>,
    is_separator: RefCell<bool>,
}
impl PartialEq for DockCommandBarItem {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl DockCommandBarItem {
    /// Constructs the original model with source defaults.
    pub fn new(header: Option<BoxedValue>) -> Rc<Self> {
        Rc::new(Self {
            header: RefCell::new(header),
            icon: RefCell::new(None),
            command: RefCell::new(None),
            command_parameter: RefCell::new(None),
            items: RefCell::new(None),
            group_id: RefCell::new(None),
            order: RefCell::new(0),
            is_separator: RefCell::new(false),
        })
    }
    /// Gets `Header`.
    pub fn header(&self) -> Option<BoxedValue> {
        self.header.borrow().clone()
    }
    /// Sets `Header` without changing notification semantics.
    pub fn set_header(&self, value: Option<BoxedValue>) {
        self.header.replace(value);
    }
    /// Gets `Icon`.
    pub fn icon(&self) -> Option<BoxedValue> {
        self.icon.borrow().clone()
    }
    /// Sets `Icon` without changing notification semantics.
    pub fn set_icon(&self, value: Option<BoxedValue>) {
        self.icon.replace(value);
    }
    /// Gets `Command`.
    pub fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.command.borrow().clone()
    }
    /// Sets `Command` without changing notification semantics.
    pub fn set_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.command.replace(value);
    }
    /// Gets `CommandParameter`.
    pub fn command_parameter(&self) -> Option<BoxedValue> {
        self.command_parameter.borrow().clone()
    }
    /// Sets `CommandParameter` without changing notification semantics.
    pub fn set_command_parameter(&self, value: Option<BoxedValue>) {
        self.command_parameter.replace(value);
    }
    /// Gets `Items`.
    pub fn items(&self) -> Option<ReadOnlyListHandle<Rc<DockCommandBarItem>>> {
        self.items.borrow().clone()
    }
    /// Sets `Items` without changing notification semantics.
    pub fn set_items(&self, value: Option<ReadOnlyListHandle<Rc<DockCommandBarItem>>>) {
        self.items.replace(value);
    }
    /// Gets `GroupId`.
    pub fn group_id(&self) -> Option<String> {
        self.group_id.borrow().clone()
    }
    /// Sets `GroupId` without changing notification semantics.
    pub fn set_group_id(&self, value: Option<String>) {
        self.group_id.replace(value);
    }
    /// Gets `Order`.
    pub fn order(&self) -> i32 {
        *self.order.borrow()
    }
    /// Sets `Order` without changing notification semantics.
    pub fn set_order(&self, value: i32) {
        self.order.replace(value);
    }
    /// Gets `IsSeparator`.
    pub fn is_separator(&self) -> bool {
        *self.is_separator.borrow()
    }
    /// Sets `IsSeparator` without changing notification semantics.
    pub fn set_is_separator(&self, value: bool) {
        self.is_separator.replace(value);
    }
}
