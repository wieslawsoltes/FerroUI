use ferroui_qml::{Instance, ObjectId, TypeRegistry, Value};
use std::{collections::BTreeMap, path::Path};

mod native {
    include!(concat!(env!("OUT_DIR"), "/dynamic_component.rs"));
}
#[cfg(feature = "webscene")]
mod guest {
    include!(concat!(env!("OUT_DIR"), "/dynamic_guest.rs"));
}
fn id(value: Value) -> ObjectId {
    let Value::Object(id) = value else {
        panic!("expected object")
    };
    id
}
fn exercise(mut app: Instance) {
    let root = app.root();
    let factory = id(app.get(root, "factory").unwrap());
    assert!(app.is_component(factory));
    assert!(app.children(factory).is_empty());
    let count = app.objects().count();
    assert!(app
        .create_object(factory, Some(root), BTreeMap::new())
        .is_err());
    assert_eq!(app.objects().count(), count);
    let a = id(app.call(root, "make", vec![Value::Number(4.)]).unwrap());
    let b = app
        .create_object(
            factory,
            None,
            BTreeMap::from([("seed".into(), Value::Number(7.))]),
        )
        .unwrap();
    assert_eq!(app.parent(a), Some(root));
    assert!(app.parent(b).is_none());
    assert_eq!(app.get(a, "score").unwrap(), Value::Number(9.));
    assert_eq!(app.get(b, "score").unwrap(), Value::Number(12.));
    app.set(a, "exposed", Value::Number(5.)).unwrap();
    app.set(root, "offset", Value::Number(10.)).unwrap();
    assert_eq!(app.get(a, "score").unwrap(), Value::Number(19.));
    assert_eq!(app.get(b, "score").unwrap(), Value::Number(20.));
    assert_eq!(app.get(root, "completions").unwrap(), Value::Number(2.));
    assert!(app.id("internal").is_none());
    assert_eq!(app.drain_created(), vec![a, b]);
    assert_eq!(app.structure_revision(), 2);
    app.call(root, "hold", vec![Value::Object(a)]).unwrap();
    app.call(
        root,
        "removeLater",
        vec![Value::Object(a), Value::Number(15.)],
    )
    .unwrap();
    app.advance_time(std::time::Duration::from_millis(14))
        .unwrap();
    assert!(app.type_name(a).is_some());
    let reject = std::rc::Rc::new(std::cell::Cell::new(true));
    let gate = reject.clone();
    app.set_validator(move |model, _| {
        if gate.get() && model.type_name(a).is_none() {
            Err(ferroui_qml::Error::new("reject removal"))
        } else {
            Ok(())
        }
    });
    assert!(app
        .advance_time(std::time::Duration::from_millis(1))
        .is_err());
    assert_eq!(app.get(root, "destructions").unwrap(), Value::Number(0.));
    assert_eq!(app.get(root, "retained").unwrap(), Value::Object(a));
    reject.set(false);
    app.advance_time(std::time::Duration::from_millis(1))
        .unwrap();
    assert!(app.type_name(a).is_none());
    assert!(!app.children(root).contains(&a));
    assert_eq!(app.get(root, "retained").unwrap(), Value::Null);
    assert_eq!(
        app.get(root, "saved").unwrap(),
        Value::List(vec![Value::Null])
    );
    assert_eq!(app.get(root, "destructions").unwrap(), Value::Number(1.));
    assert!(app.set(a, "seed", Value::Number(0.)).is_err());
    app.set(root, "masked", Value::Object(b)).unwrap();
    app.set(root, "overlayActive", Value::Bool(true)).unwrap();
    assert_eq!(app.get(root, "masked").unwrap(), Value::Null);
    app.destroy_object(b).unwrap();
    app.set(root, "overlayActive", Value::Bool(false)).unwrap();
    assert_eq!(
        app.get(root, "masked").unwrap(),
        Value::Null,
        "restore must not resurrect a retired receiver"
    );
    assert!(app.destroy_object(root).is_err());
    let replacement = id(app.call(root, "make", vec![Value::Number(11.)]).unwrap());
    assert_ne!(replacement, b);
    assert!(app.get(b, "seed").is_err());
}
#[test]
fn source_and_rust_share_the_dynamic_component_graph() {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("creation/Main.qml");
    let reg = TypeRegistry::new();
    exercise(
        Instance::new(
            &ferroui_qml::ir::parse(&std::fs::read_to_string(file).unwrap()).unwrap(),
            &reg,
            BTreeMap::new(),
        )
        .unwrap(),
    );
    exercise(Instance::new(&native::compiled_qml(), &reg, BTreeMap::new()).unwrap());
}
#[cfg(feature = "webscene")]
#[test]
fn compiled_and_source_v8_allocate_live_objects() {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("creation/Main.qml");
    let reg = TypeRegistry::new();
    let engine = std::rc::Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap());
    exercise(
        Instance::with_engine(
            &ferroui_qml_build::Compiler::new()
                .with_javascript()
                .load(file)
                .unwrap()
                .document,
            &reg,
            BTreeMap::new(),
            engine.clone(),
        )
        .unwrap(),
    );
    exercise(Instance::with_engine(&guest::compiled_qml(), &reg, BTreeMap::new(), engine).unwrap());
}

