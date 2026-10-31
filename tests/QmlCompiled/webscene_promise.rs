use super::*;
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

    assert!(app
        .call_and_await(root, "outOfTurn", vec![], deadline)
        .unwrap_err()
        .message
        .contains("outside a synchronous"));
    assert!(app
        .call(root, "failedOuter", vec![])
        .unwrap_err()
        .message
        .contains("outer failed"));
    let Value::External(after) = app.call(root, "afterFailure", vec![]).unwrap() else {
        panic!("missing later job")
    };
    assert!(
        matches!(app.promise_state(&after).unwrap(), PromiseState::Rejected(reason) if reason.contains("outside a synchronous"))
    );
    assert_eq!(app.get(root, "left").unwrap(), n(8.));
}

#[test]
fn v8_promises_settle_with_bounds_rollback_and_no_late_qml_callbacks() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("webscene/App.qml");
    promises(
        &Compiler::new()
            .with_javascript()
            .load(path)
            .unwrap()
            .document,
    );
    promises(&compiled_qml());
}
