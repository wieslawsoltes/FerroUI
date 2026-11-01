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
