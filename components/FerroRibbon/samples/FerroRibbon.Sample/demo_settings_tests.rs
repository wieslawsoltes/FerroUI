use super::*;
use serde_json::{Value, json};
use std::panic::{AssertUnwindSafe, catch_unwind};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../tests/fixtures/demo-settings.json")).unwrap()
}
fn settings() -> Rc<DemoSettings> {
    DemoSettings::new(Rc::new(|| Ok("03:04:05".into())))
}
fn state(s: &DemoSettings) -> Value {
    json!({"style":s.surface_style().raw(),"density":s.density().raw(),"mode":s.display_mode().map(|m|m.raw()),"log":s.log().borrow().clone()})
}
fn run(f: impl FnOnce() + Send + 'static) {
    use ferroui_controls::{AppBuilder, Application};
    use ferroui_headless::{FerroHeadlessPlatformExtensions, HeadlessUnitTestSession};
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(Default::default())
    });
    let result = session
        .dispatch(
            move || {
                crate::register_types();
                crate::install_application_resources();
                f();
            },
            ferroui_base::threading::CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

#[test]
fn original_defaults_and_repeated_apply_notifications_match() {
    for case in fixture()["cases"].as_array().unwrap() {
        let s = settings();
        match case["kind"].as_str().unwrap() {
            "defaults" => assert_eq!(state(&s), case["state"]),
            "apply" => {
                let trace = Rc::new(RefCell::new(Vec::new()));
                let t = trace.clone();
                let weak = Rc::downgrade(&s);
                let _sub = s.changed(move || t.borrow_mut().push(state(&weak.upgrade().unwrap())));
                for _ in 0..2 {
                    s.apply(
                        RibbonDensity::from_raw(case["density"].as_i64().unwrap() as i32),
                        case["mode"]
                            .as_i64()
                            .map(|m| RibbonDisplayMode::from_raw(m as i32)),
                    );
                }
                assert_eq!(json!(*trace.borrow()), case["trace"]);
                assert_eq!(state(&s), case["state"]);
            }
            _ => {}
        }
    }
}

#[test]
fn original_log_timestamps_nullable_messages_and_post_mutation_events_match() {
    let mut count = 0;
    for case in fixture()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["kind"] == "write")
    {
        let hms = case["hms"].as_array().unwrap();
        let time = time::Time::from_hms(
            hms[0].as_u64().unwrap() as u8,
            hms[1].as_u64().unwrap() as u8,
            hms[2].as_u64().unwrap() as u8,
        )
        .unwrap();
        let separator = case["separator"].as_str().unwrap().to_owned();
        let s = DemoSettings::new(Rc::new(move || Ok(timestamp(time, &separator))));
        let trace = Rc::new(RefCell::new(Vec::new()));
        let t = trace.clone();
        let weak = Rc::downgrade(&s);
        let _sub = s.logged(move |message| {
            t.borrow_mut()
                .push(json!({"message":message,"state":state(&weak.upgrade().unwrap())}))
        });
        s.write(case["message"].as_str()).unwrap();
        assert_eq!(json!(*trace.borrow()), case["trace"]);
        assert_eq!(state(&s), case["state"]);
        count += 1;
    }
    assert_eq!(count, 48);
}

#[test]
fn original_shared_list_identity_and_single_tail_removal_match_external_mutations() {
    for case in fixture()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["kind"] == "limit")
    {
        let s = settings();
        let log = s.log();
        assert!(Rc::ptr_eq(&log, &s.log()));
        log.borrow_mut()
            .extend((0..case["initial"].as_u64().unwrap()).map(|i| format!("seed:{i}")));
        let trace = Rc::new(RefCell::new(Vec::new()));
        let t = trace.clone();
        let weak = Rc::downgrade(&s);
        let _sub = s.logged(move |message| {
            let log = weak.upgrade().unwrap().log();
            let log = log.borrow();
            t.borrow_mut()
                .push(json!({"message":message,"count":log.len(),"first":log[0]}));
        });
        s.write(Some("one")).unwrap();
        s.write(Some("two")).unwrap();
        assert_eq!(json!(*trace.borrow()), case["trace"]);
        assert_eq!(state(&s), case["state"]);
    }
}

#[test]
fn original_reentrant_log_snapshot_and_subscriber_removal_order_match() {
    let cases = fixture();
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["kind"] == "reentry")
        .unwrap();
    let s = settings();
    let trace = Rc::new(RefCell::new(Vec::new()));
    let b = Rc::new(RefCell::new(None::<DemoSubscription>));
    let c = Rc::new(RefCell::new(None::<DemoSubscription>));
    let (weak, t, remove, add) = (Rc::downgrade(&s), trace.clone(), b.clone(), c.clone());
    let _a = s.logged(move |m| {
        t.borrow_mut()
            .push(format!("A:{}", m.as_deref().unwrap_or_default()));
        if m.as_deref() == Some("outer") {
            remove.borrow_mut().take();
            let t = t.clone();
            let s = weak.upgrade().unwrap();
            *add.borrow_mut() = Some(
                s.logged(move |m| t.borrow_mut().push(format!("C:{}", m.unwrap_or_default()))),
            );
            s.write(Some("inner")).unwrap();
        }
    });
    let t = trace.clone();
    *b.borrow_mut() =
        Some(s.logged(move |m| t.borrow_mut().push(format!("B:{}", m.unwrap_or_default()))));
    s.write(Some("outer")).unwrap();
    s.write(Some("later")).unwrap();
    assert_eq!(json!(*trace.borrow()), case["trace"]);
    assert_eq!(state(&s), case["state"]);
}

