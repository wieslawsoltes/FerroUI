use super::{StyleActivator, StyleActivatorBase};
use crate::reactive::IDisposable;
use crate::styling::PropertyEqualsSelector;
use crate::{BoxedValue, FerroProperty, StyledElement, WeakRef};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// An activator which is active when a property of a control has a specific
/// value.
pub(crate) struct PropertyEqualsActivator {
    base: StyleActivatorBase,
    this: Weak<PropertyEqualsActivator>,
    control: WeakRef<StyledElement>,
    property: &'static FerroProperty,
    value: BoxedValue,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl PropertyEqualsActivator {
    pub fn new(control: &StyledElement, property: &'static FerroProperty, value: BoxedValue) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            base: StyleActivatorBase::new(),
            this: this.clone(),
            control: control.to_ref().downgrade(),
            property,
            value,
            subscription: RefCell::new(None),
        })
    }
}

impl StyleActivator for PropertyEqualsActivator {
    fn base(&self) -> &StyleActivatorBase {
        &self.base
    }

    fn evaluate_is_active(&self) -> bool {
        match self.control.upgrade() {
            Some(control) => {
                let value = control.get_value_untyped(self.property);
                PropertyEqualsSelector::compare(&value, &self.value)
            }
            None => false,
        }
    }

    fn initialize(&self) {
        if let Some(control) = self.control.upgrade() {
            let weak = self.this.clone();
            let id = self.property.id();
            let subscription = control.property_changed(move |e| {
                if e.property().id() == id {
                    if let Some(this) = weak.upgrade() {
                        this.reevaluate_is_active();
                    }
                }
            });
            *self.subscription.borrow_mut() = Some(subscription);
        }
    }

    fn deinitialize(&self) {
        let subscription = self.subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }
}
