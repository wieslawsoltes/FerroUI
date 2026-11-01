use ferroui_qml::{AnchorEdge, Instance, TypeDefinition, TypeRegistry, Value};
include!(concat!(env!("OUT_DIR"), "/anchors.rs"));
fn types() -> TypeRegistry {
    let mut types = TypeRegistry::new();
    let item = TypeDefinition::default()
        .inherits("QtQml", "QtObject")
        .quick_anchors()
        .property("x", "real", Value::Number(0.))
        .property("y", "real", Value::Number(0.))
        .property("width", "real", Value::Undefined)
        .property("height", "real", Value::Undefined)
        .property("data", "list<QtObject>", Value::List(vec![]))
        .default_property("data")
        .visual_children("data");
    types.register("QtQuick", "Item", item).unwrap();
    types.reexport("QtQuick", "QtQml").unwrap();
    types
}
fn check(mut ui: Instance) {
    let root = ui.root();
    let follower = ui.id("follower").unwrap();
    let marker = ui.id("marker").unwrap();
    assert_eq!(
        ui.get(follower, "anchors.left").unwrap(),
        Value::AnchorLine(marker, AnchorEdge::Right)
    );
    ui.call(root, "resetLeft", vec![]).unwrap();
    assert_eq!(
        ui.get(follower, "anchors.left").unwrap(),
        Value::AnchorLine(root, AnchorEdge::Left)
    );
    ui.call(root, "detachRight", vec![]).unwrap();
    assert_eq!(ui.get(follower, "anchors.right").unwrap(), Value::Undefined);
    ui.call(root, "attachLeftToMarker", vec![]).unwrap();
    assert_eq!(
        ui.get(follower, "anchors.left").unwrap(),
        Value::AnchorLine(marker, AnchorEdge::Right)
    );
}
#[test]
fn generated_rust_transports_anchor_lines() {
    check(Instance::new(&compiled_qml(), &types(), Default::default()).unwrap())
}
#[test]
#[cfg(feature = "webscene")]
fn webscene_can_read_and_reassign_anchor_groups() {
    let src = ferroui_qml_build::Compiler::new()
        .with_javascript()
        .load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("anchors/Main.qml"))
        .unwrap();
    let engine = std::rc::Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap());
    check(Instance::with_engine(&src.document, &types(), Default::default(), engine).unwrap());
}

mod mirror {
    include!(concat!(env!("OUT_DIR"), "/mirrored.rs"));
}

fn mirror_changes(mut ui: Instance) {
    let root = ui.root();
    let inherited = |app: &mut Instance| app.call(root, "inherited", vec![]).unwrap();
    assert_eq!(inherited(&mut ui), Value::String("true/false/true".into()));
    ui.call(root, "inheritLocal", vec![]).unwrap();
    assert_eq!(inherited(&mut ui), Value::String("true/false/false".into()));
    ui.call(root, "resetLocal", vec![]).unwrap();
    assert_eq!(inherited(&mut ui), Value::String("true/true/true".into()));
    ui.call(root, "change", vec![]).unwrap();
    assert_eq!(
        inherited(&mut ui),
        Value::String("false/false/false".into())
    );
}
#[test]
fn native_mirroring_inherits_and_can_reset_an_explicit_override() {
    mirror_changes(Instance::new(&mirror::compiled_qml(), &types(), Default::default()).unwrap());
}
#[cfg(feature = "webscene")]
#[test]
fn guest_mirroring_reads_and_writes_the_attached_properties() {
    let doc = ferroui_qml_build::Compiler::new()
        .with_javascript()
        .load("anchors/Mirrored.qml")
        .unwrap()
        .document;
    let guest = std::rc::Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap());
    mirror_changes(Instance::with_engine(&doc, &types(), Default::default(), guest).unwrap());
}
