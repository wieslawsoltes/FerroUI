// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{DockCommandBarItem, DockCommandBarKind, DockCommandBarMergeMode};
use crate::{collections::ReadOnlyListHandle, BoxedValue};
use std::{cell::RefCell, rc::Rc};
/// Original command bar definition.
pub struct DockCommandBarDefinition {
    id: String,
    kind: DockCommandBarKind,
    merge_mode: RefCell<DockCommandBarMergeMode>,
    order: RefCell<i32>,
    group_id: RefCell<Option<String>>,
    items: RefCell<Option<ReadOnlyListHandle<Rc<DockCommandBarItem>>>>,
    content: RefCell<Option<BoxedValue>>,
}
impl PartialEq for DockCommandBarDefinition {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl DockCommandBarDefinition {
    /// Constructs the original model with source defaults.
    pub fn new(id: String, kind: DockCommandBarKind) -> Rc<Self> {
        Rc::new(Self {
            id,
            kind,
            merge_mode: RefCell::new(DockCommandBarMergeMode::Replace),
            order: RefCell::new(0),
            group_id: RefCell::new(None),
            items: RefCell::new(None),
            content: RefCell::new(None),
        })
    }
    /// Gets `Id`.
    pub fn id(&self) -> String {
        self.id.clone()
    }
    /// Gets `Kind`.
    pub fn kind(&self) -> DockCommandBarKind {
        self.kind
    }
    /// Gets `MergeMode`.
    pub fn merge_mode(&self) -> DockCommandBarMergeMode {
        *self.merge_mode.borrow()
    }
    /// Sets `MergeMode` without changing notification semantics.
    pub fn set_merge_mode(&self, value: DockCommandBarMergeMode) {
        self.merge_mode.replace(value);
    }
    /// Gets `Order`.
    pub fn order(&self) -> i32 {
        *self.order.borrow()
    }
    /// Sets `Order` without changing notification semantics.
    pub fn set_order(&self, value: i32) {
        self.order.replace(value);
    }
    /// Gets `GroupId`.
    pub fn group_id(&self) -> Option<String> {
        self.group_id.borrow().clone()
    }
    /// Sets `GroupId` without changing notification semantics.
    pub fn set_group_id(&self, value: Option<String>) {
        self.group_id.replace(value);
    }
    /// Gets `Items`.
    pub fn items(&self) -> Option<ReadOnlyListHandle<Rc<DockCommandBarItem>>> {
        self.items.borrow().clone()
    }
    /// Sets `Items` without changing notification semantics.
    pub fn set_items(&self, value: Option<ReadOnlyListHandle<Rc<DockCommandBarItem>>>) {
        self.items.replace(value);
    }
    /// Gets `Content`.
    pub fn content(&self) -> Option<BoxedValue> {
        self.content.borrow().clone()
    }
    /// Sets `Content` without changing notification semantics.
    pub fn set_content(&self, value: Option<BoxedValue>) {
        self.content.replace(value);
    }
}
