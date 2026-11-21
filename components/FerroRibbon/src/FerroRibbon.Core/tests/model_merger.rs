use ferroribbon_core::{ObservableObjectImpl, RibbonMergeAction, RibbonString, model::*};
use ferroui_system::{
    collections::{object_model::ObservableCollection, specialized::NotifyCollectionChangedAction},
    component_model::INotifyPropertyChanged,
    events::Event,
};
use serde_json::{Value, json};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../../tests/fixtures/merger.json")).unwrap()
}
fn s(value: &str) -> Option<RibbonString> {
    Some(value.into())
}
fn text(value: Option<RibbonString>) -> Option<String> {
    value.map(|v| v.to_utf8().unwrap())
}
fn item(id: Option<RibbonString>, label: &str, order: i32, action: i32) -> Rc<RibbonButtonModel> {
    let v = RibbonButtonModel::new(s("temporary"), s(label));
    v.set_id(id);
    v.set_order(order);
    v.set_merge_action(RibbonMergeAction::from_raw(action));
    v
}
fn snapshot(items: ObservableCollection<RibbonItemReference>) -> Value {
    json!(items.iter().map(|v|v.map(|v|json!({"Id":text(v.id()),"Label":text(v.label()),"Order":v.order(),"Action":v.merge_action().raw()}))).collect::<Vec<_>>())
}
fn ids(items: ObservableCollection<RibbonItemReference>) -> Value {
    json!(
        items
            .iter()
            .map(|v| v.and_then(|v| text(v.id())))
            .collect::<Vec<_>>()
    )
}
fn labels(items: &[RibbonItemReference]) -> String {
    items
        .iter()
        .map(|v| {
            v.as_ref()
                .and_then(|v| text(v.label()))
                .unwrap_or_else(|| "<null>".into())
        })
        .collect::<Vec<_>>()
        .join("|")
}
fn log(items: &ObservableCollection<RibbonItemReference>, trace: Rc<RefCell<Vec<String>>>) {
    let properties = trace.clone();
    items.property_changed().add(Rc::new(move |v| {
        properties.borrow_mut().push(format!("p:{v}"))
    }));
    items.add_collection_changed(Rc::new(move |e| {
        trace.borrow_mut().push(format!(
            "c:{:?}:{},{}:{}:{}",
            e.action,
            e.new_starting_index,
            e.old_starting_index,
            labels(e.new_items),
            labels(e.old_items)
        ))
    }));
}
#[test]
fn source_action_id_and_order_matrix_preserves_events_and_repeated_unmerge() {
    for row in corpus()["matrix"].as_array().unwrap() {
        let host = RibbonModel::new();
        let source = RibbonModel::new();
        for (id, order) in [("a", 10), ("b", 20), ("c", 30)] {
            host.tab_strip_items()
                .add(Some(item(s(id), &format!("host.{id}"), order, 0)));
        }
        source.tab_strip_items().add(Some(item(
            row["id"].as_str().and_then(s),
            "plugin",
            row["order"].as_i64().unwrap() as i32,
            row["action"].as_i64().unwrap() as i32,
        )));
        let trace = Rc::new(RefCell::new(vec![]));
        log(&host.tab_strip_items(), trace.clone());
        let merger = RibbonModelMerger::merge(Some(host.clone()), Some(source));
        assert_eq!(snapshot(host.tab_strip_items()), row["merged"], "{row}");
        merger.unmerge();
        assert_eq!(snapshot(host.tab_strip_items()), row["restored"], "{row}");
        merger.unmerge();
        assert_eq!(json!(*trace.borrow()), row["trace"], "{row}");
    }
}
#[test]
fn candidates_ignore_actions_and_empty_target_can_accept_null_candidate() {
    for row in corpus()["candidates"].as_array().unwrap() {
        let host = RibbonModel::new();
        let source = RibbonModel::new();
        if row["occupied"] == true {
            host.quick_access_candidates()
                .add(Some(item(s("c"), "host", 0, 0)));
        }
        let candidate = if row["missing"] == true {
            None
        } else {
            Some(item(s("c"), "plugin", -2, 2) as Rc<dyn RibbonItemModelImpl>)
        };
        source.quick_access_candidates().add(candidate);
        let trace = Rc::new(RefCell::new(vec![]));
        log(&host.quick_access_candidates(), trace.clone());
        let result = catch_unwind(AssertUnwindSafe(|| {
            RibbonModelMerger::merge(Some(host.clone()), Some(source))
        }));
        assert_eq!(result.is_err(), !row["error"].is_null());
        assert_eq!(snapshot(host.quick_access_candidates()), row["merged"]);
        if let Ok(merger) = result {
            merger.unmerge();
        }
        assert_eq!(snapshot(host.quick_access_candidates()), row["restored"]);
        assert_eq!(json!(*trace.borrow()), row["trace"]);
    }
}
#[test]
fn removed_nodes_follow_surviving_anchor_and_other_merges() {
    for row in corpus()["anchors"].as_array().unwrap() {
        let host = RibbonModel::new();
        let first = RibbonModel::new();
        let second = RibbonModel::new();
        for (id, order) in [("a", 10), ("b", 20), ("c", 30), ("d", 40)] {
            host.tab_strip_items().add(Some(item(s(id), id, order, 0)));
        }
        let a = host.tab_strip_items().get(0);
        first
            .tab_strip_items()
            .add(Some(item(s("b"), "remove", 0, 2)));
        let m1 = RibbonModelMerger::merge(Some(host.clone()), Some(first));
        second.tab_strip_items().add(Some(item(s("x"), "x", 15, 0)));
        let m2 = RibbonModelMerger::merge(Some(host.clone()), Some(second));
        if row["removeAnchor"] == true {
            host.tab_strip_items().remove(&a);
        } else {
            let count = host.tab_strip_items().count();
            host.tab_strip_items().move_item(0, count - 1);
        }
        assert_eq!(ids(host.tab_strip_items()), row["edited"]);
        m1.unmerge();
        assert_eq!(ids(host.tab_strip_items()), row["one"]);
        m2.unmerge();
        assert_eq!(ids(host.tab_strip_items()), row["two"]);
    }
}
#[test]
fn throwing_and_reentrant_unmerge_retain_original_log_semantics() {
    for row in corpus()["undo"].as_array().unwrap() {
        let host = RibbonModel::new();
        let source = RibbonModel::new();
        for id in ["a", "b", "c"] {
            host.tab_strip_items().add(Some(item(s(id), id, 0, 0)));
        }
        source
            .tab_strip_items()
            .add(Some(item(s("b"), "remove.b", 0, 2)));
        source
            .tab_strip_items()
            .add(Some(item(s("c"), "remove.c", 0, 2)));
        let merger = RibbonModelMerger::merge(Some(host.clone()), Some(source));
        let weak = Rc::downgrade(&merger);
        let reentrant = row["reentrant"] == true;
        let fired = Rc::new(Cell::new(false));
        let trace = Rc::new(RefCell::new(vec![]));
        let output = trace.clone();
        let token = host
            .tab_strip_items()
            .add_collection_changed(Rc::new(move |e| {
                output
                    .borrow_mut()
                    .push(format!("{:?}:{}", e.action, labels(e.new_items)));
                if e.action == NotifyCollectionChangedAction::Add && !fired.replace(true) {
                    if reentrant {
                        weak.upgrade().unwrap().unmerge();
                    } else {
                        panic!("notify");
                    }
                }
            }));
        let result = catch_unwind(AssertUnwindSafe(|| merger.unmerge()));
        assert_eq!(result.is_err(), !row["error"].is_null());
        assert_eq!(ids(host.tab_strip_items()), row["failed"]);
        merger.unmerge();
        assert_eq!(ids(host.tab_strip_items()), row["retried"]);
        merger.unmerge();
        assert_eq!(json!(*trace.borrow()), row["trace"]);
        host.tab_strip_items().remove_collection_changed(token);
    }
}
#[test]
fn failed_merge_keeps_completed_mutation_without_returning_handle() {
    for row in corpus()["throwing"].as_array().unwrap() {
        let host = RibbonModel::new();
        let source = RibbonModel::new();
        for id in ["x", "y"] {
            source.tab_strip_items().add(Some(item(s(id), id, 0, 0)));
        }
        if row["property"] == true {
            host.tab_strip_items()
                .property_changed()
                .add(Rc::new(|_| panic!("notify")));
        } else {
            host.tab_strip_items()
                .add_collection_changed(Rc::new(|_| panic!("notify")));
        }
        let result = catch_unwind(AssertUnwindSafe(|| {
            RibbonModelMerger::merge(Some(host.clone()), Some(source))
        }));
        assert!(result.is_err());
        assert!(!row["error"].is_null());
        assert_eq!(snapshot(host.tab_strip_items()), row["items"]);
    }
}
#[test]
fn replacement_undo_finds_current_instance_or_leaves_substitute() {
    for row in corpus()["replacement"].as_array().unwrap() {
        let host = RibbonModel::new();
        let source = RibbonModel::new();
        host.tab_strip_items().add(Some(item(s("a"), "a", 0, 0)));
        host.tab_strip_items()
            .add(Some(item(s("b"), "host.b", 0, 0)));
        let replacement: RibbonItemReference = Some(item(s("b"), "plugin.b", 0, 1));
        source.tab_strip_items().add(replacement.clone());
        let merger = RibbonModelMerger::merge(Some(host.clone()), Some(source));
        match row["edit"].as_str().unwrap() {
            "move" => host.tab_strip_items().move_item(1, 0),
            "remove" => {
                host.tab_strip_items().remove(&replacement);
            }
            "substitute" => host
                .tab_strip_items()
                .set(1, Some(item(s("b"), "other", 0, 0))),
            _ => {}
        }
        merger.unmerge();
        assert_eq!(snapshot(host.tab_strip_items()), row["items"]);
    }
}
struct EqualItem {
    base: RibbonButtonModel,
    trace: Rc<RefCell<Vec<String>>>,
    reflexive: bool,
}
impl INotifyPropertyChanged for EqualItem {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for EqualItem {}
impl RibbonNodeModelImpl for EqualItem {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.item_model().node_model()
    }
    fn equals_node(&self, value: &dyn RibbonNodeModelImpl) -> bool {
        self.trace.borrow_mut().push(format!(
            "eq:{}:{}",
            text(self.label()).unwrap(),
            text(value.label()).unwrap_or_default()
        ));
        self.reflexive && (value as &dyn Any).is::<Self>()
    }
    fn node_hash_code(&self) -> i32 {
        1
    }
}
impl RibbonItemModelImpl for EqualItem {
    fn item_model(&self) -> &RibbonItemModel {
        self.base.item_model()
    }
}
impl RibbonButtonModelImpl for EqualItem {
    fn button_model(&self) -> &RibbonButtonModel {
        &self.base
    }
}
#[test]
fn virtual_equality_can_select_a_different_node_or_fail_reflexivity() {
    register_ribbon_button_model::<EqualItem>().unwrap();
    for row in corpus()["equality"].as_array().unwrap() {
        let host = RibbonModel::new();
        let source = RibbonModel::new();
        let trace = Rc::new(RefCell::new(vec![]));
        for (id, label) in [("other", "first"), ("id", "matched")] {
            host.tab_strip_items().add(Some(Rc::new(EqualItem {
                base: RibbonButtonModel::construct(s(id), s(label)),
                trace: trace.clone(),
                reflexive: row["reflexive"] == true,
            })));
        }
        source.tab_strip_items().add(Some(item(
            s("id"),
            "plugin",
            0,
            row["action"].as_i64().unwrap() as i32,
        )));
        let result = catch_unwind(AssertUnwindSafe(|| {
            RibbonModelMerger::merge(Some(host.clone()), Some(source))
        }));
        assert_eq!(result.is_err(), !row["error"].is_null());
        assert_eq!(snapshot(host.tab_strip_items()), row["merged"]);
        if let Ok(merger) = result {
            merger.unmerge();
        }
        assert_eq!(snapshot(host.tab_strip_items()), row["restored"]);
        assert_eq!(json!(*trace.borrow()), row["trace"]);
    }
}
#[test]
fn source_snapshots_survive_source_mutation_during_notifications() {
    let host = RibbonModel::new();
    let source = RibbonModel::new();
    for id in ["a", "b"] {
        source.tab_strip_items().add(Some(item(s(id), id, 0, 0)));
    }
    let source_items = source.tab_strip_items();
    let fired = Cell::new(false);
    let token = host
        .tab_strip_items()
        .add_collection_changed(Rc::new(move |_| {
            if !fired.replace(true) {
                source_items.clear();
                source_items.add(Some(item(s("late"), "late", 0, 0)));
            }
        }));
    let merger = RibbonModelMerger::merge(Some(host.clone()), Some(source.clone()));
    let expected = &corpus()["snapshot"];
    assert_eq!(ids(host.tab_strip_items()), expected["merged"]);
    assert_eq!(ids(source.tab_strip_items()), expected["source"]);
    merger.unmerge();
    assert_eq!(ids(host.tab_strip_items()), expected["restored"]);
    host.tab_strip_items().remove_collection_changed(token);
}
#[test]
fn null_validation_order_and_implicit_public_constructor() {
    let mut trace = vec![];
    for (target, source) in [(true, true), (false, true), (true, false)] {
        let result = catch_unwind(AssertUnwindSafe(|| {
            RibbonModelMerger::merge(
                if target {
                    None
                } else {
                    Some(RibbonModel::new())
                },
                if source {
                    None
                } else {
                    Some(RibbonModel::new())
                },
            )
        }));
        let error = result.err().unwrap();
        let message = error
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| error.downcast_ref::<&str>().copied())
            .unwrap();
        trace.push(message.rsplit(": ").next().unwrap().to_owned());
    }
    assert_eq!(json!(trace), corpus()["nullArguments"]);
    RibbonModelMerger::new().unmerge();
}

