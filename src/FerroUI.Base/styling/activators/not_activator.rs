use super::{IStyleActivator, IStyleActivatorSink, StyleActivator, StyleActivatorBase};
use std::rc::{Rc, Weak};

/// An activator which inverts the state of an input activator.
pub(crate) struct NotActivator {
    base: StyleActivatorBase,
    this: Weak<NotActivator>,
    source: Rc<dyn IStyleActivator>,
}

impl NotActivator {
    pub fn new(source: Rc<dyn IStyleActivator>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self { base: StyleActivatorBase::new(), this: this.clone(), source })
    }

    fn sink(&self) -> Weak<dyn IStyleActivatorSink> {
        self.this.clone()
    }
}

impl IStyleActivatorSink for NotActivator {
    fn on_next(&self, _value: bool) {
        self.reevaluate_is_active();
    }
}

impl StyleActivator for NotActivator {
    fn base(&self) -> &StyleActivatorBase {
        &self.base
    }

    fn evaluate_is_active(&self) -> bool {
        !self.source.get_is_active()
    }

    fn initialize(&self) {
        self.source.subscribe(self.sink());
    }

    fn deinitialize(&self) {
        self.source.unsubscribe(&self.sink());
    }
}
