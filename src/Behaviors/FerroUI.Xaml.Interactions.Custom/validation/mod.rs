//! The behaviors that validate a property of their control with a set of rules, the contract of
//! the rules and the rules (`Validation/`).

mod combo_box_validation_behavior;
mod contract;
mod date_picker_validation_behavior;
mod numeric_up_down_validation_behavior;
mod property_validation_behavior;
mod rules;
mod slider_validation_behavior;
mod text_box_validation_behavior;

pub use self::combo_box_validation_behavior::ComboBoxValidationBehavior;
pub use self::contract::IValidationRule;
pub use self::date_picker_validation_behavior::DatePickerValidationBehavior;
pub use self::numeric_up_down_validation_behavior::NumericUpDownValidationBehavior;
pub use self::property_validation_behavior::{PropertyValidationBehavior, ValidationRules};
pub use self::rules::{
    MaxValueValidationRule, MinLengthValidationRule, MinValueValidationRule, NotNullValidationRule,
    RangeValidationRule, RegexValidationRule, RequiredDateValidationRule, RequiredDecimalValidationRule,
    RequiredTextValidationRule,
};
pub use self::slider_validation_behavior::SliderValidationBehavior;
pub use self::text_box_validation_behavior::TextBoxValidationBehavior;

use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_type;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::{FerroObject, Ref};

// The collection of the rules, whose items markup adds as the children of a validation behavior
// (its content).
ferro_markup_type!(class ValidationRules {
    handles: [ValidationRules, Option<ValidationRules>],
    constructors: [() => ValidationRules::new],
    properties: [
        Count: i32 { get: |list: &ValidationRules| list.count() as i32 },
    ],
    methods: [fn Add(Ref<FerroObject>) => |list: &ValidationRules, item: Ref<FerroObject>| list.add(item)],
});

/// The markup types of the folder: the collection of the rules.
pub(super) const VALIDATION_MARKUP_TYPES: &[&MarkupType] = &[<ValidationRules as MarkupTyped>::MARKUP];

/// What the untyped value conversions must know about the types of the folder: the nullable form
/// of the collection of the rules.
pub(super) fn register_validation_value_types() {
    ValueTypes::register_nullable::<ValidationRules>();
}
