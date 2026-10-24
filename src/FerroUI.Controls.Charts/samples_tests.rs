// Behavioral counterparts of Microsoft WinUI Charts/APITests/ChartSamplesTests.cs.
// Copyright (c) Microsoft Corporation. Licensed under the MIT License.
// See NOTICE.md for the pinned source and the explicit collection adaptation.

use crate::{Chart, LineSeries, SampleValue, Samples};
use ferroui_base::animation::TimeSpan;
use ferroui_base::collections::FerroList;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::DateTimeOffset;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Control, ItemsSource, Window};
use std::cell::Cell;
use std::rc::Rc;

fn scope() -> (TextTestScope, UnitTestApplicationScope) {
    let text = TextTestScope::new();
    let app = UnitTestApplication::start(TestServices::styled_window());
    (text, app)
}

fn load(series: &Ref<LineSeries>) -> (Ref<Chart>, Ref<Window>) {
    let chart = Chart::new();
    chart.series().add(series).unwrap();
    let window = Window::new();
    window.set_width(400.0);
    window.set_height(300.0);
    window.set_content(Some(Control::boxed(&chart)));
    window.show();
    window.update_layout();
    Dispatcher::ui_thread().run_jobs(None);
    assert!(chart.is_loaded());
    (chart, window)
}

fn with_y(values: Ref<Samples>) -> Ref<LineSeries> {
    let series = LineSeries::new();
    series.set_x_values(Some(Samples::from_values(["A", "B", "C"])));
    series.set_y_values(Some(values));
    series
}

#[test]
fn samples_items_source_round_trips_null() {
    let _scope = scope();
    let samples = Samples::new();
    assert!(samples.items_source().is_none());
    samples.set_source(Some(ItemsSource::from_values([1.0])));
    samples.set_items_source(None);
    assert!(samples.items_source().is_none());
    assert!(samples.snapshot().is_empty());
}

#[test]
fn samples_items_source_dependency_property_updates_and_clears_value() {
    let _scope = scope();
    let samples = Samples::new();
    let source = ItemsSource::from_values([1.0, 2.0]);
    samples.set_value(
        Samples::items_source_property(),
        Some(Rc::new(source.clone()) as BoxedValue),
    );
    assert!(samples.source().unwrap().ptr_eq(&source));
    assert_eq!(samples.value_at(1), Some(SampleValue::Number(2.0)));
    samples.clear_value(Samples::items_source_property());
    assert!(samples.is_empty());
}

#[test]
fn unsupported_scalar_items_source_is_retained_and_does_not_prevent_layout() {
    let _scope = scope();
    let samples = Samples::new();
    let value: BoxedValue = Rc::new("Jan,Feb,Mar".to_owned());
    samples.set_items_source(Some(value.clone()));
    assert!(Rc::ptr_eq(&samples.items_source().unwrap(), &value));
    assert!(samples.source().is_none());
    assert!(samples.snapshot().is_empty());
    let series = with_y(samples.clone());
    let (chart, _window) = load(&series);
    assert!(chart.is_loaded());
    let scalar: BoxedValue = Rc::new(123i32);
    samples.set_items_source(Some(scalar.clone()));
    assert!(Rc::ptr_eq(&samples.items_source().unwrap(), &scalar));
    assert!(samples.snapshot().is_empty());
}

#[test]
fn registered_collection_binding_conversion_retains_live_source() {
    let _scope = scope();
    ItemsSource::register_binding_conversion::<Rc<FerroList<f64>>>();
    let list = Rc::new(FerroList::from_items([1.0, 2.0]));
    let raw: BoxedValue = Rc::new(list.clone());
    let samples = Samples::new();
    samples.set_items_source(Some(raw.clone()));
    assert!(Rc::ptr_eq(&samples.items_source().unwrap(), &raw));
    assert_eq!(samples.count(), 2);
    let revision = samples.revision();
    list.add(3.0);
    assert_eq!(samples.revision(), revision + 1);
    assert_eq!(samples.value_at(2), Some(SampleValue::Number(3.0)));
}

#[test]
fn numeric_list_items_source_can_load() {
    let _scope = scope();
    let samples = Samples::from_values(vec![1.0, -3.0, 2.0]);
    let (chart, _window) = load(&with_y(samples.clone()));
    assert_eq!(samples.count(), 3);
    assert_eq!(samples.value_at(1), Some(SampleValue::Number(-3.0)));
    assert!(chart.bounds().width > 0.0);
}

