//! `StdDevFixture` (`DynamicData.Tests/AggregationTests/StdDevFixture.cs`): the sample standard
//! deviation of the numeric overloads (`StdDev`) of caches and lists, with a seeded generator.
//!
//! Each `[Theory]` is one test per numeric type (`_int32`, `_int64`, `_single`, `_double`).
//! Not ported: the rows of `Decimal` - no decimal type (phase 1b).

use crate::domain::Randomizer;
use crate::utilities::{ObservableExtensions, TestSourceCache, TestSourceList};
use ferroui_dynamic_data::aggregation::{AggregationCacheEx, AggregationListEx};
use ferroui_dynamic_data::cache::{IConnectableCache, SourceCacheEx};
use ferroui_dynamic_data::list::{IObservableList, SourceListEditConvenienceEx};
use ferroui_system::IObservable;
use std::rc::Rc;

/// `Seed`.
const SEED: i32 = 0x2409_1071;

/// The numeric overloads under test (`nameof(Int32)`, ...).
#[derive(Clone, Copy, Debug)]
enum NumericType {
    Int32,
    Int64,
    Single,
    Double,
}

/// The standard deviation of the overload `numeric_type` of a cache of numbers.
fn cache_std_dev(source: &TestSourceCache<i32, i32>, numeric_type: NumericType, fallback: i32) -> Rc<dyn IObservable<f64>> {
    match numeric_type {
        NumericType::Int32 => AggregationCacheEx::std_dev(&source.connect(), |value: &i32| *value, fallback),
        NumericType::Int64 => AggregationCacheEx::std_dev(&source.connect(), |value: &i32| i64::from(*value), i64::from(fallback)),
        NumericType::Single => AggregationCacheEx::std_dev(&source.connect(), |value: &i32| *value as f32, fallback as f32),
        NumericType::Double => AggregationCacheEx::std_dev(&source.connect(), |value: &i32| f64::from(*value), f64::from(fallback)),
    }
}

/// The standard deviation of the overload `numeric_type` of a list of numbers.
fn list_std_dev(source: &TestSourceList<i32>, numeric_type: NumericType, fallback: i32) -> Rc<dyn IObservable<f64>> {
    match numeric_type {
        NumericType::Int32 => AggregationListEx::std_dev(&source.connect(), |value: &i32| *value, fallback),
        NumericType::Int64 => AggregationListEx::std_dev(&source.connect(), |value: &i32| i64::from(*value), i64::from(fallback)),
        NumericType::Single => AggregationListEx::std_dev(&source.connect(), |value: &i32| *value as f32, fallback as f32),
        NumericType::Double => AggregationListEx::std_dev(&source.connect(), |value: &i32| f64::from(*value), f64::from(fallback)),
    }
}

/// `SampleStandardDeviation`: the reference sample standard deviation, the values centred so that
/// the reference itself is not subject to the cancellation under test.
fn sample_standard_deviation(values: &[i32]) -> f64 {
    let offset = *values.iter().min().expect("values");
    let centred: Vec<f64> = values.iter().map(|value| f64::from(value - offset)).collect();
    let mean = centred.iter().sum::<f64>() / centred.len() as f64;
    (centred.iter().map(|value| (value - mean) * (value - mean)).sum::<f64>() / (centred.len() - 1) as f64).sqrt()
}

/// Asserts that `actual` is within `precision` of `expected` (`BeApproximately`).
fn assert_approximately(actual: f64, expected: f64, precision: f64, because: &str) {
    assert!((actual - expected).abs() <= precision, "expected {expected} within {precision}, found {actual}: {because}");
}

/// `Cache_ThreeEquallySpacedValues_ReportsTheirSpacing`.
fn cache_three_equally_spaced_values_reports_their_spacing(numeric_type: NumericType) {
    // Arrange
    let randomizer = Randomizer::new(SEED);
    let center = randomizer.int_range(-100, 100);
    let spacing = randomizer.int_range(1, 30);
    let values = [center - spacing, center, center + spacing];
    let fallback = randomizer.int_range(1, 100);
    let source = TestSourceCache::new(|value: &i32| *value);
    let standard_deviation = cache_std_dev(&source, numeric_type, fallback);
    let (_subscription, results) = standard_deviation.validate_synchronization().record_values();

    // Act
    source.add_or_update_items(values);

    // Assert
    assert!(results.error().is_none(), "all values are within the supported numeric ranges");
    assert_eq!(
        results.recorded_values(),
        vec![f64::from(spacing)],
        "one source edit produces one aggregate; the sample variance of three equally spaced values is the square of their spacing"
    );
}

#[test]
fn cache_three_equally_spaced_values_reports_their_spacing_int32() {
    cache_three_equally_spaced_values_reports_their_spacing(NumericType::Int32);
}

