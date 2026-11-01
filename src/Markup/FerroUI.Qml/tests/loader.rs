use ferroui_qml::{ir::parse, Instance, ObjectId, TypeDefinition, TypeRegistry, Value};

fn model(source: &str) -> Instance {
    let mut registry = TypeRegistry::new();
    registry
        .register(
            "QtQuick",
            "Item",
            TypeDefinition::default()
                .inherits("QtQml", "QtObject")
                .property("width", "real", Value::Undefined)
                .property("height", "real", Value::Undefined)
                .property("data", "list<QtObject>", Value::List(vec![]))
                .default_property("data")
                .visual_children("data"),
        )
        .unwrap();
    registry
        .register("QtQuick", "Loader", TypeDefinition::quick_loader())
        .unwrap();
    registry.reexport("QtQuick", "QtQml").unwrap();
    let doc = parse(source).unwrap();
    Instance::new(&doc, &registry, Default::default()).unwrap()
}

fn item(model: &Instance, loader: ObjectId) -> ObjectId {
    let Value::Object(id) = model.get(loader, "item").unwrap() else {
        panic!("Loader has no item")
    };
    id
}

#[test]
fn switching_or_deactivating_an_inline_loader_releases_the_previous_subtree() {
    let mut model = model(
        r#"import QtQuick
        Item { id: root
          property int loads: 0
          Component { id: tile; Item { property int role: 7 } }
          Loader { id: loader; sourceComponent: tile; onLoaded: root.loads++ }
        }"#,
    );
    let loader = model.id("loader").unwrap();
    let first = item(&model, loader);
    assert_eq!(model.get(first, "role").unwrap(), Value::Number(7.));
    assert_eq!(model.get(loader, "status").unwrap(), Value::Number(1.));
    assert_eq!(model.get(model.root(), "loads").unwrap(), Value::Number(1.));
    model.set(loader, "active", Value::Bool(false)).unwrap();
    assert_eq!(model.get(loader, "item").unwrap(), Value::Null);
    assert_eq!(model.get(loader, "data").unwrap(), Value::List(vec![]));
    assert!(model.type_name(first).is_none());
    model.set(loader, "active", Value::Bool(true)).unwrap();
    assert_ne!(first, item(&model, loader));
    model.set(loader, "sourceComponent", Value::Null).unwrap();
    assert_eq!(model.get(loader, "item").unwrap(), Value::Null);
    assert_eq!(model.get(loader, "progress").unwrap(), Value::Number(0.));
}

#[test]
fn inactive_and_replaced_deferred_loaders_do_not_publish_stale_items() {
    let mut model = model(
        r#"import QtQuick
        Item { Component { id: tile; Item {} }
           Loader { id: loader; asynchronous: true; sourceComponent: tile }
        }"#,
    );
    let loader = model.id("loader").unwrap();
    assert_eq!(model.get(loader, "status").unwrap(), Value::Number(2.));
    model.set(loader, "active", Value::Bool(false)).unwrap();
    model.process_deferred(8).unwrap();
    assert_eq!(model.get(loader, "item").unwrap(), Value::Null);
    model.set(loader, "active", Value::Bool(true)).unwrap();
    model.process_deferred(8).unwrap();
    assert_eq!(model.get(loader, "status").unwrap(), Value::Number(1.));
}

#[test]
fn explicit_and_content_driven_loader_axes_follow_qt_sizing_rules() {
    let mut model = model(
        r#"import QtQuick
        Loader { id: host; sourceComponent: Component { Item {width:40; height:12; property real implicitWidth:25 } } }
    "#,
    );
    let root = model.root();
    let child = item(&model, root);
    let n = Value::Number;
    assert_eq!(model.get(root, "width").unwrap(), n(40.));
    model.set(child, "width", n(33.)).unwrap();
    assert_eq!(model.get(root, "implicitWidth").unwrap(), n(33.));
    model.set(root, "width", n(77.)).unwrap();
    assert_eq!(model.get(child, "width").unwrap(), n(77.));
    assert_eq!(model.get(root, "implicitWidth").unwrap(), n(25.));
    assert_eq!(model.get(root, "height").unwrap(), n(12.));
    model.set(child, "width", n(31.)).unwrap();
    assert_eq!(model.get(child, "width").unwrap(), n(31.));
    assert_eq!(model.get(root, "width").unwrap(), n(77.));
    model.set(root, "width", Value::Undefined).unwrap();
    assert_eq!(model.get(root, "width").unwrap(), n(31.));
    model.set(root, "active", Value::Bool(false)).unwrap();
    assert_eq!(model.get(root, "width").unwrap(), n(31.));
}

#[test]
fn missing_required_input_reports_error_without_retaining_a_failed_instance() {
    let model = model(
        r#"import QtQuick
        Loader { sourceComponent: Component { Item {required property int amount} } }
    "#,
    );
    assert_eq!(
        model.get(model.root(), "status").unwrap(),
        Value::Number(3.)
    );
    assert_eq!(model.get(model.root(), "item").unwrap(), Value::Null);
    assert_eq!(model.objects().count(), 2);
}

#[test]
fn component_valued_object_shorthand_defers_its_private_ids_and_initialization() {
    let mut model = model(
        r#"import QtQuick
        Loader {
            active: false
            sourceComponent: Item { id: privateChild; property int amount:21 }
        }"#,
    );
    let root = model.root();
    assert_eq!(model.get(root, "item").unwrap(), Value::Null);
    assert!(model.id("privateChild").is_none());
    assert_eq!(model.objects().count(), 2);
    model.set(root, "active", Value::Bool(true)).unwrap();
    assert_eq!(
        model.get(item(&model, root), "amount").unwrap(),
        Value::Number(21.)
    );
    assert!(model.id("privateChild").is_none());
}

#[test]
fn an_aborted_reload_restores_the_previously_loaded_identity_and_selection() {
    let mut model = model(
        r#"import QtQuick
        Item { id: page; property bool reject:false
            Component {id: a; Item{property bool first:true}}
            Component {id: b; Item{property bool first:false}}
            Loader {id: slot; sourceComponent:a; onLoaded: { if (page.reject) page.unknownMethod() }}
        }"#,
    );
    let loader = model.id("slot").unwrap();
    let first = item(&model, loader);
    let prior = model.structure_revision();
    model
        .set(model.root(), "reject", Value::Bool(true))
        .unwrap();
    assert!(model
        .set(
            loader,
            "sourceComponent",
            Value::Object(model.id("b").unwrap())
        )
        .is_err());
    assert_eq!(item(&model, loader), first);
    assert_eq!(model.get(first, "first").unwrap(), Value::Bool(true));
    assert_eq!(model.structure_revision(), prior);
    model
        .set(model.root(), "reject", Value::Bool(false))
        .unwrap();
    model
        .set(
            loader,
            "sourceComponent",
            Value::Object(model.id("b").unwrap()),
        )
        .unwrap();
    assert!(model.type_name(first).is_none());
    assert_eq!(
        model.get(item(&model, loader), "first").unwrap(),
        Value::Bool(false)
    );
}
