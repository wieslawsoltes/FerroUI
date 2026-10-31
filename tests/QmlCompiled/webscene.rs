use ferroui_qml::{ComponentSource, Instance, TypeRegistry, Value};
use ferroui_qml_build::Compiler;
use ferroui_qml_webscene::WebSceneEngine;
use std::{path::Path, rc::Rc};

include!(concat!(env!("OUT_DIR"), "/webscene.rs"));

fn exercise(source: &impl ComponentSource) {
    let engine = Rc::new(WebSceneEngine::from_env().expect("load the pinned WebScene runtime"));
    let mut app =
        Instance::with_engine(source, &TypeRegistry::new(), Default::default(), engine).unwrap();
    let root = app.root();
    let n = Value::Number;
    let s = |value: &str| Value::String(value.to_owned());
    assert_eq!(app.get(root, "caption").unwrap(), s("qty-012"));
    assert_eq!(app.get(root, "blockValue").unwrap(), n(20.));
    assert_eq!(app.call(root, "scopeProbe", vec![]).unwrap(), s("root/8"));
    assert_eq!(app.call(root, "cycle", vec![n(3.)]).unwrap(), n(8.));
    app.set(root, "right", n(9.)).unwrap();
    assert_eq!(app.get(root, "score").unwrap(), n(12.));
    app.set(root, "alternate", Value::Bool(true)).unwrap();
    assert_eq!(app.get(root, "score").unwrap(), n(54.));
    assert_eq!(app.call(root, "modern", vec![]).unwrap(), s("#4|#16|#1"));
    app.call(root, "saveClosure", vec![n(10.)]).unwrap();
    assert_eq!(app.call(root, "useClosure", vec![n(2.)]).unwrap(), n(12.));
    assert_eq!(app.call(root, "useClosure", vec![n(3.)]).unwrap(), n(15.));
    assert_eq!(
        app.call(root, "modules", vec![]).unwrap(),
        s("1/1:1/2:2/3:1:1")
    );
    app.set(root, "alternate", Value::Bool(false)).unwrap();
    assert_eq!(app.call(root, "throughHost", vec![n(4.)]).unwrap(), n(30.));
    let failure = app.call(root, "fail", vec![]).unwrap_err();
    assert!(
        failure.to_string().contains("fixture rollback"),
        "{failure}"
    );
    assert_eq!(app.get(root, "left").unwrap(), n(5.));
    assert_eq!(app.get(root, "caption").unwrap(), s("qty-030"));
}

#[test]
fn source_and_cargo_embedded_modules_execute_on_webscene_v8() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("webscene/App.qml");
    let linked = Compiler::new().with_javascript().load(path).unwrap();
    assert!(linked
        .dependencies
        .iter()
        .any(|p| p.ends_with("math/scale.mjs")));
    exercise(&linked.document);
    exercise(&compiled_qml());
}

#[path = "webscene_promise.rs"]
mod promises;
