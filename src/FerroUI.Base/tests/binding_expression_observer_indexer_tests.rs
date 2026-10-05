//! Port of the upstream `ExpressionObserverBuilderTests_Indexer` tests:
//! binding expressions built from a string path with an indexer, observed
//! directly.

use super::binding_expression_tests_indexer::{
    parse, strings, DoubleIndexerData, DoubleKeyDictionary, IntDictionaryData, IntKeyDictionaryData,
    NonIntegerIndexer, NonIntegerIndexerData, StringArrayData, StringDictionaryData, StringListData,
};
use super::*;
use crate::data::core::{BindingExpression, BindingExpressionOptions, UntypedBindingExpression, Value, ValueTypes};
use crate::data::model::{BindableArray, BindableDictionary, BindableList, INotifyCollectionChanged, Model};
use crate::data::{BindingChainException, BindingError, BindingErrorType, BindingNotification};
use crate::reactive::AnonymousObserver;
use crate::{ferro_model, UnsetValueType};

/// `new { Foo = 5 }`.
struct IntData {
    foo: i32,
}

ferro_model!(IntData, |b| b.read_only::<Value<i32>>("Foo", |o| o.foo));

const UNSET: &str = "(unset)";

fn build(source: BoxedValue, path: &str) -> Rc<BindingExpression> {
    BindingExpression::new(Some(source), parse(path), BindingExpressionOptions::default())
}

/// The text of a produced value: the value of a notification, with the unset
/// marker shown as [`UNSET`].
fn extract_text(value: Option<BoxedValue>) -> String {
    let value = BindingNotification::extract_value(value.as_ref());
    match &value {
        Some(v) if v.is::<UnsetValueType>() => UNSET.to_string(),
        _ => ValueTypes::to_display_string(value.as_ref()),
    }
}

/// Subscribes to the expression, collecting what it produces.
fn subscribe(target: &Rc<BindingExpression>) -> (Rc<RefCell<Vec<Option<BoxedValue>>>>, Rc<dyn IDisposable>) {
    let result = Rc::new(RefCell::new(Vec::new()));
    let sink = result.clone();
    let subscription =
        target.to_observable(None).subscribe(Rc::new(AnonymousObserver::new(move |v| sink.borrow_mut().push(v))));
    (result, subscription)
}

/// The first value the expression produces (`await target.Take(1)`).
fn first(source: BoxedValue, path: &str) -> Option<BoxedValue> {
    let target = build(source, path);
    let (result, subscription) = subscribe(&target);
    let first = result.borrow().first().cloned().expect("the expression produced a value");
    subscription.dispose();
    first
}

fn texts(result: &Rc<RefCell<Vec<Option<BoxedValue>>>>) -> Vec<String> {
    result.borrow().iter().cloned().map(extract_text).collect()
}

fn string_array(items: &[&str]) -> Rc<StringArrayData> {
    StringArrayData::new(BindableArray::new(strings(items)))
}

fn string_list(items: &[&str]) -> Rc<StringListData> {
    StringListData::new(BindableList::new(strings(items)))
}

#[test]
fn should_get_array_value() {
    let data = string_array(&["foo", "bar"]);
    let result = first(data.clone(), "Foo[1]");

    assert_eq!("bar", extract_text(result));
}

#[test]
fn should_get_unset_value_for_invalid_array_index() {
    let data = string_array(&["foo", "bar"]);
    let result = first(data.clone(), "Foo[invalid]");

    assert_eq!(UNSET, extract_text(result));
}

#[test]
fn should_get_unset_value_for_invalid_dictionary_index() {
    let data = IntKeyDictionaryData::new(BindableDictionary::new([(1, s("foo"))]));
    let result = first(data.clone(), "Foo[invalid]");

    assert_eq!(UNSET, extract_text(result));
}

#[test]
fn should_get_error_for_object_without_indexer() {
    let data = Model::new_model(IntData { foo: 5 });
    let result = first(data.clone(), "Foo[noindexer]").expect("a notification");

    let expected = BindingNotification::with_error(
        BindingError::new(BindingChainException::with_expression(
            "Type 'System.Int32' does not have an indexer.",
            "Foo[noindexer]",
            "[noindexer]",
        )),
        BindingErrorType::Error,
    );
    assert_eq!(Some(&expected), result.downcast_ref::<BindingNotification>());
}

#[test]
fn should_get_multi_dimensional_array_value() {
    let data = StringArrayData::new(BindableArray::with_lengths(&[2, 2], strings(&["foo", "bar", "baz", "qux"])));
    let result = first(data.clone(), "Foo[1, 1]");

    assert_eq!("qux", extract_text(result));
}

#[test]
fn should_get_value_for_string_indexer() {
    let data = StringDictionaryData::new(BindableDictionary::new([(s("foo"), s("bar")), (s("baz"), s("qux"))]));
    let result = first(data.clone(), "Foo[foo]");

    assert_eq!("bar", extract_text(result));
}

#[test]
fn should_get_value_for_non_string_indexer() {
    let data = DoubleIndexerData::new(DoubleKeyDictionary::new(&[(1.0, "bar"), (2.0, "qux")]));
    let result = first(data.clone(), "Foo[1.0]");

    assert_eq!("bar", extract_text(result));
}

