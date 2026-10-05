//! Port of the upstream `BindingExpressionTests.Indexer` tests.
//!
//! The upstream fixture runs every test for compiled and for string paths,
//! and so does this port. A compiled path reaches an array through an array
//! indexer node, and a list, dictionary or custom indexer through a property
//! description named after the indexer, whose getter fails for a bad index or
//! key: with the indexer accessor when the argument is one integer, with the
//! plain notifying accessor otherwise.

use super::binding_test_support::{Flavor, TargetClass};
use super::*;
use crate::data::core::expression_nodes::ExpressionNode;
use crate::data::core::parsers::{BindingExpressionGrammar, ExpressionNodeFactory};
use crate::data::core::plugins::PropertyInfoAccessorFactory;
use crate::data::core::{
    BindingExpression, BindingExpressionOptions, ClrPropertyInfo, ModelRef, TargetTypeConverter, Value, ValueType,
    INDEXER_NAME,
};
use crate::data::model::{
    BindableArray, BindableDictionary, BindableList, Event, IBindableIndexer, INotifyPropertyChanged, Model,
};
use crate::data::{BindingError, BindingMode, CompiledBindingPathBuilder};
use crate::{ferro_model, FerroProperty, Ref};
use std::collections::HashMap;

/// The compiled path element of the `Foo` property of a data type.
pub(super) trait FooData {
    fn foo(builder: CompiledBindingPathBuilder) -> CompiledBindingPathBuilder;
}

/// Declares the equivalent of `new { Foo = <inner> }`.
macro_rules! foo_data {
    ($name:ident, $inner:ty) => {
        pub(super) struct $name {
            pub foo: Rc<$inner>,
        }

        impl $name {
            pub fn new(foo: Rc<$inner>) -> Rc<Self> {
                Model::new_model(Self { foo })
            }
        }

        ferro_model!($name, |b| b.read_only::<ModelRef<$inner>>("Foo", |o| Some(o.foo.clone())));

        impl FooData for $name {
            fn foo(builder: CompiledBindingPathBuilder) -> CompiledBindingPathBuilder {
                builder.property(
                    Rc::new(ClrPropertyInfo::read_only::<$name, ModelRef<$inner>>("Foo", |o| Some(o.foo.clone()))),
                    PropertyInfoAccessorFactory::create_plain_property_accessor(),
                )
            }
        }
    };
}

foo_data!(StringArrayData, BindableArray<String>);
foo_data!(StringListData, BindableList<String>);
foo_data!(StringDictionaryData, BindableDictionary<String, String>);
foo_data!(IntDictionaryData, BindableDictionary<String, i32>);
foo_data!(IntKeyDictionaryData, BindableDictionary<i32, String>);
foo_data!(DoubleIndexerData, DoubleKeyDictionary);
foo_data!(NonIntegerIndexerData, NonIntegerIndexer);

pub(super) fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|i| s(i)).collect()
}

fn key_not_present(key: &dyn std::fmt::Display) -> BindingError {
    BindingError::message(format!("The given key '{key}' was not present in the dictionary."))
}

/// The equivalent of `Dictionary<double, string>`: an indexer whose argument
/// is not text.
pub(super) struct DoubleKeyDictionary {
    entries: RefCell<Vec<(f64, String)>>,
}

impl DoubleKeyDictionary {
    pub fn new(entries: &[(f64, &str)]) -> Rc<Self> {
        Model::new_model(Self { entries: RefCell::new(entries.iter().map(|(k, v)| (*k, s(v))).collect()) })
    }
}

impl DoubleKeyDictionary {
    fn try_get(&self, key: f64) -> Result<String, BindingError> {
        match self.entries.borrow().iter().find(|(k, _)| *k == key) {
            Some((_, value)) => Ok(value.clone()),
            None => Err(key_not_present(&key)),
        }
    }
}

impl IBindableIndexer for DoubleKeyDictionary {
    fn index_parameter_types(&self) -> Vec<ValueType> {
        vec![ValueType::of::<f64>()]
    }

    fn value_type(&self) -> ValueType {
        ValueType::of::<String>()
    }

    fn get(&self, arguments: &[BoxedValue]) -> Result<Option<BoxedValue>, BindingError> {
        let key = *arguments[0].downcast_ref::<f64>().expect("a converted argument");
        match self.entries.borrow().iter().find(|(k, _)| *k == key) {
            Some((_, value)) => Ok(Some(boxed(value.clone()))),
            None => Err(key_not_present(&key)),
        }
    }

    fn set(&self, arguments: &[BoxedValue], value: Option<&BoxedValue>) -> Result<bool, BindingError> {
        let key = *arguments[0].downcast_ref::<f64>().expect("a converted argument");
        let value = value.and_then(|v| v.downcast_ref::<String>()).cloned().unwrap_or_default();
        let mut entries = self.entries.borrow_mut();
        match entries.iter_mut().find(|(k, _)| *k == key) {
            Some(entry) => entry.1 = value,
            None => entries.push((key, value)),
        }
        Ok(true)
    }
}

