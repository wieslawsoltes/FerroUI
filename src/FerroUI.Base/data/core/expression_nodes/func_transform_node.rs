use super::{ExpressionNode, NodeState};
use crate::data::BindingError;
use crate::BoxedValue;
use std::rc::Rc;

/// A node that transforms its source with a function. Used for type casts in
/// compiled binding paths.
pub struct FuncTransformNode {
    state: NodeState,
    transform: Rc<dyn Fn(Option<&BoxedValue>) -> Option<BoxedValue>>,
}

impl FuncTransformNode {
    pub fn new(transform: Rc<dyn Fn(Option<&BoxedValue>) -> Option<BoxedValue>>) -> Rc<Self> {
        Rc::new(Self { state: NodeState::new(), transform })
    }
}

impl ExpressionNode for FuncTransformNode {
    fn state(&self) -> &NodeState {
        &self.state
    }

    fn build_string(&self, _builder: &mut String) {
        // We don't have enough information to add anything here.
    }

    fn on_source_changed(&self, source: Option<&BoxedValue>, _data_validation_error: Option<&BindingError>) {
        // Only used for type casts, which produce null from null. Any error
        // belongs to the member access which follows.
        self.state.set_value((self.transform)(source), None);
    }
}
