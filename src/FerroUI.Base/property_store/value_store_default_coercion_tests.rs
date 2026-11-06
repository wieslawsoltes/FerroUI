//! Keep the original metadata guard, owned values and effective-value local
//! observable across coercion, metadata replacement, creation and cleanup.

use super::*;
use crate::{FerroObjectImpl, Ref, StaticType, StyledPropertyMetadata, StyledPropertyOptions};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Weak;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operation { Clone, Equals, Coerce, Notify, Drop, CallbackDrop }

impl Operation {
    const ALL: [Self; 6] = [Self::Clone, Self::Equals, Self::Coerce, Self::Notify, Self::Drop, Self::CallbackDrop];
}

#[derive(Clone, Copy, Debug)]
enum Action { Panic, Read, Write, RemoveEntry }

#[derive(Clone, Copy, Debug)]
struct Injection {
    operation: Operation,
    occurrence: usize,
    action: Action,
}

struct Trace {
    active: Cell<bool>,
    next: Cell<usize>,
    counts: [Cell<usize>; 6],
    events: RefCell<Vec<String>>,
    injection: Cell<Option<Injection>>,
    fired: Cell<bool>,
    replacement_at: Cell<Option<Operation>>,
    replaced: Cell<bool>,
    owner: WeakRef<FerroObject>,
    property: Cell<Option<&'static StyledProperty<Tracked>>>,
    original_metadata: RefCell<Option<Weak<StyledPropertyMetadata<Tracked>>>>,
    entry: RefCell<Option<Weak<dyn EffectiveValueDyn>>>,
}

impl Trace {
    fn value(self: &Rc<Self>, number: i32) -> Tracked {
        let serial = self.next.get();
        self.next.set(serial + 1);
        Tracked { number, serial, trace: self.clone() }
    }

    fn record(self: &Rc<Self>, operation: Operation, detail: String) {
        if !self.active.get() { return; }
        // Weak observations do not create an extra owning guard that could
        // hide a metadata or returned-effective-value lifetime regression.
        let metadata_count = self.original_metadata.borrow().as_ref().map_or(0, Weak::strong_count);
        let entry_count = self.entry.borrow().as_ref().map_or(0, Weak::strong_count);
        self.events.borrow_mut().push(format!("{operation:?}:{detail}; metadata={metadata_count}; entry={entry_count}"));
        let count = &self.counts[operation as usize];
        let occurrence = count.get() + 1;
        count.set(occurrence);
        if self.replacement_at.get() == Some(operation) {
            self.replacement_at.set(None);
            self.replace_metadata();
        }
        let Some(injection) = self.injection.get() else { return };
        if injection.operation != operation || injection.occurrence != occurrence { return; }
        // Disarm before any reentry or destructor panic.
        self.injection.set(None);
        self.fired.set(true);
        self.events.borrow_mut().push(format!("inject:{:?}", injection.action));
        if let Action::Panic = injection.action { panic!("injected default-coercion operation panic"); }
        let owner = self.owner.upgrade().unwrap();
        let property = self.property.get().unwrap();
        match injection.action {
            Action::Read => drop(owner.get_value(property)),
            Action::Write => owner.set_value(property, self.value(90)),
            Action::RemoveEntry => {
                // Exercise the real store removal during publication. The
                // factory's returned Rc must remain its sole surviving owner
                // until the setter returns, then drop before default/metadata.
                assert!(owner.values().remove_effective_value(&owner, property));
            }
            Action::Panic => unreachable!(),
        }
    }

    fn replace_metadata(self: &Rc<Self>) {
        assert!(!self.replaced.replace(true));
        let property = self.property.get().unwrap();
        // Overrides are immutable registrations: replacing one requires the
        // existing public unregister operation before registering its successor.
        property.unregister(CoercionOwner::TYPE);
        property.override_metadata::<CoercionOwner>(
            StyledPropertyMetadata::new(Some(self.value(50))).with_coerce(coercer(self.clone(), 100, "replacement")),
        );
    }

