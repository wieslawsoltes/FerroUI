//! `SelfObservingPerson` (`Domain/SelfObservingPerson.cs`).

use super::Person;
use ferroui_reactive::ObservableExt;
use ferroui_system::{IDisposable, IObservable};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct State {
    completed: Cell<bool>,
    person: RefCell<Option<Rc<Person>>>,
    update_count: Cell<usize>,
}

/// A subscriber to a sequence of people (`SelfObservingPerson`): it keeps the latest person, counts
/// them, and records the end of the sequence.
pub struct SelfObservingPerson {
    state: Rc<State>,
    clean_up: Rc<dyn IDisposable>,
}

impl SelfObservingPerson {
    /// Subscribes to `observable` (`new SelfObservingPerson(IObservable<Person>)`): `Finally`
    /// records the end, however the subscription ends.
    pub fn new(observable: Rc<dyn IObservable<Rc<Person>>>) -> Self {
        let state = Rc::new(State { completed: Cell::new(false), person: RefCell::new(None), update_count: Cell::new(0) });
        let (on_finally, on_next) = (state.clone(), state.clone());
        let clean_up = observable.finally(move || on_finally.completed.set(true)).subscribe_next(move |person| {
            *on_next.person.borrow_mut() = Some(person);
            on_next.update_count.set(on_next.update_count.get() + 1);
        });
        Self { state, clean_up }
    }

    /// Whether the sequence has ended (`Completed`).
    pub fn completed(&self) -> bool {
        self.state.completed.get()
    }

    /// The latest person (`Person`).
    pub fn person(&self) -> Option<Rc<Person>> {
        self.state.person.borrow().clone()
    }

    /// The number of people received (`UpdateCount`).
    pub fn update_count(&self) -> usize {
        self.state.update_count.get()
    }
}

impl IDisposable for SelfObservingPerson {
    /// Ends the subscription (`Dispose`).
    fn dispose(&self) {
        self.clean_up.dispose();
    }
}
