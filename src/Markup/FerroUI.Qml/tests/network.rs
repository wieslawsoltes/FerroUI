use ferroui_qml::{
    ir::parse,
    network::{ComponentReply, ComponentTransport, ResourceError},
    Instance, ObjectId, TypeDefinition, TypeRegistry, Value,
};
use std::{cell::RefCell, collections::VecDeque, rc::Rc};
use url::Url;

#[derive(Default)]
struct Manual(RefCell<VecDeque<(Url, ComponentReply)>>);
impl ComponentTransport for Manual {
    fn request(&self, url: &Url, reply: ComponentReply) {
        self.0.borrow_mut().push_back((url.clone(), reply));
    }
}
impl Manual {
    fn take(&self, url: &str) -> ComponentReply {
        let (actual, reply) = self.0.borrow_mut().pop_front().expect("network request");
        assert_eq!(actual.as_str(), url);
        reply
    }
}
fn app(transport: Rc<dyn ComponentTransport>, source: &str) -> Instance {
    let mut registry = TypeRegistry::new();
    registry.set_component_transport(transport);
    registry
        .register(
            "QtQuick",
            "Item",
            TypeDefinition::default()
                .inherits("QtQml", "QtObject")
                .property("data", "list<QtObject>", Value::List(Vec::new()))
                .default_property("data")
                .visual_children("data")
                .property("width", "real", Value::Number(0.))
                .property("height", "real", Value::Number(0.)),
        )
        .unwrap();
    registry
        .register("QtQuick", "Loader", TypeDefinition::quick_loader())
        .unwrap();
    registry.reexport("QtQuick", "QtQml").unwrap();
    Instance::new(&parse(source).unwrap(), &registry, Default::default()).unwrap()
}
fn status(model: &Instance, id: ObjectId) -> Value {
    model.get(id, "status").unwrap()
}

