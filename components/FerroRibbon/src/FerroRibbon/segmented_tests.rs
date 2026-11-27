use super::*;
use crate::{
    controls::{IRibbonItemOwner, Ribbon, RibbonItemHelper},
    mvvm::{RibbonElementFactory, RibbonModelBindings},
};
use ferroui_base::{
    controls::ResourceDictionary,
    data::core::{ValueType, ValueTypes},
    input::InputElement,
    metadata::from_markup_value,
    reactive::{Disposable, IDisposable},
    styling::ControlTheme,
};
use serde_json::{Value, json};
fn cases(name: &str) -> Vec<Value> {
    serde_json::from_str::<Value>(include_str!("../../tests/fixtures/segmented.json")).unwrap()["corpus"]["cases"].as_array().unwrap().iter().filter(|c|c["name"]==name).cloned().collect()
}
fn text(p: Option<&BoxedValue>) -> String {
    p.map_or_else(
        || "<null>".into(),
        |v| {
            if let Some(s) = v.downcast_ref::<String>() {
                s.clone()
            } else if let Some(b) = v.downcast_ref::<bool>() {
                if *b { "True".into() } else { "False".into() }
            } else {
                panic!("Unexpected command payload {:?}", v.type_name())
            }
        },
    )
}
struct Cmd(Rc<RefCell<Vec<String>>>, String);
impl ferroui_base::input::ICommand for Cmd {
    fn can_execute(&self, p: Option<&BoxedValue>) -> bool {
        self.0
            .borrow_mut()
            .push(format!("{}can:{}", self.1, text(p)));
        true
    }
    fn execute(&self, p: Option<&BoxedValue>) {
        self.0
            .borrow_mut()
            .push(format!("{}execute:{}", self.1, text(p)));
    }
    fn can_execute_changed(&self, _: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}
fn theme(c: &RibbonSegmentedControl) {
    let value =
        crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/Segmented.xaml")
            .unwrap()
            .unwrap();
    let dict = ValueTypes::as_object(&*value)
        .unwrap()
        .cast::<ResourceDictionary>()
        .unwrap();
    let theme = dict
        .try_get_resource(&RibbonSegmentedControl::TYPE.into(), None)
        .flatten()
        .unwrap();
    c.set_resources(dict);
    c.set_theme(
        ValueTypes::as_object(&*theme)
            .unwrap()
            .cast::<ControlTheme>()
            .unwrap(),
    );
    c.apply_styling();
    c.apply_template();
}
fn segment(label: &str, value: &str) -> Ref<RibbonSegment> {
    let s = RibbonSegment::new();
    s.set_label(Some(label.into()));
    s.set_segment_value(Some(Rc::new(value.to_owned())));
    s
}
// An attached native owner is weak: retain the real ribbon for the duration of
// each fixture, just as the source harness retains its owner. The CLR retention
// difference is covered separately; it must not remove command-routing checks.
struct Fixture {
    control: Ref<RibbonSegmentedControl>,
    _owner: Ref<Ribbon>,
}
impl std::ops::Deref for Fixture {
    type Target = Ref<RibbonSegmentedControl>;
    fn deref(&self) -> &Self::Target {
        &self.control
    }
}
fn make(count: usize, t: Rc<RefCell<Vec<String>>>) -> Fixture {
    let c = RibbonSegmentedControl::new();
    for i in 0..count {
        c.segments().add(Some(segment(
            &((b'A' + i as u8) as char).to_string(),
            &format!("v{i}"),
        )));
    }
    let trace = t.clone();
    c.add_selection_changed(Some(MarkupDelegate::new(move |a| {
        let s = from_markup_value::<Option<Ref<RibbonSegment>>>(&a[1]).unwrap();
        trace.borrow_mut().push(format!(
            "selected:{}",
            s.and_then(|s| s.label()).unwrap_or_else(|| "<null>".into())
        ));
        None
    })));
    let r = Ribbon::new();
    let owner = ValueTypes::try_cast(
        &Control::boxed(r.clone()),
        ValueType::of::<Rc<dyn IRibbonItemOwner>>(),
    )
    .unwrap();
    RibbonItemHelper::set_owner(
        &c,
        Some(
            owner
                .downcast_ref::<Rc<dyn IRibbonItemOwner>>()
                .unwrap()
                .clone(),
        ),
    )
    .unwrap();
    let trace = t.clone();
    r.add_item_invoked(Some(crate::root_command_tests::invocation(move |a| {
        trace
            .borrow_mut()
            .push(format!("invoked:{}", text(a.parameter.as_ref())))
    })));
    c.set_command(Some(Rc::new(Cmd(t.clone(), String::new()))));
    theme(&c);
    t.borrow_mut().clear();
    Fixture {
        control: c,
        _owner: r,
    }
}
fn state(c: &RibbonSegmentedControl) -> Value {
    let host = c.host.borrow().clone();
    let buttons=host.map(|h|h.children().to_vec().iter().map(|b|{let b=b.cast::<RibbonToggleButton>().unwrap();json!({"Label":b.label(),"IsChecked":b.is_checked(),"IsTabStop":KeyboardNavigation::get_is_tab_stop(&b),"CanAddToQuickAccess":b.can_add_to_quick_access(),"AutomationName":AutomationProperties::get_name(&b),"layout":{"size":if b.current_size()==RibbonItemSize::Medium{"Medium"}else{"unexpected"},"IsSimplified":b.is_simplified(),"ShowLabel":b.actual_show_label()}})}).collect::<Vec<_>>());
    json!({"index":c.selected_index(),"selected":c.selected_segment().and_then(|s|s.label()),"buttons":buttons})
}
fn key(c: &RibbonSegmentedControl, k: Key) -> bool {
    let mut e = KeyEventArgs::new();
    e.key = k;
    e.set_routed_event(Some(InputElement::key_down_event()));
    c.raise_event(&e);
    e.handled()
}
#[test]
fn original_segment_defaults_programmatic_and_keyboard_states() {
    crate::group_tests::run(|| {
        let c = RibbonSegmentedControl::new();
        let d = &cases("defaults")[0];
        assert_eq!(c.selected_index(), d["SelectedIndex"]);
        assert!(c.selected_segment().is_none());
        assert_eq!(c.can_add_to_quick_access(), d["CanAddToQuickAccess"]);
        assert!(c.segments() == c.segments());
        assert_eq!(KeyboardNavigation::get_is_tab_stop(&c), d["IsTabStop"]);
        assert!(c.focusable());
        for source in cases("keyboard") {
            let t = Rc::new(RefCell::new(vec![]));
            let c = make(source["count"].as_u64().unwrap() as usize, t.clone());
            c.set_selected_index(source["initial"].as_i64().unwrap() as i32);
            t.borrow_mut().clear();
            let mut states = vec![];
            for (name, k) in [
                ("Left", Key::Left),
                ("Right", Key::Right),
                ("Home", Key::Home),
                ("End", Key::End),
                ("Enter", Key::Enter),
                ("Space", Key::Space),
                ("Up", Key::Up),
            ] {
                let handled = key(&c, k);
                states.push(json!({"key":name,"Handled":handled,"state":state(&c)}));
            }
            assert_eq!(json!(states), source["states"]);
            assert_eq!(json!(*t.borrow()), source["trace"]);
        }
        let t = Rc::new(RefCell::new(vec![]));
        let c = make(3, t.clone());
        for i in [1, 7, 7, -1] {
            c.set_selected_index(i);
        }
        let source = &cases("programmatic")[0];
        assert_eq!(state(&c), source["state"]);
        assert_eq!(json!(*t.borrow()), source["trace"]);
    });
}
#[test]
fn original_collection_rebuild_and_reentrant_selection_commands() {
    crate::group_tests::run(|| {
        let t = Rc::new(RefCell::new(vec![]));
        let c = make(3, t.clone());
        let old = c.host.borrow().as_ref().unwrap().children().get(0);
        let s = c.segments().get(0).unwrap();
        s.set_label(Some("changed".into()));
        let source = &cases("collection")[0];
        assert_eq!(state(&c), source["stale"]);
        c.segments().set(0, Some(s));
        assert_eq!(state(&c), source["rebuilt"]);
        assert!(!old.ptr_eq(&c.host.borrow().as_ref().unwrap().children().get(0)));
        c.set_selected_index(2);
        t.borrow_mut().clear();
        c.segments().remove_at(0);
        assert_eq!(state(&c), source["removed"]);
        c.segments().clear();
        assert_eq!(state(&c), source["cleared"]);
        assert_eq!(json!(*t.borrow()), source["trace"]);
        for name in [
            "reentrant_index",
            "reentrant_value_command",
            "reentrant_clear",
        ] {
            let t = Rc::new(RefCell::new(vec![]));
            let c = make(3, t.clone());
            let weak = c.downgrade();
            let trace = t.clone();
            c.add_selection_changed(Some(MarkupDelegate::new(move |a| {
                let s = from_markup_value::<Option<Ref<RibbonSegment>>>(&a[1]).unwrap();
                if let Some(s) = s
                    && s.label().as_deref() == Some("B")
                {
                    let c = weak.upgrade().unwrap();
                    match name {
                        "reentrant_index" => c.set_selected_index(2),
                        "reentrant_clear" => c.segments().clear(),
                        _ => {
                            s.set_segment_value(Some(Rc::new("replacement".to_owned())));
                            c.set_command(Some(Rc::new(Cmd(trace.clone(), "new".into()))));
                        }
                    }
                }
                None
            })));
            key(&c, Key::Right);
            let source = &cases(name)[0];
            assert_eq!(state(&c), source["state"], "{name}");
            assert_eq!(json!(*t.borrow()), source["trace"], "{name}");
        }
    });
}
#[test]
fn original_invoke_unchecked_int32_and_parameter_fallback() {
    crate::group_tests::run(|| {
        for source in cases("invoke_edges") {
            let t = Rc::new(RefCell::new(vec![]));
            let c = make(source["count"].as_u64().unwrap() as usize, t.clone());
            c.set_selected_index(source["index"].as_i64().unwrap() as i32);
            t.borrow_mut().clear();
            assert_eq!(c.invoke_core(), source["result"].as_bool().unwrap());
            assert_eq!(state(&c), source["state"]);
            assert_eq!(json!(*t.borrow()), source["trace"]);
        }
        let t = Rc::new(RefCell::new(vec![]));
        let c = make(2, t.clone());
        let s = c.segments().get(0).unwrap();
        s.set_segment_value(None);
        s.set_label(None);
        c.set_command_parameter(Some(Rc::new("parameter".to_owned())));
        key(&c, Key::Enter);
        s.set_label(Some(String::new()));
        key(&c, Key::Space);
        assert_eq!(json!(*t.borrow()), cases("fallback")[0]["trace"]);
    });
}
#[test]
fn original_overflow_is_deferred_and_rejects_mutation_at_first_error_boundary() {
    crate::group_tests::run(|| {
        let t = Rc::new(RefCell::new(vec![]));
        let c = make(2, t.clone());
        let q = c.create_overflow_menu_items_core();
        let s = RibbonSegment::new();
        s.set_label(Some("C".into()));
        c.segments().add(Some(s));
        let mut e = q.iter();
        let first = e
            .next()
            .unwrap()
            .unwrap()
            .cast::<crate::compat::RadioMenuFlyoutItem>()
            .unwrap();
        c.set_selected_index(1);
        let second = e
            .next()
            .unwrap()
            .unwrap()
            .cast::<crate::compat::RadioMenuFlyoutItem>()
            .unwrap();
        let source = &cases("overflow")[0];
        assert_eq!(state(&c), source["before"]);
        assert_eq!(first.text().as_deref(), source["first"].as_str());
        assert_eq!(second.text().as_deref(), source["second"].as_str());
        assert_eq!(first.group_name(), second.group_name());
        assert!(first.group_name().unwrap().parse::<i32>().is_ok());
        assert!(second.is_checked());
        t.borrow_mut().clear();
        first.invoke_menu_core();
        first.invoke_menu_core();
        assert!(e.next().is_some());
        let d = RibbonSegment::new();
        d.set_label(Some("D".into()));
        c.segments().add(Some(d));
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| e.next())).is_err());
        drop(e);
        assert_eq!(
            q.iter().count(),
            4,
            "fresh enumeration sees the added segment"
        );
        assert_eq!(json!(*t.borrow()), source["trace"]);
        assert_eq!(state(&c), source["state"]);
    });
}
#[test]
fn original_layout_retemplate_and_null_segment_boundaries() {
    crate::group_tests::run(|| {
        let t = Rc::new(RefCell::new(vec![]));
        let c = make(2, t);
        let old = c.host.borrow().as_ref().unwrap().clone();
        let first = old.children().get(0);
        let mut layout = RibbonItemLayout::new(RibbonItemSize::Small, c.metrics());
        layout.is_simplified = true;
        layout.show_label = Some(false);
        c.apply_layout(layout);
        let source = &cases("layout_retemplate")[0];
        assert_eq!(state(&c), source["layout"]);
        assert!(!first.ptr_eq(&old.children().get(0)));
        theme(&c);
        assert_eq!(state(&c), source["state"]);
        assert_eq!(old.children().count(), source["oldCount"]);
        assert_eq!(
            c.host.borrow().as_ref().unwrap().children().count(),
            source["newCount"]
        );
        let c = RibbonSegmentedControl::new();
        c.segments().add(None);
        c.set_selected_index(1);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| theme(&c))).is_err());
        assert_eq!(state(&c), cases("null_segment")[0]["state"]);
    });
}
#[test]
fn original_factory_scope_replacement_selection_and_disposal() {
    crate::group_tests::run(|| {
        use ferroribbon_core::model::*;
        let m = RibbonSegmentedModel::new(Some("work".into()), Some("Workspace".into()));
        let a = RibbonSegmentModel::new(Some("a".into()), Some("A".into()));
        let b = RibbonSegmentModel::new(Some("b".into()), Some("B".into()));
        m.segments().add(Some(a.clone()));
        m.segments().add(Some(b.clone()));
        m.set_selected_segment(Some(b.clone()));
        let bindings = RibbonModelBindings::new();
        let c = RibbonElementFactory::new()
            .create_segmented(m.clone(), bindings.clone())
            .cast::<RibbonSegmentedControl>()
            .unwrap();
        theme(&c);
        let source = &cases("factory")[0];
        assert_eq!(state(&c), source["initial"]);
        let old = c.segments().get(1).unwrap();
        b.set_label(Some("updated".into()));
        assert_eq!(state(&c), source["changed"]);
        assert!(!old.ptr_eq(&c.segments().get(1).unwrap()));
        m.segments().move_item(1, 0);
        assert_eq!(state(&c), source["moved"]);
        c.set_selected_index(1);
        assert_eq!(
            m.selected_segment()
                .unwrap()
                .id()
                .unwrap()
                .to_utf8()
                .unwrap(),
            source["selected"]
        );
        m.segments().remove(&Some(b.clone()));
        b.set_label(Some("detached".into()));
        assert_eq!(state(&c), source["removed"]);
        bindings.dispose();
        a.set_label(Some("disposed".into()));
        c.set_selected_index(4);
        assert_eq!(state(&c), source["disposed"]);
        assert_eq!(
            m.selected_segment()
                .unwrap()
                .id()
                .unwrap()
                .to_utf8()
                .unwrap(),
            source["modelSelected"]
        );
    });
}

