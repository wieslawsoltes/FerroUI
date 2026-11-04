//! The oracle below retains the two original owned-value algorithm frames.
//! Compare user-observable operations instead of assuming a cleanup order.

use super::*;
use crate::ferro_property_metadata::PropertyMetadata;
use crate::{FerroPropertyMetadata, StyledPropertyOptions, WeakRef};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operation {
    Clone,
    Equals,
    Coerce,
    Drop,
    Notify,
}

impl Operation {
    const ALL: [Self; 5] = [Self::Clone, Self::Equals, Self::Coerce, Self::Drop, Self::Notify];
}

#[derive(Clone, Copy, Debug)]
enum Action {
    Panic,
    Read,
    Write,
    Priorities,
}

#[derive(Clone, Copy, Debug)]
struct Injection {
    operation: Operation,
    occurrence: usize,
    action: Action,
}

struct Trace {
    active: Cell<bool>,
    next_value: Cell<usize>,
    counts: [Cell<usize>; 5],
    events: RefCell<Vec<String>>,
    injection: Cell<Option<Injection>>,
    fired: Cell<bool>,
    owner: WeakRef<FerroObject>,
    property: Cell<Option<&'static StyledProperty<Tracked>>>,
}

impl Trace {
    fn value(self: &Rc<Self>, number: i32) -> Tracked {
        let serial = self.next_value.get();
        self.next_value.set(serial + 1);
        Tracked { number, serial, trace: self.clone() }
    }

