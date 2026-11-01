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

#[test]
fn delayed_binding_coalesces_inputs_and_rolls_back_failed_turns() {
    let mut model = load(
        r#"
        property int source: 2
        property int destination: 7
        property Binding overlay: Binding { id: layer; target: root; property: "destination"; value: root.source; delayed: true }
        function abort() { source = 100; root.missing() }
    "#,
    );
    let root = model.root();
    let layer = model.id("layer").unwrap();
    let n = Value::Number;
    let read = |model: &Instance| model.get(root, "destination").unwrap();
    assert_eq!(read(&model), n(2.0));
    assert!(!model.has_deferred_work());
    assert!(model.call(root, "abort", vec![]).is_err());
    assert!(!model.has_deferred_work());
    for value in [4.0, 8.0] {
        model.set(root, "source", n(value)).unwrap();
    }
    assert_eq!(read(&model), n(2.0));
    assert_eq!(model.process_deferred(8).unwrap(), 1);
    assert_eq!(read(&model), n(8.0));
    model.set(root, "source", n(9.0)).unwrap();
    model.set(layer, "when", Value::Bool(false)).unwrap();
    assert_eq!(read(&model), n(7.0));
    model.process_deferred(8).unwrap();
    assert_eq!(read(&model), n(7.0));
    model.set(layer, "when", Value::Bool(true)).unwrap();
    assert_eq!(read(&model), n(9.0));
    model.set(layer, "delayed", Value::Bool(false)).unwrap();
    model.set(root, "source", n(10.0)).unwrap();
    assert_eq!(read(&model), n(10.0));
}

#[test]
fn grouped_and_dotted_declarations_resolve_forward_ids_and_guard_every_input() {
    let mut model = load(
        r#"
        property int baseline: 2
        property int tracked: baseline * 2
        property int literal: 12
        property QtObject current: null
        property Binding overlay: Binding {
            id: layer
            property int offset: 3
            when: root.current !== null
            root { tracked: root.current.n + offset; literal: this.offset }
            later.n: root.baseline + 5
        }
        property QtObject source: QtObject { id: later; property int n: 10 }
    "#,
    );
    let root = model.root();
    let later = model.id("later").unwrap();
    let values = |m: &Instance| {
        [
            m.get(root, "tracked").unwrap(),
            m.get(root, "literal").unwrap(),
            m.get(later, "n").unwrap(),
        ]
    };
    let n = Value::Number;
    assert_eq!(values(&model), [n(4.), n(12.), n(10.)]);
    model.set(root, "current", Value::Object(later)).unwrap();
    assert_eq!(values(&model), [n(10.), n(3.), n(7.)]);
    model.set(root, "baseline", n(4.)).unwrap();
    assert_eq!(values(&model), [n(12.), n(3.), n(9.)]);
    model.set(root, "current", Value::Null).unwrap();
    assert_eq!(values(&model), [n(8.), n(12.), n(10.)]);
    model.set(root, "baseline", n(6.)).unwrap();
    assert_eq!(values(&model), [n(12.), n(12.), n(10.)]);
}

#[test]
fn multi_target_failures_roll_back_all_overlays_and_interoperate_with_single_target() {
    let mut model = load(
        r#"
        property int x: 1; property int y: 2; property bool enabled: false
        property var source: "incompatible"
        property Binding lower: Binding { id: single; target: root; property: "x"; value: 9 }
        property Binding upper: Binding {
            when: root.enabled
            root.x: 10
            root.y: { return root.source }
            target: root; property: "nonexistent"; value: 99
        }
    "#,
    );
    let root = model.root();
    let n = Value::Number;
    assert!(model.set(root, "enabled", Value::Bool(true)).is_err());
    assert_eq!(model.get(root, "enabled").unwrap(), Value::Bool(false));
    assert_eq!(model.get(root, "x").unwrap(), n(9.));
    assert_eq!(model.get(root, "y").unwrap(), n(2.));
    model.set(root, "source", n(7.)).unwrap();
    model.set(root, "enabled", Value::Bool(true)).unwrap();
    assert_eq!(model.get(root, "x").unwrap(), n(10.));
    assert_eq!(model.get(root, "y").unwrap(), n(7.));
    model
        .set(model.id("single").unwrap(), "when", Value::Bool(false))
        .unwrap();
    model.set(root, "enabled", Value::Bool(false)).unwrap();
    assert_eq!(model.get(root, "x").unwrap(), n(1.));
    assert_eq!(model.get(root, "y").unwrap(), n(2.));
}

#[test]
fn delayed_multi_target_coalesces_one_job_without_rewriting_unchanged_siblings() {
    let mut model = load(
        r#"
        property int source: 2; property int x: 3; property int y: 4; property int z: 8
        property Binding overlay: Binding {
            id: layer; delayed: true
            root { x: root.source; y: root.source * 2; z: 17 }
        }
    "#,
    );
    let root = model.root();
    let n = Value::Number;
    model.set(root, "z", n(99.)).unwrap();
    model.set(root, "source", n(4.)).unwrap();
    model.set(root, "source", n(6.)).unwrap();
    assert_eq!(model.get(root, "x").unwrap(), n(2.));
    assert_eq!(model.get(root, "y").unwrap(), n(4.));
    assert_eq!(model.process_deferred(8).unwrap(), 1);
    assert_eq!(model.get(root, "x").unwrap(), n(6.));
    assert_eq!(model.get(root, "y").unwrap(), n(12.));
    assert_eq!(model.get(root, "z").unwrap(), n(99.));
    model.set(root, "source", n(7.)).unwrap();
    model
        .set(model.id("layer").unwrap(), "when", Value::Bool(false))
        .unwrap();
    model.process_deferred(8).unwrap();
    assert_eq!(model.get(root, "x").unwrap(), n(3.));
    assert_eq!(model.get(root, "y").unwrap(), n(4.));
    assert_eq!(model.get(root, "z").unwrap(), n(8.));
}

