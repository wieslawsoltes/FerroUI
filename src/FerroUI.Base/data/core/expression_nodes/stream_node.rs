use super::{ExpressionNode, NodeState};
use crate::data::core::plugins::{BindingPlugins, IStreamPlugin};
use crate::data::core::WeakValue;
use crate::data::BindingError;
use crate::reactive::{IDisposable, IObserver};
use crate::BoxedValue;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A node that produces the values of an observable source (`^`): through a
/// fixed stream plugin (compiled binding paths) or, when none is given, the
/// first registered plugin that matches the source.
pub struct StreamNode {
    this: Weak<StreamNode>,
    state: NodeState,
    plugin: Option<Rc<dyn IStreamPlugin>>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl StreamNode {
    pub fn new(plugin: Rc<dyn IStreamPlugin>) -> Rc<Self> {
        Self::create(Some(plugin))
    }

    /// Creates a node that selects the plugin from the registered plugins.
    pub fn new_dynamic() -> Rc<Self> {
        Self::create(None)
    }

    fn create(plugin: Option<Rc<dyn IStreamPlugin>>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            state: NodeState::new(),
            plugin,
            subscription: RefCell::new(None),
        })
    }
}

struct WeakObserver(Weak<StreamNode>);

impl IObserver<Option<BoxedValue>> for WeakObserver {
    fn on_next(&self, value: Option<BoxedValue>) {
        if let Some(node) = self.0.upgrade() {
            node.state.set_value_or_notification(value);
        }
    }
}

impl ExpressionNode for StreamNode {
    fn state(&self) -> &NodeState {
        &self.state
    }

    fn build_string(&self, builder: &mut String) {
        builder.push('^');
    }

    fn on_source_changed(&self, source: Option<&BoxedValue>, _data_validation_error: Option<&BindingError>) {
        if !self.state.validate_non_null_source(source) {
            return;
        }
        let reference = WeakValue::new(source.expect("validated"));
        let dynamic = self.plugin.is_none();
        let plugin = match &self.plugin {
            Some(plugin) => Some(plugin.clone()),
            None => BindingPlugins::stream_handlers().into_iter().find(|p| p.match_(&reference)),
        };
        match plugin.and_then(|p| p.start(&reference)) {
            Some(observable) => {
                let subscription = observable.subscribe(Rc::new(WeakObserver(self.this.clone())));
                self.subscription.replace(Some(subscription));
            }
            None if dynamic => self.state.set_value(None, None),
            None => self.state.clear_value(),
        }
    }

    fn unsubscribe(&self, _old_source: &BoxedValue) {
        let subscription = self.subscription.borrow_mut().take();
        if let Some(s) = subscription {
            s.dispose();
        }
    }
}

impl Drop for StreamNode {
    fn drop(&mut self) {
        if let Some(s) = self.subscription.get_mut().take() {
            s.dispose();
        }
    }
}
