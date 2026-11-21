use crate::controls::{
    IRibbonItem, IRibbonItemInternal, RibbonItemHelper, RibbonItemLayout, RibbonKeyTipResult,
    RibbonOverflowMenuItems,
};
use crate::tests::run;
use ferroribbon_core::{RibbonItemSize, RibbonSimplifiedVisibility, layout::RibbonSizeDefinition};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    AttachedProperty, BoxedValue, FerroObject, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, VisualImpl, ferro_class, ferro_class_info, ferro_impl_classes,
    ferro_properties, instantiate,
};
use ferroui_base::{
    input::InputElementImpl, interactivity::InteractiveImpl, layout::LayoutableImpl,
};
use ferroui_controls::{Border, Control, ControlImpl, Panel, ScrollViewer};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[repr(C)]
struct Item {
    base: Control,
    unlinked: RefCell<Option<Rc<dyn Fn()>>>,
    owned_values: crate::controls::OwnedItemValues,
}
ferro_class!(Item:Control);
ferro_impl_classes!(Item:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
ferro_class_info!(Item {interfaces:[Rc<dyn IRibbonItemInternal> => |value|Rc::new(ItemView(value)) as Rc<dyn IRibbonItemInternal>]});
ferro_properties! {impl Item {
fn text_property()->AttachedProperty<Option<ferroribbon_core::RibbonString>> {FerroProperty::register_attached::<Item,FerroObject,_>("Text",None)}
fn object_property()->AttachedProperty<Option<BoxedValue>> {FerroProperty::register_attached::<Item,FerroObject,_>("Object",None)}
fn value_property()->AttachedProperty<i32> {FerroProperty::register_attached::<Item,FerroObject,_>("Value",0)}}}
impl Item {
    fn construct() -> Self {
        Self {
            base: Control::construct(),
            unlinked: RefCell::new(None),
            owned_values: Default::default(),
        }
    }
    fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
struct ItemView(Ref<Item>);
impl IRibbonItem for ItemView {
    fn as_object(&self) -> Ref<FerroObject> {
        self.0.clone().upcast()
    }
    fn id(&self) -> Option<ferroribbon_core::RibbonString> {
        None
    }
    fn label(&self) -> Option<ferroribbon_core::RibbonString> {
        None
    }
    fn icon(&self) -> Option<BoxedValue> {
        None
    }
    fn key_tip(&self) -> Option<ferroribbon_core::RibbonString> {
        None
    }
    fn command_id(&self) -> Option<ferroribbon_core::RibbonString> {
        None
    }
    fn size(&self) -> RibbonItemSize {
        RibbonItemSize::Medium
    }
    fn effective_size_definition(&self) -> RibbonSizeDefinition {
        RibbonSizeDefinition::medium_small()
    }
    fn current_size(&self) -> RibbonItemSize {
        self.size()
    }
    fn simplified_visibility(&self) -> RibbonSimplifiedVisibility {
        RibbonSimplifiedVisibility::Auto
    }
    fn show_label_in_simplified_mode(&self) -> bool {
        false
    }
    fn can_add_to_quick_access(&self) -> bool {
        true
    }
    fn apply_layout(&self, _layout: RibbonItemLayout) {
        panic!("test fixture has no presentation")
    }
    fn create_linked_copy(&self) -> Option<Ref<Control>> {
        panic!("test fixture does not create copies")
    }
    fn create_overflow_menu_items(&self) -> RibbonOverflowMenuItems {
        panic!("test fixture has no menu")
    }
    fn on_key_tip(&self) -> Rc<RibbonKeyTipResult> {
        panic!("test fixture has no KeyTip action")
    }
    fn invoke(&self) -> bool {
        panic!("test fixture has no command")
    }
}
impl IRibbonItemInternal for ItemView {
    fn owned_values(&self) -> &crate::controls::OwnedItemValues {
        &self.0.owned_values
    }
    fn shortcut(&self) -> Option<ferroribbon_core::RibbonString> {
        None
    }
    fn screen_tip(&self) -> Option<BoxedValue> {
        None
    }
    fn linked_properties(&self) -> Vec<&'static FerroProperty> {
        vec![Item::value_property().as_property()]
    }
    fn on_unlinked(&self) {
        let callback = self.0.unlinked.borrow().clone();
        if let Some(callback) = callback {
            callback()
        }
    }
}
#[repr(C)]
struct Derived {
    base: Item,
}
ferro_class!(Derived:Item);
ferro_impl_classes!(Derived:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl Derived {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: Item::construct(),
        })
    }
}
fn get(item: &FerroObject) -> i32 {
    item.get_value(Item::value_property())
}
fn set(item: &FerroObject, value: i32) {
    item.set_value(Item::value_property(), value)
}
fn link(s: &FerroObject, t: &FerroObject, two_way: bool) {
    RibbonItemHelper::link(s, t, Item::value_property().as_property(), two_way)
}
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../tests/fixtures/item-links.json")).unwrap()
}
fn log(item: &FerroObject, name: &'static str, events: &Rc<RefCell<Vec<String>>>) {
    let events = events.clone();
    item.add_property_changed_handler(Rc::new(move |e| {
        if e.property() == Item::value_property().as_property() {
            events
                .borrow_mut()
                .push(format!("{name}:{}", get(e.sender())))
        }
    }));
}
#[test]
fn exact_initial_copy_one_two_way_order_and_explicit_unlink() {
    run(|| {
        for case in corpus()["cases"].as_array().unwrap() {
            let s = Control::new();
            let t = Control::new();
            set(&s, 1);
            let events = Rc::new(RefCell::new(Vec::new()));
            log(&s, "s", &events);
            log(&t, "t", &events);
            link(&s, &t, case["twoWay"].as_bool().unwrap());
            let mut rows = vec![[get(&s), get(&t)]];
            set(&s, 3);
            rows.push([get(&s), get(&t)]);
            set(&t, 5);
            rows.push([get(&s), get(&t)]);
            RibbonItemHelper::unlink(&t);
            set(&s, 7);
            set(&t, 9);
            rows.push([get(&s), get(&t)]);
            assert_eq!(json!(rows), case["rows"]);
            assert_eq!(json!(*events.borrow()), case["events"]);
            assert_eq!(
                json!([
                    s.property_changed_subscriber_count(),
                    t.property_changed_subscriber_count()
                ]),
                case["callbacks"]
            );
        }
    });
}
#[test]
fn initial_copy_reentry_guard_and_failure_finally_match_original() {
    run(|| {
        let s = Control::new();
        let t = Control::new();
        set(&s, 1);
        let events = Rc::new(RefCell::new(Vec::new()));
        let copy = s.clone();
        let trace = events.clone();
        t.add_property_changed_handler(Rc::new(move |e| {
            if e.property() == Item::value_property().as_property() {
                let value = get(e.sender());
                trace.borrow_mut().push(format!("t:{value}"));
                if value == 1 {
                    set(&copy, 2)
                }
                if value == 3 {
                    set(&copy, 4)
                }
            }
        }));
        log(&s, "s", &events);
        link(&s, &t, true);
        assert_eq!(json!([get(&s), get(&t)]), corpus()["initial"]);
        set(&s, 3);
        assert_eq!(json!([get(&s), get(&t)]), corpus()["reentrant"]);
        set(&t, 8);
        assert_eq!(json!([get(&s), get(&t)]), corpus()["after"]);
        assert_eq!(json!(*events.borrow()), corpus()["trace"]);
        let a = Control::new();
        let b = Control::new();
        link(&a, &b, true);
        b.add_property_changed_handler(Rc::new(|e| {
            if e.property() == Item::value_property().as_property() && get(e.sender()) == 7 {
                panic!("target failure")
            }
        }));
        let error =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| set(&a, 7))).unwrap_err();
        assert_eq!(
            json!(error.downcast_ref::<&str>().unwrap()),
            corpus()["error"]
        );
        set(&a, 8);
        assert_eq!(json!([get(&a), get(&b)]), corpus()["reset"]);
    });
}
#[test]
fn common_links_exact_type_data_context_and_repeated_unlink_hook_order() {
    run(|| {
        let source = Item::new();
        let target = Item::new();
        set(&source, 10);
        let data: BoxedValue = Rc::new(42i32);
        source.set_data_context(Some(data.clone()));
        RibbonItemHelper::link_common(&source, &target);
        source.set_is_enabled(false);
        source.set_is_visible(false);
        set(&source, 11);
        source.set_data_context(Some(Rc::new(24i32)));
        assert_eq!(
            json!({"value":get(&target),"enabled":target.is_enabled(),"visible":target.is_visible(),"data":Rc::ptr_eq(&target.data_context().unwrap(),&data),"source":RibbonItemHelper::get_source_item(&target).unwrap().ptr_eq(&source)}),
            corpus()["common"]
        );
        let unlinks = Rc::new(Cell::new(0));
        let count = unlinks.clone();
        let weak = target.downgrade();
        *target.unlinked.borrow_mut() = Some(Rc::new(move || {
            count.set(count.get() + 1);
            set(&weak.upgrade().unwrap(), 99)
        }));
        RibbonItemHelper::unlink(&target);
        RibbonItemHelper::unlink(&target);
        set(&source, 12);
        assert_eq!(
            json!({"unlinks":unlinks.get(),"source":get(&source),"target":get(&target),"stillSource":RibbonItemHelper::get_source_item(&target).unwrap().ptr_eq(&source)}),
            corpus()["unlinked"]
        );
        let derived = Derived::new();
        RibbonItemHelper::link_common(&source, &derived);
        set(&source, 13);
        assert_eq!(json!(get(&derived)), corpus()["otherType"]);
    });
}
#[test]
fn weak_endpoints_deferred_cleanup_duplicate_and_retained_source_identity() {
    run(|| {
        for (reverse, name) in [(false, "deadTarget"), (true, "deadSource")] {
            Dispatcher::ui_thread().run_jobs(None);
            let source = Control::new();
            let target = Control::new();
            link(&source, &target, true);
            let (owner, weak) = if reverse {
                let weak = source.downgrade();
                drop(source);
                (target, weak)
            } else {
                let weak = target.downgrade();
                drop(target);
                (source, weak)
            };
            assert_eq!(json!(weak.upgrade().is_some()), corpus()[name]["alive"]);
            assert_eq!(
                json!(owner.property_changed_subscriber_count()),
                corpus()[name]["before"]
            );
            set(&owner, 1);
            assert_eq!(
                json!(owner.property_changed_subscriber_count()),
                corpus()[name]["queued"]
            );
            let owned = owner.downgrade();
            drop(owner);
            assert!(
                owned.upgrade().is_some(),
                "deferred callback temporarily roots owner"
            );
            Dispatcher::ui_thread().run_jobs(None);
            assert!(owned.upgrade().is_none());
        }
        let a = Control::new();
        let b = Control::new();
        link(&a, &b, true);
        link(&a, &b, true);
        assert_eq!(
            json!([
                a.property_changed_subscriber_count(),
                b.property_changed_subscriber_count()
            ]),
            corpus()["duplicateCounts"]
        );
        RibbonItemHelper::unlink(&b);
        assert_eq!(
            json!([
                a.property_changed_subscriber_count(),
                b.property_changed_subscriber_count()
            ]),
            corpus()["removedCounts"]
        );
        let source = Control::new();
        let copy = Control::new();
        let weak = source.downgrade();
        RibbonItemHelper::set_source_item(&copy, &source);
        drop(source);
        let mut retained = vec![weak.upgrade().is_some()];
        RibbonItemHelper::unlink(&copy);
        retained.push(weak.upgrade().is_some());
        RibbonItemHelper::set_source_item(&copy, None);
        retained.push(weak.upgrade().is_some());
        assert_eq!(json!(retained), corpus()["retainedSource"]);
    });
}
#[test]
fn supported_tree_branches_are_child_first_and_do_not_walk_arbitrary_controls() {
    run(|| {
        let source = Control::new();
        let order = Rc::new(RefCell::new(Vec::new()));
        let copy = |name| {
            let item = Item::new();
            RibbonItemHelper::set_source_item(&item, &source);
            let order = order.clone();
            *item.unlinked.borrow_mut() = Some(Rc::new(move || order.borrow_mut().push(name)));
            item
        };
        let panel = Panel::new();
        panel.children().add(copy("first"));
        let border = Border::new();
        border.set_child(copy("border"));
        panel.children().add(border);
        let viewer = ScrollViewer::new();
        viewer.set_content(Some(Control::boxed(copy("scroll"))));
        panel.children().add(viewer);
        RibbonItemHelper::unlink_tree(Some(panel.upcast()));
        RibbonItemHelper::unlink_tree(None);
        // Fourth original branch is RibbonItemsContainer, still missing as an actual native class.
        assert_eq!(
            json!(*order.borrow()),
            json!(corpus()["order"].as_array().unwrap()[..3])
        );
    });
}
#[test]
fn callbacks_already_in_a_dispatch_snapshot_still_run_after_unlink() {
    run(|| {
        let s = Control::new();
        let t = Control::new();
        let weak = t.downgrade();
        s.add_property_changed_handler(Rc::new(move |e| {
            if e.property() == Item::value_property().as_property() {
                RibbonItemHelper::unlink(&weak.upgrade().unwrap())
            }
        }));
        link(&s, &t, false);
        set(&s, 1);
        assert_eq!(get(&t), 1);
        set(&s, 2);
        assert_eq!(get(&t), 1);
    });
}

