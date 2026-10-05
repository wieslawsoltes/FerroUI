use super::{ExpressionNode, NodeState, SourceNode};
use crate::data::core::ValueTypes;
use crate::data::BindingError;
use crate::reactive::IDisposable;
use crate::{BoxedValue, FerroObject, Ref, StyledElement, TypeInfo};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A node that locates an ancestor of its source element in the logical tree
/// (`$parent`, `$parent[Type, level]`), following the element's attachment
/// to the tree.
pub struct LogicalAncestorElementNode {
    this: Weak<LogicalAncestorElementNode>,
    state: NodeState,
    ancestor_type: Option<&'static TypeInfo>,
    ancestor_level: usize,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl LogicalAncestorElementNode {
    pub fn new(ancestor_type: Option<&'static TypeInfo>, ancestor_level: usize) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            state: NodeState::new(),
            ancestor_type,
            ancestor_level,
            subscription: RefCell::new(None),
        })
    }

    /// The tracked ancestor: found only while the element is attached to the
    /// logical tree.
    fn get_result(&self, relative_to: &StyledElement) -> Option<Ref<StyledElement>> {
        if !relative_to.is_attached_to_logical_tree() {
            return None;
        }
        let mut level = 0;
        let mut current = relative_to.parent();
        while let Some(ancestor) = current {
            if self.ancestor_type.is_none_or(|t| t.is_assignable_from(ancestor.get_type())) {
                if level == self.ancestor_level {
                    return Some(ancestor);
                }
                level += 1;
            }
            current = ancestor.parent();
        }
        None
    }

    fn tracked_control_changed(&self, control: Option<Ref<StyledElement>>) {
        match control {
            Some(control) => {
                let object: Ref<FerroObject> = control.upcast();
                self.state.set_value(Some(Rc::new(object)), None)
            }
            None => self.state.set_error("Ancestor not found."),
        }
    }
}

impl SourceNode for LogicalAncestorElementNode {
    fn select_source(
        &self,
        source: Option<&Option<BoxedValue>>,
        target: &Ref<FerroObject>,
        anchor: Option<&Ref<FerroObject>>,
    ) -> Option<BoxedValue> {
        if source.is_some() {
            panic!("LogicalAncestorNode is invalid in conjunction with a binding source.");
        }
        if target.is::<StyledElement>() {
            return Some(Rc::new(target.clone()));
        }
        if let Some(anchor) = anchor {
            if anchor.is::<StyledElement>() {
                return Some(Rc::new(anchor.clone()));
            }
        }
        panic!("Cannot find an ILogical to get a logical ancestor.");
    }

    fn should_log_errors(&self, _state: &NodeState, target: &FerroObject) -> bool {
        target.downcast_ref::<StyledElement>().is_some_and(|l| l.is_attached_to_logical_tree())
    }
}

impl ExpressionNode for LogicalAncestorElementNode {
    fn state(&self) -> &NodeState {
        &self.state
    }

    fn build_string(&self, builder: &mut String) {
        builder.push_str("$parent");
        if self.ancestor_level > 0 || self.ancestor_type.is_some() {
            builder.push('[');
            if let Some(t) = self.ancestor_type {
                builder.push_str(t.name());
                if self.ancestor_level > 0 {
                    builder.push(',');
                }
            }
            if self.ancestor_level > 0 {
                builder.push_str(&self.ancestor_level.to_string());
            }
            builder.push(']');
        }
    }

    fn on_source_changed(&self, source: Option<&BoxedValue>, _data_validation_error: Option<&BindingError>) {
        if !self.state.validate_non_null_source(source) {
            return;
        }
        let logical = source.and_then(|s| ValueTypes::as_object(&**s)).and_then(|o| o.cast::<StyledElement>());
        if let Some(logical) = logical {
            let weak = self.this.clone();
            let weak_logical = logical.downgrade();
            let attached = logical.attached_to_logical_tree(move |_| {
                if let (Some(this), Some(logical)) = (weak.upgrade(), weak_logical.upgrade()) {
                    this.tracked_control_changed(this.get_result(&logical));
                }
            });
            let weak = self.this.clone();
            let detached = logical.detached_from_logical_tree(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.tracked_control_changed(None);
                }
            });
            self.subscription.replace(Some(crate::reactive::Disposable::create(move || {
                attached.dispose();
                detached.dispose();
            })));
            self.tracked_control_changed(self.get_result(&logical));
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