ferro_model!(DoubleKeyDictionary, |b| b.indexer());

/// A notifying object with a string indexer.
pub(super) struct NonIntegerIndexer {
    storage: RefCell<HashMap<String, String>>,
    property_changed: Event<str>,
}

impl NonIntegerIndexer {
    pub fn new() -> Rc<Self> {
        Model::new_model(Self { storage: RefCell::new(HashMap::new()), property_changed: Event::new() })
    }

    pub fn get(&self, key: &str) -> String {
        self.storage.borrow()[key].clone()
    }

    pub fn set(&self, key: &str, value: &str) {
        self.storage.borrow_mut().insert(s(key), s(value));
        self.property_changed.raise(INDEXER_NAME);
    }

    fn try_get(&self, key: &str) -> Result<String, BindingError> {
        self.storage.borrow().get(key).cloned().ok_or_else(|| key_not_present(&key))
    }

    pub fn property_changed_subscription_count(&self) -> usize {
        self.property_changed.handler_count()
    }
}

impl INotifyPropertyChanged for NonIntegerIndexer {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl IBindableIndexer for NonIntegerIndexer {
    fn index_parameter_types(&self) -> Vec<ValueType> {
        vec![ValueType::of::<String>()]
    }

    fn value_type(&self) -> ValueType {
        ValueType::of::<String>()
    }

    fn get(&self, arguments: &[BoxedValue]) -> Result<Option<BoxedValue>, BindingError> {
        let key = arguments[0].downcast_ref::<String>().expect("a converted argument");
        match self.storage.borrow().get(key) {
            Some(value) => Ok(Some(boxed(value.clone()))),
            None => Err(key_not_present(key)),
        }
    }

