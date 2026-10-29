//! Port of `Validation/Rules/MinValueValidationRule.cs` (`MinValueValidationRule<T>`).

use super::super::contract::IValidationRule;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectImpl,
    FerroProperty, Ref, StyledProperty,
};
use std::cmp::Ordering;

/// Validation rule that checks that a value is greater than or equal to a minimum value.
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference class is generic over the type `T` of the
/// value (`MinValueValidationRule<T>`, created in markup with `x:TypeArguments`); classes of the
/// class model are not generic, so the minimum is an untyped value, compared with the validated
/// value by the ordering of untyped values (`ValueTypes::compare`, which converts the minimum to
/// the type of the value: a minimum given as text in markup compares as a number with a number).
/// An unset minimum is null (the reference's default of a value type `T` is its zero), and values
/// that do not compare are not valid (the comparer of the reference throws).
#[repr(C)]
pub struct MinValueValidationRule {
    base: FerroObject,
}

ferro_class!(MinValueValidationRule: FerroObject);
ferro_class_info!(MinValueValidationRule { new: MinValueValidationRule::new });
ferroui_base::ferro_impl_classes!(MinValueValidationRule: FerroObjectImpl);

ferro_properties! {
    impl MinValueValidationRule {
        /// Identifies the `MinValue` property.
        pub fn min_value_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<MinValueValidationRule, _>("MinValue", None)
        }

        /// Identifies the `ErrorMessage` property.
        pub fn error_message_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<MinValueValidationRule, _>(
                "ErrorMessage",
                Some("Value is below minimum.".to_string()),
            )
        }
    }
}

impl MinValueValidationRule {
    /// The static constructor: the class implements the contract of the validation rules.
    fn static_constructor() {
        <dyn IValidationRule>::register::<MinValueValidationRule>();
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
    pub fn min_value(&self) -> Option<BoxedValue> {
        self.get_value(Self::min_value_property())
    }

    pub fn set_min_value(&self, value: Option<BoxedValue>) {
        self.set_value(Self::min_value_property(), value)
    }

    /// Gets or sets the error message for failed validation. This is a property of the object
    /// model.
    pub fn error_message(&self) -> Option<String> {
        self.get_value(Self::error_message_property())
    }

    pub fn set_error_message(&self, value: Option<String>) {
        self.set_value(Self::error_message_property(), value)
    }

    /// Validates the specified value: valid when it is not null and not below the minimum.
    pub fn validate(&self, value: Option<&BoxedValue>) -> bool {
        let (Some(value), Some(min_value)) = (value, self.min_value()) else {
            return false;
        };

        !matches!(ValueTypes::compare(value, &min_value), Some(Ordering::Less) | None)
    }
}

impl IValidationRule for MinValueValidationRule {
    fn error_message(&self) -> Option<String> {
        MinValueValidationRule::error_message(self)
    }

    fn set_error_message(&self, value: Option<String>) {
        MinValueValidationRule::set_error_message(self, value)
    }

    fn validate(&self, value: Option<&BoxedValue>) -> bool {
        MinValueValidationRule::validate(self, value)
    }
}