    fn record(self: &Rc<Self>, operation: Operation, detail: String) {
        if !self.active.get() {
            return;
        }
        self.events.borrow_mut().push(format!("{operation:?}:{detail}"));
        let count = &self.counts[operation as usize];
        count.set(count.get() + 1);
        let Some(injection) = self.injection.get() else { return };
        if injection.operation != operation || injection.occurrence != count.get() {
            return;
        }
        // Disarm before invoking user code, including before a Drop panic:
        // unwinding must never encounter a second injected destructor panic.
        self.injection.set(None);
        self.fired.set(true);
        self.events.borrow_mut().push(format!("inject:{:?}", injection.action));
        if let Action::Panic = injection.action {
            panic!("injected coercion operation panic");
        }
        let owner = self.owner.upgrade().expect("owner is alive during recoercion");
        let property = self.property.get().unwrap();
        match injection.action {
            Action::Read => drop(owner.get_value(property)),
            Action::Write => owner.set_value(property, self.value(90)),
            Action::Priorities => {
                let effective = owner.values().get_effective_value(property).unwrap();
                let effective = cast_effective_value::<Tracked>(&*effective);
                effective.state.priority.set(BindingPriority::LocalValue);
                effective.state.base_priority.set(BindingPriority::LocalValue);
            }
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

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    panicked: bool,
    fired: bool,
    counts: [usize; 5],
    events: Vec<String>,
    values: (i32, Option<i32>, i32, i32),
    priorities: (BindingPriority, BindingPriority),
}

fn unique_name() -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!("DemandRootedCoercion{}", NEXT.fetch_add(1, Ordering::Relaxed))
}

fn run(original: bool, same_priority: bool, matches: (bool, bool), injection: Option<Injection>) -> Outcome {
    let owner = FerroObject::new();
    let trace = Rc::new(Trace {
        active: Cell::new(false),
        next_value: Cell::new(0),
        counts: std::array::from_fn(|_| Cell::new(0)),
        events: RefCell::new(Vec::new()),
        injection: Cell::new(injection),
        fired: Cell::new(false),
        owner: owner.downgrade(),
        property: Cell::new(None),
    });
    let coercion_trace = trace.clone();
    let notifying_trace = trace.clone();
    let property = FerroProperty::register_with::<FerroObject, _>(
        &unique_name(),
        StyledPropertyOptions::new(trace.value(1))
            .coerce(move |_, mut value: Tracked| {
                coercion_trace.record(Operation::Coerce, format!("{}={}", value.serial, value.number));
                value.number += 1;
                value
            })
            .notifying(move |_, before| notifying_trace.record(Operation::Notify, before.to_string())),
    );
    trace.property.set(Some(property));
    owner.set_value(property, trace.value(10));
    let effective = owner.values().get_effective_value(property).unwrap();
    let typed = cast_effective_value::<Tracked>(&*effective);
    let uncommon = typed.uncommon.as_ref().unwrap();
    // The real store owns the entry. Configure both animation/base branches
    // directly, without adding observer handles that could mask a drop.
    typed.value.replace(trace.value(if matches.0 { 14 } else { 10 }));
    typed.base_value.replace(Some(trace.value(if matches.1 { if same_priority { 23 } else { 24 } } else { 20 })));
    uncommon.uncoerced_value.replace(trace.value(13));
    uncommon.uncoerced_base_value.replace(trace.value(23));
    typed.state.priority.set(if same_priority { BindingPriority::LocalValue } else { BindingPriority::Animation });
    typed.state.base_priority.set(BindingPriority::LocalValue);
    trace.active.set(true);
    let panicked = catch_unwind(AssertUnwindSafe(|| {
        if original {
            typed.original_coerce_value(&owner);
        } else {
            EffectiveValueDyn::coerce_value(typed, &owner);
        }
    })).is_err();
    // Reads of fields make no additional value clones or equality calls.
    let values = (
        typed.value.borrow().number,
        typed.base_value.borrow().as_ref().map(|value| value.number),
        uncommon.uncoerced_value.borrow().number,
        uncommon.uncoerced_base_value.borrow().number,
    );
    trace.active.set(false);
    let events = trace.events.borrow().clone();
    Outcome {
        panicked,
        fired: trace.fired.get(),
        counts: std::array::from_fn(|index| trace.counts[index].get()),
        events,
        values,
        priorities: (typed.state.priority.get(), typed.state.base_priority.get()),
    }
}

#[test]
fn recoercion_matches_original_owned_value_frames_and_unwind() {
    for same_priority in [false, true] {
        for matches in [(false, false), (false, true), (true, false), (true, true)] {
            let baseline = run(true, same_priority, matches, None);
            assert!(!baseline.panicked);
            assert_eq!(run(false, same_priority, matches, None), baseline);
            for operation in Operation::ALL {
                for occurrence in 1..=baseline.counts[operation as usize] {
                    let injection = Some(Injection { operation, occurrence, action: Action::Panic });
                    let expected = run(true, same_priority, matches, injection);
                    assert!(expected.panicked && expected.fired, "{injection:?}");
                    assert_eq!(run(false, same_priority, matches, injection), expected,
                               "same_priority={same_priority}, matches={matches:?}, {injection:?}");
                }
            }
        }
    }
}

#[test]
fn recoercion_matches_original_reentrant_reads_writes_and_priority_snapshots() {
    let baseline = run(true, false, (false, false), None);
    let mut injections = vec![
        Injection { operation: Operation::Clone, occurrence: 1, action: Action::Read },
        Injection { operation: Operation::Clone, occurrence: 2, action: Action::Priorities },
        Injection { operation: Operation::Clone, occurrence: 3, action: Action::Write },
        Injection { operation: Operation::Equals, occurrence: 1, action: Action::Read },
        Injection { operation: Operation::Coerce, occurrence: 1, action: Action::Write },
        Injection { operation: Operation::Coerce, occurrence: 2, action: Action::Write },
        Injection { operation: Operation::Notify, occurrence: 1, action: Action::Write },
    ];
    injections.extend((1..=baseline.counts[Operation::Drop as usize]).map(|occurrence| {
        Injection { operation: Operation::Drop, occurrence, action: Action::Read }
    }));
    for injection in injections {
        let expected = run(true, false, (false, false), Some(injection));
        assert!(expected.fired, "{injection:?}");
        assert_eq!(run(false, false, (false, false), Some(injection)), expected, "{injection:?}");
    }
}

struct CloneOnly {
    events: Rc<RefCell<Vec<&'static str>>>,
}

impl Clone for CloneOnly {
    fn clone(&self) -> Self {
        self.events.borrow_mut().push("default clone");
        Self { events: self.events.clone() }
    }
}

impl Drop for CloneOnly {
    fn drop(&mut self) {
        self.events.borrow_mut().push("default drop");
    }
}

struct CallbackLife {
    name: &'static str,
    events: Rc<RefCell<Vec<&'static str>>>,
    panic_on_drop: bool,
}

impl Drop for CallbackLife {
    fn drop(&mut self) {
        self.events.borrow_mut().push(self.name);
        if self.panic_on_drop {
            panic!("callback destructor panic");
        }
    }
}

fn callback(life: CallbackLife) -> impl Fn(&FerroObject, CloneOnly) -> CloneOnly {
    move |_, value| {
        let _keep_alive = &life;
        value
    }
}

#[test]
fn metadata_coercion_keeps_clone_only_api_and_callback_pair_lifetimes() {
    // Naming metadata does not require Clone; its existing constructor and
    // coercion methods require Clone, but must still accept a non-PartialEq T.
    struct NeitherCloneNorEqual;
    let _: Option<StyledPropertyMetadata<NeitherCloneNorEqual>> = None;
    let events = Rc::new(RefCell::new(Vec::new()));
    let plain = StyledPropertyMetadata::<CloneOnly>::new(None);
    assert!(plain.coerce_value().is_none() && plain.coercion().is_none());
    let metadata = StyledPropertyMetadata::new(Some(CloneOnly { events: events.clone() }))
        .with_coerce(callback(CallbackLife { name: "first callback drop", events: events.clone(), panic_on_drop: false }));
    let cloned = metadata.clone();
    let mut inherited = StyledPropertyMetadata::new(None);
    inherited.merge(&metadata);
    assert!(Rc::ptr_eq(metadata.coerce_value().unwrap(), cloned.coerce_value().unwrap()));
    assert!(Rc::ptr_eq(metadata.coerce_value().unwrap(), inherited.coerce_value().unwrap()));
    let replaced = cloned.with_coerce(callback(CallbackLife {
        name: "second callback drop", events: events.clone(), panic_on_drop: false,
    }));
    let metadata = Rc::new(metadata);
    let type_safe = StyledPropertyMetadata::generate_type_safe_metadata(&metadata);
    assert!(type_safe.coerce_value().is_none() && type_safe.coercion().is_none());
    assert_eq!(&*events.borrow(), &["default clone", "default clone", "default clone"]);
    events.borrow_mut().clear();
    drop(replaced);
    assert_eq!(&*events.borrow(), &["default drop", "second callback drop"]);
    drop(inherited);
    drop(type_safe);
    drop(metadata);
    assert_eq!(&*events.borrow(), &[
        "default drop", "second callback drop", "default drop", "default drop",
        "default drop", "first callback drop",
    ]);
}

// The original metadata field order and replacement expression. The new pair
// must preserve callback/default cleanup even if replacing the old Rc panics.
struct OriginalMetadata<T> {
    _base: FerroPropertyMetadata,
    _default_value: Option<T>,
    coerce: Option<CoerceValueCallback<T>>,
}

impl<T: Clone + 'static> OriginalMetadata<T> {
    fn with_coerce(self, coerce: impl Fn(&FerroObject, T) -> T + 'static) -> Self {
        self.with_coerce_callback(Rc::new(coerce))
    }

