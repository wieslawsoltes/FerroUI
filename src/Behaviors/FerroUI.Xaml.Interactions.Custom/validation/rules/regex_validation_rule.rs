//! Port of `Validation/Rules/RegexValidationRule.cs`.

use super::super::contract::text_of;
use super::super::contract::IValidationRule;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectImpl,
    FerroProperty, Ref, StyledProperty,
};
use regex::Regex;
use std::cell::RefCell;

/// Validation rule that checks that a string matches a regular expression.
///
/// Deviation (DEVIATIONS.md, Behaviors): the pattern is a pattern of the `regex` crate, whose
/// syntax is that of the reference's regular expressions without look-around and
/// back-references; a pattern that does not compile matches nothing (the reference throws).
#[repr(C)]
pub struct RegexValidationRule {
    base: FerroObject,
    // The compiled pattern, kept while the pattern is the same (the reference's regular
    // expressions cache their compiled patterns).
    regex: RefCell<Option<(String, Option<Regex>)>>,
}

ferro_class!(RegexValidationRule: FerroObject);
ferro_class_info!(RegexValidationRule { new: RegexValidationRule::new });
ferroui_base::ferro_impl_classes!(RegexValidationRule: FerroObjectImpl);

ferro_properties! {
    impl RegexValidationRule {
        /// Identifies the `Pattern` property.
        pub fn pattern_property() -> StyledProperty<String> {
            FerroProperty::register::<RegexValidationRule, _>("Pattern", String::new())
        }

        /// Identifies the `ErrorMessage` property.
        pub fn error_message_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<RegexValidationRule, _>("ErrorMessage", Some("Invalid format.".to_string()))
        }
    }
}

impl RegexValidationRule {
    /// The static constructor: the class implements the contract of the validation rules.
    fn static_constructor() {
        <dyn IValidationRule>::register::<RegexValidationRule>();
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct(), regex: RefCell::new(None) }
    }

    /// Creates a rule.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the regular expression pattern. This is a property of the object model.
    pub fn pattern(&self) -> String {
        self.get_value(Self::pattern_property())
    }

    pub fn set_pattern(&self, value: String) {
        self.set_value(Self::pattern_property(), value)
    }

    /// Gets or sets the error message for failed validation. This is a property of the object
    /// model.
    pub fn error_message(&self) -> Option<String> {
        self.get_value(Self::error_message_property())
    }

    pub fn set_error_message(&self, value: Option<String>) {
        self.set_value(Self::error_message_property(), value)
    }

    /// Validates the specified value: valid when it is a text that matches the pattern.
    pub fn validate(&self, value: Option<&BoxedValue>) -> bool {
        let Some(text) = text_of(value) else {
            return false;
        };

        let pattern = self.pattern();
        let mut cache = self.regex.borrow_mut();
        if cache.as_ref().is_none_or(|(cached, _)| *cached != pattern) {
            let regex = Regex::new(&pattern).ok();
            *cache = Some((pattern, regex));
        }

        cache.as_ref().and_then(|(_, regex)| regex.as_ref()).is_some_and(|regex| regex.is_match(&text))
    }
}

impl IValidationRule for RegexValidationRule {
    fn error_message(&self) -> Option<String> {
        RegexValidationRule::error_message(self)
    }

    fn set_error_message(&self, value: Option<String>) {
        RegexValidationRule::set_error_message(self, value)
    }

    fn validate(&self, value: Option<&BoxedValue>) -> bool {
        RegexValidationRule::validate(self, value)
    }
}
