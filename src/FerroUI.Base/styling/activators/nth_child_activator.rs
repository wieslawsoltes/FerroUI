use super::{StyleActivator, StyleActivatorBase};
use crate::logical_tree::{ChildIndexChangedAction, ChildIndexChangedEventArgs, IChildIndexProvider};
use crate::reactive::IDisposable;
use crate::styling::NthChildSelector;
use crate::{StyledElement, WeakRef};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// An activator which is active when control's index was changed.
pub(crate) struct NthChildActivator {
    base: StyleActivatorBase,
    this: Weak<NthChildActivator>,
    control: WeakRef<StyledElement>,
    /// The logical parent that provides the child index. The provider is
    /// asked for when needed rather than held, so that the activator (which
    /// is owned by the child) does not keep its parent alive.
    provider_owner: WeakRef<StyledElement>,
    step: i32,
    offset: i32,
    reversed: bool,
    index: Cell<i32>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl NthChildActivator {
    pub fn new(
        control: &StyledElement,
        provider_owner: &StyledElement,
        step: i32,
        offset: i32,
        reversed: bool,
    ) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            base: StyleActivatorBase::new(),
            this: this.clone(),
            control: control.to_ref().downgrade(),
            provider_owner: provider_owner.to_ref().downgrade(),
            step,
            offset,
            reversed,
            index: Cell::new(-1),
            subscription: RefCell::new(None),
        })
    }

    fn provider(&self) -> Option<Rc<dyn IChildIndexProvider>> {
        self.provider_owner.upgrade().and_then(|owner| owner.child_index_provider())
    }

    fn child_index_changed(&self, e: &ChildIndexChangedEventArgs) {
        let Some(control) = self.control.upgrade() else { return };
        let Some(provider) = self.provider() else { return };

        // Run matching again if:
        // 1. Subscribed child index was changed
        // 2. Child indexes were reset
        // 3. We're a reversed (nth-last-child) selector and total count has
        //    changed
        match e.action() {
            // `evaluate_is_active` should read directly from its inputs and
            // not rely on any subscriptions to fire in order to be up-to-date.
            // In this case however the rule needs to be broken and the value
            // from the event used where possible instead of asking the
            // provider for the child index: this event can be fired during the
            // process of realizing an element of a virtualized list; in this
            // case there may be more than one nth-child style on the list
            // item and when the other is re-evaluated asking the provider may
            // not return the correct index as the element isn't yet realized.
            ChildIndexChangedAction::ChildIndexChanged if e.child().is_some_and(|c| *c == control) => {
                let index = if e.index() >= 0 { e.index() } else { provider.get_child_index(&control) };
                self.index.set(index);
                self.reevaluate_is_active();
            }
            ChildIndexChangedAction::ChildIndexesReset => {
                self.index.set(provider.get_child_index(&control));
                self.reevaluate_is_active();
            }
            ChildIndexChangedAction::TotalCountChanged if self.reversed => {
                self.index.set(provider.get_child_index(&control));
                self.reevaluate_is_active();
            }
            _ => {}
        }
    }
}

impl StyleActivator for NthChildActivator {
    fn base(&self) -> &StyleActivatorBase {
        &self.base
    }

    fn evaluate_is_active(&self) -> bool {
        let Some(provider) = self.provider() else { return false };
        let index = if self.index.get() >= 0 {
            self.index.get()
        } else {
            match self.control.upgrade() {
                Some(control) => provider.get_child_index(&control),
                None => -1,
            }
        };
        NthChildSelector::evaluate_index(index, &*provider, self.step, self.offset, self.reversed).is_match()
    }

    fn initialize(&self) {
        let Some(provider) = self.provider() else { return };
        let weak = self.this.clone();
        let subscription = provider.child_index_changed(Rc::new(move |e: &ChildIndexChangedEventArgs| {
            if let Some(this) = weak.upgrade() {
                this.child_index_changed(e);
            }
        }));
        *self.subscription.borrow_mut() = Some(subscription);
    }

    fn deinitialize(&self) {
        let subscription = self.subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }
}
