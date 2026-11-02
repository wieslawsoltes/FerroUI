use ferroui_qml::{Instance, TypeRegistry, Value};
use ferroui_qml_build::Compiler;
use std::path::Path;

mod generated {
    include!(concat!(env!("OUT_DIR"), "/incubator.rs"));
}
#[cfg(feature = "webscene")]
mod guest {
    include!(concat!(env!("OUT_DIR"), "/incubator_guest.rs"));
}
fn exercise(mut app: Instance) {
    let root = app.root();
    let mut call = |name: &str| app.call(root, name, vec![]).unwrap();
    assert_eq!(call("begin"), Value::Number(2.));
    assert_eq!(call("force"), Value::Number(1.));
    assert_eq!(call("force"), Value::Number(1.));
    assert_eq!(call("synchronous"), Value::Number(9.));
    assert_eq!(call("invalid"), Value::Number(3.));
    assert_eq!(app.get(root, "called").unwrap(), Value::Number(2.));
    assert_eq!(app.get(root, "last").unwrap(), Value::Number(3.));
    assert!(!app.has_deferred_work());
}

#[test]
fn source_and_generated_incubator_status_and_callbacks_match_qt() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("creation/Incubator.qml");
    let source = Compiler::new().load(path).unwrap();
    let registry = TypeRegistry::new();
    exercise(Instance::new(&source.document, &registry, Default::default()).unwrap());
    exercise(Instance::new(&generated::compiled_qml(), &registry, Default::default()).unwrap());
}

#[cfg(feature = "webscene")]
#[test]
fn v8_incubator_calls_closures_and_preserves_initial_bindings() {
    use ferroui_qml_webscene::WebSceneEngine;
    use std::rc::Rc;
    let registry = TypeRegistry::new();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("creation/IncubatorGuest.qml");
    let source = Compiler::new().with_javascript().load(path).unwrap();
    fn check(source: &impl ferroui_qml::ComponentSource, registry: &TypeRegistry) {
        let mut app = Instance::with_engine(
            source,
            registry,
            Default::default(),
            Rc::new(WebSceneEngine::from_env().unwrap()),
        )
        .unwrap();
        assert_eq!(
            app.call(app.root(), "run", vec![]).unwrap(),
            Value::String("true|true/1/10|16|1".into())
        );
    }
    check(&source.document, &registry);
    check(&guest::compiled_qml(), &registry);
}