#[cfg(feature = "webscene")]
#[test]
fn initial_bindings_run_before_completion() {
    mod compiled {
        include!(concat!(env!("OUT_DIR"), "/component_properties.rs"));
    }
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("creation/Initial.qml");
    let reg = TypeRegistry::new();
    let engine = std::rc::Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap());
    let file = ferroui_qml_build::Compiler::new()
        .with_javascript()
        .load(file)
        .unwrap();
    let source =
        Instance::with_engine(&file.document, &reg, Default::default(), engine.clone()).unwrap();
    let built =
        Instance::with_engine(&compiled::compiled_qml(), &reg, Default::default(), engine).unwrap();
    for mut app in [source, built] {
        assert_eq!(
            app.call(app.root(), "probe", vec![]).unwrap(),
            Value::String("7/15/false/true/5,9".into())
        );
    }
}

mod catalog_native {
    include!(concat!(env!("OUT_DIR"), "/catalog_native.rs"));
}
#[cfg(feature = "webscene")]
mod catalog_guest {
    include!(concat!(env!("OUT_DIR"), "/catalog_guest.rs"));
}
fn catalog(mut vm: Instance) {
    let root = vm.root();
    assert_eq!(vm.call(root, "make", vec![]).unwrap(), Value::Number(11.));
    assert_eq!(
        vm.call(root, "makeType", vec![]).unwrap(),
        Value::String("named".into())
    );
    assert_eq!(
        vm.call(root, "makeViaString", vec![]).unwrap(),
        Value::Number(13.)
    );
    let missing = id(vm.call(root, "missing", vec![]).unwrap());
    assert_eq!(vm.get(missing, "status").unwrap(), Value::Number(3.));
    assert!(
        matches!(vm.call(missing,"errorString",vec![]).unwrap(),Value::String(s) if s.contains("unknown.qml"))
    );
    let loading = id(vm.call(root, "startAsync", vec![]).unwrap());
    assert_eq!(vm.get(loading, "status").unwrap(), Value::Number(2.));
    vm.advance_time(std::time::Duration::ZERO).unwrap();
    assert_eq!(vm.get(loading, "status").unwrap(), Value::Number(1.));
}
#[test]
fn cargo_file_factories_are_linked_and_relocatable() {
    catalog(
        Instance::new(
            &catalog_native::compiled_qml(),
            &TypeRegistry::new(),
            Default::default(),
        )
        .unwrap(),
    );
}
#[cfg(feature = "webscene")]
#[test]
fn linked_file_and_string_factories_in_both_javascript_profiles() {
    let reg = TypeRegistry::new();
    let engine = std::rc::Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap());
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("creation/catalog/Main.qml");
    let doc = ferroui_qml_build::Compiler::new()
        .with_javascript()
        .qml_entry("Part.qml")
        .load(path)
        .unwrap()
        .document;
    catalog(Instance::with_engine(&doc, &reg, Default::default(), engine.clone()).unwrap());
    catalog(
        Instance::with_engine(
            &catalog_guest::compiled_qml(),
            &reg,
            Default::default(),
            engine,
        )
        .unwrap(),
    );
}

