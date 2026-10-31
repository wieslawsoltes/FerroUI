use ferroui_qml::{ir, Instance, TypeRegistry, Value};
use std::collections::BTreeMap;

fn load(body: &str) -> Instance {
    let doc = ir::parse(&format!("import QtQml\nQtObject {{ id: root; {body} }}")).unwrap();
    Instance::new(&doc, &TypeRegistry::new(), BTreeMap::new()).unwrap()
}

#[test]
fn conditional_binding_restores_expression_and_updates_dynamic_targets() {
    let mut view=load("
        property int seed: 2
        property int a: seed * 3
        property int b: 15
        property bool active: true
        property Binding overlay: Binding { id: layer; target: root; property: 'a'; when: root.active; value: root.seed + 10 }
    ");
    let root = view.root();
    let layer = view.id("layer").unwrap();
    assert_eq!(view.get(root, "a").unwrap(), Value::Number(12.));
    view.set(root, "seed", Value::Number(5.)).unwrap();
    assert_eq!(view.get(root, "a").unwrap(), Value::Number(15.));
    view.set(layer, "property", Value::String("b".into()))
        .unwrap();
    view.set(root, "seed", Value::Number(6.)).unwrap();
    assert_eq!(view.get(root, "a").unwrap(), Value::Number(18.));
    assert_eq!(view.get(root, "b").unwrap(), Value::Number(16.));
    assert!(view
        .set(layer, "property", Value::String("unknown".into()))
        .is_err());
    assert_eq!(
        view.get(layer, "property").unwrap(),
        Value::String("b".into())
    );
    view.set(root, "active", Value::Bool(false)).unwrap();
    assert_eq!(view.get(root, "b").unwrap(), Value::Number(15.));
}

#[test]
fn overlapping_bindings_restore_in_order_or_keep_effective_value() {
    let mut view=load("
        property int value: 3
        property Binding first: Binding { id: firstBinding; target: root; property: 'value'; value: 4 }
        property Binding second: Binding { id: secondBinding; target: root; property: 'value'; value: 5; restoreMode: Binding.RestoreNone }
    ");
    let root = view.root();
    let first = view.id("firstBinding").unwrap();
    let second = view.id("secondBinding").unwrap();
    assert_eq!(view.get(root, "value").unwrap(), Value::Number(5.));
    view.set(first, "when", Value::Bool(false)).unwrap();
    assert_eq!(view.get(root, "value").unwrap(), Value::Number(5.));
    view.set(second, "when", Value::Bool(false)).unwrap();
    assert_eq!(view.get(root, "value").unwrap(), Value::Number(5.));
    view.set(first, "when", Value::Bool(true)).unwrap();
    assert_eq!(view.get(root, "value").unwrap(), Value::Number(4.));
    view.set(first, "when", Value::Bool(false)).unwrap();
    assert_eq!(view.get(root, "value").unwrap(), Value::Number(5.));
}

#[test]
fn on_property_syntax_is_an_owned_object_outside_the_default_property() {
    let mut view = load(
        "property int n: 3; property int v: 10; Binding on n { id: source; value: root.v + 2 }",
    );
    let root = view.root();
    assert_eq!(view.get(root, "n").unwrap(), Value::Number(12.));
    assert_eq!(view.parent(view.id("source").unwrap()), Some(root));
    view.set(view.id("source").unwrap(), "when", Value::Bool(false))
        .unwrap();
    assert_eq!(view.get(root, "n").unwrap(), Value::Number(3.));
}

#[test]
fn restore_masks_distinguish_scalar_initializers_from_expression_bindings() {
    let mut vm=load("property int literal: -10; property int computed: literal * 2; Binding on literal { id: a; value: 3; restoreMode: Binding.RestoreValue }; Binding on computed { id: b; value: 9; restoreMode: Binding.RestoreValue }");
    let root = vm.root();
    vm.set(vm.id("a").unwrap(), "when", Value::Bool(false))
        .unwrap();
    vm.set(vm.id("b").unwrap(), "when", Value::Bool(false))
        .unwrap();
    assert_eq!(vm.get(root, "literal").unwrap(), Value::Number(-10.));
    assert_eq!(vm.get(root, "computed").unwrap(), Value::Number(9.));
}
