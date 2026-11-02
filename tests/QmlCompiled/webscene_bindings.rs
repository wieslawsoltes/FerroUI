use super::*;
use std::time::Duration;
include!(concat!(env!("OUT_DIR"), "/webscene_bindings.rs"));

fn exercise(source: &impl ComponentSource) {
    let engine = Rc::new(WebSceneEngine::from_env().unwrap());
    let mut app =
        Instance::with_engine(source, &TypeRegistry::new(), Default::default(), engine).unwrap();
    let root = app.root();
    let n = Value::Number;
    let call = |app: &mut Instance, name| app.call(root, name, vec![]);
    assert_eq!(
        call(&mut app, "marker").unwrap(),
        Value::String("function/false/TypeError/Error/Error".into())
    );
    assert_eq!(call(&mut app, "installBranch").unwrap(), n(109.));
    app.set(root, "right", n(11.)).unwrap();
    assert_eq!(app.get(root, "mapped").unwrap(), n(109.));
    app.set(root, "first", Value::Bool(false)).unwrap();
    assert_eq!(app.get(root, "mapped").unwrap(), n(216.));
    app.set(root, "left", n(30.)).unwrap();
    assert_eq!(app.get(root, "mapped").unwrap(), n(216.));
    assert_eq!(call(&mut app, "installAlias").unwrap(), n(111.));
    assert!(call(&mut app, "fail")
        .unwrap_err()
        .message
        .contains("undo installed binding"));
    assert_eq!(app.get(root, "mapped").unwrap(), n(111.));
    app.set(root, "right", n(12.)).unwrap();
    assert_eq!(app.get(root, "mapped").unwrap(), n(112.));
    app.set(root, "right", n(11.)).unwrap();
    assert_eq!(call(&mut app, "installArrow").unwrap(), n(130.));
    for name in ["cycle", "identityCycle"] {
        assert!(call(&mut app, name)
            .unwrap_err()
            .message
            .contains("binding cycle"));
    }
    assert_eq!(app.get(root, "mapped").unwrap(), n(130.));
    assert!(call(&mut app, "readOnly")
        .unwrap_err()
        .message
        .contains("readonly"));
    assert!(call(&mut app, "unsupported")
        .unwrap_err()
        .message
        .contains("noSuchProperty"));
    app.call(root, "overwrite", vec![n(42.)]).unwrap();
    app.set(root, "left", n(40.)).unwrap();
    assert_eq!(app.get(root, "mapped").unwrap(), n(42.));
    call(&mut app, "installNumbers").unwrap();
    assert_eq!(
        call(&mut app, "values").unwrap(),
        Value::String("40,11".into())
    );
    app.set(root, "enabled", Value::Bool(true)).unwrap();
    assert_eq!(call(&mut app, "installControlled").unwrap(), n(111.));
    app.set(root, "right", n(15.)).unwrap();
    assert_eq!(app.get(root, "controlled").unwrap(), n(115.));
    app.set(root, "left", n(4.)).unwrap();
    assert_eq!(app.get(root, "controlled").unwrap(), n(6.));
    app.set(root, "right", n(16.)).unwrap();
    assert_eq!(app.get(root, "controlled").unwrap(), n(6.));
    app.set(root, "enabled", Value::Bool(false)).unwrap();
    assert_eq!(app.get(root, "controlled").unwrap(), n(5.));
    assert_eq!(
        app.get(root, "numbers").unwrap(),
        Value::List(vec![n(4.), n(16.)])
    );
}

#[test]
fn imperative_bindings_track_source_changes_from_rust_and_js() {
    let source = Compiler::new()
        .with_javascript()
        .load(Path::new(env!("CARGO_MANIFEST_DIR")).join("webscene/Bindings.qml"))
        .unwrap();
    exercise(&source.document);
    exercise(&compiled_qml());
}

#[test]
fn stale_binding_rolls_back_and_recovers_by_assignment_or_rebinding() {
    let engine = Rc::new(
        WebSceneEngine::from_env()
            .unwrap()
            .with_timeout(Duration::from_millis(400)),
    );
    let mut app = Instance::with_engine(
        &compiled_qml(),
        &TypeRegistry::new(),
        Default::default(),
        engine,
    )
    .unwrap();
    let root = app.root();
    app.call(root, "installAlias", vec![]).unwrap();
    assert!(app.call(root, "spin", vec![]).is_err());
    assert!(app
        .set(root, "right", Value::Number(22.))
        .unwrap_err()
        .message
        .contains("binding expired"));
    assert_eq!(app.get(root, "right").unwrap(), Value::Number(9.));
    assert_eq!(app.get(root, "mapped").unwrap(), Value::Number(109.));
    app.call(root, "installAlias", vec![]).unwrap();
    app.set(root, "right", Value::Number(22.)).unwrap();
    assert_eq!(app.get(root, "mapped").unwrap(), Value::Number(122.));
    assert!(app.call(root, "spin", vec![]).is_err());
    app.set(root, "mapped", Value::Number(3.)).unwrap();
    app.set(root, "right", Value::Number(9.)).unwrap();
    assert_eq!(app.get(root, "mapped").unwrap(), Value::Number(3.));
}

#[test]
fn a_binding_marker_requires_an_imperative_assignment() {
    let doc = ferroui_qml::ir::parse_external(
        "import QtQml\nQtObject { property var invalid: Qt.binding(() => 2) }",
    )
    .unwrap();
    let engine = Rc::new(WebSceneEngine::from_env().unwrap());
    let err = Instance::with_engine(&doc, &TypeRegistry::new(), Default::default(), engine)
        .err()
        .expect("a binding marker cannot initialize a declarative property");
    assert!(err.message.contains("assigned imperatively"));
}
