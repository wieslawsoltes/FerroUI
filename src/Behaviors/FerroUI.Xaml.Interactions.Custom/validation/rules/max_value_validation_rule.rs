//! Port of `Validation/Rules/MaxValueValidationRule.cs` (`MaxValueValidationRule<T>`).

use super::super::contract::IValidationRule;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectImpl,
    FerroProperty, Ref, StyledProperty,
};
use std::cmp::Ordering;

/// Validation rule that checks that a value is less than or equal to a maximum value.
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference class is generic over the type `T` of the
/// value (`MaxValueValidationRule<T>`, created in markup with `x:TypeArguments`); classes of the
/// class model are not generic, so the maximum is an untyped value, compared with the validated
/// value by the ordering of untyped values (`ValueTypes::compare`, which converts the maximum to
/// the type of the value: a maximum given as text in markup compares as a number with a number).
/// An unset maximum is null (the reference's default of a value type `T` is its zero), and values
/// that do not compare are not valid (the comparer of the reference throws).
#[repr(C)]
pub struct MaxValueValidationRule {
    base: FerroObject,
}

ferro_class!(MaxValueValidationRule: FerroObject);
ferro_class_info!(MaxValueValidationRule { new: MaxValueValidationRule::new });
ferroui_base::ferro_impl_classes!(MaxValueValidationRule: FerroObjectImpl);

ferro_properties! {
    impl MaxValueValidationRule {
        /// Identifies the `MaxValue` property.
        pub fn max_value_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<MaxValueValidationRule, _>("MaxValue", None)
        }

        /// Identifies the `ErrorMessage` property.
        pub fn error_message_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<MaxValueValidationRule, _>(
                "ErrorMessage",
                Some("Value is above maximum.".to_string()),
            )
        }
    }
}

impl MaxValueValidationRule {
    /// The static constructor: the class implements the contract of the validation rules.
    fn static_constructor() {
        <dyn IValidationRule>::register::<MaxValueValidationRule>();
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    /// Creates a rule.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the maximum value. This is a property of the object model.
    pub fn max_value(&self) -> Option<BoxedValue> {
        self.get_value(Self::max_value_property())
    }

    pub fn set_max_value(&self, value: Option<BoxedValue>) {
        self.set_value(Self::max_value_property(), value)
    }

    /// Gets or sets the error message for failed validation. This is a property of the object
    /// model.
    pub fn error_message(&self) -> Option<String> {
        self.get_value(Self::error_message_property())
    }

    pub fn set_error_message(&self, value: Option<String>) {
        self.set_value(Self::error_message_property(), value)
    }

    /// Validates the specified value: valid when it is not null and not above the maximum.
    pub fn validate(&self, value: Option<&BoxedValue>) -> bool {
        let (Some(value), Some(max_value)) = (value, self.max_value()) else {
            return false;
        };

        !matches!(ValueTypes::compare(value, &max_value), Some(Ordering::Greater) | None)
    }
}

impl IValidationRule for MaxValueValidationRule {
    fn error_message(&self) -> Option<String> {
        MaxValueValidationRule::error_message(self)
    }

    fn set_error_message(&self, value: Option<String>) {
        MaxValueValidationRule::set_error_message(self, value)
    }

    fn validate(&self, value: Option<&BoxedValue>) -> bool {
        MaxValueValidationRule::validate(self, value)
    }
}
