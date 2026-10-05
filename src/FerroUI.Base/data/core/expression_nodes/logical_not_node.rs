use super::{ExpressionNode, ISettableNode, NodeState};
use crate::data::core::{ValueType, ValueTypes};
use crate::data::{BindingError, BindingNotification};
use crate::BoxedValue;
use std::rc::Rc;

/// A node that negates the boolean value of its source (`!`).
pub struct LogicalNotNode {
    state: NodeState,
}

impl LogicalNotNode {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { state: NodeState::new() })
    }

    /// Converts a value to a boolean with the rules of the managed
    /// `Convert.ToBoolean`: null is false, numbers are true when non-zero and
    /// text must be `true` or `false`.
    fn try_convert(value: Option<&BoxedValue>) -> Option<bool> {
        let Some(value) = value else { return Some(false) };
        if let Some(b) = value.downcast_ref::<bool>() {
            return Some(*b);
        }
        if let Some(s) = ValueTypes::try_convert(Some(value), ValueType::of::<String>())
            .flatten()
            .filter(|_| value.is::<String>() || value.is::<&'static str>())
        {
            let s = s.downcast_ref::<String>()?.trim().to_string();
            return if s.eq_ignore_ascii_case("true") {
                Some(true)
            } else if s.eq_ignore_ascii_case("false") {
                Some(false)
            } else {
                None
            };
        }
        let number = ValueTypes::try_convert_registered(value, ValueType::of::<f64>())?;
        number.downcast_ref::<f64>().map(|n| *n != 0.0)
    }
}

impl ExpressionNode for LogicalNotNode {
    fn state(&self) -> &NodeState {
        &self.state
    }

    fn build_string(&self, builder: &mut String) {
        builder.push('!');
    }

    fn build_string_with_nodes(&self, builder: &mut String, nodes: &[Rc<dyn ExpressionNode>]) {
        builder.push('!');
        let index = self.state.index();
        if index > 0 {
            nodes[index - 1].build_string_with_nodes(builder, nodes);
        }
    }

    fn on_source_changed(&self, source: Option<&BoxedValue>, data_validation_error: Option<&BindingError>) {
        let v = BindingNotification::extract_value(source);
        match Self::try_convert(v.as_ref()) {
            Some(value) => self.state.set_value(Some(Rc::new(!value)), data_validation_error),
            None => self
                .state
                .set_error(&format!("Unable to convert '{}' to bool.", ValueTypes::to_display_string(source))),
        }
    }

    fn as_settable(&self) -> Option<&dyn ISettableNode> {
        Some(self)
    }
}

impl ISettableNode for LogicalNotNode {
    fn value_type(&self) -> Option<ValueType> {
        Some(ValueType::of::<bool>())
    }

    fn write_value_to_source(
        &self,
        value: Option<&BoxedValue>,
        nodes: &[Rc<dyn ExpressionNode>],
    ) -> Result<bool, BindingError> {
        let index = self.state.index();
        if index > 0 {
            if let (Some(previous), Some(bool_value)) = (nodes[index - 1].as_settable(), Self::try_convert(value)) {
                let negated: BoxedValue = Rc::new(!bool_value);
                return previous.write_value_to_source(Some(&negated), nodes);
            }
        }
        Ok(false)
    }
}
