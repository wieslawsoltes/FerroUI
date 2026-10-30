//! Port of the upstream `MultiBinding` object tests.
//!
//! The three `MultiValueConverter_*` tests of the upstream file are with the
//! converter (`data/converters/func_multi_value_converter.rs`).

use super::*;
use crate::data::converters::{FuncMultiValueConverter, IMultiValueConverter};
use crate::data::core::ValueTypes;
use crate::data::MultiBinding;
use crate::ferro_object_extensions::to_binding;
use crate::*;

#[repr(C)]
pub struct Class1 {
    base: FerroObject,
}

ferro_class!(Class1: FerroObject);
ferro_impl_classes!(Class1: FerroObjectImpl);

impl Class1 {
    ferro_property!(pub fn foo_property() -> StyledProperty<Option<String>> {
        FerroProperty::register::<Class1, _>("Foo", None)
    });

    pub fn new() -> Ref<Self> {
        instantiate(Self { base: FerroObject::construct() })
    }

    pub fn foo(&self) -> Option<String> {
        self.get_value(Self::foo_property())
    }
}

fn string_join_converter() -> Rc<dyn IMultiValueConverter> {
    Rc::new(FuncMultiValueConverter::<Option<BoxedValue>, String>::new(|v| {
        // As `string.Join`: a null item is empty text.
        v.iter().map(|x| x.as_ref().map(|x| ValueTypes::to_display_string(Some(x))).unwrap_or_default()).collect::<Vec<_>>().join(",")
    }))
}

fn object(value: &str) -> Option<BoxedValue> {
    Some(boxed(s(value)))
}

#[test]
fn should_update() {
    let target = Class1::new();

    let b = Subject::<Option<BoxedValue>>::new();

    let mb = MultiBinding::new().with_converter(string_join_converter()).with_bindings(vec![to_binding(b.observable())]);
    target.bind_binding(Class1::foo_property(), &mb);

    assert_eq!(None, target.foo());

    b.on_next(object("Foo"));

    assert_eq!(Some(s("Foo")), target.foo());

    b.on_next(object("Bar"));

    assert_eq!(Some(s("Bar")), target.foo());
}

#[test]
fn should_update_with_multiple_bindings() {
    let target = Class1::new();

    let bindings: Vec<_> = (0..3).map(|_| Subject::<Option<BoxedValue>>::behavior(object("Empty"))).collect();

    let mb = MultiBinding::new()
        .with_converter(string_join_converter())
        .with_bindings(bindings.iter().map(|b| to_binding(b.observable())).collect());
    target.bind_binding(Class1::foo_property(), &mb);

    assert_eq!(Some(s("Empty,Empty,Empty")), target.foo());

    bindings[0].on_next(object("Foo"));

    assert_eq!(Some(s("Foo,Empty,Empty")), target.foo());

    bindings[1].on_next(object("Bar"));

    assert_eq!(Some(s("Foo,Bar,Empty")), target.foo());

    bindings[2].on_next(object("Baz"));

    assert_eq!(Some(s("Foo,Bar,Baz")), target.foo());
}

#[test]
fn should_update_when_null_value_in_bindings() {
    let target = Class1::new();

    let b = Subject::<Option<BoxedValue>>::new();

    let mb = MultiBinding::new().with_converter(string_join_converter()).with_bindings(vec![to_binding(b.observable())]);
    target.bind_binding(Class1::foo_property(), &mb);

    assert_eq!(None, target.foo());

    b.on_next(object("Foo"));

    assert_eq!(Some(s("Foo")), target.foo());

    b.on_next(None);

    assert_eq!(Some(s("")), target.foo());
}

#[test]
fn should_update_when_null_value_in_bindings_with_string_format() {
    let target = Class1::new();

    let b = Subject::<Option<BoxedValue>>::new();

    let mb = MultiBinding::new()
        .with_string_format(Some(s("Converted: {0}")))
        .with_bindings(vec![to_binding(b.observable())]);
    target.bind_binding(Class1::foo_property(), &mb);

    assert_eq!(None, target.foo());
    b.on_next(object("Foo"));
    assert_eq!(Some(s("Converted: Foo")), target.foo());
    b.on_next(None);
    assert_eq!(Some(s("Converted: ")), target.foo());
}
