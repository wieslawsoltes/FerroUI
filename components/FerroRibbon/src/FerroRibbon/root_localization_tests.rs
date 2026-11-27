//! Root string refresh, real template parts, dispatcher and subscription lifetime.
use crate::controls::{Ribbon, RibbonGroup, RibbonTab};
use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::localization::{RibbonCulture, RibbonStrings};
use ferroui_base::{Ref, threading::Dispatcher};
use ferroui_controls::{
    Button, Control, ControlImpl, Panel, ToolTip,
    automation::{AutomationProperties, peers::ControlAutomationPeer},
    templates::FuncControlTemplate,
};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn isolated(name: &str) -> bool {
    // libtest names omit the crate segment that module_path! includes.
    let name = name
        .strip_prefix(concat!(env!("CARGO_CRATE_NAME"), "::"))
        .unwrap_or(name);
    if std::env::var("FERRORIBBON_ROOT_LOCALIZATION_CASE").as_deref() == Ok(name) {
        return false;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env("FERRORIBBON_ROOT_LOCALIZATION_CASE", name)
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("running 1 test\n"),
        "isolated case did not execute exactly one test: {name}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    true
}
fn strings(name: &str) -> std::sync::Arc<RibbonStrings> {
    RibbonStrings::for_culture(Some(RibbonCulture::new(name.into(), name.into()))).unwrap()
}
fn tip(button: &Button) -> Option<String> {
    ToolTip::get_tip(button).and_then(|v| v.downcast_ref::<String>().cloned())
}
fn name(control: &Control) -> String {
    ControlAutomationPeer::create_peer_for_element(control).get_name()
}
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../tests/fixtures/root-localization.json")).unwrap()
}
fn case(name: &str) -> Value {
    corpus()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == name)
        .unwrap()
        .clone()
}
fn setup() -> (Ref<Ribbon>, Ref<RibbonTab>, Ref<ferroui_controls::Window>) {
    RibbonStrings::set_current(Some(strings("en"))).unwrap();
    let root = Ribbon::new();
    let tab = RibbonTab::new();
    tab.set_header(Some("Home".into()));
    root.tabs().add(Some(tab.clone()));
    let window = crate::ribbon_tests::mount(&root);
    assert!(root.is_loaded());
    (root, tab, window)
}
fn snapshot(root: &Ribbon, tab: &RibbonTab) -> Value {
    json!({"name":root.display_options_button().and_then(|b| AutomationProperties::get_name(&b)),
        "tip":root.display_options_button().and_then(|b| tip(&b)),
        "label":tab.overflow_button().label(),
        "overflowTip":tip(&tab.overflow_button())})
}
fn change(culture: &str) {
    RibbonStrings::set_current(Some(strings(culture))).unwrap();
    Dispatcher::ui_thread().run_jobs(None);
}
fn close(window: &ferroui_controls::Window) {
    window.close();
    window.set_content(None);
    Dispatcher::ui_thread().run_jobs(None);
}
#[test]
fn root_options_name_and_tooltip_follow_current_strings_through_native_template() {
    const NAME: &str = concat!(
        module_path!(),
        "::root_options_name_and_tooltip_follow_current_strings_through_native_template"
    );
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        RibbonStrings::set_current(Some(strings("en"))).unwrap();
        let root = Ribbon::new();
        let tab = RibbonTab::new();
        tab.set_header(Some("Home".into()));
        let group = RibbonGroup::new();
        group.set_header(Some("Tools".into()));
        group.set_is_dialog_launcher_visible(true);
        tab.groups().add(Some(group.clone()));
        root.tabs().add(Some(tab.clone()));
        let window = crate::ribbon_tests::mount(&root);
        group.set_is_group_visible(false);
        let options = root.display_options_button().unwrap();
        assert_eq!(name(&options), "Ribbon display options");
        assert_eq!(tip(&options).as_deref(), Some("Ribbon display options"));
        let french = strings("fr");
        let expected = french.ribbon_display_options().unwrap().to_utf8().unwrap();
        RibbonStrings::set_current(Some(french)).unwrap();
        assert_eq!(
            name(&options),
            "Ribbon display options",
            "Source refresh is deferred"
        );
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(name(&options), expected);
        assert_eq!(tip(&options).as_deref(), Some(expected.as_str()));
        assert_eq!(
            tab.overflow_button().label(),
            RibbonStrings::current()
                .more_options()
                .map(|v| v.to_utf8().unwrap())
        );
        let expected = RibbonStrings::current()
            .format(
                Some(&"DialogLauncher".into()),
                Some(RibbonFixedItems::new(vec!["Tools".into()])),
            )
            .unwrap()
            .to_utf8()
            .unwrap();
        assert_eq!(name(&group.dialog_launcher().unwrap()), expected);
        window.close();
        window.set_content(None);
    });
}