#[test]
fn numeric_array_items_source_can_load() {
    let _scope = scope();
    let samples = Samples::from_values([1.5, 2.5, 3.5]);
    let (_chart, _window) = load(&with_y(samples.clone()));
    assert_eq!(samples.value_at(2), Some(SampleValue::Number(3.5)));
}

#[test]
fn string_list_items_source_can_load_as_x_values() {
    let _scope = scope();
    let samples = Samples::from_values(vec!["A".to_owned(), "B".to_owned()]);
    let series = with_y(Samples::from_values([2.0, 4.0]));
    series.set_x_values(Some(samples.clone()));
    let (_chart, _window) = load(&series);
    assert_eq!(samples.value_at(1), Some(SampleValue::Category("B".into())));
}

#[test]
fn date_time_offset_list_items_source_can_load_as_x_values() {
    let _scope = scope();
    let dates = [
        DateTimeOffset::new(1601, 1, 1, 0, 0, 0, TimeSpan::ZERO),
        DateTimeOffset::UNIX_EPOCH,
    ];
    let samples = Samples::from_values(dates);
    let series = with_y(Samples::from_values([1.0, 2.0]));
    series.set_x_values(Some(samples.clone()));
    let (_chart, _window) = load(&series);
    assert_eq!(samples.value_at(0), Some(SampleValue::DateTime(dates[0])));
}

#[test]
fn boxed_numeric_items_source_can_load() {
    let _scope = scope();
    let values: Vec<BoxedValue> = vec![Rc::new(2.0), Rc::new(4.0)];
    let samples = Samples::from_source(values.into());
    let (_chart, _window) = load(&with_y(samples.clone()));
    assert_eq!(samples.value_at(1), Some(SampleValue::Number(4.0)));
}

#[test]
fn boxed_mixed_numeric_items_source_can_load() {
    let _scope = scope();
    let values: Vec<BoxedValue> = vec![Rc::new(2u8), Rc::new(-3i16), Rc::new(4.5f32)];
    let samples = Samples::from_source(values.into());
    let (_chart, _window) = load(&with_y(samples.clone()));
    assert_eq!(
        &*samples.snapshot(),
        &[
            Some(SampleValue::Number(2.0)),
            Some(SampleValue::Number(-3.0)),
            Some(SampleValue::Number(4.5))
        ]
    );
}

#[test]
fn boxed_string_items_source_can_load_as_x_values() {
    let _scope = scope();
    let values: Vec<BoxedValue> = vec![Rc::new("one".to_owned()), Rc::new("two".to_owned())];
    let samples = Samples::from_source(values.into());
    let series = with_y(Samples::from_values([1.0, 2.0]));
    series.set_x_values(Some(samples.clone()));
    let (_chart, _window) = load(&series);
    assert_eq!(
        samples.value_at(0),
        Some(SampleValue::Category("one".to_owned()))
    );
}

#[test]
fn empty_boxed_items_source_can_load_as_y_values() {
    let _scope = scope();
    let samples = Samples::from_source(Vec::<BoxedValue>::new().into());
    let (_chart, _window) = load(&with_y(samples.clone()));
    assert!(samples.is_empty());
    assert!(samples.get_at(0).is_none());
}

#[test]
fn unsupported_boxed_element_is_retained_and_does_not_prevent_layout() {
    let _scope = scope();
    let values: Vec<Option<BoxedValue>> = vec![
        Some(Rc::new(1.0)),
        Some(Rc::new(true)),
        None,
        Some(Rc::new(4.0)),
    ];
    let samples = Samples::from_source(values.into());
    let (_chart, _window) = load(&with_y(samples.clone()));
    assert_eq!(
        samples.get_at(1).unwrap().downcast_ref::<bool>(),
        Some(&true)
    );
    assert_eq!(
        &*samples.snapshot(),
        &[
            Some(SampleValue::Number(1.0)),
            None,
            None,
            Some(SampleValue::Number(4.0))
        ]
    );
}

#[test]
fn integer_and_float_sources_are_retained_without_set_validation() {
    let _scope = scope();
    let integers = Samples::from_values([i64::MIN, 0, i64::MAX]);
    assert_eq!(integers.count(), 3);
    assert_eq!(
        integers.value_at(0),
        Some(SampleValue::Number(i64::MIN as f64))
    );
    let floats = Samples::from_values([f32::INFINITY, f32::NAN, -1.0]);
    assert!(matches!(floats.value_at(0), Some(SampleValue::Number(value)) if value.is_infinite()));
    assert!(matches!(floats.value_at(1), Some(SampleValue::Number(value)) if value.is_nan()));
}

