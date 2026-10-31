//! The explicit native suite runs against the published WebScene V8 library.
//! FERROUI_WEBSCENE_LIBRARY=... cargo test -p ferroui-qml-webscene -- --ignored
use ferroui_qml::{Instance, TypeRegistry, Value};
use ferroui_qml_webscene::WebSceneEngine;
use std::{rc::Rc, time::Duration};

fn engine(timeout: Duration) -> Rc<WebSceneEngine> {
    let library =
        std::env::var_os("FERROUI_WEBSCENE_LIBRARY").expect("set FERROUI_WEBSCENE_LIBRARY");
    Rc::new(
        WebSceneEngine::new(env!("CARGO_BIN_EXE_ferroui-qml-webscene-worker"), library)
            .unwrap()
            .with_timeout(timeout),
    )
}
fn load(source: &str, backend: Rc<WebSceneEngine>) -> Instance {
    let doc = ferroui_qml_compiler::parse_external(source).unwrap();
    Instance::with_engine(&doc, &TypeRegistry::new(), Default::default(), backend).unwrap()
}
#[test]
#[ignore = "requires the WebScene native ABI 3 package"]
fn native_v8_tracks_dynamic_dependencies_and_reenters_qml_on_method_calls() {
    let mut app = load(
        r#"
        import QtQml
        QtObject {
            id: root
            property int a: 3
            property int b: 7
            property bool first: true
            property int observed: (first ? a : b) * 2
            function sum(z: int, a: int): int { return arguments[0] * 100 + arguments[1] }
            function update(): int { root.a = 9; return root.sum(root.observed, 2) }
        }
    "#,
        engine(Duration::from_secs(4)),
    );
    let root = app.root();
    assert_eq!(app.get(root, "observed").unwrap(), Value::Number(6.));
    assert_eq!(
        app.call(root, "update", vec![]).unwrap(),
        Value::Number(1802.)
    );
    app.set(root, "first", Value::Bool(false)).unwrap();
    app.set(root, "a", Value::Number(19.)).unwrap();
    assert_eq!(app.get(root, "observed").unwrap(), Value::Number(14.));
}
#[test]
#[ignore = "requires the WebScene native ABI 3 package"]
fn captured_functions_and_js_objects_keep_their_lexical_owner_and_identity() {
    let mut app = load(
        r#"
        import QtQml
        QtObject {
            id: root
            property int seed: 2
            property var counter: (() => { let n = 0; return () => ++n + seed })()
            property var key: new Map()
            property QtObject child: QtObject {
                property int seed: 90
                function test(): int { const m = root.key; m.set(root, 9); return root.counter() + m.get(root) }
            }
        }
    "#,
        engine(Duration::from_secs(4)),
    );
    let Value::Object(child) = app.get(app.root(), "child").unwrap() else {
        panic!()
    };
    assert_eq!(app.call(child, "test", vec![]).unwrap(), Value::Number(12.));
    assert_eq!(app.call(child, "test", vec![]).unwrap(), Value::Number(13.));
}
#[test]
#[ignore = "requires the WebScene native ABI 3 package"]
fn runaway_script_is_killed_and_next_turn_restarts_with_committed_qml_state() {
    let mut app = load(
        r#"
        import QtQml
        QtObject {
            property int n: 7
            function runaway() { n = 8; while (true) {} }
            function read(): int { return n }
        }
    "#,
        engine(Duration::from_millis(750)),
    );
    let root = app.root();
    let error = app.call(root, "runaway", vec![]).unwrap_err();
    assert!(error.message.contains("deadline"), "{error}");
    assert_eq!(app.get(root, "n").unwrap(), Value::Number(7.));
    assert_eq!(app.call(root, "read", vec![]).unwrap(), Value::Number(7.));
}

#[test]
#[ignore = "requires the WebScene native ABI 3 package"]
fn separate_instances_sharing_a_backend_keep_independent_modules_globals_and_deadlines() {
    use ferroui_qml::ir::{self, Binding, Expr, MemberKind, ScriptKind};
    use std::collections::BTreeMap;
    let backend = engine(Duration::from_millis(900));
    let make = |source: &str| {
        let mut document = ferroui_qml_compiler::parse_external(
            r#"
            import QtQml
            QtObject {
                property int count: Lib.next()
                function current(): int { return Lib.next() }
                function runaway() { for (;;) {} }
            }
        "#,
        )
        .unwrap();
        let url = "https://qml.invalid/modules/shared/counter.js";
        document.scripts.insert(
            url.to_string(),
            ir::ScriptResource {
                source: source.into(),
                kind: ScriptKind::Library,
                preload: true,
                imports: BTreeMap::new(),
                dependencies: BTreeMap::new(),
                exports: vec!["next".into()],
            },
        );
        for member in &mut document.root.members {
            match &mut member.kind {
                MemberKind::Property {
                    value: Some(Binding::Expr(Expr::JavaScript(js))),
                    ..
                } => {
                    js.imports.insert("Lib".into(), url.into());
                }
                MemberKind::Function { body, .. } => {
                    if let [ir::Stmt::JavaScript(js)] = body.as_mut_slice() {
                        js.imports.insert("Lib".into(), url.into());
                    }
                }
                _ => (),
            }
        }
        Instance::with_engine(
            &document,
            &TypeRegistry::new(),
            Default::default(),
            backend.clone(),
        )
        .unwrap()
    };
    let counter = "let n=0; function next(){return ++n;}";
    let mut first = make(counter);
    let mut second = make(counter);
    let mut different = make("let n=40; function next(){return ++n;}");
    assert_eq!(first.get(first.root(), "count").unwrap(), Value::Number(1.));
    assert_eq!(
        second.get(second.root(), "count").unwrap(),
        Value::Number(1.)
    );
    assert_eq!(
        different.call(different.root(), "current", vec![]).unwrap(),
        Value::Number(42.)
    );
    assert!(first
        .call(first.root(), "runaway", vec![])
        .unwrap_err()
        .message
        .contains("deadline"));
    assert_eq!(
        second.call(second.root(), "current", vec![]).unwrap(),
        Value::Number(2.)
    );
    assert_eq!(
        different.call(different.root(), "current", vec![]).unwrap(),
        Value::Number(43.)
    );
}
