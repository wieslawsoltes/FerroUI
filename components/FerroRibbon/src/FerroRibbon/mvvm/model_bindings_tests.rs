use super::*;
use ferroribbon_core::collections::RibbonItems;
use ferroui_system::events::Event;
use serde_json::{Value, json};
use std::collections::HashMap;
struct Model {
    name: String,
    value: Cell<i32>,
    event: Event<str>,
}
impl Model {
    fn new(name: &str) -> Rc<Self> {
        Rc::new(Self {
            name: name.into(),
            value: Cell::new(0),
            event: Event::new(),
        })
    }
    fn raise(&self, name: &str) {
        self.event.raise(name);
    }
}
impl PartialEq for Model {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl INotifyPropertyChanged for Model {
    fn property_changed(&self) -> &Event<str> {
        &self.event
    }
}
// The unchanged C# reference seam installs DispatcherQueue.Current by default.
// This helper owns the corresponding native per-thread test context.
fn local_scope() -> Rc<RibbonModelBindings> {
    let _context = Dispatcher::current_dispatcher();
    RibbonModelBindings::new()
}
type Trace = Rc<RefCell<Vec<String>>>;
fn action(trace: &Trace, value: &str) -> RibbonAction {
    let trace = trace.clone();
    let value = value.to_string();
    Rc::new(move || trace.borrow_mut().push(value.clone()))
}
fn properties(values: &[&str]) -> Option<Vec<Option<RibbonString>>> {
    Some(values.iter().map(|v| Some((*v).into())).collect())
}
fn expected(name: &str) -> Value {
    let v: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/model-bindings.json")).unwrap();
    v["corpus"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == name)
        .unwrap()
        .clone()
}
fn check(name: &str, mut actual: Value) {
    actual
        .as_object_mut()
        .unwrap()
        .insert("name".into(), json!(name));
    assert_eq!(actual, expected(name), "source case {name}");
}
#[test]
fn whole_source_scope_order_children_and_reentrant_registration() {
    let trace = Trace::default();
    let model = Model::new("m");
    let b = local_scope();
    b.one_way(
        Some(model.clone()),
        Some(action(&trace, "one")),
        properties(&["Value"]),
    );
    b.watch(
        Some(model.clone()),
        Some(action(&trace, "watch")),
        properties(&["Other"]),
    );
    model.raise("Other");
    model.raise("");
    model.raise("value");
    b.suspend();
    model.raise("Value");
    let suspended = model.event.handler_count();
    b.one_way(
        Some(model.clone()),
        Some(action(&trace, "late")),
        properties(&["Value"]),
    );
    b.resume();
    let resumed = model.event.handler_count();
    model.raise("Value");
    b.close();
    b.close();
    b.add(Some(action(&trace, "cleanup-after")));
    check(
        "scope_order",
        json!({"trace":trace.borrow().clone(),"suspended":suspended,"resumed":resumed,"disposed":model.event.handler_count()}),
    );
    let trace = Trace::default();
    let model = Model::new("m");
    let parent = local_scope();
    parent.suspend();
    let child = parent.child();
    child.one_way(
        Some(model.clone()),
        Some(action(&trace, "child")),
        properties(&[]),
    );
    parent.resume();
    parent.suspend();
    let late = parent.child();
    late.watch(
        Some(model.clone()),
        Some(action(&trace, "late")),
        properties(&[]),
    );
    child.close();
    parent.resume();
    model.raise("");
    parent.close();
    check(
        "children",
        json!({"trace":trace.borrow().clone(),"disposed":model.event.handler_count(),"childDisposed":child.is_disposed(),"lateDisposed":late.is_disposed()}),
    );
    let trace = Trace::default();
    let model = Model::new("m");
    let b = local_scope();
    let weak = Rc::downgrade(&b);
    let t = trace.clone();
    b.one_way(
        Some(model.clone()),
        Some(Rc::new(move || {
            t.borrow_mut().push("apply".into());
            weak.upgrade().unwrap().close();
        })),
        properties(&[]),
    );
    model.raise("Value");
    b.close();
    check(
        "dispose_during_initial",
        json!({"trace":trace.borrow().clone(),"count":model.event.handler_count(),"disposed":b.is_disposed()}),
    );
    let trace = Trace::default();
    let model = Model::new("m");
    let b = local_scope();
    let dispose = Rc::new(Cell::new(false));
    let child = b.child();
    let d = dispose.clone();
    let weak = Rc::downgrade(&child);
    let t = trace.clone();
    b.watch(
        Some(model.clone()),
        Some(Rc::new(move || {
            t.borrow_mut().push("first".into());
            if d.get() {
                weak.upgrade().unwrap().close();
            }
        })),
        properties(&[]),
    );
    child.watch(
        Some(model.clone()),
        Some(action(&trace, "child")),
        properties(&[]),
    );
    b.suspend();
    dispose.set(true);
    b.resume();
    model.raise("Value");
    b.close();
    check(
        "resume_removes_child",
        json!({"trace":trace.borrow().clone(),"count":model.event.handler_count()}),
    );
}
#[test]
fn whole_source_collection_subscription_order() {
    let trace = Trace::default();
    let c = ObservableCollection::<Option<Rc<Model>>>::new_managed();
    let b = local_scope();
    b.collection(Some(Rc::new(c.clone())), Some(action(&trace, "collection")));
    b.watch_collection(Some(Rc::new(c.clone())), Some(action(&trace, "watch")));
    c.add(Some(Model::new("a")));
    b.suspend();
    c.add(Some(Model::new("b")));
    b.resume();
    b.dispose();
    c.clear();
    check("collection_order", json!({"trace":trace.borrow().clone()}));
    assert_eq!(c.collection_changed().handler_count(), 0);
}
#[test]
fn whole_source_two_way_uses_actual_native_property_without_feedback() {
    crate::group_tests::run(|| {
        use crate::controls::RibbonButton;
        let trace = Trace::default();
        let m = Model::new("m");
        let e = RibbonButton::new();
        e.set_label(Some("0".into()));
        let b = local_scope();
        let model = m.clone();
        let weak = e.downgrade();
        let t = trace.clone();
        let to_element: RibbonAction = Rc::new(move || {
            t.borrow_mut().push("toElement".into());
            weak.upgrade()
                .unwrap()
                .set_label(Some(model.value.get().to_string()));
        });
        let model = m.clone();
        let weak = e.downgrade();
        let t = trace.clone();
        let to_model: RibbonAction = Rc::new(move || {
            t.borrow_mut().push("toModel".into());
            model
                .value
                .set(weak.upgrade().unwrap().label().unwrap().parse().unwrap());
            model.raise("Value");
        });
        let baseline = e.property_changed_subscriber_count();
        b.two_way(
            Some(m.clone()),
            Some("Value".into()),
            Some(to_element),
            Some(e.clone().upcast()),
            Some(RibbonButton::label_property().as_property()),
            Some(to_model),
        );
        m.value.set(4);
        m.raise("Value");
        e.set_label(Some("9".into()));
        b.suspend();
        e.set_label(Some("12".into()));
        m.value.set(7);
        b.resume();
        b.dispose();
        e.set_label(Some("15".into()));
        check(
            "two_way",
            json!({"trace":trace.borrow().clone(),"model":m.value.get(),"element":e.label().unwrap().parse::<i32>().unwrap(),"modelHandlers":m.event.handler_count(),"elementHandlers":e.property_changed_subscriber_count()-baseline}),
        );
    });
}
fn names(values: &[Option<Rc<Model>>]) -> Vec<String> {
    values
        .iter()
        .map(|v| v.as_ref().unwrap().name.clone())
        .collect()
}
#[test]
fn whole_source_mirror_identity_duplicates_recreate_suspend_and_disposal() {
    for remove in [false, true] {
        let trace = Trace::default();
        let a = Model::new("a");
        let b = Model::new("b");
        let skip = Model::new("skip");
        let source =
            ObservableCollection::from_items_managed(vec![Some(a.clone()), Some(b.clone())]);
        let target = ObservableCollection::from_items_managed(vec![Some(Model::new("xaml"))]);
        let t = trace.clone();
        let target2 = target.clone();
        target.add_collection_changed(Rc::new(move |e| {
            t.borrow_mut().push(format!(
                "{:?}:{}",
                e.action,
                names(&target2.to_vec()).join(",")
            ))
        }));
        let scope = local_scope();
        let next = Rc::new(Cell::new(0));
        let scopes = Rc::new(RefCell::new(
            HashMap::<String, Rc<RibbonModelBindings>>::new(),
        ));
        let out = scopes.clone();
        let t = trace.clone();
        let mirror = scope.items(
            source.clone(),
            Rc::new(target.clone()),
            Rc::new(move |model: Option<Rc<Model>>, s| {
                let model = model.unwrap();
                next.set(next.get() + 1);
                let name = format!("{}{}", model.name, next.get());
                t.borrow_mut().push(format!("create:{name}"));
                s.add(Some(action(&t, &format!("dispose:{name}"))));
                out.borrow_mut().insert(name.clone(), s);
                if model.name == "skip" {
                    None
                } else {
                    Some(Model::new(&name))
                }
            }),
            remove,
            Some(action(&trace, "changing")),
            Some(action(&trace, "changed")),
        );
        let first = mirror.find_element(&Some(a.clone()));
        let snapshot = mirror.elements().iter().collect::<Vec<_>>();
        source.move_item(1, 0);
        source.add(Some(a.clone()));
        source.add(Some(skip));
        let recreated = scopes.borrow().get("b2").unwrap().clone();
        recreated.request_recreate();
        source.remove(&Some(a));
        scope.suspend();
        source.clear();
        source.add(Some(b));
        scope.resume();
        scope.dispose();
        // Ordinary CLR references and native DependencyObjects have separate source cases.
        let name = if remove {
            "plain_mirror_true"
        } else {
            "plain_mirror_false"
        };
        let exp = expected(name);
        let actual = json!({"name":name,"trace":trace.borrow().clone(),"unlinks":[],"snapshot":names(&snapshot),"first":first.map(|v|v.name.clone()),"target":names(&target.to_vec()),"elements":names(&mirror.elements().iter().collect::<Vec<_>>())});
        assert_eq!(actual, exp);
    }
}
#[test]
fn whole_source_create_failure_disposes_child_and_plain_list_arrangement() {
    let source = ObservableCollection::from_items_managed(vec![Some(Model::new("a"))]);
    let target = RibbonItems::new();
    let scope = local_scope();
    let trace = Trace::default();
    let t = trace.clone();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        scope.items(
            source.clone(),
            Rc::new(target.clone()),
            Rc::new(move |m, s| {
                s.add(Some(action(&t, "cleanup")));
                let _ = m;
                panic!("create failed")
            }),
            false,
            None,
            None,
        )
    }));
    assert!(result.is_err());
    let mirror = scope.items(
        source.clone(),
        Rc::new(target.clone()),
        Rc::new(|m, _| m),
        true,
        None,
        None,
    );
    source.add(Some(Model::new("b")));
    scope.dispose();
    check(
        "creation_failure",
        json!({"error":"create failed","trace":trace.borrow().clone(),"target":names(&target.to_vec()),"elements":names(&mirror.elements().iter().collect::<Vec<_>>())}),
    );
    let a = Model::new("a");
    let b = Model::new("b");
    let x = Model::new("x");
    let target = RibbonItems::from_values(vec![
        Some(x.clone()),
        Some(a.clone()),
        Some(x),
        Some(b.clone()),
    ]);
    RibbonModelBindings::arrange_contiguous(&target, &[Some(b.clone()), Some(a), Some(b)]);
    check("plain_arrange", json!({"target":names(&target.to_vec())}));
}