    fn with_coerce_callback(mut self, coerce: CoerceValueCallback<T>) -> Self {
        self.coerce = Some(coerce);
        self
    }
}

#[test]
fn metadata_replacement_matches_original_callback_drop_unwind() {
    fn events(original: bool, panic_on_drop: bool) -> (bool, Vec<&'static str>) {
        let events = Rc::new(RefCell::new(Vec::new()));
        let panicked = catch_unwind(AssertUnwindSafe(|| {
            let default = Some(CloneOnly { events: events.clone() });
            let first = callback(CallbackLife { name: "first callback drop", events: events.clone(), panic_on_drop });
            let second = callback(CallbackLife { name: "second callback drop", events: events.clone(), panic_on_drop: false });
            if original {
                drop(OriginalMetadata {
                    _base: FerroPropertyMetadata::new(crate::data::BindingMode::Default, Some(false)),
                    _default_value: default,
                    coerce: None,
                }.with_coerce(first).with_coerce(second));
            } else {
                drop(StyledPropertyMetadata::new(default).with_coerce(first).with_coerce(second));
            }
        })).is_err();
        let observed = events.borrow().clone();
        (panicked, observed)
    }
    for panic_on_drop in [false, true] {
        let original = events(true, panic_on_drop);
        assert_eq!(original.0, panic_on_drop);
        assert_eq!(events(false, panic_on_drop), original);
    }
}

