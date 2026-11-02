use ferroui_qml::{ComponentSource, Engine, TypeRegistry, Value};

mod first {
    include!(concat!(env!("OUT_DIR"), "/singletons.rs"));
}
mod second {
    include!(concat!(env!("OUT_DIR"), "/independent_root.rs"));
}
#[cfg(feature = "webscene")]
mod guest {
    include!(concat!(env!("OUT_DIR"), "/independent_guest.rs"));
}

fn lifecycle(engine: &mut Engine, other: &impl ComponentSource) {
    let initial = Default::default();
    let a = engine.load_root(&first::compiled_qml(), initial).unwrap();
    let state = engine.instance().get(a, "state").unwrap();
    let b = engine.load_root(other, Default::default()).unwrap();
    let c = engine
        .load_root(other, [("base".into(), Value::Number(42.))].into())
        .unwrap();
    assert_eq!(engine.id(b, "root"), Some(b));
    assert_eq!(engine.id(c, "root"), Some(c));
    assert_ne!(engine.id(b, "inner"), engine.id(c, "inner"));
    assert_eq!(engine.id(a, "inner"), None);
    assert_eq!(engine.roots().count(), 3);
    assert_eq!(engine.instance().get(b, "state").unwrap(), state);
    assert_eq!(
        engine.instance().get(b, "observed").unwrap(),
        Value::Number(7.)
    );
    assert_eq!(
        engine.instance().get(c, "observed").unwrap(),
        Value::Number(44.)
    );
    let Value::Object(singleton) = state else {
        panic!("singleton")
    };
    assert!(engine.instance_mut().destroy_object(singleton).is_err());
    engine.instance_mut().call(b, "tick", vec![]).unwrap();
    assert_eq!(
        engine.instance().get(a, "total").unwrap(),
        Value::Number(9.)
    );
    let consumer = engine.instance().get(a, "first").unwrap();
    engine.instance_mut().set(b, "consumer", consumer).unwrap();
    assert!(engine
        .instance_mut()
        .set(b, "consumer", Value::Object(c))
        .is_err());
    let count = engine.instance().objects().count();
    assert!(engine
        .load_root(other, [("failOnLoad".into(), Value::Bool(true))].into())
        .is_err());
    assert_eq!(engine.roots().count(), 3);
    assert_eq!(engine.instance().objects().count(), count);
    assert_eq!(
        engine.instance().get(singleton, "count").unwrap(),
        Value::Number(3.)
    );
    engine
        .instance_mut()
        .set(b, "failOnUnload", Value::Bool(true))
        .unwrap();
    assert!(engine.unload_root(b).is_err());
    assert_eq!(
        engine.instance().get(singleton, "count").unwrap(),
        Value::Number(3.)
    );
    assert_eq!(engine.id(b, "root"), Some(b));
    engine
        .instance_mut()
        .set(b, "failOnUnload", Value::Bool(false))
        .unwrap();
    for id in [a, b, c] {
        engine.unload_root(id).unwrap();
    }
    assert!(engine.instance().get(a, "state").is_err());
    assert_eq!(engine.roots().count(), 0);
    let d = engine.load_root(other, Default::default()).unwrap();
    assert_eq!(engine.instance().get(d, "state").unwrap(), state);
    assert_ne!(b, d);
    assert_eq!(engine.id(b, "root"), None);
    assert!(engine.unload_root(b).is_err());
}

#[test]
fn independently_linked_source_and_aot_share_engine_singletons() {
    let types = TypeRegistry::new();
    let interpreted = ferroui_qml_build::Compiler::new()
        .import_path("singletons/imports")
        .load("singletons/Independent.qml")
        .unwrap();
    lifecycle(&mut Engine::new(&types).unwrap(), &interpreted.document);
    lifecycle(&mut Engine::new(&types).unwrap(), &second::compiled_qml());
}
#[cfg(feature = "webscene")]
#[test]
fn v8_root_shares_singleton_and_lifecycle_with_native_root() {
    let guest = std::rc::Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap());
    let mut engine = Engine::with_script_engine(&TypeRegistry::new(), guest).unwrap();
    lifecycle(&mut engine, &guest::compiled_qml());
}

#[cfg(feature = "webscene")]
#[test]
fn guest_module_cache_is_shared_until_engine_reset() {
    let registry = TypeRegistry::new();
    let vm = std::rc::Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap());
    let mut engine = Engine::with_script_engine(&registry, vm).unwrap();
    let linked = ferroui_qml_build::Compiler::new()
        .with_javascript()
        .import_path("singletons/imports")
        .load("singletons/Scripted.qml")
        .unwrap()
        .document;
    for expected in [3., 4.] {
        let root = engine.load_root(&linked, Default::default()).unwrap();
        assert_eq!(
            engine.instance_mut().call(root, "next", vec![]).unwrap(),
            Value::Number(expected)
        );
        engine.unload_root(root).unwrap();
    }
    let mut conflicting = linked.clone();
    conflicting
        .scripts
        .values_mut()
        .next()
        .unwrap()
        .source
        .push_str("// changed");
    assert!(engine.load_root(&conflicting, Default::default()).is_err());
    assert_eq!(engine.roots().count(), 0);
    engine.reset().unwrap();
    let root = engine.load_root(&linked, Default::default()).unwrap();
    assert_eq!(
        engine.instance_mut().call(root, "next", vec![]).unwrap(),
        Value::Number(3.)
    );
}
