//! Compare the original inherited-change algorithm and its real store callers.
//! Values can reenter or panic from Clone, equality, notification and Drop.

use super::*;
use crate::{FerroObjectImpl, FerroPropertyRegistry, Ref, StaticType, StyledPropertyOptions, WeakRef};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Copy, Debug)]
enum Operation { Clone, Equals, Notify, Drop }

impl Operation {
    const ALL: [Self; 4] = [Self::Clone, Self::Equals, Self::Notify, Self::Drop];
}

#[derive(Clone, Copy, Debug)]
enum Action { Panic, Read, Write, ChangeNewValue }

#[derive(Clone, Copy, Debug)]
struct Injection {
    operation: Operation,
    occurrence: usize,
    action: Action,
}

struct Trace {
    active: Cell<bool>,
    next: Cell<usize>,
    counts: [Cell<usize>; 4],
    events: RefCell<Vec<String>>,
    injection: Cell<Option<Injection>>,
    fired: Cell<bool>,
    target: WeakRef<FerroObject>,
    new_parent: WeakRef<FerroObject>,
    property: Cell<Option<&'static StyledProperty<Tracked>>>,
}

impl Trace {
    fn value(self: &Rc<Self>, number: i32) -> Tracked {
        let serial = self.next.get();
        self.next.set(serial + 1);
        Tracked { number, serial, trace: self.clone() }
    }

    fn record(self: &Rc<Self>, operation: Operation, detail: String) {
        if !self.active.get() { return; }
        self.events.borrow_mut().push(format!("{operation:?}:{detail}"));
        let count = &self.counts[operation as usize];
        count.set(count.get() + 1);
        let Some(injection) = self.injection.get() else { return };
        if injection.operation as usize != operation as usize || injection.occurrence != count.get() {
            return;
        }
        // Disarm before any user action, including a destructor panic.
        self.injection.set(None);
        self.fired.set(true);
        self.events.borrow_mut().push(format!("inject:{:?}", injection.action));
        if let Action::Panic = injection.action {
            panic!("injected inherited-change operation panic");
        }
        let property = self.property.get().unwrap();
        match injection.action {
            Action::Read => drop(self.target.upgrade().unwrap().get_value(property)),
            Action::Write => self.target.upgrade().unwrap().set_value(property, self.value(90)),
            Action::ChangeNewValue => self.new_parent.upgrade().unwrap().set_value(property, self.value(70)),
            Action::Panic => unreachable!(),
        }
    }
}

struct Tracked {
    number: i32,
    serial: usize,
    trace: Rc<Trace>,
}

impl Clone for Tracked {
    fn clone(&self) -> Self {
        self.trace.record(Operation::Clone, format!("{}={}", self.serial, self.number));
        self.trace.value(self.number)
    }
}

impl PartialEq for Tracked {
    fn eq(&self, other: &Self) -> bool {
        self.trace.record(Operation::Equals, format!("{}={} / {}={}", self.serial, self.number, other.serial, other.number));
        self.number == other.number
    }
}

impl Drop for Tracked {
    fn drop(&mut self) {
        self.trace.record(Operation::Drop, format!("{}={}", self.serial, self.number));
    }
}

fn unique_name() -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!("DemandRootedInheritance{}", NEXT.fetch_add(1, Ordering::Relaxed))
}

fn original_callback<T: PropertyValue>(
    source: &dyn EffectiveValueDyn,
    owner: &FerroObject,
    old_value: Option<&dyn EffectiveValueDyn>,
    new_value: Option<&dyn EffectiveValueDyn>,
) {
    cast_effective_value::<T>(source).original_inherited_value_changed(owner, old_value, new_value);
}

fn callback<T: PropertyValue>(original: bool) -> crate::ferro_property::InheritedValueChanged {
    if original { original_callback::<T> } else { EffectiveValue::<T>::notify_inherited_value_changed }
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    panicked: bool,
    fired: bool,
    counts: [usize; 4],
    events: Vec<String>,
    values: (Option<i32>, Option<i32>, Option<i32>),
}