    fn set(&self, arguments: &[BoxedValue], value: Option<&BoxedValue>) -> Result<bool, BindingError> {
        let key = arguments[0].downcast_ref::<String>().expect("a converted argument");
        let value = value.and_then(|v| v.downcast_ref::<String>()).cloned().unwrap_or_default();
        self.set(key, &value);
        Ok(true)
    }
}

ferro_model!(NonIntegerIndexer, |b| b.notify_property_changed().indexer());

/// The nodes of a string path.
pub(super) fn parse(path: &str) -> Vec<Rc<dyn ExpressionNode>> {
    let mut nodes: Vec<Rc<dyn ExpressionNode>> = Vec::new();
    let (ast, _) = BindingExpressionGrammar::parse(path).expect("the path parses");
    ExpressionNodeFactory::create_from_ast(&ast, None, None, &mut nodes).expect("the path resolves");
    nodes
}

/// The nodes of a compiled path.
fn compiled(
    build: impl FnOnce(CompiledBindingPathBuilder) -> CompiledBindingPathBuilder,
) -> Vec<Rc<dyn ExpressionNode>> {
    let mut nodes: Vec<Rc<dyn ExpressionNode>> = Vec::new();
    build(CompiledBindingPathBuilder::new()).build().build_expression(&mut nodes);
    nodes
}

/// The nodes of `Foo[index]` over a list of strings.
fn list_nodes(flavor: Flavor, index: i32) -> Vec<Rc<dyn ExpressionNode>> {
    match flavor {
        Flavor::Compiled => compiled(|b| StringListData::foo(b).list_item::<String>(index)),
        Flavor::Reflection => parse(&format!("Foo[{index}]")),
    }
}

/// The nodes of `Foo[key]` over a dictionary with string keys.
fn dictionary_nodes<D: FooData, V: crate::PropertyValue>(flavor: Flavor, key: &str) -> Vec<Rc<dyn ExpressionNode>> {
    match flavor {
        Flavor::Compiled => compiled(|b| D::foo(b).dictionary_item::<String, V>(s(key))),
        Flavor::Reflection => parse(&format!("Foo[{key}]")),
    }
}

/// The nodes of `Foo["foo"]` over the notifying object with a string
/// indexer.
fn non_integer_indexer_nodes(flavor: Flavor) -> Vec<Rc<dyn ExpressionNode>> {
    match flavor {
        Flavor::Compiled => compiled(|b| {
            NonIntegerIndexerData::foo(b).property(
                Rc::new(ClrPropertyInfo::read_write_fallible::<NonIntegerIndexer, Value<String>>(
                    INDEXER_NAME,
                    |o| o.try_get("foo"),
                    |o, v| {
                        o.set("foo", &v);
                        Ok(())
                    },
                )),
                PropertyInfoAccessorFactory::create_inpc_property_accessor::<NonIntegerIndexer>(),
            )
        }),
        Flavor::Reflection => parse("Foo[foo]"),
    }
}

/// The nodes of `Foo[indexes]` over an array: by compiled path or by string
/// path.
fn array_nodes(flavor: Flavor, indexes: &[i32]) -> Vec<Rc<dyn ExpressionNode>> {
    match flavor {
        Flavor::Compiled => compiled(|b| StringArrayData::foo(b).array_element(indexes)),
        Flavor::Reflection => {
            parse(&format!("Foo[{}]", indexes.iter().map(i32::to_string).collect::<Vec<_>>().join(",")))
        }
    }
}

/// Creates a target bound to `source` through `nodes`, the way the upstream
/// fixture does.
fn create_target_with_source(
    source: BoxedValue,
    nodes: Vec<Rc<dyn ExpressionNode>>,
    target_property: &'static FerroProperty,
    mode: BindingMode,
) -> Ref<TargetClass> {
    let target = TargetClass::new();
    let expression = BindingExpression::new(
        Some(source),
        nodes,
        BindingExpressionOptions {
            mode,
            target_type_converter: Some(TargetTypeConverter::get_reflection_converter()),
            ..BindingExpressionOptions::default()
        },
    );
    target.values().add_binding_expression(&target, target_property, expression);
    target
}

fn string_target(source: BoxedValue, nodes: Vec<Rc<dyn ExpressionNode>>) -> Ref<TargetClass> {
    create_target_with_source(source, nodes, TargetClass::string_property(), BindingMode::OneWay)
}

fn should_get_array_value(f: Flavor) {
    let data = StringArrayData::new(BindableArray::new(strings(&["foo", "bar"])));
    let target = string_target(data.clone(), array_nodes(f, &[1]));

    assert_eq!(Some(s("bar")), target.string());
}

fn should_get_multi_dimensional_array_value(f: Flavor) {
    let data = StringArrayData::new(BindableArray::with_lengths(&[2, 2], strings(&["foo", "bar", "baz", "qux"])));
    let target = string_target(data.clone(), array_nodes(f, &[1, 1]));

    assert_eq!(Some(s("qux")), target.string());
}

fn should_get_value_for_string_indexer(f: Flavor) {
    let data = StringDictionaryData::new(BindableDictionary::new([(s("foo"), s("bar")), (s("baz"), s("qux"))]));
    let target = string_target(data.clone(), dictionary_nodes::<StringDictionaryData, String>(f, "foo"));

    assert_eq!(Some(s("bar")), target.string());
}

fn should_get_value_for_non_string_indexer(f: Flavor) {
    let data = DoubleIndexerData::new(DoubleKeyDictionary::new(&[(1.0, "bar"), (2.0, "qux")]));
    let nodes = match f {
        Flavor::Compiled => compiled(|b| {
            DoubleIndexerData::foo(b).property(
                Rc::new(ClrPropertyInfo::read_only_fallible::<DoubleKeyDictionary, Value<String>>(
                    INDEXER_NAME,
                    |o| o.try_get(1.0),
                )),
                PropertyInfoAccessorFactory::create_plain_property_accessor(),
            )
        }),
        Flavor::Reflection => parse("Foo[1.0]"),
    };
    let target = string_target(data.clone(), nodes);

    assert_eq!(Some(s("bar")), target.string());
}

fn array_out_of_bounds_should_return_unset_value(f: Flavor) {
    let data = StringArrayData::new(BindableArray::new(strings(&["foo", "bar"])));
    let target = string_target(data.clone(), array_nodes(f, &[2]));

    assert!(!target.is_set(TargetClass::string_property()));
}

fn list_out_of_bounds_should_return_unset_value(f: Flavor) {
    let data = StringListData::new(BindableList::new(strings(&["foo", "bar"])));
    let target = string_target(data.clone(), list_nodes(f, 2));

    assert!(!target.is_set(TargetClass::string_property()));
}

fn should_get_list_value(f: Flavor) {
    let data = StringListData::new(BindableList::new(strings(&["foo", "bar"])));
    let target = string_target(data.clone(), list_nodes(f, 1));

    assert_eq!(Some(s("bar")), target.string());
}

fn should_track_incc_add(f: Flavor) {
    let data = StringListData::new(BindableList::new(strings(&["foo", "bar"])));
    let target = string_target(data.clone(), list_nodes(f, 2));

    assert!(!target.is_set(TargetClass::string_property()));

    data.foo.items().add(s("baz"));

    assert_eq!(Some(s("baz")), target.string());
}

fn should_track_incc_remove(f: Flavor) {
    let data = StringListData::new(BindableList::new(strings(&["foo", "bar"])));
    let target = string_target(data.clone(), list_nodes(f, 0));

    assert_eq!(Some(s("foo")), target.string());

    data.foo.items().remove_at(0);

    assert_eq!(Some(s("bar")), target.string());
}

fn should_track_incc_replace(f: Flavor) {
    let data = StringListData::new(BindableList::new(strings(&["foo", "bar"])));
    let target = string_target(data.clone(), list_nodes(f, 1));

    assert_eq!(Some(s("bar")), target.string());

    data.foo.items().set(1, s("baz"));

    assert_eq!(Some(s("baz")), target.string());
}

fn should_track_incc_move(f: Flavor) {
    let data = StringListData::new(BindableList::new(strings(&["foo", "bar"])));
    let target = string_target(data.clone(), list_nodes(f, 1));

    assert_eq!(Some(s("bar")), target.string());

    data.foo.items().move_item(0, 1);

    assert_eq!(Some(s("foo")), target.string());
}

fn should_track_incc_reset(f: Flavor) {
    let data = StringListData::new(BindableList::new(strings(&["foo", "bar"])));
    let target = string_target(data.clone(), list_nodes(f, 1));

    assert_eq!(Some(s("bar")), target.string());

    data.foo.items().clear();

    assert_eq!(None, target.string());
}

fn should_track_non_integer_indexer(f: Flavor) {
    let data = NonIntegerIndexerData::new(NonIntegerIndexer::new());
    data.foo.set("foo", "bar");
    data.foo.set("baz", "qux");

    let target = string_target(data.clone(), non_integer_indexer_nodes(f));

    assert_eq!(Some(s("bar")), target.string());

    data.foo.set("foo", "bar2");

    assert_eq!(Some(s("bar2")), target.string());
}

fn should_set_array_index(f: Flavor) {
    let data = StringArrayData::new(BindableArray::new(strings(&["foo", "bar"])));
    let target =
        create_target_with_source(data.clone(), array_nodes(f, &[1]), TargetClass::string_property(), BindingMode::TwoWay);

    target.set_string(Some("baz"));

    assert_eq!("baz", data.foo.get(&[1]));
}

fn should_set_existing_dictionary_entry(f: Flavor) {
    let data = IntDictionaryData::new(BindableDictionary::new([(s("foo"), 1)]));
    let target =
        create_target_with_source(
        data.clone(),
        dictionary_nodes::<IntDictionaryData, i32>(f, "foo"),
        TargetClass::int_property(),
        BindingMode::TwoWay,
    );

    target.set_int(4);

    assert_eq!(4, data.foo.items().get(&s("foo")));
}

fn should_add_new_dictionary_entry(f: Flavor) {
    let data = IntDictionaryData::new(BindableDictionary::new([(s("foo"), 1)]));
    let target =
        create_target_with_source(
        data.clone(),
        dictionary_nodes::<IntDictionaryData, i32>(f, "bar"),
        TargetClass::int_property(),
        BindingMode::TwoWay,
    );

    target.set_int(4);

    assert_eq!(4, data.foo.items().get(&s("bar")));
}

fn should_set_non_integer_indexer(f: Flavor) {
    let data = NonIntegerIndexerData::new(NonIntegerIndexer::new());
    data.foo.set("foo", "bar");
    data.foo.set("baz", "qux");

    let target =
        create_target_with_source(data.clone(), non_integer_indexer_nodes(f), TargetClass::string_property(), BindingMode::TwoWay);

    target.set_string(Some("bar2"));

    assert_eq!("bar2", data.foo.get("foo"));
}

fn indexer_only_binding_works(f: Flavor) {
    let data = BindableArray::new([1, 2, 3]);
    let nodes = match f {
        Flavor::Compiled => compiled(|b| b.array_element(&[1])),
        Flavor::Reflection => parse("[1]"),
    };
    let target = create_target_with_source(data.clone(), nodes, TargetClass::int_property(), BindingMode::OneWay);

    assert_eq!(data.get(&[1]), target.int());
}

macro_rules! both_flavors {
    ($($name:ident),* $(,)?) => {
        mod compiled {
            $(#[test] fn $name() { super::$name(super::Flavor::Compiled) })*
        }

        mod reflection {
            $(#[test] fn $name() { super::$name(super::Flavor::Reflection) })*
        }
    };
}

both_flavors!(
    should_get_array_value,
    should_get_multi_dimensional_array_value,
    should_get_value_for_string_indexer,
    should_get_value_for_non_string_indexer,
    array_out_of_bounds_should_return_unset_value,
    list_out_of_bounds_should_return_unset_value,
    should_get_list_value,
    should_track_incc_add,
    should_track_incc_remove,
    should_track_incc_replace,
    should_track_incc_move,
    should_track_incc_reset,
    should_track_non_integer_indexer,
    should_set_array_index,
    should_set_existing_dictionary_entry,
    should_add_new_dictionary_entry,
    should_set_non_integer_indexer,
    indexer_only_binding_works,
);
