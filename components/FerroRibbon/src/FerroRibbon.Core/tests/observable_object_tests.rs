use ferroribbon_core::{ObservableObject, ObservableObjectImpl};
use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
use std::{cell::RefCell, rc::Rc};

struct Model {
    base: ObservableObject,
    value: RefCell<f64>,
    trace: Rc<RefCell<Vec<String>>>,
}
impl Model {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            base: ObservableObject::construct(),
            value: RefCell::new(0.),
            trace: Rc::new(RefCell::new(Vec::new())),
        })
    }
    fn set(&self, value: f64) -> bool {
        ObservableObject::set_property(self, &self.value, value, "Value")
    }
}
impl INotifyPropertyChanged for Model {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for Model {
    fn on_property_changed(&self, name: &str) {
        self.trace
            .borrow_mut()
            .push(format!("override:{name}:{}", self.value.borrow()));
        self.property_changed().raise(name);
    }
}
#[test]
fn stores_before_override_and_suppresses_equal_writes_including_nan() {
    let m = Model::new();
    assert!(!m.set(-0.));
    assert_eq!(m.value.borrow().to_bits(), 0_f64.to_bits());
    assert!(m.set(1.));
    assert!(!m.set(1.));
    assert!(m.set(f64::NAN));
    assert!(!m.set(f64::from_bits(0x7ff8_0000_0000_0001)));
    assert_eq!(
        *m.trace.borrow(),
        ["override:Value:1", "override:Value:NaN"]
    );
}
#[test]
fn handlers_mutate_subscriptions_and_reenter_in_source_snapshot_order() {
    let m = Model::new();
    let weak = Rc::downgrade(&m);
    let trace = m.trace.clone();
    let removed = Rc::new(std::cell::Cell::new(0));
    let remove = removed.clone();
    m.property_changed().add(Rc::new(move |_| {
        let m = weak.upgrade().unwrap();
        trace.borrow_mut().push(format!("a:{}", m.value.borrow()));
        if *m.value.borrow() == 1. {
            m.property_changed().remove(remove.get());
            m.set(2.);
        }
    }));
    let trace = m.trace.clone();
    removed.set(
        m.property_changed()
            .add(Rc::new(move |_| trace.borrow_mut().push("b".into()))),
    );
    m.set(1.);
    assert_eq!(
        *m.trace.borrow(),
        ["override:Value:1", "a:1", "override:Value:2", "a:2", "b"]
    );
}
#[test]
fn exception_propagates_after_storage_and_stops_later_handlers() {
    let m = Model::new();
    let trace = m.trace.clone();
    m.property_changed().add(Rc::new(|_| panic!("handler")));
    m.property_changed().add(Rc::new(move |_| {
        trace.borrow_mut().push("unreachable".into())
    }));
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| m.set(3.))).is_err());
    assert_eq!(*m.value.borrow(), 3.);
    assert_eq!(*m.trace.borrow(), ["override:Value:3"]);
}
#[test]
fn explicit_removal_releases_captured_objects() {
    let m = Model::new();
    let retained = Rc::new(());
    let weak = Rc::downgrade(&retained);
    let token = m.property_changed().add(Rc::new(move |_| {
        let _ = &retained;
    }));
    assert!(weak.upgrade().is_some());
    assert!(m.property_changed().remove(token));
    assert!(weak.upgrade().is_none());
    assert!(!m.property_changed().remove(token));
}
#[test]
fn framework_collection_identity_and_notification_order() {
    use ferroui_system::collections::object_model::ObservableCollection;
    let collection = ObservableCollection::<String>::new();
    let alias = collection.clone();
    let trace = Rc::new(RefCell::new(Vec::new()));
    let t = trace.clone();
    let p = collection
        .property_changed()
        .add(Rc::new(move |name| t.borrow_mut().push(name.to_owned())));
    let t = trace.clone();
    let c = collection.add_collection_changed(Rc::new(move |args| {
        t.borrow_mut().push(format!("{:?}", args.action))
    }));
    alias.add("x".into());
    alias.set(0, "y".into());
    alias.remove_at(0);
    assert!(alias.ptr_eq(&collection));
    assert!(collection.is_empty());
    assert_eq!(
        *trace.borrow(),
        [
            "Count", "Item[]", "Add", "Item[]", "Replace", "Count", "Item[]", "Remove"
        ]
    );
    collection.property_changed().remove(p);
    collection.remove_collection_changed(c);
    collection.clear();
    assert_eq!(trace.borrow().len(), 8);
}

#[test]
fn unchanged_csharp_observable_traces() {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/observable.json")).unwrap();
    let m = Model::new();
    let changes = [
        m.set(-0.),
        m.set(1.),
        m.set(1.),
        m.set(f64::NAN),
        m.set(f64::from_bits(0x7ff8_0000_0000_0001)),
    ];
    assert_eq!(serde_json::json!(changes), reference["equal"]["changes"]);
    assert_eq!(
        serde_json::json!(*m.trace.borrow()),
        reference["equal"]["trace"]
    );
    let m = Model::new();
    let weak = Rc::downgrade(&m);
    let token = Rc::new(std::cell::Cell::new(0));
    let removed = token.clone();
    m.property_changed().add(Rc::new(move |_| {
        let m = weak.upgrade().unwrap();
        m.trace.borrow_mut().push(format!("a:{}", m.value.borrow()));
        if *m.value.borrow() == 1. {
            m.property_changed().remove(removed.get());
            m.set(2.);
        }
    }));
    let trace = m.trace.clone();
    token.set(
        m.property_changed()
            .add(Rc::new(move |_| trace.borrow_mut().push("b".into()))),
    );
    m.set(1.);
    assert_eq!(serde_json::json!(*m.trace.borrow()), reference["reentrant"]);
    // This source distinction is evidence for the documented native notification gap.
    assert_eq!(reference["names"], serde_json::json!([null, "", "Value"]));
    assert_eq!(reference["senders"], serde_json::json!([true, true, true]));
}
