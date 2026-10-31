use ferroui_qml::{ir, Error, Instance, Parameter, TypeDefinition, TypeRegistry, Value};
use std::collections::BTreeMap;

fn component(body: &str) -> Result<Instance, Error> {
    let doc = ir::parse(&format!("import QtQml 6.12\nQtObject {{ {body} }}")).unwrap();
    Instance::new(&doc, &TypeRegistry::new(), BTreeMap::new())
}

#[test]
fn rewires_dynamic_reads_and_detaches_imperative_bindings_even_for_equal_values() {
    let mut ui = component(
        "
        property int choice: choose()
        property bool left: true
        property int a: 2
        property int b: 8
        function choose(): int { return left ? a : b }
    ",
    )
    .unwrap();
    let root = ui.root();
    assert_eq!(ui.get(root, "choice").unwrap(), Value::Number(2.));
    ui.set(root, "left", Value::Bool(false)).unwrap();
    assert_eq!(ui.get(root, "choice").unwrap(), Value::Number(8.));
    ui.drain_changes();
    ui.set(root, "a", Value::Number(11.)).unwrap();
    assert_eq!(ui.drain_changes().len(), 1);
    ui.set(root, "b", Value::Number(11.)).unwrap();
    assert_eq!(ui.get(root, "choice").unwrap(), Value::Number(11.));
    ui.set(root, "choice", Value::Number(11.)).unwrap();
    ui.set(root, "b", Value::Number(13.)).unwrap();
    assert_eq!(ui.get(root, "choice").unwrap(), Value::Number(11.));
}

#[test]
fn cycle_rolls_back_source_and_previous_dependencies() {
    let mut ui = component(
        "
        property bool cycle: false
        property int a: cycle ? b : source
        property int b: a + 1
        property int source: 3
    ",
    )
    .unwrap();
    let root = ui.root();
    assert!(ui
        .set(root, "cycle", Value::Bool(true))
        .unwrap_err()
        .message
        .contains("cycle"));
    assert_eq!(ui.get(root, "cycle").unwrap(), Value::Bool(false));
    assert!(ui.drain_changes().is_empty());
    ui.set(root, "source", Value::Number(6.)).unwrap();
    assert_eq!(ui.get(root, "b").unwrap(), Value::Number(7.));
    assert_eq!(ui.drain_changes().len(), 3);
}

#[test]
fn aliases_point_to_later_children_and_forward_notifications() {
    let mut ui = component(
        "
        id: root
        property alias counter: model.count
        property alias modelObject: model
        property int changes: 0
        onCounterChanged: changes += 1
        property QtObject child: QtObject { id: model; property int count: 2 }
    ",
    )
    .unwrap();
    let root = ui.root();
    let model = ui.id("model").unwrap();
    assert_eq!(ui.children(root), &[model]);
    assert_eq!(ui.get(root, "modelObject").unwrap(), Value::Object(model));
    ui.set(root, "counter", Value::Number(5.)).unwrap();
    assert_eq!(ui.get(model, "count").unwrap(), Value::Number(5.));
    assert_eq!(ui.get(root, "changes").unwrap(), Value::Number(1.));
    assert!(ui.set(root, "modelObject", Value::Null).is_err());
    assert!(component("id: root; property alias a: root.b; property alias b: root.a").is_err());
}

#[test]
fn methods_and_signals_settle_as_one_operation() {
    let mut ui = component(
        "
        property int score: 1
        readonly property int doubled: score * 2
        signal grant(int value)
        onGrant: (n) => { score += n }
        function award(n: int): int { grant(n); return doubled }
        function invalid() { score = 90; doubled = 5 }
        Component.onCompleted: grant(2)
    ",
    )
    .unwrap();
    let root = ui.root();
    assert_eq!(
        ui.call(root, "award", vec![Value::Number(4.)]).unwrap(),
        Value::Number(14.)
    );
    assert_eq!(ui.drain_changes().len(), 2);
    assert!(ui
        .call(root, "invalid", vec![])
        .unwrap_err()
        .message
        .contains("readonly"));
    assert!(ui
        .emit(root, "grant", vec![Value::String("bad".into())])
        .is_err());
    assert!(ui.drain_changes().is_empty());
    assert_eq!(ui.get(root, "score").unwrap(), Value::Number(7.));
}

