//! `ToListObservable` (`Linq/QueryLanguageEx.cs`, the experimental `ObservableEx.ToListObservable`)
//! and the list it returns (`ListObservable.cs`).
//!
//! Deviation (DEVIATIONS.md, Reactive): the members of upstream's `ListObservable<T>` (an
//! `IList<T>`) first wait for the sequence to end (`Wait`, blocking) and rethrow its error. On the
//! one thread of the port nothing can end the sequence during such a wait: the members panic when
//! the sequence has not ended, and return the error of the sequence as an `Err`. The sequence of
//! the list, which notifies only the end of the source (upstream's `IObservable<object>`), is a
//! sequence of `()`.

use crate::disposables::StableCompositeDisposable;
use crate::linq::element_at::ArgumentOutOfRangeError;
use crate::linq::{ObservableExt, SequenceError};
use crate::subjects::AsyncSubject;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::Rc;

/// The values of a sequence as a list, once the sequence has ended (`ListObservable<T>`), and the
/// sequence of its end.
///
/// The list subscribes to the source when it is made, and keeps its values; the list can then be
/// read and changed as a list once the source has completed.
pub struct ListObservable<T> {
    subscription: Rc<dyn IDisposable>,
    subject: Rc<AsyncSubject<()>>,
    results: Rc<RefCell<Vec<T>>>,
}

