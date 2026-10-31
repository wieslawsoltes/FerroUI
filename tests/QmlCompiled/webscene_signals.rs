use super::*;
use std::time::Duration;
include!(concat!(env!("OUT_DIR"), "/webscene_signals.rs"));

fn exercise_signals(source: &impl ComponentSource) {
    let engine = Rc::new(WebSceneEngine::from_env().unwrap());
    let mut model =
        Instance::with_engine(source, &TypeRegistry::new(), Default::default(), engine).unwrap();
    let root = model.root();
    let string = |s: &str| Value::String(s.into());
    let emit = |model: &mut Instance, n: f64| model.emit(root, "reading", vec![Value::Number(n)]);
    assert!(model.call(root, "connectAndFail", vec![]).is_err());
    emit(&mut model, 7.).unwrap();
    assert_eq!(model.get(root, "history").unwrap(), string(""));
    model.call(root, "install", vec![]).unwrap();
    assert!(model.call(root, "disconnectAndFail", vec![]).is_err());
    emit(&mut model, 2.).unwrap();
    assert_eq!(model.get(root, "history").unwrap(), string("M2M2S2"));
    model.call(root, "removeAll", vec![]).unwrap();
    emit(&mut model, 8.).unwrap();
    assert_eq!(model.get(root, "history").unwrap(), string("M2M2S2"));
    assert_eq!(
        model.call(root, "explicitReceiver", vec![]).unwrap(),
        string("4")
    );
    assert_eq!(
        model.call(root, "mutation", vec![]).unwrap(),
        string("A/AAC")
    );
    assert_eq!(model.call(root, "badArgs", vec![]).unwrap(), string("2"));
    model.call(root, "observeCount", vec![]).unwrap();
    model.set(root, "count", Value::Number(6.)).unwrap();
    assert_eq!(model.get(root, "history").unwrap(), string("M2M2S2C6"));
    model.call(root, "installFailure", vec![]).unwrap();
    model.watch_signal(root, "reading").unwrap();
    assert!(emit(&mut model, 3.)
        .unwrap_err()
        .message
        .contains("callback failed"));
    assert_eq!(model.get(root, "history").unwrap(), string("M2M2S2C6"));
    assert!(model.drain_signals().is_empty());
    model.call(root, "clearFailure", vec![]).unwrap();
    emit(&mut model, 4.).unwrap();
}

#[test]
fn imperative_signal_connections_match_qt_from_rust_and_js() {
    let source = Compiler::new()
        .with_javascript()
        .load(Path::new(env!("CARGO_MANIFEST_DIR")).join("webscene/Signals.qml"))
        .unwrap();
    exercise_signals(&source.document);
    exercise_signals(&compiled_qml());
}

#[test]
fn signal_connections_from_an_expired_v8_generation_are_pruned() {
    let engine = Rc::new(
        WebSceneEngine::from_env()
            .unwrap()
            .with_timeout(Duration::from_millis(400)),
    );
    let mut model = Instance::with_engine(
        &compiled_qml(),
        &TypeRegistry::new(),
        Default::default(),
        engine,
    )
    .unwrap();
    let root = model.root();
    model.call(root, "install", vec![]).unwrap();
    assert!(model.call(root, "spin", vec![]).is_err());
    model
        .emit(root, "reading", vec![Value::Number(3.)])
        .unwrap();
    assert_eq!(
        model.get(root, "history").unwrap(),
        Value::String(String::new())
    );
    model.call(root, "install", vec![]).unwrap();
    model
        .emit(root, "reading", vec![Value::Number(5.)])
        .unwrap();
    assert_eq!(
        model.get(root, "history").unwrap(),
        Value::String("M5M5S5".into())
    );
}
