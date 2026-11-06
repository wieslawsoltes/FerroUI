//! Private constructor oracles used by the effective-value construction tests.
//! These helpers add no test names to the existing `metadata::` filter.

use super::*;
use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Weak;

static HOST: TypeInfo = TypeInfo::new("CoercionConstructionHost", None);

impl<M: PropertyMetadata> MetadataTable<M> {
    // The original constructor, including the metadata owner and field order.
    fn original_new_for_construction(host_type: &'static TypeInfo, mut metadata: M) -> Self {
        metadata.freeze();
        let metadata = Rc::new(metadata);
        let default_metadata = M::generate_type_safe_metadata(&metadata);
        Self {
            default_metadata,
            inner: RefCell::new(MetadataEntries::Single(Some((host_type, metadata)))),
        }
    }
}

impl<T: Clone + 'static> StyledPropertyMetadata<T> {
    // The original public generator. In particular, unfrozen metadata with
    // no coercer still needs a new frozen copy instead of an Rc clone.
    fn original_generate_for_construction(this: &Rc<Self>) -> Rc<Self> {
        if this.is_read_only() && this.coerce.is_none() {
            return this.clone();
        }
        let mut copy = StyledPropertyMetadata::new(this.default_value.clone())
            .with_default_binding_mode(this.default_binding_mode())
            .with_enable_data_validation(this.enable_data_validation().unwrap_or(false));
        copy.freeze();
        Rc::new(copy)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Clone,
    ValueDrop,
    CallbackDrop,
    Reentry,
}

const INJECTABLE: [Kind; 3] = [Kind::Clone, Kind::ValueDrop, Kind::CallbackDrop];
type Failure = Option<(Kind, usize)>;

#[derive(Debug, PartialEq, Eq)]
struct Event {
    kind: Kind,
    serial: usize,
    callback_owners: usize,
    metadata_owners: usize,
}

struct Trace {
    active: Cell<bool>,
    next_serial: Cell<usize>,
    counts: [Cell<usize>; 4],
    failure: Cell<Failure>,
    fired: Cell<bool>,
    events: RefCell<Vec<Event>>,
    callback: RefCell<Option<Weak<dyn Fn(&FerroObject, Tracked) -> Tracked>>>,
    metadata: RefCell<Weak<StyledPropertyMetadata<Tracked>>>,
    on_clone: RefCell<Option<Box<dyn FnOnce()>>>,
}

impl Trace {
    fn new(failure: Failure) -> Rc<Self> {
        Rc::new(Self {
            active: Cell::new(false),
            next_serial: Cell::new(0),
            counts: std::array::from_fn(|_| Cell::new(0)),
            failure: Cell::new(failure),
            fired: Cell::new(false),
            events: RefCell::new(Vec::new()),
            callback: RefCell::new(None),
            metadata: RefCell::new(Weak::new()),
            on_clone: RefCell::new(None),
        })
    }

    fn value(self: &Rc<Self>, number: i32) -> Tracked {
        let serial = self.next_serial.get();
        self.next_serial.set(serial + 1);
        Tracked { number, serial, trace: self.clone() }
    }

    fn record(&self, kind: Kind, serial: usize) {
        if !self.active.get() {
            return;
        }
        self.events.borrow_mut().push(Event {
            kind,
            serial,
            callback_owners: self.callback.borrow().as_ref().map_or(0, Weak::strong_count),
            metadata_owners: self.metadata.borrow().strong_count(),
        });
        let count = &self.counts[kind as usize];
        count.set(count.get() + 1);
        if self.failure.get() == Some((kind, count.get())) {
            // A destructor failure must not cause another injected panic
            // while Rust is cleaning up the remaining initialized fields.
            self.failure.set(None);
            self.fired.set(true);
            panic!("injected metadata construction operation");
        }
        if kind == Kind::Clone {
            let hook = self.on_clone.borrow_mut().take();
            if let Some(hook) = hook {
                hook();
            }
        }
    }
}

