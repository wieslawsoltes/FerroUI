// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.

use super::item_collection_transition::ItemCollectionTransition;
use ferroui_controls::Control;
use ferroui_base::Ref;
use std::rc::{Rc, Weak};

/// A started transition's completion handle. Keeping this handle alone does
/// not retain the transition or its element after the provider's keepalive.
pub struct ItemCollectionTransitionProgress {
    transition: Weak<ItemCollectionTransition>,
}
impl PartialEq for ItemCollectionTransitionProgress {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl ItemCollectionTransitionProgress {
    pub(crate) fn new(transition: &Rc<ItemCollectionTransition>) -> Rc<Self> {
        Rc::new(Self {
            transition: Rc::downgrade(transition),
        })
    }
    pub fn transition(&self) -> Option<Rc<ItemCollectionTransition>> {
        self.transition.upgrade()
    }
    pub fn element(&self) -> Option<Ref<Control>> {
        self.transition().map(|transition| transition.element())
    }
    /// Raises TransitionCompleted synchronously. Repeated calls intentionally
    /// raise repeated notifications, preserving the public lifecycle contract.
    pub fn complete(&self) {
        if let Some(transition) = self.transition() {
            if let Some(provider) = transition.owning_provider() {
                provider.notify_transition_completed(transition);
            }
        }
    }
}