use crate::controls::{IRibbonItem, RibbonButton, RibbonButtonImpl, RibbonButtonImplExt};
use ferroui_base::{
    FerroObjectImpl, StyledElementImpl, VisualImpl,
    data::core::{ValueType, ValueTypes},
    ferro_class, ferro_class_info, ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{
    ButtonImpl, ContentControlImpl, Control, ControlImpl, primitives::TemplatedControlImpl,
};
#[repr(C)]
struct TrackingButton {
    base: RibbonButton,
    unlinks: Trace,
}
ferro_class!(TrackingButton:RibbonButton);
ferro_class_info!(TrackingButton {});
ferro_impl_classes!(TrackingButton:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl);
impl TrackingButton {
    fn new_trace(unlinks: Trace) -> Ref<Self> {
        instantiate(Self {
            base: RibbonButton::construct(),
            unlinks,
        })
    }
}
impl RibbonButtonImpl for TrackingButton {
    fn create_copy_instance(this: &Self) -> Ref<RibbonButton> {
        Self::new_trace(this.unlinks.clone()).upcast()
    }
    fn on_unlinked_core(this: &Self) {
        this.unlinks.borrow_mut().push(this.name().unwrap());
        Self::parent_on_unlinked_core(this);
    }
}
fn native_names(values: &[Option<Ref<Control>>]) -> Vec<String> {
    values
        .iter()
        .map(|v| v.as_ref().unwrap().name().unwrap())
        .collect()
}
#[test]
fn whole_source_native_mirror_unlinks_actual_copies_in_order_and_keeps_xaml_items() {
    crate::group_tests::run(|| {
        for remove in [false, true] {
            let trace = Trace::default();
            let unlinks = Trace::default();
            let a = Model::new("a");
            let b = Model::new("b");
            let source =
                ObservableCollection::from_items_managed(vec![Some(a.clone()), Some(b.clone())]);
            let xaml = RibbonButton::new();
            xaml.set_name(Some("xaml".into()));
            let target = ObservableCollection::from_items_managed(vec![Some(
                xaml.clone().upcast::<Control>(),
            )]);
            let baseline = target.clone();
            let t = trace.clone();
            target.add_collection_changed(Rc::new(move |e| {
                t.borrow_mut().push(format!(
                    "{:?}:{}",
                    e.action,
                    native_names(&baseline.to_vec()).join(",")
                ))
            }));
            let owner = local_scope();
            let next = Rc::new(Cell::new(0));
            let scopes = Rc::new(RefCell::new(HashMap::new()));
            let out = scopes.clone();
            let t = trace.clone();
            let u = unlinks.clone();
            let sources = Rc::new(RefCell::new(Vec::new()));
            let kept_sources = sources.clone();
            let copies = Rc::new(RefCell::new(Vec::new()));
            let kept_copies = copies.clone();
            let mirror = owner.items(
                source.clone(),
                Rc::new(target.clone()),
                Rc::new(move |m: Option<Rc<Model>>, s| {
                    let m = m.unwrap();
                    next.set(next.get() + 1);
                    let name = format!("{}{}", m.name, next.get());
                    t.borrow_mut().push(format!("create:{name}"));
                    s.add(Some(action(&t, &format!("dispose:{name}"))));
                    out.borrow_mut().insert(name.clone(), s);
                    if m.name == "skip" {
                        return None;
                    }
                    let original = TrackingButton::new_trace(u.clone());
                    original.set_label(Some(name.clone()));
                    let boxed = Control::boxed(original.clone());
                    let item = ValueTypes::try_convert(
                        Some(&boxed),
                        ValueType::of::<Rc<dyn IRibbonItem>>(),
                    )
                    .flatten()
                    .unwrap();
                    let copy = item
                        .downcast_ref::<Rc<dyn IRibbonItem>>()
                        .unwrap()
                        .create_linked_copy()
                        .unwrap();
                    copy.set_name(Some(name));
                    kept_sources.borrow_mut().push(original);
                    kept_copies.borrow_mut().push(copy.clone());
                    Some(copy)
                }),
                remove,
                Some(action(&trace, "changing")),
                Some(action(&trace, "changed")),
            );
            let first = mirror.find_element(&Some(a.clone()));
            let snapshot = mirror.elements().iter().collect::<Vec<_>>();
            source.move_item(1, 0);
            source.add(Some(a.clone()));
            source.add(Some(Model::new("skip")));
            let child = scopes.borrow().get("b2").unwrap().clone();
            child.request_recreate();
            source.remove(&Some(a));
            owner.suspend();
            source.clear();
            source.add(Some(b));
            owner.resume();
            owner.dispose();
            check(
                if remove {
                    "mirror_true"
                } else {
                    "mirror_false"
                },
                json!({"trace":trace.borrow().clone(),"unlinks":unlinks.borrow().clone(),"snapshot":native_names(&snapshot),"first":first.and_then(|v|v.name()),"target":native_names(&target.to_vec()),"elements":native_names(&mirror.elements().iter().collect::<Vec<_>>())}),
            );
            assert!(target.to_vec().iter().flatten().any(|v| v.ptr_eq(&xaml)));
            for (original, copy) in sources.borrow().iter().zip(copies.borrow().iter()) {
                let copy = copy.cast::<RibbonButton>().unwrap();
                let before = copy.label();
                original.set_label(Some("after disposal".into()));
                assert_eq!(
                    copy.label(),
                    before,
                    "UnlinkTree must disconnect even retained generated elements"
                );
            }
        }
    });
}
#[derive(Default)]
struct EqualModel {
    name: String,
}
impl PartialEq for EqualModel {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl EqualModel {
    fn new(name: &str) -> Rc<Self> {
        Rc::new(Self { name: name.into() })
    }
}
#[test]
fn source_mirror_distinguishes_reference_identity_from_target_equals() {
    let source = ObservableCollection::from_items_managed(vec![
        Some(EqualModel::new("a")),
        Some(EqualModel::new("b")),
    ]);
    let target = RibbonItems::new();
    let scope = local_scope();
    let created = Rc::new(Cell::new(0));
    let count = created.clone();
    let mirror = scope.items(
        source.clone(),
        Rc::new(target.clone()),
        Rc::new(move |m: Option<Rc<EqualModel>>, _| {
            count.set(count.get() + 1);
            Some(Model::new(&format!("{}{}", m.unwrap().name, count.get())))
        }),
        true,
        None,
        None,
    );
    let first = mirror.find_element(&source.get(0));
    source.move_item(1, 0);
    check(
        "reference_not_equals",
        json!({"created":created.get(),"target":names(&target.to_vec()),"same":first.same_reference(&mirror.find_element(&source.get(1)))}),
    );
    scope.dispose();
    let source = ObservableCollection::from_items_managed(vec![
        Some(Model::new("a")),
        Some(Model::new("b")),
    ]);
    let target = RibbonItems::from_values(vec![Some(EqualModel::new("xaml"))]);
    let scope = local_scope();
    let mirror = scope.items(
        source.clone(),
        Rc::new(target.clone()),
        Rc::new(|m: Option<Rc<Model>>, _| Some(EqualModel::new(&m.unwrap().name))),
        false,
        None,
        None,
    );
    source.move_item(1, 0);
    check(
        "target_equals",
        json!({"target":target.iter().map(|v|v.unwrap().name.clone()).collect::<Vec<_>>(),"elements":mirror.elements().iter().map(|v|v.unwrap().name.clone()).collect::<Vec<_>>()}),
    );
    scope.dispose();
    let source = ObservableCollection::from_items_managed(vec![Some(Model::new("a"))]);
    let target = RibbonItems::new();
    let owner = local_scope();
    owner.dispose();
    let created = Rc::new(Cell::new(0));
    let count = created.clone();
    let mirror = owner.items(
        source,
        Rc::new(target.clone()),
        Rc::new(move |m, _| {
            count.set(count.get() + 1);
            m
        }),
        true,
        None,
        None,
    );
    let before = target.len();
    mirror.reconcile();
    mirror.dispose();
    check(
        "disposed_owner_mirror",
        json!({"created":created.get(),"before":before,"after":target.len()}),
    );
}
#[test]
fn source_two_way_guards_reset_after_each_throwing_direction() {
    crate::group_tests::run(|| {
        let m = Model::new("m");
        let e = RibbonButton::new();
        e.set_label(Some("0".into()));
        let b = local_scope();
        let trace = Trace::default();
        let fail_element = Rc::new(Cell::new(false));
        let fail_model = Rc::new(Cell::new(false));
        let f = fail_element.clone();
        let t = trace.clone();
        let model = m.clone();
        let weak = e.downgrade();
        let to_element: RibbonAction = Rc::new(move || {
            t.borrow_mut().push("toElement".into());
            assert!(!f.get(), "element");
            weak.upgrade()
                .unwrap()
                .set_label(Some(model.value.get().to_string()));
        });
        let f = fail_model.clone();
        let t = trace.clone();
        let model = m.clone();
        let weak = e.downgrade();
        let to_model: RibbonAction = Rc::new(move || {
            t.borrow_mut().push("toModel".into());
            assert!(!f.get(), "model");
            model
                .value
                .set(weak.upgrade().unwrap().label().unwrap().parse().unwrap());
            model.raise("Value");
        });
        b.two_way(
            Some(m.clone()),
            Some("Value".into()),
            Some(to_element),
            Some(e.clone().upcast()),
            Some(RibbonButton::label_property().as_property()),
            Some(to_model),
        );
        let mut failures = 0;
        fail_element.set(true);
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| m.raise("Value"))).is_err() {
            failures += 1;
        }
        fail_element.set(false);
        m.value.set(4);
        m.raise("Value");
        fail_model.set(true);
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            e.set_label(Some("8".into()))
        }))
        .is_err()
        {
            failures += 1;
        }
        fail_model.set(false);
        e.set_label(Some("9".into()));
        b.dispose();
        check(
            "two_way_failure_guards",
            json!({"trace":trace.borrow().clone(),"failures":failures,"model":m.value.get(),"element":e.label().unwrap().parse::<i32>().unwrap()}),
        );
    });
}
#[test]
fn source_argument_validation_precedes_disposed_guard_selectively() {
    crate::group_tests::run(|| {
        let b = local_scope();
        b.dispose();
        let m = Model::new("m");
        let calls = Rc::new(Cell::new(0));
        let c = calls.clone();
        let f: RibbonAction = Rc::new(move || c.set(c.get() + 1));
        let mut errors = vec![];
        let mut probe = |name: &str, parameter: &str, action: &dyn Fn()| {
            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)).is_err() {
                errors.push(format!("{name}:{parameter}"));
            }
        };
        probe("one", "model", &|| {
            b.one_way(None, Some(f.clone()), properties(&[]))
        });
        probe("watch", "onChanged", &|| {
            b.watch(Some(m.clone()), None, properties(&[]))
        });
        probe("collection", "collection", &|| {
            b.collection(None, Some(f.clone()))
        });
        let e = RibbonButton::new();
        probe("two", "unused", &|| {
            b.two_way(
                None,
                Some("Value".into()),
                None,
                Some(e.clone().upcast()),
                Some(RibbonButton::label_property().as_property()),
                None,
            )
        });
        probe("element", "element", &|| {
            b.two_way(
                Some(m.clone()),
                Some("Value".into()),
                Some(f.clone()),
                None,
                Some(RibbonButton::label_property().as_property()),
                Some(f.clone()),
            )
        });
        probe("add", "dispose", &|| b.add(None));
        check(
            "disposed_argument_order",
            json!({"errors":errors,"apply":calls.get()}),
        );
    });
}
#[test]
fn explicit_disposal_releases_model_and_native_callback_actions_without_cycles() {
    crate::group_tests::run(|| {
        let scope = local_scope();
        let model = Model::new("m");
        let weak = Rc::downgrade(&model);
        let element = RibbonButton::new();
        let endpoint = element.downgrade();
        let bound = model.clone();
        scope.two_way(
            Some(model.clone()),
            Some("Value".into()),
            Some(Rc::new(move || {
                let _ = &bound;
                let _ = endpoint.upgrade();
            })),
            Some(element.clone().upcast()),
            Some(RibbonButton::label_property().as_property()),
            Some(Rc::new(|| {})),
        );
        drop(model);
        assert!(weak.upgrade().is_some(), "live scope retains its model");
        scope.dispose();
        assert!(
            weak.upgrade().is_none(),
            "explicit disposal releases entry closure model references"
        );
        let owned = Rc::new(1);
        let weak = Rc::downgrade(&owned);
        let strong = owned.clone();
        scope.add(Some(Rc::new(move || {
            assert_eq!(*strong, 1);
        })));
        drop(owned);
        assert!(
            weak.upgrade().is_none(),
            "Add after disposal executes immediately without retaining callback"
        );
    });
}
#[test]
fn dispatcher_inheritance_is_live_and_foreign_non_send_callbacks_fail_explicitly() {
    crate::group_tests::run(|| {
        let d = Dispatcher::current_dispatcher();
        let parent = RibbonModelBindings::with_dispatcher(Some(d.clone()));
        let child = parent.child();
        assert!(Arc::ptr_eq(&child.dispatcher().unwrap(), &d));
        let foreign = std::thread::spawn(Dispatcher::current_dispatcher)
            .join()
            .unwrap();
        parent.set_dispatcher(Some(foreign.clone()));
        assert!(Arc::ptr_eq(&child.dispatcher().unwrap(), &foreign));
        child.set_dispatcher(Some(d));
        let model = Model::new("m");
        let calls = Rc::new(Cell::new(0));
        let c = calls.clone();
        parent.watch(
            Some(model.clone()),
            Some(Rc::new(move || c.set(c.get() + 1))),
            properties(&[]),
        );
        let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| model.raise("Value")))
            .unwrap_err();
        let message = error
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| error.downcast_ref::<String>().map(String::as_str))
            .unwrap();
        assert!(message.contains("cross-thread callbacks require a Send"));
        assert_eq!(calls.get(), 0);
        assert_eq!(
            expected("foreign_dispatch")["trace"],
            json!(["watch"]),
            "source refresh after Resume is recorded; native dispatch gap is not called parity"
        );
        parent.dispose();
        assert_eq!(model.event.handler_count(), 0);
    });
}
struct AuditedList {
    values: RibbonList<Option<Rc<Model>>>,
    trace: Trace,
}
impl RibbonBindingList<Option<Rc<Model>>> for AuditedList {
    fn count(&self) -> usize {
        self.trace.borrow_mut().push("count".into());
        self.values.len()
    }
    fn index_of(&self, v: &Option<Rc<Model>>) -> Option<usize> {
        self.trace
            .borrow_mut()
            .push(format!("index:{}", v.as_ref().unwrap().name));
        self.values.iter().position(|x| x == *v)
    }
    fn insert(&self, i: usize, v: Option<Rc<Model>>) {
        self.trace
            .borrow_mut()
            .push(format!("insert:{i}:{}", v.as_ref().unwrap().name));
        self.values.insert(i, v);
    }
    fn remove_at(&self, i: usize) {
        self.trace.borrow_mut().push(format!("remove:{i}"));
        self.values.remove_at(i);
    }
    fn remove(&self, v: &Option<Rc<Model>>) -> bool {
        self.values.remove(v)
    }
}
#[test]
fn source_arrange_count_default_is_eager_and_plain_branch_reads_after_remove() {
    let a = Model::new("a");
    let b = Model::new("b");
    let x = Model::new("x");
    let target = AuditedList {
        values: RibbonItems::from_values(vec![
            Some(x.clone()),
            Some(a.clone()),
            Some(x),
            Some(b.clone()),
        ]),
        trace: Trace::default(),
    };
    RibbonModelBindings::arrange_contiguous(&target, &[Some(b.clone()), Some(a), Some(b)]);
    check(
        "arrange_call_order",
        json!({"trace":target.trace.borrow().clone(),"target":names(&target.values.to_vec())}),
    );
    let scope = local_scope();
    let source = ObservableCollection::new_managed();
    let target = RibbonItems::<Option<Rc<Model>>>::new();
    let mirror = scope.items(
        source.clone(),
        Rc::new(target),
        Rc::new(|m, _| m),
        false,
        None,
        None,
    );
    let first = mirror.elements();
    let second = mirror.elements();
    source.add(Some(Model::new("a")));
    let nonempty = mirror.elements();
    let fresh = mirror.elements();
    check(
        "element_array_identity",
        json!({"emptySame":first==second,"nonemptySame":nonempty==fresh,"snapshot":first.iter().count(),"nonemptyCount":nonempty.iter().count()}),
    );
    scope.dispose();
}
#[test]
fn source_event_snapshot_and_resume_reentry_preserve_direct_callback_rules() {
    let trace = Trace::default();
    let model = Model::new("m");
    let scope = local_scope();
    let weak = Rc::downgrade(&scope);
    let t = trace.clone();
    scope.watch(
        Some(model.clone()),
        Some(Rc::new(move || {
            t.borrow_mut().push("first".into());
            weak.upgrade().unwrap().suspend();
        })),
        properties(&[]),
    );
    scope.watch(
        Some(model.clone()),
        Some(action(&trace, "second")),
        properties(&[]),
    );
    model.raise("Value");
    let after_suspend = model.event.handler_count();
    scope.resume();
    let after_resume = model.event.handler_count();
    let suspended = scope.is_suspended();
    model.raise("Value");
    scope.dispose();
    check(
        "snapshot_and_resume_suspend",
        json!({"trace":trace.borrow().clone(),"afterSuspend":after_suspend,"afterResume":after_resume,"suspended":suspended,"handlers":model.event.handler_count()}),
    );
    let trace = Trace::default();
    let model = Model::new("m");
    let scope = local_scope();
    let weak = Rc::downgrade(&scope);
    let t = trace.clone();
    scope.watch(
        Some(model.clone()),
        Some(Rc::new(move || {
            t.borrow_mut().push("first".into());
            weak.upgrade().unwrap().dispose();
        })),
        properties(&[]),
    );
    scope.watch(
        Some(model.clone()),
        Some(action(&trace, "second")),
        properties(&[]),
    );
    scope.suspend();
    scope.resume();
    check(
        "resume_disposes_scope",
        json!({"trace":trace.borrow().clone(),"handlers":model.event.handler_count(),"disposed":scope.is_disposed()}),
    );
}

