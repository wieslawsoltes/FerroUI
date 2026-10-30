//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Custom/ValidationRuleTests.cs` of
//! Xaml.Behaviors (with its window `BindableValidationRule001.xaml`, loaded with the run-time
//! loader, and the binding source of the window, `ValidationRuleBindingSource`).
//!
//! Adapted: the jobs of the dispatcher run after showing the window (the `Loaded` event of the
//! slider, which validates, is queued); the rules are not generic, so their values are untyped
//! (`MaxValueValidationRule<double>` with a maximum of 40 is a rule whose maximum is the number
//! 40.0, `MaxValueValidationRule<int>` one whose maximum is the integer 10); the binding source is
//! registered as a type of the crate by the test, so that the window names it without a prefix.

use super::support::{find, load, run_jobs, start_app};
use crate::{
    MaxValueValidationRule, MinLengthValidationRule, MinValueValidationRule, NotNullValidationRule,
    PropertyValidationBehavior, RangeValidationRule, RegexValidationRule, RequiredDateValidationRule,
    RequiredDecimalValidationRule, RequiredTextValidationRule, SliderValidationBehavior,
};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectImpl,
    FerroProperty, Ref, StaticType, StyledProperty, TypeInfo,
};
use ferroui_controls::{Slider, TextBox, Window};
use ferroui_xaml_interactivity::Interaction;
use std::rc::Rc;

/// `ValidationRuleBindingSource` of `BindableValidationRule001.xaml.cs`: the data context of the
/// window, with the bounds and the error message the rule binds to.
#[repr(C)]
pub struct ValidationRuleBindingSource {
    base: FerroObject,
}

ferro_class!(ValidationRuleBindingSource: FerroObject);
ferro_class_info!(ValidationRuleBindingSource { new: ValidationRuleBindingSource::new });
ferroui_base::ferro_impl_classes!(ValidationRuleBindingSource: FerroObjectImpl);

ferro_properties! {
    impl ValidationRuleBindingSource {
        pub fn minimum_property() -> StyledProperty<f64> {
            FerroProperty::register::<ValidationRuleBindingSource, _>("Minimum", 0.0)
        }

        pub fn maximum_property() -> StyledProperty<f64> {
            FerroProperty::register::<ValidationRuleBindingSource, _>("Maximum", 0.0)
        }

        pub fn error_message_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<ValidationRuleBindingSource, _>("ErrorMessage", None)
        }
    }
}

impl ValidationRuleBindingSource {
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn set_maximum(&self, value: f64) {
        self.set_value(Self::maximum_property(), value)
    }

    pub fn error_message(&self) -> Option<String> {
        self.get_value(Self::error_message_property())
    }

    pub fn set_error_message(&self, value: Option<String>) {
        self.set_value(Self::error_message_property(), value)
    }
}

/// `new BindableValidationRule001()`.
fn bindable_validation_rule_001() -> Ref<Window> {
    TypeInfo::register_all(&[<ValidationRuleBindingSource as StaticType>::TYPE]);
    load::<Window>("BindableValidationRule001.xaml", include_str!("BindableValidationRule001.xaml"))
}

/// `Assert.IsType<ValidationRuleBindingSource>(window.DataContext)`.
fn binding_source(window: &Window) -> Ref<ValidationRuleBindingSource> {
    window
        .data_context()
        .and_then(|context| ValueTypes::as_object(&*context))
        .and_then(|object| object.cast::<ValidationRuleBindingSource>())
        .expect("the data context is a ValidationRuleBindingSource")
}

/// `Assert.IsType<SliderValidationBehavior>(Assert.Single(Interaction.GetBehaviors(window.TargetSlider)))`.
fn single_slider_validation_behavior(window: &Window) -> Ref<SliderValidationBehavior> {
    let slider = find::<Slider>(window, "TargetSlider");
    let behaviors = Interaction::get_behaviors(&slider).snapshot();
    assert_eq!(1, behaviors.len());
    behaviors[0].cast::<SliderValidationBehavior>().expect("a SliderValidationBehavior")
}

/// The integer an untyped value holds.
fn int_of(value: Option<BoxedValue>) -> Option<i32> {
    value.and_then(|value| value.downcast_ref::<i32>().copied())
}

/// The text a string property holds.
fn text(value: &str) -> Option<String> {
    Some(value.to_string())
}

#[test]
fn bound_range_changes_revalidate_associated_property() {
    let _app = start_app();
    let window = bindable_validation_rule_001();
    let source = binding_source(&window);
    let behavior = single_slider_validation_behavior(&window);

    window.show();
    run_jobs();

    assert!(behavior.is_valid());
    assert_eq!(None, behavior.error());

    source.set_maximum(40.0);

    assert!(!behavior.is_valid());
    assert_eq!(source.error_message(), behavior.error());

    source.set_error_message(text("The configured maximum is too small."));

    assert_eq!(source.error_message(), behavior.error());
}

#[test]
fn rule_collection_changes_update_subscriptions_and_validation() {
    let _app = start_app();
    let window = bindable_validation_rule_001();
    let behavior = single_slider_validation_behavior(&window);

    window.show();
    run_jobs();

    behavior.rules().clear();

    assert!(behavior.is_valid());
    assert_eq!(None, behavior.error());

    let rule = MaxValueValidationRule::new();
    rule.set_max_value(Some(Rc::new(40.0_f64)));
    rule.set_error_message(text("Maximum is 40."));
    behavior.rules().add(rule.clone().upcast());

    assert!(!behavior.is_valid());
    assert_eq!(rule.error_message(), behavior.error());

    rule.set_max_value(Some(Rc::new(100.0_f64)));

    assert!(behavior.is_valid());
    assert_eq!(None, behavior.error());
}

