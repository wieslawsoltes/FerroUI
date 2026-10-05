use super::{IStyleActivator, IStyleActivatorSink, StyleActivator, StyleActivatorBase};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// An aggregate activator for container queries which is active when all of
/// its inputs are active.
pub(crate) struct AndQueryActivator {
    base: StyleActivatorBase,
    this: Weak<AndQueryActivator>,
    sources: RefCell<Vec<Rc<dyn IStyleActivator>>>,
}

impl AndQueryActivator {
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
        if self.base.is_subscribed() {
            panic!("AndQueryActivator is already subscribed.");
        }
        self.sources.borrow_mut().push(activator);
    }

    fn sink(&self) -> Weak<dyn IStyleActivatorSink> {
        self.this.clone()
    }
}

impl IStyleActivatorSink for AndQueryActivator {
    fn on_next(&self, _value: bool) {
        self.reevaluate_is_active();
    }
}

impl StyleActivator for AndQueryActivator {
    fn base(&self) -> &StyleActivatorBase {
        &self.base
    }

    fn evaluate_is_active(&self) -> bool {
        // Every source is evaluated, as each evaluation also records the
        // initial value of that source.
        let sources = self.sources.borrow();
        let mut result = true;
        for source in sources.iter() {
            if !source.get_is_active() {
                result = false;
            }
        }
        result
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
