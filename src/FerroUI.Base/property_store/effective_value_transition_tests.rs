//! Compare original owned-value scopes with the borrowed transition engine.

use super::*;
use crate::{ObjectType, StyledPropertyOptions, WeakRef};
use std::mem::{align_of, needs_drop, size_of};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operation {
    Clone,
    Equals,
    Coerce,
    Drop,
    Notifying,
    Changed,
    Inherited,
}

impl Operation {
    const ALL: [Self; 7] = [Self::Clone, Self::Equals, Self::Coerce, Self::Drop, Self::Notifying, Self::Changed, Self::Inherited];
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

#[derive(Clone, Copy, Debug)]
enum Mode {
    Write,
    Notify,
}

#[derive(Clone, Copy, Debug)]
struct Case {
    priority: BindingPriority,
    current_priority: BindingPriority,
    base_priority: BindingPriority,
    current: i32,
    base: Option<i32>,
    input: i32,
    coerce: bool,
    inherits: bool,
    overridden: bool,
    coerced_default: bool,
}

impl Default for Case {
    fn default() -> Self {
        Self {
            priority: BindingPriority::LocalValue,
            current_priority: BindingPriority::Style,
            base_priority: BindingPriority::Style,
            current: 10,
            base: Some(20),
            input: 30,
            coerce: false,
            inherits: false,
            overridden: false,
            coerced_default: false,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct State {
    current: Option<i32>,
    base: Option<Option<i32>>,
    uncoerced: Option<(Option<i32>, Option<i32>)>,
    priority: BindingPriority,
    base_priority: BindingPriority,
    overridden: bool,
    coerced_default: bool,
}

fn state(value: &EffectiveValue<Tracked>) -> State {
    State {
        current: value.value.try_borrow().ok().map(|v| v.number),
        base: value.base_value.try_borrow().ok().map(|v| v.as_ref().map(|v| v.number)),
        uncoerced: value.uncommon.as_ref().map(|v| (
            v.uncoerced_value.try_borrow().ok().map(|v| v.number),
            v.uncoerced_base_value.try_borrow().ok().map(|v| v.number),
        )),
        priority: value.state.priority.get(),
        base_priority: value.state.base_priority.get(),
        overridden: value.state.is_overriden_current_value.get(),
        coerced_default: value.state.is_coerced_default_value.get(),
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Event {
    operation: Operation,
    detail: String,
    state: State,
}

struct Trace {
    active: Cell<bool>,
    original: bool,
    next_serial: Cell<usize>,
    counts: [Cell<usize>; 7],
    injection: Cell<Option<Injection>>,
    fired: Cell<bool>,
    events: RefCell<Vec<Event>>,
    owner: WeakRef<FerroObject>,
    property: Cell<Option<&'static StyledProperty<Tracked>>>,
}

impl Trace {
    fn value(self: &Rc<Self>, number: i32) -> Tracked {
        let serial = self.next_serial.get();
        self.next_serial.set(serial + 1);
        Tracked { number, serial, trace: self.clone() }
    }

    fn record(self: &Rc<Self>, operation: Operation, detail: String) {
        if !self.active.get() {
            return;
        }
        let owner = self.owner.upgrade().expect("test owner is alive");
        let property = self.property.get().unwrap();
        let effective = owner.values().get_effective_value(property).unwrap();
        let typed = cast_effective_value::<Tracked>(&*effective);
        self.events.borrow_mut().push(Event { operation, detail, state: state(typed) });
        let count = &self.counts[operation as usize];
        count.set(count.get() + 1);
        let Some(injection) = self.injection.get() else { return };
        if injection.operation != operation || injection.occurrence != count.get() {
            return;
        }
        // Disarm before user reentry or an injected destructor panic. A
        // single failure must not become a double panic during cleanup.
        self.injection.set(None);
        self.fired.set(true);
        match injection.action {
            Action::Panic => panic!("injected borrowed transition operation"),
            Action::Read => drop(owner.get_value(property)),
            Action::Write => {
                let value = self.value(90);
                if self.original {
                    typed.original_set_for_transition(&owner, value, BindingPriority::LocalValue, false, false);
                } else {
                    typed.set_and_raise_core(&owner, value, BindingPriority::LocalValue, false, false);
                }
            }
            Action::Priorities => {
                typed.state.priority.set(BindingPriority::Animation);
                typed.state.base_priority.set(BindingPriority::LocalValue);
            }
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
    counts: [usize; 7],
    events: Vec<Event>,
    state: State,
}

fn unique_name() -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!("BorrowedTransition{}", NEXT.fetch_add(1, Ordering::Relaxed))
}

fn run(original: bool, mode: Mode, case: Case, injection: Option<Injection>) -> Outcome {
    let owner = FerroObject::new();
    let child = FerroObject::new();
    let trace = Rc::new(Trace {
        active: Cell::new(false),
        original,
        next_serial: Cell::new(0),
        counts: std::array::from_fn(|_| Cell::new(0)),
        injection: Cell::new(injection),
        fired: Cell::new(false),
        events: RefCell::new(Vec::new()),
        owner: owner.downgrade(),
        property: Cell::new(None),
    });
    let notify_trace = trace.clone();
    let options = StyledPropertyOptions::new(trace.value(1))
        .inherits(case.inherits)
        .notifying(move |sender, before| {
            // Registered inheritable properties outlive each test owner. An
            // unrelated later inheritance traversal must not upgrade it.
            if !notify_trace.active.get() {
                return;
            }
            let owner = notify_trace.owner.upgrade().unwrap();
            let parent = std::ptr::eq(sender, &*owner);
            notify_trace.record(Operation::Notifying, format!("parent={parent},before={before}"));
        });
    let options = if case.coerce {
        let coerce_trace = trace.clone();
        options.coerce(move |_, mut value: Tracked| {
            coerce_trace.record(Operation::Coerce, format!("{}={}", value.serial, value.number));
            value.number += 1;
            value
        })
    } else {
        options
    };
    let property = FerroProperty::register_with::<FerroObject, _>(&unique_name(), options);
    trace.property.set(Some(property));
    owner.set_value(property, trace.value(case.current));
    if case.inherits {
        child.set_inheritance_parent(&owner);
    }
    let effective = owner.values().get_effective_value(property).unwrap();
    let typed = cast_effective_value::<Tracked>(&*effective);
    typed.value.replace(trace.value(case.current));
    typed.base_value.replace(case.base.map(|number| trace.value(number)));
    if let Some(uncommon) = &typed.uncommon {
        uncommon.uncoerced_value.replace(trace.value(11));
        uncommon.uncoerced_base_value.replace(trace.value(21));
    }
    typed.state.priority.set(case.current_priority);
    typed.state.base_priority.set(case.base_priority);
    typed.state.is_overriden_current_value.set(false);
    typed.state.is_coerced_default_value.set(false);
    let changed_trace = trace.clone();
    owner.add_property_changed_handler(Rc::new(move |event| {
        if std::ptr::eq(event.property(), &**property) {
            let old = event.old_value().map(|value| value.downcast_ref::<Tracked>().unwrap());
            let new = event.new_value().downcast_ref::<Tracked>().unwrap();
            changed_trace.record(Operation::Changed, format!("{:?} -> {}={}, {:?}", old.map(|value| (value.serial, value.number)), new.serial, new.number, event.priority()));
        }
    }));
    let inherited_trace = trace.clone();
    child.add_property_changed_handler(Rc::new(move |event| {
        if std::ptr::eq(event.property(), &**property) {
            let old = event.old_value().map(|value| value.downcast_ref::<Tracked>().unwrap());
            let new = event.new_value().downcast_ref::<Tracked>().unwrap();
            inherited_trace.record(Operation::Inherited, format!("{:?} -> {}={}, inherited={}", old.map(|value| (value.serial, value.number)), new.serial, new.number, event.is_inherited_value_change()));
        }
    }));
    let input = trace.value(case.input);
    trace.active.set(true);
    let panicked = catch_unwind(AssertUnwindSafe(|| {
        match (original, mode) {
            (true, Mode::Write) => typed.original_set_for_transition(&owner, input, case.priority, case.overridden, case.coerced_default),
            (false, Mode::Write) => typed.set_and_raise_core(&owner, input, case.priority, case.overridden, case.coerced_default),
            (true, Mode::Notify) => typed.original_notify_for_transition(&owner, input),
            (false, Mode::Notify) => typed.notify_value_changed(&owner, input),
        }
    })).is_err();
    let after = state(typed);
    trace.active.set(false);
    Outcome {
        panicked,
        fired: trace.fired.get(),
        counts: std::array::from_fn(|i| trace.counts[i].get()),
        events: trace.events.take(),
        state: after,
    }
}

fn compare_panics(mode: Mode, case: Case) -> Outcome {
    let expected = run(true, mode, case, None);
    assert!(!expected.panicked && !expected.fired);
    assert_eq!(run(false, mode, case, None), expected, "{mode:?}, {case:?}");
    for operation in Operation::ALL {
        for occurrence in 1..=expected.counts[operation as usize] {
            let injection = Some(Injection { operation, occurrence, action: Action::Panic });
            let original = run(true, mode, case, injection);
            assert!(original.panicked && original.fired, "{mode:?}, {operation:?}#{occurrence}");
            assert_eq!(run(false, mode, case, injection), original, "{mode:?}, {case:?}, {operation:?}#{occurrence}");
        }
    }
    expected
}

#[test]
fn shared_write_matches_original_operations_and_unwind() {
    let cases = [
        Case::default(),
        Case { current: 30, ..Case::default() },
        Case { current: 30, base: Some(30), ..Case::default() },
        Case { priority: BindingPriority::Animation, ..Case::default() },
        Case { current_priority: BindingPriority::Animation, base_priority: BindingPriority::Unset, priority: BindingPriority::Template, ..Case::default() },
        Case { current_priority: BindingPriority::Animation, base_priority: BindingPriority::LocalValue, priority: BindingPriority::Style, ..Case::default() },
        Case { coerce: true, ..Case::default() },
        Case { coerce: true, coerced_default: true, ..Case::default() },
        Case { priority: BindingPriority::Unset, current_priority: BindingPriority::Unset, base_priority: BindingPriority::Unset, overridden: true, ..Case::default() },
        Case { coerce: true, inherits: true, ..Case::default() },
    ];
    for case in cases {
        let observed = compare_panics(Mode::Write, case);
        assert!(observed.counts[Operation::Clone as usize] >= 1, "the old-value clone is observed");
        assert_eq!(observed.state.overridden, case.overridden);
        assert_eq!(observed.state.coerced_default, case.coerced_default);
    }
}

#[test]
fn shared_write_keeps_live_priority_reads_under_reentry() {
    let case = Case { priority: BindingPriority::StyleTrigger, coerce: true, ..Case::default() };
    for (operation, occurrence, action) in [
        (Operation::Coerce, 1, Action::Priorities),
        (Operation::Drop, 1, Action::Priorities),
        (Operation::Clone, 2, Action::Read),
        (Operation::Equals, 1, Action::Write),
        (Operation::Notifying, 1, Action::Write),
        (Operation::Changed, 1, Action::Write),
    ] {
        let injection = Some(Injection { operation, occurrence, action });
        let original = run(true, Mode::Write, case, injection);
        assert!(original.fired, "{operation:?}#{occurrence}");
        assert_eq!(run(false, Mode::Write, case, injection), original, "{operation:?}#{occurrence}, {action:?}");
        if matches!(action, Action::Priorities) {
            assert!(!original.panicked);
            assert_eq!(original.state.base, Some(case.base), "the base decision reloads the priority after user code");
        }
    }
}

#[test]
fn shared_notification_keeps_snapshot_lifetimes_and_unwind() {
    let case = Case { inherits: true, ..Case::default() };
    let observed = compare_panics(Mode::Notify, case);
    assert!(observed.counts[Operation::Inherited as usize] > 0);
    let final_notify = observed.events.iter().rposition(|event| event.operation == Operation::Notifying && event.detail == "parent=true,before=false").unwrap();
    assert!(observed.events[..final_notify].iter().any(|event| event.operation == Operation::Drop));
    assert!(observed.events[final_notify + 1..].iter().any(|event| event.operation == Operation::Drop));
    for (operation, occurrence, action) in [
        (Operation::Notifying, 1, Action::Write),
        (Operation::Changed, 1, Action::Write),
        (Operation::Inherited, 1, Action::Read),
        (Operation::Drop, 1, Action::Write),
    ] {
        let injection = Some(Injection { operation, occurrence, action });
        let original = run(true, Mode::Notify, case, injection);
        assert!(original.fired);
        assert_eq!(run(false, Mode::Notify, case, injection), original, "{operation:?}#{occurrence}, {action:?}");
    }
}

#[test]
fn borrowed_transition_frames_keep_storage_layout_and_nan_semantics() {
    struct DropOnly;
    impl Drop for DropOnly {
        fn drop(&mut self) { panic!("borrowed frames must not own a value"); }
    }
    #[allow(dead_code)]
    struct Original<T> {
        metadata: Rc<StyledPropertyMetadata<T>>,
        state: EffectiveValueState,
        value: RefCell<T>,
        base_value: RefCell<Option<T>>,
        uncommon: Option<UncommonFields<T>>,
    }
    assert_eq!(size_of::<Original<DropOnly>>(), size_of::<EffectiveValue<DropOnly>>());
    assert_eq!(align_of::<Original<DropOnly>>(), align_of::<EffectiveValue<DropOnly>>());
    assert_eq!(size_of::<Original<String>>(), size_of::<EffectiveValue<String>>());
    assert_eq!(align_of::<Original<String>>(), align_of::<EffectiveValue<String>>());
    assert!(!needs_drop::<transition::WriteFrame<'_, '_, DropOnly>>());
    assert!(!needs_drop::<transition::NotificationFrame<'_, '_, DropOnly>>());
    for input in [f64::NAN, 0.0, -0.0, 1.0] {
        let run = |original: bool| {
            let owner = FerroObject::new();
            let notifications = Rc::new(Cell::new(0));
            let observed = notifications.clone();
            let property = FerroProperty::register_with::<FerroObject, _>(
                &unique_name(),
                StyledPropertyOptions::new(f64::NAN).notifying(move |_, _| observed.set(observed.get() + 1)),
            );
            let effective = EffectiveValue::new(&owner, property, None);
            if original {
                effective.original_set_for_transition(&owner, input, BindingPriority::LocalValue, false, false);
            } else {
                effective.set_and_raise_core(&owner, input, BindingPriority::LocalValue, false, false);
            }
            let result = (effective.value.borrow().to_bits(), effective.base_value.borrow().map(f64::to_bits), notifications.get());
            result
        };
        let original = run(true);
        assert_eq!(run(false), original);
        assert_eq!(original.2, if input.is_nan() { 0 } else { 2 });
    }
}

// Exact original scopes; the setter selects the original notification oracle.
impl<T: PropertyValue> EffectiveValue<T> {
    fn original_set_for_transition(
        &self,
        owner: &FerroObject,
        value: T,
        priority: BindingPriority,
        is_overridden_current_value: bool,
        is_coerced_default_value: bool,
    ) {
        let old_value = self.value();
        let mut value_changed = false;
        let mut base_value_changed = false;

        self.state.is_overriden_current_value.set(is_overridden_current_value);
        self.state.is_coerced_default_value.set(is_coerced_default_value);

        // `v = value`, or the coerced value. The value as it was given is
        // only read again where it is stored uncoerced, so without a
        // coercion `v` is the value itself and not a copy of it.
        let (v, uncoerced) = match &self.uncommon {
            Some(uncommon) if !is_coerced_default_value => {
                ((uncommon.coerce)(owner, value.clone()), Some((uncommon, value)))
            }
            _ => (value, None),
        };

        if priority <= self.state.priority.get() {
            value_changed = !value_equals(&*self.value.borrow(), &v);
            // `replace` releases the borrow before the previous value is
            // dropped: dropping a value may run code that reads the property.
            self.value.replace(v.clone());
            self.state.priority.set(priority);
            if let Some((uncommon, value)) = &uncoerced {
                uncommon.uncoerced_value.replace(value.clone());
            }
        }

        if priority <= self.state.base_priority.get() && priority >= BindingPriority::LocalValue {
            base_value_changed = !self.base_value.borrow().as_ref().is_some_and(|base| value_equals(base, &v));
            self.base_value.replace(Some(v));
            self.state.base_priority.set(priority);
            if let Some((uncommon, value)) = uncoerced {
                uncommon.uncoerced_base_value.replace(value);
            }
        }

        if value_changed {
            self.original_notify_for_transition(owner, old_value);
        } else if base_value_changed {
            self.notify_base_value_changed(owner);
        }
    }

    fn original_notify_for_transition(&self, owner: &FerroObject, old_value: T) {
        let notifying = self.state.property.notifying();
        if let Some(notifying) = notifying {
            notifying(owner, true);
        }
        let new_value = self.value();
        owner.raise_property_changed(self.state.property, Some(&old_value), &new_value, self.state.priority.get(), true);
        if self.state.property.inherits() {
            // A change handler may have changed the value again: inheritance
            // children are told the value as it is now, not the value that
            // was announced to the handlers of this object.
            let new_value = self.value();
            owner.values().on_inherited_effective_value_changed(owner, self.state.property, &old_value, &new_value);
        }
        if let Some(notifying) = notifying {
            notifying(owner, false);
        }
    }
}