// Deliberately no PartialEq: the public metadata API only requires Clone.
struct Tracked {
    number: i32,
    serial: usize,
    trace: Rc<Trace>,
}

impl Clone for Tracked {
    fn clone(&self) -> Self {
        self.trace.record(Kind::Clone, self.serial);
        self.trace.value(self.number)
    }
}

impl Drop for Tracked {
    fn drop(&mut self) {
        self.trace.record(Kind::ValueDrop, self.serial);
    }
}

struct CallbackDrop(Rc<Trace>);

impl Drop for CallbackDrop {
    fn drop(&mut self) {
        self.0.record(Kind::CallbackDrop, 0);
    }
}

fn metadata(trace: &Rc<Trace>, has_default: bool, has_coercer: bool) -> StyledPropertyMetadata<Tracked> {
    let metadata = StyledPropertyMetadata::new(has_default.then(|| trace.value(7)))
        .with_default_binding_mode(BindingMode::TwoWay)
        .with_enable_data_validation(true);
    if !has_coercer {
        return metadata;
    }
    let retained = CallbackDrop(trace.clone());
    let callback: CoerceValueCallback<Tracked> = Rc::new(move |_, value| {
        let _retained = &retained;
        value
    });
    *trace.callback.borrow_mut() = Some(Rc::downgrade(&callback));
    metadata.with_registered_coercion(Coercion::new(callback))
}

#[derive(Debug, PartialEq, Eq)]
struct Snapshot {
    shares_input: bool,
    input_frozen: bool,
    output_frozen: bool,
    input_has_coercer: bool,
    output_has_coercer: bool,
    output_number: Option<i32>,
    mode: BindingMode,
    validation: Option<bool>,
}