fn current(owner: &FerroObject, property: &'static StyledProperty<Tracked>) -> Option<i32> {
    let value = owner.values().get_effective_value(property)?;
    let number = cast_effective_value::<Tracked>(&*value).value.borrow().number;
    Some(number)
}

fn run(original: bool, present: (bool, bool), equal: bool, injection: Option<Injection>) -> Outcome {
    let old_parent = FerroObject::new();
    let new_parent = FerroObject::new();
    let target = FerroObject::new();
    let trace = Rc::new(Trace {
        active: Cell::new(false), next: Cell::new(0),
        counts: std::array::from_fn(|_| Cell::new(0)),
        events: RefCell::new(Vec::new()), injection: Cell::new(injection), fired: Cell::new(false),
        target: target.downgrade(), new_parent: new_parent.downgrade(), property: Cell::new(None),
    });
    let property = StyledProperty::create(
        &unique_name(), FerroObject::TYPE, FerroObject::TYPE,
        StyledPropertyMetadata::new(Some(trace.value(5))), true, None, None, false,
    );
    // Both routes use the real registry/store; the original callback contains
    // the exact pre-change method body below, with only its name changed.
    property.set_inherited_value_changed(Some(callback::<Tracked>(original)));
    FerroPropertyRegistry::instance().register(FerroObject::TYPE, property);
    trace.property.set(Some(property));
    let old_number = if equal && !present.1 { 5 } else { 11 };
    let new_number = if equal { if present.0 { old_number } else { 5 } } else { 29 };
    old_parent.set_value(property, trace.value(old_number));
    new_parent.set_value(property, trace.value(new_number));
    let observed = trace.clone();
    let _subscription = target.property_changed(move |e| {
        if e.property() != property.as_property() { return; }
        let old = e.old_value().unwrap().downcast_ref::<Tracked>().unwrap();
        let new = e.new_value().downcast_ref::<Tracked>().unwrap();
        observed.record(Operation::Notify, format!("{}={} -> {}={}; {:?}; {}",
            old.serial, old.number, new.serial, new.number, e.priority(), e.is_inherited_value_change()));
    });
    let old = present.0.then(|| old_parent.values().get_effective_value(property).unwrap());
    let new = present.1.then(|| new_parent.values().get_effective_value(property).unwrap());
    let source = old.as_deref().or(new.as_deref()).unwrap();
    trace.active.set(true);
    let panicked = catch_unwind(AssertUnwindSafe(|| {
        source.raise_inherited_value_changed(property, &target, old.as_deref(), new.as_deref());
    })).is_err();
    trace.active.set(false);
    let values = (current(&old_parent, property), current(&new_parent, property), current(&target, property));
    let events = std::mem::take(&mut *trace.events.borrow_mut());
    Outcome { panicked, fired: trace.fired.get(), counts: std::array::from_fn(|i| trace.counts[i].get()), events, values }
}

#[test]
fn inherited_notification_matches_original_values_defaults_and_unwind() {
    for present in [(true, true), (true, false), (false, true)] {
        for equal in [false, true] {
            let expected = run(true, present, equal, None);
            assert!(!expected.panicked);
            assert_eq!(run(false, present, equal, None), expected);
            for operation in Operation::ALL {
                for occurrence in 1..=expected.counts[operation as usize] {
                    let injection = Some(Injection { operation, occurrence, action: Action::Panic });
                    let expected = run(true, present, equal, injection);
                    assert!(expected.panicked && expected.fired, "{present:?}, equal={equal}, {injection:?}");
                    assert_eq!(run(false, present, equal, injection), expected,
                        "{present:?}, equal={equal}, {injection:?}");
                }
            }
        }
    }
}

