//! Source QuickAccessTests and native ownership/layout/routing extensions.
use super::*;
use crate::controls::{
    IRibbonLayoutHost, RibbonButton, RibbonGroup, RibbonSplitButton, RibbonToggleButton,
};
use ferroui_base::{
    Rect, Size,
    data::core::{ValueType, ValueTypes},
    input::{InputElement, Key, KeyEventArgs, KeyModifiers},
    threading::Dispatcher,
};
use ferroui_controls::{
    Window,
    automation::peers::{AutomationControlType, ControlAutomationPeer},
};
fn isolated(name: &str) -> bool {
    if std::env::var("FERRORIBBON_QAT_ISOLATED").as_deref() == Ok(name) {
        return false;
    }
    let o = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env("FERRORIBBON_QAT_ISOLATED", name)
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    true
}
fn settle() {
    Dispatcher::ui_thread().run_jobs(None);
}
fn layout(w: &Window) {
    w.measure(Size::new(1400., 400.));
    w.arrange(Rect::new(0., 0., 1400., 400.));
    settle();
}
fn setup() -> (
    Ref<Ribbon>,
    Ref<Window>,
    Ref<RibbonButton>,
    Ref<RibbonToggleButton>,
) {
    let r = Ribbon::new();
    r.set_is_application_button_visible(false);
    r.set_is_display_options_button_visible(false);
    r.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Top);
    let tab = RibbonTab::new();
    tab.set_id(Some("home".into()));
    tab.set_header(Some("Home".into()));
    tab.set_key_tip(Some("H".into()));
    let group = RibbonGroup::new();
    group.set_header(Some("Tools".into()));
    let cut = RibbonButton::new();
    cut.set_id(Some("cut".into()));
    cut.set_label(Some("Cut".into()));
    let bold = RibbonToggleButton::new();
    bold.set_id(Some("bold".into()));
    bold.set_label(Some("Bold".into()));
    group.items().add(Some(cut.clone().upcast()));
    group.items().add(Some(bold.clone().upcast()));
    tab.groups().add(Some(group));
    r.tabs().add(Some(tab));
    let save = RibbonButton::new();
    save.set_id(Some("save".into()));
    save.set_label(Some("Save".into()));
    r.quick_access_tool_bar()
        .unwrap()
        .items()
        .add(Some(save.upcast()));
    let w = crate::ribbon_tests::mount(&r);
    for name in ["Checked", "Split"] {
        let value = crate::compiled_shared::try_load(
            None,
            &format!("ferres://FerroRibbon/Themes/{name}.xaml"),
        )
        .unwrap()
        .unwrap();
        r.resources().add_merged_dictionary(
            ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<ferroui_base::controls::ResourceDictionary>()
                .unwrap(),
        );
    }
    layout(&w);
    (r, w, cut, bold)
}
fn ids(r: &Ribbon) -> Vec<String> {
    r.get_quick_access_item_ids()
        .iter()
        .map(|s| s.unwrap().to_utf8().unwrap())
        .collect()
}
fn send(c: &Control, key: Key, mods: KeyModifiers) -> bool {
    let mut e = KeyEventArgs::new();
    e.key = key;
    e.key_modifiers = mods;
    e.set_routed_event(Some(InputElement::key_down_event()));
    c.raise_event(&e);
    e.handled()
}
#[test]
fn linked_qat_copies_track_source_group_enablement_until_unlinked() {
    crate::group_tests::run(|| {
        let split = RibbonSplitButton::new();
        split.set_id(Some("paste".into()));
        split.set_label(Some("Paste".into()));
        let button = RibbonButton::new();
        button.set_id(Some("copy".into()));
        button.set_label(Some("Copy".into()));
        for source in [split.upcast::<Control>(), button.upcast()] {
            check_linked_qat_enablement(source);
        }
    });
}

