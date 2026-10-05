//! Port of the upstream `BindingExpressionTests.Property` tests.
//!
//! Not ported: `Should_Get_Simple_Property_From_Base_Class` and
//! `Should_Not_Throw_Exception_On_Duplicate_Properties` (model types have no
//! inheritance, so there are no inherited or hidden members to look up).

use super::binding_test_support::*;
use super::*;
use crate::data::core::expression_nodes::CastTarget;
use crate::data::core::{Maybe, ModelRef, Value, ValueType};
use crate::data::model::Model;
use crate::data::BindingMode;
use crate::ferro_model;

/// `new { Foo = "foo" }`.
struct FooData {
    foo: Option<String>,
}

impl FooData {
    fn new(foo: Option<&str>) -> Rc<Self> {
        Model::new_model(Self { foo: foo.map(s) })
    }

    fn foo_step() -> Step {
        plain_read_only_prop::<FooData, Maybe<String>>("Foo", Out::String, |o| o.foo.clone())
    }
}

ferro_model!(FooData, |b| b.read_only::<Maybe<String>>("Foo", |o| o.foo.clone()));

/// `new { Foo = new { Bar = new { Baz = "baz" } } }`.
struct ChainFoo {
    foo: Rc<ChainBar>,
}

struct ChainBar {
    bar: Rc<ChainBaz>,
}

struct ChainBaz {
    baz: String,
}

ferro_model!(ChainFoo, |b| b.read_only::<ModelRef<ChainBar>>("Foo", |o| Some(o.foo.clone())));
ferro_model!(ChainBar, |b| b.read_only::<ModelRef<ChainBaz>>("Bar", |o| Some(o.bar.clone())));
ferro_model!(ChainBaz, |b| b.read_only::<Value<String>>("Baz", |o| o.baz.clone()));

/// `new { MissingMember = "Yes" }`.
struct MissingMemberData {
    missing_member: String,
}

ferro_model!(MissingMemberData, |b| b.read_only::<Value<String>>("MissingMember", |o| o.missing_member.clone()));