#[test]
fn layout_default_metrics_and_keytip_result_identity_are_not_conflated() {
    run(|| {
        use crate::primitives::RibbonMetrics;
        use ferroribbon_core::{RibbonGroupState, model::RibbonItemsSource};
        use std::sync::Arc;
        let default = RibbonItemLayout::default();
        assert_eq!(default.size, RibbonItemSize::Large);
        assert!(default.metrics.is_none());
        assert_eq!(default.group_state, RibbonGroupState::Large);
        let metrics = RibbonMetrics::comfortable();
        let layout = RibbonItemLayout::new(RibbonItemSize::Medium, Some(metrics.clone()));
        assert!(Arc::ptr_eq(layout.metrics.as_ref().unwrap(), &metrics));
        assert!(!layout.is_simplified);
        assert_eq!(layout.show_label, None);
        assert!(Rc::ptr_eq(
            &RibbonKeyTipResult::close(),
            &RibbonKeyTipResult::close()
        ));
        let a = RibbonKeyTipResult::new();
        let b = RibbonKeyTipResult::scope(None);
        assert!(!Rc::ptr_eq(&a, &b));
        assert!(!Rc::ptr_eq(&a, &RibbonKeyTipResult::close()));
        assert!(a.next_scope().is_none());
        let element = Control::new();
        let weak = element.downgrade();
        let count = Rc::new(Cell::new(0));
        let calls = count.clone();
        let target = element.clone();
        let scope = RibbonKeyTipResult::scope(Some(Rc::new(move || {
            calls.set(calls.get() + 1);
            RibbonItemsSource::from_values(vec![Some(target.clone()), Some(target.clone()), None])
        })));
        drop(element);
        assert!(weak.upgrade().is_some());
        assert_eq!(count.get(), 0);
        let callback = scope.next_scope().unwrap();
        let values = callback().iter().collect::<Vec<_>>();
        assert!(
            values[0]
                .as_ref()
                .unwrap()
                .ptr_eq(values[1].as_ref().unwrap())
        );
        assert!(values[2].is_none());
        assert_eq!(count.get(), 1);
        drop(scope);
        drop(values);
        assert!(weak.upgrade().is_some());
        drop(callback);
        assert!(weak.upgrade().is_none());
    });
}

