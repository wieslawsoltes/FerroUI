//! `WhenPropertyChangedFixture.UnitTests` (`DynamicData.Tests/Binding/WhenPropertyChangedFixture.UnitTests.Conversions.cs`):
//! the paths whose steps are converted (an expression's `Convert` nodes, [`PropertyChain::map`]).
//!
//! A conversion of `double` to `decimal` is [`Decimal::from_f64`], `(long)amount` is `as i64`,
//! a user-defined conversion operator is a `From` implementation, and a cast of a reference to a
//! type it already has is a step that reads the object itself.

use super::when_property_changed_fixture::{convert_amount, truncate_amount, unit_tests_randomizer, ChildModel, Decimal, IHasAge, Money, ObservablePrice, ParentModel};
use crate::domain::{fakers, Person};
use crate::utilities::ObservableExtensions;
use ferroui_dynamic_data::binding::{NotifyPropertyChangedEx, PropertyChain};
use ferroui_system::component_model::PropertyPath;
use ferroui_system::prop;
use std::rc::Rc;

/// `price => ((decimal)price.Amount).Scale`.
fn amount_scale() -> PropertyChain<Rc<ObservablePrice>, u8> {
    PropertyChain::new(prop!(ObservablePrice => amount)).map(|amount| Decimal::from_f64(amount).scale())
}

/// An odd number of `1..=ushort.MaxValue` over `divisor` (`(_randomizer.Int(1, ushort.MaxValue) | 1) / divisor`).
fn odd_fraction(randomizer: &crate::domain::Randomizer, divisor: f64) -> f64 {
    f64::from(randomizer.int_range(1, i32::from(u16::MAX)) | 1) / divisor
}

#[test]
fn numeric_conversion_before_property_access_initial_value_is_observed() {
    let randomizer = unit_tests_randomizer();

    // Arrange
    // An odd numerator produces an exactly representable, non-integral amount.
    let amount = odd_fraction(&randomizer, 4.0);
    let model = ObservablePrice::new(amount);
    let expected_scale = Decimal::from_f64(amount).scale();

    // Act
    let (subscription, results) = model.when_value_changed(amount_scale()).record_values();

    // Assert
    assert!(results.error().is_none(), "the next property belongs to the converted decimal, not the source double");
    assert_eq!(results.recorded_values(), vec![expected_scale], "the initial value must match the supplied expression");
    assert!(!results.has_completed(), "further amount changes remain observable");

    subscription.dispose();
}

/// Verifies that property changes remain observable through a value-changing numeric conversion.
fn numeric_conversion_before_property_access_property_changes_are_observed(notify_on_initial_value: bool) {
    let randomizer = unit_tests_randomizer();

    // Arrange
    // Odd quarters and eighths have distinct decimal scales without floating-point rounding.
    let initial_amount = odd_fraction(&randomizer, 4.0);
    let changed_amount = odd_fraction(&randomizer, 8.0);
    let model = ObservablePrice::new(initial_amount);
    let initial_scale = Decimal::from_f64(initial_amount).scale();
    let changed_scale = Decimal::from_f64(changed_amount).scale();
    let expected_scales = if notify_on_initial_value { vec![initial_scale, changed_scale] } else { vec![changed_scale] };
    let (subscription, results) = model.when_value_changed_with(amount_scale(), notify_on_initial_value).record_values();

    // Act
    model.set_amount(changed_amount);

    // Assert
    assert!(results.error().is_none(), "conversion must be applied on both initial and subsequent chain reads");
    assert_eq!(results.recorded_values(), expected_scales, "notifications must match the expression and initial-value option");
    assert!(!results.has_completed(), "amount changes do not complete the observation");

    subscription.dispose();
}

#[test]
fn numeric_conversion_before_property_access_property_changes_are_observed_case1() {
    numeric_conversion_before_property_access_property_changes_are_observed(false);
}

#[test]
fn numeric_conversion_before_property_access_property_changes_are_observed_case2() {
    numeric_conversion_before_property_access_property_changes_are_observed(true);
}

/// Verifies that a numeric conversion at the end of a property path preserves initial and changed values.
#[test]
fn numeric_conversion_at_leaf_property_changes_are_observed() {
    let randomizer = unit_tests_randomizer();

    // Arrange
    let initial_amount = randomizer.double();
    let changed_amount = initial_amount + randomizer.double_range(1.0, 2.0);
    let model = ObservablePrice::new(initial_amount);
    let expected_amounts = vec![Decimal::from_f64(initial_amount), Decimal::from_f64(changed_amount)];
    // price => (decimal)price.Amount
    let (subscription, results) = model.when_value_changed(PropertyChain::new(prop!(ObservablePrice => amount)).map(Decimal::from_f64)).record_values();

    // Act
    model.set_amount(changed_amount);

    // Assert
    assert!(results.error().is_none(), "a conversion must also remain supported as the final expression step");
    assert_eq!(results.recorded_values(), expected_amounts, "each observed value must be converted to the requested type");
    assert!(!results.has_completed(), "further amount changes remain observable");

    subscription.dispose();
}