#[test]
fn reference_current_dispatcher_takes_precedence_over_application_fallback() {
    const NAME: &str = "mvvm::ribbon_model_bindings::tests::reference_current_dispatcher_takes_precedence_over_application_fallback";
    if std::env::var_os("FERRORIBBON_BINDING_CONTEXT_ISOLATED").is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env("FERRORIBBON_BINDING_CONTEXT_ISOLATED", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let scope = local_scope();
    let foreign = std::thread::spawn(Dispatcher::current_dispatcher)
        .join()
        .unwrap();
    RibbonModelBindings::set_fallback_dispatcher(Some(foreign));
    let model = Model::new("local");
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    scope.watch(
        Some(model.clone()),
        Some(Rc::new(move || count.set(count.get() + 1))),
        properties(&[]),
    );
    model.raise("Value");
    assert_eq!(
        calls.get(),
        1,
        "the reference's owning-thread queue bypasses the global application fallback"
    );
    scope.dispose();
    RibbonModelBindings::set_fallback_dispatcher(None);
}

#[test]
fn dropping_last_scope_owner_cancels_native_model_subscriptions() {
    let model = Model::new("m");
    let calls = Rc::new(Cell::new(0));
    let scope = local_scope();
    let weak = Rc::downgrade(&scope);
    let count = calls.clone();
    scope.watch(
        Some(model.clone()),
        Some(Rc::new(move || count.set(count.get() + 1))),
        properties(&[]),
    );
    assert_eq!(model.event.handler_count(), 1);
    drop(scope);
    assert!(weak.upgrade().is_none());
    assert_eq!(model.event.handler_count(), 0);
    model.raise("Value");
    assert_eq!(calls.get(), 0);
}

#[test]
fn dropping_parent_owner_disposes_retained_child_scope() {
    let model = Model::new("m");
    let parent = local_scope();
    let weak_parent = Rc::downgrade(&parent);
    let child = parent.child();
    child.watch(Some(model.clone()), Some(Rc::new(|| {})), properties(&[]));
    assert_eq!(model.event.handler_count(), 1);
    drop(parent);
    assert!(weak_parent.upgrade().is_none());
    assert!(child.is_disposed());
    assert_eq!(model.event.handler_count(), 0);
    child.dispose();
}

#[test]
fn final_owner_drop_cleans_registration_installed_after_initial_reentrant_dispose() {
    let model = Model::new("m");
    let scope = local_scope();
    let weak = Rc::downgrade(&scope);
    let trace = Trace::default();
    let output = trace.clone();
    scope.one_way(
        Some(model.clone()),
        Some(Rc::new(move || {
            output.borrow_mut().push("apply".into());
            weak.upgrade().unwrap().dispose();
        })),
        properties(&[]),
    );
    model.raise("Value");
    scope.dispose();
    check(
        "dispose_during_initial",
        json!({"trace":trace.borrow().clone(),"count":model.event.handler_count(),"disposed":scope.is_disposed()}),
    );
    drop(scope);
    assert_eq!(
        model.event.handler_count(),
        0,
        "the last Rust owner must retire the late registration"
    );
    model.raise("Value");
    assert_eq!(&*trace.borrow(), &["apply"]);
}