#[test]
fn array_out_of_bounds_should_return_unset_value() {
    let data = string_array(&["foo", "bar"]);
    let result = first(data.clone(), "Foo[2]");

    assert_eq!(UNSET, extract_text(result));
}

#[test]
fn array_with_wrong_dimensions_should_return_unset_value() {
    let data = string_array(&["foo", "bar"]);
    let result = first(data.clone(), "Foo[1,2]");

    assert_eq!(UNSET, extract_text(result));
}

#[test]
fn list_out_of_bounds_should_return_unset_value() {
    let data = string_list(&["foo", "bar"]);
    let result = first(data.clone(), "Foo[2]");

    assert_eq!(UNSET, extract_text(result));
}

#[test]
fn should_get_list_value() {
    let data = string_list(&["foo", "bar"]);
    let result = first(data.clone(), "Foo[1]");

    assert_eq!("bar", extract_text(result));
}

#[test]
fn should_track_incc_add() {
    let data = string_list(&["foo", "bar"]);
    let target = build(data.clone(), "Foo[2]");

    let (result, sub) = subscribe(&target);
    data.foo.items().add(s("baz"));
    sub.dispose();

    assert_eq!(vec![UNSET, "baz"], texts(&result));
    assert_eq!(0, data.foo.collection_changed().handler_count());
}

#[test]
fn should_track_incc_remove() {
    let data = string_list(&["foo", "bar"]);
    let target = build(data.clone(), "Foo[0]");

    let (result, sub) = subscribe(&target);
    data.foo.items().remove_at(0);
    sub.dispose();

    assert_eq!(vec!["foo", "bar"], texts(&result));
    assert_eq!(0, data.foo.collection_changed().handler_count());
}

#[test]
fn should_track_incc_replace() {
    let data = string_list(&["foo", "bar"]);
    let target = build(data.clone(), "Foo[1]");

    let (result, sub) = subscribe(&target);
    data.foo.items().set(1, s("baz"));
    sub.dispose();

    assert_eq!(vec!["bar", "baz"], texts(&result));
    assert_eq!(0, data.foo.collection_changed().handler_count());
}

#[test]
fn should_track_incc_move() {
    let data = string_list(&["foo", "bar"]);
    let target = build(data.clone(), "Foo[1]");

    let (result, _sub) = subscribe(&target);
    data.foo.items().move_item(0, 1);

    assert_eq!(vec!["bar", "foo"], texts(&result));
}

#[test]
fn should_track_incc_reset() {
    let data = string_list(&["foo", "bar"]);
    let target = build(data.clone(), "Foo[1]");

    let (result, _sub) = subscribe(&target);
    data.foo.items().clear();

    assert_eq!(vec!["bar", UNSET], texts(&result));
}

#[test]
fn should_track_non_integer_indexer() {
    let data = NonIntegerIndexerData::new(NonIntegerIndexer::new());
    data.foo.set("foo", "bar");
    data.foo.set("baz", "qux");

    let target = build(data.clone(), "Foo[foo]");

    let (result, sub) = subscribe(&target);
    data.foo.set("foo", "bar2");
    sub.dispose();

    assert_eq!(vec!["bar", "bar2"], texts(&result));
    assert_eq!(0, data.foo.property_changed_subscription_count());
}

#[test]
fn should_set_array_index() {
    let data = string_array(&["foo", "bar"]);
    let target = build(data.clone(), "Foo[1]");

    let (_, sub) = subscribe(&target);
    assert!(target.write_value_to_source(Some(boxed(s("baz")))));
    sub.dispose();

    assert_eq!("baz", data.foo.get(&[1]));
}

#[test]
fn should_set_existing_dictionary_entry() {
    let data = IntDictionaryData::new(BindableDictionary::new([(s("foo"), 1)]));
    let target = build(data.clone(), "Foo[foo]");

    let (_, sub) = subscribe(&target);
    assert!(target.write_value_to_source(Some(boxed(4))));
    sub.dispose();

    assert_eq!(4, data.foo.items().get(&s("foo")));
}

#[test]
fn should_add_new_dictionary_entry() {
    let data = IntDictionaryData::new(BindableDictionary::new([(s("foo"), 1)]));
    let target = build(data.clone(), "Foo[bar]");

    let (_, sub) = subscribe(&target);
    assert!(target.write_value_to_source(Some(boxed(4))));
    sub.dispose();

    assert_eq!(4, data.foo.items().get(&s("bar")));
}

#[test]
fn should_set_non_integer_indexer() {
    let data = NonIntegerIndexerData::new(NonIntegerIndexer::new());
    data.foo.set("foo", "bar");
    data.foo.set("baz", "qux");

    let target = build(data.clone(), "Foo[foo]");

    let (_, sub) = subscribe(&target);
    assert!(target.write_value_to_source(Some(boxed(s("bar2")))));
    sub.dispose();

    assert_eq!("bar2", data.foo.get("foo"));
}

#[test]
fn indexer_only_binding_works() {
    let data = BindableArray::new([1, 2, 3]);

    let value = first(data.clone(), "[1]").expect("a value");

    assert_eq!(Some(&data.get(&[1])), value.downcast_ref::<i32>());
}