#[test]
fn inherited_notification_matches_original_clone_equality_and_handler_reentry() {
    for present in [(true, true), (true, false), (false, true)] {
        let baseline = run(true, present, false, None);
        let mut injections = vec![
            Injection { operation: Operation::Clone, occurrence: 1, action: Action::Read },
            Injection { operation: Operation::Clone, occurrence: 1, action: Action::ChangeNewValue },
            Injection { operation: Operation::Clone, occurrence: 2, action: Action::Write },
            Injection { operation: Operation::Equals, occurrence: 1, action: Action::Read },
            Injection { operation: Operation::Equals, occurrence: 1, action: Action::Write },
            Injection { operation: Operation::Notify, occurrence: 1, action: Action::Write },
        ];
        injections.extend((1..=baseline.counts[Operation::Drop as usize]).map(|occurrence| {
            Injection { operation: Operation::Drop, occurrence, action: Action::Read }
        }));
        for injection in injections {
            let expected = run(true, present, false, Some(injection));
            assert!(expected.fired, "{present:?}, {injection:?}");
            assert_eq!(run(false, present, false, Some(injection)), expected, "{present:?}, {injection:?}");
        }
    }
}

#[repr(C)]
struct MetadataOwner { base: FerroObject }
crate::ferro_class!(MetadataOwner: FerroObject);
crate::ferro_impl_classes!(MetadataOwner: FerroObjectImpl);

#[repr(C)]
struct MetadataChild { base: MetadataOwner }
crate::ferro_class!(MetadataChild: MetadataOwner);
crate::ferro_impl_classes!(MetadataChild: FerroObjectImpl);

impl MetadataChild {
    fn new() -> Ref<Self> {
        crate::instantiate(Self { base: MetadataOwner { base: FerroObject::construct() } })
    }
}

#[repr(C)]
struct AdditionalOwner { base: FerroObject }
crate::ferro_class!(AdditionalOwner: FerroObject);
crate::ferro_impl_classes!(AdditionalOwner: FerroObjectImpl);

impl AdditionalOwner {
    fn new() -> Ref<Self> { crate::instantiate(Self { base: FerroObject::construct() }) }
}

fn metadata_scenario(original: bool, attached: bool) -> (Vec<String>, Vec<i32>) {
    let events = Rc::new(RefCell::new(Vec::new()));
    let active = Rc::new(Cell::new(false));
    let (notified, recording) = (events.clone(), active.clone());
    let notifying: crate::ferro_property::NotifyingCallback = Box::new(move |_, before| {
        if recording.get() { notified.borrow_mut().push(format!("notifying:{before}")); }
    });
    let metadata = StyledPropertyMetadata::new(Some(10)).with_coerce(|owner, value| {
        assert!(owner.downcast_ref::<MetadataOwner>().is_some());
        value + 1
    });
    let property: &'static StyledProperty<i32> = if attached {
        let property = crate::AttachedProperty::create(
            &unique_name(), MetadataOwner::TYPE, MetadataOwner::TYPE, metadata, true, None, Some(notifying),
        );
        property.set_inherited_value_changed(Some(callback::<i32>(original)));
        FerroPropertyRegistry::instance().register(MetadataOwner::TYPE, property);
        FerroPropertyRegistry::instance().register_attached(MetadataOwner::TYPE, property);
        property
    } else {
        let property = StyledProperty::create(
            &unique_name(), MetadataOwner::TYPE, MetadataOwner::TYPE, metadata, true, None, Some(notifying), false,
        );
        property.set_inherited_value_changed(Some(callback::<i32>(original)));
        FerroPropertyRegistry::instance().register(MetadataOwner::TYPE, property);
        property
    };
    // The first entry retains the pre-override metadata. The second entry sees
    // the override. A callback using a fresh lookup instead of its source's
    // cached metadata changes both fallback values and subsequent setters.
    let first = MetadataChild::new();
    first.set_value(property, 30);
    property.override_metadata::<MetadataChild>(StyledPropertyMetadata::new(Some(20)).with_coerce(|_, value| value + 2));
    let second = MetadataChild::new();
    second.set_value(property, 30);
    let target = FerroObject::new();
    let changes = events.clone();
    let _subscription = target.property_changed(move |e| {
        if e.property() == property.as_property() {
            changes.borrow_mut().push(format!("change:{}->{}:{:?}:{}", e.get_old_value::<i32>().unwrap(),
                e.get_new_value::<i32>(), e.priority(), e.is_inherited_value_change()));
        }
    });
    let mut values = Vec::new();
    active.set(true);
    target.set_inheritance_parent(&first);
    values.push(target.get_value(property));
    first.set_value(property, 40);
    values.push(target.get_value(property));
    target.set_inheritance_parent(None);
    values.push(target.get_value(property));
    target.set_inheritance_parent(&second);
    values.push(target.get_value(property));
    target.set_value(property, 90);
    second.set_value(property, 40);
    values.push(target.get_value(property));
    target.clear_value(property);
    values.push(target.get_value(property));
    // An unrelated host uses type-safe metadata with no host-specific coercer.
    let unrelated = FerroObject::new();
    unrelated.set_value(property, 7);
    target.set_inheritance_parent(&unrelated);
    values.push(target.get_value(property));
    target.set_inheritance_parent(None);
    values.push(target.get_value(property));
    property.add_owner_with::<AdditionalOwner>(StyledPropertyMetadata::new(Some(30)).with_coerce(|_, value| value + 3));
    let additional = AdditionalOwner::new();
    additional.set_value(property, 8);
    target.set_inheritance_parent(&additional);
    values.push(target.get_value(property));
    active.set(false);
    let events = std::mem::take(&mut *events.borrow_mut());
    (events, values)
}