#[test]
fn original_refresh_cases_match_native_parts_for_cultures_overrides_and_latest_current() {
    if isolated(concat!(
        module_path!(),
        "::original_refresh_cases_match_native_parts_for_cultures_overrides_and_latest_current"
    )) {
        return;
    }
    crate::group_tests::run(|| {
        for culture in ["en", "fr", "de", "pl", "es"] {
            let (root, tab, window) = setup();
            change(culture);
            assert_eq!(
                snapshot(&root, &tab),
                case(&format!("culture:{culture}"))["state"]
            );
            close(&window);
        }
        for value in ["", "e\u{301}👩‍💻", "العربية"] {
            let (root, tab, window) = setup();
            for key in ["RibbonDisplayOptions", "MoreOptions"] {
                RibbonStrings::current()
                    .override_string(Some(key.into()), Some(value.into()))
                    .unwrap();
            }
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(
                snapshot(&root, &tab),
                case(&format!("override:{value}"))["state"]
            );
            close(&window);
        }
        let (root, tab, window) = setup();
        RibbonStrings::set_current(Some(strings("fr"))).unwrap();
        let queued = snapshot(&root, &tab);
        RibbonStrings::set_current(Some(strings("de"))).unwrap();
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            json!([queued, snapshot(&root, &tab)]),
            case("latest-current")["states"]
        );
        close(&window);
    });
}

#[test]
fn unloaded_roots_stop_notifications_and_queued_work_survives_detach() {
    if isolated(concat!(
        module_path!(),
        "::unloaded_roots_stop_notifications_and_queued_work_survives_detach"
    )) {
        return;
    }
    crate::group_tests::run(|| {
        let event = RibbonStrings::current_changed();
        let baseline = event.handler_count();
        let (root, tab, window) = setup();
        assert_eq!(event.handler_count(), baseline + 1);
        // Repeated Loaded replaces the same handler, rather than stacking it.
        <Ribbon as ControlImpl>::on_loaded(
            &root,
            &ferroui_base::interactivity::RoutedEventArgs::with_event(Control::loaded_event()),
        );
        assert_eq!(event.handler_count(), baseline + 1);
        change("fr");
        assert_eq!(snapshot(&root, &tab), case("duplicate-load")["state"]);
        change("en");
        RibbonStrings::set_current(Some(strings("fr"))).unwrap();
        window.set_content(None);
        assert!(!root.is_loaded());
        assert_eq!(event.handler_count(), baseline);
        Dispatcher::ui_thread().run_jobs(None);
        let queued = snapshot(&root, &tab);
        change("de");
        let unloaded = snapshot(&root, &tab);
        window.set_content(Some(Control::boxed(root.clone())));
        Dispatcher::ui_thread().run_jobs(None);
        assert!(root.is_loaded());
        assert_eq!(event.handler_count(), baseline + 1);
        let reloaded = snapshot(&root, &tab);
        change("es");
        assert_eq!(
            json!([queued, unloaded, reloaded, snapshot(&root, &tab)]),
            case("unload-reload")["states"]
        );
        close(&window);
        assert_eq!(event.handler_count(), baseline);
        let weak = root.downgrade();
        drop(root);
        drop(tab);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(weak.upgrade().is_none());

        let (root, tab, window) = setup();
        let weak = root.downgrade();
        RibbonStrings::set_current(Some(strings("fr"))).unwrap();
        window.set_content(None);
        drop(root);
        drop(tab);
        assert!(
            weak.upgrade().is_some(),
            "pending source refresh retains its receiver"
        );
        Dispatcher::ui_thread().run_jobs(None);
        assert!(
            weak.upgrade().is_none(),
            "queue drains and releases unloaded root"
        );
        assert_eq!(event.handler_count(), baseline);
        close(&window);
    });
}