#[test]
fn remote_dependencies_link_relative_to_their_source_and_initial_values_precede_completion() {
    let transport = Rc::new(Manual::default());
    let mut model = app(
        transport.clone(),
        r#"import QtQuick
        Item { id: page; Loader { id: slot }
            function start() { slot.setSource("https://widgets.test/cards/Card.qml", {amount: 41}) }
        }"#,
    );
    let slot = model.id("slot").unwrap();
    model.call(model.root(), "start", vec![]).unwrap();
    assert_eq!(status(&model, slot), Value::Number(2.));
    transport.take("https://widgets.test/cards/Card.qml").complete(Ok(br#"import QtQuick
        import "parts" as Parts
        Item { required property int amount; property int answer: part.extra + amount; Parts.Badge { id: part } }
    "#.to_vec()));
    assert_eq!(model.poll_component_loads(1).unwrap(), 1);
    assert_eq!(model.get(slot, "item").unwrap(), Value::Null);
    transport
        .take("https://widgets.test/cards/parts/qmldir")
        .complete(Err(ResourceError::NotFound));
    model.poll_component_loads(1).unwrap();
    transport
        .take("https://widgets.test/cards/parts/Badge.qml")
        .complete(Ok(
            b"import QtQuick\nItem { property int extra: 1 }".to_vec()
        ));
    assert_eq!(model.poll_component_loads(2).unwrap(), 1);
    assert_eq!(status(&model, slot), Value::Number(1.));
    let Value::Object(item) = model.get(slot, "item").unwrap() else {
        panic!("no loaded item")
    };
    assert_eq!(model.get(item, "answer").unwrap(), Value::Number(42.));
    assert!(!model.has_pending_component_loads());
}

#[test]
fn aborted_transactions_and_retired_factories_cannot_deliver_network_work() {
    let transport = Rc::new(Manual::default());
    let mut model = app(
        transport.clone(),
        r#"import QtQuick
        Item { Loader { id: slot }
            function fail() { Qt.createComponent("https://widgets.test/Uncommitted.qml"); missing() }
        }"#,
    );
    assert!(model.call(model.root(), "fail", vec![]).is_err());
    assert!(transport.0.borrow().is_empty());
    let slot = model.id("slot").unwrap();
    model
        .set(
            slot,
            "source",
            Value::String("https://widgets.test/A.qml".into()),
        )
        .unwrap();
    let stale = transport.take("https://widgets.test/A.qml");
    model
        .set(
            slot,
            "source",
            Value::String("https://widgets.test/B.qml".into()),
        )
        .unwrap();
    assert!(!stale.is_pending());
    stale.complete(Ok(b"import QtQuick\nItem {}".to_vec()));
    transport
        .take("https://widgets.test/B.qml")
        .complete(Err(ResourceError::NotFound));
    assert_eq!(model.poll_component_loads(4).unwrap(), 1);
    assert_eq!(status(&model, slot), Value::Number(3.));
    let Value::Object(factory) = model.get(slot, "sourceComponent").unwrap() else {
        panic!("no factory")
    };
    assert!(
        matches!(model.call(factory,"errorString",vec![]).unwrap(),Value::String(s) if s.contains("B.qml") && s.contains("not found"))
    );
}

#[test]
fn dependency_cycles_and_transport_abandonment_are_terminal_errors() {
    let transport = Rc::new(Manual::default());
    let mut model = app(transport.clone(), "import QtQml\nQtObject {}");
    let factory = model
        .create_component(model.root(), "https://widgets.test/A.qml", None)
        .unwrap();
    transport
        .take("https://widgets.test/A.qml")
        .complete(Ok(b"import QtQml\nB {}".to_vec()));
    model.poll_component_loads(1).unwrap();
    transport
        .take("https://widgets.test/qmldir")
        .complete(Err(ResourceError::NotFound));
    model.poll_component_loads(1).unwrap();
    transport
        .take("https://widgets.test/B.qml")
        .complete(Ok(b"import QtQml\nA {}".to_vec()));
    model.poll_component_loads(1).unwrap();
    assert_eq!(status(&model, factory), Value::Number(3.));
    assert!(
        matches!(model.call(factory,"errorString",vec![]).unwrap(),Value::String(s) if s.contains("recurs") || s.contains("cycl"))
    );
    let factory = model
        .create_component(model.root(), "https://widgets.test/B.qml", None)
        .unwrap();
    drop(transport.take("https://widgets.test/B.qml"));
    model.poll_component_loads(1).unwrap();
    assert_eq!(status(&model, factory), Value::Number(3.));
}

#[cfg(all(feature = "http", not(target_arch = "wasm32")))]
#[test]
fn native_http_transport_loads_over_loopback_and_refuses_unlisted_origins() {
    use ferroui_qml::network::HttpComponentTransport;
    use std::{
        io::{BufRead, BufReader, Write},
        net::TcpListener,
        sync::{mpsc, Arc},
        time::Duration,
    };
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let origin = Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
    let serve = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut lines = BufReader::new(&stream).lines();
        assert_eq!(lines.next().unwrap().unwrap(), "GET /Card.qml HTTP/1.1");
        for line in lines {
            if line.unwrap().is_empty() {
                break;
            }
        }
        let body = "import QtQml\nQtObject { property int answer: 42 }";
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
    });
    let mut model = app(
        Rc::new(HttpComponentTransport::new(std::slice::from_ref(&origin)).unwrap()),
        "import QtQml\nQtObject {}",
    );
    let (wake, rx) = mpsc::channel();
    model.set_component_waker(Some(Arc::new(move || {
        let _ = wake.send(());
    })));
    let factory = model
        .create_component(
            model.root(),
            origin.join("Card.qml").unwrap().as_str(),
            None,
        )
        .unwrap();
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    model.poll_component_loads(1).unwrap();
    assert_eq!(status(&model, factory), Value::Number(1.));
    serve.join().unwrap();
    let factory = model
        .create_component(model.root(), "https://other.test/Card.qml", None)
        .unwrap();
    model.poll_component_loads(1).unwrap();
    assert_eq!(status(&model, factory), Value::Number(3.));
}