fn snapshot(input: &Rc<StyledPropertyMetadata<Tracked>>, output: &Rc<StyledPropertyMetadata<Tracked>>) -> Snapshot {
    Snapshot {
        shares_input: Rc::ptr_eq(input, output),
        input_frozen: input.is_read_only(),
        output_frozen: output.is_read_only(),
        input_has_coercer: input.coerce_value().is_some(),
        output_has_coercer: output.coerce_value().is_some(),
        output_number: output.default_value.as_ref().map(|v| v.number),
        mode: output.default_binding_mode(),
        validation: output.enable_data_validation(),
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Observation {
    panicked: bool,
    fired: bool,
    events: Vec<Event>,
    snapshots: Vec<Snapshot>,
}

fn finish(trace: &Trace, panicked: bool, snapshots: Vec<Snapshot>) -> Observation {
    trace.active.set(false);
    // A Clone failure may occur before the one-shot reentry hook is taken.
    drop(trace.on_clone.borrow_mut().take());
    Observation { panicked, fired: trace.fired.get(), events: trace.events.take(), snapshots }
}

fn run_table(original: bool, has_default: bool, has_coercer: bool, failure: Failure) -> Observation {
    let trace = Trace::new(failure);
    let metadata = metadata(&trace, has_default, has_coercer);
    let mut snapshots = Vec::new();
    trace.active.set(true);
    let panicked = catch_unwind(AssertUnwindSafe(|| {
        let table = if original {
            MetadataTable::original_new_for_construction(&HOST, metadata)
        } else {
            MetadataTable::new(&HOST, metadata)
        };
        {
            let registered = table.get(&HOST);
            snapshots.push(snapshot(&registered, table.default_metadata()));
        }
        drop(table);
    })).is_err();
    finish(&trace, panicked, snapshots)
}

fn run_generation(
    original: bool,
    frozen: bool,
    has_default: bool,
    has_coercer: bool,
    reenter: bool,
    failure: Failure,
) -> Observation {
    let trace = Trace::new(failure);
    let mut metadata = metadata(&trace, has_default, has_coercer);
    if frozen {
        metadata.freeze();
    }
    let metadata = Rc::new(metadata);
    *trace.metadata.borrow_mut() = Rc::downgrade(&metadata);
    let external = Rc::new(RefCell::new(None));
    if reenter {
        assert!(frozen && has_default && has_coercer);
        *external.borrow_mut() = Some(metadata.clone());
        let external = external.clone();
        let trace_for_hook = trace.clone();
        *trace.on_clone.borrow_mut() = Some(Box::new(move || {
            // Replace the external owner while generation borrows the old
            // metadata. Only the original caller must keep that input alive.
            let old = external.borrow_mut().take();
            drop(old);
            *external.borrow_mut() = Some(Rc::new(StyledPropertyMetadata::new(Some(trace_for_hook.value(99)))));
            trace_for_hook.record(Kind::Reentry, 0);
        }));
    }
    let mut snapshots = Vec::new();
    trace.active.set(true);
    let panicked = catch_unwind(AssertUnwindSafe(|| {
        let result = if original {
            StyledPropertyMetadata::original_generate_for_construction(&metadata)
        } else if reenter {
            StyledPropertyMetadata::generate_frozen_type_safe_metadata(&metadata)
        } else {
            StyledPropertyMetadata::generate_type_safe_metadata(&metadata)
        };
        snapshots.push(snapshot(&metadata, &result));
        drop(metadata);
        drop(result);
    })).is_err();
    let observation = finish(&trace, panicked, snapshots);
    drop(external);
    observation
}

fn compare_failures(run: impl Fn(bool, Failure) -> Observation) -> Observation {
    let expected = run(true, None);
    assert!(!expected.panicked && !expected.fired);
    assert_eq!(run(false, None), expected);
    for kind in INJECTABLE {
        let count = expected.events.iter().filter(|event| event.kind == kind).count();
        for occurrence in 1..=count {
            let failure = Some((kind, occurrence));
            let original = run(true, failure);
            let candidate = run(false, failure);
            assert!(original.panicked && original.fired, "{kind:?} occurrence {occurrence}");
            assert_eq!(candidate, original, "{kind:?} occurrence {occurrence}");
        }
    }
    expected
}

pub(crate) fn assert_table_frames() {
    for has_default in [false, true] {
        for has_coercer in [false, true] {
            let observed = compare_failures(|original, failure| run_table(original, has_default, has_coercer, failure));
            let result = &observed.snapshots[0];
            assert_eq!(result.shares_input, !has_coercer);
            assert!(result.input_frozen && result.output_frozen);
            assert!(!result.output_has_coercer);
            assert_eq!(result.output_number, has_default.then_some(7));
        }
    }
}

pub(crate) fn assert_public_generation() {
    for frozen in [false, true] {
        for has_default in [false, true] {
            for has_coercer in [false, true] {
                let observed = compare_failures(|original, failure| {
                    run_generation(original, frozen, has_default, has_coercer, false, failure)
                });
                let result = &observed.snapshots[0];
                assert_eq!(result.shares_input, frozen && !has_coercer);
                assert_eq!(result.input_frozen, frozen);
                assert!(result.output_frozen && !result.output_has_coercer);
                assert_eq!(result.output_number, has_default.then_some(7));
                assert_eq!(result.mode, BindingMode::TwoWay);
                assert_eq!(result.validation, Some(true));
            }
        }
    }
}

pub(crate) fn assert_generation_reentry() {
    let observed = compare_failures(|original, failure| run_generation(original, true, true, true, true, failure));
    let entry = observed.events.iter().find(|event| event.kind == Kind::Reentry).unwrap();
    assert_eq!(entry.metadata_owners, 1, "the caller alone retains the original input after replacement");
    assert_eq!(entry.callback_owners, 1);
    assert_eq!(observed.snapshots[0].output_number, Some(7));
    assert!(observed.events.iter().any(|event| event.kind == Kind::CallbackDrop));
}