impl<T: 'static> ListObservable<T> {
    /// The list of the values of `source` (`ListObservable(IObservable<T>)`).
    pub fn new(source: Rc<dyn IObservable<T>>) -> Rc<Self> {
        let subject: Rc<AsyncSubject<()>> = AsyncSubject::new();
        let results: Rc<RefCell<Vec<T>>> = Rc::new(RefCell::new(Vec::new()));
        let (values, on_error, on_completed) = (results.clone(), subject.clone(), subject.clone());
        let subscription = source.subscribe_all(
            move |value| values.borrow_mut().push(value),
            move |error| on_error.on_error(error),
            move || on_completed.on_completed(),
        );
        Rc::new(Self { subscription, subject, results })
    }

    /// `Wait`: the end of the source, its error as an `Err`. Panics when the source has not ended,
    /// where upstream blocks until it does (`DefaultIfEmpty().Wait()`).
    fn wait(&self) -> Result<(), ObservableError> {
        self.subject.get_result().map(|_| ())
    }

    fn check_index(index: usize, count: usize) -> Result<(), ObservableError> {
        if index < count {
            Ok(())
        } else {
            Err(Rc::new(ArgumentOutOfRangeError::new("index")))
        }
    }

    /// The last value (`Value`); an `Err` with [`SequenceError::NoElements`] when there is none.
    pub fn value(&self) -> Result<T, ObservableError>
    where
        T: Clone,
    {
        self.wait()?;
        self.results.borrow().last().cloned().ok_or_else(|| Rc::new(SequenceError::NoElements) as ObservableError)
    }

    /// The index of the first value equal to `item` (`IndexOf`).
    pub fn index_of(&self, item: &T) -> Result<Option<usize>, ObservableError>
    where
        T: PartialEq,
    {
        self.wait()?;
        Ok(self.results.borrow().iter().position(|value| value == item))
    }

    /// Inserts `item` at `index` (`Insert`); an index beyond the count is an
    /// [`ArgumentOutOfRangeError`].
    pub fn insert(&self, index: usize, item: T) -> Result<(), ObservableError> {
        self.wait()?;
        let count = self.results.borrow().len();
        Self::check_index(index, count + 1)?;
        self.results.borrow_mut().insert(index, item);
        Ok(())
    }

    /// Removes the value at `index` (`RemoveAt`).
    pub fn remove_at(&self, index: usize) -> Result<(), ObservableError> {
        self.wait()?;
        let count = self.results.borrow().len();
        Self::check_index(index, count)?;
        self.results.borrow_mut().remove(index);
        Ok(())
    }

    /// The value at `index` (the getter of the indexer).
    pub fn get(&self, index: usize) -> Result<T, ObservableError>
    where
        T: Clone,
    {
        self.wait()?;
        let results = self.results.borrow();
        Self::check_index(index, results.len())?;
        Ok(results[index].clone())
    }

    /// Replaces the value at `index` (the setter of the indexer).
    pub fn set(&self, index: usize, value: T) -> Result<(), ObservableError> {
        self.wait()?;
        let mut results = self.results.borrow_mut();
        Self::check_index(index, results.len())?;
        results[index] = value;
        Ok(())
    }

    /// Adds `item` at the end (`Add`).
    pub fn add(&self, item: T) -> Result<(), ObservableError> {
        self.wait()?;
        self.results.borrow_mut().push(item);
        Ok(())
    }

    /// Removes every value (`Clear`).
    pub fn clear(&self) -> Result<(), ObservableError> {
        self.wait()?;
        self.results.borrow_mut().clear();
        Ok(())
    }

    /// Whether a value equals `item` (`Contains`).
    pub fn contains(&self, item: &T) -> Result<bool, ObservableError>
    where
        T: PartialEq,
    {
        self.wait()?;
        Ok(self.results.borrow().contains(item))
    }

    /// Copies the values into `array` from `array_index` on (`CopyTo`); an array too short is an
    /// [`ArgumentOutOfRangeError`].
    pub fn copy_to(&self, array: &mut [T], array_index: usize) -> Result<(), ObservableError>
    where
        T: Clone,
    {
        self.wait()?;
        let results = self.results.borrow();
        if array_index > array.len() || array.len() - array_index < results.len() {
            return Err(Rc::new(ArgumentOutOfRangeError::new("arrayIndex")));
        }
        array[array_index..array_index + results.len()].clone_from_slice(&results);
        Ok(())
    }

    /// The count of the values (`Count`).
    pub fn count(&self) -> Result<usize, ObservableError> {
        self.wait()?;
        Ok(self.results.borrow().len())
    }

    /// Whether the list cannot be changed (`IsReadOnly`): it can.
    pub fn is_read_only(&self) -> bool {
        false
    }

    /// Removes the first value equal to `item` (`Remove`); whether there was one.
    pub fn remove(&self, item: &T) -> Result<bool, ObservableError>
    where
        T: PartialEq,
    {
        self.wait()?;
        let mut results = self.results.borrow_mut();
        match results.iter().position(|value| value == item) {
            Some(index) => {
                results.remove(index);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// The values, in order (`GetEnumerator`).
    pub fn to_vec(&self) -> Result<Vec<T>, ObservableError>
    where
        T: Clone,
    {
        self.wait()?;
        Ok(self.results.borrow().clone())
    }
}

impl<T: 'static> IObservable<()> for ListObservable<T> {
    /// The end of the source (`Subscribe(IObserver<object>)`); disposing the subscription also
    /// disposes the subscription of the list to the source.
    fn subscribe(&self, observer: Rc<dyn IObserver<()>>) -> Rc<dyn IDisposable> {
        StableCompositeDisposable::create(self.subscription.clone(), self.subject.subscribe(observer))
    }
}

pub(crate) fn to_list_observable<T: 'static>(source: Rc<dyn IObservable<T>>) -> Rc<ListObservable<T>> {
    ListObservable::new(source)
}

/// The methods of `ToListObservable` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! to_list_observable_methods {
    () => {
        /// The list of the values of the sequence, which subscribes to it at once
        /// (`ToListObservable<TSource>(IObservable<TSource>)`).
        fn to_list_observable(&self) -> std::rc::Rc<crate::linq::to_list_observable::ListObservable<T>> {
            crate::linq::to_list_observable::to_list_observable(self.observable())
        }
    };
}
pub(crate) use to_list_observable_methods;