#[test]
fn links_preserve_utf16_units_object_identity_and_null_in_both_directions() {
    run(|| {
        use ferroribbon_core::RibbonString;
        let source = Control::new();
        let target = Control::new();
        let text = RibbonString::from_utf16(vec![0xd800, 65, 0xdc00]);
        source.set_value(Item::text_property(), Some(text.clone()));
        RibbonItemHelper::link(&source, &target, Item::text_property().as_property(), true);
        let linked = target.get_value(Item::text_property()).unwrap();
        assert!(linked.ptr_eq(&text));
        assert_eq!(json!(linked.as_utf16()), corpus()["utf16"]);
        let original = Control::new();
        source.set_value(Item::object_property(), Some(Control::boxed(&original)));
        RibbonItemHelper::link(
            &source,
            &target,
            Item::object_property().as_property(),
            true,
        );
        assert_eq!(
            json!(
                Control::from_boxed(&target.get_value(Item::object_property()).unwrap())
                    .unwrap()
                    .ptr_eq(&original)
            ),
            corpus()["objectSame"]
        );
        target.set_value(Item::object_property(), None);
        assert_eq!(
            json!(source.get_value(Item::object_property()).is_none()),
            corpus()["nullState"]
        );
    });
}

#[repr(C)]
struct Host {
    base: Panel,
    calls: Cell<i32>,
}
ferro_class!(Host:Panel);
ferro_impl_classes!(Host:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,ferroui_controls::PanelImpl);
ferro_class_info!(Host {interfaces:[Rc<dyn crate::controls::IRibbonLayoutHost> => |value|Rc::new(HostView(value)) as Rc<dyn crate::controls::IRibbonLayoutHost>]});
impl Host {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: Panel::construct(),
            calls: Cell::new(0),
        })
    }
}
struct HostView(Ref<Host>);
impl crate::controls::IRibbonLayoutHost for HostView {
    fn as_object(&self) -> Ref<FerroObject> {
        self.0.clone().upcast()
    }
    fn invalidate_items_layout(&self) {
        self.0.calls.set(self.0.calls.get() + 1)
    }
}
#[test]
fn native_owner_walk_cache_invalidation_and_id_precedence_follow_original() {
    run(|| {
        use crate::controls::IRibbonLayoutHost;
        use ferroribbon_core::RibbonString;
        let outer = Host::new();
        let near = Host::new();
        let element = Control::new();
        near.children().add(element.clone());
        outer.children().add(near.clone());
        element.measure(ferroui_base::Size::new(10., 10.));
        assert!(element.is_measure_valid());
        RibbonItemHelper::invalidate_host_layout(&element);
        assert!(!element.is_measure_valid());
        let found =
            RibbonItemHelper::find_ancestor::<Rc<dyn IRibbonLayoutHost>>(Some(&element)).unwrap();
        let skip =
            RibbonItemHelper::find_ancestor::<Rc<dyn IRibbonLayoutHost>>(Some(&near)).unwrap();
        assert_eq!(
            json!({"invalidations":1,"near":near.calls.get(),"outer":outer.calls.get(),"found":found.as_object().ptr_eq(&near),"skipSelf":skip.as_object().ptr_eq(&outer)}),
            corpus()["hosts"]
        );
        assert!(
            RibbonItemHelper::find_ancestor::<Ref<Host>>(Some(&element))
                .unwrap()
                .ptr_eq(&near)
        );
        assert!(RibbonItemHelper::find_ancestor::<Ref<Panel>>(None).is_none());
        let item = Control::new();
        item.set_name(Some("named".into()));
        let ids = [
            RibbonItemHelper::get_id(&item, Some("explicit".into()), Some("command".into())),
            RibbonItemHelper::get_id(&item, Some("".into()), Some("command".into())),
            RibbonItemHelper::get_id(&item, Some(" ".into()), Some("command".into())),
        ];
        assert_eq!(
            json!(ids.map(|v| v.map(|v| v.to_utf8().unwrap()))),
            corpus()["ids"]
        );
        item.set_name(Some("".into()));
        assert_eq!(
            json!(
                RibbonItemHelper::get_id(&item, None, Some("command".into()))
                    .unwrap()
                    .to_utf8()
                    .unwrap()
            ),
            corpus()["fallback"]
        );
        assert_eq!(
            json!(RibbonItemHelper::get_id(&item, None, Some("".into())).is_none()),
            json!(corpus()["missing"].is_null())
        );
        let id = RibbonString::from_utf16(vec![0xd800]);
        assert!(
            RibbonItemHelper::get_id(&item, Some(id.clone()), None)
                .unwrap()
                .ptr_eq(&id)
        );
    });
}
