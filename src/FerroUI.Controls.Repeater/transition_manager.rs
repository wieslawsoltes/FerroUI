// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.

use super::item_collection_transition::{
    ItemCollectionTransition, ItemCollectionTransitionOperation as Operation,
    ItemCollectionTransitionTriggers as Triggers,
};
use super::item_collection_transition_provider::ItemCollectionTransitionProvider;
use ferroui_controls::Control;
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{Rect, Ref};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

/// Derives animation triggers from collection and layout changes during the
/// current layout cycle and returns removed elements when their transition ends.
pub struct TransitionManager {
    provider: RefCell<Option<Ref<ItemCollectionTransitionProvider>>>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    recycle: Rc<dyn Fn(Ref<Control>)>,
    triggers: Cell<Triggers>,
    provider_change: Cell<u64>,
    transitions: RefCell<HashMap<Ref<Control>, Vec<Weak<ItemCollectionTransition>>>>,
    removals: RefCell<HashMap<Ref<Control>, Weak<ItemCollectionTransition>>>,
}
impl TransitionManager {
    pub fn new(recycle: Rc<dyn Fn(Ref<Control>)>) -> Rc<Self> {
        Rc::new(Self {
            provider: RefCell::new(None),
            subscription: RefCell::new(None),
            recycle,
            triggers: Cell::new(Triggers::empty()),
            provider_change: Cell::new(0),
            transitions: RefCell::new(HashMap::new()),
            removals: RefCell::new(HashMap::new()),
        })
    }
    pub fn on_transition_provider_changed(
        self: &Rc<Self>,
        provider: Option<Ref<ItemCollectionTransitionProvider>>,
    ) {
        if self.provider.borrow().as_ref() == provider.as_ref() {
            return;
        }
        let change = self.provider_change.get().wrapping_add(1);
        self.provider_change.set(change);
        if !self.clear_previous_provider_transitions(change) {
            return;
        }
        let old = self.subscription.borrow_mut().take();
        if let Some(old) = old {
            old.dispose();
        }
        *self.provider.borrow_mut() = provider.clone();
        if let Some(provider) = provider {
            let weak = Rc::downgrade(self);
            let subscription = provider.transition_completed(move |args| {
                if let Some(this) = weak.upgrade() {
                    this.on_transition_completed(&args.transition());
                }
            });
            *self.subscription.borrow_mut() = Some(subscription);
        }
    }
    fn clear_previous_provider_transitions(&self, change: u64) -> bool {
        loop {
            if self.provider_change.get() != change {
                // A completion or factory callback installed a newer provider.
                return false;
            }
            let elements: HashSet<_> = self
                .transitions
                .borrow()
                .keys()
                .chain(self.removals.borrow().keys())
                .cloned()
                .collect();
            if elements.is_empty() {
                return true;
            }
            for element in elements {
                if self.provider_change.get() != change {
                    return false;
                }
                let was_removing = self.removals.borrow().contains_key(&element);
                self.on_element_clearing(&element);
                // Cancellation can complete synchronously. The old identity is
                // already invalid, so explicitly release its Animator owner;
                // a callback that reused and removed this control again owns a
                // different pending identity and must keep it.
                let recycle = was_removing && !self.removals.borrow().contains_key(&element);
                if recycle {
                    (self.recycle)(element);
                }
            }
            // Factory and completion callbacks may queue more work while the
            // old provider is still installed. Drain that work before detaching
            // its subscription, unless a nested provider change superseded us.
        }
    }
    pub fn on_layout_changing(&self) {
        self.triggers
            .set(self.triggers.get() | Triggers::LAYOUT_TRANSITION);
    }
    pub fn on_items_source_changed(&self, action: NotifyCollectionChangedAction) {
        let added = match action {
            NotifyCollectionChangedAction::Add => Triggers::COLLECTION_CHANGE_ADD,
            NotifyCollectionChangedAction::Remove => Triggers::COLLECTION_CHANGE_REMOVE,
            NotifyCollectionChangedAction::Replace => {
                Triggers::COLLECTION_CHANGE_ADD | Triggers::COLLECTION_CHANGE_REMOVE
            }
            NotifyCollectionChangedAction::Reset => Triggers::COLLECTION_CHANGE_RESET,
            NotifyCollectionChangedAction::Move => Triggers::empty(),
        };
        self.triggers.set(self.triggers.get() | added);
    }
    pub fn on_element_prepared(&self, element: &Ref<Control>) {
        let provider = self.provider.borrow().clone();
        let triggers = self.triggers.get()
            & (Triggers::COLLECTION_CHANGE_ADD
                | Triggers::COLLECTION_CHANGE_RESET
                | Triggers::LAYOUT_TRANSITION);
        if let Some(provider) = provider.filter(|_| !triggers.is_empty()) {
            let transition = ItemCollectionTransition::new(
                provider.clone(),
                element.clone(),
                Operation::Add,
                triggers,
            );
            self.track_transition(&transition);
            provider.queue_transition(transition);
        }
    }
    pub fn clear_element(&self, element: &Ref<Control>) -> bool {
        let provider = self.provider.borrow().clone();
        let triggers = self.triggers.get()
            & (Triggers::COLLECTION_CHANGE_REMOVE | Triggers::COLLECTION_CHANGE_RESET);
        let Some(provider) = provider.filter(|_| !triggers.is_empty()) else {
            return false;
        };
        let transition = ItemCollectionTransition::new(
            provider.clone(),
            element.clone(),
            Operation::Remove,
            triggers,
        );
        if !provider.should_animate(transition.clone()) {
            return false;
        }
        self.removals
            .borrow_mut()
            .insert(element.clone(), Rc::downgrade(&transition));
        self.track_transition(&transition);
        provider.queue_transition(transition);
        true
    }
    pub fn on_element_bounds_changed(
        &self,
        element: &Ref<Control>,
        old_bounds: Rect,
        new_bounds: Rect,
    ) {
        let provider = self.provider.borrow().clone();
        if let Some(provider) = provider {
            let recorded = self.triggers.get();
            let triggers = if recorded.is_empty() {
                Triggers::LAYOUT_TRANSITION
            } else {
                recorded
            };
            let transition = ItemCollectionTransition::with_bounds(
                provider.clone(),
                element.clone(),
                triggers,
                old_bounds,
                new_bounds,
            );
            self.track_transition(&transition);
            provider.queue_transition(transition);
        }
    }
    /// Invalidate the removal identity and cancel native animation values before
    /// ElementClearing or the old factory can observe and reuse this control.
    pub fn on_element_clearing(&self, element: &Ref<Control>) {
        self.removals.borrow_mut().remove(element);
        let transitions = self.transitions.borrow_mut().remove(element);
        let mut providers: Vec<_> = self.provider.borrow().clone().into_iter().collect();
        for transition in transitions
            .into_iter()
            .flatten()
            .filter_map(|t| t.upgrade())
        {
            transition.cancel();
            if let Some(provider) = transition.owning_provider() {
                if !providers.contains(&provider) {
                    providers.push(provider);
                }
            }
        }
        for provider in providers {
            provider.cancel_transitions_for_element(element);
        }
    }
    fn track_transition(&self, transition: &Rc<ItemCollectionTransition>) {
        let mut transitions = self.transitions.borrow_mut();
        let tracked = transitions.entry(transition.element()).or_default();
        tracked.retain(|transition| transition.strong_count() != 0);
        tracked.push(Rc::downgrade(transition));
    }
    fn on_transition_completed(&self, transition: &Rc<ItemCollectionTransition>) {
        let element = transition.element();
        {
            let mut transitions = self.transitions.borrow_mut();
            if let Some(tracked) = transitions.get_mut(&element) {
                tracked.retain(|candidate| {
                    candidate
                        .upgrade()
                        .is_some_and(|candidate| !Rc::ptr_eq(&candidate, transition))
                });
                if tracked.is_empty() {
                    transitions.remove(&element);
                }
            }
        }
        // A public progress handle may report Complete repeatedly, even after
        // this same control has been reused and removed a second time. Only the
        // exact removal that transferred ownership to Animator may recycle it.
        let recycle = if transition.operation() == Operation::Remove {
            let mut removals = self.removals.borrow_mut();
            let matches = removals
                .get(&element)
                .and_then(Weak::upgrade)
                .is_some_and(|pending| Rc::ptr_eq(&pending, transition));
            if matches {
                removals.remove(&element);
            }
            matches
        } else {
            false
        };
        if recycle {
            (self.recycle)(element);
        }
    }
    pub fn on_owner_arranged(&self) {
        self.triggers.set(Triggers::empty());
    }
}
impl Drop for TransitionManager {
    fn drop(&mut self) {
        if let Some(subscription) = self.subscription.get_mut().take() {
            subscription.dispose();
        }
    }
}
