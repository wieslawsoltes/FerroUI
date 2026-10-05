use super::{ExpressionNode, NodeState, SourceNode};
use crate::data::core::ValueTypes;
use crate::data::BindingError;
use crate::reactive::IDisposable;
use crate::{BoxedValue, FerroObject, Ref, StyledElement};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A node that reads the templated parent of its source element
/// (`$templatedParent`).
pub struct TemplatedParentNode {
    this: Weak<TemplatedParentNode>,
    state: NodeState,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl TemplatedParentNode {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), state: NodeState::new(), subscription: RefCell::new(None) })
    }

    fn read(element: &StyledElement) -> Option<BoxedValue> {
        element.templated_parent().map(|p| Rc::new(p) as BoxedValue)
    }
}

impl SourceNode for TemplatedParentNode {
    fn select_source(
        &self,
        source: Option<&Option<BoxedValue>>,
        target: &Ref<FerroObject>,
        anchor: Option<&Ref<FerroObject>>,
    ) -> Option<BoxedValue> {
        if source.is_some() {
            panic!("TemplatedParentNode is invalid in conjunction with a binding source.");
        }
        if target.is::<StyledElement>() {
            return Some(Rc::new(target.clone()));
        }
        if let Some(anchor) = anchor {
            if anchor.is::<StyledElement>() {
                return Some(Rc::new(anchor.clone()));
            }
        }
        panic!("Cannot find a StyledElement to get a TemplatedParent.");
    }
}

impl ExpressionNode for TemplatedParentNode {
    fn state(&self) -> &NodeState {
        &self.state
    }

    fn build_string(&self, builder: &mut String) {
        builder.push_str("$templatedParent");
    }

    fn on_source_changed(&self, source: Option<&BoxedValue>, _data_validation_error: Option<&BindingError>) {
        if !self.state.validate_non_null_source(source) {
            return;
        }
        let source = source.expect("validated");
        match ValueTypes::as_object(&**source).and_then(|o| o.cast::<StyledElement>()) {
            Some(element) => {
                let weak = self.this.clone();
                let id = StyledElement::templated_parent_property().id();
                let subscription = element.property_changed(move |e| {
                    if e.property().id() == id {
                        if let Some(this) = weak.upgrade() {
                            let element = this.state.source_object().and_then(|o| o.cast::<StyledElement>());
                            if let Some(element) = element {
                                this.state.set_value(Self::read(&element), None);
                            }
                        }
                    }
                });
                self.subscription.replace(Some(subscription));
                self.state.set_value(Self::read(&element), None);
            }
            None => self
                .state
                .set_error(&format!("Unable to read TemplatedParent from '{}'.", (**source).type_name())),
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
