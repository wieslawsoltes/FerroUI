use ferroui_qml::{Instance, TypeRegistry, Value};
include!(concat!(env!("OUT_DIR"), "/pragmas.rs"));
fn exercise(mut ui: Instance) {
    let id = ui.root();
    assert_eq!(
        ui.call(id, "arities", vec![]).unwrap(),
        Value::String("39/0/0/kept".into())
    );
    assert_eq!(
        ui.call(id, "policies", vec![]).unwrap(),
        Value::String("2/base/1/replaced".into())
    );
    let report = |ui: &mut Instance| ui.call(id, "report", vec![]).unwrap();
    assert_eq!(
        report(&mut ui),
        Value::String("2/base/sub/1/new/3/old/retained/3/4/7/-1/1/9".into())
    );
    ui.call(id, "change", vec![]).unwrap();
    assert_eq!(
        report(&mut ui),
        Value::String("2/base/sub/1/new/3/old/retained/0/6/5/6/0/0".into())
    );
}
#[test]
fn imported_pragmas_from_generated_rust() {
    exercise(Instance::new(&compiled_qml(), &TypeRegistry::new(), Default::default()).unwrap());
}
#[cfg(feature = "webscene")]
#[test]
fn imported_pragmas_with_webscene() {
    use std::{path::Path, rc::Rc};
    let unit = ferroui_qml_build::Compiler::new()
        .with_javascript()
        .load(Path::new(env!("CARGO_MANIFEST_DIR")).join("pragmas/Main.qml"))
        .unwrap();
    exercise(
        Instance::with_engine(
            &unit.document,
            &TypeRegistry::new(),
            Default::default(),
            Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap()),
        )
        .unwrap(),
    );
}