fn node_snapshot(nodes: impl IntoIterator<Item = RibbonNodeReference>) -> Value {
    json!(nodes.into_iter().map(|n|n.map(|n|json!({"Id":text(n.id()),"Label":text(n.label()),"Order":n.order(),"Action":n.merge_action().raw()}))).collect::<Vec<_>>())
}
fn node_ids(nodes: impl IntoIterator<Item = RibbonNodeReference>) -> Value {
    json!(
        nodes
            .into_iter()
            .map(|n| n.and_then(|n| text(n.id())))
            .collect::<Vec<_>>()
    )
}
struct Graph {
    model: Rc<RibbonModel>,
    group: Rc<RibbonGroupModel>,
    split: Rc<RibbonSplitButtonModel>,
    gallery: Rc<RibbonGalleryModel>,
    context: Rc<RibbonContextualGroupModel>,
}
fn build(plugin: bool) -> Graph {
    let prefix = if plugin { "plugin." } else { "host." };
    let m = RibbonModel::new();
    m.set_title(s(&format!("{prefix}title")));
    let tab = RibbonTabModel::new(s("home"), s(&format!("{prefix}home")));
    let g = RibbonGroupModel::new(s("font"), s(&format!("{prefix}font")));
    g.set_row_count(if plugin { 1 } else { 3 });
    let container = RibbonButtonGroupModel::new(s("format"));
    container.items().add(Some(RibbonToggleButtonModel::new(
        s(if plugin { "strike" } else { "bold" }),
        None,
    )));
    g.items().add(Some(container));
    let split = RibbonSplitButtonModel::new(s("split"), None);
    let menu = RibbonMenuItemModel::new(s("menu"), None);
    menu.items().add(Some(RibbonMenuItemModel::new(
        s(if plugin { "new.sub" } else { "old.sub" }),
        None,
    )));
    split.menu_items().add(Some(menu));
    if plugin {
        split
            .menu_items()
            .add(Some(RibbonMenuItemModel::new(s("new.menu"), None)));
    }
    g.items().add(Some(split.clone()));
    let gallery = RibbonGalleryModel::new(s("gallery"), None);
    gallery.items().add(Some(RibbonGalleryItemModel::new(
        s("choice"),
        s(&format!("{prefix}choice")),
    )));
    gallery
        .menu_items()
        .add(Some(RibbonMenuItemModel::new(s("gallery.menu"), None)));
    if plugin {
        gallery
            .items()
            .add(Some(RibbonGalleryItemModel::new(s("new.choice"), None)));
        gallery
            .menu_items()
            .add(Some(RibbonMenuItemModel::new(s("new.gallery.menu"), None)));
    }
    g.items().add(Some(gallery.clone()));
    let color = RibbonColorPickerModel::new(s("color"), None);
    color.set_show_automatic(!plugin);
    g.items().add(Some(color));
    g.items().add(Some(item(
        s("cut"),
        &format!("{prefix}cut"),
        0,
        if plugin { 2 } else { 0 },
    )));
    if plugin {
        g.items()
            .add(Some(item(s("new"), &format!("{prefix}new"), 0, 0)));
    }
    g.slide_out_items().add(Some(item(
        s(if plugin { "new.slide" } else { "old.slide" }),
        "slide",
        0,
        0,
    )));
    tab.groups().add(Some(g.clone()));
    if plugin {
        tab.groups()
            .add(Some(RibbonGroupModel::new(s("extra"), None)));
    }
    m.tabs().add(Some(tab));
    if plugin {
        m.tabs().add(Some(RibbonTabModel::new(s("addins"), None)));
    }
    let context = RibbonContextualGroupModel::new(s("ctx"), None);
    context.set_is_visible(plugin);
    m.contextual_groups().add(Some(context.clone()));
    if plugin {
        m.contextual_groups()
            .add(Some(RibbonContextualGroupModel::new(s("new.ctx"), None)));
    }
    m.quick_access_items()
        .add(Some(item(s("undo"), "undo", 0, if plugin { 2 } else { 0 })));
    m.tab_strip_items().add(Some(item(
        s(if plugin { "new.strip" } else { "strip" }),
        "strip",
        0,
        0,
    )));
    let back = RibbonBackstageItemModel::new(s("back"), s(&format!("{prefix}back")));
    back.set_merge_action(if plugin {
        RibbonMergeAction::Replace
    } else {
        RibbonMergeAction::Merge
    });
    m.backstage().items().add(Some(back));
    m.quick_access_candidates().add(Some(item(
        s("candidate"),
        "candidate",
        0,
        if plugin { 2 } else { 0 },
    )));
    if plugin {
        m.quick_access_candidates()
            .add(Some(item(s("new.candidate"), "new.candidate", 0, 2)));
    }
    Graph {
        model: m,
        group: g,
        split,
        gallery,
        context,
    }
}
fn topology(h: &Graph) -> Value {
    let color = try_cast_node::<dyn RibbonColorPickerModelImpl>(
        h.model
            .find_item(s("color"))
            .map(|v| v as Rc<dyn RibbonNodeModelImpl>),
    )
    .unwrap()
    .unwrap();
    let menus=h.split.menu_items().iter().map(|n|{let n=n.unwrap();let menu=try_cast_node::<dyn RibbonMenuItemModelImpl>(Some(n.clone())).unwrap();json!({"Id":text(n.id()),"children":menu.map_or_else(||json!([]),|m|node_ids(m.items().iter()))})}).collect::<Vec<_>>();
    json!({"Title":text(h.model.title()),"tabs":node_snapshot(h.model.tabs().iter().map(|v|v.map(|v|v as Rc<dyn RibbonNodeModelImpl>))),"groups":node_ids(h.model.tabs().get(0).unwrap().groups().iter().map(|v|v.map(|v|v as Rc<dyn RibbonNodeModelImpl>))),"items":node_ids(h.model.clone().enumerate_items().iter().map(|v|v.map(|v|v as Rc<dyn RibbonNodeModelImpl>))),"slide":ids(h.group.slide_out_items()),"menus":menus,"choices":node_snapshot(h.gallery.items().iter().map(|v|v.map(|v|v as Rc<dyn RibbonNodeModelImpl>))),"galleryMenu":node_ids(h.gallery.menu_items().iter()),"context":node_ids(h.model.contextual_groups().iter().map(|v|v.map(|v|v as Rc<dyn RibbonNodeModelImpl>))),"contextVisible":h.context.is_visible(),"RowCount":h.group.row_count(),"color":color.show_automatic(),"qat":ids(h.model.quick_access_items()),"strip":ids(h.model.tab_strip_items()),"backstage":node_snapshot(h.model.backstage().items().iter().map(|v|v.map(|v|v as Rc<dyn RibbonNodeModelImpl>))),"candidates":snapshot(h.model.quick_access_candidates())})
}
#[test]
fn recursive_topology_matches_actual_merge_boundaries_and_undo() {
    let host = build(false);
    let plugin = build(true);
    let expected = &corpus()["topology"];
    assert_eq!(topology(&host), expected["original"]);
    let merger = RibbonModelMerger::merge(Some(host.model.clone()), Some(plugin.model));
    assert_eq!(topology(&host), expected["merged"]);
    merger.unmerge();
    assert_eq!(topology(&host), expected["restored"]);
}
#[test]
fn catalog_sharing_copying_callback_replacement_and_failures_match_source() {
    use ferroribbon_core::commands::{RibbonCommandCatalog, RibbonCommandDescriptor};
    fn add(c: &Rc<RibbonCommandCatalog>, id: &str, label: &str) {
        c.register(Some(RibbonCommandDescriptor::new(s(id), s(label), None)));
    }
    fn commands(c: Option<Rc<RibbonCommandCatalog>>) -> Value {
        c.map_or(Value::Null, |c| {
            json!(
                c.commands()
                    .iter()
                    .map(|d| json!({"Id":text(Some(d.id())),"Label":text(d.label())}))
                    .collect::<Vec<_>>()
            )
        })
    }
    for row in corpus()["catalog"].as_array().unwrap() {
        let mode = row["mode"].as_str().unwrap();
        let host = RibbonModel::new();
        let source = RibbonModel::new();
        let src = RibbonCommandCatalog::new();
        add(&src, "run", "source.duplicate");
        add(&src, "plugin", "source.plugin");
        if mode == "copy-source-mutate" {
            add(&src, "later", "source.later");
        }
        source.set_command_catalog(if mode == "none" {
            None
        } else {
            Some(src.clone())
        });
        if !mode.starts_with("share") {
            let cat = RibbonCommandCatalog::new();
            add(&cat, "RUN", "host");
            host.set_command_catalog(Some(cat));
        }
        if mode == "same" {
            host.set_command_catalog(Some(src.clone()));
        }
        source
            .tab_strip_items()
            .add(Some(item(s("command-item"), "command-item", 0, 0)));
        let events = Rc::new(RefCell::new(vec![]));
        let unmerging = Rc::new(Cell::new(false));
        let subscription = host.command_catalog().map(|cat| {
            let log = events.clone();
            let source = Rc::downgrade(&src);
            let mode = mode.to_owned();
            let fired = Cell::new(false);
            let armed = unmerging.clone();
            let token = cat.commands_changed().add(Rc::new(move |(owner, id)| {
                log.borrow_mut()
                    .push(format!("host:{}", text(Some(id.clone())).unwrap()));
                if id == &RibbonString::from("plugin") && !fired.get() {
                    if mode == "copy-replace" {
                        fired.set(true);
                        add(owner, "plugin", "other");
                    } else if mode == "copy-source-mutate" {
                        fired.set(true);
                        let source = source.upgrade().unwrap();
                        source.unregister(Some(&"later".into()));
                        add(&source, "late", "source.late");
                    } else if mode == "copy-register-throw"
                        || (mode == "copy-unregister-throw" && armed.get())
                    {
                        fired.set(true);
                        panic!("notify");
                    }
                }
            }));
            (cat, token)
        });
        let property = if mode == "share-throw" {
            Some(host.property_changed().add(Rc::new(|v| {
                if v == "CommandCatalog" {
                    panic!("notify");
                }
            })))
        } else {
            None
        };
        let result = catch_unwind(AssertUnwindSafe(|| {
            RibbonModelMerger::merge(Some(host.clone()), Some(source.clone()))
        }));
        assert_eq!(result.is_err(), !row["mergeError"].is_null(), "{mode}");
        let merger = result.ok();
        if mode == "share-replace" {
            let other = RibbonCommandCatalog::new();
            add(&other, "other", "other");
            host.set_command_catalog(Some(other));
        }
        let state = || {
            let cat = host.command_catalog();
            let plugin = cat.as_ref().and_then(|c| c.find(Some(&"plugin".into())));
            json!({"catalog":commands(cat.clone()),"same":cat.as_ref().is_some_and(|c|source.command_catalog().is_some_and(|s|Rc::ptr_eq(c,&s))),"pluginSame":plugin.is_some_and(|d|src.find(Some(&"plugin".into())).is_some_and(|s|Rc::ptr_eq(&d,&s))),"strip":ids(host.tab_strip_items())})
        };
        assert_eq!(state(), row["merged"], "{mode}");
        unmerging.set(true);
        let result = catch_unwind(AssertUnwindSafe(|| {
            if let Some(m) = &merger {
                m.unmerge();
            }
        }));
        assert_eq!(result.is_err(), !row["undoError"].is_null(), "{mode}");
        assert_eq!(state(), row["first"], "{mode}");
        if let Some(m) = merger {
            m.unmerge();
        }
        assert_eq!(state(), row["restored"], "{mode}");
        assert_eq!(json!(*events.borrow()), row["events"], "{mode}");
        assert_eq!(commands(Some(src)), row["source"]);
        if let Some((cat, token)) = subscription {
            cat.commands_changed().remove(token);
        }
        if let Some(token) = property {
            host.property_changed().remove(token);
        }
    }
}