#[test]
fn original_click_order_repeated_choice_and_reentrant_detached_button() {
    crate::group_tests::run(|| {
        for source in cases("click") {
            let trace = Rc::new(RefCell::new(vec![]));
            let c = make(3, trace.clone());
            let mode = source["mode"].as_str().unwrap().to_owned();
            if mode != "normal" {
                let weak = c.downgrade();
                c.add_selection_changed(Some(MarkupDelegate::new(move |a| {
                    let s = from_markup_value::<Option<Ref<RibbonSegment>>>(&a[1]).unwrap();
                    if s.and_then(|s| s.label()).as_deref() == Some("B") {
                        let c = weak.upgrade().unwrap();
                        if mode == "index" {
                            c.set_selected_index(2)
                        } else {
                            c.segments().clear();
                        }
                    }
                    None
                })));
            }
            let host = c.host.borrow().as_ref().unwrap().clone();
            let first = host.children().get(0).cast::<RibbonToggleButton>().unwrap();
            let second = host.children().get(1).cast::<RibbonToggleButton>().unwrap();
            let click = |b: &RibbonToggleButton| {
                ferroui_controls::automation::peers::ToggleButtonAutomationPeer::new(b)
                    .toggle()
                    .unwrap()
            };
            click(&first);
            click(&second);
            click(&second);
            assert_eq!(
                json!(*trace.borrow()),
                source["trace"],
                "{}",
                source["mode"]
            );
            assert_eq!(state(&c), source["state"]);
            assert_eq!(json!(second.is_checked()), source["retainedChecked"]);
        }
    });
}

