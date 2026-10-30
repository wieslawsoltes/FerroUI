// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.

use super::item_collection_transition::ItemCollectionTransition;
use ferroui_controls::Control;
use ferroui_base::Ref;
use std::rc::Rc;

#[derive(Clone, PartialEq)]
pub struct ItemCollectionTransitionCompletedEventArgs {
    transition: Rc<ItemCollectionTransition>,
}
impl ItemCollectionTransitionCompletedEventArgs {
    pub fn new(transition: Rc<ItemCollectionTransition>) -> Self {
        Self { transition }
    }
    pub fn transition(&self) -> Rc<ItemCollectionTransition> {
        self.transition.clone()
    }
    pub fn element(&self) -> Ref<Control> {
        self.transition.element()
    }
}