#[test]
fn inherited_notifications_keep_source_metadata_across_overrides_and_attached_hosts() {
    for attached in [false, true] {
        let expected = metadata_scenario(true, attached);
        assert_eq!(expected.1, [31, 41, 10, 32, 90, 42, 7, 10, 11]);
        assert!(expected.0.iter().any(|event| event == "change:10->31:Inherited:true"));
        assert!(expected.0.iter().any(|event| event == "change:20->32:Inherited:true"));
        assert_eq!(metadata_scenario(false, attached), expected);
    }
}

#[test]
fn explicit_inheritance_registration_and_opt_out_keep_public_behavior() {
    let plain = FerroProperty::register::<FerroObject, _>(&unique_name(), 1);
    let enabled = FerroProperty::register_with::<FerroObject, _>(
        &unique_name(), StyledPropertyOptions::new(2).inherits(true).inherits(false).inherits(true),
    );
    let disabled = FerroProperty::register_attached_with::<MetadataOwner, FerroObject, _>(
        &unique_name(), StyledPropertyOptions::new(3).inherits(true).inherits(false),
    );
    let attached = FerroProperty::register_attached_with::<MetadataOwner, FerroObject, _>(
        &unique_name(), StyledPropertyOptions::new(4).inherits(true),
    );
    let parent = FerroObject::new();
    let target = FerroObject::new();
    parent.set_value(plain, 11);
    parent.set_value(enabled, 12);
    parent.set_value(disabled, 13);
    parent.set_value(attached, 14);
    let raised = Rc::new(RefCell::new(Vec::new()));
    let changes = raised.clone();
    let _subscription = target.property_changed(move |e| changes.borrow_mut().push(e.property().id()));
    target.set_inheritance_parent(&parent);
    assert_eq!((target.get_value(plain), target.get_value(enabled), target.get_value(disabled), target.get_value(attached)),
        (1, 12, 3, 14));
    assert!(!plain.inherits() && enabled.inherits() && !disabled.inherits() && attached.inherits());
    let mut actual = raised.borrow().clone();
    let mut expected = vec![enabled.id(), attached.id()];
    actual.sort_unstable();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}

// Copied from bbff5971; only the method name differs.
impl<T: PropertyValue> EffectiveValue<T> {
    fn original_inherited_value_changed(
        &self,
        owner: &FerroObject,
        old_value: Option<&dyn EffectiveValueDyn>,
        new_value: Option<&dyn EffectiveValueDyn>,
    ) {
        debug_assert!(old_value.is_some() || new_value.is_some());
        let default = || self.metadata.default_value().clone();
        let o = old_value.map(|v| cast_effective_value::<T>(v).value()).unwrap_or_else(default);
        let n = new_value.map(|v| cast_effective_value::<T>(v).value()).unwrap_or_else(default);
        let priority = if new_value.is_some() { BindingPriority::Inherited } else { BindingPriority::Unset };
        if !value_equals(&o, &n) {
            owner.raise_inherited_property_changed(self.state.property, &o, &n, priority);
        }
    }
}