    fn capture_entry(&self, owner: &FerroObject) {
        if !self.active.get() { return; }
        if let Some(entry) = owner.values().get_effective_value(self.property.get().unwrap()) {
            *self.entry.borrow_mut() = Some(Rc::downgrade(&entry));
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
    fn drop(&mut self) { self.trace.record(Operation::Drop, format!("{}={}", self.serial, self.number)); }
}

struct CallbackLife {
    trace: Rc<Trace>,
    name: &'static str,
}

impl Drop for CallbackLife {
    fn drop(&mut self) { self.trace.record(Operation::CallbackDrop, self.name.to_string()); }
}

fn coercer(trace: Rc<Trace>, delta: i32, name: &'static str) -> impl Fn(&FerroObject, Tracked) -> Tracked {
    let life = CallbackLife { trace: trace.clone(), name };
    move |_: &FerroObject, mut value: Tracked| {
        let _retain_whole_capture = &life;
        trace.record(Operation::Coerce, format!("{name}:{}={}", value.serial, value.number));
        value.number += delta;
        value
    }
}

#[repr(C)]
struct CoercionOwner { base: FerroObject }
crate::ferro_class!(CoercionOwner: FerroObject);
crate::ferro_impl_classes!(CoercionOwner: FerroObjectImpl);

impl CoercionOwner {
    fn new() -> Ref<Self> { crate::instantiate(Self { base: FerroObject::construct() }) }
}

fn unique_name() -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!("DemandRootedDefault{}", NEXT.fetch_add(1, Ordering::Relaxed))
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    panicked: bool,
    fired: bool,
    replaced: bool,
    counts: [usize; 6],
    events: Vec<String>,
    value: Option<i32>,
    after_write: Option<i32>,
    original_metadata_alive: bool,
    entry_owners: usize,
}

fn run(original: bool, delta: i32, replacement_at: Option<Operation>, injection: Option<Injection>) -> Outcome {
    let concrete = CoercionOwner::new();
    let owner: &FerroObject = &concrete;
    let trace = Rc::new(Trace {
        active: Cell::new(false), next: Cell::new(0),
        counts: std::array::from_fn(|_| Cell::new(0)), events: RefCell::new(Vec::new()),
        injection: Cell::new(injection), fired: Cell::new(false), replacement_at: Cell::new(replacement_at),
        replaced: Cell::new(false), owner: owner.to_ref().downgrade(), property: Cell::new(None),
        original_metadata: RefCell::new(None), entry: RefCell::new(None),
    });
    let notifying = trace.clone();
    let property = FerroProperty::register_with::<FerroObject, _>(
        &unique_name(), StyledPropertyOptions::new(trace.value(1)).notifying(move |owner, before| {
            if before { notifying.capture_entry(owner); }
            notifying.record(Operation::Notify, before.to_string());
        }),
    );
    trace.property.set(Some(property));
    // Use a replaceable derived-owner override, not the immutable declaring
    // owner's metadata. No test Rc is kept alive across the operation.
    property.override_metadata::<CoercionOwner>(
        StyledPropertyMetadata::new(Some(trace.value(5))).with_coerce(coercer(trace.clone(), delta, "original")),
    );
    let metadata = property.get_metadata_for(owner);
    *trace.original_metadata.borrow_mut() = Some(Rc::downgrade(&metadata));
    drop(metadata);
    trace.active.set(true);
    let panicked = catch_unwind(AssertUnwindSafe(|| {
        if original { owner.values().original_coerce_default_value(owner, property); }
        else { owner.values().coerce_default_value(owner, property); }
    })).is_err();
    trace.active.set(false);
    let value = owner.values().get_effective_value(property).map(|entry| cast_effective_value::<Tracked>(&*entry).value().number);
    let original_metadata_alive = trace.original_metadata.borrow().as_ref().unwrap().strong_count() != 0;
    let entry_owners = trace.entry.borrow().as_ref().map_or(0, Weak::strong_count);
    // A subsequent ordinary write proves which metadata the factory retained.
    // It is outside the trace and is never attempted after a partial unwind.
    let after_write = if !panicked && value.is_some() {
        owner.set_value(property, trace.value(3));
        Some(owner.get_value(property).number)
    } else { None };
    let events = std::mem::take(&mut *trace.events.borrow_mut());
    Outcome {
        panicked, fired: trace.fired.get(), replaced: trace.replaced.get(),
        counts: std::array::from_fn(|i| trace.counts[i].get()), events, value, after_write,
        original_metadata_alive, entry_owners,
    }
}

#[test]
fn default_coercion_matches_original_metadata_values_factory_and_unwind() {
    for delta in [0, 1] {
        for replacement_at in [None, Some(Operation::Coerce), Some(Operation::Equals)] {
            let baseline = run(true, delta, replacement_at, None);
            assert!(!baseline.panicked);
            assert_eq!(run(false, delta, replacement_at, None), baseline);
            for operation in Operation::ALL {
                for occurrence in 1..=baseline.counts[operation as usize] {
                    let injection = Some(Injection { operation, occurrence, action: Action::Panic });
                    let expected = run(true, delta, replacement_at, injection);
                    assert!(expected.panicked && expected.fired, "{operation:?}:{occurrence}");
                    assert_eq!(run(false, delta, replacement_at, injection), expected,
                        "delta={delta}, replacement={replacement_at:?}, {injection:?}");
                }
            }
        }
    }
}

#[test]
fn default_coercion_keeps_metadata_reload_and_returned_entry_cleanup_during_reentry() {
    for replacement_at in [Some(Operation::Coerce), Some(Operation::Equals)] {
        let expected = run(true, 1, replacement_at, None);
        assert!(expected.replaced && !expected.original_metadata_alive);
        assert_eq!((expected.value, expected.after_write), (Some(6), Some(103)));
        assert_eq!(expected.counts[Operation::CallbackDrop as usize], 1);
        assert_eq!(run(false, 1, replacement_at, None), expected);
    }
    let baseline = run(true, 1, None, None);
    let mut injections = vec![
        Injection { operation: Operation::Clone, occurrence: 1, action: Action::Read },
        Injection { operation: Operation::Equals, occurrence: 1, action: Action::Read },
        Injection { operation: Operation::Coerce, occurrence: 1, action: Action::Write },
        Injection { operation: Operation::Equals, occurrence: 1, action: Action::Write },
        Injection { operation: Operation::Notify, occurrence: 1, action: Action::RemoveEntry },
    ];
    injections.extend((1..=baseline.counts[Operation::Drop as usize]).map(|occurrence| {
        Injection { operation: Operation::Drop, occurrence, action: Action::Read }
    }));
    for injection in injections {
        let expected = run(true, 1, None, Some(injection));
        assert!(expected.fired, "{injection:?}");
        if matches!(injection.action, Action::Write) && cfg!(debug_assertions) {
            assert!(expected.panicked, "the reentrant write must reach the original duplicate-insertion assertion");
        }
        if matches!(injection.action, Action::RemoveEntry) {
            assert!(!expected.panicked);
            assert_eq!((expected.value, expected.entry_owners), (None, 0));
        }
        assert_eq!(run(false, 1, None, Some(injection)), expected, "{injection:?}");
    }
}

#[test]
fn default_coercion_retains_partial_eq_nan_and_absent_callback_behavior() {
    assert!(!std::mem::needs_drop::<CoercedDefaultFactory<'_, Tracked>>());
    for original in [false, true] {
        let owner = FerroObject::new();
        for (default, with_coercer, creates_entry) in [(1.0, false, false), (1.0, true, false), (f64::NAN, true, true)] {
            let mut options = StyledPropertyOptions::new(default);
            if with_coercer { options = options.coerce(|_, value| value); }
            let property = FerroProperty::register_with::<FerroObject, _>(&unique_name(), options);
            if original { owner.values().original_coerce_default_value(&owner, property); }
            else { owner.values().coerce_default_value(&owner, property); }
            let entry = owner.values().get_effective_value(property);
            assert_eq!(entry.is_some(), creates_entry);
            if let Some(entry) = entry {
                assert!(cast_effective_value::<f64>(&*entry).value().is_nan());
                assert!(!entry.can_remove());
            }
        }
    }
}

// Copied from bbff5971; only the oracle method name differs.
impl ValueStore {
    fn original_coerce_default_value<T: PropertyValue>(&self, owner: &FerroObject, property: &'static StyledProperty<T>) {
        let metadata = property.get_metadata_for(owner);
        let Some(coerce) = metadata.coerce_value() else { return };
        let default = metadata.default_value().clone();
        let coerced = coerce(owner, default.clone());
        if default == coerced {
            return;
        }
        // The default value isn't valid according to the coerce function, so
        // an effective value entry is needed.
        let effective = self.create_effective_value(owner, property);
        self.add_effective_value(owner, property, effective.clone());
        effective.set_coerced_default_value_and_raise(owner, coerced);
    }
}