#[test]
fn iterator_source_is_retained_without_set_validation() {
    let _scope = scope();
    let reads = Rc::new(Cell::new(0));
    let counter = reads.clone();
    let samples = Samples::from_values((0..4).inspect(move |_| counter.set(counter.get() + 1)));
    assert_eq!(
        reads.get(),
        4,
        "An enumerable is materialized by the existing ItemsSource adapter."
    );
    assert_eq!(samples.value_at(3), Some(SampleValue::Number(3.0)));
    assert_eq!(reads.get(), 4);
}

#[test]
fn x_values_accept_strings_and_numbers() {
    let _scope = scope();
    let values: Vec<BoxedValue> = vec![Rc::new("a".to_owned()), Rc::new(12.0)];
    let samples = Samples::from_source(values.into());
    let series = with_y(Samples::from_values([1.0, 2.0]));
    series.set_x_values(Some(samples.clone()));
    let (_chart, _window) = load(&series);
    assert_eq!(samples.value_at(0), Some(SampleValue::Category("a".into())));
    assert_eq!(samples.value_at(1), Some(SampleValue::Number(12.0)));
}

#[test]
fn string_y_values_are_retained_but_do_not_prevent_layout() {
    let _scope = scope();
    let samples = Samples::from_values(["not a number", "also not"]);
    let (_chart, _window) = load(&with_y(samples.clone()));
    assert_eq!(samples.count(), 2);
    assert!(matches!(
        samples.value_at(0),
        Some(SampleValue::Category(_))
    ));
}

#[test]
fn null_samples_and_null_items_source_do_not_prevent_layout() {
    let _scope = scope();
    let series = LineSeries::new();
    let (chart, window) = load(&series);
    series.set_y_values(Some(Samples::new()));
    window.update_layout();
    assert!(chart.is_loaded());
    assert!(series.y_values().unwrap().is_empty());
}

#[test]
fn clearing_x_values_keeps_chart_usable() {
    let _scope = scope();
    let series = with_y(Samples::from_values([1.0, 2.0]));
    let (chart, window) = load(&series);
    series.set_x_values(None);
    window.update_layout();
    assert!(chart.is_loaded());
    assert!(series.x_values().is_none());
}

#[test]
fn x_values_can_flip_between_numeric_and_string_sources() {
    let _scope = scope();
    let series = with_y(Samples::from_values([1.0, 2.0]));
    let (chart, window) = load(&series);
    let samples = series.x_values().unwrap();
    samples.set_source(Some(ItemsSource::from_values([10, 20])));
    window.update_layout();
    assert_eq!(samples.value_at(1), Some(SampleValue::Number(20.0)));
    samples.set_source(Some(ItemsSource::from_values(["last", "first"])));
    window.update_layout();
    assert_eq!(
        samples.value_at(0),
        Some(SampleValue::Category("last".into()))
    );
    assert!(chart.is_loaded());
}

#[test]
fn replacing_samples_leaves_old_source_harmless() {
    let _scope = scope();
    let old_list = Rc::new(FerroList::from_items([1.0, 2.0]));
    let old_samples = Samples::from_source(old_list.clone().into());
    let series = with_y(old_samples.clone());
    let (chart, window) = load(&series);
    let replacement = Samples::from_values([8.0, 9.0]);
    series.set_y_values(Some(replacement.clone()));
    old_list.add(99.0);
    window.update_layout();
    assert_eq!(series.y_values(), Some(replacement));
    assert!(chart.is_loaded());
}

#[test]
fn swapping_items_source_leaves_old_source_harmless() {
    let _scope = scope();
    let old = Rc::new(FerroList::from_items([1.0]));
    let samples = Samples::from_source(old.clone().into());
    let next = Rc::new(FerroList::from_items([10.0]));
    samples.set_source(Some(next.clone().into()));
    assert!(!old.has_collection_changed_subscribers());
    let revision = samples.revision();
    old.add(2.0);
    assert_eq!(samples.revision(), revision);
    assert_eq!(samples.value_at(0), Some(SampleValue::Number(10.0)));
    next.set(0, 20.0);
    assert_eq!(samples.revision(), revision + 1);
    assert_eq!(samples.value_at(0), Some(SampleValue::Number(20.0)));
}