#[test]
fn registered_module_validates_versions_and_integrates_rust_functions() {
    let mut registry = TypeRegistry::new();
    let parameter = Parameter {
        name: "units".into(),
        ty: Some("int".into()),
    };
    registry
        .register(
            "Demo",
            "Device",
            TypeDefinition::default()
                .property("font.pixelSize", "real", Value::Number(10.))
                .method("scale", vec![parameter], "int", |args| match args {
                    [Value::Number(n)] => Ok(Value::Number(n * 2.)),
                    _ => unreachable!(),
                }),
        )
        .unwrap();
    registry.set_module_version("Demo", 1, 3);
    let source="import Demo 1.2 as Test\nTest.Device { font { pixelSize: 16 }; required property int n; property int r: scale(n) + font.pixelSize }";
    let doc = ir::parse(source).unwrap();
    assert!(Instance::new(&doc, &registry, BTreeMap::new()).is_err());
    let ui = Instance::new(
        &doc,
        &registry,
        BTreeMap::from([("n".into(), Value::Number(3.))]),
    )
    .unwrap();
    assert_eq!(ui.get(ui.root(), "r").unwrap(), Value::Number(22.));
    assert!(Instance::new(
        &ir::parse(&source.replace("1.2", "1.4")).unwrap(),
        &registry,
        BTreeMap::new()
    )
    .is_err());
}

#[test]
fn default_properties_own_children_and_object_handles_cannot_cross_components() {
    let source = "default property list<QtObject> content; QtObject { id: child }; QtObject {}";
    let mut one = component(source).unwrap();
    let two = component(source).unwrap();
    assert_eq!(
        one.get(one.root(), "content").unwrap(),
        Value::List(
            one.children(one.root())
                .iter()
                .copied()
                .map(Value::Object)
                .collect()
        )
    );
    assert!(one.get(two.root(), "content").is_err());
    assert!(one
        .set(
            one.root(),
            "content",
            Value::List(vec![Value::Object(two.root())])
        )
        .is_err());
}

#[test]
fn failed_initialization_reports_unknowns_and_binding_cycles() {
    for body in [
        "unknown: 3",
        "property int a: b; property int b: a",
        "required property int x",
        "onMissing: 1",
        "QtObject {}",
    ] {
        assert!(component(body).is_err(), "{body}");
    }
}

#[test]
fn validator_rolls_back_derived_changes_and_observed_emissions() {
    let mut ui = component("property int n: 0; signal changed(int next); function update(next: int) { n = next; changed(n) }").unwrap();
    let root = ui.root();
    ui.watch_signal(root, "changed").unwrap();
    ui.set_validator(move |view, changes| {
        if changes.iter().any(|c| c.property == "n") && view.get(root, "n")? == Value::Number(-1.) {
            return Err(Error::new("invalid native value"));
        }
        Ok(())
    });
    assert!(ui.call(root, "update", vec![Value::Number(-1.)]).is_err());
    assert!(ui.drain_signals().is_empty());
    assert!(ui.drain_changes().is_empty());
    assert_eq!(ui.get(root, "n").unwrap(), Value::Number(0.));
    ui.call(root, "update", vec![Value::Number(10.)]).unwrap();
    let emissions = ui.drain_signals();
    assert_eq!(emissions.len(), 1);
    assert_eq!(emissions[0].arguments, [Value::Number(10.)]);
    ui.unwatch_signal(root, "changed");
    ui.emit(root, "changed", vec![Value::Number(2.)]).unwrap();
    assert!(ui.drain_signals().is_empty());
}

#[test]
fn native_subtypes_preserve_contract_and_object_assignability() {
    let mut registry = TypeRegistry::new();
    registry
        .register(
            "Demo",
            "Base",
            TypeDefinition::default().property("width", "int", Value::Number(10.)),
        )
        .unwrap();
    registry
        .register(
            "Demo",
            "Derived",
            TypeDefinition::default().inherits("Demo", "Base"),
        )
        .unwrap();
    let doc=ir::parse("import Demo\nBase { property Base item: Derived { width: 21 }; property int childWidth: item.width }").unwrap();
    let ui = Instance::new(&doc, &registry, BTreeMap::new()).unwrap();
    assert_eq!(ui.get(ui.root(), "childWidth").unwrap(), Value::Number(21.));
}

