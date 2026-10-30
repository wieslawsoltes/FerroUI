//! The evaluation of state triggers (WinUI's `VisualStateManagerActuator::InitializeStateTriggers`,
//! the `StateTriggerVariantMap` of `dxaml/xcp/components/qualifiers/VariantMap.h` and
//! `CVisualStateManager2::OnSelectionChanged` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md); docs/porting/winui-calculator-gaps.md, section 5.7.
//!
//! The groups of an element with triggers are evaluated when the element is attached to a
//! visual tree, when its groups change while it is, when the client size of its window changes
//! and when a trigger changes. A group changes state only when the state its triggers select
//! changes.

use super::{TriggerRank, VisualState, VisualStateGroup, VisualStateGroupCollection, VisualStateManager};
use crate::Control;
use ferroui_base::collections::{CollectionChangedHandler, NotifyCollectionChangedEventArgs};
use ferroui_base::layout::Layoutable;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{ferro_properties, ferro_static_type, AttachedProperty, FerroProperty, Ref, Visual, WeakRef};
use std::cell::RefCell;
use std::rc::Rc;

/// The subscriptions of an element with visual state groups.
pub(crate) struct TriggerHost {
    element: WeakRef<Control>,
    groups: VisualStateGroupCollection,
    // Attached and detached, and the changes of the collection.
    element_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    // The size changes of the root of the tree, while the element is in one.
    root_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

/// The value of the private attached property that keeps a [`TriggerHost`] with its element.
#[derive(Clone)]
pub(crate) struct TriggerHostRef(Rc<TriggerHost>);

impl PartialEq for TriggerHostRef {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Drop for TriggerHost {
    fn drop(&mut self) {
        for subscription in self.element_subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
        if let Some(subscription) = self.root_subscription.get_mut().take() {
            subscription.dispose();
        }
    }
}

struct StateTriggerHosts;

ferro_static_type!(StateTriggerHosts);

ferro_properties! { impl StateTriggerHosts {
    fn host_property() -> AttachedProperty<Option<TriggerHostRef>> {
        FerroProperty::register_attached::<StateTriggerHosts, Control, Option<TriggerHostRef>>("StateTriggerHost", None)
    }
} }

/// The groups of `element` are now `groups`: the triggers are followed while it is in a tree.
pub(crate) fn install(element: &Control, groups: Option<&VisualStateGroupCollection>) {
    // The host of the previous collection drops its subscriptions.
    element.set_value(StateTriggerHosts::host_property(), None);
    let Some(groups) = groups else { return };
    let host = Rc::new(TriggerHost {
        element: element.to_ref().downgrade(),
        groups: groups.clone(),
        element_subscriptions: RefCell::new(Vec::new()),
        root_subscription: RefCell::new(None),
    });
    let weak = Rc::downgrade(&host);
    let attached = {
        let weak = weak.clone();
        element.attached_to_visual_tree(move |_| {
            if let Some(host) = weak.upgrade() {
                host.attach();
            }
        })
    };
    let detached = {
        let weak = weak.clone();
        element.detached_from_visual_tree(move |_| {
            if let Some(host) = weak.upgrade() {
                host.detach();
            }
        })
    };
    let collection = {
        let weak = weak.clone();
        let handler: Rc<CollectionChangedHandler<Ref<VisualStateGroup>>> =
            Rc::new(move |_: &NotifyCollectionChangedEventArgs<'_, Ref<VisualStateGroup>>| {
                if let Some(host) = weak.upgrade() {
                    if host.root_subscription.borrow().is_some() {
                        host.attach();
                    }
                }
            });
        let token = groups.add_collection_changed(handler);
        let list = groups.clone();
        Disposable::create(move || {
            list.remove_collection_changed(token);
        })
    };
    host.element_subscriptions.borrow_mut().extend([attached, detached, collection]);
    element.set_value(StateTriggerHosts::host_property(), Some(TriggerHostRef(host.clone())));
    if element.is_attached_to_visual_tree() {
        host.attach();
    }
}

impl TriggerHost {
    /// The element is in a tree: the triggers are linked to their groups, the size of the root
    /// is followed and every group is evaluated.
    fn attach(self: &Rc<Self>) {
        let Some(element) = self.element.upgrade() else { return };
        for group in self.groups.snapshot().iter() {
            group.set_trigger_root(Some(&element));
            for state in group.states().snapshot().iter() {
                for trigger in state.state_triggers().snapshot().iter() {
                    trigger.set_owner(Some(group));
                }
            }
        }
        if self.root_subscription.borrow().is_none() {
            if let Some(root) = element.visual_root() {
                let weak = Rc::downgrade(self);
                let subscription = root.property_changed(move |change| {
                    let property = change.property();
                    if property == Visual::bounds_property().as_property()
                        || property == crate::TopLevel::client_size_property().as_property()
                        || property == Layoutable::width_property().as_property()
                        || property == Layoutable::height_property().as_property()
                    {
                        if let Some(host) = weak.upgrade() {
                            host.evaluate();
                        }
                    }
                });
                *self.root_subscription.borrow_mut() = Some(subscription);
            }
        }
        self.evaluate();
    }

    /// The element left its tree: the size of the root is no longer followed and the triggers
    /// no longer select states (the groups keep their states, as WinUI's do).
    fn detach(&self) {
        if let Some(subscription) = self.root_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        for group in self.groups.snapshot().iter() {
            for state in group.states().snapshot().iter() {
                for trigger in state.state_triggers().snapshot().iter() {
                    trigger.set_owner(None);
                }
            }
        }
    }

    fn evaluate(&self) {
        for group in self.groups.snapshot().iter() {
            evaluate_group(group);
        }
    }
}

/// Selects the state of `group` from its triggers and goes to it if it changed (WinUI's
/// `OnSelectionChanged`: always with transitions; no state when no trigger qualifies).
pub(crate) fn evaluate_group(group: &Ref<VisualStateGroup>) {
    let Some(root) = group.trigger_root() else { return };
    let window = root.presentation_source().map(|source| source.client_size());
    let mut has_triggers = false;
    let mut best: Option<(TriggerRank, Ref<VisualState>)> = None;
    for state in group.states().snapshot().iter() {
        for trigger in state.state_triggers().snapshot().iter() {
            has_triggers = true;
            if let Some(rank) = trigger.qualification(window) {
                if best.as_ref().is_none_or(|(best_rank, _)| rank.beats(best_rank)) {
                    best = Some((rank, state.clone()));
                }
            }
        }
    }
    if !has_triggers {
        return;
    }
    let winner = best.map(|(_, state)| state);
    if !group.set_trigger_selection(winner.as_ref()) {
        return;
    }
    let control = control_of(&root);
    match VisualStateManager::get_custom_visual_state_manager(&root) {
        Some(custom) => {
            if let Some(state) = &winner {
                custom.go_to_state_core(&control, &root, &state.name(), Some(group), Some(state), true);
            }
        }
        None => {
            VisualStateManager::change_state(&control, &root, group, winner.as_ref(), true);
        }
    }
}

/// The control whose states the groups of `root` are: its templated parent (the root of a
/// template), else its logical parent (the content of a user control or a page), else itself.
fn control_of(root: &Ref<Control>) -> Ref<Control> {
    root.templated_parent()
        .and_then(|parent| parent.cast::<Control>())
        .or_else(|| root.parent().and_then(|parent| parent.cast::<Control>()))
        .unwrap_or_else(|| root.clone())
}
