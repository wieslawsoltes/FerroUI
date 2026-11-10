//! Compare complete original constructor frames, including partial unwind.

use super::*;
use crate::{ObjectType, StyledPropertyOptions, WeakRef};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Weak;
use std::sync::atomic::{AtomicUsize, Ordering};

impl<T: PropertyValue> EffectiveValue<T> {
    // The original constructor body, kept as the owning-frame oracle.
    fn original_new_for_construction(
        owner: &FerroObject,
        property: &'static StyledProperty<T>,
        inherited: Option<&EffectiveValue<T>>,
    ) -> Self {
        let metadata = property.get_metadata_for(owner);
        let value = match inherited {
            Some(inherited) => inherited.value(),
            None => metadata.default_value().clone(),
        };
        let uncommon = metadata.coerce_value().map(|coerce| {
            Box::new(UncommonFields {
                coerce: coerce.clone(),
                uncoerced_value: RefCell::new(value.clone()),
                uncoerced_base_value: RefCell::new(value.clone()),
            })
        });
        Self {
            metadata,
            state: EffectiveValueState::new(property),
            value: RefCell::new(value),
            base_value: RefCell::new(None),
            uncommon,
        }
    }
}

#[test]
fn frozen_metadata_construction_matches_original_owned_frames_and_unwind() {
    crate::ferro_property_metadata::construction_oracles::assert_table_frames();
}

#[test]
fn public_type_safe_metadata_keeps_unfrozen_clone_only_behavior() {
    crate::ferro_property_metadata::construction_oracles::assert_public_generation();
}

#[test]
fn frozen_metadata_generation_keeps_reentrant_input_ownership() {
    crate::ferro_property_metadata::construction_oracles::assert_generation_reentry();
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Clone,
    ValueDrop,
    CallbackDrop,
}

type Failure = Option<(Kind, usize)>;

#[derive(Debug, PartialEq, Eq)]
struct Event {
    kind: Kind,
    serial: usize,
    number: i32,
    callback: &'static str,
    callback_owners: usize,
    metadata_owners: usize,
}

struct Trace {
    active: Cell<bool>,
    next_serial: Cell<usize>,
    counts: [Cell<usize>; 3],
    failure: Cell<Failure>,
    fired: Cell<bool>,
    replace_on_clone: Cell<Option<usize>>,
    replaced: Cell<bool>,
    events: RefCell<Vec<Event>>,
    callback: RefCell<Option<Weak<dyn Fn(&FerroObject, Tracked) -> Tracked>>>,
    metadata: RefCell<Weak<StyledPropertyMetadata<Tracked>>>,
    owner: WeakRef<FerroObject>,
    property: Cell<Option<&'static StyledProperty<Tracked>>>,
}

impl Trace {
    fn value(self: &Rc<Self>, number: i32) -> Tracked {
        let serial = self.next_serial.get();
        self.next_serial.set(serial + 1);
        Tracked { serial, number, trace: self.clone() }
    }

    fn record(self: &Rc<Self>, kind: Kind, serial: usize, number: i32, callback_name: &'static str) {
        if !self.active.get() {
            return;
        }
        self.events.borrow_mut().push(Event {
            kind,
            serial,
            number,
            callback: callback_name,
            callback_owners: self.callback.borrow().as_ref().map_or(0, Weak::strong_count),
            metadata_owners: self.metadata.borrow().strong_count(),
        });
        let count = &self.counts[kind as usize];
        count.set(count.get() + 1);
        if self.failure.get() == Some((kind, count.get())) {
            self.failure.set(None);
            self.fired.set(true);
            panic!("injected effective-value construction operation");
        }
        if kind == Kind::Clone && self.replace_on_clone.get() == Some(count.get()) {
            // Disarm before registration can drop or clone another value.
            self.replace_on_clone.set(None);
            self.replaced.set(true);
            let owner = self.owner.upgrade().unwrap();
            let property = self.property.get().unwrap();
            property.unregister(FerroObject::TYPE);
            property.override_metadata::<FerroObject>(
                StyledPropertyMetadata::new(Some(self.value(99)))
                    .with_coerce(callback(self.clone(), "replacement")),
            );
            assert_eq!(property.get_metadata_for(&owner).default_value().number, 99);
        }
    }
}

struct Tracked {
    serial: usize,
    number: i32,
    trace: Rc<Trace>,
}

impl Clone for Tracked {
    fn clone(&self) -> Self {
        self.trace.record(Kind::Clone, self.serial, self.number, "");
        self.trace.value(self.number)
    }
}

impl PartialEq for Tracked {
    fn eq(&self, other: &Self) -> bool {
        self.number == other.number
    }
}

impl Drop for Tracked {
    fn drop(&mut self) {
        self.trace.record(Kind::ValueDrop, self.serial, self.number, "");
    }
}

struct CallbackDrop {
    trace: Rc<Trace>,
    name: &'static str,
}

impl Drop for CallbackDrop {
    fn drop(&mut self) {
        self.trace.record(Kind::CallbackDrop, 0, 0, self.name);
    }
}

