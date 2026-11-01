use super::quick_registry;
use ferroui_qml::{Instance, Value};
include!(concat!(env!("OUT_DIR"), "/views.rs"));

fn exercise(mut ui: Instance) {
    let root = ui.root();
    ui.call(root, "scrollMiddle", vec![]).unwrap();
    assert_eq!(
        ui.call(root, "report", vec![]).unwrap(),
        Value::String("1000000/20000000/10000000/500000/500000/true".into())
    );
    ui.call(root, "changeCurrent", vec![]).unwrap();
    assert_eq!(
        ui.call(root, "selection", vec![]).unwrap(),
        Value::String("0/updated/updated".into())
    );
    assert!(ui.objects().count() < 24);
}
#[test]
fn virtualized_view_from_generated_rust() {
    exercise(Instance::new(&compiled_qml(), &quick_registry(), Default::default()).unwrap());
}
#[cfg(feature = "webscene")]
#[test]
fn virtualized_view_with_webscene() {
    use std::{path::Path, rc::Rc};
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("views/Main.qml");
    let source = ferroui_qml_build::Compiler::new()
        .with_javascript()
        .load(path)
        .unwrap();
    exercise(
        Instance::with_engine(
            &source.document,
            &quick_registry(),
            Default::default(),
            Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap()),
        )
        .unwrap(),
    );
}
