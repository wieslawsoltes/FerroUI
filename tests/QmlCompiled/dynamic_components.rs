use ferroui_qml::{Instance, ObjectId, TypeRegistry, Value};
use std::{collections::BTreeMap, path::Path};

mod native {
    include!(concat!(env!("OUT_DIR"), "/dynamic_component.rs"));
}
#[cfg(feature = "webscene")]
mod guest {
    include!(concat!(env!("OUT_DIR"), "/dynamic_guest.rs"));
}
fn id(value: Value) -> ObjectId {
    let Value::Object(id) = value else {
        panic!("expected object")
    };
    id
}
fn exercise(mut app: Instance) {
    let root = app.root();
    let factory = id(app.get(root, "factory").unwrap());
    assert!(app.is_component(factory));
    assert!(app.children(factory).is_empty());
    let count = app.objects().count();
    assert!(app
        .create_object(factory, Some(root), BTreeMap::new())
        .is_err());
    assert_eq!(app.objects().count(), count);
    let a = id(app.call(root, "make", vec![Value::Number(4.)]).unwrap());
    let b = app
        .create_object(
            factory,
            None,
            BTreeMap::from([("seed".into(), Value::Number(7.))]),
        )
        .unwrap();
    assert_eq!(app.parent(a), Some(root));
    assert!(app.parent(b).is_none());
    assert_eq!(app.get(a, "score").unwrap(), Value::Number(9.));
    assert_eq!(app.get(b, "score").unwrap(), Value::Number(12.));
    app.set(a, "exposed", Value::Number(5.)).unwrap();
    app.set(root, "offset", Value::Number(10.)).unwrap();
    assert_eq!(app.get(a, "score").unwrap(), Value::Number(19.));
    assert_eq!(app.get(b, "score").unwrap(), Value::Number(20.));
    assert_eq!(app.get(root, "completions").unwrap(), Value::Number(2.));
    assert!(app.id("internal").is_none());
    assert_eq!(app.drain_created(), vec![a, b]);
    assert_eq!(app.structure_revision(), 2);
}
#[test]
fn source_and_rust_share_the_dynamic_component_graph() {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("creation/Main.qml");
    let reg = TypeRegistry::new();
    exercise(
        Instance::new(
            &ferroui_qml::ir::parse(&std::fs::read_to_string(file).unwrap()).unwrap(),
            &reg,
            BTreeMap::new(),
        )
        .unwrap(),
    );
    exercise(Instance::new(&native::compiled_qml(), &reg, BTreeMap::new()).unwrap());
}
#[cfg(feature = "webscene")]
#[test]
fn compiled_and_source_v8_allocate_live_objects() {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("creation/Main.qml");
    let reg = TypeRegistry::new();
    let engine = std::rc::Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap());
    exercise(
        Instance::with_engine(
            &ferroui_qml_build::Compiler::new()
                .with_javascript()
                .load(file)
                .unwrap()
                .document,
            &reg,
            BTreeMap::new(),
            engine.clone(),
        )
        .unwrap(),
    );
    exercise(Instance::with_engine(&guest::compiled_qml(), &reg, BTreeMap::new(), engine).unwrap());
}
