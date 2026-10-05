//! Ported from the upstream `BindingTests_RelativeSource`.

use super::test_support::*;
use crate::data::Binding;
use ferroui_base::data::core::{ModelRef, Value};
use ferroui_base::data::model::Model;
use ferroui_base::data::RelativeSource;
use ferroui_base::*;
use std::rc::Rc;

struct ValueModel {
    value: String,
}

ferro_model!(ValueModel, |b| b.read_only::<Value<String>>("Value", |o| o.value.clone()));

struct FooModel {
    foo: Rc<ValueModel>,
}

ferro_model!(FooModel, |b| b.read_only::<ModelRef<ValueModel>>("Foo", |o| Some(o.foo.clone())));

fn decorator(name: &str) -> Ref<Decorator> {
    let result = Decorator::new();
    result.set_name(Some(s(name)));
    result
}

fn ancestor_binding(path: &str, level: i32) -> Rc<Binding> {
    let relative_source = RelativeSource::empty();
    relative_source.set_ancestor_type(Some(Decorator::TYPE));
    relative_source.set_ancestor_level(level);
    let binding = Binding::with_path(path);
    binding.set_relative_source(Some(relative_source));
    binding
}

#[test]
fn should_bind_to_first_ancestor() {
    let target = TextBlock::new();
    let decorator = decorator("decorator");
    decorator.set_child(Some(target.clone().upcast()));
    let _root = TestRoot::with_child(&decorator);

    let binding = ancestor_binding("Name", 1);
    target.bind_binding(TextBox::text_property(), &binding);
    assert_eq!(target.text().as_deref(), Some("decorator"));
}

#[test]
fn should_bind_to_second_ancestor() {
    let target = TextBlock::new();
    let decorator2 = decorator("decorator2");
    decorator2.set_child(Some(target.clone().upcast()));
    let decorator1 = decorator("decorator1");
    decorator1.set_child(Some(decorator2.clone().upcast()));
    let _root = TestRoot::with_child(&decorator1);

    let binding = ancestor_binding("Name", 2);

    target.bind_binding(TextBox::text_property(), &binding);
    assert_eq!(target.text().as_deref(), Some("decorator1"));
}

#[test]
fn should_bind_to_derived_ancestor_type() {
    let target = TextBlock::new();
    let border = Border::new();
    border.set_name(Some(s("border")));
    border.set_child(Some(target.clone().upcast()));
    let _root = TestRoot::with_child(&border);

    let binding = ancestor_binding("Name", 1);

    target.bind_binding(TextBox::text_property(), &binding);
    assert_eq!(target.text().as_deref(), Some("border"));
}

#[test]
fn should_produce_null_if_ancestor_not_found() {
    let target = TextBlock::new();
    let decorator = decorator("decorator");
    decorator.set_child(Some(target.clone().upcast()));
    let _root = TestRoot::with_child(&decorator);

    let binding = ancestor_binding("Name", 2);

    target.bind_binding(TextBox::text_property(), &binding);
    assert!(target.text().is_none());
}

#[test]
fn should_update_when_detached_and_attached_to_visual_tree() {
    let target = TextBlock::new();
    let decorator1 = decorator("decorator1");
    decorator1.set_child(Some(target.clone().upcast()));
    let _root1 = TestRoot::with_child(&decorator1);

    let decorator2 = decorator("decorator2");
    let _root2 = TestRoot::with_child(&decorator2);

    let binding = ancestor_binding("Name", 1);

    target.bind_binding(TextBox::text_property(), &binding);
    assert_eq!(target.text().as_deref(), Some("decorator1"));

    decorator1.set_child(None);
    assert!(target.text().is_none());

    decorator2.set_child(Some(target.clone().upcast()));
    assert_eq!(target.text().as_deref(), Some("decorator2"));
}

#[test]
fn should_update_when_detached_and_attached_to_visual_tree_with_binding_path() {
    let view_model = Model::new_model(ValueModel { value: s("Foo") });

    let target = TextBlock::new();
    let decorator1 = decorator("decorator1");
    decorator1.set_child(Some(target.clone().upcast()));
    let root1 = TestRoot::with_child(&decorator1);
    root1.set_data_context(Some(view_model.clone()));

    let decorator2 = decorator("decorator2");
    let root2 = TestRoot::with_child(&decorator2);
    root2.set_data_context(Some(view_model.clone()));

    let binding = ancestor_binding("DataContext.Value", 1);

    target.bind_binding(TextBox::text_property(), &binding);
    assert_eq!(target.text().as_deref(), Some("Foo"));

    decorator1.set_child(None);
    assert!(target.text().is_none());

    decorator2.set_child(Some(target.clone().upcast()));
    assert_eq!(target.text().as_deref(), Some("Foo"));
}

#[test]
fn should_update_when_detached_and_attached_to_visual_tree_with_complex_binding_path() {
    let vm = Model::new_model(FooModel { foo: Model::new_model(ValueModel { value: s("Foo") }) });

    let target = TextBlock::new();
    let decorator1 = decorator("decorator1");
    decorator1.set_child(Some(target.clone().upcast()));
    let root1 = TestRoot::with_child(&decorator1);
    root1.set_data_context(Some(vm.clone()));

    let decorator2 = decorator("decorator2");
    let root2 = TestRoot::with_child(&decorator2);
    root2.set_data_context(Some(vm.clone()));

    let binding = ancestor_binding("DataContext.Foo.Value", 1);

    target.bind_binding(TextBox::text_property(), &binding);
    assert_eq!(target.text().as_deref(), Some("Foo"));

    decorator1.set_child(None);
    assert!(target.text().is_none());

    decorator2.set_child(Some(target.clone().upcast()));
    assert_eq!(target.text().as_deref(), Some("Foo"));
}
