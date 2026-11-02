//! The desktop and browser conformance suites use the same compiled worker scripts.
use super::*;
use ferroui_qml::Error;
include!(concat!(env!("OUT_DIR"), "/browser_workers.rs"));

async fn until(
    app: &mut Instance,
    what: &str,
    condition: impl Fn(&Instance) -> bool,
) -> Result<(), Error> {
    for _ in 0..2000 {
        app.poll_worker_scripts(16)?;
        if condition(app) {
            return Ok(());
        }
        next_turn().await;
    }
    Err(Error::new(format!("WorkerScript did not deliver {what}")))
}

pub(super) async fn check() -> Result<(), Error> {
    let engine = Rc::new(WebSceneEngine::browser()?.with_timeout(Duration::from_secs(3)));
    engine.prepare_browser_instance().await?;
    let mut app = Instance::with_engine(
        &compiled_qml(),
        &TypeRegistry::new(),
        Default::default(),
        engine,
    )?;
    let root = app.root();
    let string = |s: &str| Value::String(s.into());
    let has = |app: &Instance, name, value| app.get(root, name).ok() == Some(value);

    // Send during startup. Each worker must preserve order, copy the input
    // before it is mutated and resolve its dependencies against its own URL.
    app.call(root, "start", vec![])?;
    app.call(root, "startNested", vec![])?;
    until(&mut app, "initial replies", |app| {
        has(app, "answer", string("1:2/3"))
            && has(app, "localValue", Value::Number(3.))
            && has(app, "nestedValue", Value::Number(21.))
    })
    .await?;

    app.call(root, "syncChild", vec![])?;
    app.call(root, "localChildEdit", vec![])?;
    until(&mut app, "model conflict", |app| {
        matches!(app.get(root,"answer"), Ok(Value::String(s)) if s.starts_with("nested-conflict:"))
    }).await?;
    if app.call(root, "groupTitle", vec![])? != string("original")
        || app.call(root, "childReport", vec![])? != string("1/51")
    {
        return Err(Error::new("conflicting nested sync mutated the model"));
    }
    app.call(root, "syncChild", vec![])?;
    until(&mut app, "model receipt", |app| {
        has(app, "answer", string("nested-2"))
    })
    .await?;
    if app.call(root, "groupTitle", vec![])? != string("updated")
        || app.call(root, "childReport", vec![])? != string("2/17")
    {
        return Err(Error::new("successful nested sync lost a model edit"));
    }

    // Wait for the first model request to reach the inbox, replace the actor
    // before pumping the owner transaction, then sync from the new generation.
    let waiting = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let notification = waiting.clone();
    app.set_worker_waker(Some(std::sync::Arc::new(move || {
        notification.store(true, std::sync::atomic::Ordering::Release)
    })));
    app.call(root, "syncRows", vec![])?;
    for _ in 0..1000 {
        if waiting.load(std::sync::atomic::Ordering::Acquire) {
            break;
        }
        next_turn().await;
    }
    if !waiting.load(std::sync::atomic::Ordering::Acquire) {
        return Err(Error::new("model synchronization did not wake its owner"));
    }
    app.set_worker_waker(None);
    app.call(root, "restart", vec![])?;
    app.call(root, "syncRows", vec![])?;
    until(&mut app, "replacement model sync", |app| {
        has(app, "answer", string("synced-2"))
    })
    .await?;
    let Value::Object(rows) = app.get(root, "rows")? else {
        return Err(Error::new("expected ListModel"));
    };
    if app.get(rows, "count")? != Value::Number(2.) {
        return Err(Error::new(
            "retired WorkerScript model edits reached the owner",
        ));
    }

    // A blocked guest streams its first reply and the other worker keeps
    // running. Expiry retires only the runaway actor.
    app.call(root, "spin", vec![])?;
    app.call(root, "control", vec![])?;
    until(&mut app, "independent replies", |app| {
        has(app, "answer", string("started")) && has(app, "localValue", Value::Number(8.))
    })
    .await?;
    let timeout = until(&mut app, "watchdog", |_| false).await.unwrap_err();
    if !timeout.message.contains("deadline") || !has(&app, "observedReady", Value::Bool(false)) {
        return Err(Error::new(format!("WorkerScript watchdog: {timeout}")));
    }

    app.call(root, "restart", vec![])?;
    app.call(root, "start", vec![])?;
    until(&mut app, "replacement", |app| {
        has(app, "answer", string("1:2/3"))
    })
    .await?;
    app.call(root, "selectClassic", vec![string("loose.js")])?;
    app.call(root, "control", vec![])?;
    until(&mut app, "runtime source", |app| {
        has(app, "localValue", Value::Number(45.))
    })
    .await?;

    // Retire an actor with a queued send. Its replacement has independent
    // state, and the old reply cannot reach a QML handler.
    let Value::Object(factory) = app.get(root, "factory")? else {
        return Err(Error::new("expected worker factory"));
    };
    let old = app.create_object(factory, Some(root), Default::default())?;
    app.call(old, "sendMessage", vec![Value::Number(70.)])?;
    app.destroy_object(old)?;
    let new = app.create_object(factory, Some(root), Default::default())?;
    until(&mut app, "new worker", |app| {
        app.get(new, "ready").ok() == Some(Value::Bool(true))
    })
    .await?;
    if !has(&app, "localValue", Value::Number(45.)) {
        return Err(Error::new("destroyed WorkerScript delivered a reply"));
    }
    app.call(new, "sendMessage", vec![Value::Number(7.)])?;
    until(&mut app, "fresh counter", |app| {
        has(app, "localValue", Value::Number(7.))
    })
    .await
}
