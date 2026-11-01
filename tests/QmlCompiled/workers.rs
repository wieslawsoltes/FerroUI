use ferroui_qml::{ComponentSource, Instance, TypeRegistry, Value};
use ferroui_qml_build::Compiler;
use ferroui_qml_webscene::WebSceneEngine;
use std::{
    path::Path,
    rc::Rc,
    time::{Duration, Instant},
};

mod generated {
    include!(concat!(env!("OUT_DIR"), "/workers.rs"));
}
fn until(model: &mut Instance, condition: impl Fn(&Instance) -> bool) {
    let limit = Instant::now() + Duration::from_secs(6);
    while !condition(model) {
        assert!(Instant::now() < limit, "worker did not deliver");
        model.poll_worker_scripts(16).unwrap();
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn construct(source: &impl ComponentSource) -> Instance {
    let engine = Rc::new(
        WebSceneEngine::from_env()
            .unwrap()
            .with_timeout(Duration::from_secs(2)),
    );
    Instance::with_engine(source, &TypeRegistry::new(), Default::default(), engine).unwrap()
}
fn exercise(source: &impl ComponentSource) {
    let mut app = construct(source);
    let root = app.root();
    assert!(app.has_worker_scripts());
    assert!(app.call(root, "abortSend", vec![]).is_err());
    app.call(root, "start", vec![]).unwrap();
    until(&mut app, |app| {
        app.get(root, "answer").unwrap() == Value::String("1:2/3".into())
            && app.get(root, "localValue").unwrap() == Value::Number(3.)
    });
    assert_eq!(app.get(root, "replies").unwrap(), Value::Number(1.));
    app.call(root, "restart", vec![]).unwrap();
    app.call(root, "start", vec![]).unwrap();
    until(&mut app, |app| {
        app.get(root, "replies").unwrap() == Value::Number(2.)
    });
    assert_eq!(
        app.get(root, "answer").unwrap(),
        Value::String("1:2/3".into())
    );
}
#[test]
fn source_and_compiled_workers_copy_data_preserve_fifo_and_replace_their_realm() {
    let doc = Compiler::new()
        .with_javascript()
        .load(Path::new(env!("CARGO_MANIFEST_DIR")).join("workers/Main.qml"))
        .unwrap();
    exercise(&doc.document);
    exercise(&generated::compiled_qml());
}
#[test]
fn worker_replies_stream_before_timeout_and_other_workers_remain_usable() {
    let mut app = construct(&generated::compiled_qml());
    let root = app.root();
    let Value::Object(actor) = app.get(root, "actor").unwrap() else {
        panic!()
    };
    until(&mut app, |app| {
        app.get(actor, "ready").unwrap() == Value::Bool(true)
    });
    app.call(root, "spin", vec![]).unwrap();
    app.call(root, "control", vec![]).unwrap();
    until(&mut app, |app| {
        app.get(root, "answer").unwrap() == Value::String("started".into())
            && app.get(root, "localValue").unwrap() == Value::Number(5.)
    });
    let limit = Instant::now() + Duration::from_secs(8);
    loop {
        if let Err(e) = app.poll_worker_scripts(8) {
            assert!(e.message.contains("deadline"), "{e}");
            break;
        }
        assert!(Instant::now() < limit);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(app.get(actor, "ready").unwrap(), Value::Bool(false));
    app.call(root, "restart", vec![]).unwrap();
    app.call(root, "start", vec![]).unwrap();
    until(&mut app, |app| {
        app.get(root, "answer").unwrap() == Value::String("1:2/3".into())
    });
}
