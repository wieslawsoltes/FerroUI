//! Port of `Helpers/ComparisonConditionTypeHelper.cs`.

use super::member_lookup::{compare_values, is_comparable, type_name_of};
use super::TypeConverterHelper;
use crate::ComparisonConditionType;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::BoxedValue;
use std::cmp::Ordering;

/// Compares the operands of a condition.
pub struct ComparisonConditionTypeHelper;

impl ComparisonConditionTypeHelper {
    /// Compares `left_operand` and `right_operand` with `operator_type`.
    ///
    /// A text right operand is first converted to the type of the left operand. Two comparable
    /// operands are compared by order (after converting the right one to the type of the left
    /// one; operands that do not convert or compare are not equal); other operands support only
    /// [`Equal`](ComparisonConditionType::Equal) and
    /// [`NotEqual`](ComparisonConditionType::NotEqual).
    ///
    /// # Errors
    /// An ordering operator with an operand that is not comparable: the argument exception of the
    /// reference.
    pub fn compare(
        left_operand: Option<&BoxedValue>,
        operator_type: ComparisonConditionType,
        right_operand: Option<&BoxedValue>,
    ) -> Result<bool, String> {
        let left_operand = left_operand.cloned().and_then(ValueTypes::normalize);
        let mut right_operand = right_operand.cloned().and_then(ValueTypes::normalize);

        if let (Some(left), Some(right)) = (&left_operand, &right_operand) {
            if let Some(right_operand_string) = right.downcast_ref::<String>() {
                let left_operand_type = ValueType::of_value(&**left);
                // A conversion error keeps the original operand so comparable evaluation can use
                // non-equal semantics.
                if let Ok(Some(converted_right_operand)) =
                    TypeConverterHelper::convert(right_operand_string, left_operand_type)
                {
                    right_operand = Some(converted_right_operand);
                }
            }
        }

        let left_comparable_operand = left_operand.as_ref().filter(|operand| is_comparable(operand));
        let right_comparable_operand = right_operand.as_ref().filter(|operand| is_comparable(operand));
        if let (Some(left), Some(right)) = (left_comparable_operand, right_comparable_operand) {
            return Ok(Self::evaluate_comparable(left, operator_type, right));
        }

        match operator_type {
            ComparisonConditionType::Equal => Ok(ValueTypes::identity_equals(left_operand.as_ref(), right_operand.as_ref())),

            ComparisonConditionType::NotEqual => {
                Ok(!ValueTypes::identity_equals(left_operand.as_ref(), right_operand.as_ref()))
            }

            ComparisonConditionType::LessThan
            | ComparisonConditionType::LessThanOrEqual
            | ComparisonConditionType::GreaterThan
            | ComparisonConditionType::GreaterThanOrEqual => {
                let name = |operand: Option<&BoxedValue>| operand.map_or_else(|| "null".to_string(), type_name_of);
                Err(match (left_comparable_operand, right_comparable_operand) {
                    (None, None) => format!(
                        "Binding property of type {} and Value property of type {} cannot be used with operator {}.",
                        name(left_operand.as_ref()),
                        name(right_operand.as_ref()),
                        operator_type
                    ),
                    (None, Some(_)) => format!(
                        "Binding property of type {} cannot be used with operator {}.",
                        name(left_operand.as_ref()),
                        operator_type
                    ),
                    _ => format!(
                        "Value property of type {} cannot be used with operator {}.",
                        name(right_operand.as_ref()),
                        operator_type
                    ),
                })
            }
        }
    }

    /// Evaluates both operands that are comparable.
    fn evaluate_comparable(left_operand: &BoxedValue, operator_type: ComparisonConditionType, right_operand: &BoxedValue) -> bool {
        // `Convert.ChangeType(rightOperand, leftOperand.GetType())`: operands that cannot be
        // converted to a common comparable type are not equal.
        let Some(Some(converted_operand)) = ValueTypes::try_convert(Some(right_operand), ValueType::of_value(&**left_operand))
        else {
            return Self::is_non_equal_result(operator_type);
        };

        let Some(comparison) = compare_values(left_operand, &converted_operand) else {
            return Self::is_non_equal_result(operator_type);
        };

        match operator_type {
            ComparisonConditionType::Equal => comparison == Ordering::Equal,
            ComparisonConditionType::NotEqual => comparison != Ordering::Equal,
            ComparisonConditionType::LessThan => comparison == Ordering::Less,
            ComparisonConditionType::LessThanOrEqual => comparison != Ordering::Greater,
            ComparisonConditionType::GreaterThan => comparison == Ordering::Greater,
            ComparisonConditionType::GreaterThanOrEqual => comparison != Ordering::Less,
        }
    }

    fn is_non_equal_result(operator_type: ComparisonConditionType) -> bool {
        operator_type == ComparisonConditionType::NotEqual
    }
}
