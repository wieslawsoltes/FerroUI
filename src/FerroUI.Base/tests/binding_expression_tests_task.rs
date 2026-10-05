//! Port of the upstream `BindingExpressionTests.Task` tests, for compiled
//! and for string paths.
//!
//! The synchronization context of the upstream tests is the dispatcher of the
//! test thread: posted callbacks run when its jobs are run.

use super::binding_test_support::{Flavor, TargetClass};
use super::*;
use crate::data::core::expression_nodes::ExpressionNode;
use crate::data::core::parsers::{BindingExpressionGrammar, ExpressionNodeFactory};
use crate::data::core::plugins::{PropertyInfoAccessorFactory, TaskValue, TaskValueSource};
use crate::data::core::{BindingExpression, BindingExpressionOptions, ClrPropertyInfo, Maybe, TargetTypeConverter, Value};
use crate::data::model::{Event, INotifyPropertyChanged, Model};
use crate::data::{BindingChainException, BindingError, BindingErrorType, CompiledBindingPathBuilder};
use crate::threading::Dispatcher;
use crate::{ferro_model, Ref};

/// `new { Foo = task }`.
struct TaskData {
    foo: TaskValue,
}

impl TaskData {
    fn new(foo: TaskValue) -> Rc<Self> {
        Model::new_model(Self { foo })
    }
}

ferro_model!(TaskData, |b| b.read_only::<Value<TaskValue>>("Foo", |o| o.foo.clone()));

/// The part of the view model of the upstream fixture these tests use.
struct ViewModel {
    string_value: RefCell<Option<String>>,
    next_task: RefCell<Option<TaskValue>>,
    property_changed: Event<str>,
}

impl ViewModel {
    fn new() -> Rc<Self> {
        Model::new_model(Self {
            string_value: RefCell::new(None),
            next_task: RefCell::new(None),
            property_changed: Event::new(),
        })
    }

    fn string_value(&self) -> Option<String> {
        self.string_value.borrow().clone()
    }

    fn set_string_value(&self, value: Option<String>) {
        self.string_value.replace(value);
        self.property_changed.raise("StringValue");
    }

    fn next_task(&self) -> Option<TaskValue> {
        self.next_task.borrow().clone()
    }

    fn set_next_task(&self, value: Option<TaskValue>) {
        self.next_task.replace(value);
        self.property_changed.raise("NextTask");
    }
}

impl INotifyPropertyChanged for ViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(ViewModel, |b| b
    .notify_property_changed()
    .property::<Maybe<String>>("StringValue", |o| o.string_value(), |o, v| o.set_string_value(v))
    .property::<Maybe<TaskValue>>("NextTask", |o| o.next_task(), |o, v| o.set_next_task(v)));

fn foo(builder: CompiledBindingPathBuilder) -> CompiledBindingPathBuilder {
    builder.property(
        Rc::new(ClrPropertyInfo::read_only::<TaskData, Value<TaskValue>>("Foo", |o| o.foo.clone())),
        PropertyInfoAccessorFactory::create_plain_property_accessor(),
    )
}

/// The nodes of a path: the compiled form, or the string form parsed.
fn nodes(f: Flavor, compiled: CompiledBindingPathBuilder, text: &str) -> Vec<Rc<dyn ExpressionNode>> {
    let mut nodes: Vec<Rc<dyn ExpressionNode>> = Vec::new();
    match f {
        Flavor::Compiled => {
            compiled.build().build_expression(&mut nodes);
        }
        Flavor::Reflection => {
            let (ast, _) = BindingExpressionGrammar::parse(text).expect("the path parses");
            ExpressionNodeFactory::create_from_ast(&ast, None, None, &mut nodes).expect("the path resolves");
        }
    }
    nodes
}

/// Creates a target whose string property is bound to `source`.
fn create_target_with_source(
    source: BoxedValue,
    nodes: Vec<Rc<dyn ExpressionNode>>,
    enable_data_validation: bool,
) -> Ref<TargetClass> {
    let target = TargetClass::new();
    let expression = BindingExpression::new(
        Some(source),
        nodes,
        BindingExpressionOptions {
            enable_data_validation,
            target_type_converter: Some(TargetTypeConverter::get_reflection_converter()),
            ..BindingExpressionOptions::default()
        },
    );
    target.values().add_binding_expression(&target, TargetClass::string_property(), expression);
    target
}

fn execute_posted_callbacks() {
    Dispatcher::current_dispatcher().run_jobs(None);
}

#[track_caller]
fn assert_binding_error(target: &TargetClass, expected: BindingChainException, expected_type: BindingErrorType) {
    let (error_type, error) =
        target.binding_notification(TargetClass::string_property()).expect("a binding notification");
    assert_eq!(expected_type, error_type);
    assert_eq!(expected.to_string(), error.to_string());
}

fn not_supported() -> BindingError {
    BindingError::message("Specified method is not supported.")
}

