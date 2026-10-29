//! Port of `Validation/Rules/RequiredTextValidationRule.cs`.

use super::super::contract::IValidationRule;
use super::super::contract::text_of;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectImpl,
    FerroProperty, Ref, StyledProperty,
};

/// Validation rule that requires a non-empty string value.
#[repr(C)]
pub struct RequiredTextValidationRule {
    base: FerroObject,
}

ferro_class!(RequiredTextValidationRule: FerroObject);
ferro_class_info!(RequiredTextValidationRule { new: RequiredTextValidationRule::new });
ferroui_base::ferro_impl_classes!(RequiredTextValidationRule: FerroObjectImpl);

ferro_properties! {
    impl RequiredTextValidationRule {
        /// Identifies the `ErrorMessage` property.
        pub fn error_message_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<RequiredTextValidationRule, _>("ErrorMessage", Some("Value is required.".to_string()))
        }
    }
}

impl RequiredTextValidationRule {
    /// The static constructor: the class implements the contract of the validation rules.
    fn static_constructor() {
        <dyn IValidationRule>::register::<RequiredTextValidationRule>();
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

    /// Validates the specified value: valid when it is a text that is not empty and not only white
    /// space.
    pub fn validate(&self, value: Option<&BoxedValue>) -> bool {
        text_of(value).is_some_and(|text| !text.trim().is_empty())
    }
}

impl IValidationRule for RequiredTextValidationRule {
    fn error_message(&self) -> Option<String> {
        RequiredTextValidationRule::error_message(self)
    }

    fn set_error_message(&self, value: Option<String>) {
        RequiredTextValidationRule::set_error_message(self, value)
    }

    fn validate(&self, value: Option<&BoxedValue>) -> bool {
        RequiredTextValidationRule::validate(self, value)
    }
}