#[test]
fn segmented_native_lifetime_audit_records_clr_retention_gap_without_cycles() {
    crate::group_tests::run(|| {
        let source = cases("retained_graph");
        for kind in ["collection", "button", "menu"] {
            assert_eq!(
                source.iter().find(|v| v["kind"] == kind).unwrap()["alive"],
                true,
                "source retains through its delegate"
            );
            let fixture = make(1, Rc::new(RefCell::new(vec![])));
            let weak = fixture.downgrade();
            let collection = fixture.segments();
            let button = fixture.host.borrow().as_ref().unwrap().children().get(0);
            let menu = fixture
                .create_overflow_menu_items_core()
                .iter()
                .next()
                .unwrap()
                .unwrap();
            let keeper: BoxedValue = match kind {
                "collection" => Rc::new(collection.clone()),
                "button" => Control::boxed(button),
                _ => Control::boxed(menu),
            };
            drop(fixture);
            assert!(
                weak.upgrade().is_none(),
                "known native weak graph differs from CLR; no parity claim"
            );
            collection.clear();
            drop(keeper);
        }
        assert_eq!(
            source.iter().find(|v| v["kind"] == "none").unwrap()["alive"],
            false
        );
        assert_eq!(
            source.iter().find(|v| v["kind"] == "query").unwrap()["alive"],
            true
        );
        let control = RibbonSegmentedControl::new();
        let weak = control.downgrade();
        let query = control.create_overflow_menu_items_core();
        drop(control);
        assert!(weak.upgrade().is_some());
        drop(query);
        assert!(weak.upgrade().is_none());
    });
}