#[test]
fn an_identified_remote_import_resolves_versioned_exports_and_rejects_missing_modules() {
    let transport = Rc::new(Manual::default());
    let mut registry = TypeRegistry::new();
    registry.set_component_transport(transport.clone());
    registry
        .add_component_import_path(Url::parse("https://widgets.test/modules").unwrap())
        .unwrap();
    let doc = parse("import QtQml\nQtObject {}").unwrap();
    let mut instance = Instance::new(&doc, &registry, Default::default()).unwrap();
    let factory = instance
        .create_component(instance.root(), "https://widgets.test/Card.qml", None)
        .unwrap();
    transport.take("https://widgets.test/Card.qml").complete(Ok(
        b"import QtQml\nimport Lab.Widgets 1.1 as L\nL.Card {}".to_vec(),
    ));
    instance.poll_component_loads(1).unwrap();
    transport
        .take("https://widgets.test/modules/Lab/Widgets/qmldir")
        .complete(Ok(
            b"module Lab.Widgets\nCard 1.0 First.qml\nCard 1.1 Current.qml\n".to_vec(),
        ));
    instance.poll_component_loads(1).unwrap();
    transport
        .take("https://widgets.test/modules/Lab/Widgets/Current.qml")
        .complete(Ok(
            b"import QtQml\nQtObject { property int generation: 2 }".to_vec()
        ));
    instance.poll_component_loads(1).unwrap();
    let item = instance
        .create_object(factory, None, Default::default())
        .unwrap();
    assert_eq!(instance.get(item, "generation").unwrap(), Value::Number(2.));
    let factory = instance
        .create_component(instance.root(), "https://widgets.test/Bad.qml", None)
        .unwrap();
    transport
        .take("https://widgets.test/Bad.qml")
        .complete(Ok(b"import QtQml\nimport Not.There\nQtObject {}".to_vec()));
    instance.poll_component_loads(1).unwrap();
    transport
        .take("https://widgets.test/modules/Not/There/qmldir")
        .complete(Err(ResourceError::NotFound));
    instance.poll_component_loads(1).unwrap();
    assert_eq!(status(&instance, factory), Value::Number(3.));
}

#[cfg(feature = "javascript")]
#[test]
fn remote_bindings_share_one_import_table_for_the_document() {
    use ferroui_qml::{ir, Error, ScriptContext, ScriptEngine};
    use std::{cell::Cell, collections::BTreeSet, sync::Arc};

    #[derive(Default)]
    struct Observer {
        reads: Cell<usize>,
        tables: RefCell<BTreeSet<usize>>,
    }
    impl ScriptEngine for Observer {
        fn evaluate(
            &self,
            _: &mut ScriptContext<'_, '_>,
            script: &ir::JavaScript,
        ) -> Result<Value, Error> {
            assert_eq!(script.imports.len(), 72);
            self.tables
                .borrow_mut()
                .insert(Arc::as_ptr(&script.imports) as usize);
            self.reads.set(self.reads.get() + 1);
            Ok(Value::Number(1.))
        }
        fn prepare_script_resources(&self, _: &mut ScriptContext<'_, '_>) -> Result<(), Error> {
            Ok(())
        }
    }
    let mut source = String::from("import QtQml\n");
    for i in 0..72 {
        source.push_str(&format!("import \"logic.mjs\" as Alias{i}\n"));
    }
    source.push_str("QtObject {\n");
    for i in 0..160 {
        source.push_str(&format!("property int p{i}: Alias0.answer + {i}\n"));
    }
    source.push('}');
    let transport = Rc::new(Manual::default());
    let mut registry = TypeRegistry::new();
    registry.set_component_transport(transport.clone());
    let observer = Rc::new(Observer::default());
    let doc = ir::parse_external("import QtQml\nQtObject {}").unwrap();
    let mut model =
        Instance::with_engine(&doc, &registry, Default::default(), observer.clone()).unwrap();
    let prefix = format!("https://widgets.test/{}/", "a".repeat(450));
    let entry = format!("{prefix}Card.qml");
    let factory = model.create_component(model.root(), &entry, None).unwrap();
    transport.take(&entry).complete(Ok(source.into_bytes()));
    model.poll_component_loads(1).unwrap();
    transport
        .take(&format!("{prefix}logic.mjs"))
        .complete(Ok(b"export const answer=1".to_vec()));
    model.poll_component_loads(1).unwrap();
    model
        .create_object(factory, None, Default::default())
        .unwrap();
    assert_eq!(observer.reads.get(), 160);
    assert_eq!(observer.tables.borrow().len(), 1);
}