#[test]
fn worker_changes_refresh_native_parts_only_on_the_owner_dispatcher() {
    if isolated(concat!(
        module_path!(),
        "::worker_changes_refresh_native_parts_only_on_the_owner_dispatcher"
    )) {
        return;
    }
    crate::group_tests::run(|| {
        let (root, tab, window) = setup();
        let group = RibbonGroup::new();
        group.set_header(Some("Tools".into()));
        group.set_is_dialog_launcher_visible(true);
        tab.groups().add(Some(group.clone()));
        Dispatcher::ui_thread().run_jobs(None);
        let before = snapshot(&root, &tab);
        let owner = std::thread::current().id();
        let notifications = Rc::new(RefCell::new(Vec::new()));
        let seen = notifications.clone();
        let options = root.display_options_button().unwrap();
        let token = options.add_property_changed_handler(Rc::new(move |args| {
            if args.property() == AutomationProperties::name_property().as_property() {
                seen.borrow_mut().push(std::thread::current().id());
            }
        }));
        std::thread::spawn(|| RibbonStrings::set_current(Some(strings("fr"))).unwrap())
            .join()
            .unwrap();
        assert_eq!(
            snapshot(&root, &tab),
            before,
            "worker only queues the source refresh"
        );
        assert!(notifications.borrow().is_empty());
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(snapshot(&root, &tab), case("culture:fr")["state"]);
        assert_eq!(*notifications.borrow(), vec![owner]);
        assert_eq!(
            name(&group.dialog_launcher().unwrap()),
            RibbonStrings::current()
                .format(
                    Some(&"DialogLauncher".into()),
                    Some(RibbonFixedItems::new(vec!["Tools".into()]))
                )
                .unwrap()
                .to_utf8()
                .unwrap()
        );
        options.remove_property_changed_handler(token);
        close(&window);
    });
}

#[test]
fn worker_event_snapshot_retains_unloaded_receiver_until_owner_queue_drains() {
    if isolated(concat!(
        module_path!(),
        "::worker_event_snapshot_retains_unloaded_receiver_until_owner_queue_drains"
    )) {
        return;
    }
    crate::group_tests::run(|| {
        use std::{
            sync::{Arc, Mutex, mpsc},
            time::Duration,
        };
        RibbonStrings::set_current(Some(strings("en"))).unwrap();
        let event = RibbonStrings::current_changed();
        let baseline = event.handler_count();
        let receivers = super::RECEIVERS.with(|r| r.borrow().len());
        let (entered, entry) = mpsc::channel();
        let (release, released) = mpsc::channel();
        let released = Mutex::new(released);
        let token = event.add(Arc::new(move |_| {
            entered.send(()).unwrap();
            released
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
        }));
        // setup assigns Current, so mount after installing the blocker without setup.
        let root = Ribbon::new();
        let tab = RibbonTab::new();
        root.tabs().add(Some(tab.clone()));
        let window = crate::ribbon_tests::mount(&root);
        let weak = root.downgrade();
        let worker =
            std::thread::spawn(|| RibbonStrings::set_current(Some(strings("fr"))).unwrap());
        entry.recv_timeout(Duration::from_secs(5)).unwrap();
        window.set_content(None);
        drop(root);
        drop(tab);
        assert!(
            weak.upgrade().is_some(),
            "captured source event snapshot still owns receiver link"
        );
        assert_eq!(event.handler_count(), baseline + 1);
        event.remove(token);
        release.send(()).unwrap();
        worker.join().unwrap();
        assert!(
            weak.upgrade().is_some(),
            "posted refresh retains the actual unloaded native root"
        );
        Dispatcher::ui_thread().run_jobs(None);
        assert!(weak.upgrade().is_none());
        assert_eq!(super::RECEIVERS.with(|r| r.borrow().len()), receivers);
        assert_eq!(event.handler_count(), baseline);
        close(&window);
    });
}

