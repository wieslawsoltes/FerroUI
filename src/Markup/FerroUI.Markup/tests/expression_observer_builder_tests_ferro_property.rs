//! Ported from the upstream expression observer builder tests for registered
//! properties (the test class is named after this port's property class).

use super::test_support::*;
use ferroui_base::data::core::BindingExpression;
use ferroui_base::*;
use std::rc::Rc;

#[repr(C)]
struct Class1 {
    base: FerroObject,
}

ferro_class!(Class1: FerroObject);
ferro_impl_classes!(Class1: FerroObjectImpl);

impl Class1 {
    ferro_property!(fn foo_property() -> StyledProperty<String> {
        FerroProperty::register::<Class1, _>("Foo", s("foo"))
    });

    fn new() -> Ref<Self> {
        Self::foo_property();
        instantiate(Self { base: FerroObject::construct() })
    }
}

fn build(source: &Ref<Class1>, path: &str) -> Rc<BindingExpression> {
    let source: BoxedValue = Rc::new(source.clone().upcast::<FerroObject>());
    build_expression(Some(source), path, None, false).expect("a valid path")
}

#[test]
fn should_get_ferro_property_by_name() {
    let data = Class1::new();
    let target = build(&data, "Foo").to_observable(None);
    let result = take_one(&*target);

    assert_eq!(as_string(&result).as_deref(), Some("foo"));

    assert_eq!(data.property_changed_subscriber_count(), 0);
}

#[test]
fn should_track_ferro_property_by_name() {
    let data = Class1::new();
    let target = build(&data, "Foo");

    let (result, sub) = record(&*target.to_observable(None));
    data.set_value(Class1::foo_property(), s("bar"));

    let values: Vec<_> = result.borrow().iter().map(as_string).collect();
    assert_eq!(values, vec![Some(s("foo")), Some(s("bar"))]);

    sub.dispose();

    assert_eq!(data.property_changed_subscriber_count(), 0);
}
