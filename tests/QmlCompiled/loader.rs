use ferroui_qml::{ComponentSource, Instance, TypeDefinition, TypeRegistry, Value};
use ferroui_qml_build::Compiler;
use std::path::Path;

mod compiled {
    include!(concat!(env!("OUT_DIR"), "/loader_model.rs"));
}
#[cfg(feature = "webscene")]
mod compiled_js {
    include!(concat!(env!("OUT_DIR"), "/loader_js.rs"));
}
#[cfg(feature = "webscene")]
mod js_bindings {
    include!(concat!(env!("OUT_DIR"), "/loader_initial_bindings.rs"));
}

fn registry() -> TypeRegistry {
    let mut types = TypeRegistry::new();
    types
        .register(
            "QtQuick",
            "Item",
            TypeDefinition::default()
                .inherits("QtQml", "QtObject")
                .property("width", "real", Value::Undefined)
                .property("height", "real", Value::Undefined)
                .property("implicitWidth", "real", Value::Number(0.))
                .property("implicitHeight", "real", Value::Number(0.))
                .property("data", "list<QtObject>", Value::List(vec![]))
                .default_property("data")
                .visual_children("data"),
        )
        .unwrap();
    types
        .register("QtQuick", "Loader", TypeDefinition::quick_loader())
        .unwrap();
    types.reexport("QtQuick", "QtQml").unwrap();
    types
}

fn exercise(mut app: Instance) {
    let root = app.root();
    let loader = app.id("slot").unwrap();
    let report = |app: &mut Instance, wanted: &str| {
        assert_eq!(
            app.call(root, "report", vec![]).unwrap(),
            Value::String(wanted.into())
        )
    };
    report(&mut app, "0/0/0/0/0");
    app.call(root, "inlineOn", vec![]).unwrap();
    report(&mut app, "1/40/12/3/1");
    app.call(root, "deactivate", vec![]).unwrap();
    report(&mut app, "0/40/12/0/1");
    app.call(root, "enqueue", vec![Value::Number(12.)]).unwrap();
    report(&mut app, "0/40/12/0/1");
    app.call(root, "activate", vec![]).unwrap();
    report(&mut app, "1/30/20/12/2");
    let Value::Object(first) = app.get(loader, "item").unwrap() else {
        panic!("item")
    };
    assert_eq!(app.get(first, "completed").unwrap(), Value::Number(12.));
    app.call(root, "enqueue", vec![Value::Number(13.)]).unwrap();
    report(&mut app, "1/30/20/13/3");
    assert!(app.type_name(first).is_none());
    app.call(root, "missing", vec![]).unwrap();
    report(&mut app, "3/30/20/0/3");
    app.call(root, "removeItem", vec![]).unwrap();
    report(&mut app, "0/30/20/0/3");
}

#[test]
fn linked_source_and_rust_generation_switch_loader_sources_and_initial_properties() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("loader/Main.qml");
    let linked = Compiler::new().load(path).unwrap();
    assert!(linked.dependencies.iter().any(|p| p.ends_with("Tile.qml")));
    fn make(s: &impl ComponentSource) -> Instance {
        Instance::new(s, &registry(), Default::default()).unwrap()
    }
    exercise(make(&linked.document));
    exercise(make(&compiled::compiled_qml()));
}

#[cfg(feature = "webscene")]
#[test]
fn v8_loaders_use_the_same_lifecycle_and_keep_initial_binding_closures_live() {
    use ferroui_qml_webscene::WebSceneEngine;
    use std::rc::Rc;
    fn make(s: &impl ComponentSource) -> Instance {
        Instance::with_engine(
            s,
            &registry(),
            Default::default(),
            Rc::new(WebSceneEngine::from_env().unwrap()),
        )
        .unwrap()
    }
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("loader");
    let compiler = Compiler::new().with_javascript();
    let linked = compiler.load(dir.join("Main.qml")).unwrap();
    exercise(make(&linked.document));
    exercise(make(&compiled_js::compiled_qml()));
    let links = compiler.load(dir.join("Bindings.qml")).unwrap();
    for mut app in [make(&links.document), make(&js_bindings::compiled_qml())] {
        let root = app.root();
        app.call(root, "bind", vec![]).unwrap();
        let Value::Object(item) = app.get(app.id("slot").unwrap(), "item").unwrap() else {
            panic!("loaded item")
        };
        assert_eq!(app.get(item, "completed").unwrap(), Value::Number(14.));
        app.set(root, "seed", Value::Number(11.)).unwrap();
        assert_eq!(app.get(item, "amount").unwrap(), Value::Number(22.));
    }
}