/// Verifies that a reference cast preserves observation of properties on the runtime model.
#[test]
fn reference_cast_before_property_access_property_changes_are_observed() {
    let randomizer = unit_tests_randomizer();

    // Arrange
    let person = fakers::person().with_seed(&randomizer).generate_one();
    // INotifyPropertyChanged model = person: the model is the person itself, and the cast
    // `(Person)source` is a step that reads the object, without notifications of its own.
    let model = person.clone();
    let initial_age = person.age();
    let changed_age = initial_age + randomizer.int_range(1, i32::from(u8::MAX));
    // source => ((Person)source).Age
    let path = PropertyPath::value("(Person)", |source: &Rc<Person>| source.clone()).then(prop!(Person => age));
    let (subscription, results) = model.when_value_changed(path).record_values();

    // Act
    person.set_age(changed_age);

    // Assert
    assert!(results.error().is_none(), "supported reference casts must preserve property observation");
    assert_eq!(results.recorded_values(), vec![initial_age, changed_age], "the runtime model remains the notification source");
    assert!(!results.has_completed(), "further age changes remain observable");

    subscription.dispose();
}

/// Verifies that an interface cast preserves observation after replacing an intermediate object.
#[test]
fn interface_cast_before_property_access_replacement_child_changes_are_observed() {
    let randomizer = unit_tests_randomizer();

    // Arrange
    let initial_age = randomizer.int_range(1, i32::from(u8::MAX));
    let replacement_age = initial_age + randomizer.int_range(1, i32::from(u8::MAX));
    let changed_age = replacement_age + randomizer.int_range(1, i32::from(u8::MAX));
    let parent = ParentModel::new(Some(ChildModel::new(initial_age)));
    let replacement = ChildModel::new(replacement_age);
    // source => ((IHasAge)source.Child!).Age
    let path = PropertyChain::nullable(prop!(ParentModel => child)).then(PropertyPath::property("Age", |child: &Rc<ChildModel>| <ChildModel as IHasAge>::age(child)));
    let (subscription, results) = parent.when_value_changed(path).record_values();
    parent.set_child(Some(replacement.clone()));

    // Act
    replacement.set_age(changed_age);

    // Assert
    assert!(results.error().is_none(), "supported interface casts must preserve nested property observation");
    assert_eq!(
        results.recorded_values(),
        vec![initial_age, replacement_age, changed_age],
        "the interface property must follow the replacement child and its subsequent changes"
    );
    assert!(!results.has_completed(), "further child changes remain observable");

    subscription.dispose();
}

/// Verifies that converted properties retain independent values and notification sources.
fn numeric_conversions_on_different_properties_changes_are_independent(change_amount: bool) {
    let randomizer = unit_tests_randomizer();

    // Arrange
    let initial_amount = odd_fraction(&randomizer, 4.0);
    let initial_other_amount = odd_fraction(&randomizer, 8.0);
    let changed_amount = odd_fraction(&randomizer, 16.0);
    let model = ObservablePrice::with_amounts(initial_amount, initial_other_amount);
    let initial_scale = Decimal::from_f64(initial_amount).scale();
    let initial_other_scale = Decimal::from_f64(initial_other_amount).scale();
    let changed_scale = Decimal::from_f64(changed_amount).scale();
    let expected_scales = if change_amount { vec![initial_scale, changed_scale] } else { vec![initial_scale] };
    let expected_other_scales = if change_amount { vec![initial_other_scale] } else { vec![initial_other_scale, changed_scale] };
    let (amount_subscription, amounts) = model.when_value_changed(amount_scale()).record_values();
    // price => ((decimal)price.OtherAmount).Scale
    let (other_subscription, other_amounts) =
        model.when_value_changed(PropertyChain::new(prop!(ObservablePrice => other_amount)).map(|amount| Decimal::from_f64(amount).scale())).record_values();

    // Act
    if change_amount {
        model.set_amount(changed_amount);
    } else {
        model.set_other_amount(changed_amount);
    }

    // Assert
    assert!(amounts.error().is_none(), "the first converted property remains observable");
    assert!(other_amounts.error().is_none(), "the second converted property remains observable");
    assert_eq!(amounts.recorded_values(), expected_scales, "the first path observes only its own property");
    assert_eq!(other_amounts.recorded_values(), expected_other_scales, "the second path observes only its own property");
    assert!(!amounts.has_completed(), "the first observation remains active");
    assert!(!other_amounts.has_completed(), "the second observation remains active");

    other_subscription.dispose();
    amount_subscription.dispose();
}

