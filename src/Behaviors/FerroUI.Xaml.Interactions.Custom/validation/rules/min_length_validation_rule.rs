//! Port of `Validation/Rules/MinLengthValidationRule.cs`.

use super::super::contract::text_of;
use super::super::contract::IValidationRule;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectImpl,
    FerroProperty, Ref, StyledProperty,
};

/// Validation rule that checks that a string has a minimum length.
#[repr(C)]
pub struct MinLengthValidationRule {
    base: FerroObject,
}

ferro_class!(MinLengthValidationRule: FerroObject);
ferro_class_info!(MinLengthValidationRule { new: MinLengthValidationRule::new });
ferroui_base::ferro_impl_classes!(MinLengthValidationRule: FerroObjectImpl);

ferro_properties! {
    impl MinLengthValidationRule {
        /// Identifies the `Length` property.
        pub fn length_property() -> StyledProperty<i32> {
            FerroProperty::register::<MinLengthValidationRule, _>("Length", 0)
        }

        /// Identifies the `ErrorMessage` property.
        pub fn error_message_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<MinLengthValidationRule, _>("ErrorMessage", Some("Value is too short.".to_string()))
        }
    }
}

impl MinLengthValidationRule {
    /// The static constructor: the class implements the contract of the validation rules.
    fn static_constructor() {
        <dyn IValidationRule>::register::<MinLengthValidationRule>();
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    /// Creates a rule.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the minimum length. This is a property of the object model.
    pub fn length(&self) -> i32 {
        self.get_value(Self::length_property())
    }

    pub fn set_length(&self, value: i32) {
        self.set_value(Self::length_property(), value)
    }

    /// Gets or sets the error message for failed validation. This is a property of the object
    /// model.
    pub fn error_message(&self) -> Option<String> {
        self.get_value(Self::error_message_property())
    }

    pub fn set_error_message(&self, value: Option<String>) {
        self.set_value(Self::error_message_property(), value)
    }

    /// Validates the specified value: valid when it is a text at least [`length`](Self::length)
    /// long (in UTF-16 code units, the length of a string of the reference).
    pub fn validate(&self, value: Option<&BoxedValue>) -> bool {
        text_of(value).is_some_and(|text| text.encode_utf16().count() as i64 >= i64::from(self.length()))
    }
}

impl IValidationRule for MinLengthValidationRule {
    fn error_message(&self) -> Option<String> {
        MinLengthValidationRule::error_message(self)
    }

    fn set_error_message(&self, value: Option<String>) {
        MinLengthValidationRule::set_error_message(self, value)
    }

    fn validate(&self, value: Option<&BoxedValue>) -> bool {
        MinLengthValidationRule::validate(self, value)
    }
}