#[test]
fn static_target_check_resolves_private_ids_in_each_composite_instance() {
    struct Natives;
    impl ir::ComponentResolver for Natives {
        fn resolve(
            &mut self,
            _: &str,
            _: &[ir::Import],
            _: &str,
        ) -> Result<Option<ir::ComponentSource>, String> {
            Ok(None)
        }
    }
    let doc = ir::parse(r#"import QtQml
        QtObject {
            component Panel: QtObject {
                id: panel; property int source: 2; property int x: 1
                property QtObject content: QtObject { id: dest; property int y: 1 }
                property alias result: dest.y
                property Binding overlay: Binding { dest { y: panel.source }; panel.x: panel.source * 2 }
            }
            property Panel a: Panel { id: one; source: 4 }
            property Panel b: Panel { id: two; source: 9 }
        }"#).unwrap();
    let linked = ir::link(
        ir::ComponentSource {
            key: "Main.qml".into(),
            document: doc.into(),
        },
        &mut Natives,
    )
    .unwrap();
    let registry = TypeRegistry::new();
    registry.check(&linked).unwrap();
    let model = Instance::new(&linked, &registry, BTreeMap::new()).unwrap();
    for (id, expected) in [("one", 4.), ("two", 9.)] {
        let panel = model.id(id).unwrap();
        assert_eq!(model.get(panel, "result").unwrap(), Value::Number(expected));
        assert_eq!(model.get(panel, "x").unwrap(), Value::Number(expected * 2.));
    }
    for (path, error) in [
        ("unknown.x", "unknown Binding target id"),
        ("root.missing", "has no property"),
        ("root.fixed", "readonly"),
    ] {
        let bad = ir::parse(&format!("import QtQml\nQtObject {{ id: root; readonly property int fixed: 5; property Binding b: Binding {{ when: false; {path}: 1 }} }}")).unwrap();
        assert!(registry
            .check(&bad)
            .unwrap_err()
            .to_string()
            .contains(error));
    }
}

#[test]
fn destroying_a_delayed_controller_restores_native_groups_and_cancels_queued_work() {
    use ferroui_qml::TypeDefinition;
    let mut registry = TypeRegistry::new();
    registry
        .register(
            "Demo",
            "Label",
            TypeDefinition::default()
                .inherits("QtQml", "QtObject")
                .property("font.size", "int", Value::Number(12.)),
        )
        .unwrap();
    let doc = ir::parse(
        r#"
        import QtQml
        import Demo
        Label {
            id: label; property int n: 7; property int x: n + 1
            property Component factory: Component {
                Binding { delayed: true; label { font { size: label.n }; x: label.n * 3 } }
            }
        }"#,
    )
    .unwrap();
    let mut model = Instance::new(&doc, &registry, BTreeMap::new()).unwrap();
    let root = model.root();
    let Value::Object(factory) = model.get(root, "factory").unwrap() else {
        panic!()
    };
    let controller = model
        .create_object(factory, Some(root), BTreeMap::new())
        .unwrap();
    let n = Value::Number;
    assert_eq!(model.get(root, "font.size").unwrap(), n(7.));
    model.set(root, "n", n(9.)).unwrap();
    assert!(model.has_deferred_work());
    model.destroy_object(controller).unwrap();
    assert_eq!(model.get(root, "x").unwrap(), n(10.));
    assert_eq!(model.get(root, "font.size").unwrap(), n(12.));
    assert!(!model.has_deferred_work());
}

#[test]
fn conditional_multi_target_rejects_feedback_even_if_all_values_are_unchanged() {
    let mut model = load(
        r#"
        property bool feedback: false
        property int a: 5
        property int b: 5
        property Binding overlay: Binding {
            root.a: root.feedback ? root.b : 5
            root.b: root.a
        }
    "#,
    );
    let root = model.root();
    let error = model.set(root, "feedback", Value::Bool(true)).unwrap_err();
    assert!(error.message.contains("cycle"), "{error}");
    assert_eq!(model.get(root, "feedback").unwrap(), Value::Bool(false));
    assert_eq!(model.get(root, "b").unwrap(), Value::Number(5.));
}

#[test]
fn activating_multi_target_bindings_can_reverse_displaced_dependency_edges() {
    let mut model = load(
        r#"
        property int a: 2; property int b: a; property bool enabled: false
        property Binding layer: Binding { when: root.enabled; root { a: root.b; b: 3 } }
    "#,
    );
    let root = model.root();
    model.set(root, "enabled", Value::Bool(true)).unwrap();
    assert_eq!(model.get(root, "a").unwrap(), Value::Number(3.));
    assert_eq!(model.get(root, "b").unwrap(), Value::Number(3.));
    model.set(root, "enabled", Value::Bool(false)).unwrap();
    assert_eq!(model.get(root, "a").unwrap(), Value::Number(2.));
    assert_eq!(model.get(root, "b").unwrap(), Value::Number(2.));
}
