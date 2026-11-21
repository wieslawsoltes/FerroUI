use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::{commands::RibbonCommandCatalog, model::*, *};
use ferroui_system::{
    collections::object_model::ObservableCollection, component_model::INotifyPropertyChanged,
    events::Event,
};
use serde_json::{Value, json};
use std::{
    any::Any,
    cell::RefCell,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../../tests/fixtures/root-model.json")).unwrap()
}
fn s(v: &str) -> Option<RibbonString> {
    Some(v.into())
}
fn string(v: Option<RibbonString>) -> Option<String> {
    v.map(|v| v.to_utf8().unwrap())
}
fn node_ids(values: impl IntoIterator<Item = RibbonNodeReference>) -> Value {
    json!(
        values
            .into_iter()
            .map(|v| v.and_then(|v| string(v.id())))
            .collect::<Vec<_>>()
    )
}
fn item_ids(values: impl IntoIterator<Item = RibbonItemReference>) -> Value {
    node_ids(
        values
            .into_iter()
            .map(|v| v.map(|v| v as Rc<dyn RibbonNodeModelImpl>)),
    )
}
fn tab_ids(values: impl IntoIterator<Item = RibbonTabReference>) -> Value {
    node_ids(
        values
            .into_iter()
            .map(|v| v.map(|v| v as Rc<dyn RibbonNodeModelImpl>)),
    )
}
fn button(id: &str, command: Option<&str>) -> Rc<RibbonButtonModel> {
    let v = RibbonButtonModel::new(s(id), None);
    v.set_command_id(command.and_then(s));
    v
}
struct Graph {
    model: Rc<RibbonModel>,
    group: Rc<RibbonGroupModel>,
    container: Rc<RibbonButtonGroupModel>,
    toggle: Rc<RibbonToggleButtonModel>,
    split: Rc<RibbonSplitButtonModel>,
}
fn graph() -> Graph {
    let m = RibbonModel::new();
    let t = RibbonTabModel::new(s("home"), None);
    let g = RibbonGroupModel::new(s("group"), None);
    let container = RibbonButtonGroupModel::new(s("container"));
    let toggle = RibbonToggleButtonModel::new(s("toggle"), None);
    toggle.set_command_id(s("run"));
    let row = RibbonRowModel::new(s("row"));
    row.items().add(Some(toggle.clone()));
    row.items().add(Some(button("inner", Some("RUN"))));
    container.items().add(Some(row));
    let split = RibbonSplitButtonModel::new(s("split"), None);
    split.set_command_id(s("run"));
    let menu = RibbonMenuItemModel::new(s("menu"), None);
    menu.set_command_id(s("run"));
    let submenu = RibbonMenuItemModel::new(s("submenu"), None);
    submenu.set_command_id(s("run"));
    menu.items().add(Some(submenu));
    split.menu_items().add(Some(menu));
    let gallery = RibbonGalleryModel::new(s("gallery"), None);
    let gm = RibbonMenuItemModel::new(s("gallery.menu"), None);
    gm.set_command_id(s("run"));
    gallery.menu_items().add(Some(gm));
    let color = RibbonColorPickerModel::new(s("color"), None);
    let check = RibbonCheckBoxModel::new(s("check"), None);
    check.set_command_id(s("run"));
    for v in [
        container.clone() as Rc<dyn RibbonItemModelImpl>,
        split.clone(),
        gallery,
        color,
        check,
    ] {
        g.items().add(Some(v));
    }
    g.slide_out_items().add(Some(button("slide", Some("run"))));
    t.groups().add(Some(g.clone()));
    m.tabs().add(Some(t));
    m.tab_strip_items().add(Some(button("strip", Some("run"))));
    m.tab_strip_items().add(Some(toggle.clone()));
    m.quick_access_items().add(Some(toggle.clone()));
    m.quick_access_items().add(Some(button("qat", Some("run"))));
    m.quick_access_candidates()
        .add(Some(button("candidate", Some("run"))));
    let back = RibbonBackstageItemModel::new(s("back"), None);
    back.set_command_id(s("run"));
    m.backstage().items().add(Some(back));
    m.contextual_groups()
        .add(Some(RibbonContextualGroupModel::new(s("ctx"), None)));
    let contextual = RibbonTabModel::new(s("context"), None);
    contextual.set_contextual_group_id(s("ctx"));
    m.tabs().add(Some(contextual));
    Graph {
        model: m,
        group: g,
        container,
        toggle,
        split,
    }
}
fn snapshot(m: &dyn RibbonModelImpl) -> Value {
    json!({"SelectedTabId":string(m.selected_tab_id()),"DisplayMode":m.display_mode().raw(),"VisibilityMode":m.visibility_mode().raw(),"Density":m.density().raw(),"QuickAccessPosition":m.quick_access_position().raw(),"IsQuickAccessVisible":m.is_quick_access_visible(),"ShowQuickAccessLabels":m.show_quick_access_labels(),"MinimizeBehavior":m.minimize_behavior().raw(),"ShowGroupCaptions":m.show_group_captions(),"ApplicationButtonLabel":string(m.application_button_label()),"IsApplicationButtonVisible":m.is_application_button_visible(),"Title":string(m.title()),"Catalog":m.command_catalog().is_some(),"Tabs":m.tabs().count(),"Groups":m.contextual_groups().count(),"Qat":m.quick_access_items().count(),"Candidates":m.quick_access_candidates().count(),"Strip":m.tab_strip_items().count(),"Backstage":true})
}
fn ids(values: Vec<Option<RibbonString>>) -> RibbonContextualGroupIds {
    RibbonFixedItems::new(values)
}
#[test]
fn defaults_properties_and_source_notifications() {
    let data = corpus();
    let m = RibbonModel::new();
    assert_eq!(snapshot(m.as_ref()), data["properties"]["defaults"]);
    let trace = Rc::new(RefCell::new(vec![]));
    let log = trace.clone();
    m.property_changed()
        .add(Rc::new(move |v| log.borrow_mut().push(v.to_owned())));
    m.set_selected_tab_id(s("x"));
    m.set_selected_tab_id(s("x"));
    m.set_selected_tab_id(None);
    m.set_display_mode(RibbonDisplayMode::from_raw(99));
    m.set_visibility_mode(RibbonVisibilityMode::from_raw(-1));
    m.set_density(RibbonDensity::from_raw(99));
    m.set_quick_access_position(RibbonQuickAccessPosition::from_raw(99));
    m.set_is_quick_access_visible(false);
    m.set_show_quick_access_labels(true);
    m.set_minimize_behavior(RibbonMinimizeBehavior::from_raw(99));
    m.set_show_group_captions(false);
    m.set_application_button_label(None);
    m.set_is_application_button_visible(false);
    m.set_title(s("title"));
    let catalog = RibbonCommandCatalog::new();
    m.set_command_catalog(Some(catalog.clone()));
    m.set_command_catalog(Some(catalog));
    m.set_command_catalog(None);
    assert_eq!(snapshot(m.as_ref()), data["properties"]["changed"]);
    assert_eq!(json!(*trace.borrow()), data["properties"]["trace"]);
    assert!(m.tabs().ptr_eq(&m.tabs()));
    assert!(m.quick_access_items().ptr_eq(&m.quick_access_items()));
    assert!(Rc::ptr_eq(&m.backstage(), &m.backstage()));
    let malformed = RibbonString::from_utf16(vec![0xd800, 0, 0xdc00]);
    m.set_title(Some(malformed.clone()));
    assert_eq!(m.title(), Some(malformed));
}
#[test]
fn selection_resolves_first_id_and_nested_notifications() {
    let expected = &corpus()["selection"];
    let m = RibbonModel::new();
    let first = RibbonTabModel::new(s("same"), None);
    let second = RibbonTabModel::new(s("same"), None);
    m.tabs().add(Some(first.clone()));
    m.tabs().add(Some(second.clone()));
    m.set_selected_tab(Some(second.clone()));
    let current = m.selected_tab().unwrap();
    assert_eq!(
        std::ptr::addr_eq(Rc::as_ptr(&current), Rc::as_ptr(&first)),
        expected["resolvesFirst"].as_bool().unwrap()
    );
    m.tabs().remove(&Some(first));
    assert_eq!(
        std::ptr::addr_eq(Rc::as_ptr(&m.selected_tab().unwrap()), Rc::as_ptr(&second)),
        expected["resolvesSecond"].as_bool().unwrap()
    );
    let trace = Rc::new(RefCell::new(vec![]));
    let log = trace.clone();
    let weak = Rc::downgrade(&m);
    m.property_changed().add(Rc::new(move |v| {
        log.borrow_mut().push(v.to_owned());
        let owner = weak.upgrade().unwrap();
        if v == "SelectedTabId" && owner.selected_tab_id() == s("outer") {
            owner.set_selected_tab_id(s("inner"));
        }
    }));
    m.set_selected_tab_id(s("outer"));
    m.set_selected_tab(Some(RibbonTabModel::new(s("external"), None)));
    assert_eq!(
        m.selected_tab().is_none(),
        expected["externalMissing"].as_bool().unwrap()
    );
    second.set_id(None);
    m.set_selected_tab(None);
    assert_eq!(
        std::ptr::addr_eq(Rc::as_ptr(&m.selected_tab().unwrap()), Rc::as_ptr(&second)),
        expected["nullResolves"].as_bool().unwrap()
    );
    assert_eq!(json!(*trace.borrow()), expected["trace"]);
}
#[test]
fn original_queries_paths_and_command_state_reach_menus_and_backstage() {
    let data = corpus();
    let expected = &data["queries"];
    let x = graph();
    let m = &x.model;
    assert_eq!(item_ids(m.clone().enumerate_items()), expected["items"]);
    let paths = m
        .clone()
        .enumerate_items_with_path()
        .iter()
        .map(|(t, g, i)| {
            format!(
                "{}/{}/{}",
                string(t.id()).unwrap(),
                string(g.id()).unwrap(),
                string(i.unwrap().id()).unwrap()
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(json!(paths), expected["paths"]);
    assert_eq!(
        item_ids(m.clone().find_items_by_command(s("RuN"))),
        expected["byItems"]
    );
    assert_eq!(
        node_ids(m.find_nodes_by_command(s("RUN"))),
        expected["byNodes"]
    );
    let query = m.find_nodes_by_command(s("run"));
    x.group.items().add(Some(button("late.item", Some("run"))));
    let late = RibbonMenuItemModel::new(s("late.menu"), None);
    late.set_command_id(s("run"));
    x.split.menu_items().add(Some(late));
    let late = RibbonBackstageItemModel::new(s("late.back"), None);
    late.set_command_id(s("run"));
    m.backstage().items().add(Some(late));
    assert_eq!(node_ids(&query), expected["captured"]);
    assert_eq!(node_ids(&query), expected["repeated"]);
    m.set_command_enabled(s("rUn"), false);
    m.set_command_checked(s("run"), true);
    let enabled = m
        .find_nodes_by_command(s("run"))
        .iter()
        .map(|n| {
            let n = n.unwrap();
            json!({"Id":string(n.id()),"IsEnabled":n.is_enabled()})
        })
        .collect::<Vec<_>>();
    assert_eq!(json!(enabled), expected["enabled"]);
    let check = try_cast_node::<dyn RibbonCheckBoxModelImpl>(
        m.find_item(s("check"))
            .map(|v| v as Rc<dyn RibbonNodeModelImpl>),
    )
    .unwrap()
    .unwrap();
    let menu = try_cast_node::<dyn RibbonMenuItemModelImpl>(x.split.menu_items().get(0))
        .unwrap()
        .unwrap();
    assert_eq!(
        json!([
            Some(x.toggle.is_checked()),
            Some(x.split.is_checked()),
            check.is_checked(),
            Some(menu.is_checked())
        ]),
        expected["checkedValues"]
    );
    assert_eq!(tab_ids(m.clone().visible_tabs()), expected["before"]);
    m.set_active_contextual_groups(Some(ids(vec![s("ctx")])));
    assert_eq!(tab_ids(m.clone().visible_tabs()), expected["after"]);
    assert!(
        try_cast_node::<dyn RibbonToggleButtonModelImpl>(
            m.find_item(s("toggle"))
                .map(|v| v as Rc<dyn RibbonNodeModelImpl>)
        )
        .unwrap()
        .is_some()
    );
    assert_eq!(
        string(m.find_group(s("group")).unwrap().id()),
        Some("group".into())
    );
    assert_eq!(
        string(m.find_node(s("ctx")).unwrap().id()),
        Some("ctx".into())
    );
}
#[test]
fn live_queries_match_reference_through_first_error_boundary() {
    for row in corpus()["steps"].as_array().unwrap() {
        let x = graph();
        let m = &x.model;
        let kind = row["kind"].as_str().unwrap();
        let query = match kind {
            "items" => RibbonItemsSource::new({
                let q = m.clone().enumerate_items();
                move || {
                    Box::new(
                        q.iter()
                            .map(|v| v.map(|v| v as Rc<dyn RibbonNodeModelImpl>)),
                    )
                }
            }),
            "paths" => RibbonItemsSource::new({
                let q = m.clone().enumerate_items_with_path();
                move || {
                    Box::new(
                        q.iter()
                            .map(|(_, _, v)| v.map(|v| v as Rc<dyn RibbonNodeModelImpl>)),
                    )
                }
            }),
            "visible" => RibbonItemsSource::new({
                let q = m.clone().visible_tabs();
                move || {
                    Box::new(
                        q.iter()
                            .map(|v| v.map(|v| v as Rc<dyn RibbonNodeModelImpl>)),
                    )
                }
            }),
            "command-items" => RibbonItemsSource::new({
                let q = m.clone().find_items_by_command(s("run"));
                move || {
                    Box::new(
                        q.iter()
                            .map(|v| v.map(|v| v as Rc<dyn RibbonNodeModelImpl>)),
                    )
                }
            }),
            "command-nodes" => m.find_nodes_by_command(s("run")),
            _ => RibbonItemsSource::new({
                let q =
                    RibbonModel::flatten(Some(RibbonItemsSource::from_collection(x.group.items())));
                move || {
                    Box::new(
                        q.iter()
                            .map(|v| v.map(|v| v as Rc<dyn RibbonNodeModelImpl>)),
                    )
                }
            }),
        };
        let mut iterator = query.iter();
        let mut values = vec![];
        let step = |iterator: &mut RibbonIterator<RibbonNodeReference>| match catch_unwind(
            AssertUnwindSafe(|| iterator.next()),
        ) {
            Ok(Some(v)) => format!(
                "value:{}",
                v.and_then(|v| string(v.id())).unwrap_or_default()
            ),
            Ok(None) => "end".into(),
            Err(_) => "error".into(),
        };
        if row["timing"] == "after" {
            values.push(step(&mut iterator));
        }
        if row["timing"] == "end" {
            loop {
                let value = step(&mut iterator);
                let end = value == "end";
                values.push(value);
                if end {
                    break;
                }
            }
        }
        if kind == "visible" {
            m.tabs().add(Some(RibbonTabModel::new(s("late.tab"), None)));
        } else {
            x.group.items().add(Some(button("late", Some("run"))));
        }
        for _ in 0..24 {
            let value = step(&mut iterator);
            let failed = value == "error";
            values.push(value);
            if failed {
                break;
            }
        }
        // Rust Iterator has no recovery contract after a caught panic. Preserve
        // every source value and the FIRST error boundary; discard the failed
        // iterator rather than emulating C# MoveNext after unwinding.
        let expected = row["values"].as_array().unwrap();
        let end = expected
            .iter()
            .position(|v| v == "error")
            .map_or(expected.len(), |i| i + 1);
        assert_eq!(
            json!(values),
            json!(&expected[..end]),
            "{kind} {}",
            row["timing"]
        );
    }
}
#[test]
fn children_and_slide_out_enumerators_start_at_original_yield_boundaries() {
    let x = graph();
    let mut iter =
        RibbonModel::flatten(Some(RibbonItemsSource::from_collection(x.group.items()))).iter();
    let prefix = string(iter.next().unwrap().unwrap().id());
    x.container.items().add(Some(button("late.child", None)));
    let result = item_ids(iter);
    let mut concat = x.model.clone().enumerate_items().iter();
    concat.next();
    x.group
        .slide_out_items()
        .add(Some(button("late.slide", None)));
    let remaining = item_ids(concat);
    let data = corpus();
    assert_eq!(json!(prefix), data["deferred"]["prefix"]);
    assert_eq!(result, data["deferred"]["result"]);
    assert_eq!(remaining, data["deferred"]["remaining"]);
}
#[test]
fn null_entries_and_null_inputs_preserve_failure_timing() {
    let data = corpus();
    let expected = &data["nulls"];
    let m = RibbonModel::new();
    m.quick_access_items().add(None);
    m.backstage()
        .items()
        .add(Some(RibbonBackstageItemModel::new(s("back"), None)));
    assert_eq!(node_ids(m.find_nodes_by_command(None)), expected["nodes"]);
    assert!(
        catch_unwind(AssertUnwindSafe(|| m
            .clone()
            .find_items_by_command(None)
            .iter()
            .collect::<Vec<_>>()))
        .is_err()
    );
    assert!(!expected["itemError"].is_null());
    let flat = RibbonModel::flatten(None);
    assert!(catch_unwind(AssertUnwindSafe(|| flat.iter().collect::<Vec<_>>())).is_err());
    assert!(!expected["flatError"].is_null());
    m.set_active_contextual_groups(None);
    assert!(expected["activeEmptyError"].is_null());
    let group = RibbonContextualGroupModel::new(s("ctx"), None);
    group.set_is_visible(true);
    m.contextual_groups().add(Some(group.clone()));
    m.set_active_contextual_groups(None);
    assert!(expected["activeError"].is_null());
    assert_eq!(json!(group.is_visible()), expected["activeVisible"]);
    assert!(
        expected["activeCalls"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "System.MemoryExtensions.Contains")
    );
    m.contextual_groups().add(None);
    assert!(catch_unwind(AssertUnwindSafe(|| m.set_active_contextual_groups(None))).is_err());
    assert_eq!(expected["nullGroupError"], "NullReferenceException");
}
struct EqualButton {
    base: RibbonButtonModel,
    trace: Rc<RefCell<Vec<String>>>,
    hash: i32,
    key: &'static str,
}
impl INotifyPropertyChanged for EqualButton {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for EqualButton {}
impl RibbonNodeModelImpl for EqualButton {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.item_model().node_model()
    }
    fn node_hash_code(&self) -> i32 {
        self.trace
            .borrow_mut()
            .push(format!("hash:{}", string(self.id()).unwrap()));
        self.hash
    }
    fn equals_node(&self, other: &dyn RibbonNodeModelImpl) -> bool {
        self.trace.borrow_mut().push(format!(
            "equals:{}:{}",
            string(self.id()).unwrap(),
            string(other.id()).unwrap_or_default()
        ));
        (other as &dyn Any)
            .downcast_ref::<Self>()
            .is_some_and(|b| b.key == self.key)
    }
}
impl RibbonItemModelImpl for EqualButton {
    fn item_model(&self) -> &RibbonItemModel {
        self.base.item_model()
    }
}
impl RibbonButtonModelImpl for EqualButton {
    fn button_model(&self) -> &RibbonButtonModel {
        &self.base
    }
}
#[test]
fn distinct_uses_hash_and_existing_equals_in_collision_chain_order() {
    register_ribbon_button_model::<EqualButton>().unwrap();
    let m = RibbonModel::new();
    let trace = Rc::new(RefCell::new(vec![]));
    for (id, hash, key) in [("a", 7, "x"), ("b", 7, "y"), ("c", 7, "x"), ("d", 8, "x")] {
        let v = Rc::new(EqualButton {
            base: RibbonButtonModel::construct(s(id), None),
            trace: trace.clone(),
            hash,
            key,
        });
        v.set_command_id(s("run"));
        m.quick_access_items().add(Some(v));
    }
    let expected = &corpus()["equality"];
    assert_eq!(
        item_ids(m.find_items_by_command(s("run"))),
        expected["values"]
    );
    assert_eq!(json!(*trace.borrow()), expected["trace"]);
}
#[test]
fn contextual_id_array_remains_aliased_during_notifications() {
    let m = RibbonModel::new();
    let a = RibbonContextualGroupModel::new(s("a"), None);
    let b = RibbonContextualGroupModel::new(s("b"), None);
    m.contextual_groups().add(Some(a.clone()));
    m.contextual_groups().add(Some(b.clone()));
    let values = ids(vec![s("a")]);
    let captured = values.clone();
    a.property_changed()
        .add(Rc::new(move |_| captured.set(0, s("b"))));
    m.set_active_contextual_groups(Some(values));
    assert_eq!(
        json!({"a":a.is_visible(),"b":b.is_visible()}),
        corpus()["aliased"]
    );
}
#[test]
fn traversal_keeps_owner_and_parents_alive_until_disposal() {
    let x = graph();
    let owner = Rc::downgrade(&x.model);
    let mut iterator = x.model.clone().enumerate_items().iter();
    drop(x);
    assert!(owner.upgrade().is_some());
    assert!(iterator.next().is_some());
    drop(iterator);
    assert!(owner.upgrade().is_none());
    let x = graph();
    let tab = x.model.tabs().get(0).unwrap();
    let tab_weak = Rc::downgrade(&tab);
    let group_weak = Rc::downgrade(&x.group);
    let mut iterator = x.model.clone().enumerate_items().iter();
    iterator.next();
    x.model.tabs().clear();
    tab.groups().clear();
    drop(tab);
    let model = x.model.clone();
    drop(x);
    assert!(tab_weak.upgrade().is_some());
    assert!(group_weak.upgrade().is_some());
    drop(iterator);
    assert!(tab_weak.upgrade().is_none());
    assert!(group_weak.upgrade().is_none());
    drop(model);
}
#[test]
fn repeatable_flatten_yields_nulls_and_independent_cursors() {
    let items = ObservableCollection::<RibbonItemReference>::new();
    items.add(None);
    let q = RibbonModel::flatten(Some(RibbonItemsSource::from_collection(items.clone())));
    let mut a = q.iter();
    let mut b = q.iter();
    assert!(a.next().unwrap().is_none());
    assert!(b.next().unwrap().is_none());
    items.add(Some(button("later", None)));
    assert!(catch_unwind(AssertUnwindSafe(|| a.next())).is_err());
    // Native callers dispose a failed iterator. A fresh traversal observes the
    // changed live collection; no CLR iterator completion/recovery is simulated.
    drop(a);
    drop(b);
    assert_eq!(item_ids(&q), json!([null, "later"]));
}

#[test]
fn lookup_precedence_duplicate_ids_and_ordinal_matching() {
    let m = RibbonModel::new();
    let t = RibbonTabModel::new(s("id"), None);
    let g = RibbonGroupModel::new(s("id"), None);
    let item = button("id", None);
    g.items().add(Some(item.clone()));
    t.groups().add(Some(g.clone()));
    m.tabs().add(Some(t.clone()));
    let ctx = RibbonContextualGroupModel::new(s("id"), None);
    m.contextual_groups().add(Some(ctx.clone()));
    let back = RibbonBackstageItemModel::new(s("id"), None);
    m.backstage().items().add(Some(back.clone()));
    let name = || m.find_node(s("id")).map(|v| v.model_type_name().to_owned());
    let mut names = vec![name()];
    t.set_id(s("tab"));
    names.push(name());
    g.set_id(s("group"));
    names.push(name());
    item.set_id(s("item"));
    names.push(name());
    ctx.set_id(s("ctx"));
    names.push(name());
    back.set_id(s("back"));
    names.push(name());
    let expected = &corpus()["precedence"];
    assert_eq!(json!(names), expected["names"]);
    assert_eq!(
        json!(m.find_tab(s("TAB")).is_some()),
        expected["insensitive"]
    );
    item.set_id(None);
    let found = m.find_item(None).unwrap();
    assert_eq!(
        json!(std::ptr::addr_eq(Rc::as_ptr(&found), Rc::as_ptr(&item))),
        expected["nullLookup"]
    );
}
#[test]
fn reentrant_state_updates_preserve_eager_capture_later_menus_and_partial_writes() {
    for row in corpus()["reentrant"].as_array().unwrap() {
        let x = graph();
        let trace = Rc::new(RefCell::new(vec![]));
        let log = trace.clone();
        let late_menu = RibbonMenuItemModel::new(s("late.menu"), None);
        late_menu.set_command_id(s("run"));
        let subscription = if row["invalidate"] == true {
            let parent = x.split.menu_items().get(0).unwrap();
            let roots = x.split.menu_items();
            let token = parent.property_changed().add(Rc::new(move |v| {
                if v == "IsEnabled" {
                    log.borrow_mut().push("mutate:menu");
                    roots.add(Some(late_menu.clone()));
                }
            }));
            (parent, token)
        } else {
            let primary = x.group.items();
            let roots = x.split.menu_items();
            let backstage = x.model.backstage().items();
            let late = button("late", Some("run"));
            let late_back = RibbonBackstageItemModel::new(s("late.back"), None);
            late_back.set_command_id(s("run"));
            let token = x.toggle.property_changed().add(Rc::new(move |v| {
                if v == "IsEnabled" {
                    log.borrow_mut().push("mutate:item");
                    primary.add(Some(late.clone()));
                    roots.add(Some(late_menu.clone()));
                    backstage.add(Some(late_back.clone()));
                }
            }));
            (x.toggle.clone() as Rc<dyn RibbonNodeModelImpl>, token)
        };
        let result = catch_unwind(AssertUnwindSafe(|| {
            x.model.set_command_enabled(s("run"), false)
        }));
        assert_eq!(result.is_err(), !row["error"].is_null());
        assert_eq!(json!(*trace.borrow()), row["trace"]);
        let states = x
            .model
            .find_nodes_by_command(s("run"))
            .iter()
            .map(|n| {
                let n = n.unwrap();
                json!({"Id":string(n.id()),"IsEnabled":n.is_enabled()})
            })
            .collect::<Vec<_>>();
        assert_eq!(json!(states), row["states"]);
        subscription.0.property_changed().remove(subscription.1);
    }
}
#[test]
fn nested_menu_children_are_read_after_the_parent_is_yielded() {
    let x = graph();
    let mut iterator = x.model.find_nodes_by_command(s("run")).iter();
    let mut prefix = vec![];
    for node in iterator.by_ref() {
        let node = node.unwrap();
        let id = string(node.id()).unwrap();
        prefix.push(id.clone());
        if id == "menu" {
            break;
        }
    }
    let parent = try_cast_node::<dyn RibbonMenuItemModelImpl>(x.split.menu_items().get(0))
        .unwrap()
        .unwrap();
    let child = RibbonMenuItemModel::new(s("late.child"), None);
    child.set_command_id(s("run"));
    parent.items().add(Some(child));
    let expected = &corpus()["deferredMenu"];
    assert_eq!(json!(prefix), expected["prefix"]);
    assert_eq!(node_ids(iterator), expected["rest"]);
}

#[test]
fn item_sources_defer_creation_and_raii_dispose_on_break_completion_and_failure() {
    struct Probe {
        trace: Rc<RefCell<Vec<String>>>,
        fail: bool,
        index: i32,
        items: Vec<RibbonItemReference>,
    }
    impl Iterator for Probe {
        type Item = RibbonItemReference;
        fn next(&mut self) -> Option<Self::Item> {
            self.index += 1;
            self.trace.borrow_mut().push(format!("next:{}", self.index));
            assert!(!(self.fail && self.index == 1), "iterator failure");
            self.items.get(self.index as usize).cloned().inspect(|_| {
                self.trace
                    .borrow_mut()
                    .push(format!("current:{}", self.index))
            })
        }
    }
    impl Drop for Probe {
        fn drop(&mut self) {
            self.trace.borrow_mut().push("dispose".into());
        }
    }
    for row in corpus()["disposal"].as_array().unwrap() {
        let trace = Rc::new(RefCell::new(vec![]));
        let log = trace.clone();
        let fail = row["mode"] == "fault";
        let query = RibbonModel::flatten(Some(RibbonItemsSource::new(move || {
            log.borrow_mut().push("get".into());
            Box::new(Probe {
                trace: log.clone(),
                fail,
                index: -1,
                items: vec![Some(button("a", None)), Some(button("b", None))],
            })
        })));
        trace.borrow_mut().push("query".into());
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            // Own the iterator inside the fallible operation. Rust unwinding
            // drops it before the caller handles the error, as required by RAII.
            let mut iterator = query.iter();
            trace.borrow_mut().push("iterator".into());
            for item in iterator.by_ref() {
                trace
                    .borrow_mut()
                    .push(format!("value:{}", string(item.unwrap().id()).unwrap()));
                if row["mode"] == "early" {
                    break;
                }
            }
        }));
        if outcome.is_err() {
            trace.borrow_mut().push("error".into());
        }
        trace.borrow_mut().push("done".into());
        let expected: Vec<_> = row["trace"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|value| row["mode"] != "fault" || *value != "end")
            .cloned()
            .collect();
        assert_eq!(json!(*trace.borrow()), json!(expected));
    }
}

