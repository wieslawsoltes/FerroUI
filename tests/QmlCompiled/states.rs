use ferroui_qml::{Instance, Value};
use std::{path::Path, time::Duration};
mod built {
    include!(concat!(env!("OUT_DIR"), "/states.rs"));
}
fn exercise(mut m: Instance) {
    let root = m.root();
    m.call(root, "open", vec![]).unwrap();
    assert_eq!(m.get(root, "width").unwrap(), Value::Number(20.));
    assert_eq!(m.get(root, "value").unwrap(), Value::Number(50.));
    m.advance_time(Duration::from_millis(25)).unwrap();
    assert_eq!(m.get(root, "width").unwrap(), Value::Number(30.));
    m.advance_time(Duration::from_millis(75)).unwrap();
    assert_eq!(m.get(root, "width").unwrap(), Value::Number(100.));
    assert_eq!(m.get(root, "value").unwrap(), Value::Number(102.));
    m.call(root, "close", vec![]).unwrap();
    m.advance_time(Duration::from_millis(100)).unwrap();
    assert_eq!(m.get(root, "width").unwrap(), Value::Number(20.));
    assert_eq!(m.get(root, "value").unwrap(), Value::Number(50.));
    m.call(root, "play", vec![]).unwrap();
    for (ms, gauge, accent, latched) in [
        (50, 5., "#000000", false),
        (100, 10., "#7f7f7f", false),
        (125, 10., "#ffffff", false),
        (75, 15., "#ffffff", true),
        (50, 20., "#ffffff", true),
    ] {
        m.advance_time(Duration::from_millis(ms)).unwrap();
        assert_eq!(m.get(root, "gauge").unwrap(), Value::Number(gauge));
        assert_eq!(m.get(root, "accent").unwrap(), Value::String(accent.into()));
        assert_eq!(m.get(root, "latched").unwrap(), Value::Bool(latched));
    }
}
fn path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("states/Main.qml")
}
#[test]
fn state_changes_and_transitions_in_source_and_compiled_rust() {
    let types = super::quick_registry();
    let loaded = ferroui_qml_build::Compiler::new().load(path()).unwrap();
    exercise(Instance::new(&loaded.document, &types, Default::default()).unwrap());
    exercise(Instance::new(&built::compiled_qml(), &types, Default::default()).unwrap());
}
#[cfg(feature = "webscene")]
#[test]
fn state_changes_and_transitions_in_source_and_compiled_v8() {
    mod guest {
        include!(concat!(env!("OUT_DIR"), "/states_guest.rs"));
    }
    let types = super::quick_registry();
    let engine = std::rc::Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap());
    let loaded = ferroui_qml_build::Compiler::new()
        .with_javascript()
        .load(path())
        .unwrap();
    exercise(
        Instance::with_engine(&loaded.document, &types, Default::default(), engine.clone())
            .unwrap(),
    );
    exercise(
        Instance::with_engine(&guest::compiled_qml(), &types, Default::default(), engine).unwrap(),
    );
}
