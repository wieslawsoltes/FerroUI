// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the crate root.

use super::{IndexPath, SelectionError};
use ferroui_controls::ItemsSource;
use ferroui_base::BoxedValue;
use std::{cell::{Cell, RefCell}, rc::Rc};

/// Shared mutable event arguments. Source access expires when the synchronous callback ends.
pub struct SelectionModelChildrenRequestedEventArgs {
    source: RefCell<Option<BoxedValue>>,
    source_index: RefCell<Option<IndexPath>>,
    children: RefCell<Option<ItemsSource>>,
    active: Cell<bool>,
}

impl SelectionModelChildrenRequestedEventArgs {
    pub(crate) fn new(source: Option<BoxedValue>, source_index: IndexPath) -> Rc<Self> {
        Rc::new(Self {
            source: RefCell::new(source), source_index: RefCell::new(Some(source_index)),
            children: RefCell::new(None), active: Cell::new(true),
        })
    }
    pub fn source(&self) -> Result<Option<BoxedValue>, SelectionError> {
        if !self.active.get() { return Err(SelectionError::EventArgsExpired); }
        Ok(self.source.borrow().clone())
    }
    pub fn source_index(&self) -> Result<IndexPath, SelectionError> {
        if !self.active.get() { return Err(SelectionError::EventArgsExpired); }
        self.source_index.borrow().clone().ok_or(SelectionError::EventArgsExpired)
    }
    pub fn children(&self) -> Option<ItemsSource> { self.children.borrow().clone() }
    pub fn set_children(&self, children: Option<ItemsSource>) { *self.children.borrow_mut() = children; }
    pub(crate) fn expire(&self) {
        self.active.set(false);
        self.source.borrow_mut().take();
        self.source_index.borrow_mut().take();
        self.children.borrow_mut().take();
    }
}

impl PartialEq for SelectionModelChildrenRequestedEventArgs {
    fn eq(&self, other: &Self) -> bool { std::ptr::eq(self, other) }
}

