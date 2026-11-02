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
    use ferroui_qml_build::Compiler;

    let shared = engine(Duration::from_millis(900));
    let files = tempfile::tempdir().unwrap();
    let project = files.path();
    std::fs::write(
        project.join("View.qml"),
        r#"
        import QtQml
        import "Library.js" as Lib
        import "Module.mjs" as Es
        QtObject {
            property int count: Lib.next()
            function both(): string { return Lib.next() + "/" + Es.next() }
            function runaway() { for (;;) {} }
        }
    "#,
    )
    .unwrap();
    let compile = |start: i32| {
        std::fs::write(
            project.join("Library.js"),
            format!(".pragma library\nlet n={start}; function next(){{return ++n}}"),
        )
        .unwrap();
        std::fs::write(
            project.join("Module.mjs"),
            format!("let n={}; export const next=()=>++n;", start * 10),
        )
        .unwrap();
        Compiler::new()
            .with_javascript()
            .load(project.join("View.qml"))
            .unwrap()
            .document
    };
    let create = |document: &ferroui_qml::Document| {
        Instance::with_engine(
            document,
            &TypeRegistry::new(),
            Default::default(),
            shared.clone(),
        )
        .unwrap()
    };
    let original = compile(0);
    let mut first = create(&original);
    let mut second = create(&original);
    let revised = compile(40);
    assert!(original.scripts.keys().eq(revised.scripts.keys()));
    let mut third = create(&revised);
    let call = |app: &mut Instance| app.call(app.root(), "both", vec![]).unwrap();
    assert_eq!(first.get(first.root(), "count").unwrap(), Value::Number(1.));
    assert_eq!(
        second.get(second.root(), "count").unwrap(),
        Value::Number(1.)
    );
    assert_eq!(
        third.get(third.root(), "count").unwrap(),
        Value::Number(41.)
    );
    assert_eq!(call(&mut first), Value::String("2/1".into()));
    assert_eq!(call(&mut second), Value::String("2/1".into()));
    assert_eq!(call(&mut third), Value::String("42/401".into()));
    assert!(first
        .call(first.root(), "runaway", vec![])
        .unwrap_err()
        .message
        .contains("deadline"));
    assert_eq!(call(&mut second), Value::String("3/2".into()));
    assert_eq!(call(&mut third), Value::String("43/402".into()));
}

#[test]
#[ignore = "requires the WebScene native ABI 3 package"]
fn collecting_transient_host_proxies_releases_records_but_preserves_retained_cycles_and_aliases() {
    use ferroui_qml::Namespace;
    use std::collections::BTreeMap;
    let mut app = load(
        r#"
        import QtQml
        QtObject {
            function pin(item) { globalThis.kept = item; globalThis.lookup = new Map([[item, 41]]) }
            function consume(item): int { return item.index }
            function retained(): bool { return kept === kept.self && lookup.get(kept.self) === 41 }
        }
    "#,
        engine(Duration::from_secs(5)),
    );
    let root = app.root();
    let kept = app.new_record(BTreeMap::new()).unwrap();
    app.set_record_property(&kept, "self", Value::Record(kept.clone()))
        .unwrap();
    app.call(root, "pin", vec![Value::Record(kept)]).unwrap();

    // The ABI requests V8 low-memory work at pressure thresholds; reaching
    // several thresholds exercises separate JS jobs without sleeps or racing
    // FinalizationRegistry callbacks. The identity stays valid in the Map.
    let sentinel = Rc::new(Namespace::default());
    let weak = Rc::downgrade(&sentinel);
    let mut payload = Some(sentinel);
    for index in 0..1024 {
        let mut fields = BTreeMap::from([("index".into(), Value::Number(f64::from(index)))]);
        if let Some(sentinel) = payload.take() {
            fields.insert("sentinel".into(), Value::Namespace(sentinel));
        }
        let record = app.new_record(fields).unwrap();
        app.set_record_property(&record, "self", Value::Record(record.clone()))
            .unwrap();
        assert_eq!(
            app.call(root, "consume", vec![Value::Record(record)])
                .unwrap(),
            Value::Number(f64::from(index))
        );
    }
    for _ in 0..4096 {
        app.new_record(BTreeMap::new()).unwrap();
    }
    assert!(
        weak.upgrade().is_none(),
        "discarded native record remains a GC root"
    );
    assert_eq!(
        app.call(root, "retained", vec![]).unwrap(),
        Value::Bool(true)
    );
}

#[test]
#[ignore = "requires the WebScene native ABI 3 package"]
fn reflecting_qml_members_returns_their_values_and_qt_descriptor_flags() {
    let mut app = load(
        r#"
        import QtQml
        QtObject {
            id: root
            property int count: 7
            readonly property int fixedCount: 3
            function report(): string {
                const d = Object.getOwnPropertyDescriptor(root, "count")
                const fixed = Object.getOwnPropertyDescriptor(root, "fixedCount")
                const method = Object.getOwnPropertyDescriptor(root, "report")
                return [d.value,d.writable,d.enumerable,d.configurable,
                    fixed.value,fixed.writable,method.value===root.report,
                    Object.getOwnPropertyDescriptor(root, "missing")===undefined].join("/")
            }
        }
    "#,
        engine(Duration::from_secs(4)),
    );
    let root = app.root();
    assert_eq!(
        app.call(root, "report", vec![]).unwrap(),
        Value::String("7/true/true/true/3/true/true/true".into())
    );
    app.set(root, "count", Value::Number(9.)).unwrap();
    assert_eq!(
        app.call(root, "report", vec![]).unwrap(),
        Value::String("9/true/true/true/3/true/true/true".into())
    );
}
