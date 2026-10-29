//! Port of `Validation/Rules/RequiredDateValidationRule.cs`.

use super::super::contract::IValidationRule;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectImpl,
    FerroProperty, Ref, StyledProperty,
};

/// Validation rule that requires a date value.
///
/// The rule validates the selected date of a date picker (a date and time with an offset, null
/// when no date is selected).
#[repr(C)]
pub struct RequiredDateValidationRule {
    base: FerroObject,
}

ferro_class!(RequiredDateValidationRule: FerroObject);
ferro_class_info!(RequiredDateValidationRule { new: RequiredDateValidationRule::new });
ferroui_base::ferro_impl_classes!(RequiredDateValidationRule: FerroObjectImpl);

ferro_properties! {
    impl RequiredDateValidationRule {
        /// Identifies the `ErrorMessage` property.
        pub fn error_message_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<RequiredDateValidationRule, _>("ErrorMessage", Some("Date is required.".to_string()))
        }
    }
}

impl RequiredDateValidationRule {
    /// The static constructor: the class implements the contract of the validation rules.
    fn static_constructor() {
        <dyn IValidationRule>::register::<RequiredDateValidationRule>();
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    /// Creates a rule.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the error message for failed validation. This is a property of the object
    /// model.
    pub fn error_message(&self) -> Option<String> {
        self.get_value(Self::error_message_property())
    }

    pub fn set_error_message(&self, value: Option<String>) {
        self.set_value(Self::error_message_property(), value)
    }

    /// Validates the specified value: valid when it is a date (not null).
    pub fn validate(&self, value: Option<&BoxedValue>) -> bool {
        value.is_some()
    }
}

impl IValidationRule for RequiredDateValidationRule {
    fn error_message(&self) -> Option<String> {
        RequiredDateValidationRule::error_message(self)
    }

    fn set_error_message(&self, value: Option<String>) {
        RequiredDateValidationRule::set_error_message(self, value)
    }

    fn validate(&self, value: Option<&BoxedValue>) -> bool {
        RequiredDateValidationRule::validate(self, value)
    }
}
