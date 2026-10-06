use super::{Disposable, IDisposable, IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::Rc;

/// The overridable members of an observable built on
/// [`LightweightObservableBase`]: the observable embeds the base, returns
/// it from [`observable_base`](Self::observable_base) and implements
/// `IObservable` with [`LightweightObservableBase::subscribe`].
pub trait LightweightObservable<T>: 'static {
    /// The embedded state.
    fn observable_base(&self) -> &LightweightObservableBase<T>;

    /// Called when the first observer subscribes.
    fn initialize(&self);

    /// Called when the last observer unsubscribes, and after the
    /// observable completed or failed.
    fn deinitialize(&self);

    /// Called after an observer subscribed; `first` tells whether it is
    /// the only one.
    fn subscribed(&self, _observer: &Rc<dyn IObserver<T>>, _first: bool) {}
}

/// The state of a lightweight observable: its observers, notified in
/// subscription order, and the error it failed with. Once it completes or
/// fails, the list is gone and later subscribers are told so at once.
pub struct LightweightObservableBase<T> {
    error: RefCell<Option<ObservableError>>,
    observers: RefCell<Option<Vec<Rc<dyn IObserver<T>>>>>,
}

impl<T> Default for LightweightObservableBase<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> LightweightObservableBase<T> {
    pub fn new() -> Self {
        Self { error: RefCell::new(None), observers: RefCell::new(Some(Vec::new())) }
    }

    /// Whether any observer is subscribed.
    pub fn has_observers(&self) -> bool {
        self.observers.borrow().as_ref().is_some_and(|observers| !observers.is_empty())
    }

    /// Subscribes `observer` to `owner`. The first subscription initializes
    /// the owner; disposing the returned handle removes the observer.
    pub fn subscribe<O: LightweightObservable<T>>(owner: &Rc<O>, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable>
    where
        T: 'static,
    {
        let base = owner.observable_base();
        let first = {
            let mut observers = base.observers.borrow_mut();
            match observers.as_mut() {
                Some(observers) => {
                    let first = observers.is_empty();
                    observers.push(observer.clone());
                    first
                }
                None => {
                    drop(observers);
                    let error = base.error.borrow().clone();
                    match error {
                        Some(error) => observer.on_error(error),
                        None => observer.on_completed(),
                    }
                    return Disposable::empty();
                }
            }
        };

        if first {
            owner.initialize();
        }

        owner.subscribed(&observer, first);

        Rc::new(RemoveObserver { parent: RefCell::new(Some((owner.clone(), observer))) })
    }

    fn remove<O: LightweightObservable<T>>(owner: &O, observer: &Rc<dyn IObserver<T>>) {
        let base = owner.observable_base();
        let now_empty = {
            let mut observers = base.observers.borrow_mut();
            let Some(observers) = observers.as_mut() else { return };
            if let Some(index) = observers.iter().position(|o| std::ptr::addr_eq(Rc::as_ptr(o), Rc::as_ptr(observer))) {
                observers.remove(index);
            }
            if observers.is_empty() {
                observers.shrink_to_fit();
                true
            } else {
                false
            }
        };
        if now_empty {
            owner.deinitialize();
        }
    }

    /// The observers to notify, copied so that they can subscribe and
    /// unsubscribe while they are notified.
    fn snapshot(&self) -> Option<Vec<Rc<dyn IObserver<T>>>> {
        self.observers.borrow().clone()
    }

    /// Notifies every observer of a value.
    pub fn publish_next(&self, value: T)
    where
        T: Clone,
    {
        let Some(observers) = self.snapshot() else { return };
        for observer in &observers {
            observer.on_next(value.clone());
        }
    }

    /// Completes the observable: notifies every observer and deinitializes
    /// the owner.
    pub fn publish_completed<O: LightweightObservable<T>>(owner: &O) {
        let Some(observers) = owner.observable_base().observers.borrow_mut().take() else { return };
        for observer in &observers {
            observer.on_completed();
        }
        owner.deinitialize();
    }

    /// Fails the observable: notifies every observer and deinitializes the
    /// owner.
    pub fn publish_error<O: LightweightObservable<T>>(owner: &O, error: ObservableError) {
        let base = owner.observable_base();
        let Some(observers) = base.observers.borrow_mut().take() else { return };
        *base.error.borrow_mut() = Some(error.clone());
        for observer in &observers {
            observer.on_error(error.clone());
        }
        owner.deinitialize();
    }
}

/// The subscription returned by [`LightweightObservableBase::subscribe`].
struct RemoveObserver<O, T> {
    parent: RefCell<Option<(Rc<O>, Rc<dyn IObserver<T>>)>>,
}

impl<O: LightweightObservable<T>, T: 'static> IDisposable for RemoveObserver<O, T> {
    fn dispose(&self) {
        let parent = self.parent.borrow_mut().take();
        if let Some((parent, observer)) = parent {
            LightweightObservableBase::remove(&*parent, &observer);
        }
    }
}
