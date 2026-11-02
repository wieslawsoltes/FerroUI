//! The frontend, Rust code generator and V8 all resolve the same Rust receiver.
use ferroui_qml::{
    ComponentSource, Error, Instance, Parameter, TypeDefinition, TypeRegistry, Value,
};
use std::{cell::Cell, rc::Rc};

mod compiled {
    include!(concat!(env!("OUT_DIR"), "/native_host.rs"));
}
#[cfg(feature = "webscene")]
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/webscene_host.rs"));
}

fn registry(calls: Rc<Cell<usize>>) -> TypeRegistry {
    let mut registry = TypeRegistry::new();
    let argument = Parameter {
        name: "value".into(),
        ty: Some("int".into()),
    };
    registry
        .register(
            "Host",
            "Meter",
            TypeDefinition::default()
                .inherits("QtQml", "QtObject")
                .property("input", "int", Value::Number(0.))
                .signal("committed", vec![])
                .context_method("percent", vec![], "int", move |ctx, _| {
                    calls.set(calls.get() + 1);
                    let Value::Number(n) = ctx.get(ctx.owner(), "input")? else {
                        unreachable!()
                    };
                    Ok(Value::Number(100. * n))
                })
                .context_method("commitValue", vec![argument.clone()], "var", |ctx, args| {
                    ctx.set(ctx.owner(), "input", args[0].clone())?;
                    ctx.call(ctx.owner(), "committed", vec![])
                })
                .context_method("abortWrite", vec![argument], "var", |ctx, args| {
                    ctx.set(ctx.owner(), "input", args[0].clone())?;
                    Err(Error::new("host validation failed"))
                }),
        )
        .unwrap();
    registry
}

fn check(mut instance: Instance, calls: Rc<Cell<usize>>) {
    let root = instance.root();
    let Value::Object(second) = instance.get(root, "second").unwrap() else {
        panic!("missing second meter");
    };
    assert_eq!(instance.get(root, "selected").unwrap(), Value::Number(200.));
    let initial = calls.get();
    instance.set(second, "input", Value::Number(7.)).unwrap();
    assert_eq!(
        calls.get(),
        initial,
        "a native read must not subscribe to the inactive receiver"
    );
    instance.call(root, "edit", vec![]).unwrap();
    assert_eq!(instance.get(root, "selected").unwrap(), Value::Number(800.));
    assert_eq!(instance.get(root, "edits").unwrap(), Value::Number(1.));
    assert!(instance.call(root, "fail", vec![]).is_err());
    assert_eq!(instance.get(root, "selected").unwrap(), Value::Number(800.));
    instance
        .set(root, "chooseFirst", Value::Bool(false))
        .unwrap();
    assert_eq!(instance.get(root, "selected").unwrap(), Value::Number(700.));
    let Value::Object(first) = instance.get(root, "first").unwrap() else {
        unreachable!()
    };
    instance.call(root, "schedule", vec![]).unwrap();
    let method = instance.method(first, "commitValue").unwrap();
    assert_eq!(
        instance.get(root, "pending").unwrap(),
        Value::Method(method.clone())
    );
    assert_eq!(instance.process_deferred(16).unwrap(), 1);
    assert_eq!(instance.get(first, "input").unwrap(), Value::Number(10.));
    instance
        .set(root, "pending", Value::Method(method))
        .unwrap();
    instance.call(root, "runSaved", vec![]).unwrap();
    assert_eq!(instance.get(first, "input").unwrap(), Value::Number(11.));
}

#[test]
fn linked_and_rust_compiled_scripts_use_transactional_host_methods() {
    let source = ferroui_qml::ir::parse(include_str!("host/Main.qml")).unwrap();
    fn run(source: &impl ComponentSource) {
        let calls = Rc::new(Cell::new(0));
        let app = Instance::new(source, &registry(calls.clone()), Default::default()).unwrap();
        check(app, calls);
    }
    run(&source);
    run(&compiled::compiled_qml());
}

#[cfg(feature = "webscene")]
#[test]
fn source_and_embedded_v8_scripts_use_transactional_host_methods() {
    use std::path::Path;
    let source = ferroui_qml_build::Compiler::new()
        .with_javascript()
        .load(Path::new(env!("CARGO_MANIFEST_DIR")).join("host/Main.qml"))
        .unwrap();
    fn run(source: &impl ComponentSource) {
        let calls = Rc::new(Cell::new(0));
        let engine = Rc::new(ferroui_qml_webscene::WebSceneEngine::from_env().unwrap());
        let app =
            Instance::with_engine(source, &registry(calls.clone()), Default::default(), engine)
                .unwrap();
        check(app, calls);
    }
    run(&source.document);
    run(&embedded::compiled_qml());
}