#[test]
fn cache_three_equally_spaced_values_reports_their_spacing_int64() {
    cache_three_equally_spaced_values_reports_their_spacing(NumericType::Int64);
}

#[test]
fn cache_three_equally_spaced_values_reports_their_spacing_single() {
    cache_three_equally_spaced_values_reports_their_spacing(NumericType::Single);
}

#[test]
fn cache_three_equally_spaced_values_reports_their_spacing_double() {
    cache_three_equally_spaced_values_reports_their_spacing(NumericType::Double);
}

// Not ported: Cache_ThreeEquallySpacedValues_ReportsTheirSpacing(Decimal) - no decimal type (phase 1b)

/// `List_ThreeEquallySpacedValues_ReportsTheirSpacing`.
fn list_three_equally_spaced_values_reports_their_spacing(numeric_type: NumericType) {
    // Arrange
    let randomizer = Randomizer::new(SEED);
    let center = randomizer.int_range(-100, 100);
    let spacing = randomizer.int_range(1, 30);
    let values = [center - spacing, center, center + spacing];
    let fallback = randomizer.int_range(1, 100);
    let source = TestSourceList::new();
    let standard_deviation = list_std_dev(&source, numeric_type, fallback);
    let (_subscription, results) = standard_deviation.validate_synchronization().record_values();

    // Act
    source.add_range(values);

    // Assert
    assert!(results.error().is_none(), "all values are within the supported numeric ranges");
    assert_eq!(
        results.recorded_values(),
        vec![f64::from(spacing)],
        "one source edit produces one aggregate; the sample variance of three equally spaced values is the square of their spacing"
    );
}

#[test]
fn list_three_equally_spaced_values_reports_their_spacing_int32() {
    list_three_equally_spaced_values_reports_their_spacing(NumericType::Int32);
}

#[test]
fn list_three_equally_spaced_values_reports_their_spacing_int64() {
    list_three_equally_spaced_values_reports_their_spacing(NumericType::Int64);
}

#[test]
fn list_three_equally_spaced_values_reports_their_spacing_single() {
    list_three_equally_spaced_values_reports_their_spacing(NumericType::Single);
}

#[test]
fn list_three_equally_spaced_values_reports_their_spacing_double() {
    list_three_equally_spaced_values_reports_their_spacing(NumericType::Double);
}

// Not ported: List_ThreeEquallySpacedValues_ReportsTheirSpacing(Decimal) - no decimal type (phase 1b)

/// `IntegerValues_FractionalMean_PreservesFractionalVariance`.
fn integer_values_fractional_mean_preserves_fractional_variance(numeric_type: NumericType) {
    // Arrange
    let randomizer = Randomizer::new(SEED);
    let start = randomizer.int_range(-100, 100);
    let spacing = randomizer.int_range(2, 30);
    let adjustment = randomizer.int_range(1, 2);
    let values = [start, start + spacing, start + spacing + spacing + adjustment];
    let mean = values.iter().map(|value| f64::from(*value)).sum::<f64>() / values.len() as f64;
    let expected = (values.iter().map(|value| (f64::from(*value) - mean).powi(2)).sum::<f64>() / (values.len() - 1) as f64).sqrt();
    let fallback = randomizer.int_range(1, 100);
    let source = TestSourceCache::new(|value: &i32| *value);
    let standard_deviation = cache_std_dev(&source, numeric_type, fallback);
    let (_subscription, results) = standard_deviation.validate_synchronization().record_values();

    // Act
    source.add_or_update_items(values);

    // Assert
    assert!(results.error().is_none(), "integer inputs with a fractional mean are valid");
    let recorded = results.recorded_values();
    assert_eq!(recorded.len(), 1, "one source edit produces one aggregate");
    assert_approximately(recorded[0], expected, 1e-10, "integer inputs do not imply an integer-valued mean or variance");
}

#[test]
fn integer_values_fractional_mean_preserves_fractional_variance_int32() {
    integer_values_fractional_mean_preserves_fractional_variance(NumericType::Int32);
}

#[test]
fn integer_values_fractional_mean_preserves_fractional_variance_int64() {
    integer_values_fractional_mean_preserves_fractional_variance(NumericType::Int64);
}

/// `IntegerValues_OffsetBeyondExactDoubleRange_ReportsTheirSpacing`.
fn integer_values_offset_beyond_exact_double_range_reports_their_spacing(numeric_type: NumericType) {
    // Arrange
    let randomizer = Randomizer::new(SEED);
    let offset = randomizer.int_range(100_000_000, 500_000_000);
    let spacing = randomizer.int_range(1, 30);
    let values = [offset, offset + spacing, offset + spacing + spacing];
    let fallback = randomizer.int_range(1, 100);
    let source = TestSourceCache::new(|value: &i32| *value);
    let standard_deviation = cache_std_dev(&source, numeric_type, fallback);
    let (_subscription, results) = standard_deviation.validate_synchronization().record_values();

    // Act
    source.add_or_update_items(values);

    // Assert
    assert!(results.error().is_none(), "the squared moments of these values remain within the accumulator range");
    assert_eq!(
        results.recorded_values(),
        vec![f64::from(spacing)],
        "one source edit produces one aggregate; standard deviation is translation invariant, so offsetting the values cannot change their spread"
    );
}