#[test]
fn empty_ids_do_not_match_merge_lists_but_do_match_candidates_and_self_merge_undo_uses_equality() {
    for row in corpus()["emptyIds"].as_array().unwrap() {
        let host = RibbonModel::new();
        let source = RibbonModel::new();
        let id = row["id"].as_str().and_then(s);
        let old: RibbonItemReference = Some(item(id.clone(), "host", 0, 0));
        let added: RibbonItemReference = Some(item(
            id,
            "plugin",
            0,
            row["action"].as_i64().unwrap() as i32,
        ));
        host.tab_strip_items().add(old.clone());
        host.quick_access_candidates().add(old);
        source.tab_strip_items().add(added.clone());
        source.quick_access_candidates().add(added);
        let merger = RibbonModelMerger::merge(Some(host.clone()), Some(source));
        assert_eq!(snapshot(host.tab_strip_items()), row["merged"]);
        assert_eq!(snapshot(host.quick_access_candidates()), row["candidates"]);
        merger.unmerge();
        assert_eq!(snapshot(host.tab_strip_items()), row["restored"]);
    }
    let model = RibbonModel::new();
    model.tab_strip_items().add(Some(item(s("a"), "a", 0, 3)));
    model.tab_strip_items().add(Some(item(s("b"), "b", 0, 0)));
    let merger = RibbonModelMerger::merge(Some(model.clone()), Some(model.clone()));
    assert_eq!(ids(model.tab_strip_items()), corpus()["self"]["merged"]);
    merger.unmerge();
    assert_eq!(ids(model.tab_strip_items()), corpus()["self"]["restored"]);
}
#[test]
fn dropping_merger_never_rolls_back_and_undo_retains_replaced_objects_without_root_cycle() {
    let host = RibbonModel::new();
    let source = RibbonModel::new();
    let original = item(s("id"), "original", 0, 0);
    let original_weak = Rc::downgrade(&original);
    host.tab_strip_items().add(Some(original));
    let replacement = item(s("id"), "replacement", 0, 1);
    let replacement_weak = Rc::downgrade(&replacement);
    source.tab_strip_items().add(Some(replacement));
    let root = Rc::downgrade(&host);
    let merger = RibbonModelMerger::merge(Some(host.clone()), Some(source.clone()));
    assert!(original_weak.upgrade().is_some());
    merger.unmerge();
    assert!(original_weak.upgrade().is_some());
    assert!(replacement_weak.upgrade().is_some());
    drop(source);
    assert!(replacement_weak.upgrade().is_none());
    drop(host);
    assert!(root.upgrade().is_none());
    assert!(original_weak.upgrade().is_none());
    drop(merger);
    let host = RibbonModel::new();
    let source = RibbonModel::new();
    source
        .tab_strip_items()
        .add(Some(item(s("added"), "added", 0, 0)));
    drop(RibbonModelMerger::merge(Some(host.clone()), Some(source)));
    assert_eq!(ids(host.tab_strip_items()), json!(["added"]));
    // The no-catalog merge log retains collections, rather than the root itself.
    let source = RibbonModel::new();
    source
        .tab_strip_items()
        .add(Some(item(s("more"), "more", 0, 0)));
    let weak = Rc::downgrade(&host);
    let merger = RibbonModelMerger::merge(Some(host.clone()), Some(source));
    drop(host);
    assert!(weak.upgrade().is_none());
    merger.unmerge();
}
#[test]
fn upstream_merge_add_remove_unmerge_order_and_nested_catalog_regressions() {
    use ferroribbon_core::commands::{RibbonCommandCatalog, RibbonCommandDescriptor};
    let host = RibbonModel::new();
    let home = RibbonTabModel::new(s("home"), s("Home"));
    let clipboard = RibbonGroupModel::new(s("clipboard"), s("Clipboard"));
    clipboard.items().add(Some(item(s("cut"), "Cut", 0, 0)));
    home.groups().add(Some(clipboard.clone()));
    host.tabs().add(Some(home));
    let plugin = RibbonModel::new();
    let home = RibbonTabModel::new(s("home"), s("Home"));
    let group = RibbonGroupModel::new(s("clipboard"), s("Clipboard"));
    group.items().add(Some(item(s("copy"), "Copy", 0, 0)));
    group.items().add(Some(item(s("cut"), "cut", 0, 2)));
    home.groups().add(Some(group));
    let addin = RibbonGroupModel::new(s("addin"), s("Add-in"));
    addin.items().add(Some(item(s("addin-run"), "Run", 0, 0)));
    home.groups().add(Some(addin));
    plugin.tabs().add(Some(home));
    plugin
        .tabs()
        .add(Some(RibbonTabModel::new(s("addins"), s("Add-ins"))));
    let merger = RibbonModelMerger::merge(Some(host.clone()), Some(plugin));
    assert!(
        clipboard
            .items()
            .iter()
            .any(|i| i.unwrap().id() == s("copy"))
    );
    assert!(
        !clipboard
            .items()
            .iter()
            .any(|i| i.unwrap().id() == s("cut"))
    );
    assert!(host.find_group(s("addin")).is_some());
    assert!(host.find_tab(s("addins")).is_some());
    merger.unmerge();
    assert!(
        !clipboard
            .items()
            .iter()
            .any(|i| i.unwrap().id() == s("copy"))
    );
    assert!(
        clipboard
            .items()
            .iter()
            .any(|i| i.unwrap().id() == s("cut"))
    );
    assert!(host.find_group(s("addin")).is_none());
    assert!(host.find_tab(s("addins")).is_none());
    let target = RibbonModel::new();
    for (id, order) in [("a", 10), ("c", 30)] {
        let tab = RibbonTabModel::new(s(id), None);
        tab.set_order(order);
        target.tabs().add(Some(tab));
    }
    let source = RibbonModel::new();
    let b = RibbonTabModel::new(s("b"), None);
    b.set_order(20);
    source.tabs().add(Some(b));
    let _merge = RibbonModelMerger::merge(Some(target.clone()), Some(source));
    assert_eq!(
        node_ids(
            target
                .tabs()
                .iter()
                .map(|t| t.map(|t| t as Rc<dyn RibbonNodeModelImpl>))
        ),
        json!(["a", "b", "c"])
    );
    let host = RibbonModel::new();
    host.set_command_catalog(Some(RibbonCommandCatalog::new()));
    let tab = RibbonTabModel::new(s("home"), None);
    let group = RibbonGroupModel::new(s("font"), None);
    let row = RibbonButtonGroupModel::new(s("format"));
    row.items()
        .add(Some(RibbonToggleButtonModel::new(s("bold"), None)));
    group.items().add(Some(row.clone()));
    tab.groups().add(Some(group));
    host.tabs().add(Some(tab));
    host.quick_access_items()
        .add(Some(item(s("undo"), "Undo", 0, 0)));
    let plugin = RibbonModel::new();
    let cat = RibbonCommandCatalog::new();
    cat.register(Some(RibbonCommandDescriptor::new(
        s("plugin.run"),
        s("Run"),
        None,
    )));
    plugin.set_command_catalog(Some(cat));
    let tab = RibbonTabModel::new(s("home"), None);
    let group = RibbonGroupModel::new(s("font"), None);
    let format = RibbonButtonGroupModel::new(s("format"));
    format
        .items()
        .add(Some(RibbonToggleButtonModel::new(s("strike"), None)));
    group.items().add(Some(format));
    tab.groups().add(Some(group));
    plugin.tabs().add(Some(tab));
    plugin
        .quick_access_items()
        .add(Some(item(s("undo"), "undo", 0, 2)));
    let merger = RibbonModelMerger::merge(Some(host.clone()), Some(plugin));
    assert!(
        host.command_catalog()
            .unwrap()
            .find(Some(&"plugin.run".into()))
            .is_some()
    );
    assert_eq!(ids(row.items()), json!(["bold", "strike"]));
    assert!(host.quick_access_items().is_empty());
    merger.unmerge();
    assert!(
        host.command_catalog()
            .unwrap()
            .find(Some(&"plugin.run".into()))
            .is_none()
    );
    assert_eq!(ids(row.items()), json!(["bold"]));
    assert_eq!(ids(host.quick_access_items()), json!(["undo"]));
}