fn check_linked_qat_enablement(source: Ref<Control>) {
    let ribbon = Ribbon::new();
    let tab = RibbonTab::new();
    let group = RibbonGroup::new();
    group.items().add(Some(source.clone()));
    tab.groups().add(Some(group.clone()));
    ribbon.tabs().add(Some(tab));
    let window = crate::ribbon_tests::mount(&ribbon);

    assert!(ribbon.add_to_quick_access(&source));
    layout(&window);
    let copy = ribbon.find_quick_access_copy(Some(source.clone())).unwrap();
    assert!(copy.visual_parent().is_some());
    assert!(copy.is_effectively_enabled());

    group.set_is_enabled(false);
    assert!(!source.is_effectively_enabled());
    assert!(!copy.is_effectively_enabled(), "existing QAT copy");
    assert!(!as_item(&copy).unwrap().invoke());
    let detached_copy = as_item(&source).unwrap().create_linked_copy().unwrap();
    assert!(
        !detached_copy.is_enabled(),
        "copy created under a disabled group"
    );

    copy.set_is_enabled(true);
    assert!(copy.is_effectively_enabled());
    assert!(!source.is_effectively_enabled(), "enabled link is one way");
    source.set_is_enabled(false);
    assert!(
        !copy.is_effectively_enabled(),
        "local write under disabled group"
    );
    copy.set_is_enabled(true);
    assert!(copy.is_effectively_enabled());
    source.clear_value(InputElement::is_enabled_property());
    assert_eq!(
        source.get_local_value(InputElement::is_enabled_property()),
        None
    );
    assert!(!source.is_effectively_enabled());
    assert!(
        !copy.is_effectively_enabled(),
        "clearing the local enabled value under a disabled group"
    );
    source.set_is_enabled(true);
    assert!(!copy.is_effectively_enabled());

    group.set_is_enabled(true);
    assert!(source.is_effectively_enabled());
    assert!(copy.is_effectively_enabled());
    assert!(detached_copy.is_enabled());
    assert!(as_item(&copy).unwrap().invoke());
    source.set_is_enabled(false);
    assert!(!copy.is_effectively_enabled());
    assert!(!detached_copy.is_enabled());

    assert!(ribbon.remove_from_quick_access(&source));
    assert!(
        ribbon
            .find_quick_access_copy(Some(source.clone()))
            .is_none()
    );
    source.set_is_enabled(true);
    assert!(!copy.is_enabled(), "removed QAT copy was unlinked");
    assert!(detached_copy.is_enabled());
    assert_eq!(RibbonItemHelper::get_source_item(&copy), Some(source));
    RibbonItemHelper::unlink(&detached_copy);
    window.close();
}

