// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.

use super::item_collection_transition_progress::ItemCollectionTransitionProgress;
use super::item_collection_transition_provider::ItemCollectionTransitionProvider;
use ferroui_controls::Control;
use ferroui_base::{Rect, Ref, WeakRef};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ItemCollectionTransitionOperation {
    Add = 0,
    Remove = 1,
    Move = 2,
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct ItemCollectionTransitionTriggers: u32 {
        const COLLECTION_CHANGE_ADD = 1;
        const COLLECTION_CHANGE_REMOVE = 2;
        const COLLECTION_CHANGE_RESET = 4;
        const LAYOUT_TRANSITION = 8;
    }
}

/// One addition, removal, or bounds change submitted to a transition provider.
pub struct ItemCollectionTransition {
    provider: WeakRef<ItemCollectionTransitionProvider>,
    element: Ref<Control>,
    operation: ItemCollectionTransitionOperation,
    triggers: ItemCollectionTransitionTriggers,
    old_bounds: Rect,
    new_bounds: Rect,
    progress: RefCell<Option<Rc<ItemCollectionTransitionProgress>>>,
    cancelled: Cell<bool>,
}
impl PartialEq for ItemCollectionTransition {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl ItemCollectionTransition {
    pub fn new(
        provider: Ref<ItemCollectionTransitionProvider>,
        element: Ref<Control>,
        operation: ItemCollectionTransitionOperation,
        triggers: ItemCollectionTransitionTriggers,
    ) -> Rc<Self> {
        assert_ne!(
            operation,
            ItemCollectionTransitionOperation::Move,
            "A Move transition requires old and new bounds."
        );
        Self::create(
            provider,
            element,
            operation,
            triggers,
            Rect::default(),
            Rect::default(),
        )
    }
    pub fn with_bounds(
        provider: Ref<ItemCollectionTransitionProvider>,
        element: Ref<Control>,
        triggers: ItemCollectionTransitionTriggers,
        old_bounds: Rect,
        new_bounds: Rect,
    ) -> Rc<Self> {
        Self::create(
            provider,
            element,
            ItemCollectionTransitionOperation::Move,
            triggers,
            old_bounds,
            new_bounds,
        )
    }
    fn create(
        provider: Ref<ItemCollectionTransitionProvider>,
        element: Ref<Control>,
        operation: ItemCollectionTransitionOperation,
        triggers: ItemCollectionTransitionTriggers,
        old_bounds: Rect,
        new_bounds: Rect,
    ) -> Rc<Self> {
        Rc::new(Self {
            provider: provider.downgrade(),
            element,
            operation,
            triggers,
            old_bounds,
            new_bounds,
            progress: RefCell::new(None),
            cancelled: Cell::new(false),
        })
    }
    pub fn owning_provider(&self) -> Option<Ref<ItemCollectionTransitionProvider>> {
        self.provider.upgrade()
    }
    pub fn element(&self) -> Ref<Control> {
        self.element.clone()
    }
    pub fn operation(&self) -> ItemCollectionTransitionOperation {
        self.operation
    }
    pub fn triggers(&self) -> ItemCollectionTransitionTriggers {
        self.triggers
    }
    pub fn old_bounds(&self) -> Rect {
        self.old_bounds
    }
    pub fn new_bounds(&self) -> Rect {
        self.new_bounds
    }
    pub fn has_started(&self) -> bool {
        self.progress.borrow().is_some()
    }
    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled.get()
    }
    pub(crate) fn cancel(&self) {
        self.cancelled.set(true);
    }
    pub fn start(self: &Rc<Self>) -> Rc<ItemCollectionTransitionProgress> {
        if let Some(progress) = self.progress.borrow().as_ref() {
            return progress.clone();
        }
        let progress = ItemCollectionTransitionProgress::new(self);
        *self.progress.borrow_mut() = Some(progress.clone());
        progress
    }
}
