// Licensed under the MIT license. See the repository LICENSE file.
//! Readonly System.Nullable<T> members over the native erased nullable representation.
use super::{append_member, ExpressionNode, NodeState};
use crate::data::core::{ValueType, ValueTypes};
use crate::data::{BindingError, BindingNotification};
use crate::BoxedValue;
use std::rc::Rc;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NullableMember {
    Value,
    HasValue,
}
impl NullableMember {
    pub fn name(self) -> &'static str {
        match self {
            Self::Value => "Value",
            Self::HasValue => "HasValue",
        }
    }
    /// CLR boxing erases Nullable: Some(T) is T and None is null. Accept only that T
    /// or its registered nullable carrier, never a user conversion from another type.
    pub fn read(
        self,
        source: Option<&BoxedValue>,
        inner: ValueType,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let value = match source {
            None => None,
            Some(value) => {
                let actual = ValueType::of_value(&**value);
                if actual == inner {
                    Some(value.clone())
                } else if ValueTypes::nullable_carrier(actual.id()) == Some(inner) {
                    ValueTypes::normalize(value.clone())
                } else {
                    return Err(BindingError::message(format!(
                        "InvalidCastException: Nullable<{}> cannot receive {}.",
                        inner.name(),
                        actual.name()
                    )));
                }
            }
        };
        match self {
            Self::HasValue => Ok(Some(crate::utilities::BooleanBoxes::box_(value.is_some()))),
            Self::Value => match value {
                None => Err(BindingError::message(
                    "InvalidOperationException: Nullable object must have a value.",
                )),
                Some(value) => ValueTypes::copy_nullable_value(&value, inner)
                    .map(Some)
                    .ok_or_else(|| {
                        BindingError::message(
                            "Nullable value carrier has no registered copy operation.",
                        )
                    }),
            },
        }
    }
}
pub struct NullableMemberNode {
    state: NodeState,
    inner: ValueType,
    member: NullableMember,
    accepts_null: bool,
}
impl NullableMemberNode {
    pub fn new(inner: ValueType, member: NullableMember, accepts_null: bool) -> Rc<Self> {
        Rc::new(Self {
            state: NodeState::new(),
            inner,
            member,
            accepts_null,
        })
    }
}
impl ExpressionNode for NullableMemberNode {
    fn state(&self) -> &NodeState {
        &self.state
    }
    fn build_string(&self, builder: &mut String) {
        append_member(builder, self.member.name());
    }
    fn on_source_changed(&self, source: Option<&BoxedValue>, validation: Option<&BindingError>) {
        if source
            .and_then(|v| v.downcast_ref::<BindingNotification>())
            .is_some_and(|n| n.error_type() == crate::data::BindingErrorType::Error)
        {
            self.state.set_value_or_notification(source.cloned());
            return;
        }
        let value = BindingNotification::extract_value(source);
        if self.accepts_null && value.is_none() {
            self.state.short_circuit_null();
            return;
        }
        match self.member.read(value.as_ref(), self.inner) {
            Ok(value) => self.state.set_value(value, validation),
            Err(e) => self.state.set_error(&e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    #[derive(Clone, PartialEq)]
    struct Value {
        value: Cell<i32>,
    }
    #[test]
    fn nullable_members_preserve_empty_present_default_copy_and_nominal_casts() {
        ValueTypes::register_nullable::<Value>();
        let inner = ValueType::of::<Value>();
        assert_eq!(
            *NullableMember::HasValue
                .read(None, inner)
                .unwrap()
                .unwrap()
                .downcast_ref::<bool>()
                .unwrap(),
            false
        );
        assert!(NullableMember::Value
            .read(None, inner)
            .err()
            .unwrap()
            .to_string()
            .contains("InvalidOperationException"));
        let source: BoxedValue = Rc::new(Value {
            value: Cell::new(0),
        });
        let value = NullableMember::Value
            .read(Some(&source), inner)
            .unwrap()
            .unwrap();
        assert!(!Rc::ptr_eq(&source, &value));
        value.downcast_ref::<Value>().unwrap().value.set(99);
        assert_eq!(source.downcast_ref::<Value>().unwrap().value.get(), 0);
        assert_eq!(
            *NullableMember::HasValue
                .read(Some(&source), inner)
                .unwrap()
                .unwrap()
                .downcast_ref::<bool>()
                .unwrap(),
            true
        );
        let empty: BoxedValue = Rc::new(Option::<Value>::None);
        assert!(!*NullableMember::HasValue
            .read(Some(&empty), inner)
            .unwrap()
            .unwrap()
            .downcast_ref::<bool>()
            .unwrap());
        assert!(NullableMember::Value.read(Some(&empty), inner).is_err());
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        ValueTypes::register_cast::<i32, Value>(move |_| {
            count.set(count.get() + 1);
            Value {
                value: Cell::new(4),
            }
        });
        let wrong: BoxedValue = Rc::new(3i32);
        assert!(NullableMember::Value
            .read(Some(&wrong), inner)
            .err()
            .unwrap()
            .to_string()
            .contains("InvalidCastException"));
        assert_eq!(calls.get(), 0);
    }
    #[test]
    fn nullable_member_node_preserves_conditional_null_and_upstream_errors() {
        ValueTypes::register_nullable::<i32>();
        let node = NullableMemberNode::new(ValueType::of::<i32>(), NullableMember::HasValue, false);
        node.on_source_changed(None, None);
        assert!(!*node
            .state()
            .value()
            .unwrap()
            .downcast_ref::<bool>()
            .unwrap());
        let error: BoxedValue = Rc::new(BindingNotification::with_error(
            BindingError::message("source failure"),
            crate::data::BindingErrorType::Error,
        ));
        node.on_source_changed(Some(&error), None);
        assert!(Rc::ptr_eq(
            &node.state().value().unwrap(),
            &crate::FerroProperty::unset_value()
        ));
        let conditional =
            NullableMemberNode::new(ValueType::of::<i32>(), NullableMember::Value, true);
        conditional.on_source_changed(None, None);
        assert!(conditional.state().value().is_none());
    }
}
