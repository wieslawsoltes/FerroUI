use ferroribbon_core::collections::RibbonFixedItems;
fn isolated(name: &str) -> bool {
    if std::env::var("FERRORIBBON_LOCALIZATION_CASE").as_deref() == Ok(name) {
        return false;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env("FERRORIBBON_LOCALIZATION_CASE", name)
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
use ferroribbon_core::{RibbonString, localization::*, model::RibbonItemsSource};
use serde_json::{Value, json};
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../../tests/fixtures/localization.json")).unwrap()
}
fn s(v: &str) -> Option<RibbonString> {
    Some(v.into())
}
fn read(v: &Value) -> Option<RibbonString> {
    v.as_array().map(|v| {
        RibbonString::from_utf16(
            v.iter()
                .map(|v| v.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )
    })
}
fn dump(v: Option<RibbonString>) -> Value {
    v.map_or(Value::Null, |v| json!(v.as_utf16()))
}
fn named(name: &str, iso: &str) -> Arc<dyn RibbonCultureInfo> {
    RibbonCulture::new(name.into(), iso.into())
}
fn state(value: Option<Arc<RibbonStrings>>) -> Value {
    let value = value.unwrap_or_else(RibbonStrings::current);
    json!({"culture":dump(value.culture().name()),"iso":dump(value.culture().two_letter_iso_language_name()),"file":dump(value.file()),"options":dump(value.options()),"search":dump(value.search()),"extra":dump(value.get(s("Extra").as_ref()).unwrap()),"unknown":dump(value.get(s("unknown").as_ref()).unwrap())})
}
fn entries(
    values: Vec<(&str, Option<&str>)>,
) -> Option<RibbonItemsSource<(Option<RibbonString>, Option<RibbonString>)>> {
    Some(RibbonItemsSource::from_values(
        values
            .into_iter()
            .map(|(k, v)| (s(k), v.map(Into::into)))
            .collect(),
    ))
}
#[test]
fn builtins_culture_fallback_and_requested_culture_identity_match_source() {
    if isolated("builtins_culture_fallback_and_requested_culture_identity_match_source") {
        return;
    }
    let d = corpus();
    RibbonStrings::set_current_ui_culture_provider(Some(Rc::new(|| named("fr-FR", "fr"))));
    assert_eq!(state(None), d["initial"]);
    for row in d["cultures"].as_array().unwrap() {
        if !row["error"].is_null() {
            continue;
        }
        let culture = RibbonCulture::new(read(&row["name"]).unwrap(), read(&row["iso"]).unwrap());
        let strings = RibbonStrings::for_culture(Some(culture.clone())).unwrap();
        assert!(Arc::ptr_eq(
            &strings.culture(),
            &(culture as Arc<dyn RibbonCultureInfo>)
        ));
        for value in row["values"].as_array().unwrap() {
            let key = read(&value["key"]).unwrap();
            assert_eq!(
                dump(strings.get(Some(&key)).unwrap()),
                value["value"],
                "{}",
                row["requested"]
            );
        }
    }
    let strings = RibbonStrings::for_culture(Some(RibbonCulture::invariant())).unwrap();
    assert_eq!(
        strings
            .format(
                Some(&"TablePickerFormat".into()),
                Some(RibbonFixedItems::new(vec![3.into(), 4.into()]))
            )
            .unwrap()
            .to_utf8()
            .unwrap(),
        "3x4 Table"
    );
}
#[test]
fn registry_refresh_overrides_events_live_key_views_and_partial_failures_match_source() {
    if isolated(
        "registry_refresh_overrides_events_live_key_views_and_partial_failures_match_source",
    ) {
        return;
    }
    let d = corpus();
    let expected = &d["registry"];
    let events = Arc::new(Mutex::new(vec![]));
    let output = events.clone();
    let event = RibbonStrings::current_changed();
    let token = event.add(Arc::new(move |_| {
        output
            .lock()
            .unwrap()
            .push(json!({"sender":true,"empty":true,"state":state(None)}))
    }));
    let cultures = RibbonStrings::built_in_cultures();
    let keys = RibbonStrings::keys();
    let trace = Arc::new(Mutex::new(Vec::<String>::new()));
    for row in expected["log"].as_array().unwrap() {
        let action = row["action"].as_str().unwrap();
        let result = catch_unwind(AssertUnwindSafe(|| match action {
            "current-de" => RibbonStrings::set_current(Some(
                RibbonStrings::for_culture(Some(named("de-AT", "de"))).unwrap(),
            )),
            "override" | "same-override" => {
                let current = RibbonStrings::current();
                let result = current.override_string(s("File"), s("Start"))?;
                assert!(Arc::ptr_eq(&result, &current));
                Ok(())
            }
            "register-neutral" => {
                RibbonStrings::register(s("DE"), entries(vec![("Options", Some("Neutral"))]))
            }
            "register-exact" => RibbonStrings::register(
                s("de-AT"),
                entries(vec![
                    ("File", Some("Exact")),
                    ("Options", Some("Exact options")),
                ]),
            ),
            "neutral-under-exact" => {
                RibbonStrings::register(s("de"), entries(vec![("Options", Some("unused"))]))
            }
            "same-current" => RibbonStrings::set_current(Some(RibbonStrings::current())),
            "off-current" => RibbonStrings::for_culture(Some(named("fr", "fr")))?
                .override_string(s("File"), s("Local"))
                .map(|_| ()),
            "null-current" => RibbonStrings::set_current(None),
            "null-name" => RibbonStrings::register(None, None),
            "empty-name" => RibbonStrings::register(s(""), None),
            "null-values" => RibbonStrings::register(s("zz-null"), None),
            "override-null-key" => RibbonStrings::current()
                .override_string(None, None)
                .map(|_| ()),
            "override-empty-key" => RibbonStrings::current()
                .override_string(s(""), None)
                .map(|_| ()),
            "override-null-value" => RibbonStrings::current()
                .override_string(s("File"), None)
                .map(|_| ()),
            "partial-register" => RibbonStrings::register(
                s("de-AT"),
                Some(RibbonItemsSource::new(|| {
                    Box::new(
                        std::iter::once((s("File"), s("first")))
                            .chain(std::iter::once_with(|| panic!("InvalidOperation: input"))),
                    )
                })),
            ),
            "refresh-after-partial" => RibbonStrings::register(s("de-AT"), entries(vec![])),
            "nested-register" => {
                let trace = trace.clone();
                RibbonStrings::register(
                    s("de-AT"),
                    Some(RibbonItemsSource::new(move || {
                        trace.lock().unwrap().push("enumerate".into());
                        let trace = trace.clone();
                        Box::new(std::iter::once((s("File"), s("first"))).chain(
                            std::iter::once_with(move || {
                                RibbonStrings::register(
                                    s("de-AT"),
                                    entries(vec![("Extra", Some("nested"))]),
                                )
                                .unwrap();
                                trace.lock().unwrap().push("after-action".into());
                                (s("Options"), s("last"))
                            }),
                        ))
                    })),
                )
            }
            "space-name" => RibbonStrings::register(s(" "), entries(vec![("File", Some("space"))])),
            "null-table-value" => RibbonStrings::register(
                s("zz"),
                entries(vec![("File", None), ("Options", Some(""))]),
            ),
            "current-null-table" => RibbonStrings::set_current(Some(
                RibbonStrings::for_culture(Some(named("zz", "zz"))).unwrap(),
            )),
            "english-extension" => {
                RibbonStrings::register(s("en"), entries(vec![("Extra", Some("English extra"))]))
            }
            _ => panic!("unmapped {action}"),
        }));
        match result {
            Err(_) => assert_eq!(row["error"]["type"], "InvalidOperationException"),
            Ok(Err(error)) => {
                assert_eq!(error.reference_type_name(), row["error"]["type"]);
                assert_eq!(json!(error.parameter), row["error"]["param"]);
            }
            Ok(Ok(())) => assert!(row["error"].is_null(), "{action}"),
        }
        assert_eq!(state(None), row["state"], "{action}");
        assert_eq!(
            json!(events.lock().unwrap().len()),
            row["raised"],
            "{action}"
        );
        assert_eq!(
            json!(cultures.iter().map(|s| dump(Some(s))).collect::<Vec<_>>()),
            row["cultures"],
            "{action}"
        );
        assert_eq!(
            json!(keys.iter().map(|s| dump(Some(s))).collect::<Vec<_>>()),
            row["keys"],
            "{action}"
        );
    }
    assert_eq!(json!(*events.lock().unwrap()), expected["events"]);
    assert_eq!(json!(*trace.lock().unwrap()), expected["trace"]);
    assert!(std::sync::Arc::ptr_eq(
        &cultures,
        &RibbonStrings::built_in_cultures()
    ));
    assert!(std::sync::Arc::ptr_eq(&keys, &RibbonStrings::keys()));
    event.remove(token);
}
#[test]
fn value_snapshots_and_english_fallback_have_different_update_behavior() {
    if isolated("value_snapshots_and_english_fallback_have_different_update_behavior") {
        return;
    }
    let data = corpus();
    let expected = &data["snapshots"];
    let culture = named("ja-JP", "ja");
    let old = RibbonStrings::for_culture(Some(culture.clone())).unwrap();
    RibbonStrings::register(
        s("en"),
        entries(vec![
            ("File", Some("English changed")),
            ("Late", Some("late")),
        ]),
    )
    .unwrap();
    let fresh = RibbonStrings::for_culture(Some(culture.clone())).unwrap();
    let cs = RibbonStrings::for_culture(Some(named("cs-CZ", "cs"))).unwrap();
    RibbonStrings::register(s("CS"), entries(vec![("File", Some("Soubor"))])).unwrap();
    let cs_new = RibbonStrings::for_culture(Some(named("cs-CZ", "cs"))).unwrap();
    assert_eq!(dump(old.file()), expected["old"]);
    assert_eq!(dump(fresh.file()), expected["fresh"]);
    assert_eq!(dump(old.get(s("Late").as_ref()).unwrap()), expected["late"]);
    assert_eq!(dump(cs.file()), expected["csOld"]);
    assert_eq!(dump(cs_new.file()), expected["csNew"]);
    assert!(Arc::ptr_eq(&culture, &old.culture()));
    assert!(!Arc::ptr_eq(&old, &fresh));
    let key: RibbonString = "not a key".into();
    assert!(fresh.get(Some(&key)).unwrap().unwrap().ptr_eq(&key));
}
struct Culture {
    trace: Arc<Mutex<Vec<String>>>,
    name: Option<RibbonString>,
    iso: Option<RibbonString>,
}
impl RibbonCultureInfo for Culture {
    fn name(&self) -> Option<RibbonString> {
        self.trace.lock().unwrap().push("name".into());
        self.name.clone()
    }
    fn two_letter_iso_language_name(&self) -> Option<RibbonString> {
        self.trace.lock().unwrap().push("iso".into());
        self.iso.clone()
    }
}
#[test]
fn virtual_culture_dispatch_short_circuits_and_keeps_the_original_object() {
    if isolated("virtual_culture_dispatch_short_circuits_and_keeps_the_original_object") {
        return;
    }
    for row in corpus()["virtualCulture"].as_array().unwrap() {
        let trace = Arc::new(Mutex::new(vec![]));
        let culture = Arc::new(Culture {
            trace: trace.clone(),
            name: if row["nullName"].as_bool().unwrap() {
                None
            } else {
                read(&row["inputName"])
            },
            iso: read(&row["iso"]),
        });
        let result = RibbonStrings::for_culture(Some(culture.clone()));
        assert_eq!(result.is_err(), !row["error"].is_null());
        assert_eq!(json!(*trace.lock().unwrap()), row["trace"]);
        match result {
            Err(e) => {
                assert_eq!(e.reference_type_name(), row["error"]["type"]);
                assert_eq!(json!(e.parameter), row["error"]["param"]);
            }
            Ok(v) => {
                assert_eq!(dump(v.file()), row["file"]);
                assert!(Arc::ptr_eq(
                    &v.culture(),
                    &(culture as Arc<dyn RibbonCultureInfo>)
                ));
            }
        }
    }
}
#[test]
fn nested_events_preserve_snapshot_order_stop_on_failure_and_keep_applied_mutations() {
    if isolated("nested_events_preserve_snapshot_order_stop_on_failure_and_keep_applied_mutations")
    {
        return;
    }
    let data = corpus();
    let expected = &data["events"];
    let strings = RibbonStrings::for_culture(Some(named("en-US", "en"))).unwrap();
    let trace = Arc::new(Mutex::new(vec![]));
    let changed = Arc::new(AtomicBool::new(false));
    let event = RibbonStrings::current_changed();
    let output = trace.clone();
    let a = event.add(Arc::new(move |_| {
        let current = RibbonStrings::current();
        let file = current.file().unwrap().to_utf8().unwrap();
        output.lock().unwrap().push(format!("a:{file}"));
        if !changed.load(Ordering::SeqCst) && file == "Outer" {
            changed.store(true, Ordering::SeqCst);
            current.override_string(s("File"), s("Nested")).unwrap();
        }
        if current.file().unwrap().to_utf8().unwrap() == "Throw" {
            panic!("InvalidOperation: handler");
        }
    }));
    let output = trace.clone();
    let b = event.add(Arc::new(move |_| {
        output.lock().unwrap().push(format!(
            "b:{}",
            RibbonStrings::current().file().unwrap().to_utf8().unwrap()
        ))
    }));
    RibbonStrings::set_current(Some(strings.clone())).unwrap();
    trace.lock().unwrap().clear();
    strings.override_string(s("File"), s("Outer")).unwrap();
    assert!(
        catch_unwind(AssertUnwindSafe(
            || strings.override_string(s("File"), s("Throw"))
        ))
        .is_err()
    );
    assert_eq!(json!(*trace.lock().unwrap()), expected["trace"]);
    assert_eq!(state(None), expected["state"]);
    assert!(Arc::ptr_eq(&strings, &RibbonStrings::current()));
    event.remove(a);
    event.remove(b);
}
struct FormatCulture {
    trace: Arc<Mutex<Vec<String>>>,
    custom: Option<RibbonString>,
}
impl RibbonCultureInfo for FormatCulture {
    fn name(&self) -> Option<RibbonString> {
        s("")
    }
    fn two_letter_iso_language_name(&self) -> Option<RibbonString> {
        s("iv")
    }
    fn custom_formatter(&self) -> Option<Rc<RibbonCustomFormatter>> {
        self.trace
            .lock()
            .unwrap()
            .push("provider:ICustomFormatter".into());
        let trace = self.trace.clone();
        let text = self.custom.clone();
        Some(Rc::new(move |format, arg, _| {
            trace.lock().unwrap().push(format!(
                "custom:{}:{}",
                format
                    .map(|s| s.to_utf8().unwrap())
                    .unwrap_or_else(|| "<null>".into()),
                if matches!(arg, RibbonFormatArgument::Null) {
                    "null"
                } else {
                    "value"
                }
            ));
            Ok(text.clone())
        }))
    }
}
struct Probe {
    trace: Arc<Mutex<Vec<String>>>,
    text: Option<RibbonString>,
}
impl RibbonFormatObject for Probe {
    fn format(
        &self,
        format: Option<&RibbonString>,
        _: &Arc<dyn RibbonCultureInfo>,
    ) -> Result<Option<RibbonString>, RibbonLocalizationError> {
        self.trace.lock().unwrap().push(format!(
            "arg:{}",
            format
                .map(|s| s.to_utf8().unwrap())
                .unwrap_or_else(|| "<null>".into())
        ));
        Ok(self.text.clone())
    }
}
#[test]
fn composite_format_callbacks_braces_alignment_utf16_and_error_precedence_match_source() {
    if isolated(
        "composite_format_callbacks_braces_alignment_utf16_and_error_precedence_match_source",
    ) {
        return;
    }
    let data = corpus();
    for row in data["formatting"]["cases"].as_array().unwrap() {
        let trace = Arc::new(Mutex::new(vec![]));
        let culture = Arc::new(FormatCulture {
            trace: trace.clone(),
            custom: read(&row["custom"]),
        });
        let strings = RibbonStrings::for_culture(Some(culture)).unwrap();
        strings
            .override_string(s("Fmt"), read(&row["template"]))
            .unwrap();
        let probe = Rc::new(Probe {
            trace: trace.clone(),
            text: if row["nullable"].as_bool().unwrap() {
                None
            } else {
                s("😀e\u{301}")
            },
        });
        let result = strings.format(
            s("Fmt").as_ref(),
            Some(RibbonFixedItems::new(vec![
                RibbonFormatArgument::Object(probe),
                "two".into(),
            ])),
        );
        assert_eq!(result.is_err(), !row["error"].is_null(), "{row}");
        assert_eq!(json!(*trace.lock().unwrap()), row["trace"], "{row}");
        match result {
            Ok(s) => assert_eq!(dump(Some(s)), row["result"]),
            Err(e) => assert_eq!(e.reference_type_name(), row["error"]["type"]),
        }
    }
    for row in data["formatting"]["nulls"].as_array().unwrap() {
        let strings = RibbonStrings::for_culture(Some(RibbonCulture::invariant())).unwrap();
        let args = if row["argsNull"].as_bool().unwrap() {
            None
        } else {
            Some(RibbonFixedItems::new(vec![
                RibbonFormatArgument::Null;
                row["argsCount"].as_u64().unwrap()
                    as usize
            ]))
        };
        let key = read(&row["key"]);
        let result = strings.format(key.as_ref(), args);
        assert_eq!(result.is_err(), !row["error"].is_null());
        match result {
            Ok(s) => assert_eq!(dump(Some(s)), row["result"]),
            Err(e) => {
                assert_eq!(e.reference_type_name(), row["error"]["type"]);
                assert_eq!(json!(e.parameter), row["error"]["param"]);
            }
        }
    }
}
#[test]
fn composite_arguments_remain_live_and_weak_callbacks_do_not_form_cycles() {
    if isolated("composite_arguments_remain_live_and_weak_callbacks_do_not_form_cycles") {
        return;
    }
    struct Mutator {
        array: std::rc::Weak<RibbonFixedItems<RibbonFormatArgument>>,
        trace: Arc<Mutex<Vec<String>>>,
    }
    impl RibbonFormatObject for Mutator {
        fn format(
            &self,
            _: Option<&RibbonString>,
            _: &Arc<dyn RibbonCultureInfo>,
        ) -> Result<Option<RibbonString>, RibbonLocalizationError> {
            self.trace.lock().unwrap().push("format".into());
            self.array.upgrade().unwrap().set(1, "new".into());
            Ok(s("first"))
        }
    }
    let expected = &corpus()["formatting"]["mutate"];
    let array = RibbonFixedItems::new(vec![RibbonFormatArgument::Null, "old".into()]);
    let trace = Arc::new(Mutex::new(vec![]));
    let value = Rc::new(Mutator {
        array: Rc::downgrade(&array),
        trace: trace.clone(),
    });
    let weak = Rc::downgrade(&value);
    array.set(0, RibbonFormatArgument::Object(value));
    let strings = RibbonStrings::for_culture(Some(RibbonCulture::invariant()))
        .unwrap()
        .override_string(s("Fmt"), s("{0}/{1}/{0}"))
        .unwrap();
    assert_eq!(
        dump(Some(
            strings
                .format(s("Fmt").as_ref(), Some(array.clone()))
                .unwrap()
        )),
        expected["result"]
    );
    assert_eq!(json!(*trace.lock().unwrap()), expected["trace"]);
    drop(array);
    assert!(weak.upgrade().is_none());
}
#[test]
fn key_collections_are_live_versioned_and_static_event_subscribers_require_removal() {
    if isolated("key_collections_are_live_versioned_and_static_event_subscribers_require_removal") {
        return;
    }
    let view = RibbonStrings::built_in_cultures();
    let mut active = view.iter();
    active.next();
    RibbonStrings::register(s("x-keys"), entries(vec![])).unwrap();
    assert!(catch_unwind(AssertUnwindSafe(|| active.next())).is_err());
    assert!(view.iter().any(|n| n == RibbonString::from("x-keys")));
    let event = RibbonStrings::current_changed();
    let target = Arc::new(12);
    let weak = Arc::downgrade(&target);
    let token = event.add(Arc::new(move |_| assert_eq!(*target, 12)));
    assert!(weak.upgrade().is_some());
    event.remove(token);
    assert!(weak.upgrade().is_none());
}

// Development-only mapping to unchanged C# fixture vocabulary.
// The shipped Result errors expose Rust kinds, Display and std::error::Error.
trait ReferenceErrorName {
    fn reference_type_name(&self) -> &'static str;
}
impl ReferenceErrorName for RibbonLocalizationError {
    fn reference_type_name(&self) -> &'static str {
        match self.kind {
            RibbonLocalizationErrorKind::ArgumentNull => "ArgumentNullException",
            RibbonLocalizationErrorKind::Argument => "ArgumentException",
            RibbonLocalizationErrorKind::Format => "FormatException",
            RibbonLocalizationErrorKind::InvalidOperation => "InvalidOperationException",
            RibbonLocalizationErrorKind::Unavailable => "Unavailable",
        }
    }
}
