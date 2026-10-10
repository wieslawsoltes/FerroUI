//! Port of the binding expression tests of the reference leak tests: a
//! binding expression that is subscribed to does not keep what its path
//! reads alive.
//!
//! The reference builds the path of an expression from a lambda over an
//! anonymous object; here the anonymous object is a model type with the
//! one property, and the path is its text, resolved at run time through the
//! metadata of the model (the path a lambda is turned into is resolved by
//! reflection in the reference too).

use crate::leak::Tracked;
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::expression_nodes::ExpressionNode;
use ferroui_base::data::core::parsers::{BindingExpressionGrammar, ExpressionNodeFactory};
use ferroui_base::data::core::{
    BindingExpression, BindingExpressionOptions, ModelRef, TargetTypeConverter, Value, INDEXER_NAME,
};
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::reactive::{AnonymousObserver, IObservable};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{ferro_model, BoxedValue};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

// --- the sources of the tests -----------------------------------------------

/// `new { Foo = list }`.
struct ListSource {
    foo: FerroList<String>,
}

ferro_model!(ListSource, |b| b.read_only::<Value<FerroList<String>>>("Foo", |o| o.foo.clone()));

/// `new { Foo = indexer }`.
struct IndexerSource {
    foo: Rc<NonIntegerIndexer>,
}

ferro_model!(IndexerSource, |b| b.read_only::<ModelRef<NonIntegerIndexer>>("Foo", |o| Some(o.foo.clone())));

/// `new { Foo = methodBound }`.
struct MethodSource {
    foo: Rc<MethodBound>,
}

ferro_model!(MethodSource, |b| b.read_only::<ModelRef<MethodBound>>("Foo", |o| Some(o.foo.clone())));

struct MethodBound;

impl MethodBound {
    fn a(&self) {}
}

ferro_model!(MethodBound, |b| b.method("A", |o, _| o.a()));

/// A notifying object with an indexer whose argument is not an integer.
struct NonIntegerIndexer {
    storage: RefCell<HashMap<String, String>>,
    property_changed: Event<str>,
}

impl INotifyPropertyChanged for NonIntegerIndexer {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(NonIntegerIndexer, |b| b.notify_property_changed());

// The scenario binds to the object and never reads through its indexer.
#[allow(dead_code)]
impl NonIntegerIndexer {
    fn new() -> Rc<Self> {
        Model::new_model(Self { storage: RefCell::new(HashMap::new()), property_changed: Event::new() })
    }

    fn get(&self, key: &str) -> String {
        self.storage.borrow()[key].clone()
    }

    fn set(&self, key: &str, value: &str) {
        self.storage.borrow_mut().insert(key.to_string(), value.to_string());
        self.property_changed.raise(INDEXER_NAME);
    }
}

// --- creating expressions ---------------------------------------------------

/// The helper of the reference: a binding expression over `source` with the
/// nodes of `path`, the defaults of the reference for everything else (one
/// way, local value priority, no fallback, the reflection converter of the
/// target type).
fn create_binding_expression(source: Option<BoxedValue>, path: &str, enable_data_validation: bool) -> Rc<BindingExpression> {
    let (ast, _) = BindingExpressionGrammar::parse(path).expect("the path parses");
    let mut nodes: Vec<Rc<dyn ExpressionNode>> = Vec::new();
    ExpressionNodeFactory::create_from_ast(&ast, None, None, &mut nodes).expect("the path resolves");

    BindingExpression::new(
        source,
        nodes,
        BindingExpressionOptions {
            enable_data_validation,
            target_type_converter: Some(TargetTypeConverter::get_reflection_converter()),
            ..BindingExpressionOptions::default()
        },
    )
}

/// `target.ToObservable().Subscribe(_ => { })`.
fn subscribe(target: &Rc<BindingExpression>) {
    let _ = target.to_observable(None).subscribe(Rc::new(AnonymousObserver::new(|_: Option<BoxedValue>| {})));
}

// --- tests ------------------------------------------------------------------

#[test]
fn should_not_keep_source_alive_observable_collection() {
    let _scope = Dispatcher::unit_test_scope();

    let source = {
        let list = FerroList::from_items(["foo".to_string(), "bar".to_string()]);
        let source = Model::new_model(ListSource { foo: list.clone() });
        let target = create_binding_expression(Some(source.clone() as BoxedValue), "Foo", false);

        subscribe(&target);

        let tracked = Tracked::list("the list the path reads", &list);
        tracked.assert_alive();
        tracked
    };

    source.assert_freed();
}

#[test]
fn should_not_keep_source_alive_observable_collection_with_data_validation() {
    let _scope = Dispatcher::unit_test_scope();

    let source = {
        let list = FerroList::from_items(["foo".to_string(), "bar".to_string()]);
        let source = Model::new_model(ListSource { foo: list.clone() });
        let target = create_binding_expression(Some(source.clone() as BoxedValue), "Foo", true);

        subscribe(&target);

        let tracked = Tracked::list("the list the path reads", &list);
        tracked.assert_alive();
        tracked
    };

    source.assert_freed();
}

#[test]
fn should_not_keep_source_alive_non_integer_indexer() {
    let _scope = Dispatcher::unit_test_scope();

    let source = {
        let indexer = NonIntegerIndexer::new();
        let source = Model::new_model(IndexerSource { foo: indexer.clone() });
        let target = create_binding_expression(Some(source.clone() as BoxedValue), "Foo", false);

        subscribe(&target);

        let tracked = Tracked::shared("the indexer the path reads", &indexer);
        tracked.assert_alive();
        tracked
    };

    source.assert_freed();
}

#[test]
fn should_not_keep_source_alive_method_binding() {
    let _scope = Dispatcher::unit_test_scope();

    let source = {
        let method_bound = Model::new_model(MethodBound);
        let source = Model::new_model(MethodSource { foo: method_bound.clone() });
        let target = create_binding_expression(Some(source.clone() as BoxedValue), "Foo.A", false);

        subscribe(&target);

        let tracked = Tracked::shared("the object whose method the path reads", &method_bound);
        tracked.assert_alive();
        tracked
    };

    source.assert_freed();
}