#[test]
fn upstream_add_remove_links_both_directions_and_preserves_removed_source_reference() {
    const NAME: &str = "controls::ribbon::quick_access::tests::upstream_add_remove_links_both_directions_and_preserves_removed_source_reference";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w, _, bold) = setup();
        assert!(r.add_to_quick_access(&bold));
        assert!(!r.add_to_quick_access(&bold));
        settle();
        let copy = r
            .find_quick_access_copy(Some(bold.clone().upcast()))
            .unwrap()
            .cast::<RibbonToggleButton>()
            .unwrap();
        copy.set_is_checked(Some(true));
        assert_eq!(bold.is_checked(), Some(true));
        bold.set_is_checked(Some(false));
        assert_eq!(copy.is_checked(), Some(false));
        assert_eq!(ids(&r), ["save", "bold"]);
        assert!(r.is_in_quick_access(&copy));
        assert!(r.remove_from_quick_access(&bold));
        assert_eq!(ids(&r), ["save"]);
        bold.set_is_checked(Some(true));
        bold.set_label(Some("Strong".into()));
        assert_eq!(copy.is_checked(), Some(false));
        assert_eq!(copy.label().as_deref(), Some("Bold"));
        assert_eq!(
            RibbonItemHelper::get_source_item(&copy),
            Some(bold.upcast())
        );
        w.close();
        w.set_content(None);
        settle();
    });
}
#[test]
fn upstream_copy_clicks_original_once_and_owner_command_state_includes_qat() {
    const NAME: &str = "controls::ribbon::quick_access::tests::upstream_copy_clicks_original_once_and_owner_command_state_includes_qat";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w, cut, _) = setup();
        let clicks = Rc::new(Cell::new(0));
        let n = clicks.clone();
        cut.click(move |_, _| n.set(n.get() + 1));
        let invoked = Rc::new(Cell::new(0));
        let n = invoked.clone();
        r.add_item_invoked(Some(crate::root_command_tests::invocation(move |e| {
            if e.item_id == Some("cut".into()) {
                n.set(n.get() + 1);
            }
        })));
        let commands = Rc::new(Cell::new(0));
        let n = commands.clone();
        let cmd = ferroribbon_core::commands::RibbonRelayCommand::new(move || n.set(n.get() + 1));
        cut.set_command(Some(crate::commands::adapt_core_command(cmd)));
        cut.set_command_id(Some("cutCommand".into()));
        assert!(r.add_to_quick_access(&cut));
        settle();
        let copy = r
            .find_quick_access_copy(Some(cut.clone().upcast()))
            .unwrap()
            .cast::<RibbonButton>()
            .unwrap();
        copy.perform_click();
        assert_eq!((clicks.get(), invoked.get(), commands.get()), (1, 1, 1));
        assert_eq!(
            copy.command_id(),
            None,
            "source LinkCommon does not copy CommandId"
        );
        assert_eq!(
            r.find_items_by_command(Some("cutCommand".into()))
                .iter()
                .count(),
            1
        );
        let own = RibbonButton::new();
        own.set_command_id(Some("cutCommand".into()));
        r.quick_access_tool_bar()
            .unwrap()
            .items()
            .add(Some(own.clone().upcast()));
        assert_eq!(
            r.find_items_by_command(Some("cutCommand".into()))
                .iter()
                .count(),
            2
        );
        r.set_command_enabled(Some("cutCommand".into()), false);
        assert!(!cut.is_enabled());
        assert!(!copy.is_enabled());
        r.set_command_enabled(Some("cutCommand".into()), true);
        r.quick_access_tool_bar().unwrap().items().clear();
        w.close();
        w.set_content(None);
        settle();
    });
}
#[test]
fn upstream_hosts_visibility_external_parent_and_first_load_defaults_keep_identity() {
    const NAME: &str = "controls::ribbon::quick_access::tests::upstream_hosts_visibility_external_parent_and_first_load_defaults_keep_identity";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w, _, _) = setup();
        let q = r.quick_access_tool_bar().unwrap();
        let above = r.parts.borrow().quick_access_above.clone().unwrap();
        let below = r.parts.borrow().quick_access_below.clone().unwrap();
        assert!(above.child().unwrap().ptr_eq(&q));
        let height = r.desired_size().height;
        let defaults = r.default_quick_access_item_ids().unwrap();
        assert_eq!(defaults.get(0), Some("save".into()));
        r.set_quick_access_position(RibbonQuickAccessPosition::BelowRibbon);
        layout(&w);
        assert!(below.child().unwrap().ptr_eq(&q));
        assert!(q.get_visual_root().is_some());
        r.set_is_quick_access_visible(false);
        layout(&w);
        assert!(r.desired_size().height < height);
        assert!(q.get_visual_root().is_none());
        r.set_is_quick_access_visible(true);
        r.set_quick_access_position(RibbonQuickAccessPosition::AboveRibbon);
        r.set_is_quick_access_hosted_externally(true);
        let external = Border::new();
        external.set_child(q.clone());
        r.update_quick_access_placement();
        assert!(external.child().unwrap().ptr_eq(&q));
        let departed_host_peer = ControlAutomationPeer::create_peer_for_element(&external);
        let destination_peer = ControlAutomationPeer::create_peer_for_element(&below);
        let toolbar_peer = ControlAutomationPeer::create_peer_for_element(&q);
        assert_eq!(
            departed_host_peer.get_children().as_slice(),
            std::slice::from_ref(&toolbar_peer)
        );
        assert_eq!(toolbar_peer.get_parent(), Some(departed_host_peer.clone()));

        r.set_quick_access_position(RibbonQuickAccessPosition::BelowRibbon);
        assert!(external.child().is_none());
        assert!(below.child().unwrap().ptr_eq(&q));
        assert_eq!(
            destination_peer.get_children().as_slice(),
            std::slice::from_ref(&toolbar_peer)
        );
        assert!(departed_host_peer.get_children().is_empty());
        assert_eq!(toolbar_peer.get_parent(), Some(destination_peer));
        r.set_visibility_mode(RibbonVisibilityMode::FullScreen);
        assert!(q.get_visual_root().is_none());
        r.set_is_full_screen_revealed(true);
        assert!(q.get_visual_root().is_some());
        assert!(Rc::ptr_eq(
            &r.default_quick_access_item_ids().unwrap(),
            &defaults
        ));
        assert_eq!(
            r.default_quick_access_position(),
            RibbonQuickAccessPosition::AboveRibbon
        );
        w.close();
        w.set_content(None);
        settle();
    });
}
#[test]
fn native_qat_metrics_theme_peer_layout_contract_and_collection_identity() {
    const NAME: &str = "controls::ribbon::quick_access::tests::native_qat_metrics_theme_peer_layout_contract_and_collection_identity";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w, cut, _) = setup();
        let q = r.quick_access_tool_bar().unwrap();
        let items = q.items();
        assert!(q.items() == items);
        assert!(q.show_customize_button());
        assert!(!q.show_labels());
        assert!(q.customize_button().unwrap().is_visible());
        q.set_show_customize_button(false);
        assert!(!q.customize_button().unwrap().is_visible());
        r.add_to_quick_access(&cut);
        settle();
        let copy = r
            .find_quick_access_copy(Some(cut.upcast()))
            .unwrap()
            .cast::<RibbonButton>()
            .unwrap();
        assert_eq!(copy.current_size(), ferroribbon_core::RibbonItemSize::Small);
        assert!(copy.is_simplified());
        assert!(!copy.actual_show_label());
        r.set_show_quick_access_labels(true);
        assert!(copy.actual_show_label());
        r.set_density(ferroribbon_core::RibbonDensity::Touch);
        assert!(std::sync::Arc::ptr_eq(
            q.metrics().as_ref().unwrap(),
            &r.metrics()
        ));
        let host = ValueTypes::try_cast(
            &Control::boxed(q.clone()),
            ValueType::of::<Rc<dyn IRibbonLayoutHost>>(),
        )
        .unwrap();
        host.downcast_ref::<Rc<dyn IRibbonLayoutHost>>()
            .unwrap()
            .invalidate_items_layout();
        let peer = ControlAutomationPeer::create_peer_for_element(&q);
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::ToolBar
        );
        assert_eq!(peer.get_class_name(), "RibbonQuickAccessToolBar");
        assert!(!q.customize_button().unwrap().desired_size().width.is_nan());
        w.close();
        w.set_content(None);
        settle();
    });
}
#[test]
fn native_numeric_keytips_reserve_collisions_and_qat_shortcuts_deduplicate_source() {
    const NAME: &str = "controls::ribbon::quick_access::tests::native_numeric_keytips_reserve_collisions_and_qat_shortcuts_deduplicate_source";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w, cut, _) = setup();
        let q = r.quick_access_tool_bar().unwrap();
        for i in 1..11 {
            let b = RibbonButton::new();
            b.set_label(Some(format!("Tool{i}")));
            q.items().add(Some(b.upcast()));
        }
        let local = RibbonButton::new();
        local.set_id(Some("qat-only".into()));
        local.set_label(Some("Only".into()));
        local.set_shortcut(Some("Ctrl+Q".into()));
        let calls = Rc::new(Cell::new(0));
        let n = calls.clone();
        local.click(move |_, _| n.set(n.get() + 1));
        q.items().add(Some(local.clone().upcast()));
        layout(&w);
        r.show_key_tips();
        settle();
        let entries = r.current_key_tips();
        assert_eq!(entries.get(0).tip, Some("1".into()));
        assert_eq!(entries.get(8).tip, Some("9".into()));
        assert_eq!(entries.get(9).tip, Some("09".into()));
        assert!(
            entries
                .get(0)
                .target
                .as_ref()
                .unwrap()
                .ptr_eq(q.items().get(0).as_ref().unwrap())
        );
        r.cancel_key_tips();
        r.set_is_shortcut_routing_enabled(true);
        assert!(send(&r, Key::Q, KeyModifiers::CONTROL));
        assert_eq!(calls.get(), 1);
        cut.set_shortcut(Some("Ctrl+X".into()));
        let n = calls.clone();
        cut.click(move |_, _| n.set(n.get() + 1));
        r.add_to_quick_access(&cut);
        assert!(send(&r, Key::X, KeyModifiers::CONTROL));
        assert_eq!(calls.get(), 2);
        w.close();
        w.set_content(None);
        settle();
    });
}
#[test]
fn native_move_keeps_links_replace_clear_unlink_and_menu_actions_use_snapshots() {
    const NAME: &str = "controls::ribbon::quick_access::tests::native_move_keeps_links_replace_clear_unlink_and_menu_actions_use_snapshots";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w, cut, bold) = setup();
        r.add_to_quick_access(&cut);
        r.add_to_quick_access(&bold);
        let q = r.quick_access_tool_bar().unwrap();
        let copy = r
            .find_quick_access_copy(Some(cut.clone().upcast()))
            .unwrap()
            .cast::<RibbonButton>()
            .unwrap();
        q.items().move_item(1, 0);
        cut.set_label(Some("Cut moved".into()));
        assert_eq!(copy.label().as_deref(), Some("Cut moved"));
        q.items().set(0, Some(RibbonButton::new().upcast()));
        cut.set_label(Some("After replace".into()));
        assert_eq!(copy.label().as_deref(), Some("Cut moved"));
        let b = r
            .find_quick_access_copy(Some(bold.clone().upcast()))
            .unwrap()
            .cast::<RibbonToggleButton>()
            .unwrap();
        q.items().clear();
        bold.set_is_checked(Some(true));
        assert_ne!(b.is_checked(), Some(true));
        r.quick_access_candidate_ids().add(Some("cut".into()));
        r.quick_access_candidate_ids().add(Some("cut".into()));
        let menu = r.build_quick_access_customize_menu();
        assert_eq!(menu.items().count(), 7);
        let toggle = menu
            .items()
            .get_at(1)
            .as_ref()
            .and_then(|v| ValueTypes::as_object(&**v))
            .unwrap()
            .cast::<ToggleMenuFlyoutItem>()
            .unwrap();
        toggle.invoke_menu_core();
        assert!(r.is_in_quick_access(&cut));
        toggle.invoke_menu_core();
        assert!(!r.is_in_quick_access(&cut));
        let labels = menu
            .items()
            .get_at(5)
            .as_ref()
            .and_then(|v| ValueTypes::as_object(&**v))
            .unwrap()
            .cast::<ToggleMenuFlyoutItem>()
            .unwrap();
        labels.invoke_menu_core();
        assert!(r.show_quick_access_labels());
        w.close();
        w.set_content(None);
        settle();
    });
}
struct Signal(Box<dyn Fn()>);
impl PartialEq for Signal {
    fn eq(&self, o: &Self) -> bool {
        std::ptr::eq(self, o)
    }
}
ferroui_base::ferro_markup_type!(class Signal{this:Rc<Signal>,handles:[Rc<Signal>],methods:[fn Invoke(Option<BoxedValue>,ferroui_base::utilities::EventArgs)=>|s:&Rc<Signal>,_,_|(s.0)()]});
fn signal(f: impl Fn() + 'static) -> MarkupDelegate {
    use ferroui_base::metadata::MarkupTyped;
    ValueTypes::register_reference::<Signal>();
    MarkupDelegate::for_method(
        Some(Rc::new(Signal(Box::new(f)))),
        Signal::MARKUP,
        &Signal::MARKUP.methods[0],
    )
}
fn source_case(name: &str) -> serde_json::Value {
    let v: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/quick-access.json")).unwrap();
    v["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap()
        .clone()
}
#[test]
fn unchanged_source_null_replacement_and_public_notification_order() {
    const NAME: &str = "controls::ribbon::quick_access::tests::unchanged_source_null_replacement_and_public_notification_order";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w, _, _) = setup();
        let source = source_case("replacementNull");
        let old = r.quick_access_tool_bar().unwrap();
        let q = RibbonQuickAccessToolBar::new();
        let item = RibbonButton::new();
        item.set_id(Some("raw".into()));
        q.items().add(Some(item.clone().upcast()));
        let trace = Rc::new(RefCell::new(Vec::new()));
        let t = trace.clone();
        r.add_state_changed(Some(signal(move || t.borrow_mut().push("state"))));
        let t = trace.clone();
        r.add_quick_access_changed(Some(signal(move || t.borrow_mut().push("qat"))));
        r.set_quick_access_tool_bar(Some(q.clone()));
        assert_eq!(old.ribbon(), None);
        assert_eq!(q.ribbon(), Some(r.clone()));
        assert!(
            r.find_quick_access_copy(None).unwrap().ptr_eq(&item),
            "source null query matches first unlinked item"
        );
        r.set_quick_access_tool_bar(None);
        let other = Ribbon::new();
        let empty = source_case("emptyIdsIdentity");
        assert_eq!(
            Rc::ptr_eq(
                &r.get_quick_access_item_ids(),
                &other.get_quick_access_item_ids()
            ),
            empty["same"].as_bool().unwrap()
        );
        let keyboard: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/keyboard.json")).unwrap();
        let empty = keyboard["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "emptyCurrentIdentity")
            .unwrap();
        assert_eq!(
            Rc::ptr_eq(&r.current_key_tips(), &other.current_key_tips()),
            empty["same"].as_bool().unwrap()
        );

        assert_eq!(q.ribbon(), None);
        let above = r.parts.borrow().quick_access_above.clone().unwrap();
        assert_eq!(
            above.child().is_some_and(|c| c.ptr_eq(&q)),
            source["oldVisualRetained"].as_bool().unwrap()
        );
        let expected = source["trace"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|s| s.as_str())
            .filter(|s| *s != "invalidate")
            .collect::<Vec<_>>();
        assert_eq!(&*trace.borrow(), &expected);
        w.close();
        w.set_content(None);
        settle();
    });
}
#[test]
fn reentrant_copy_removal_during_original_click_keeps_one_invocation() {
    const NAME: &str = "controls::ribbon::quick_access::tests::reentrant_copy_removal_during_original_click_keeps_one_invocation";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w, cut, _) = setup();
        r.add_to_quick_access(&cut);
        let copy = r
            .find_quick_access_copy(Some(cut.clone().upcast()))
            .unwrap()
            .cast::<RibbonButton>()
            .unwrap();
        let calls = Rc::new(Cell::new(0));
        let n = calls.clone();
        let root = r.downgrade();
        let source = cut.downgrade();
        cut.click(move |_, _| {
            n.set(n.get() + 1);
            if let (Some(r), Some(c)) = (root.upgrade(), source.upgrade()) {
                assert!(r.remove_from_quick_access(&c));
            }
        });
        copy.perform_click();
        assert_eq!(calls.get(), 1);
        assert!(!r.is_in_quick_access(&cut));
        cut.set_label(Some("after reentrant removal".into()));
        assert_eq!(copy.label().as_deref(), Some("Cut"));
        assert_eq!(RibbonItemHelper::get_source_item(&copy), Some(cut.upcast()));
        w.close();
        w.set_content(None);
        settle();
    });
}
#[test]
fn unchanged_source_qat_reservations_include_collisions_and_duplicate_explicit_tips() {
    const NAME: &str = "controls::ribbon::quick_access::tests::unchanged_source_qat_reservations_include_collisions_and_duplicate_explicit_tips";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let data: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/keyboard.json")).unwrap();
        for case in data["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["name"] == "qatReservation")
        {
            let (r, w, _, _) = setup();
            r.set_is_application_button_visible(true);
            let second = RibbonTab::new();
            second.set_header(Some("Insert".into()));
            second.set_key_tip(Some("N".into()));
            r.tabs().add(Some(second));
            let q = r.quick_access_tool_bar().unwrap();
            q.items().clear();
            for i in 0..case["count"].as_i64().unwrap() {
                let b = RibbonButton::new();
                b.set_label(Some(format!("Quick{i}")));
                b.set_key_tip(if i == 0 {
                    Some("H".into())
                } else if i == 1 || i == 2 {
                    Some("X".into())
                } else {
                    None
                });
                q.items().add(Some(b.upcast()));
            }
            layout(&w);
            r.show_key_tips();
            settle();
            let keys = r
                .current_key_tips()
                .iter()
                .map(|e| e.tip.unwrap().to_utf8().unwrap())
                .collect::<Vec<_>>();
            assert_eq!(serde_json::json!(keys), case["keys"], "{case}");
            r.cancel_key_tips();
            w.close();
            w.set_content(None);
            settle();
        }
    });
}
#[cfg(feature = "source-button-semantics-reproducer")]
#[test]
fn source_externally_retained_qat_items_keep_toolbar_alive() {
    const NAME: &str = "controls::ribbon::quick_access::tests::source_externally_retained_qat_items_keep_toolbar_alive";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let source = source_case("externalItemsLifetime");
        assert_eq!(source["kept"], true);
        let (weak, items) = {
            let q = RibbonQuickAccessToolBar::new();
            (q.downgrade(), q.items())
        };
        assert!(
            weak.upgrade().is_some(),
            "source Items.CollectionChanged delegate strongly retains toolbar; native weak control callback cannot reproduce tracing-GC ownership"
        );
        assert_eq!(items.count() as u64, source["count"].as_u64().unwrap());
    });
}
#[test]
fn ordinary_qat_customize_menu_does_not_acquire_source_keyboard_root_routing() {
    const NAME: &str = "controls::ribbon::quick_access::tests::ordinary_qat_customize_menu_does_not_acquire_source_keyboard_root_routing";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w, cut, _) = setup();
        cut.set_shortcut(Some("Ctrl+X".into()));
        r.set_is_shortcut_routing_enabled(true);
        let calls = Rc::new(Cell::new(0));
        let n = calls.clone();
        cut.click(move |_, _| n.set(n.get() + 1));
        let menu = r.build_quick_access_customize_menu();
        let target = r
            .quick_access_tool_bar()
            .unwrap()
            .customize_button()
            .unwrap();
        menu.show_at(&target);
        settle();
        let entry = menu.items().get_at(2).unwrap();
        let entry = ValueTypes::as_object(&*entry)
            .unwrap()
            .cast::<Control>()
            .unwrap();
        send(&entry, Key::X, KeyModifiers::CONTROL);
        assert_eq!(
            calls.get(),
            0,
            "source customize menu has no keyboard-root forwarding"
        );
        send(&entry, Key::F10, KeyModifiers::NONE);
        assert!(!r.is_key_tip_mode());
        menu.hide();
        menu.show_at(&target);
        settle();
        send(&entry, Key::X, KeyModifiers::CONTROL);
        assert_eq!(calls.get(), 0);
        menu.hide();
        assert!(send(&r, Key::X, KeyModifiers::CONTROL));
        assert_eq!(calls.get(), 1);
        w.close();
        w.set_content(None);
        settle();
    });
}
