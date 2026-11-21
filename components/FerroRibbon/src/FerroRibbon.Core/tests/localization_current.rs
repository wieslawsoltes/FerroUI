//! Required source-global Current assignment and setter-thread notification.
use ferroribbon_core::{localization::*, model::RibbonItemsSource};
use serde_json::{Value, json};
use std::{
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};
fn isolated(name: &str) -> bool {
    if std::env::var("FERRORIBBON_CURRENT_CASE").as_deref() == Ok(name) {
        return false;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env("FERRORIBBON_CURRENT_CASE", name)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("running 1 test\n") && output.status.success(),
        "{name}\n{text}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    true
}
fn case(name: &str) -> Value {
    serde_json::from_str::<Value>(include_str!(
        "../../../tests/fixtures/localization-registry.json"
    ))
    .unwrap()["cases"][name]
        .clone()
}
fn get(name: &str) -> Arc<RibbonStrings> {
    RibbonStrings::for_culture(Some(RibbonCulture::new(name.into(), "x".into()))).unwrap()
}
#[test]
fn worker_assignment_and_override_match_original_current_fixture() {
    if isolated("worker_assignment_and_override_match_original_current_fixture") {
        return;
    }
    RibbonStrings::set_current(Some(
        RibbonStrings::for_culture(Some(RibbonCulture::new("en".into(), "en".into()))).unwrap(),
    ))
    .unwrap();
    let events = Arc::new(Mutex::new(Vec::new()));
    let seen = events.clone();
    let changed = RibbonStrings::current_changed();
    let token = changed.add(Arc::new(move |_| {
        seen.lock().unwrap().push((
            std::thread::current().id(),
            RibbonStrings::current().file().unwrap().as_utf16().to_vec(),
        ))
    }));
    let worker = std::thread::spawn(|| {
        RibbonStrings::set_current(Some(
            RibbonStrings::for_culture(Some(RibbonCulture::new("fr".into(), "fr".into()))).unwrap(),
        ))
        .unwrap();
        RibbonStrings::current()
            .override_string(Some("File".into()), Some("Thread override".into()))
            .unwrap();
        std::thread::current().id()
    })
    .join()
    .unwrap();
    changed.remove(token);
    let source: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/localization-registry.json"
    ))
    .unwrap();
    let source = &source["cases"]["current"];
    assert_eq!(
        serde_json::json!(RibbonStrings::current().file().unwrap().as_utf16()),
        source["mainFile"]
    );
    let trace = events.lock().unwrap();
    assert_eq!(
        serde_json::json!(trace.iter().map(|(_, v)| v).collect::<Vec<_>>()),
        source["events"]
    );
    assert!(trace.iter().all(|(id, _)| *id == worker));
}

#[test]
fn current_and_culture_retain_the_same_identity_on_other_threads() {
    if isolated("current_and_culture_retain_the_same_identity_on_other_threads") {
        return;
    }
    fn thread_safe<T: Send + Sync>() {}
    thread_safe::<RibbonStrings>();
    thread_safe::<RibbonStringsChanged>();
    let culture: Arc<dyn RibbonCultureInfo> = RibbonCulture::new("fr".into(), "fr".into());
    let value = RibbonStrings::for_culture(Some(culture.clone())).unwrap();
    RibbonStrings::set_current(Some(value.clone())).unwrap();
    let result = std::thread::spawn(move || {
        json!({
            "sameValue": Arc::ptr_eq(&value, &RibbonStrings::current()),
            "sameCulture": Arc::ptr_eq(&culture, &RibbonStrings::current().culture())
        })
    })
    .join()
    .unwrap();
    assert_eq!(result, case("identity"));
}

#[test]
fn worker_registration_refreshes_current_once_and_preserves_overrides_and_culture() {
    if isolated("worker_registration_refreshes_current_once_and_preserves_overrides_and_culture") {
        return;
    }
    let culture: Arc<dyn RibbonCultureInfo> = RibbonCulture::new("x-refresh".into(), "x".into());
    let old = RibbonStrings::for_culture(Some(culture.clone())).unwrap();
    old.override_string(Some("File".into()), Some("Keep override".into()))
        .unwrap();
    RibbonStrings::set_current(Some(old.clone())).unwrap();
    let trace = Arc::new(Mutex::new(Vec::new()));
    let seen = trace.clone();
    let event = RibbonStrings::current_changed();
    let token = event.add(Arc::new(move |_| {
        seen.lock().unwrap().push(std::thread::current().id())
    }));
    let worker = std::thread::spawn(|| {
        RibbonStrings::register(
            Some("X-REFRESH".into()),
            Some(RibbonItemsSource::from_values(vec![
                (Some("File".into()), Some("Registered".into())),
                (Some("Options".into()), Some("Worker options".into())),
            ])),
        )
        .unwrap();
        std::thread::current().id()
    })
    .join()
    .unwrap();
    event.remove(token);
    let current = RibbonStrings::current();
    let trace = trace.lock().unwrap();
    assert_eq!(
        json!({"replaced": !Arc::ptr_eq(&old, &current),
        "sameCulture": Arc::ptr_eq(&culture, &current.culture()),
        "file":current.file().unwrap().as_utf16(), "options":current.options().unwrap().as_utf16(),
        "oldOptions":old.options().unwrap().as_utf16(), "count":trace.len(),
        "eventsOnWorker":trace.iter().all(|id| *id == worker)}),
        case("refresh")
    );
}

#[test]
fn removing_subscriptions_preserves_an_in_flight_notification_snapshot() {
    if isolated("removing_subscriptions_preserves_an_in_flight_notification_snapshot") {
        return;
    }
    RibbonStrings::set_current(Some(get("en"))).unwrap();
    let event = RibbonStrings::current_changed();
    let trace = Arc::new(Mutex::new(Vec::new()));
    let (entered, entry) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let released = Mutex::new(released);
    let first_trace = trace.clone();
    let first_target = Arc::new(());
    let first_weak = Arc::downgrade(&first_target);
    let first = event.add(Arc::new(move |_| {
        let _keep = &first_target;
        first_trace.lock().unwrap().push("first");
        entered.send(()).unwrap();
        released
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
    }));
    let second_trace = trace.clone();
    let target = Arc::new(());
    let weak = Arc::downgrade(&target);
    let second = event.add(Arc::new(move |_| {
        let _keep = &target;
        assert!(
            first_weak.upgrade().is_some(),
            "the full invocation snapshot retains earlier handlers too"
        );
        second_trace.lock().unwrap().push("second");
    }));
    let worker = std::thread::spawn(|| RibbonStrings::set_current(Some(get("fr"))).unwrap());
    entry.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(event.remove(first));
    assert!(event.remove(second));
    assert!(
        weak.upgrade().is_some(),
        "snapshot retains removed callback captures"
    );
    release.send(()).unwrap();
    worker.join().unwrap();
    assert!(
        weak.upgrade().is_none(),
        "completed snapshot releases removed callback captures"
    );
    RibbonStrings::set_current(Some(get("de"))).unwrap();
    assert_eq!(json!({"trace":*trace.lock().unwrap()}), case("snapshot"));
}