#[test]
fn numeric_collection_changes_while_loaded_keep_chart_usable() {
    let _scope = scope();
    let list = Rc::new(FerroList::from_items([1.0, 2.0]));
    let samples = Samples::from_source(list.clone().into());
    let series = with_y(samples.clone());
    let (chart, window) = load(&series);
    let old_snapshot = samples.snapshot();
    list.add(3.0);
    list.insert(0, -1.0);
    list.set(1, 12.0);
    list.move_item(0, 2);
    list.remove_at(0);
    window.update_layout();
    assert_eq!(
        &*samples.snapshot(),
        &[
            Some(SampleValue::Number(2.0)),
            Some(SampleValue::Number(-1.0)),
            Some(SampleValue::Number(3.0))
        ]
    );
    assert_eq!(
        old_snapshot.len(),
        2,
        "Retained snapshots survive later mutation."
    );
    list.clear();
    window.update_layout();
    assert!(samples.is_empty());
    assert!(chart.is_loaded());
}

#[test]
fn string_category_collection_changes_while_loaded_keep_chart_usable() {
    let _scope = scope();
    let list = Rc::new(FerroList::from_items(["a".to_owned(), "b".to_owned()]));
    let samples = Samples::from_source(list.clone().into());
    let series = with_y(Samples::from_values([1.0, 2.0]));
    series.set_x_values(Some(samples.clone()));
    let (chart, window) = load(&series);
    list.set(0, "renamed".into());
    list.remove_at(1);
    list.add("new".into());
    window.update_layout();
    assert_eq!(
        samples.value_at(0),
        Some(SampleValue::Category("renamed".into()))
    );
    assert_eq!(
        samples.value_at(1),
        Some(SampleValue::Category("new".into()))
    );
    assert!(chart.is_loaded());
}

#[test]
fn one_samples_instance_can_be_shared_by_two_series() {
    let _scope = scope();
    let list = Rc::new(FerroList::from_items([1.0, 2.0]));
    let samples = Samples::from_source(list.clone().into());
    let first = with_y(samples.clone());
    let second = with_y(samples.clone());
    let chart = Chart::new();
    chart.series().add(&first).unwrap();
    chart.series().add(&second).unwrap();
    list.set(0, 10.0);
    assert!(Rc::ptr_eq(
        &first.y_values().unwrap().snapshot(),
        &second.y_values().unwrap().snapshot()
    ));
    assert_eq!(samples.value_at(0), Some(SampleValue::Number(10.0)));
}

#[test]
fn samples_can_be_used_by_series_and_chart_samples_collection() {
    let _scope = scope();
    let samples = Samples::from_values([1.0]);
    let series = with_y(samples.clone());
    let chart = Chart::new();
    chart.data().add(&samples).unwrap();
    chart.series().add(&series).unwrap();
    let another = Chart::new();
    another.data().add(&samples).unwrap();
    assert_eq!(chart.data().get(0).unwrap(), another.data().get(0).unwrap());
}

#[test]
fn snapshot_decodes_once_per_revision_and_detaches_on_drop() {
    let _scope = scope();
    let list = Rc::new(FerroList::from_items([1.0, 2.0]));
    let samples = Samples::from_source(list.clone().into());
    let first = samples.snapshot();
    assert!(Rc::ptr_eq(&first, &samples.snapshot()));
    list.add(3.0);
    assert!(!Rc::ptr_eq(&first, &samples.snapshot()));
    let weak = samples.downgrade();
    drop(samples);
    assert!(weak.upgrade().is_none());
    assert!(!list.has_collection_changed_subscribers());
}

#[test]
fn sample_change_handlers_allow_reentrant_source_replacement_and_unsubscription() {
    let _scope = scope();
    let list = Rc::new(FerroList::from_items([1.0]));
    let samples = Samples::from_source(list.clone().into());
    let calls = Rc::new(Cell::new(0));
    let weak = samples.downgrade();
    let count = calls.clone();
    let token = samples.add_changed(Rc::new(move || {
        let previous = count.get();
        count.set(previous + 1);
        if previous == 0 {
            weak.upgrade()
                .unwrap()
                .set_source(Some(ItemsSource::from_values([42.0])));
        }
    }));
    list.add(2.0);
    assert_eq!(calls.get(), 2);
    assert_eq!(samples.value_at(0), Some(SampleValue::Number(42.0)));
    assert!(samples.remove_changed(token));
    samples.set_items_source(None);
    assert_eq!(calls.get(), 2);
}