#[cfg(feature = "webscene")]
#[test]
fn factory_origins_survive_closures_and_script_imports() {
    mod compiled {
        include!(concat!(env!("OUT_DIR"), "/catalog_origin.rs"));
    }
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("creation/origin/Main.qml");
    let engine = std::rc::Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap());
    let reg = TypeRegistry::new();
    let loaded = ferroui_qml_build::Compiler::new()
        .with_javascript()
        .qml_entry("Part.qml")
        .qml_entry("sub/Part.qml")
        .qml_entry("sub/scripts/Part.qml")
        .load(file)
        .unwrap();
    let source =
        Instance::with_engine(&loaded.document, &reg, Default::default(), engine.clone()).unwrap();
    let built =
        Instance::with_engine(&compiled::compiled_qml(), &reg, Default::default(), engine).unwrap();
    for mut app in [source, built] {
        let root = app.root();
        for (name, relative) in [
            ("capturedUrl", "captured.svg"),
            ("classicUrl", "sub/scripts/classic.svg"),
            ("esmUrl", "module.svg"),
        ] {
            let expected = format!("https://qml.invalid/components/project/{relative}");
            assert_eq!(
                app.call(root, name, vec![]).unwrap(),
                Value::String(expected),
                "{name}"
            );
        }
        app.call(root, "capture", vec![]).unwrap();
        assert_eq!(
            app.call(root, "run", vec![]).unwrap(),
            Value::String("73/84/95".into())
        );
    }
}
#[test]
fn deferred_type_contracts_are_checked_without_running_javascript() {
    let reg = TypeRegistry::new();
    for (body,valid) in [
        ("QtObject { required property string supplied; Component.onCompleted: unknownAtRuntime() }",true),
        ("QtObject { nonExistent: 42 }",false),
        ("QtObject { property QtObject nested: QtObject { required property int missing } }",false),
        ("QtObject { property Component deeper: Component { QtObject { bad: 0 } } }",false),
    ]{
        let qml=format!("import QtQml; QtObject {{ property Component deferred: Component {{ {body} }} }}");
        let doc=ferroui_qml::ir::parse(&qml).unwrap();assert_eq!(reg.check(&doc).is_ok(),valid,"{qml}");
    }
}

fn check_url_utilities(mut app: Instance) {
    const BASE: &str = "https://qml.invalid/components/project/";
    for (name, expected) in [
        ("local", format!("{BASE}logo.svg?mode=dark#mark")),
        ("relative", "../icons/logo.svg".into()),
        ("absolute", "qrc:/app/icons/logo.svg".into()),
        ("empty", String::new()),
        ("display", format!("{BASE}icons/large café%2Fdark.svg")),
        ("definition", format!("{BASE}sub/reader.svg")),
        ("site", format!("{BASE}site.svg")),
        ("objectContext", format!("{BASE}parent.svg")),
        ("engineBase", format!("{BASE}fallback.svg")),
    ] {
        assert_eq!(
            app.call(app.root(), name, vec![]).unwrap(),
            Value::String(expected),
            "{name}"
        );
    }
}

#[test]
fn url_utilities_preserve_component_and_instantiation_site_origins() {
    mod generated {
        include!(concat!(env!("OUT_DIR"), "/url_utilities.rs"));
    }
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("creation/urls/Main.qml");
    let doc = ferroui_qml_build::Compiler::new()
        .load(source)
        .unwrap()
        .document;
    let reg = TypeRegistry::new();
    check_url_utilities(Instance::new(&doc, &reg, Default::default()).unwrap());
    check_url_utilities(
        Instance::new(&generated::compiled_qml(), &reg, Default::default()).unwrap(),
    );
}

#[cfg(feature = "webscene")]
#[test]
fn guest_url_utilities_preserve_component_and_instantiation_site_origins() {
    mod generated {
        include!(concat!(env!("OUT_DIR"), "/url_utilities_guest.rs"));
    }
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("creation/urls/Main.qml");
    let doc = ferroui_qml_build::Compiler::new()
        .with_javascript()
        .load(source)
        .unwrap()
        .document;
    let engine = std::rc::Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap());
    let reg = TypeRegistry::new();
    check_url_utilities(
        Instance::with_engine(&doc, &reg, Default::default(), engine.clone()).unwrap(),
    );
    check_url_utilities(
        Instance::with_engine(&generated::compiled_qml(), &reg, Default::default(), engine)
            .unwrap(),
    );
}
