//! Port of `Validation/Rules/RangeValidationRule.cs` (`RangeValidationRule<T>`).

use super::super::contract::IValidationRule;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectImpl,
    FerroProperty, Ref, StyledProperty,
};
use std::cmp::Ordering;

/// Validation rule that checks that a value is within a range.
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference class is generic over the type `T` of the
/// value (`RangeValidationRule<T>`, created in markup with `x:TypeArguments`); classes of the class
/// model are not generic, so the bounds are untyped values, compared with the validated value by
/// the ordering of untyped values (`ValueTypes::compare`, which converts a bound to the type of
/// the value). Unset bounds are null (the reference's default of a value type `T` is its zero),
/// and values that do not compare are not valid (the comparer of the reference throws).
#[repr(C)]
pub struct RangeValidationRule {
    base: FerroObject,
}

ferro_class!(RangeValidationRule: FerroObject);
ferro_class_info!(RangeValidationRule { new: RangeValidationRule::new });
ferroui_base::ferro_impl_classes!(RangeValidationRule: FerroObjectImpl);

ferro_properties! {
    impl RangeValidationRule {
        /// Identifies the `Minimum` property.
        pub fn minimum_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<RangeValidationRule, _>("Minimum", None)
        }

        /// Identifies the `Maximum` property.
        pub fn maximum_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<RangeValidationRule, _>("Maximum", None)
        }

        /// Identifies the `ErrorMessage` property.
        pub fn error_message_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<RangeValidationRule, _>("ErrorMessage", Some("Value is out of range.".to_string()))
        }
    }
}

impl RangeValidationRule {
    /// The static constructor: the class implements the contract of the validation rules.
    fn static_constructor() {
        <dyn IValidationRule>::register::<RangeValidationRule>();
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    /// Creates a rule.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the minimum value. This is a property of the object model.
    pub fn minimum(&self) -> Option<BoxedValue> {
        self.get_value(Self::minimum_property())
    }

    pub fn set_minimum(&self, value: Option<BoxedValue>) {
        self.set_value(Self::minimum_property(), value)
    }

    /// Gets or sets the maximum value. This is a property of the object model.
    pub fn maximum(&self) -> Option<BoxedValue> {
        self.get_value(Self::maximum_property())
    }

    pub fn set_maximum(&self, value: Option<BoxedValue>) {
        self.set_value(Self::maximum_property(), value)
    }

    /// Gets or sets the error message for failed validation. This is a property of the object
    /// model.
    pub fn error_message(&self) -> Option<String> {
        self.get_value(Self::error_message_property())
    }

    pub fn set_error_message(&self, value: Option<String>) {
        self.set_value(Self::error_message_property(), value)
    }

    /// Validates the specified value: valid when it is not null and within the bounds.
    pub fn validate(&self, value: Option<&BoxedValue>) -> bool {
        let (Some(value), Some(minimum), Some(maximum)) = (value, self.minimum(), self.maximum()) else {
            return false;
        };

        if matches!(ValueTypes::compare(value, &minimum), Some(Ordering::Less) | None) {
            return false;
        }

        if matches!(ValueTypes::compare(value, &maximum), Some(Ordering::Greater) | None) {
            return false;
        }

        true
    }
}

impl IValidationRule for RangeValidationRule {
    fn error_message(&self) -> Option<String> {
        RangeValidationRule::error_message(self)
    }

    fn set_error_message(&self, value: Option<String>) {
        RangeValidationRule::set_error_message(self, value)
    }

    fn validate(&self, value: Option<&BoxedValue>) -> bool {
        RangeValidationRule::validate(self, value)
    }
}