#[test]
fn original_notification_failure_keeps_log_mutation_and_stops_later_subscribers() {
    let cases = fixture();
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["kind"] == "throw")
        .unwrap();
    let s = settings();
    let trace = Rc::new(RefCell::new(Vec::new()));
    let t = trace.clone();
    let _first = s.logged(move |m| {
        t.borrow_mut()
            .push(format!("first:{}", m.unwrap_or_default()));
        panic!("stop")
    });
    let t = trace.clone();
    let _second = s.logged(move |m| {
        t.borrow_mut()
            .push(format!("second:{}", m.unwrap_or_default()))
    });
    assert!(catch_unwind(AssertUnwindSafe(|| s.write(Some("fail")))).is_err());
    assert_eq!(json!(*trace.borrow()), case["trace"]);
    assert_eq!(state(&s), case["state"]);
}

#[test]
fn clock_errors_and_raii_subscription_cleanup_do_not_publish_false_success() {
    let s = DemoSettings::new(Rc::new(|| Err(time::error::IndeterminateOffset)));
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let subscription = s.logged(move |_| count.set(count.get() + 1));
    assert!(s.write(Some("failure")).is_err());
    assert!(s.log().borrow().is_empty());
    assert_eq!(calls.get(), 0);
    subscription.unsubscribe();
    subscription.unsubscribe();
    drop(subscription);
    let s = settings();
    let target = Rc::new(Cell::new(0));
    let weak = Rc::downgrade(&target);
    let count = target.clone();
    let sub = s.logged(move |_| count.set(count.get() + 1));
    drop(target);
    assert!(weak.upgrade().is_some());
    drop(sub);
    assert!(weak.upgrade().is_none());
    s.write(Some("after disposal")).unwrap();
}

