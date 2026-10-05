//! Ported from the upstream `ExpressionObserverBuilderTests_AttachedProperty`.

use super::test_support::*;
use ferroui_base::data::core::expression_nodes::CastTarget;
use ferroui_base::data::core::parsers::TypeResolver;
use ferroui_base::data::core::{ExpressionParseException, ObservableSink, ValueTypes};
use ferroui_base::*;
use std::rc::Rc;

struct Owner;

impl StaticType for Owner {
    const TYPE: &'static TypeInfo = {
        static TYPE: TypeInfo = TypeInfo::new("Owner", None);
        &TYPE
    };
}

impl Owner {
    ferro_property!(fn foo_property() -> AttachedProperty<String> {
        FerroProperty::register_attached::<Owner, Class1, _>("Foo", s("foo"))
    });

    ferro_property!(fn something_property() -> AttachedProperty<Option<Ref<FerroObject>>> {
        FerroProperty::register_attached::<Owner, Class1, _>("Something", None)
    });
}

#[repr(C)]
struct Class1 {
    base: FerroObject,
}

ferro_class!(Class1: FerroObject);
ferro_impl_classes!(Class1: FerroObjectImpl);

impl Class1 {
    ferro_property!(fn next_property() -> StyledProperty<Option<Ref<Class1>>> {
        FerroProperty::register::<Class1, _>("Next", None)
    });

    fn new() -> Ref<Self> {
        Self::next_property();
        Owner::foo_property();
        Owner::something_property();
        ValueTypes::register_object::<Class1>();
        instantiate(Self { base: FerroObject::construct() })
    }

    fn set_next(&self, value: &Ref<Class1>) {
        self.set_value(Self::next_property(), Some(value.clone()))
    }
}

fn type_resolver() -> TypeResolver {
    Rc::new(|_, name| (name == "Owner").then_some(CastTarget::Class(Owner::TYPE)))
}

fn build(
    source: &Ref<Class1>,
    path: &str,
    type_resolver: &TypeResolver,
) -> Result<Rc<ObservableSink>, ExpressionParseException> {
    let source: BoxedValue = Rc::new(source.clone().upcast::<FerroObject>());
    Ok(build_expression(Some(source), path, Some(type_resolver), false)?.to_observable(None))
}

#[test]
fn should_get_attached_property_value() {
    let data = Class1::new();
    let target = build(&data, "(Owner.Foo)", &type_resolver()).unwrap();
    let result = take_one(&*target);

    assert_eq!(as_string(&result).as_deref(), Some("foo"));

    assert_eq!(data.property_changed_subscriber_count(), 0);
}

#[test]
fn should_get_attached_property_value_with_namespace() {
    let data = Class1::new();
    let resolver: TypeResolver =
        Rc::new(|ns, name| (ns == Some("NS") && name == "Owner").then_some(CastTarget::Class(Owner::TYPE)));
    let target = build(&data, "(NS:Owner.Foo)", &resolver).unwrap();
    let result = take_one(&*target);

    assert_eq!(as_string(&result).as_deref(), Some("foo"));
    assert_eq!(data.property_changed_subscriber_count(), 0);
}

#[test]
fn should_get_chained_attached_property_value() {
    let data = Class1::new();
    let next = Class1::new();
    next.set_value(Owner::foo_property(), s("bar"));
    data.set_next(&next);

    let target = build(&data, "Next.(Owner.Foo)", &type_resolver()).unwrap();
    let result = take_one(&*target);

    assert_eq!(as_string(&result).as_deref(), Some("bar"));

    assert_eq!(data.property_changed_subscriber_count(), 0);
}

#[test]
fn should_get_chained_attached_property_value_with_type_cast() {
    let expected = Class1::new();

    let data = Class1::new();
    let something = Class1::new();
    something.set_next(&expected);
    data.set_value(Owner::something_property(), Some(something.clone().upcast()));

    let owner_resolver = type_resolver();
    let resolver: TypeResolver = Rc::new(move |ns, name| {
        if name == "Class1" {
            Some(CastTarget::Class(Class1::TYPE))
        } else {
            owner_resolver(ns, name)
        }
    });
    let target = build(&data, "((Class1)(Owner.Something)).Next", &resolver).unwrap();
    let result = take_one(&*target);

    let result = result.as_ref().and_then(|v| ValueTypes::as_object(&**v)).expect("an object");
    assert!(result.ptr_eq(&expected));

    assert_eq!(data.property_changed_subscriber_count(), 0);
}

#[test]
fn should_track_simple_attached_value() {
    let data = Class1::new();
    let target = build(&data, "(Owner.Foo)", &type_resolver()).unwrap();

    let (result, sub) = record(&*target);
    data.set_value(Owner::foo_property(), s("bar"));

    let values: Vec<_> = result.borrow().iter().map(as_string).collect();
    assert_eq!(values, vec![Some(s("foo")), Some(s("bar"))]);

    sub.dispose();

    assert_eq!(data.property_changed_subscriber_count(), 0);
}

#[test]
fn should_track_chained_attached_value() {
    let data = Class1::new();
    let next = Class1::new();
    next.set_value(Owner::foo_property(), s("foo"));
    data.set_next(&next);

    let target = build(&data, "Next.(Owner.Foo)", &type_resolver()).unwrap();

    let (result, sub) = record(&*target);
    next.set_value(Owner::foo_property(), s("bar"));

    let values: Vec<_> = result.borrow().iter().map(as_string).collect();
    assert_eq!(values, vec![Some(s("foo")), Some(s("bar"))]);

    sub.dispose();

    assert_eq!(data.property_changed_subscriber_count(), 0);
}

#[test]
fn should_not_keep_source_alive() {
    let run = || {
        let source = Class1::new();
        let target = build(&source, "(Owner.Foo)", &type_resolver()).unwrap();
        (target, source.downgrade())
    };

    let (target, source) = run();
    let (_values, _sub) = record(&*target);

    assert!(source.upgrade().is_none());
}

#[test]
fn should_fail_with_attached_property_with_only_1_part() {
    let data = Class1::new();

    assert!(build(&data, "(Owner.)", &type_resolver()).is_err());
}

#[test]
fn should_fail_with_attached_property_with_more_than_2_parts() {
    let data = Class1::new();

    assert!(build(&data, "(Owner.Foo.Bar)", &type_resolver()).is_err());
}
