#![cfg(feature = "webscene")]
use ferroui_qml::{
    ir,
    network::{ComponentReply, ComponentTransport, ResourceError, Url},
    Instance, ObjectId, TypeRegistry, Value,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
#[derive(Default)]
struct Files(RefCell<Vec<String>>, Cell<bool>);
impl ComponentTransport for Files {
    fn request(&self, url: &Url, reply: ComponentReply) {
        self.0.borrow_mut().push(url.to_string());
        let data = match url.path() {
            "/scripts/Remote.qml" => {
                let extended=include_str!("network/scripts/Remote.qml").replace("    function classicOrigin",
                    "    function dynamicValue() { return import(\"./parts/leaf.mjs\").then(m => m.adjust(1)) }\n    function computedValue(name) { return import(\"./parts/\" + name + \".mjs\").then(m => m.adjust(2)) }\n    function moduleOrigin() { return Store.origin() }\n    function classicOrigin");
                reply.complete(Ok(extended.into_bytes()));
                return;
            }
            "/scripts/parts/logic.js" => include_str!("network/scripts/parts/logic.js"),
            "/scripts/parts/math.js" => include_str!("network/scripts/parts/math.js"),
            "/scripts/parts/store.mjs" => concat!(
                include_str!("network/scripts/parts/store.mjs"),
                "\nexport function origin() { return import.meta.url }"
            ),
            "/scripts/parts/leaf.mjs" => include_str!("network/scripts/parts/leaf.mjs"),
            "/Bad.qml" => {
                "import QtQml\nimport \"bad.mjs\" as Bad\nQtObject {property int value:Bad.number}"
            }
            "/bad.mjs" if self.1.get() => "export const number=42;",
            "/bad.mjs" => "throw new Error('remote initialization failed'); export const number=2;",
            "/Cross.qml" => "import QtQml\nimport \"cross.js\" as Bad\nQtObject {}",
            "/cross.js" => {
                ".import \"https://other.test/spoof.js\" as Foreign\nfunction f(){return 0}"
            }
            _ => {
                reply.complete(Err(ResourceError::NotFound));
                return;
            }
        };
        reply.complete(Ok(data.as_bytes().to_vec()));
    }
}
fn load(app: &mut Instance, path: &str) -> ObjectId {
    let factory = app
        .create_component(app.root(), &format!("https://app.test/{path}"), None)
        .unwrap();
    app.poll_component_loads(128).unwrap();
    factory
}
#[test]
fn fetched_modules_extend_a_live_realm_and_keep_their_source_urls() {
    let files = Rc::new(Files::default());
    let mut types = TypeRegistry::new();
    types.set_component_transport(files.clone());
    let doc=ir::parse_external("import QtQml\nQtObject {property var keeper: (()=>{let n=0;return ()=>++n})() ; function next(){return keeper()} }").unwrap();
    let mut app = Instance::with_engine(
        &doc,
        &types,
        Default::default(),
        Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap()),
    )
    .unwrap();
    let root = app.root();
    assert_eq!(app.call(root, "next", vec![]).unwrap(), Value::Number(1.));
    let factory = load(&mut app, "scripts/Remote.qml");
    assert_eq!(
        app.get(factory, "status").unwrap(),
        Value::Number(1.),
        "{:?}",
        app.call(factory, "errorString", vec![])
    );
    let a = app
        .create_object(factory, Some(root), Default::default())
        .unwrap();
    let b = app
        .create_object(factory, Some(root), Default::default())
        .unwrap();
    for (object, text) in [(a, "5/41"), (a, "6/42"), (b, "5/43")] {
        assert_eq!(
            app.call(object, "compute", vec![]).unwrap(),
            Value::String(text.into())
        );
    }
    assert_eq!(
        app.call(a, "classicOrigin", vec![]).unwrap(),
        Value::String("https://app.test/scripts/parts/relative.qml".into())
    );
    assert_eq!(
        app.call(a, "moduleOrigin", vec![]).unwrap(),
        Value::String("https://app.test/scripts/parts/store.mjs".into())
    );
    assert_eq!(
        app.call_and_await(a, "dynamicValue", vec![], Duration::from_secs(1))
            .unwrap(),
        Value::Number(41.)
    );
    assert_eq!(
        files
            .0
            .borrow()
            .iter()
            .filter(|u| u.ends_with("/leaf.mjs"))
            .count(),
        1
    );
    assert_eq!(
        app.call_and_await(
            a,
            "computedValue",
            vec![Value::String("leaf".into())],
            Duration::from_secs(1)
        )
        .unwrap(),
        Value::Number(42.)
    );
    for (path, cause) in [
        ("Bad.qml", "remote initialization failed"),
        ("Cross.qml", "disallowed script dependency"),
    ] {
        let bad = load(&mut app, path);
        assert_eq!(app.get(bad, "status").unwrap(), Value::Number(3.));
        assert!(
            matches!(app.call(bad,"errorString",vec![]).unwrap(),Value::String(s) if s.contains(cause))
        );
    }
    files.1.set(true);
    let retry = load(&mut app, "Bad.qml");
    assert_eq!(
        app.get(retry, "status").unwrap(),
        Value::Number(1.),
        "{:?}",
        app.call(retry, "errorString", vec![])
    );
    let fixed = app.create_object(retry, None, Default::default()).unwrap();
    assert_eq!(app.get(fixed, "value").unwrap(), Value::Number(42.));
    assert!(files
        .0
        .borrow()
        .iter()
        .all(|u| u.starts_with("https://app.test/")));
    assert_eq!(app.call(root, "next", vec![]).unwrap(), Value::Number(2.));
    assert_eq!(
        app.call(a, "compute", vec![]).unwrap(),
        Value::String("7/44".into())
    );
}