binding_tests! {
    fn should_get_simple_property_value(f) {
        let data = FooData::new(Some("foo"));
        let target = create_target_with_source(f, src(&data), &Path::of(FooData::foo_step()), Opts::default());

        assert_eq!(target.string(), Some(s("foo")));
    }

    fn should_get_simple_property_value_null(f) {
        let data = FooData::new(None);
        let target = create_target_with_source(f, src(&data), &Path::of(FooData::foo_step()), Opts::default());

        assert_eq!(target.string(), None);
    }

    fn should_get_simple_property_chain(f) {
        let data = Model::new_model(ChainFoo {
            foo: Model::new_model(ChainBar { bar: Model::new_model(ChainBaz { baz: s("baz") }) }),
        });
        let path = Path::of(plain_read_only_prop::<ChainFoo, ModelRef<ChainBar>>("Foo", Out::Object, |o| {
            Some(o.foo.clone())
        }))
        .then(plain_read_only_prop::<ChainBar, ModelRef<ChainBaz>>("Bar", Out::Object, |o| Some(o.bar.clone())))
        .then(plain_read_only_prop::<ChainBaz, Value<String>>("Baz", Out::String, |o| o.baz.clone()));
        let target = create_target_with_source(f, src(&data), &path, Opts::default());

        assert_eq!(target.string(), Some(s("baz")));
    }

    fn should_track_simple_property_value(f) {
        let data = ViewModel::with_string("foo");
        let target = create_target_with_source(f, src(&data), &Path::of(ViewModel::string_step()), Opts::default());

        assert_eq!(target.string(), Some(s("foo")));

        data.set_string_value(Some(s("bar")));

        assert_eq!(target.string(), Some(s("bar")));
    }

    fn property_changed_event_args_with_null_property_name_should_trigger_update(f) {
        let data = ViewModel::with_string("foo");
        let target = create_target_with_source(f, src(&data), &Path::of(ViewModel::string_step()), Opts::default());

        assert_eq!(target.string(), Some(s("foo")));

        data.set_string_value_without_raising("bar");

        assert_eq!(target.string(), Some(s("foo")));

        data.raise_property_changed("");

        assert_eq!(target.string(), Some(s("bar")));
    }

    fn should_track_end_of_property_chain_changing(f) {
        let data = ViewModel::with_next(ViewModel::with_string("bar"));
        let path = Path::of(ViewModel::next_step()).then(ViewModel::string_step());
        let target = create_target_with_source(f, src(&data), &path, Opts::default());

        assert_eq!(target.string(), Some(s("bar")));

        data.next().unwrap().set_string_value(Some(s("baz")));

        assert_eq!(target.string(), Some(s("baz")));

        data.next().unwrap().set_string_value(None);

        assert_eq!(target.string(), None);
    }

    fn should_track_property_chain_changing(f) {
        let data = ViewModel::with_next(ViewModel::with_string("bar"));
        let path = Path::of(ViewModel::next_step()).then(ViewModel::string_step());
        let target = create_target_with_source(f, src(&data), &path, Opts::default());
        let _old = data.next();

        assert_eq!(target.string(), Some(s("bar")));

        data.set_next(Some(ViewModel::with_string("baz")));

        assert_eq!(target.string(), Some(s("baz")));

        data.set_next(Some(ViewModel::new()));

        assert_eq!(target.string(), None);
    }

    fn should_track_property_chain_breaking_with_null_then_mending(f) {
        let data = ViewModel::with_next(ViewModel::with_next(ViewModel::with_string("bar")));
        let path = Path::of(ViewModel::next_step()).then(ViewModel::next_step()).then(ViewModel::string_step());
        let target = create_target_with_source(f, src(&data), &path, Opts::default());

        assert_eq!(target.string(), Some(s("bar")));

        let old = data.next();
        data.set_next(None);

        assert_eq!(target.string(), None);

        data.set_next(old);

        assert_eq!(target.string(), Some(s("bar")));
    }

    fn should_track_property_chain_breaking_with_missing_member_then_mending(f) {
        let data = ViewModel::new();
        data.set_object_value(src(&ViewModel::with_string("bar")));
        let path = Path::of(ViewModel::object_step())
            .then(Step::Cast(CastTarget::Value(ValueType::of::<ViewModel>())))
            .then(ViewModel::string_step());
        let target = create_target_with_source(f, src(&data), &path, Opts::default());

        assert_eq!(target.string(), Some(s("bar")));

        let old = data.object_value();
        data.set_object_value(src(&Model::new_model(MissingMemberData { missing_member: s("Yes") })));

        assert_eq!(target.string(), None);

        data.set_object_value(old);

        assert_eq!(target.string(), Some(s("bar")));
    }

    fn should_not_keep_source_alive(f) {
        let run = || {
            let source = ViewModel::with_string("foo");
            let target =
                create_target_with_source(f, src(&source), &Path::of(ViewModel::string_step()), Opts::default());
            (target, Rc::downgrade(&source))
        };

        let result = run();

        assert!(result.1.upgrade().is_none());
    }

    fn can_convert_int_to_enum_two_way(f) {
        let data = ViewModel::with_int(1);
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::int_step()),
            Opts::mode(BindingMode::TwoWay).with_property(DockPanel::dock_property()),
        );

        assert_eq!(target.get_value(DockPanel::dock_property()), Dock::Bottom);

        target.set_value(DockPanel::dock_property(), Dock::Right);

        assert_eq!(data.int_value(), 2);
    }

    fn converter_should_be_called_on_property_changed_even_if_property_not_changed(f) {
        // Issue #16137
        let data = ViewModel::new();
        let converter = PrefixConverter::new(Some("foo"));
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::int_step()),
            Opts::property(TargetClass::string_property()).with_converter(converter.clone()),
        );

        assert_eq!(target.string(), Some(s("foo0")));

        converter.prefix.replace(Some(s("bar")));
        data.raise_property_changed("IntValue");

        assert_eq!(target.string(), Some(s("bar0")));
    }
}