#[test]
fn numeric_conversions_on_different_properties_changes_are_independent_case1() {
    numeric_conversions_on_different_properties_changes_are_independent(false);
}

#[test]
fn numeric_conversions_on_different_properties_changes_are_independent_case2() {
    numeric_conversions_on_different_properties_changes_are_independent(true);
}

/// Verifies that different conversions of one property retain their own evaluation paths.
#[test]
fn different_conversion_paths_on_same_property_changes_use_each_conversion() {
    let randomizer = unit_tests_randomizer();

    // Arrange
    let initial_amount = odd_fraction(&randomizer, 4.0);
    let changed_amount = odd_fraction(&randomizer, 8.0);
    let model = ObservablePrice::new(initial_amount);
    let expected_scales = vec![Decimal::from_f64(initial_amount).scale(), Decimal::from_f64(changed_amount).scale()];
    let expected_truncated_scales = vec![Decimal::from_i64(initial_amount as i64).scale(), Decimal::from_i64(changed_amount as i64).scale()];
    let (exact_subscription, exact) = model.when_value_changed(amount_scale()).record_values();
    // price => ((decimal)(long)price.Amount).Scale
    let (truncated_subscription, truncated) =
        model.when_value_changed(PropertyChain::new(prop!(ObservablePrice => amount)).map(|amount| Decimal::from_i64(amount as i64).scale())).record_values();

    // Act
    model.set_amount(changed_amount);

    // Assert
    assert!(exact.error().is_none(), "the direct conversion remains observable");
    assert!(truncated.error().is_none(), "the conversion through an integer remains observable");
    assert_eq!(exact.recorded_values(), expected_scales, "the direct path preserves fractional digits");
    assert_eq!(truncated.recorded_values(), expected_truncated_scales, "the integer conversion discards fractional digits");
    assert!(!exact.has_completed(), "the direct observation remains active");
    assert!(!truncated.has_completed(), "the integer-converted observation remains active");

    truncated_subscription.dispose();
    exact_subscription.dispose();
}

/// Verifies that conversions to different result types do not share incompatible property factories.
#[test]
fn leaf_conversions_with_different_result_types_changes_use_each_result_type() {
    let randomizer = unit_tests_randomizer();

    // Arrange
    let initial_amount = odd_fraction(&randomizer, 4.0);
    let changed_amount = initial_amount + f64::from(randomizer.int_range(1, i32::from(u16::MAX)));
    let model = ObservablePrice::new(initial_amount);
    let expected_decimals = vec![Decimal::from_f64(initial_amount), Decimal::from_f64(changed_amount)];
    let expected_integers = vec![initial_amount as i64, changed_amount as i64];
    // price => (decimal)price.Amount
    let (decimal_subscription, decimals) = model.when_value_changed(PropertyChain::new(prop!(ObservablePrice => amount)).map(Decimal::from_f64)).record_values();
    // price => (long)price.Amount
    let (integer_subscription, integers) = model.when_value_changed(PropertyChain::new(prop!(ObservablePrice => amount)).map(|amount| amount as i64)).record_values();

    // Act
    model.set_amount(changed_amount);

    // Assert
    assert!(decimals.error().is_none(), "the decimal result must have a compatible factory");
    assert!(integers.error().is_none(), "the integer result must have a compatible factory");
    assert_eq!(decimals.recorded_values(), expected_decimals, "the decimal path preserves fractional values");
    assert_eq!(integers.recorded_values(), expected_integers, "the integer path truncates fractional values");
    assert!(!decimals.has_completed(), "the decimal observation remains active");
    assert!(!integers.has_completed(), "the integer observation remains active");

    integer_subscription.dispose();
    decimal_subscription.dispose();
}