#[test]
fn clearing_successful_undo_releases_captures_before_reentrant_native_drop() {
    struct DropButton {
        base: RibbonButtonModel,
        callback: RefCell<Option<Box<dyn Fn()>>>,
    }
    impl INotifyPropertyChanged for DropButton {
        fn property_changed(&self) -> &Event<str> {
            self.base.property_changed()
        }
    }
    impl ObservableObjectImpl for DropButton {}
    impl RibbonNodeModelImpl for DropButton {
        fn node_model(&self) -> &RibbonNodeModel {
            self.base.item_model().node_model()
        }
    }
    impl RibbonItemModelImpl for DropButton {
        fn item_model(&self) -> &RibbonItemModel {
            self.base.item_model()
        }
    }
    impl RibbonButtonModelImpl for DropButton {
        fn button_model(&self) -> &RibbonButtonModel {
            &self.base
        }
    }
    impl Drop for DropButton {
        fn drop(&mut self) {
            if let Some(callback) = self.callback.get_mut().take() {
                callback();
            }
        }
    }
    register_ribbon_button_model::<DropButton>().unwrap();
    let host = RibbonModel::new();
    let source = RibbonModel::new();
    host.tab_strip_items()
        .add(Some(item(s("id"), "original", 0, 0)));
    let replacement = Rc::new(DropButton {
        base: RibbonButtonModel::construct(s("id"), s("replacement")),
        callback: RefCell::new(None),
    });
    replacement.set_merge_action(RibbonMergeAction::Replace);
    source.tab_strip_items().add(Some(replacement.clone()));
    let merger = RibbonModelMerger::merge(Some(host.clone()), Some(source));
    let called = Rc::new(Cell::new(false));
    let output = called.clone();
    let weak = Rc::downgrade(&merger);
    *replacement.callback.borrow_mut() = Some(Box::new(move || {
        weak.upgrade().unwrap().unmerge();
        output.set(true);
    }));
    drop(replacement);
    assert!(!called.get());
    merger.unmerge();
    assert!(called.get());
    assert_eq!(
        text(host.tab_strip_items().get(0).unwrap().label()),
        Some("original".into())
    );
}

// Keeps the original source expectation.
#[path = "../../../tests/fixtures/framework/merger_equality_reentry.rs"]
mod collection_reentry;
