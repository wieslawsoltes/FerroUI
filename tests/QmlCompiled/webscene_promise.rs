use super::*;
mod compiled_async {
    include!(concat!(env!("OUT_DIR"), "/async_model.rs"));
}
use ferroui_qml::PromiseState;
use std::time::{Duration, Instant};

fn promises(source: &impl ComponentSource) {
    let engine = Rc::new(WebSceneEngine::from_env().unwrap());
    let mut app =
        Instance::with_engine(source, &TypeRegistry::new(), Default::default(), engine).unwrap();
    let root = app.root();
    let n = Value::Number;
    let deadline = Duration::from_millis(250);
    assert_eq!(
        app.call(root, "ordered", vec![n(8.), n(2.)]).unwrap(),
        Value::String("8/2".into())
    );
    assert_eq!(
        app.call_and_await(root, "fulfilledLater", vec![n(8.), n(4.)], deadline)
            .unwrap(),
        n(12.)
    );
    assert_eq!(app.get(root, "left").unwrap(), n(8.));
    assert_eq!(
        app.call_and_await(root, "importAndSettle", vec![n(2.)], deadline)
            .unwrap(),
        n(42.)
    );
    let rejection = app
        .call_and_await(root, "rejectedLater", vec![n(24.)], deadline)
        .unwrap_err();
    assert!(
        rejection.message.contains("expected rejection"),
        "{rejection}"
    );
    assert_eq!(app.get(root, "left").unwrap(), n(8.));

    let Value::External(handle) = app.call(root, "pendingLater", vec![]).unwrap() else {
        panic!("expected retained Promise")
    };
    assert_eq!(app.promise_state(&handle).unwrap(), PromiseState::Pending);
    let Value::String(origin) = app
        .call_and_await(root, "moduleOrigin", vec![], deadline)
        .unwrap()
    else {
        panic!("missing module URL")
    };
    assert!(origin.ends_with("/caf%C3%A9%20data.mjs"), "{origin}");

    let started = Instant::now();
    let timeout = app
        .settle_promise(&handle, Duration::from_millis(20))
        .unwrap_err();
    assert!(timeout.message.contains("did not settle"), "{timeout}");
    assert!(started.elapsed() < Duration::from_secs(3));
    app.call(root, "finishLater", vec![n(37.)]).unwrap();
    assert_eq!(app.settle_promise(&handle, deadline).unwrap(), n(37.));
    assert_eq!(
        app.promise_state(&handle).unwrap(),
        PromiseState::Fulfilled(n(37.))
    );

    assert_eq!(
        app.call_and_await(root, "immediate", vec![n(2.)], deadline)
            .unwrap(),
        n(30.)
    );
    let err = app
        .call_and_await(root, "rejectAfterAwait", vec![], deadline)
        .unwrap_err();
    assert!(err.message.contains("asynchronous failure"), "{err}");
    assert_eq!(app.get(root, "left").unwrap(), n(10.));
    assert_eq!(app.get(root, "checksum").unwrap(), n(30.));
    assert_eq!(
        app.call_and_await(root, "captureThis", vec![n(5.)], deadline)
            .unwrap(),
        n(15.)
    );
    assert_eq!(
        app.call_and_await(root, "moduleContinuations", vec![], deadline)
            .unwrap(),
        n(31.)
    );
    let retained = Value::String("retained".into());
    assert_eq!(
        app.call_and_await(root, "retainRow", vec![], deadline)
            .unwrap(),
        retained
    );
    assert_eq!(app.call(root, "readRetainedRow", vec![]).unwrap(), retained);
    app.call_and_await(root, "outOfTurn", vec![], deadline)
        .unwrap();
    assert_eq!(app.get(root, "left").unwrap(), n(89.));
    let Value::Object(child) = app.get(root, "child").unwrap() else {
        panic!("child")
    };
    let Value::External(job) = app.call(child, "suspendChild", vec![]).unwrap() else {
        panic!("promise")
    };
    app.call(root, "resumeChild", vec![]).unwrap();
    assert_eq!(app.settle_promise(&job, deadline).unwrap(), n(105.));
    assert_eq!(app.get(root, "left").unwrap(), n(89.));
    let Value::External(aborted) = app.call(child, "suspendChild", vec![]).unwrap() else {
        panic!("promise")
    };
    assert!(app
        .call(root, "abortWakeChild", vec![])
        .unwrap_err()
        .message
        .contains("resolver aborted"));
    assert!(
        matches!(app.promise_state(&aborted).unwrap(), PromiseState::Rejected(reason) if reason.contains("rolled back"))
    );
    assert_eq!(app.get(child, "left").unwrap(), n(105.));
    assert!(app
        .call(root, "failedOuter", vec![])
        .unwrap_err()
        .message
        .contains("outer failed"));
    let Value::External(after) = app.call(root, "afterFailure", vec![]).unwrap() else {
        panic!("promise")
    };
    assert!(
        matches!(app.promise_state(&after).unwrap(), PromiseState::Rejected(reason) if reason.contains("rolled back"))
    );
    assert!(app
        .call(root, "abandonPending", vec![])
        .unwrap_err()
        .message
        .contains("cancel suspended work"));
    assert!(app
        .call_and_await(root, "wakeAbandoned", vec![], deadline)
        .unwrap_err()
        .message
        .contains("rolled back"));
    let started = Instant::now();
    let runaway = app
        .call_and_await(root, "spinAfterAwait", vec![], Duration::from_millis(150))
        .unwrap_err();
    assert!(runaway.message.contains("deadline"), "{runaway}");
    assert!(started.elapsed() < Duration::from_secs(3));
    assert_eq!(app.get(root, "left").unwrap(), n(89.));
}

#[test]
fn v8_continuations_reenter_the_owner_thread_with_bounded_and_revocable_transactions() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("webscene/Async.qml");
    promises(
        &Compiler::new()
            .with_javascript()
            .load(path)
            .unwrap()
            .document,
    );
    promises(&compiled_async::compiled_qml());
}
