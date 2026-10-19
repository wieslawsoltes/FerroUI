//! `PropertyChangingObservableInitialEmitSerializationTests`
//! (`ReactiveUI.Binding.Tests/Observables/PropertyChangingObservableInitialEmitSerializationTests.cs`).
//!
//! Upstream's competitor writes on another thread while the initial value is being read, and
//! waits for it there. On one thread, the competitor's writes run inside the initial read, the
//! window that upstream's thread interleaves with: the notifications they raise are queued behind
//! the initial value, as upstream's serialization queues them.

use super::emission_recorder::EmissionRecorder;
use ferroui_reactiveui::*;
use ferroui_system::component_model::INotifyPropertyChanging;
use ferroui_system::events::Event;
use ferroui_system::IObservable;
use std::cell::Cell;
use std::rc::Rc;

/// The value of the competitor's second write.
const SECOND_WRITE: i32 = 2;

fn assert_sequence(actual: &[i32], expected: &[i32]) {
    assert_eq!(actual.len(), expected.len());
    for (index, value) in expected.iter().enumerate() {
        assert_eq!(actual[index], *value);
    }
}

/// The competitor of the initial emit (`InitialEmitCompetitor`): a read that, the first time it
/// runs, writes 1 and then [`SECOND_WRITE`] through `write`, and returns the value it read on entry
/// (`CreateContendedRead`).
fn contended_read<S: 'static>(read: impl Fn(&Rc<S>) -> i32 + 'static, write: impl Fn(&Rc<S>, i32) + 'static) -> impl Fn(&Rc<S>) -> i32 + 'static {
    let contended = Cell::new(false);
    move |source: &Rc<S>| {
        let value_on_entry = read(source);
        if contended.replace(true) {
            return value_on_entry;
        }
        write(source, 1);
        write(source, SECOND_WRITE);
        value_on_entry
    }
}

#[test]
fn subscribe_conventional_source_writes_during_initial_emit_does_not_deliver_a_stale_value_last() {
    let source = RaiseThenWriteViewModel::new();
    source.set_version(0);
    let recorder = EmissionRecorder::<i32>::new();
    let observable = PropertyChangingObservable::new(
        &source,
        "Version",
        contended_read(|source: &Rc<RaiseThenWriteViewModel>| source.version(), |source: &Rc<RaiseThenWriteViewModel>, version: i32| source.set_version(version)),
    );

    let subscription = observable.subscribe(recorder.observer());
    // The competing emits are held behind the initial emit, so the initial 0 lands first and the
    // pre-change values that follow it only ever move forward.
    assert_sequence(&recorder.snapshot(), &[0, 0, 1]);
    subscription.dispose();
}

#[test]
fn subscribe_atypical_source_writes_before_raising_does_not_deliver_a_stale_value_last() {
    let source = WriteThenRaiseViewModel::new();
    source.set_version(0);
    let recorder = EmissionRecorder::<i32>::new();
    let observable = PropertyChangingObservable::new(
        &source,
        "Version",
        contended_read(|source: &Rc<WriteThenRaiseViewModel>| source.version(), |source: &Rc<WriteThenRaiseViewModel>, version: i32| source.set_version(version)),
    );

    let subscription = observable.subscribe(recorder.observer());
    assert_sequence(&recorder.snapshot(), &[0, 1, SECOND_WRITE]);
    subscription.dispose();
}

#[test]
fn subscribe_no_concurrent_notification_emits_the_initial_value_to_every_subscriber() {
    let source = RaiseThenWriteViewModel::new();
    source.set_version(0);
    let observable = PropertyChangingObservable::new(&source, "Version", |x: &Rc<RaiseThenWriteViewModel>| x.version());
    let first = EmissionRecorder::<i32>::new();
    let second = EmissionRecorder::<i32>::new();

    let first_subscription = observable.subscribe(first.observer());
    let second_subscription = observable.subscribe(second.observer());
    assert_sequence(&first.snapshot(), &[0]);
    assert_sequence(&second.snapshot(), &[0]);
    second_subscription.dispose();
    first_subscription.dispose();
}

/// A view model that raises `PropertyChanging`, then writes (`RaiseThenWriteViewModel`).
struct RaiseThenWriteViewModel {
    version: Cell<i32>,
    property_changing: Event<str>,
}

impl RaiseThenWriteViewModel {
    fn new() -> Rc<Self> {
        Rc::new(Self { version: Cell::new(0), property_changing: Event::new() })
    }

    fn version(&self) -> i32 {
        self.version.get()
    }

    fn set_version(&self, value: i32) {
        self.property_changing.raise("Version");
        self.version.set(value);
    }
}

impl INotifyPropertyChanging for RaiseThenWriteViewModel {
    fn property_changing(&self) -> &Event<str> {
        &self.property_changing
    }
}

/// A view model that writes, then raises `PropertyChanging` (`WriteThenRaiseViewModel`).
struct WriteThenRaiseViewModel {
    version: Cell<i32>,
    property_changing: Event<str>,
}

impl WriteThenRaiseViewModel {
    fn new() -> Rc<Self> {
        Rc::new(Self { version: Cell::new(0), property_changing: Event::new() })
    }

    fn version(&self) -> i32 {
        self.version.get()
    }

    fn set_version(&self, value: i32) {
        self.version.set(value);
        self.property_changing.raise("Version");
    }
}

impl INotifyPropertyChanging for WriteThenRaiseViewModel {
    fn property_changing(&self) -> &Event<str> {
        &self.property_changing
    }
}
