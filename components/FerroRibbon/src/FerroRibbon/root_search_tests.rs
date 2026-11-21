use super::*;
use crate::controls::{
    RibbonBackstage, RibbonButtonImpl, RibbonControlBaseImpl, RibbonGroup, RibbonInputBaseImpl,
    RibbonKeyTipResult, RibbonTab,
};
use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::commands::{RibbonCommandDescriptor, RibbonRelayCommand};
use ferroui_base::{
    FerroObjectImpl, StyledElementImpl, VisualImpl, ferro_class, ferro_impl_classes,
    input::InputElementImpl, instantiate, interactivity::InteractiveImpl, layout::LayoutableImpl,
    metadata::MarkupTyped,
};
use ferroui_controls::{
    ButtonImpl, ContentControlImpl, ControlImpl, primitives::TemplatedControlImpl,
};
use serde_json::{Value, json};
use std::cell::Cell;
fn case(name: &str) -> Value {
    serde_json::from_str::<Value>(include_str!("../../tests/fixtures/root-search.json")).unwrap()["corpus"]["cases"].as_array().unwrap().iter().find(|v|v["name"]==name).unwrap().clone()
}
fn make() -> (Ref<Ribbon>, Ref<RibbonTab>, Ref<RibbonGroup>) {
    let r = Ribbon::new();
    let t = RibbonTab::new();
    t.set_id(Some("home".into()));
    t.set_header(Some("Hôme".into()));
    let g = RibbonGroup::new();
    g.set_id(Some("tools".into()));
    g.set_header(Some("Tools".into()));
    t.groups().add(Some(g.clone()));
    r.tabs().add(Some(t.clone()));
    (r, t, g)
}
fn button(id: Option<&str>, label: Option<&str>) -> Ref<RibbonButton> {
    let b = RibbonButton::new();
    b.set_id(id.map(Into::into));
    b.set_label(label.map(Into::into));
    b
}
fn text(s: Option<RibbonString>) -> Option<String> {
    s.map(|s| s.to_utf8().unwrap())
}
fn target_name(target: &BoxedValue) -> String {
    if let Some(item) =
        ValueTypes::as_object(&**target).and_then(|o| o.cast::<RibbonBackstageItem>())
    {
        return format!(
            "backstage:{}",
            item.id().or_else(|| item.header()).unwrap_or_default()
        );
    }
    if let Some(item) = ValueTypes::as_object(&**target).and_then(|o| o.cast::<Control>()) {
        return item.name().unwrap_or_default();
    }
    if let Some(d) = cast::<Rc<RibbonCommandDescriptor>>(target) {
        return format!("descriptor:{}", d.id().to_utf8().unwrap());
    }
    if cast::<MarkupDelegate>(target).is_some() {
        return "action".into();
    }
    "unknown".into()
}
fn rows(r: &Ribbon) -> Value {
    let mut rows = Vec::new();
    for entry in r.build_search_entries().iter() {
        let entry = entry.unwrap();
        let target = native_target(&entry);
        rows.push(json!({
            "Id":text(entry.id()),
            "Label":text(entry.label()),
            "Path":text(entry.path()),
            "Description":text(entry.description()),
            "keywords":entry.keywords().map(|v|v.iter().map(text).collect::<Vec<_>>()),
            "Shortcut":text(entry.shortcut()),
            "IsEnabled":entry.is_enabled(),
            "target":target.as_ref().map(target_name)
        }));
    }
    json!(rows)
}
struct Action(Rc<Cell<i32>>);
impl PartialEq for Action {
    fn eq(&self, o: &Self) -> bool {
        std::ptr::eq(self, o)
    }
}
ferroui_base::ferro_markup_type!(class Action{handles:[Rc<Action>],this:Rc<Action>,methods:[
fn Invoke()=>|v:&Rc<Action>|v.0.set(v.0.get()+1),
fn Result()->bool =>|v:&Rc<Action>|{v.0.set(v.0.get()+1);true},
fn WithArgument(i32)=>|v:&Rc<Action>,_:i32|v.0.set(v.0.get()+1),
static try fn Fail()=>||Err::<(),_>("typed-failed")
]});
fn action(count: Rc<Cell<i32>>) -> BoxedValue {
    ValueTypes::register_reference::<Action>();
    Rc::new(
        MarkupDelegate::for_action_method(
            Some(Rc::new(Action(count))),
            Action::MARKUP,
            &Action::MARKUP.methods[0],
        )
        .expect("declared source Action"),
    )
}
fn entry(id: &str, target: Option<BoxedValue>) -> Rc<RibbonSearchEntry> {
    RibbonSearchEntry::with_options(
        Some(id.into()),
        Some(id.into()),
        RibbonSearchEntryOptions {
            target: crate::metadata::model_objects::from_native(target),
            ..Default::default()
        },
    )
}
#[test]
fn original_root_search_entry_order_ids_filters_keywords_targets_and_catalog_dedup() {
    crate::group_tests::run(|| {
        let (r, t, g) = make();
        for (id, label, cmd, name) in [
            (Some("same"), "First\nLine", Some("PASTE"), "first"),
            (Some("same"), "Duplicate", Some("other"), "duplicate"),
            (Some("Same"), "Case ID", None, "case"),
            (None, "Fallback", None, ""),
            (Some("space"), "\u{85} \u{2003}", None, "space"),
            (Some("hidden"), "Hidden", None, "hidden"),
            (Some("off"), "Excluded", None, "off"),
            (Some("disabled"), "Disabled", None, "disabled"),
        ] {
            let b = button(id, Some(label));
            b.set_name(Some(name.into()));
            b.set_command_id(cmd.map(Into::into));
            if name == "hidden" {
                b.set_is_visible(false);
            }
            if name == "off" {
                RibbonSearch::set_is_searchable(&b, false);
            }
            if name == "disabled" {
                b.set_is_enabled(false);
            }
            if name == "first" {
                b.set_screen_tip(Some(Rc::new("Description".to_string())));
                b.set_shortcut(Some("Ctrl+V".into()));
                RibbonSearch::set_keywords(
                    &b,
                    Some("  alpha, ,\u{85}beta\u{3000},é\u{301},𐐨 ".into()),
                );
            }
            g.items().add(Some(b.upcast()));
        }
        let gone = RibbonGroup::new();
        gone.set_is_visible(false);
        gone.items()
            .add(Some(button(Some("gone"), Some("Gone")).upcast()));
        t.groups().add(Some(gone));
        let unavailable = RibbonTab::new();
        unavailable.set_is_tab_visible(false);
        let gone = RibbonGroup::new();
        gone.items().add(Some(
            button(Some("unavailable"), Some("Unavailable")).upcast(),
        ));
        unavailable.groups().add(Some(gone));
        r.tabs().add(Some(unavailable));
        let back = RibbonBackstage::new();
        for (id, header, visible) in [
            (Some("save"), "Save", true),
            (None, "", true),
            (None, " ", true),
            (Some("save"), "Other", true),
            (None, "No", false),
        ] {
            let item = RibbonBackstageItem::new();
            item.set_id(id.map(Into::into));
            item.set_header(Some(header.into()));
            item.set_is_visible(visible);
            back.items().add(Some(item));
        }
        r.set_backstage(Some(back));
        let catalog = ferroribbon_core::commands::RibbonCommandCatalog::new();
        catalog.register(Some(RibbonCommandDescriptor::new(
            Some("paste".into()),
            Some("Paste catalog".into()),
            None,
        )));
        let d = RibbonCommandDescriptor::new(Some("undo".into()), Some("Undo".into()), None);
        d.set_category(Some("Edit".into()));
        d.set_description(Some("Undo desc".into()));
        d.set_shortcut(Some("Ctrl+Z".into()));
        d.set_is_enabled(false);
        d.keywords().borrow_mut().push(Some("back".into()));
        catalog.register(Some(d));
        r.set_command_catalog(Some(catalog));
        let extras = r.additional_search_entries();
        extras.add(Some(RibbonSearchEntry::new(
            Some("same".into()),
            Some("Extra duplicate".into()),
        )));
        extras.add(Some(RibbonSearchEntry::new(
            Some("command/paste".into()),
            Some("Not reserved".into()),
        )));
        let extra = RibbonSearchEntry::with_options(
            Some("extra".into()),
            Some("Help".into()),
            RibbonSearchEntryOptions {
                target: crate::metadata::model_objects::from_native(Some(action(Rc::new(
                    Cell::new(0),
                )))),
                ..Default::default()
            },
        );
        extras.add(Some(extra.clone()));
        extras.add(Some(RibbonSearchEntry::new(None, Some("Null ID".into()))));
        assert!(extras == r.additional_search_entries());
        assert_eq!(rows(&r), case("entries")["rows"]);
        let built = r.build_search_entries();
        assert!(Rc::ptr_eq(&built.get(8).unwrap(), &extra));
        extras.clear();
        assert_eq!(
            built.count(),
            10,
            "source fresh index retains captured entries"
        );
        assert_eq!(r.build_search_entries().count(), 7);
    });
}
#[test]
fn original_search_descriptions_native_model_unknown_and_reindexing() {
    crate::group_tests::run(|| {
        let (r, _, g) = make();
        for (id, label) in [
            ("native", "Native"),
            ("model", "Model"),
            ("unknown", "Unknown"),
        ] {
            let b = button(Some(id), Some(label));
            let tip: BoxedValue = match id {
                "native" => {
                    let t = RibbonScreenTip::new();
                    t.set_description(Some("native tip".into()));
                    Rc::new(t)
                }
                "model" => Rc::new(ferroribbon_core::model::RibbonScreenTip::with_description(
                    None,
                    Some("model tip".into()),
                )),
                _ => Rc::new(9i32),
            };
            b.set_screen_tip(Some(tip));
            g.items().add(Some(b.upcast()));
        }
        assert_eq!(rows(&r), case("descriptions")["rows"]);
        let (r, _, g) = make();
        let b = button(Some("a"), Some("Alpha"));
        g.items().add(Some(b.clone().upcast()));
        let first = r.search_default(Some(&"Alpha".into()));
        assert_eq!(first.len(), 1);
        b.set_label(Some("Beta".into()));
        assert_eq!(r.search_default(Some(&"Alpha".into())).len(), 0);
        assert_eq!(
            text(
                r.search_default(Some(&"Beta".into()))
                    .get(0)
                    .unwrap()
                    .entry()
                    .unwrap()
                    .label()
            )
            .as_deref(),
            Some("Beta")
        );
        for limit in [0, -1, i32::MIN] {
            assert_eq!(r.search(Some(&"Beta".into()), limit).len(), 0);
        }
    });
}
#[repr(C)]
struct Direct {
    base: RibbonButton,
    count: Rc<Cell<i32>>,
}
ferro_class!(Direct:RibbonButton);
ferro_impl_classes!(Direct:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl);
impl RibbonButtonImpl for Direct {
    fn invoke_core(t: &Self) -> bool {
        t.count.set(t.count.get() + 1);
        false
    }
}
#[test]
fn original_execute_dispatch_does_not_gate_entry_flag_and_marks_before_unsupported() {
    crate::group_tests::run(|| {
        let (r, _, _) = make();
        let calls = Rc::new(Cell::new(0));
        let direct = instantiate(Direct {
            base: RibbonButton::construct(),
            count: calls.clone(),
        });
        let p = r.execute_search_entry(Some(entry("p", Some(Control::boxed(direct)))));
        let actions = Rc::new(Cell::new(0));
        let a = action(actions.clone());
        let ok = r.execute_search_entry(Some(entry("action", Some(a.clone()))));
        let disabled = r.execute_search_entry(Some(RibbonSearchEntry::with_options(
            Some("action".into()),
            Some("Action".into()),
            RibbonSearchEntryOptions {
                target: crate::metadata::model_objects::from_native(Some(a)),
                is_enabled: false,
                ..Default::default()
            },
        )));
        let unknown = r.execute_search_entry(Some(entry("unknown", Some(Rc::new(7i32)))));
        assert_eq!(
            json!({"name":"direct_execution","p":p,"a":ok,"disabled":disabled,"unsupported":unknown,"calls":actions.get(),"Invocations":calls.get(),"recent":r.search_engine().get_recent_entries().iter().map(|e|text(e.id())).collect::<Vec<_>>()}),
            case("direct_execution")
        );
        // MarkUsed happens even when dispatch fails and an entry's own enabled flag is false.
        let indexed = RibbonSearchEntry::new(Some("unknown".into()), Some("Unknown".into()));
        r.search_engine()
            .set_entries(Some(RibbonItemsSource::from_values(vec![Some(
                indexed.clone(),
            )])));
        r.execute_search_entry(Some(entry("unknown", None)));
        assert!(Rc::ptr_eq(
            &r.search_engine()
                .get_recent_entries()
                .iter()
                .next()
                .unwrap(),
            &indexed
        ));
    });
}
#[test]
fn original_backstage_and_descriptor_dispatch_uses_current_catalog() {
    crate::group_tests::run(|| {
        let (r, _, _) = make();
        let back = RibbonBackstage::new();
        let called = Rc::new(Cell::new(0));
        let c = called.clone();
        back.add_item_invoked(Some(backstage_handler(move || c.set(c.get() + 1))));
        r.set_backstage(Some(back));
        let item = RibbonBackstageItem::new();
        item.set_content(Some(Rc::new("page".to_string())));
        let ok = r.execute_search_entry(Some(RibbonSearchEntry::with_options(
            Some("back".into()),
            Some("Back".into()),
            RibbonSearchEntryOptions {
                target: crate::metadata::model_objects::from_native(Some(Rc::new(item))),
                is_enabled: false,
                ..Default::default()
            },
        )));
        let count = Rc::new(Cell::new(0));
        let c = count.clone();
        let catalog = ferroribbon_core::commands::RibbonCommandCatalog::new();
        catalog.register(Some(RibbonCommandDescriptor::new(
            Some("run".into()),
            Some("Run".into()),
            Some(RibbonRelayCommand::new(move || c.set(c.get() + 1))),
        )));
        r.set_command_catalog(Some(catalog));
        let descriptor =
            RibbonCommandDescriptor::new(Some("RUN".into()), Some("Other instance".into()), None);
        let command = r.execute_search_entry(Some(entry("cmd", Some(Rc::new(descriptor.clone())))));
        r.set_command_catalog(None);
        let absent = r.execute_search_entry(Some(entry("none", Some(Rc::new(descriptor)))));
        assert_eq!(
            json!({"name":"backstage_catalog","ok":ok,"IsBackstageOpen":r.is_backstage_open(),"invoked":called.get(),"command":command,"absent":absent,"calls":count.get()}),
            case("backstage_catalog")
        );
    });
}
struct BackHandler(Box<dyn Fn()>);
impl PartialEq for BackHandler {
    fn eq(&self, o: &Self) -> bool {
        std::ptr::eq(self, o)
    }
}
ferroui_base::ferro_markup_type!(class BackHandler{handles:[Rc<BackHandler>],this:Rc<BackHandler>,methods:[fn Invoke(Option<BoxedValue>,Ref<RibbonBackstageItem>)=>|v:&Rc<BackHandler>,_,_|(v.0)()]});
fn backstage_handler(f: impl Fn() + 'static) -> MarkupDelegate {
    ValueTypes::register_reference::<BackHandler>();
    MarkupDelegate::for_method(
        Some(Rc::new(BackHandler(Box::new(f)))),
        BackHandler::MARKUP,
        &BackHandler::MARKUP.methods[0],
    )
}
#[test]
fn search_null_failures_remain_explicit() {
    crate::group_tests::run(|| {
        let (r, _, _) = make();
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| r.execute_search_entry(None)))
                .is_err()
        );
        r.additional_search_entries().add(None);
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| r.build_search_entries()))
                .is_err()
        );
        r.additional_search_entries().clear();
        // Ribbon.cs OnTabsCollectionChanged dereferences each tab immediately.
        // Unlike the search-only reference seams, the real collection notifies it.
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| r.tabs().add(None))).is_err()
        );
        assert_eq!(
            r.tabs().count(),
            2,
            "source mutation precedes failing collection notification"
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| r.build_search_entries()))
                .is_err()
        );
        r.tabs().clear();
    });
}
#[test]
fn tied_search_uses_native_collation_and_never_silently_uses_ordinal() {
    crate::group_tests::run(|| {
        let (r, _, _) = make();
        let words = RibbonFixedItems::new(vec![Some("tie".into())]) as RibbonReadOnlyList<_>;
        for name in ["z", "ä"] {
            r.additional_search_entries()
                .add(Some(RibbonSearchEntry::with_options(
                    Some(name.into()),
                    Some(name.into()),
                    RibbonSearchEntryOptions {
                        keywords: Some(words.clone()),
                        ..Default::default()
                    },
                )));
        }
        #[cfg(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "emscripten",
            target_os = "linux"
        ))]
        assert_eq!(
            r.search_default(Some(&"tie".into()))
                .iter()
                .map(|result| result
                    .unwrap()
                    .entry()
                    .unwrap()
                    .label()
                    .unwrap()
                    .to_utf8()
                    .unwrap())
                .collect::<Vec<_>>(),
            ["ä", "z"]
        );
        #[cfg(not(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "emscripten",
            target_os = "linux"
        )))]
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || r.search_default(Some(&"tie".into()))
            ))
            .is_err()
        );
    });
}