#[test]
fn original_surface_style_order_equality_reentry_and_failure_match_native_theme() {
    run(|| {
        for case in fixture()["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["kind"] == "style")
        {
            RibbonTheme::apply_style(RibbonThemeStyle::Office).unwrap();
            let s = settings();
            let trace = Rc::new(RefCell::new(Vec::new()));
            let (t, weak) = (trace.clone(), Rc::downgrade(&s));
            let theme = RibbonTheme::changed(move |_, _| {
                let s = weak.upgrade().unwrap();
                t.borrow_mut().push(
                    json!({"call":"theme","style":s.surface_style().raw(),"state":state(&s)}),
                );
            });
            let (t, weak) = (trace.clone(), Rc::downgrade(&s));
            let _surface = s.surface_style_changed(move || {
                let s = weak.upgrade().unwrap();
                t.borrow_mut().push(
                    json!({"call":"surface","style":s.surface_style().raw(),"state":state(&s)}),
                );
            });
            for _ in 0..2 {
                s.set_surface_style(RibbonThemeStyle::from_raw(
                    case["style"].as_i64().unwrap() as i32
                ))
                .unwrap();
            }
            theme.dispose();
            assert_eq!(json!(*trace.borrow()), case["trace"]);
            assert_eq!(state(&s), case["state"]);
        }
        for kind in ["style-reentry", "style-throw"] {
            let cases = fixture();
            let case = cases["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["kind"] == kind)
                .unwrap();
            RibbonTheme::apply_style(RibbonThemeStyle::Office).unwrap();
            let s = settings();
            let trace = Rc::new(RefCell::new(Vec::new()));
            let (t, weak) = (trace.clone(), Rc::downgrade(&s));
            let theme = RibbonTheme::changed(move |_, _| {
                let s = weak.upgrade().unwrap();
                t.borrow_mut()
                    .push(format!("theme:{}", s.surface_style().raw()));
                if kind == "style-throw" {
                    panic!("stop")
                }
                if s.surface_style() == RibbonThemeStyle::Cad {
                    s.set_surface_style(RibbonThemeStyle::Office).unwrap();
                }
            });
            let (t, weak) = (trace.clone(), Rc::downgrade(&s));
            let _surface = s.surface_style_changed(move || {
                t.borrow_mut().push(format!(
                    "surface:{}",
                    weak.upgrade().unwrap().surface_style().raw()
                ))
            });
            let result = catch_unwind(AssertUnwindSafe(|| {
                s.set_surface_style(RibbonThemeStyle::Cad)
            }));
            theme.dispose();
            assert_eq!(result.is_err(), kind == "style-throw");
            assert_eq!(json!(*trace.borrow()), case["trace"]);
            assert_eq!(state(&s), case["state"]);
        }
    });
}

#[test]
fn source_palette_and_word_ribbon_references_retain_identity_until_replaced() {
    run(|| {
        let s = settings();
        let ribbon = Ribbon::new();
        let weak = ribbon.downgrade();
        let palette = RibbonThemePalette::word();
        s.set_palette_override(Some(palette.clone()));
        assert!(Rc::ptr_eq(&s.palette_override().unwrap(), &palette));
        s.set_word_ribbon(Some(ribbon.clone()));
        assert!(ribbon.ptr_eq(&s.word_ribbon().unwrap()));
        drop(ribbon);
        assert!(weak.upgrade().is_some());
        s.set_word_ribbon(None);
        assert!(weak.upgrade().is_none());
        let log = s.log();
        drop(s);
        assert!(log.borrow().is_empty());
    });
}

#[cfg(unix)]
#[test]
fn native_local_clock_matches_source_summer_winter_and_fractional_zones() {
    for zone in ["UTC", "America/New_York", "Asia/Kathmandu", "Europe/Lisbon"] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "demo_settings::tests::local_clock_zone_child",
                "--ignored",
                "--nocapture",
            ])
            .env("TZ", zone)
            .env("FERRORIBBON_CLOCK_ZONE_CHILD", zone)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{zone}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
#[cfg(unix)]
#[test]
#[ignore = "Child of native_local_clock test; each timezone requires its own process"]
fn local_clock_zone_child() {
    use ferroui_base::utilities::CultureInfo;
    let zone =
        std::env::var("FERRORIBBON_CLOCK_ZONE_CHILD").expect("run through native_local_clock test");
    CultureInfo::set_current_formatting(CultureInfo::invariant_culture());
    let mut count = 0;
    for case in fixture()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["kind"] == "zone" && c["zone"] == zone)
    {
        let utc =
            time::OffsetDateTime::from_unix_timestamp(case["unix"].as_i64().unwrap()).unwrap();
        assert_eq!(
            local_timestamp_at(utc).unwrap(),
            case["stamp"].as_str().unwrap()
        );
        let s = DemoSettings::new(Rc::new(move || local_timestamp_at(utc)));
        s.write(Some(&zone)).unwrap();
        assert_eq!(state(&s), case["state"]);
        count += 1;
    }
    assert_eq!(count, 2);
    let mut info = DateTimeFormatInfo::new();
    info.set_time_separator("∶");
    CultureInfo::set_current_formatting(
        CultureInfo::invariant_culture().with_date_time_format(info),
    );
    assert_eq!(local_timestamp().unwrap().matches('∶').count(), 2);
}

#[cfg(feature = "desktop")]
#[test]
fn actual_cad_pages_share_the_source_log_and_release_injected_services() {
    use ferroui_controls::{AppBuilder, Application};
    use ferroui_desktop::AppBuilderDesktopExtensions;
    use ferroui_headless::{
        FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
    };
    let session = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>()
            .use_platform_detect()
            .use_headless(FerroHeadlessPlatformOptions {
                use_headless_drawing: false,
                ..Default::default()
            })
    });
    let result = session
        .dispatch(
            || {
                crate::register_types();
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                crate::install_application_resources();
                assert!(Rc::ptr_eq(
                    &DemoSettings::current(),
                    &DemoSettings::current()
                ));
                let s = settings();
                let weak_service = Rc::downgrade(&s);
                let log = s.log();
                let events = Rc::new(RefCell::new(Vec::new()));
                let e = events.clone();
                let _sub = s.logged(move |message| e.borrow_mut().push(message));
                let first = crate::pages::CadPage::new_with_settings(s.clone());
                let second = crate::pages::CadPage::new_with_settings(s.clone());
                let weak_first = first.downgrade();
                let weak_model = Rc::downgrade(&first.view_model());
                first.run_command("line");
                second.view_model().echo("shared from second", false);
                assert_eq!(
                    *events.borrow(),
                    [
                        Some("CAD · Command: _LINE".into()),
                        Some("CAD · Specify first point:".into()),
                        Some("CAD · shared from second".into())
                    ]
                );
                assert_eq!(
                    *log.borrow(),
                    [
                        "03:04:05  CAD · shared from second",
                        "03:04:05  CAD · Specify first point:",
                        "03:04:05  CAD · Command: _LINE"
                    ]
                );
                drop(first);
                ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
                assert!(weak_first.upgrade().is_none());
                assert!(weak_model.upgrade().is_none());
                drop(s);
                assert!(weak_service.upgrade().is_some());
                drop(second);
                ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
                assert!(weak_service.upgrade().is_none());
                assert_eq!(log.borrow().len(), 3);
            },
            ferroui_base::threading::CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}
