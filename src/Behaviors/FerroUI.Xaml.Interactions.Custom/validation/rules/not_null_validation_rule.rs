//! Port of `Validation/Rules/NotNullValidationRule.cs` (`NotNullValidationRule<T>`).

use super::super::contract::IValidationRule;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectImpl,
    FerroProperty, Ref, StyledProperty,
};

/// Validation rule that checks that a value is not null.
///
/// The type argument `T` of the reference only types the validated value, which is untyped here
/// (see [`IValidationRule`]).
#[repr(C)]
pub struct NotNullValidationRule {
    base: FerroObject,
}

ferro_class!(NotNullValidationRule: FerroObject);
ferro_class_info!(NotNullValidationRule { new: NotNullValidationRule::new });
ferroui_base::ferro_impl_classes!(NotNullValidationRule: FerroObjectImpl);

ferro_properties! {
    impl NotNullValidationRule {
        /// Identifies the `ErrorMessage` property.
        pub fn error_message_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<NotNullValidationRule, _>("ErrorMessage", Some("Value is required.".to_string()))
        }
    }
}

impl NotNullValidationRule {
    /// The static constructor: the class implements the contract of the validation rules.
    fn static_constructor() {
        <dyn IValidationRule>::register::<NotNullValidationRule>();
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

    /// Validates the specified value: valid when it is not null.
    pub fn validate(&self, value: Option<&BoxedValue>) -> bool {
        value.is_some()
    }
}

impl IValidationRule for NotNullValidationRule {
    fn error_message(&self) -> Option<String> {
        NotNullValidationRule::error_message(self)
    }

    fn set_error_message(&self, value: Option<String>) {
        NotNullValidationRule::set_error_message(self, value)
    }

    fn validate(&self, value: Option<&BoxedValue>) -> bool {
        NotNullValidationRule::validate(self, value)
    }
}
