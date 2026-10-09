use super::{ExpressionNode, NodeState, SourceNode};
use crate::controls::{NameScopeLocator, NameScopeRef};
use crate::data::BindingError;
use crate::reactive::{IDisposable, IObserver};
use crate::{BoxedValue, FerroObject, Ref, StyledElement};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A node that locates an element by name in a name scope (`#name`).
pub struct NamedElementNode {
    this: Weak<NamedElementNode>,
    state: NodeState,
    name_scope: Option<Weak<dyn crate::controls::INameScope>>,
    name: Box<str>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl NamedElementNode {
    pub fn new(name_scope: Option<&NameScopeRef>, name: &str) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            state: NodeState::locating_an_element(),
            name_scope: name_scope.map(|s| Rc::downgrade(&s.0)),
            name: name.into(),
            subscription: RefCell::new(None),
        })
    }
}

struct WeakObserver(Weak<NamedElementNode>);

impl IObserver<Option<Ref<FerroObject>>> for WeakObserver {
    fn on_next(&self, value: Option<Ref<FerroObject>>) {
        if let Some(node) = self.0.upgrade() {
            node.state.set_value(value.map(|v| Rc::new(v) as BoxedValue), None);
        }
    }
}

impl SourceNode for NamedElementNode {
    fn should_log_errors(&self, _state: &NodeState, target: &FerroObject) -> bool {
        // Errors are not logged when the target element isn't rooted.
        target.downcast_ref::<StyledElement>().is_none_or(|l| l.is_attached_to_logical_tree())
    }
}

impl ExpressionNode for NamedElementNode {
    fn state(&self) -> &NodeState {
        &self.state
    }

    fn build_string(&self, builder: &mut String) {
        builder.push('#');
        builder.push_str(&self.name);
    }

    fn on_source_changed(&self, source: Option<&BoxedValue>, _data_validation_error: Option<&BindingError>) {
        if !self.state.validate_non_null_source(source) {
            return;
        }
        match self.name_scope.as_ref().and_then(Weak::upgrade) {
            Some(scope) => {
                let subscription = NameScopeLocator::track(&NameScopeRef(scope), &self.name)
                    .subscribe(Rc::new(WeakObserver(self.this.clone())));
                self.subscription.replace(Some(subscription));
            }
            None => self.state.set_error("NameScope not found."),
        }
    }

    fn unsubscribe(&self, _old_source: &BoxedValue) {
        let subscription = self.subscription.borrow_mut().take();
        if let Some(s) = subscription {
            s.dispose();
        }
    }

    fn as_source_node(&self) -> Option<&dyn SourceNode> {
        Some(self)
    }
}