#[repr(C)]
struct TrackedInput {
    base: RibbonInputBase,
    keys: Rc<Cell<i32>>,
}
ferro_class!(TrackedInput:RibbonInputBase);
ferro_impl_classes!(TrackedInput:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,RibbonInputBaseImpl);
impl RibbonControlBaseImpl for TrackedInput {
    fn on_key_tip_core(t: &Self) -> Rc<RibbonKeyTipResult> {
        t.keys.set(t.keys.get() + 1);
        RibbonKeyTipResult::new()
    }
}
#[repr(C)]
struct TrackedGallery {
    base: RibbonGallery,
    keys: Rc<Cell<i32>>,
}
ferro_class!(TrackedGallery:RibbonGallery);
ferro_impl_classes!(TrackedGallery:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl RibbonControlBaseImpl for TrackedGallery {
    fn on_key_tip_core(t: &Self) -> Rc<RibbonKeyTipResult> {
        t.keys.set(t.keys.get() + 1);
        RibbonKeyTipResult::new()
    }
}
#[repr(C)]
struct TrackedDrop {
    base: RibbonDropDownButton,
    keys: Rc<Cell<i32>>,
}
ferro_class!(TrackedDrop:RibbonDropDownButton);
ferro_impl_classes!(TrackedDrop:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl);
impl RibbonButtonImpl for TrackedDrop {
    fn on_key_tip_core(t: &Self) -> Rc<RibbonKeyTipResult> {
        t.keys.set(t.keys.get() + 1);
        RibbonKeyTipResult::new()
    }
}
#[repr(C)]
struct TrackedFlyout {
    base: RibbonButton,
    keys: Rc<Cell<i32>>,
}
ferro_class!(TrackedFlyout:RibbonButton);
ferro_impl_classes!(TrackedFlyout:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl);
impl RibbonButtonImpl for TrackedFlyout {
    fn on_key_tip_core(t: &Self) -> Rc<RibbonKeyTipResult> {
        t.keys.set(t.keys.get() + 1);
        RibbonKeyTipResult::new()
    }
}
#[test]
fn original_ui_execution_selects_opens_and_defers_exactly_one_virtual_keytip() {
    crate::group_tests::run(|| {
        for row in case("ui_execution")["rows"].as_array().unwrap() {
            let kind = row["kind"].as_str().unwrap();
            let (r, t, g) = make();
            let keys = Rc::new(Cell::new(0));
            let control: Ref<Control> = match kind {
                "input" => instantiate(TrackedInput {
                    base: RibbonInputBase::construct(),
                    keys: keys.clone(),
                })
                .upcast(),
                "gallery" => instantiate(TrackedGallery {
                    base: RibbonGallery::construct(),
                    keys: keys.clone(),
                })
                .upcast(),
                "drop" => instantiate(TrackedDrop {
                    base: RibbonDropDownButton::construct(),
                    keys: keys.clone(),
                })
                .upcast(),
                _ => {
                    let c = instantiate(TrackedFlyout {
                        base: RibbonButton::construct(),
                        keys: keys.clone(),
                    });
                    c.set_flyout(Some(ferroui_controls::Flyout::new().upcast()));
                    c.upcast()
                }
            };
            control.set_width(400.);
            if kind != "outside" {
                g.items().add(Some(control.clone()));
                // Keep the group genuinely collapsed after popup measurement;
                // a single item can have a smaller Small state than its header button.
                for i in 0..20 {
                    let filler = button(Some(&format!("filler{i}")), Some("Filler"));
                    filler.set_icon(Some(Rc::new("A".to_string())));
                    g.items().add(Some(filler.upcast()));
                }
            }
            let w = crate::ribbon_tests::mount(&r);
            w.set_width(80.);
            w.measure(ferroui_base::Size::new(80., 300.));
            w.arrange(ferroui_base::Rect::new(0., 0., 80., 300.));
            Dispatcher::ui_thread().run_jobs(None);
            assert!(r.bounds().width <= 80., "actual root constraint");
            r.set_visibility_mode(RibbonVisibilityMode::TabsOnly);
            g.apply_state(RibbonGroupState::Collapsed, false, r.metrics(), None);
            let ok = r.execute_search_entry(Some(entry(kind, Some(Control::boxed(control)))));
            assert_eq!(ok, row["ok"].as_bool().unwrap());
            assert_eq!(keys.get(), row["before"].as_i64().unwrap() as i32);
            if kind != "outside" {
                assert!(r.selected_tab().unwrap().ptr_eq(&t));
                assert!(r.is_minimized_popup_open());
                if !g.is_popup_open() {
                    let state = g.state();
                    let widths: Vec<_> = (0..4)
                        .map(|i| g.measure_width(RibbonGroupState::from_raw(i), r.metrics(), 300.))
                        .collect();
                    panic!(
                        "{kind}: state={state}, simplified={}, root_width={}, widths={widths:?}, collapsed_button={}, children={}",
                        g.is_simplified(),
                        r.bounds().width,
                        g.collapsed_button().is_some(),
                        g.items_panel().children().count()
                    );
                }
            }
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(keys.get(), row["after"].as_i64().unwrap() as i32);
            w.close();
        }
    });
}

// Source runtime scenario: FeatureTests.cs Search_finds_and_executes_items.
#[test]
fn source_runtime_search_toggles_actual_item_and_rebuilds_recent_and_backstage_entries() {
    crate::group_tests::run(|| {
        let (r, t, g) = make();
        t.set_header(Some("Home".into()));
        g.set_header(Some("Font".into()));
        let bold = crate::controls::RibbonToggleButton::new();
        bold.set_id(Some("bold".into()));
        bold.set_label(Some("Bold".into()));
        g.items().add(Some(bold.clone().upcast()));
        let back = RibbonBackstage::new();
        let info = RibbonBackstageItem::new();
        info.set_id(Some("info".into()));
        info.set_header(Some("Info".into()));
        back.items().add(Some(info.clone()));
        r.set_backstage(Some(back));
        let w = crate::ribbon_tests::mount(&r);
        let results = r.search_default(Some(&"bold".into()));
        assert!(!results.is_empty());
        let result = results.get(0).unwrap().entry().unwrap();
        assert_eq!(text(result.label()).as_deref(), Some("Bold"));
        assert_eq!(text(result.path()).as_deref(), Some("Home › Font"));
        assert!(r.execute_search_entry(Some(result)));
        assert_eq!(bold.is_checked(), Some(true));
        assert!(
            r.search_default(Some(&"".into()))
                .iter()
                .flatten()
                .any(|v| v.entry().unwrap().label() == Some("Bold".into()))
        );
        let result = r
            .search_default(Some(&"info".into()))
            .get(0)
            .unwrap()
            .entry()
            .unwrap();
        assert!(
            ValueTypes::as_object(&*native_target(&result).unwrap())
                .unwrap()
                .ptr_eq(&info)
        );
        w.close();
    });
}
#[test]
fn queued_search_retains_external_target_only_until_virtual_keytip_runs() {
    crate::group_tests::run(|| {
        let (r, _, _) = make();
        let keys = Rc::new(Cell::new(0));
        let control = instantiate(TrackedInput {
            base: RibbonInputBase::construct(),
            keys: keys.clone(),
        });
        let weak = control.downgrade();
        assert!(r.execute_search_entry(Some(entry(
            "outside-lifetime",
            Some(Control::boxed(control.upcast::<Control>()))
        ))));
        assert!(weak.upgrade().is_some());
        assert_eq!(keys.get(), 0);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(keys.get(), 1);
        assert!(
            weak.upgrade().is_none(),
            "queued source target released after dispatch"
        );
    });
}

#[test]
fn search_entry_enabled_snapshot_tracks_inherited_group_disable_and_current_invocation() {
    crate::group_tests::run(|| {
        let (r, _, g) = make();
        let b = button(Some("alpha"), Some("Alpha"));
        let clicks = Rc::new(Cell::new(0));
        let observed = clicks.clone();
        b.click(move |_, _| observed.set(observed.get() + 1));
        g.items().add(Some(b.clone().upcast()));
        let w = crate::ribbon_tests::mount(&r);
        assert!(r.build_search_entries().get(0).unwrap().is_enabled());
        g.set_is_enabled(false);
        assert!(
            !b.is_enabled(),
            "public enabled property includes inherited disabling"
        );
        assert!(
            !b.is_effectively_enabled(),
            "parent disables the source Control.IsEnabled value"
        );
        let disabled = r.build_search_entries().get(0).unwrap();
        assert!(
            !disabled.is_enabled(),
            "source search entry snapshots inherited disabling"
        );
        assert!(!r.execute_search_entry(Some(disabled.clone())));
        assert_eq!(clicks.get(), 0);
        g.set_is_enabled(true);
        assert!(b.is_enabled());
        assert!(b.is_effectively_enabled());
        assert!(
            !disabled.is_enabled(),
            "existing source record remains immutable"
        );
        assert!(r.build_search_entries().get(0).unwrap().is_enabled());
        assert!(
            r.execute_search_entry(Some(disabled)),
            "dispatch reads current item eligibility without gating snapshot flag"
        );
        assert_eq!(clicks.get(), 1);
        w.close();
    });
}

#[test]
fn source_nominal_delegate_target_matrix_rejects_non_actions_before_any_invocation() {
    crate::group_tests::run(|| {
        use ferroui_base::metadata::{MarkupDelegateType, SYSTEM_ACTION_DELEGATE};
        static FUNC: MarkupDelegateType = MarkupDelegateType::new(
            "System.Func`1[System.Boolean]",
            &[],
            Some(ValueType::of::<bool>),
        );
        static ARG: MarkupDelegateType = MarkupDelegateType::new(
            "System.Action`1[System.Int32]",
            &[ValueType::of::<i32>],
            None,
        );
        static CUSTOM: MarkupDelegateType = MarkupDelegateType::new("CustomVoid", &[], None);
        let (r, _, _) = make();
        ValueTypes::register_reference::<Action>();
        let mut rows = Vec::new();
        for (kind, type_, method) in [
            ("action", &SYSTEM_ACTION_DELEGATE, "Invoke"),
            ("func", &FUNC, "Result"),
            ("arg", &ARG, "WithArgument"),
            ("custom", &CUSTOM, "Invoke"),
        ] {
            let count = Rc::new(Cell::new(0));
            let callback = MarkupDelegate::for_typed_method(
                type_,
                Some(Rc::new(Action(count.clone()))),
                Action::MARKUP,
                Action::MARKUP.find_methods(method).next().unwrap(),
            )
            .unwrap();
            let ok = r.execute_search_entry(Some(entry(kind, Some(Rc::new(callback)))));
            rows.push(json!({"kind":kind,"ok":ok,"calls":count.get()}));
        }
        let recent: Vec<_> = r.search_engine().recent().iter().map(text).collect();
        assert_eq!(
            json!({"name":"delegate_targets","rows":rows,"recent":recent}),
            case("delegate_targets")
        );
    });
}
#[test]
fn untyped_void_callbacks_are_not_assumed_to_be_source_actions_and_multicast_keeps_type() {
    crate::group_tests::run(|| {
        let (r, _, _) = make();
        let count = Rc::new(Cell::new(0));
        let observed = count.clone();
        let unknown = MarkupDelegate::new(move |_| {
            observed.set(observed.get() + 1);
            None
        });
        assert!(!r.execute_search_entry(Some(entry("untyped", Some(Rc::new(unknown))))));
        assert_eq!(count.get(), 0);
        let a = action(count.clone());
        let a = cast::<MarkupDelegate>(&a).unwrap();
        let list = MarkupDelegate::combine(Some(&a), Some(&a)).unwrap();
        assert!(r.execute_search_entry(Some(entry("multi", Some(Rc::new(list))))));
        assert_eq!(count.get(), 2);
    });
}

#[test]
fn source_nominal_delegate_equality_removal_and_failure_order_fixture() {
    crate::group_tests::run(|| {
        use ferroui_base::metadata::{
            MarkupDelegateType, MarkupInvokeError, SYSTEM_ACTION_DELEGATE,
        };
        static CUSTOM: MarkupDelegateType = MarkupDelegateType::new("CustomVoid", &[], None);
        ValueTypes::register_reference::<Action>();
        let expected = case("delegate_nominal");
        let target: BoxedValue = Rc::new(Action(Rc::new(Cell::new(0))));
        let method = &Action::MARKUP.methods[0];
        let a = MarkupDelegate::for_action_method(Some(target.clone()), Action::MARKUP, method)
            .unwrap();
        let same = MarkupDelegate::for_action_method(Some(target.clone()), Action::MARKUP, method)
            .unwrap();
        let custom =
            MarkupDelegate::for_typed_method(&CUSTOM, Some(target), Action::MARKUP, method)
                .unwrap();
        assert_eq!(a == same, expected["sameMethod"].as_bool().unwrap());
        assert_eq!(
            a.ptr_eq(&same),
            expected["sameReference"].as_bool().unwrap()
        );
        assert_eq!(a == custom, expected["customEqual"].as_bool().unwrap());
        for (field, combine) in [("mismatchCombine", true), ("mismatchRemove", false)] {
            let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if combine {
                    MarkupDelegate::combine(Some(&a), Some(&custom))
                } else {
                    MarkupDelegate::remove(Some(&a), Some(&custom))
                }
            }))
            .expect_err("nominal type mismatch");
            let message = error
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| error.downcast_ref::<&str>().copied())
                .unwrap_or("");
            assert!(
                message.starts_with(expected[field].as_str().unwrap()),
                "{message}"
            );
        }
        assert_eq!(
            MarkupDelegate::combine(None, Some(&custom))
                .unwrap()
                .ptr_eq(&custom),
            expected["nullCombineSame"].as_bool().unwrap()
        );
        assert_eq!(
            MarkupDelegate::remove(None, Some(&custom)).is_none(),
            expected["nullRemove"].as_bool().unwrap()
        );
        let trace = Rc::new(std::cell::RefCell::new(Vec::new()));
        let make = |name| {
            let trace = trace.clone();
            MarkupDelegate::action(move || trace.borrow_mut().push(name))
        };
        let first = make("a");
        let second = make("b");
        let pair = MarkupDelegate::combine(Some(&first), Some(&second)).unwrap();
        let twice = MarkupDelegate::combine(Some(&pair), Some(&pair)).unwrap();
        let removed = MarkupDelegate::remove(Some(&twice), Some(&pair)).unwrap();
        removed.try_invoke(&[]).unwrap();
        assert_eq!(removed == pair, expected["removedEqual"].as_bool().unwrap());
        assert_eq!(
            removed.ptr_eq(&pair),
            expected["removedReference"].as_bool().unwrap()
        );
        assert_eq!(
            removed.invocation_list().len(),
            expected["removedCount"].as_u64().unwrap() as usize
        );
        assert_eq!(json!(&*trace.borrow()), expected["trace"]);
        trace.borrow_mut().clear();
        let before = make("before");
        let after = make("after");
        let fail = MarkupDelegate::for_action_method(
            None,
            Action::MARKUP,
            Action::MARKUP.find_methods("Fail").next().unwrap(),
        )
        .unwrap();
        let prefix = MarkupDelegate::combine(Some(&before), Some(&fail)).unwrap();
        let all = MarkupDelegate::combine(Some(&prefix), Some(&after)).unwrap();
        assert!(all.is_of_type(&SYSTEM_ACTION_DELEGATE));
        // Source CLR InvalidOperationException is represented as Failed here;
        // this proves error propagation/order, not CLR exception type identity.
        assert_eq!(expected["failure"], "InvalidOperationException");
        assert_eq!(
            all.try_invoke(&[]),
            Err(MarkupInvokeError::Failed("typed-failed".into()))
        );
        assert_eq!(json!(&*trace.borrow()), expected["throwTrace"]);
    });
}