#[test]
fn upstream_model_lookup_context_command_and_selected_tab_tests() {
    // Original ModelTests.CreateModel topology and assertions, without control dependencies.
    let model = RibbonModel::new();
    let home = RibbonTabModel::new(s("home"), s("Home"));
    let clipboard = RibbonGroupModel::new(s("clipboard"), s("Clipboard"));
    let paste = RibbonSplitButtonModel::new(s("paste"), s("Paste"));
    paste.set_icon(Some(RibbonIcon::glyph(s(RibbonIcons::Paste))));
    paste.set_command_id(s("paste"));
    clipboard.items().add(Some(paste));
    let cut = button("cut", Some("cut"));
    cut.set_label(s("Cut"));
    cut.set_icon(Some(RibbonIcon::glyph(s(RibbonIcons::Cut))));
    clipboard.items().add(Some(cut));
    home.groups().add(Some(clipboard));
    let font = RibbonGroupModel::new(s("font"), s("Font"));
    font.set_items_layout(RibbonGroupItemsLayout::Rows);
    let row = RibbonRowModel::new(s("font-row2"));
    let bis = RibbonButtonGroupModel::new(s("bis"));
    let bold = RibbonToggleButtonModel::new(s("bold"), s("Bold"));
    bold.set_icon(Some(RibbonIcon::glyph(s(RibbonIcons::Bold))));
    bold.set_command_id(s("bold"));
    bis.items().add(Some(bold));
    let italic = RibbonToggleButtonModel::new(s("italic"), s("Italic"));
    italic.set_icon(Some(RibbonIcon::glyph(s(RibbonIcons::Italic))));
    bis.items().add(Some(italic));
    row.items().add(Some(bis));
    font.items().add(Some(row));
    home.groups().add(Some(font));
    model.tabs().add(Some(home));
    model
        .contextual_groups()
        .add(Some(RibbonContextualGroupModel::new(
            s("table-tools"),
            s("Table Tools"),
        )));
    let design = RibbonTabModel::new(s("table-design"), s("Table Design"));
    design.set_contextual_group_id(s("table-tools"));
    model.tabs().add(Some(design));
    assert!(
        try_cast_node::<dyn RibbonToggleButtonModelImpl>(
            model
                .find_item(s("bold"))
                .map(|v| v as Rc<dyn RibbonNodeModelImpl>)
        )
        .unwrap()
        .is_some()
    );
    assert_eq!(model.find_group(s("font")).unwrap().id(), s("font"));
    assert_eq!(
        model.find_node(s("table-tools")).unwrap().id(),
        s("table-tools")
    );
    assert_eq!(model.clone().enumerate_items().iter().count(), 6);
    assert!(
        !model
            .clone()
            .visible_tabs()
            .iter()
            .any(|v| v.unwrap().id() == s("table-design"))
    );
    model.set_active_contextual_groups(Some(ids(vec![s("table-tools")])));
    assert!(
        model
            .clone()
            .visible_tabs()
            .iter()
            .any(|v| v.unwrap().id() == s("table-design"))
    );
    assert!(model.find_tab(s("table-design")).unwrap().is_contextual());
    model.set_command_enabled(s("cut"), false);
    model.set_command_checked(s("bold"), true);
    assert!(!model.find_item(s("cut")).unwrap().is_enabled());
    let bold = try_cast_node::<dyn RibbonToggleButtonModelImpl>(
        model
            .find_item(s("bold"))
            .map(|v| v as Rc<dyn RibbonNodeModelImpl>),
    )
    .unwrap()
    .unwrap();
    assert!(bold.is_checked());
    model.set_selected_tab(model.find_tab(s("home")));
    assert_eq!(model.selected_tab_id(), s("home"));
    assert!(Rc::ptr_eq(
        &model.find_tab(s("home")).unwrap(),
        &model.selected_tab().unwrap()
    ));
}
