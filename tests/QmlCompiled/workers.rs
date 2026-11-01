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
    let Value::Object(pane) = app.get(root, "pane").unwrap() else {
        panic!()
    };
    app.call(pane, "start", vec![]).unwrap();
    until(&mut app, |app| {
        app.get(pane, "result").unwrap() == Value::Number(21.)
    });
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
    app.call(
        root,
        "selectClassic",
        vec![Value::String("loose.js".into())],
    )
    .unwrap();
    app.call(root, "control", vec![]).unwrap();
    until(&mut app, |app| {
        app.get(root, "localValue").unwrap() == Value::Number(45.)
    });
}
#[test]
fn source_and_compiled_workers_copy_data_preserve_fifo_and_replace_their_realm() {
    let doc = Compiler::new()
        .with_javascript()
        .worker_script_entry("loose.js")
        .worker_script_entry("nested/nested-only.js")
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

#[test]
fn model_sync_acknowledges_owner_transactions_and_retries_after_conflicting_edits() {
    let mut app = construct(&generated::compiled_qml());
    let root = app.root();
    let Value::Object(actor) = app.get(root, "actor").unwrap() else {
        panic!()
    };
    let Value::Object(model) = app.get(root, "rows").unwrap() else {
        panic!()
    };
    until(&mut app, |app| {
        app.get(actor, "ready").unwrap() == Value::Bool(true)
    });
    app.call(root, "syncRows", vec![]).unwrap();
    // No event pump can commit the worker's edits before our local revision changes.
    app.call(
        model,
        "setProperty",
        vec![
            Value::Number(0.),
            Value::String("number".into()),
            Value::Number(42.),
        ],
    )
    .unwrap();
    until(
        &mut app,
        |app| matches!(app.get(root,"answer"),Ok(Value::String(s)) if s.starts_with("conflict:")),
    );
    assert_eq!(app.get(model, "count").unwrap(), Value::Number(1.));
    let Value::ModelRow(row) = app.call(model, "get", vec![Value::Number(0.)]).unwrap() else {
        panic!()
    };
    assert_eq!(app.model_role(row, "number").unwrap(), Value::Number(42.));
    app.call(root, "syncRows", vec![]).unwrap();
    until(&mut app, |app| {
        app.get(root, "answer").unwrap() == Value::String("synced-2".into())
    });
    assert_eq!(app.get(model, "count").unwrap(), Value::Number(2.));
    assert_eq!(app.model_role(row, "number").unwrap(), Value::Number(9.));
}

#[test]
fn retiring_a_dynamic_worker_revokes_queued_results_before_a_replacement_starts() {
    let mut app = construct(&generated::compiled_qml());
    let root = app.root();
    let Value::Object(factory) = app.get(root, "factory").unwrap() else {
        panic!()
    };
    let old = app
        .create_object(factory, Some(root), Default::default())
        .unwrap();
    for worker in [
        old,
        match app.get(root, "actor").unwrap() {
            Value::Object(id) => id,
            _ => panic!(),
        },
        match app.get(root, "classic").unwrap() {
            Value::Object(id) => id,
            _ => panic!(),
        },
    ] {
        until(&mut app, |app| {
            app.get(worker, "ready").unwrap() == Value::Bool(true)
        });
    }
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    app.set_worker_waker(Some(std::sync::Arc::new(move || {
        let _ = tx.try_send(());
    })));
    app.call(old, "sendMessage", vec![Value::Number(30.)])
        .unwrap();
    rx.recv_timeout(Duration::from_secs(6)).unwrap();
    app.destroy_object(old).unwrap();
    let replacement = app
        .create_object(factory, Some(root), Default::default())
        .unwrap();
    assert_ne!(old, replacement);
    assert!(app.get(old, "ready").is_err());
    until(&mut app, |app| {
        app.get(replacement, "ready").unwrap() == Value::Bool(true)
    });
    assert_eq!(app.get(root, "localValue").unwrap(), Value::Number(0.));
    app.call(replacement, "sendMessage", vec![Value::Number(3.)])
        .unwrap();
    until(&mut app, |app| {
        app.get(root, "localValue").unwrap() == Value::Number(3.)
    });
}

#[test]
fn replacing_source_settles_bindings_on_ready_before_returning_to_rust() {
    let mut app = construct(&generated::compiled_qml());
    let root = app.root();
    let Value::Object(actor) = app.get(root, "actor").unwrap() else {
        panic!()
    };
    until(&mut app, |app| {
        app.get(actor, "ready").unwrap() == Value::Bool(true)
    });
    assert_eq!(app.get(root, "observedReady").unwrap(), Value::Bool(true));
    app.set(actor, "source", Value::String(String::new()))
        .unwrap();
    assert_eq!(app.get(actor, "ready").unwrap(), Value::Bool(false));
    assert_eq!(app.get(root, "observedReady").unwrap(), Value::Bool(false));
}