#[test]
fn recoercion_keeps_nan_equality() {
    for original in [false, true] {
        let owner = FerroObject::new();
        let notifications = Rc::new(Cell::new(0));
        let observed = notifications.clone();
        let property = FerroProperty::register_with::<FerroObject, _>(
            &unique_name(),
            StyledPropertyOptions::new(f64::NAN).coerce(|_, value| value)
                .notifying(move |_, _| observed.set(observed.get() + 1)),
        );
        owner.set_value(property, f64::NAN);
        let effective = owner.values().get_effective_value(property).unwrap();
        let typed = cast_effective_value::<f64>(&*effective);
        notifications.set(0);
        if original {
            typed.original_coerce_value(&owner);
        } else {
            EffectiveValueDyn::coerce_value(typed, &owner);
        }
        assert!(owner.get_value(property).is_nan());
        assert_eq!(notifications.get(), 0);
    }
}

// Copied from 4622c95a: only the two oracle method names and their call differ.
impl<T: PropertyValue> EffectiveValue<T> {
    fn original_coerce_value(&self, owner: &FerroObject) {
        let Some(uncommon) = &self.uncommon else { return };
        let value = uncommon.uncoerced_value.borrow().clone();
        let base_value = uncommon.uncoerced_base_value.borrow().clone();
        self.original_set_and_raise_core_with_base(owner, value, self.state.priority.get(), base_value, self.state.base_priority.get());
    }

    fn original_set_and_raise_core_with_base(
        &self,
        owner: &FerroObject,
        value: T,
        priority: BindingPriority,
        base_value: T,
        base_priority: BindingPriority,
    ) {
        debug_assert!(base_priority > BindingPriority::Animation);
        debug_assert!(priority <= base_priority);

        let old_value = self.value();
        let mut value_changed = false;
        let mut base_value_changed = false;
        let mut v = value.clone();
        let mut bv = base_value.clone();

        if let Some(uncommon) = &self.uncommon {
            v = (uncommon.coerce)(owner, value.clone());
            if priority != base_priority {
                bv = (uncommon.coerce)(owner, base_value.clone());
            }
        }

        if !value_equals(&*self.value.borrow(), &v) {
            self.value.replace(v.clone());
            value_changed = true;
            if let Some(uncommon) = &self.uncommon {
                uncommon.uncoerced_value.replace(value);
            }
        }

        if !self.base_value.borrow().as_ref().is_some_and(|base| value_equals(base, &bv)) {
            self.base_value.replace(Some(v));
            base_value_changed = true;
            if let Some(uncommon) = &self.uncommon {
                uncommon.uncoerced_value.replace(base_value);
            }
        }

        self.state.priority.set(priority);
        self.state.base_priority.set(base_priority);

        if value_changed {
            self.notify_value_changed(owner, old_value);
        }
        if base_value_changed {
            self.notify_base_value_changed(owner);
        }
    }
}

