use super::{ExpressionNode, NodeState, SourceNode};
use crate::data::core::ValueTypes;
use crate::data::BindingError;
use crate::reactive::IDisposable;
use crate::{BoxedValue, FerroObject, IDataContextProvider, Ref, StyledElement, Visual};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The source selection shared by the data context nodes.
fn select_data_context_source(
    source: Option<&Option<BoxedValue>>,
    target: &Ref<FerroObject>,
    anchor: Option<&Ref<FerroObject>>,
) -> Option<BoxedValue> {
    if source.is_some() {
        panic!("DataContextNode is invalid in conjunction with a binding source.");
    }
    if <dyn IDataContextProvider>::is_implemented_by(target) {
        return Some(Rc::new(target.clone()));
    }
    if let Some(anchor) = anchor {
        if <dyn IDataContextProvider>::is_implemented_by(anchor) {
            return Some(Rc::new(anchor.clone()));
        }
    }
    panic!("Cannot find a DataContext to bind to.");
}

/// The data context of an object that provides one.
fn data_context_of(object: &FerroObject) -> Option<BoxedValue> {
    object.get_value(StyledElement::data_context_property())
}

/// A node that reads the data context of its source element.
pub struct DataContextNode {
    this: Weak<DataContextNode>,
    state: NodeState,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl DataContextNode {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), state: NodeState::new(), subscription: RefCell::new(None) })
    }
}

impl SourceNode for DataContextNode {
    fn select_source(
        &self,
        source: Option<&Option<BoxedValue>>,
        target: &Ref<FerroObject>,
        anchor: Option<&Ref<FerroObject>>,
    ) -> Option<BoxedValue> {
        select_data_context_source(source, target, anchor)
    }
}

impl ExpressionNode for DataContextNode {
    fn state(&self) -> &NodeState {
        &self.state
    }

    fn on_source_changed(&self, source: Option<&BoxedValue>, _data_validation_error: Option<&BindingError>) {
        if !self.state.validate_non_null_source(source) {
            return;
        }
        let source = source.expect("validated");
        match ValueTypes::as_object(&**source).filter(|o| <dyn IDataContextProvider>::is_implemented_by(o)) {
            Some(object) => {
                let weak = self.this.clone();
                let id = StyledElement::data_context_property().id();
                let subscription = object.property_changed(move |e| {
                    if e.property().id() == id {
                        if let Some(this) = weak.upgrade() {
                            let object = this
                                .state
                                .source_object()
                                .filter(|o| <dyn IDataContextProvider>::is_implemented_by(o));
                            if let Some(object) = object {
                                this.state.set_value(ValueTypes::normalize(Rc::new(data_context_of(&object))), None);
                            }
                        }
                    }
                });
                self.subscription.replace(Some(subscription));
                self.state.set_value(ValueTypes::normalize(Rc::new(data_context_of(&object))), None);
            }
            None => self
                .state
                .set_error(&format!("Unable to read DataContext from '{}'.", (**source).type_name())),
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

    fn is_data_context_node(&self) -> bool {
        true
    }
}

/// A node that reads the data context of the visual parent of its source
/// element. Used when binding the `DataContext` property itself.
pub struct ParentDataContextNode {
    this: Weak<ParentDataContextNode>,
    state: NodeState,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    /// `None` until a parent (or the absence of one) has been recorded.
    parent: RefCell<Option<Option<Ref<FerroObject>>>>,
    parent_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl ParentDataContextNode {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            state: NodeState::new(),
            subscription: RefCell::new(None),
            parent: RefCell::new(None),
            parent_subscription: RefCell::new(None),
        })
    }

    fn set_parent(&self, parent: Option<Ref<FerroObject>>) {
        let same = match &*self.parent.borrow() {
            Some(current) => match (current, &parent) {
                (Some(a), Some(b)) => a.ptr_eq(b),
                (None, None) => true,
                _ => false,
            },
            None => false,
        };
        if same {
            return;
        }
        let old = self.parent_subscription.borrow_mut().take();
        if let Some(s) = old {
            s.dispose();
        }
        self.parent.replace(Some(parent.clone()));

        match parent.filter(|p| <dyn IDataContextProvider>::is_implemented_by(p)) {
            Some(object) => {
                let weak = self.this.clone();
                let id = StyledElement::data_context_property().id();
                let weak_object = object.downgrade();
                let subscription = object.property_changed(move |e| {
                    if e.property().id() == id {
                        if let (Some(this), Some(object)) = (weak.upgrade(), weak_object.upgrade()) {
                            this.state.set_value(ValueTypes::normalize(Rc::new(data_context_of(&object))), None);
                        }
                    }
                });
                self.parent_subscription.replace(Some(subscription));
                self.state.set_value(ValueTypes::normalize(Rc::new(data_context_of(&object))), None);
            }
            None => self.state.set_value(None, None),
        }
    }
}

impl SourceNode for ParentDataContextNode {
    fn select_source(
        &self,
        source: Option<&Option<BoxedValue>>,
        target: &Ref<FerroObject>,
        anchor: Option<&Ref<FerroObject>>,
    ) -> Option<BoxedValue> {
        select_data_context_source(source, target, anchor)
    }
}

impl ExpressionNode for ParentDataContextNode {
    fn state(&self) -> &NodeState {
        &self.state
    }

    fn on_source_changed(&self, source: Option<&BoxedValue>, _data_validation_error: Option<&BindingError>) {
        if !self.state.validate_non_null_source(source) {
            return;
        }
        let object = source.and_then(|s| ValueTypes::as_object(&**s));
        if let Some(object) = &object {
            let weak = self.this.clone();
            let id = Visual::visual_parent_property().id();
            let weak_object = object.downgrade();
            let subscription = object.property_changed(move |e| {
                if e.property().id() == id {
                    if let (Some(this), Some(object)) = (weak.upgrade(), weak_object.upgrade()) {
                        let parent = object.cast::<Visual>().and_then(|v| v.visual_parent());
                        this.set_parent(parent.map(|p| p.upcast()));
                    }
                }
            });
            self.subscription.replace(Some(subscription));
        }
        let parent = object.and_then(|o| o.cast::<Visual>()).and_then(|v| v.visual_parent());
        self.set_parent(parent.map(|p| p.upcast()));
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

    fn is_data_context_node(&self) -> bool {
        true
    }
}
