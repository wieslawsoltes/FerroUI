//! ATT = Ancestor Transform Tracker.
//!
//! While we generally avoid dealing with keeping world transforms up to
//! date, we still need it for cases like adorners. Instead of updating
//! world transforms eagerly, we use a subscription model where visuals can
//! subscribe to notifications when any ancestor's world transform changes.

use super::ServerCompositionVisual;
use crate::Matrix;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// A subscription to the transform changes of the ancestor chain of a
/// visual: the two callbacks upstream stores as delegates.
#[derive(Clone)]
pub(super) enum ActSubscriber {
    /// `ParentActSubscriptionAction` of a child: forwards to the
    /// subscribers of that child.
    CombinedTransformChanged(Weak<ServerCompositionVisual>),
    /// `AdornedVisualActSubscriptionAction` of an adorner: enqueues the
    /// adorner for an update.
    AdornedVisualWorldTransformChanged(Weak<ServerCompositionVisual>),
}

impl ActSubscriber {
    fn same(&self, other: &ActSubscriber) -> bool {
        match (self, other) {
            (ActSubscriber::CombinedTransformChanged(a), ActSubscriber::CombinedTransformChanged(b)) => {
                Weak::ptr_eq(a, b)
            }
            (
                ActSubscriber::AdornedVisualWorldTransformChanged(a),
                ActSubscriber::AdornedVisualWorldTransformChanged(b),
            ) => Weak::ptr_eq(a, b),
            _ => false,
        }
    }

    fn invoke(&self) {
        match self {
            ActSubscriber::CombinedTransformChanged(visual) => {
                if let Some(visual) = visual.upgrade() {
                    visual.att_helper_combined_transform_changed();
                }
            }
            ActSubscriber::AdornedVisualWorldTransformChanged(visual) => {
                if let Some(visual) = visual.upgrade() {
                    visual.adorner_helper_enqueue_for_adorner_update();
                }
            }
        }
    }
}

#[derive(Default)]
pub(super) struct AttHelper {
    pub(super) ancestor_chain_transform_subscribers: RefCell<Vec<ActSubscriber>>,
    // We keep adorner stuff here too
    pub(super) enqueued_for_adorner_update: Cell<bool>,
}

impl ServerCompositionVisual {
    fn parent_act_subscription_action(&self) -> ActSubscriber {
        ActSubscriber::CombinedTransformChanged(self.this.clone())
    }

    pub(super) fn adorned_visual_act_subscription_action(&self) -> ActSubscriber {
        ActSubscriber::AdornedVisualWorldTransformChanged(self.this.clone())
    }

    /// The number of subscribers to the transforms of the ancestor chain of
    /// the visual: the adorners of the visual, and its children that have
    /// subscribers of their own (a diagnostics member, used by tests to
    /// verify that subscriptions are released).
    pub fn ancestor_transform_subscriber_count(&self) -> usize {
        self.att_helper.ancestor_chain_transform_subscribers.borrow().len()
    }

    pub(super) fn att_helper_combined_transform_changed(&self) {
        if self.att_helper.ancestor_chain_transform_subscribers.borrow().is_empty() {
            return;
        }
        let subscribers = self.att_helper.ancestor_chain_transform_subscribers.borrow().clone();
        for subscriber in subscribers {
            subscriber.invoke();
        }
    }

    pub(super) fn att_helper_parent_changing(&self) {
        if let Some(parent) = self.parent() {
            if !self.att_helper.ancestor_chain_transform_subscribers.borrow().is_empty() {
                parent.att_helper_unsubscribe_from_act_notification(&self.parent_act_subscription_action());
            }
        }
    }

    pub(super) fn att_helper_parent_changed(&self) {
        let parent = self.parent();
        if let Some(parent) = &parent {
            if !self.att_helper.ancestor_chain_transform_subscribers.borrow().is_empty() {
                parent.att_helper_subscribe_to_act_notification(self.parent_act_subscription_action());
            }
        }
        if parent.is_some() && self.adorned_visual().is_some() {
            self.adorner_helper_enqueue_for_adorner_update();
        }
    }

    pub(super) fn att_helper_subscribe_to_act_notification(&self, cb: ActSubscriber) {
        let count = {
            let mut subscribers = self.att_helper.ancestor_chain_transform_subscribers.borrow_mut();
            if !subscribers.iter().any(|s| s.same(&cb)) {
                subscribers.push(cb);
            }
            subscribers.len()
        };
        if count == 1 {
            if let Some(parent) = self.parent() {
                parent.att_helper_subscribe_to_act_notification(self.parent_act_subscription_action());
            }
        }
    }

    pub(super) fn att_helper_unsubscribe_from_act_notification(&self, cb: &ActSubscriber) {
        let count = {
            let mut subscribers = self.att_helper.ancestor_chain_transform_subscribers.borrow_mut();
            subscribers.retain(|s| !s.same(cb));
            subscribers.len()
        };
        if count == 0 {
            if let Some(parent) = self.parent() {
                parent.att_helper_unsubscribe_from_act_notification(&self.parent_act_subscription_action());
            }
        }
    }

    /// The transform from `visual` to `ancestor`; `None` if `ancestor` is
    /// not an ancestor of `visual`.
    pub(super) fn compute_transform_from_ancestor(
        visual: &Rc<ServerCompositionVisual>,
        ancestor: &Rc<ServerCompositionVisual>,
    ) -> Option<Matrix> {
        let mut transform = visual.own_transform.get().unwrap_or(Matrix::IDENTITY);
        let mut visual = visual.clone();
        while let Some(parent) = visual.parent() {
            visual = parent;
            if Rc::ptr_eq(&visual, ancestor) {
                // Walked up to ancestor
                return Some(transform);
            }
            if let Some(own_transform) = visual.own_transform.get() {
                transform = transform * own_transform;
            }
        }
        // Visual is a part of a different subtree, this is not supported
        None
    }
}
