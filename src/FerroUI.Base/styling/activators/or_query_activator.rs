use super::{IStyleActivator, IStyleActivatorSink, StyleActivator, StyleActivatorBase};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// An aggregate activator for container queries which is active when any of
/// its inputs are active.
pub(crate) struct OrQueryActivator {
    base: StyleActivatorBase,
    this: Weak<OrQueryActivator>,
    sources: RefCell<Vec<Rc<dyn IStyleActivator>>>,
}

impl OrQueryActivator {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            base: StyleActivatorBase::new(),
            this: this.clone(),
            sources: RefCell::new(Vec::new()),
        })
    }

    pub fn count(&self) -> usize {
        self.sources.borrow().len()
    }

    pub fn add(&self, activator: Rc<dyn IStyleActivator>) {
        self.sources.borrow_mut().push(activator);
    }

    fn sink(&self) -> Weak<dyn IStyleActivatorSink> {
        self.this.clone()
    }
}

impl IStyleActivatorSink for OrQueryActivator {
    fn on_next(&self, _value: bool) {
        self.reevaluate_is_active();
    }
}

impl StyleActivator for OrQueryActivator {
    fn base(&self) -> &StyleActivatorBase {
        &self.base
    }

    fn evaluate_is_active(&self) -> bool {
        let sources = self.sources.borrow();
        if sources.is_empty() {
            return true;
        }
        sources.iter().any(|source| source.get_is_active())
    }

    fn initialize(&self) {
        for source in self.sources.borrow().iter() {
            source.subscribe(self.sink());
        }
    }

    fn deinitialize(&self) {
        let sink = self.sink();
        for source in self.sources.borrow().iter() {
            source.unsubscribe(&sink);
        }
    }
}
