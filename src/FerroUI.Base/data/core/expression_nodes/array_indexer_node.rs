use super::{ExpressionNode, ISettableNode, NodeState};
use crate::data::core::{ValueType, ValueTypes};
use crate::data::model::ModelTypes;
use crate::data::BindingError;
use crate::BoxedValue;
use std::rc::Rc;

/// A node in a binding expression which accesses an array with integer
/// indexers.
pub struct ArrayIndexerNode {
    state: NodeState,
    indexes: Vec<i32>,
}

impl ArrayIndexerNode {
    pub fn new(indexes: Vec<i32>) -> Rc<Self> {
        Rc::new(Self { state: NodeState::new(), indexes })
    }

    pub fn indexes(&self) -> &[i32] {
        &self.indexes
    }
}

impl ExpressionNode for ArrayIndexerNode {
    fn state(&self) -> &NodeState {
        &self.state
    }

    fn build_string(&self, builder: &mut String) {
        builder.push('[');
        for (i, index) in self.indexes.iter().enumerate() {
            builder.push_str(&index.to_string());
            if i != self.indexes.len() - 1 {
                builder.push(',');
            }
        }
        builder.push(']');
    }

    fn on_source_changed(&self, source: Option<&BoxedValue>, _data_validation_error: Option<&BindingError>) {
        if !self.state.validate_non_null_source(source) {
            return;
        }
        let Some(source) = source else { return };
        let model = ModelTypes::find(&**source);
        match model.as_ref().and_then(|m| m.as_array(&**source)) {
            Some(array) => match array.get_value(&self.indexes) {
                Ok(value) => self.state.set_value(value.and_then(ValueTypes::normalize), None),
                Err(e) => self.state.set_error(&e.to_string()),
            },
            None => self.state.clear_value(),
        }
    }

    fn as_settable(&self) -> Option<&dyn ISettableNode> {
        Some(self)
    }
}

impl ISettableNode for ArrayIndexerNode {
    fn value_type(&self) -> Option<ValueType> {
        let source = self.state.source()?;
        let model = ModelTypes::find(&*source)?;
        model.as_array(&*source).map(|a| a.element_type())
    }

    fn write_value_to_source(
        &self,
        value: Option<&BoxedValue>,
        _nodes: &[Rc<dyn ExpressionNode>],
    ) -> Result<bool, BindingError> {
        let Some(source) = self.state.source() else { return Ok(false) };
        let model = ModelTypes::find(&*source);
        match model.as_ref().and_then(|m| m.as_array(&*source)) {
            Some(array) => array.set_value(&self.indexes, value).map(|()| true),
            None => Ok(false),
        }
    }
}