#[test]
fn integer_values_offset_beyond_exact_double_range_reports_their_spacing_int32() {
    integer_values_offset_beyond_exact_double_range_reports_their_spacing(NumericType::Int32);
}

#[test]
fn integer_values_offset_beyond_exact_double_range_reports_their_spacing_int64() {
    integer_values_offset_beyond_exact_double_range_reports_their_spacing(NumericType::Int64);
}

/// `IntegerValues_OffsetBeyondExactDoubleRangeThenRemovedAndReplaced_TracksTheirSpacing`.
fn integer_values_offset_beyond_exact_double_range_then_removed_and_replaced_tracks_their_spacing(numeric_type: NumericType) {
    // Arrange
    let randomizer = Randomizer::new(SEED);
    let offset = randomizer.int_range(100_000_000, 500_000_000);
    let spacing = randomizer.int_range(1, 30);
    let values = [offset, offset + spacing, offset + spacing + spacing, offset + spacing + spacing + spacing];
    let replacement = offset - spacing;
    let after_removal = [values[0], values[1], values[2]];
    let after_replacement = [replacement, values[1], values[2]];
    let fallback = randomizer.int_range(1, 100);
    let source = TestSourceList::new();
    let standard_deviation = list_std_dev(&source, numeric_type, fallback);
    let (_subscription, results) = standard_deviation.validate_synchronization().record_values();

    // Act
    source.add_range(values);
    source.remove(&values[3]);
    source.replace(&values[0], replacement);

    // Assert
    assert!(results.error().is_none(), "the squared moments of these values remain within the accumulator range");
    let recorded = results.recorded_values();
    assert_eq!(recorded.len(), 3, "each of the three source edits produces one aggregate");
    assert_approximately(recorded[0], sample_standard_deviation(&values), 1e-9, "the initial population is four equally spaced values");
    assert_approximately(recorded[1], sample_standard_deviation(&after_removal), 1e-9, "removing an item must recalculate the variance from the remaining offset values");
    assert_approximately(
        recorded[2],
        sample_standard_deviation(&after_replacement),
        1e-9,
        "replacing an item must recalculate the variance from the remaining offset values",
    );
}

#[test]
fn integer_values_offset_beyond_exact_double_range_then_removed_and_replaced_tracks_their_spacing_int32() {
    integer_values_offset_beyond_exact_double_range_then_removed_and_replaced_tracks_their_spacing(NumericType::Int32);
}

#[test]
fn integer_values_offset_beyond_exact_double_range_then_removed_and_replaced_tracks_their_spacing_int64() {
    integer_values_offset_beyond_exact_double_range_then_removed_and_replaced_tracks_their_spacing(NumericType::Int64);
}

/// `Cache_AllItemsRemoved_ReportsFallback`.
fn cache_all_items_removed_reports_fallback(numeric_type: NumericType) {
    // Arrange
    let randomizer = Randomizer::new(SEED);
    let center = randomizer.int_range(-100, 100);
    let spacing = randomizer.int_range(1, 30);
    let values = [center - spacing, center, center + spacing];
    let fallback = randomizer.int_range(1, 100);
    let source = TestSourceCache::new(|value: &i32| *value);
    source.add_or_update_items(values);
    let standard_deviation = cache_std_dev(&source, numeric_type, fallback);
    let (_subscription, results) = standard_deviation.validate_synchronization().record_values();

    // Act
    source.clear();

    // Assert
    assert!(results.error().is_none(), "an empty collection has a defined fallback");
    assert_eq!(results.recorded_values().last().copied(), Some(f64::from(fallback)), "sample variance is not calculated for fewer than two items");
}

#[test]
fn cache_all_items_removed_reports_fallback_int32() {
    cache_all_items_removed_reports_fallback(NumericType::Int32);
}

#[test]
fn cache_all_items_removed_reports_fallback_int64() {
    cache_all_items_removed_reports_fallback(NumericType::Int64);
}

#[test]
fn cache_all_items_removed_reports_fallback_single() {
    cache_all_items_removed_reports_fallback(NumericType::Single);
}

#[test]
fn cache_all_items_removed_reports_fallback_double() {
    cache_all_items_removed_reports_fallback(NumericType::Double);
}

// Not ported: Cache_AllItemsRemoved_ReportsFallback(Decimal) - no decimal type (phase 1b)