mod end_reevaluation_tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct EndOutcome {
        observed: Outcome,
        coerced_default: bool,
    }

    fn run_end(original: bool, delta: i32, priority: BindingPriority, injection: Option<Injection>) -> EndOutcome {
        let owner = FerroObject::new();
        let trace = Rc::new(Trace {
            active: Cell::new(false), next_value: Cell::new(0),
            counts: std::array::from_fn(|_| Cell::new(0)), events: RefCell::new(Vec::new()),
            injection: Cell::new(injection), fired: Cell::new(false),
            owner: owner.downgrade(), property: Cell::new(None),
        });
        let (coercion_trace, notifying_trace) = (trace.clone(), trace.clone());
        let property = FerroProperty::register_with::<FerroObject, _>(
            &unique_name(),
            StyledPropertyOptions::new(trace.value(1))
                .coerce(move |_, mut value: Tracked| {
                    coercion_trace.record(Operation::Coerce, format!("{}={}", value.serial, value.number));
                    value.number += delta;
                    value
                })
                .notifying(move |_, before| notifying_trace.record(Operation::Notify, before.to_string())),
        );
        trace.property.set(Some(property));
        owner.set_value(property, trace.value(10));
        let effective = owner.values().get_effective_value(property).unwrap();
        let typed = cast_effective_value::<Tracked>(&*effective);
        typed.state.priority.set(priority);
        typed.state.base_priority.set(priority);
        trace.active.set(true);
        let panicked = catch_unwind(AssertUnwindSafe(|| {
            if original {
                typed.original_end_reevaluation(&owner);
            } else {
                EffectiveValueDyn::end_reevaluation(typed, &owner);
            }
        })).is_err();
        let uncommon = typed.uncommon.as_ref().unwrap();
        let values = (
            typed.value.borrow().number,
            typed.base_value.borrow().as_ref().map(|value| value.number),
            uncommon.uncoerced_value.borrow().number,
            uncommon.uncoerced_base_value.borrow().number,
        );
        trace.active.set(false);
        let events = trace.events.borrow().clone();
        EndOutcome {
            observed: Outcome {
                panicked, fired: trace.fired.get(), events, values,
                counts: std::array::from_fn(|index| trace.counts[index].get()),
                priorities: (typed.state.priority.get(), typed.state.base_priority.get()),
            },
            coerced_default: typed.state.is_coerced_default_value.get(),
        }
    }

    #[test]
    fn end_reevaluation_matches_original_owned_values_reentry_and_unwind() {
        for delta in [0, 1] {
            for priority in [BindingPriority::Unset, BindingPriority::LocalValue] {
                let baseline = run_end(true, delta, priority, None);
                assert!(!baseline.observed.panicked);
                assert_eq!(run_end(false, delta, priority, None), baseline);
                if priority != BindingPriority::Unset {
                    assert_eq!(baseline.observed.counts, [0; 5]);
                }
                for operation in Operation::ALL {
                    for occurrence in 1..=baseline.observed.counts[operation as usize] {
                        let injection = Some(Injection { operation, occurrence, action: Action::Panic });
                        let expected = run_end(true, delta, priority, injection);
                        assert!(expected.observed.panicked && expected.observed.fired, "{injection:?}");
                        assert_eq!(run_end(false, delta, priority, injection), expected,
                            "delta={delta}, priority={priority:?}, {injection:?}");
                    }
                }
            }
        }
        let priority = BindingPriority::Unset;
        let baseline = run_end(true, 1, priority, None);
        let mut injections = vec![
            Injection { operation: Operation::Clone, occurrence: 1, action: Action::Read },
            Injection { operation: Operation::Clone, occurrence: 2, action: Action::Write },
            Injection { operation: Operation::Equals, occurrence: 1, action: Action::Priorities },
            Injection { operation: Operation::Coerce, occurrence: 1, action: Action::Write },
            Injection { operation: Operation::Notify, occurrence: 1, action: Action::Write },
        ];
        injections.extend((1..=baseline.observed.counts[Operation::Drop as usize]).map(|occurrence| {
            Injection { operation: Operation::Drop, occurrence, action: Action::Read }
        }));
        for injection in injections {
            let expected = run_end(true, 1, priority, Some(injection));
            assert!(expected.observed.fired, "{injection:?}");
            assert_eq!(run_end(false, 1, priority, Some(injection)), expected, "{injection:?}");
        }
    }

    #[test]
    fn end_reevaluation_keeps_nan_and_the_retained_uncommon_callback() {
        for original in [false, true] {
            let owner = FerroObject::new();
            let property = FerroProperty::register_with::<FerroObject, _>(
                &unique_name(), StyledPropertyOptions::new(f64::NAN).coerce(|_, value| value),
            );
            let effective = EffectiveValue::new(&owner, property, None);
            if original { effective.original_end_reevaluation(&owner); }
            else { EffectiveValueDyn::end_reevaluation(&effective, &owner); }
            assert!(effective.value().is_nan());
            assert!(!effective.state.is_coerced_default_value.get());

            let property = FerroProperty::register_with::<FerroObject, _>(
                &unique_name(), StyledPropertyOptions::new(1_i32).coerce(|_, _| panic!("metadata callback used")),
            );
            let mut effective = EffectiveValue::new(&owner, property, None);
            // The algorithm must still call the callback retained in uncommon,
            // even though metadata supplies the demand-rooted dispatch entry.
            effective.uncommon.as_mut().unwrap().coerce = Rc::new(|_, value| value + 5);
            if original { effective.original_end_reevaluation(&owner); }
            else { EffectiveValueDyn::end_reevaluation(&effective, &owner); }
            assert_eq!(effective.value(), 6);
        }
    }

    // Copied from bbff5971; only the oracle method name differs.
    impl<T: PropertyValue> EffectiveValue<T> {
        fn original_end_reevaluation(&self, owner: &FerroObject) {
            if self.state.priority.get() == BindingPriority::Unset {
                if let Some(uncommon) = &self.uncommon {
                    // Coerce the default value and raise if it differs.
                    let default = self.metadata.default_value().clone();
                    let coerced = (uncommon.coerce)(owner, default.clone());
                    if !value_equals(&default, &coerced) {
                        self.set_coerced_default_value_and_raise(owner, coerced);
                    }
                }
            }
        }
    }
}