#[test]
fn refresh_rereads_current_and_template_receiver_after_tooltip_notifications() {
    if isolated(concat!(
        module_path!(),
        "::refresh_rereads_current_and_template_receiver_after_tooltip_notifications"
    )) {
        return;
    }
    crate::group_tests::run(|| {
        let (root, tab, window) = setup();
        let options = root.display_options_button().unwrap();
        let first = Rc::new(Cell::new(true));
        let once = first.clone();
        let token = options.add_property_changed_handler(Rc::new(move |e| {
            if e.property() == ToolTip::tip_property().as_property() && once.replace(false) {
                RibbonStrings::set_current(Some(strings("de"))).unwrap();
            }
        }));
        let during = Rc::new(RefCell::new(None));
        let seen = during.clone();
        let weak = root.downgrade();
        let weak_tab = tab.downgrade();
        let overflow = tab.overflow_button();
        let overflow_token = overflow.add_property_changed_handler(Rc::new(move |e| {
            if e.property() == ToolTip::tip_property().as_property() {
                *seen.borrow_mut() = Some(snapshot(
                    &weak.upgrade().unwrap(),
                    &weak_tab.upgrade().unwrap(),
                ));
            }
        }));
        change("fr");
        assert_eq!(
            json!([during.borrow().clone().unwrap(), snapshot(&root, &tab)]),
            case("tip-reentry")["states"]
        );
        options.remove_property_changed_handler(token);
        overflow.remove_property_changed_handler(overflow_token);
        close(&window);

        let (root, tab, window) = setup();
        let original = root.display_options_button().unwrap();
        let replacement = Button::new();
        let next = replacement.clone();
        let weak = root.downgrade();
        // Replace only the current part field inside the notification. This
        // isolates the exact receiver re-read; real template lifecycle has its
        // own tests and must not be inferred from this observation seam.
        let token = original.add_property_changed_handler(Rc::new(move |e| {
            if e.property() == ToolTip::tip_property().as_property() {
                weak.upgrade().unwrap().parts.borrow_mut().options = Some(next.clone());
            }
        }));
        change("fr");
        let c = case("part-reentry");
        assert_eq!(snapshot(&root, &tab), c["state"]);
        assert_eq!(
            json!({"name":AutomationProperties::get_name(&original),"tip":tip(&original)}),
            c["original"]
        );
        original.remove_property_changed_handler(token);
        close(&window);
    });
}

#[test]
fn missing_options_template_still_refreshes_tab() {
    if isolated(concat!(
        module_path!(),
        "::missing_options_template_still_refreshes_tab"
    )) {
        return;
    }
    crate::group_tests::run(|| {
        let (root, tab, window) = setup();
        root.set_template(Some(FuncControlTemplate::for_type::<Ribbon>(|_, _| {
            Panel::new().upcast()
        })));
        root.apply_template();
        assert!(root.display_options_button().is_none());
        change("fr");
        assert_eq!(snapshot(&root, &tab), case("missing-part")["state"]);
        close(&window);
    });
}

#[test]
fn source_localization_bookkeeping_precedes_application_lifecycle_handlers() {
    if isolated(concat!(
        module_path!(),
        "::source_localization_bookkeeping_precedes_application_lifecycle_handlers"
    )) {
        return;
    }
    crate::group_tests::run(|| {
        RibbonStrings::set_current(Some(strings("en"))).unwrap();
        let event = RibbonStrings::current_changed();
        let baseline = event.handler_count();
        let root = Ribbon::new();
        let tab = RibbonTab::new();
        tab.set_header(Some("Home".into()));
        root.tabs().add(Some(tab.clone()));
        let observed = Rc::new(RefCell::new(Vec::new()));
        let trace = observed.clone();
        let events = event.clone();
        let loaded = root.loaded(move |_, _| {
            trace.borrow_mut().push(events.handler_count() - baseline);
            RibbonStrings::set_current(Some(strings("fr"))).unwrap();
        });
        let trace = observed.clone();
        let events = event.clone();
        let unloaded = root.unloaded(move |_, _| {
            trace.borrow_mut().push(events.handler_count() - baseline);
            RibbonStrings::set_current(Some(strings("de"))).unwrap();
        });
        let window = crate::ribbon_tests::mount(&root);
        assert_eq!(
            *observed.borrow(),
            [1],
            "source constructor installs the ribbon's handler before the application's Loaded handler"
        );
        let loaded_state = snapshot(&root, &tab);
        window.set_content(None);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            *observed.borrow(),
            [1, 0],
            "source unload bookkeeping precedes application Unloaded handlers"
        );
        assert_eq!(
            json!([loaded_state, snapshot(&root, &tab)]),
            case("lifecycle-order")["states"]
        );
        root.remove_handler(Control::loaded_event(), loaded);
        root.remove_handler(Control::unloaded_event(), unloaded);
        close(&window);
    });
}
