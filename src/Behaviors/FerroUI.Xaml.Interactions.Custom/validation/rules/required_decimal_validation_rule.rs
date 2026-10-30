//! Port of `Validation/Rules/RequiredDecimalValidationRule.cs`.

use super::super::contract::IValidationRule;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectImpl,
    FerroProperty, Ref, StyledProperty,
};

/// Validation rule that requires a decimal value.
///
/// The rule validates the value of a numeric up-down (a decimal, null when the control holds no
/// value).
#[repr(C)]
pub struct RequiredDecimalValidationRule {
    base: FerroObject,
}

ferro_class!(RequiredDecimalValidationRule: FerroObject);
ferro_class_info!(RequiredDecimalValidationRule { new: RequiredDecimalValidationRule::new });
ferroui_base::ferro_impl_classes!(RequiredDecimalValidationRule: FerroObjectImpl);

ferro_properties! {
    impl RequiredDecimalValidationRule {
        /// Identifies the `ErrorMessage` property.
        pub fn error_message_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<RequiredDecimalValidationRule, _>("ErrorMessage", Some("Value is required.".to_string()))
        }
    }
}

impl RequiredDecimalValidationRule {
    /// The static constructor: the class implements the contract of the validation rules.
    fn static_constructor() {
        <dyn IValidationRule>::register::<RequiredDecimalValidationRule>();
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

    /// Validates the specified value: valid when it is a value (not null).
    pub fn validate(&self, value: Option<&BoxedValue>) -> bool {
        value.is_some()
    }
}

impl IValidationRule for RequiredDecimalValidationRule {
    fn error_message(&self) -> Option<String> {
        RequiredDecimalValidationRule::error_message(self)
    }

    fn set_error_message(&self, value: Option<String>) {
        RequiredDecimalValidationRule::set_error_message(self, value)
    }

    fn validate(&self, value: Option<&BoxedValue>) -> bool {
        RequiredDecimalValidationRule::validate(self, value)
    }
}