/// Verifies that conversion methods with matching source and result types retain distinct behavior.
///
/// Upstream builds the two expressions with `Expression.Convert` and a conversion method; here
/// each path maps the amount with its method.
#[test]
fn conversion_methods_with_same_signature_changes_use_each_method() {
    let randomizer = unit_tests_randomizer();

    // Arrange
    let initial_amount = odd_fraction(&randomizer, 4.0);
    let changed_amount = odd_fraction(&randomizer, 8.0);
    let model = ObservablePrice::new(initial_amount);
    let exact_conversion: fn(f64) -> Decimal = convert_amount;
    let truncated_conversion: fn(f64) -> Decimal = truncate_amount;
    let exact_path = PropertyChain::new(prop!(ObservablePrice => amount)).map(exact_conversion).map(Decimal::scale);
    let truncated_path = PropertyChain::new(prop!(ObservablePrice => amount)).map(truncated_conversion).map(Decimal::scale);
    let expected_scales = vec![exact_conversion(initial_amount).scale(), exact_conversion(changed_amount).scale()];
    let expected_truncated_scales = vec![truncated_conversion(initial_amount).scale(), truncated_conversion(changed_amount).scale()];
    let (exact_subscription, exact) = model.when_value_changed(exact_path).record_values();
    let (truncated_subscription, truncated) = model.when_value_changed(truncated_path).record_values();

    // Act
    model.set_amount(changed_amount);

    // Assert
    assert!(exact.error().is_none(), "the fractional conversion method remains observable");
    assert!(truncated.error().is_none(), "the truncating conversion method remains observable");
    assert_eq!(exact.recorded_values(), expected_scales, "the fractional conversion method belongs to this path");
    assert_eq!(truncated.recorded_values(), expected_truncated_scales, "the truncating conversion method belongs to this path");
    assert!(!exact.has_completed(), "the fractional observation remains active");
    assert!(!truncated.has_completed(), "the truncating observation remains active");

    truncated_subscription.dispose();
    exact_subscription.dispose();
}

/// `price => ((Money)price.Amount).WholeUnits`.
fn money_whole_units() -> PropertyChain<Rc<ObservablePrice>, i32> {
    PropertyChain::new(prop!(ObservablePrice => amount)).map(Money::from).map(|money| money.whole_units())
}

/// Verifies that a user-defined conversion operator is applied before reading a property of its result.
#[test]
fn user_defined_conversion_before_property_access_property_changes_are_observed() {
    let randomizer = unit_tests_randomizer();

    // Arrange
    // A fractional amount proves the operator ran, because it keeps only whole units.
    let initial_amount = f64::from(randomizer.int_range(1, i32::from(u16::MAX))) + 0.5;
    let changed_amount = initial_amount + f64::from(randomizer.int_range(1, i32::from(u16::MAX)));
    let model = ObservablePrice::new(initial_amount);
    let expected_units = vec![Money::from(initial_amount).whole_units(), Money::from(changed_amount).whole_units()];
    let (subscription, results) = model.when_value_changed(money_whole_units()).record_values();

    // Act
    model.set_amount(changed_amount);

    // Assert
    assert!(results.error().is_none(), "a user-defined conversion operator must run before the next property is read");
    assert_eq!(results.recorded_values(), expected_units, "each observed value must be read from the converted result");
    assert!(!results.has_completed(), "further amount changes remain observable");

    subscription.dispose();
}

/// Verifies that a user-defined conversion keeps its own evaluation path alongside a built-in conversion.
#[test]
fn user_defined_and_built_in_conversions_on_same_property_changes_use_each_conversion() {
    let randomizer = unit_tests_randomizer();

    // Arrange
    let initial_amount = f64::from(randomizer.int_range(1, i32::from(u16::MAX))) + 0.5;
    let changed_amount = initial_amount + f64::from(randomizer.int_range(1, i32::from(u16::MAX)));
    let model = ObservablePrice::new(initial_amount);
    let expected_units = vec![Money::from(initial_amount).whole_units(), Money::from(changed_amount).whole_units()];
    let expected_scales = vec![Decimal::from_f64(initial_amount).scale(), Decimal::from_f64(changed_amount).scale()];
    let (money_subscription, units) = model.when_value_changed(money_whole_units()).record_values();
    let (decimal_subscription, scales) = model.when_value_changed(amount_scale()).record_values();

    // Act
    model.set_amount(changed_amount);

    // Assert
    assert!(units.error().is_none(), "the user-defined conversion must have a compatible factory");
    assert!(scales.error().is_none(), "the built-in conversion must have a compatible factory");
    assert_eq!(units.recorded_values(), expected_units, "the user-defined path reports whole units");
    assert_eq!(scales.recorded_values(), expected_scales, "the built-in path reports the decimal scale");
    assert!(!units.has_completed(), "the user-defined observation remains active");
    assert!(!scales.has_completed(), "the built-in observation remains active");

    decimal_subscription.dispose();
    money_subscription.dispose();
}

// Not ported: EquivalentConversionPaths_DifferentParameterNames_ReuseFactory - reflection (ObservablePropertyFactoryCache, the cache of the factories of parsed expressions, is not ported)