fn task_from_exception(e: BindingError) -> TaskValue {
    let tcs = TaskValueSource::new();
    tcs.set_exception(e);
    tcs.task()
}

fn should_not_get_task_result_without_stream_binding(f: Flavor) {
    let tcs = TaskValueSource::new();
    let data = TaskData::new(tcs.task());
    let target = create_target_with_source(data.clone(), nodes(f, foo(CompiledBindingPathBuilder::new()), "Foo"), false);

    assert_eq!(None, target.string());

    tcs.set_result(s("foo"));
    execute_posted_callbacks();

    assert_eq!(None, target.string());
}

fn should_get_completed_task_value(f: Flavor) {
    let data = TaskData::new(TaskValue::from_result(s("foo")));
    let target = create_target_with_source(
        data.clone(),
        nodes(f, foo(CompiledBindingPathBuilder::new()).stream_task(), "Foo^"),
        false,
    );

    assert_eq!(Some(s("foo")), target.string());
}

fn should_get_property_value_from_task(f: Flavor) {
    let tcs = TaskValueSource::new();
    let data = ViewModel::new();
    data.set_next_task(Some(tcs.task()));
    let compiled = CompiledBindingPathBuilder::new()
        .notifying_property::<ViewModel, Maybe<TaskValue>>("NextTask", |o| o.next_task(), |o, v| o.set_next_task(v))
        .stream_task()
        .notifying_property::<ViewModel, Maybe<String>>(
            "StringValue",
            |o| o.string_value(),
            |o, v| o.set_string_value(v),
        );
    let target = create_target_with_source(data.clone(), nodes(f, compiled, "NextTask^.StringValue"), false);

    let result = ViewModel::new();
    result.set_string_value(Some(s("foo")));
    tcs.set_untyped_result(Some(result));
    execute_posted_callbacks();

    assert_eq!(Some(s("foo")), target.string());
}

fn should_update_data_validation_on_task_exception(f: Flavor) {
    let tcs = TaskValueSource::new();
    let data = TaskData::new(tcs.task());
    let target = create_target_with_source(
        data.clone(),
        nodes(f, foo(CompiledBindingPathBuilder::new()).stream_task(), "Foo^"),
        true,
    );

    tcs.set_exception(not_supported());
    execute_posted_callbacks();

    assert_binding_error(
        &target,
        BindingChainException::with_expression("Specified method is not supported.", "Foo^", "^"),
        BindingErrorType::Error,
    );
}

fn should_update_data_validation_on_faulted_task(f: Flavor) {
    let data = TaskData::new(task_from_exception(not_supported()));
    let target = create_target_with_source(
        data.clone(),
        nodes(f, foo(CompiledBindingPathBuilder::new()).stream_task(), "Foo^"),
        true,
    );

    assert_binding_error(
        &target,
        BindingChainException::with_expression("Specified method is not supported.", "Foo^", "^"),
        BindingErrorType::Error,
    );
}

fn should_get_simple_task_value_with_data_data_validation_enabled(f: Flavor) {
    let tcs = TaskValueSource::new();
    let data = TaskData::new(tcs.task());
    let target = create_target_with_source(
        data.clone(),
        nodes(f, foo(CompiledBindingPathBuilder::new()).stream_task(), "Foo^"),
        true,
    );

    tcs.set_result(s("foo"));
    execute_posted_callbacks();

    // What does it mean to have data validation on a task? Without a use-case
    // it's hard to know what to do here so for the moment the value is
    // returned.
    assert_eq!(Some(s("foo")), target.string());
}

// Not upstream: the outcome of a running task is delivered from a dispatcher
// job, not from within the completion.
#[test]
fn running_task_result_is_delivered_from_a_dispatcher_job() {
    let tcs = TaskValueSource::new();
    let data = TaskData::new(tcs.task());
    let target = create_target_with_source(
        data.clone(),
        nodes(Flavor::Reflection, CompiledBindingPathBuilder::new(), "Foo^"),
        false,
    );

    tcs.set_result(s("foo"));
    assert_eq!(None, target.string());

    execute_posted_callbacks();
    assert_eq!(Some(s("foo")), target.string());
}

// Not upstream: a task driven by a future on the dispatcher.
#[test]
fn task_run_streams_the_outcome_of_a_future() {
    let data = TaskData::new(TaskValue::run(async { Ok(Some(boxed(s("foo")))) }));
    assert!(!data.foo.is_completed());
    let target = create_target_with_source(
        data.clone(),
        nodes(Flavor::Reflection, CompiledBindingPathBuilder::new(), "Foo^"),
        false,
    );
    assert_eq!(None, target.string());

    execute_posted_callbacks();

    assert!(data.foo.is_completed_successfully());
    assert_eq!(Some(s("foo")), target.string());
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
    should_not_get_task_result_without_stream_binding,
    should_get_completed_task_value,
    should_get_property_value_from_task,
    should_update_data_validation_on_task_exception,
    should_update_data_validation_on_faulted_task,
    should_get_simple_task_value_with_data_data_validation_enabled,
);
