//! Port of the upstream `BindingExpressionTests.Observable` tests.

use super::binding_test_support::*;
use super::*;
use crate::data::core::plugins::ObservableValue;
use crate::data::core::Value;
use crate::data::model::Model;
use crate::ferro_model;

/// `new { Foo = source }`.
struct FooData {
    foo: ObservableValue,
}

impl FooData {
    fn new(foo: &ObservableValue) -> Rc<Self> {
        Model::new_model(Self { foo: foo.clone() })
    }

    fn foo_step() -> Step {
        plain_read_only_prop::<FooData, Value<ObservableValue>>("Foo", Out::Object, |o| o.foo.clone())
    }
}

ferro_model!(FooData, |b| b.read_only::<Value<ObservableValue>>("Foo", |o| o.foo.clone()));

binding_tests! {
    fn should_not_get_observable_value_without_streaming(f) {
        let source = Subject::behavior(s("foo"));
        let observable = ObservableValue::new(source.observable());
        let data = FooData::new(&observable);
        let target = create_target_with_source(f, src(&data), &Path::of(FooData::foo_step()), Opts::default());

        let value = target.object().expect("a value");
        assert!(value.downcast_ref::<ObservableValue>() == Some(&observable));
    }

    fn should_get_simple_observable_value(f) {
        let source = Subject::behavior(s("foo"));
        let data = FooData::new(&ObservableValue::new(source.observable()));
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(FooData::foo_step()).then(Step::Stream(Out::String)),
            Opts::default(),
        );

        assert_eq!(target.string(), Some(s("foo")));

        source.on_next(s("bar"));

        assert_eq!(target.string(), Some(s("bar")));
    }

    fn should_get_property_value_from_observable(f) {
        let source: Subject<Option<BoxedValue>> = Subject::behavior(src(&ViewModel::with_string("foo")));
        let data = ViewModel::new();
        data.set_next_observable(Some(ObservableValue(source.observable())));
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::next_observable_step()).then(Step::Stream(Out::Object)).then(ViewModel::string_step()),
            Opts::default(),
        );

        assert_eq!(target.string(), Some(s("foo")));
    }

    fn should_get_simple_observable_value_with_data_validation_enabled(f) {
        let source = Subject::behavior(s("foo"));
        let data = FooData::new(&ObservableValue::new(source.observable()));
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(FooData::foo_step()).then(Step::Stream(Out::String)),
            Opts::default().with_data_validation(true),
        );

        assert_eq!(target.string(), Some(s("foo")));

        source.on_next(s("bar"));

        assert_eq!(target.string(), Some(s("bar")));
    }

    fn should_work_with_value_type(f) {
        let source = Subject::behavior(1);
        let data = FooData::new(&ObservableValue::new(source.observable()));
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(FooData::foo_step()).then(Step::Stream(Out::Int)),
            Opts::default(),
        );

        assert_eq!(target.int(), 1);

        source.on_next(42);

        assert_eq!(target.int(), 42);
    }
}