#[test]
fn connections_follow_target_changes_and_deliver_typed_signal_arguments() {
    let mut ui = component(
        "
        id: root
        signal accepted(int n)
        property int total: 0
        property QtObject child: QtObject { id: other; signal accepted(int n) }
        property Connections connection: Connections {
            target: root
            function onAccepted(n) { root.total += n }
        }
    ",
    )
    .unwrap();
    let root = ui.root();
    let other = ui.id("other").unwrap();
    let Value::Object(listener) = ui.get(root, "connection").unwrap() else {
        panic!("expected listener")
    };
    ui.emit(root, "accepted", vec![Value::Number(2.)]).unwrap();
    ui.set(listener, "target", Value::Object(other)).unwrap();
    ui.emit(root, "accepted", vec![Value::Number(9.)]).unwrap();
    ui.emit(other, "accepted", vec![Value::Number(5.)]).unwrap();
    ui.set(listener, "enabled", Value::Bool(false)).unwrap();
    ui.emit(other, "accepted", vec![Value::Number(4.)]).unwrap();
    assert_eq!(ui.get(root, "total").unwrap(), Value::Number(7.));
    assert!(ui.set(listener, "target", Value::Object(listener)).is_err());
    assert_eq!(ui.get(listener, "target").unwrap(), Value::Object(other));
    ui.set(listener, "ignoreUnknownSignals", Value::Bool(true))
        .unwrap();
    ui.set(listener, "target", Value::Object(listener)).unwrap();
}

#[test]
fn property_notify_reaches_parent_connections_and_rust_observers() {
    let mut ui = component(
        "
        id: root
        property int n: 1
        property int observed: 0
        property Connections watcher: Connections {
            function onNChanged() { root.observed = root.n }
        }
    ",
    )
    .unwrap();
    let root = ui.root();
    ui.watch_signal(root, "nChanged").unwrap();
    ui.set(root, "n", Value::Number(42.)).unwrap();
    assert_eq!(ui.get(root, "observed").unwrap(), Value::Number(42.));
    assert_eq!(ui.drain_signals()[0].signal, "nChanged");
}

#[test]
fn default_alias_and_grouped_alias_resolve_forward_references() {
    let mut registry = TypeRegistry::new();
    registry
        .register(
            "Demo",
            "Panel",
            TypeDefinition::default()
                .property("data", "list<QtObject>", Value::List(vec![]))
                .property("font.size", "int", Value::Number(8.)),
        )
        .unwrap();
    let doc=ir::parse("import QtQml\nimport Demo as D\nQtObject { default property alias content: panel.data; property alias size: panel.font.size; property D.Panel holder: D.Panel { id: panel }; QtObject { id: added } }").unwrap();
    let mut view = Instance::new(&doc, &registry, BTreeMap::new()).unwrap();
    let root = view.root();
    assert_eq!(
        view.get(root, "content").unwrap(),
        Value::List(vec![Value::Object(view.id("added").unwrap())])
    );
    view.set(root, "size", Value::Number(12.)).unwrap();
    assert_eq!(
        view.get(view.id("panel").unwrap(), "font.size").unwrap(),
        Value::Number(12.)
    );
}

#[test]
fn equal_type_names_from_distinct_imports_cannot_cross_typed_slots_or_signatures() {
    let mut registry = TypeRegistry::new();
    for module in ["First", "Second"] {
        registry
            .register(module, "Device", TypeDefinition::default())
            .unwrap();
    }
    let doc=ir::parse("import QtQml\nimport First as A\nimport Second as B\nQtObject { property A.Device a: A.Device {}; property B.Device b: B.Device {}; signal assign(x: A.Device); function accept(obj: A.Device): A.Device { return obj } }").unwrap();
    let mut view = Instance::new(&doc, &registry, BTreeMap::new()).unwrap();
    let root = view.root();
    let b = view.get(root, "b").unwrap();
    assert!(view.set(root, "a", b.clone()).is_err());
    assert!(view.emit(root, "assign", vec![b.clone()]).is_err());
    assert!(view.call(root, "accept", vec![b]).is_err());
}
