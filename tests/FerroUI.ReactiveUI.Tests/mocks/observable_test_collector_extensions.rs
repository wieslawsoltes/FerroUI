//! `ObservableTestCollectorExtensions` (`ReactiveUI.Test.Utilities/ObservableTestCollectorExtensions.cs`).

use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::Rc;

/// The values a sequence produced, in order (`Collection<T>`).
pub type Collection<T> = Rc<RefCell<Vec<T>>>;

/// Subscribes to `source` and collects its values (`Collect()`); an error of the sequence panics.
///
/// The name differs from upstream's `Collect` because `ObservableExt::collect` is Rx's pull
/// adapter of that name.
pub fn collect_items<T: 'static>(source: &Rc<dyn IObservable<T>>) -> Collection<T> {
    let items: Collection<T> = Rc::new(RefCell::new(Vec::new()));
    let _ = source.subscribe(Rc::new(CollectingObserver { items: items.clone() }));
    items
}

/// Adds every value to the collection (`CollectingObserver<T>`).
struct CollectingObserver<T> {
    items: Collection<T>,
}

impl<T> IObserver<T> for CollectingObserver<T> {
    fn on_next(&self, value: T) {
        self.items.borrow_mut().push(value);
    }

    fn on_error(&self, error: ObservableError) {
        panic!("{error}");
    }

    fn on_completed(&self) {
        // No completion handling needed for a test collector.
    }
}