#[test]
fn validation_rules_expose_configuration_as_properties() {
    let _app = start_app();

    let max_value_rule = MaxValueValidationRule::new();
    assert_eq!(text("Value is above maximum."), max_value_rule.error_message());
    max_value_rule.set_value(MaxValueValidationRule::max_value_property(), Some(Rc::new(10_i32) as BoxedValue));
    max_value_rule.set_value(MaxValueValidationRule::error_message_property(), text("max"));
    assert_eq!(Some(10), int_of(max_value_rule.max_value()));
    assert_eq!(text("max"), max_value_rule.error_message());

    let min_length_rule = MinLengthValidationRule::new();
    assert_eq!(text("Value is too short."), min_length_rule.error_message());
    min_length_rule.set_value(MinLengthValidationRule::length_property(), 3);
    min_length_rule.set_value(MinLengthValidationRule::error_message_property(), text("length"));
    assert_eq!(3, min_length_rule.length());
    assert_eq!(text("length"), min_length_rule.error_message());

    let min_value_rule = MinValueValidationRule::new();
    assert_eq!(text("Value is below minimum."), min_value_rule.error_message());
    min_value_rule.set_value(MinValueValidationRule::min_value_property(), Some(Rc::new(1_i32) as BoxedValue));
    min_value_rule.set_value(MinValueValidationRule::error_message_property(), text("min"));
    assert_eq!(Some(1), int_of(min_value_rule.min_value()));
    assert_eq!(text("min"), min_value_rule.error_message());

    let not_null_rule = NotNullValidationRule::new();
    assert_eq!(text("Value is required."), not_null_rule.error_message());
    not_null_rule.set_value(NotNullValidationRule::error_message_property(), text("null"));
    assert_eq!(text("null"), not_null_rule.error_message());

    let range_rule = RangeValidationRule::new();
    assert_eq!(text("Value is out of range."), range_rule.error_message());
    range_rule.set_value(RangeValidationRule::minimum_property(), Some(Rc::new(2_i32) as BoxedValue));
    range_rule.set_value(RangeValidationRule::maximum_property(), Some(Rc::new(8_i32) as BoxedValue));
    range_rule.set_value(RangeValidationRule::error_message_property(), text("range"));
    assert_eq!(Some(2), int_of(range_rule.minimum()));
    assert_eq!(Some(8), int_of(range_rule.maximum()));
    assert_eq!(text("range"), range_rule.error_message());

    let regex_rule = RegexValidationRule::new();
    assert_eq!(text("Invalid format."), regex_rule.error_message());
    regex_rule.set_value(RegexValidationRule::pattern_property(), "^[0-9]+$".to_string());
    regex_rule.set_value(RegexValidationRule::error_message_property(), text("regex"));
    assert_eq!("^[0-9]+$", regex_rule.pattern());
    assert_eq!(text("regex"), regex_rule.error_message());

    let required_date_rule = RequiredDateValidationRule::new();
    assert_eq!(text("Date is required."), required_date_rule.error_message());
    required_date_rule.set_value(RequiredDateValidationRule::error_message_property(), text("date"));
    assert_eq!(text("date"), required_date_rule.error_message());

    let required_decimal_rule = RequiredDecimalValidationRule::new();
    assert_eq!(text("Value is required."), required_decimal_rule.error_message());
    required_decimal_rule.set_value(RequiredDecimalValidationRule::error_message_property(), text("decimal"));
    assert_eq!(text("decimal"), required_decimal_rule.error_message());

    let required_text_rule = RequiredTextValidationRule::new();
    assert_eq!(text("Value is required."), required_text_rule.error_message());
    required_text_rule.set_value(RequiredTextValidationRule::error_message_property(), text("text"));
    assert_eq!(text("text"), required_text_rule.error_message());
}

/// A text box whose text a validation behavior created by markup validates: the behavior without
/// type arguments, which markup cannot give a class of the object model.
const PROPERTY_VALIDATION_XAML: &str = r#"<Window xmlns="https://github.com/ferroui"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
  <TextBox x:Name="Box" Text="ab">
    <Interaction.Behaviors>
      <PropertyValidationBehavior Property="{x:Static TextBox.TextProperty}">
        <RequiredTextValidationRule ErrorMessage="Value is required." />
        <MinLengthValidationRule Length="3" ErrorMessage="Minimum length is 3." />
      </PropertyValidationBehavior>
    </Interaction.Behaviors>
  </TextBox>
</Window>"#;

// Not from the reference: `<PropertyValidationBehavior>` of a document (the reference closes the
// generic behavior with `x:TypeArguments="TextBox, sys:String"`) validates the property of the
// associated object whatever the type of its value.
#[test]
fn property_validation_behavior_is_created_by_markup() {
    let _app = start_app();
    let window = load::<Window>("PropertyValidationBehavior", PROPERTY_VALIDATION_XAML);
    let text_box = find::<TextBox>(&window, "Box");
    let behaviors = Interaction::get_behaviors(&text_box).snapshot();
    assert_eq!(1, behaviors.len());
    let behavior = behaviors[0].cast::<PropertyValidationBehavior>().expect("a PropertyValidationBehavior");
    assert_eq!(2, behavior.rules().count());

    window.show();
    run_jobs();

    assert!(!behavior.is_valid());
    assert_eq!(behavior.error().as_deref(), Some("Minimum length is 3."));

    text_box.set_text(Some("abc"));
    assert!(behavior.is_valid());
    assert!(behavior.error().is_none());

    text_box.set_text(Some(""));
    assert!(!behavior.is_valid());
    window.close();
}