#[test]
fn segmented_factory_preserves_lossless_core_string_payload_allocation() {
    crate::group_tests::run(|| {
        use ferroribbon_core::model::*;
        let value = ferroribbon_core::RibbonString::from_utf16(vec![0xd800, 0x0061]);
        let segment = RibbonSegmentModel::with_options(
            Some("id".into()),
            Some("Label".into()),
            None,
            Some(RibbonObject::value(value.clone())),
        );
        let model = RibbonSegmentedModel::new(None, None);
        model.segments().add(Some(segment));
        let bindings = RibbonModelBindings::new();
        let control = RibbonElementFactory::new()
            .create_segmented(model, bindings.clone())
            .cast::<RibbonSegmentedControl>()
            .unwrap();
        let payload = control.segments().get(0).unwrap().value().unwrap();
        let actual = payload
            .downcast_ref::<ferroribbon_core::RibbonString>()
            .unwrap();
        assert!(actual.ptr_eq(&value));
        assert_eq!(actual.as_utf16(), &[0xd800, 0x0061]);
        assert!(actual.to_utf8().is_err());
        bindings.dispose();
    });
}

#[test]
fn segmented_physical_keyboard_routing_is_unchanged_in_rtl_and_preserves_labels() {
    crate::group_tests::run(|| {
        let reference = cases("keyboard")
            .into_iter()
            .find(|v| v["count"] == 3 && v["initial"] == 0)
            .unwrap();
        for direction in [
            ferroui_base::media::FlowDirection::LeftToRight,
            ferroui_base::media::FlowDirection::RightToLeft,
        ] {
            let trace = Rc::new(RefCell::new(vec![]));
            let c = make(3, trace.clone());
            c.set_flow_direction(direction);
            let mut states = vec![];
            for (name, k) in [
                ("Left", Key::Left),
                ("Right", Key::Right),
                ("Home", Key::Home),
                ("End", Key::End),
                ("Enter", Key::Enter),
                ("Space", Key::Space),
                ("Up", Key::Up),
            ] {
                let handled = key(&c, k);
                states.push(json!({"key":name,"Handled":handled,"state":state(&c)}));
            }
            assert_eq!(json!(states), reference["states"]);
            assert_eq!(json!(*trace.borrow()), reference["trace"]);
        }
        let trace = Rc::new(RefCell::new(vec![]));
        let c = make(0, trace.clone());
        for label in [None, Some(""), Some("אב e\u{301} 🦀")] {
            let segment = RibbonSegment::new();
            segment.set_label(label.map(str::to_owned));
            c.segments().add(Some(segment));
        }
        let reference = &cases("unicode_labels")[0];
        assert_eq!(state(&c), reference["initial"]);
        key(&c, Key::Right);
        key(&c, Key::End);
        assert_eq!(state(&c), reference["state"]);
        assert_eq!(json!(*trace.borrow()), reference["trace"]);
    });
}
