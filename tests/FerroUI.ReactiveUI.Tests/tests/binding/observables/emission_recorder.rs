//! `EmissionRecorder<T>` (`ReactiveUI.Binding.Tests/TestModels/EmissionRecorder.cs`): the recording
//! observer of the tests of the observables, kept beside them.

use ferroui_system::{IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Records the values and errors it receives, the deepest nesting of its deliveries, and runs a hook
/// from inside the delivery of the first value (`EmissionRecorder<T>`).
///
/// Upstream counts the deliveries running at once across threads; on one thread the count is the
/// depth of nested deliveries, which a serialized observable keeps at one.
pub struct EmissionRecorder<T> {
    values: RefCell<Vec<T>>,
    errors: RefCell<Vec<ObservableError>>,
    in_flight: Cell<usize>,
    max_in_flight: Cell<usize>,
    on_first_value: RefCell<Option<Box<dyn FnOnce()>>>,
}

/// Leaves the delivery when it ends, a panic included (upstream's `finally`).
struct InFlight<'a>(&'a Cell<usize>);

impl Drop for InFlight<'_> {
    fn drop(&mut self) {
        self.0.set(self.0.get() - 1);
    }
}

#[allow(dead_code)]
impl<T: Clone + 'static> EmissionRecorder<T> {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            values: RefCell::new(Vec::new()),
            errors: RefCell::new(Vec::new()),
            in_flight: Cell::new(0),
            max_in_flight: Cell::new(0),
            on_first_value: RefCell::new(None),
        })
    }

    /// Sets the hook run from inside the delivery of the first value (`OnFirstValue`).
    pub fn set_on_first_value(&self, hook: impl FnOnce() + 'static) {
        *self.on_first_value.borrow_mut() = Some(Box::new(hook));
    }

    /// The deepest nesting of deliveries seen (`MaxConcurrentEmissions`).
    pub fn max_concurrent_emissions(&self) -> usize {
        self.max_in_flight.get()
    }

    /// The values received so far (`Snapshot`).
    pub fn snapshot(&self) -> Vec<T> {
        self.values.borrow().clone()
    }

    /// The errors received so far (`ErrorSnapshot`).
    pub fn error_snapshot(&self) -> Vec<ObservableError> {
        self.errors.borrow().clone()
    }

    /// The recorder as an observer.
    pub fn observer(self: &Rc<Self>) -> Rc<dyn IObserver<T>> {
        self.clone()
    }
}

impl<T> IObserver<T> for EmissionRecorder<T> {
    fn on_next(&self, value: T) {
        let now = self.in_flight.get() + 1;
        self.in_flight.set(now);
        let _in_flight = InFlight(&self.in_flight);
        if now > self.max_in_flight.get() {
            self.max_in_flight.set(now);
        }
        let first = {
            let mut values = self.values.borrow_mut();
            values.push(value);
            values.len() == 1
        };
        // Upstream clears the hook on every value and runs it only on the first.
        let hook = self.on_first_value.borrow_mut().take();
        if first {
            if let Some(hook) = hook {
                hook();
            }
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.errors.borrow_mut().push(error);
    }

    fn on_completed(&self) {}
}
