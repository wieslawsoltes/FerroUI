use ferroui_qml::{Instance, TypeDefinition, TypeRegistry, Value};
include!(concat!(env!("OUT_DIR"), "/ranges.rs"));

fn registry() -> TypeRegistry {
    let mut types = super::quick_registry();
    let base = TypeDefinition::default().inherits("QtQuick", "Item");
    for (name, ty) in [
        ("Slider", base.clone().quick_slider()),
        ("SpinBox", base.clone().quick_spin_box()),
        ("ProgressBar", base.quick_progress_bar()),
    ] {
        types.register("QtQuick.Controls", name, ty).unwrap();
    }
    types
}
fn exercise(mut ui: Instance) {
    let id = ui.root();
    let report = |ui: &mut Instance| ui.call(id, "report", vec![]).unwrap();
    assert_eq!(
        report(&mut ui),
        Value::String("6/0.4/0.6/0.4/6/6/4/0/0".into())
    );
    ui.call(id, "step", vec![]).unwrap();
    assert_eq!(
        report(&mut ui),
        Value::String("9/0.1/0.9/0.1/3/3/4/0/0".into())
    );
    ui.call(id, "clamp", vec![]).unwrap();
    assert_eq!(
        report(&mut ui),
        Value::String("10/0/1/0/10/10/4/0/0".into())
    );
}
#[test]
fn ranges_from_compiled_rust() {
    exercise(Instance::new(&compiled_qml(), &registry(), Default::default()).unwrap());
}
#[cfg(feature = "webscene")]
#[test]
fn ranges_with_webscene() {
    use std::{path::Path, rc::Rc};
    let unit = ferroui_qml_build::Compiler::new()
        .with_javascript()
        .load(Path::new(env!("CARGO_MANIFEST_DIR")).join("ranges/Main.qml"))
        .unwrap();
    exercise(
        Instance::with_engine(
            &unit.document,
            &registry(),
            Default::default(),
            Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap()),
        )
        .unwrap(),
    );
}
