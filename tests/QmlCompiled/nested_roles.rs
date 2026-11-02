use super::quick_registry;
use ferroui_qml::{Instance, Value};
include!(concat!(env!("OUT_DIR"), "/nested_roles.rs"));
fn exercise(mut ui: Instance) {
    let root = ui.root();
    let call = |ui: &mut Instance, name| ui.call(root, name, vec![]);
    call(&mut ui, "remember").unwrap();
    call(&mut ui, "edit").unwrap();
    assert_eq!(
        call(&mut ui, "nestedEdit").unwrap(),
        Value::String("2/A/BB/true/7".into())
    );
    let saved = ui.get(root, "saved").unwrap();
    assert!(call(&mut ui, "rollbackEdit").is_err());
    assert_eq!(ui.get(root, "saved").unwrap(), saved);
    assert_eq!(
        call(&mut ui, "nestedEdit").unwrap(),
        Value::String("2/A/BB/true/7".into())
    );
    call(&mut ui, "replace").unwrap();
    call(&mut ui, "addNested").unwrap();
    call(&mut ui, "appendGroup").unwrap();
    assert_eq!(
        call(&mut ui, "replaced").unwrap(),
        Value::String("2/X/Y/true/9".into())
    );
    let Value::Object(old) = saved else {
        panic!("missing nested model")
    };
    assert!(ui.type_name(old).is_none());
    call(&mut ui, "clearData").unwrap();
    assert_eq!(
        ui.objects()
            .filter(|id| ui.type_name(*id) == Some("ListModel"))
            .count(),
        1
    );
}
#[test]
fn nested_role_models_from_source_and_generated_rust() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("nested_roles/Main.qml");
    let doc = ferroui_qml_build::Compiler::new().load(path).unwrap();
    exercise(Instance::new(&doc.document, &quick_registry(), Default::default()).unwrap());
    exercise(Instance::new(&compiled_qml(), &quick_registry(), Default::default()).unwrap());
}
#[cfg(feature = "webscene")]
#[test]
fn webscene_can_edit_nested_role_models() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("nested_roles/Main.qml");
    let doc = ferroui_qml_build::Compiler::new()
        .with_javascript()
        .load(path)
        .unwrap();
    exercise(
        Instance::with_engine(
            &doc.document,
            &quick_registry(),
            Default::default(),
            std::rc::Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap()),
        )
        .unwrap(),
    );
}
