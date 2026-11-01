use ferroui_qml::{
    ir,
    network::{ComponentReply, ComponentTransport, Url},
    Instance, TypeRegistry, Value,
};
use std::rc::Rc;

struct Fixtures;
impl ComponentTransport for Fixtures {
    fn request(&self, url: &Url, reply: ComponentReply) {
        let text = match url.path() {
            "/Card.qml" => include_str!("network/Card.qml"),
            "/parts/qmldir" => include_str!("network/parts/qmldir"),
            "/parts/ValueBadge.qml" => include_str!("network/parts/ValueBadge.qml"),
            other => panic!("unexpected request {other}"),
        };
        reply.complete(Ok(text.as_bytes().to_vec()));
    }
}

fn exercise(mut instance: Instance) {
    let root = instance.root();
    let factory = instance
        .create_component(root, "https://test.example/Card.qml", Some(root))
        .unwrap();
    assert_eq!(instance.get(factory, "status").unwrap(), Value::Number(2.));
    assert_eq!(instance.poll_component_loads(10).unwrap(), 3);
    assert_eq!(instance.get(factory, "status").unwrap(), Value::Number(1.));
    let item = instance
        .create_object(
            factory,
            Some(root),
            std::collections::BTreeMap::from([("amount".into(), Value::Number(41.))]),
        )
        .unwrap();
    assert_eq!(
        instance.call(item, "format", vec![]).unwrap(),
        Value::String("42 points".into())
    );
}
#[test]
fn network_fixture_uses_the_same_typed_component_lifecycle_as_a_local_file() {
    let mut types = TypeRegistry::new();
    types.set_component_transport(Rc::new(Fixtures));
    let doc = ir::parse("import QtQml\nQtObject {}").unwrap();
    exercise(Instance::new(&doc, &types, Default::default()).unwrap());
}
#[cfg(feature = "webscene")]
#[test]
fn webscene_executes_remote_component_bindings_and_methods() {
    let mut types = TypeRegistry::new();
    types.set_component_transport(Rc::new(Fixtures));
    let doc = ir::parse_external("import QtQml\nQtObject {}").unwrap();
    exercise(
        Instance::with_engine(
            &doc,
            &types,
            Default::default(),
            Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap()),
        )
        .unwrap(),
    );
}