fn callback(trace: Rc<Trace>, name: &'static str) -> impl Fn(&FerroObject, Tracked) -> Tracked {
    let retained = CallbackDrop { trace, name };
    move |_, value| {
        let _retained = &retained;
        value
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Snapshot {
    retains_original_metadata: bool,
    retains_original_callback: bool,
    value: i32,
    base_value: Option<i32>,
    uncommon_values: Option<(i32, i32)>,
    priority: BindingPriority,
    base_priority: BindingPriority,
}

#[derive(Debug, PartialEq, Eq)]
struct Observation {
    panicked: bool,
    fired: bool,
    replaced: bool,
    events: Vec<Event>,
    snapshots: Vec<Snapshot>,
    callback_owners_after: usize,
    metadata_owners_after: usize,
}

fn run(original: bool, inherited: bool, coerce: bool, replace_on_clone: Option<usize>, failure: Failure) -> Observation {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let owner = FerroObject::new();
    let trace = Rc::new(Trace {
        active: Cell::new(false),
        next_serial: Cell::new(0),
        counts: std::array::from_fn(|_| Cell::new(0)),
        failure: Cell::new(failure),
        fired: Cell::new(false),
        replace_on_clone: Cell::new(replace_on_clone),
        replaced: Cell::new(false),
        events: RefCell::new(Vec::new()),
        callback: RefCell::new(None),
        metadata: RefCell::new(Weak::new()),
        owner: owner.downgrade(),
        property: Cell::new(None),
    });
    let options = StyledPropertyOptions::new(trace.value(7));
    let options = if coerce { options.coerce(callback(trace.clone(), "original")) } else { options };
    let property = FerroProperty::register_with::<FerroObject, _>(
        &format!("CoercionConstruction{}", NEXT.fetch_add(1, Ordering::Relaxed)),
        options,
    );
    trace.property.set(Some(property));
    {
        let metadata = property.get_metadata_for(&owner);
        *trace.metadata.borrow_mut() = Rc::downgrade(&metadata);
        *trace.callback.borrow_mut() = metadata.coerce_value().map(Rc::downgrade);
    }
    let inherited = inherited.then(|| {
        let result = EffectiveValue::original_new_for_construction(&owner, property, None);
        result.value.replace(trace.value(23));
        result
    });
    let mut snapshots = Vec::new();
    trace.active.set(true);
    let panicked = catch_unwind(AssertUnwindSafe(|| {
        let effective = if original {
            EffectiveValue::original_new_for_construction(&owner, property, inherited.as_ref())
        } else {
            EffectiveValue::new(&owner, property, inherited.as_ref())
        };
        snapshots.push(Snapshot {
            retains_original_metadata: std::ptr::eq(Rc::as_ptr(&effective.metadata), trace.metadata.borrow().as_ptr()),
            retains_original_callback: match (&effective.uncommon, trace.callback.borrow().as_ref()) {
                (Some(uncommon), Some(callback)) => std::ptr::eq(Rc::as_ptr(&uncommon.coerce), callback.as_ptr()),
                (None, None) => true,
                _ => false,
            },
            value: effective.value.borrow().number,
            base_value: effective.base_value.borrow().as_ref().map(|value| value.number),
            uncommon_values: effective.uncommon.as_ref().map(|fields| (
                fields.uncoerced_value.borrow().number,
                fields.uncoerced_base_value.borrow().number,
            )),
            priority: effective.state.priority.get(),
            base_priority: effective.state.base_priority.get(),
        });
        drop(effective);
    })).is_err();
    trace.active.set(false);
    let callback_owners_after = trace.callback.borrow().as_ref().map_or(0, Weak::strong_count);
    let metadata_owners_after = trace.metadata.borrow().strong_count();
    let observed = Observation {
        panicked,
        fired: trace.fired.get(),
        replaced: trace.replaced.get(),
        events: trace.events.take(),
        snapshots,
        callback_owners_after,
        metadata_owners_after,
    };
    drop(inherited);
    observed
}

fn compare_failures(inherited: bool, coerce: bool, reentry: Option<usize>) -> Observation {
    let expected = run(true, inherited, coerce, reentry, None);
    assert!(!expected.panicked && !expected.fired);
    assert_eq!(run(false, inherited, coerce, reentry, None), expected);
    for kind in [Kind::Clone, Kind::ValueDrop, Kind::CallbackDrop] {
        let count = expected.events.iter().filter(|event| event.kind == kind).count();
        for occurrence in 1..=count {
            let failure = Some((kind, occurrence));
            let original = run(true, inherited, coerce, reentry, failure);
            assert!(original.panicked && original.fired, "{kind:?} occurrence {occurrence}");
            assert_eq!(run(false, inherited, coerce, reentry, failure), original,
                       "{kind:?} occurrence {occurrence}, inherited={inherited}, reentry={reentry:?}");
        }
    }
    expected
}

#[test]
fn effective_construction_matches_original_default_and_inherited_frames() {
    for inherited in [false, true] {
        for coerce in [false, true] {
            let observed = compare_failures(inherited, coerce, None);
            let result = &observed.snapshots[0];
            let value = if inherited { 23 } else { 7 };
            assert!(result.retains_original_metadata && result.retains_original_callback);
            assert_eq!(result.value, value);
            assert_eq!(result.base_value, None);
            assert_eq!(result.uncommon_values, coerce.then_some((value, value)));
            assert_eq!(result.priority, BindingPriority::Unset);
            assert_eq!(result.base_priority, BindingPriority::Unset);
            let clones: Vec<_> = observed.events.iter().filter(|event| event.kind == Kind::Clone).collect();
            assert_eq!(clones.len(), if coerce { 3 } else { 1 });
            if coerce {
                assert_eq!(clones[1].callback_owners, clones[0].callback_owners + 1);
                assert_eq!(clones[2].callback_owners, clones[1].callback_owners);
            }
        }
    }
}

#[test]
fn effective_construction_keeps_metadata_snapshot_during_clone_reentry() {
    for inherited in [false, true] {
        for occurrence in 1..=3 {
            let observed = compare_failures(inherited, true, Some(occurrence));
            assert!(observed.replaced);
            let result = &observed.snapshots[0];
            assert!(result.retains_original_metadata && result.retains_original_callback);
            assert_eq!(result.value, if inherited { 23 } else { 7 });
            if !inherited {
                assert_eq!(observed.metadata_owners_after, 0);
                assert_eq!(observed.callback_owners_after, 0);
            }
        }
    }
}
