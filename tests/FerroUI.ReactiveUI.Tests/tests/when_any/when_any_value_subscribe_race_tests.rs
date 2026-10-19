//! `WhenAnyValueSubscribeRaceTests` (`ReactiveUI.Tests/WhenAny/WhenAnyValueSubscribeRaceTests.cs`):
//! `WhenAnyValue` delivers the changes made while it delivers its initial value.
//!
//! On one thread, the change made during the initial delivery is made by the observer itself,
//! which is what the first two tests do.

use crate::mocks::TestFixture;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use ferroui_system::prop;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The property value written mid-race to prove the update was not lost.
const RACED_WORD: &str = "raced";

#[test]
fn when_any_value_mutation_during_initial_emit_is_delivered() {
    let fixture = TestFixture::new();
    fixture.set_is_only_one_word(Some("initial".to_owned()));
    let values = Rc::new(RefCell::new(Vec::<Option<String>>::new()));

    // Mutate while the observer is delivering its initial value.
    let (sink, target): (_, Weak<TestFixture>) = (values.clone(), Rc::downgrade(&fixture));
    let subscription = fixture.when_any_value(prop!(TestFixture => is_only_one_word)).subscribe_next(move |value| {
        sink.borrow_mut().push(value);
        if sink.borrow().len() != 1 {
            return;
        }

        if let Some(fixture) = target.upgrade() {
            fixture.set_is_only_one_word(Some(RACED_WORD.to_owned()));
        }
    });

    assert_eq!(fixture.is_only_one_word().as_deref(), Some(RACED_WORD));
    assert!(values.borrow().iter().any(|value| value.as_deref() == Some(RACED_WORD)));
    subscription.dispose();
}

#[test]
fn when_any_value_mutation_during_initial_emit_is_delivered_plain_inpc() {
    const RACED_VALUE: i32 = 2;

    let notifier = PlainInpc::new();
    notifier.set_value(1);
    let values = Rc::new(RefCell::new(Vec::<i32>::new()));

    let (sink, target): (_, Weak<PlainInpc>) = (values.clone(), Rc::downgrade(&notifier));
    let subscription = notifier.when_any_value(prop!(PlainInpc => value)).subscribe_next(move |value| {
        sink.borrow_mut().push(value);
        if sink.borrow().len() != 1 {
            return;
        }

        if let Some(notifier) = target.upgrade() {
            notifier.set_value(RACED_VALUE);
        }
    });

    assert_eq!(notifier.value(), RACED_VALUE);
    assert!(values.borrow().contains(&RACED_VALUE));
    subscription.dispose();
}

// Not ported: WhenAnyValue_ConcurrentMutationDuringSubscribe_NeverLosesFinalValue_Stress - real threads

/// A plain `INotifyPropertyChanged` source for the subscription test (`PlainInpc`).
struct PlainInpc {
    property_changed: Event<str>,
    value: Cell<i32>,
}

impl PlainInpc {
    fn new() -> Rc<Self> {
        Rc::new(Self { property_changed: Event::new(), value: Cell::new(0) })
    }

    /// `Value`.
    fn value(&self) -> i32 {
        self.value.get()
    }

    /// Sets `Value`, raising `PropertyChanged` only when it actually changes.
    fn set_value(&self, value: i32) {
        if self.value.get() == value {
            return;
        }

        self.value.set(value);
        self.property_changed.raise("Value");
    }
}

impl INotifyPropertyChanged for PlainInpc {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}
