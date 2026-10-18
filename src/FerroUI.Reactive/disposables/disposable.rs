//! `Disposable`, `AnonymousDisposable`, `ICancelable` and `DisposeWith` (`Disposable.cs`,
//! `AnonymousDisposable.cs`, `ICancelable.cs`, `DisposableMixins.cs`).

use super::CompositeDisposable;
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A disposable that knows whether it has been disposed (`ICancelable`).
pub trait ICancelable: IDisposable {
    /// Whether the object has been disposed.
    fn is_disposed(&self) -> bool;
}

/// The factory of the common disposables (`Disposable`).
pub struct Disposable;

struct EmptyDisposable;

impl IDisposable for EmptyDisposable {
    fn dispose(&self) {}
}

impl Disposable {
    /// A disposable that does nothing when disposed (`Disposable.Empty`). The instance is shared by
    /// the thread.
    pub fn empty() -> Rc<dyn IDisposable> {
        ferroui_system::ferro_thread_local! {
            static EMPTY: Rc<dyn IDisposable> = Rc::new(EmptyDisposable);
        }
        EMPTY.with(Rc::clone)
    }

    /// A disposable that runs `dispose` the first time it is disposed (`Disposable.Create`).
    pub fn create(dispose: impl FnOnce() + 'static) -> Rc<dyn IDisposable> {
        AnonymousDisposable::new(dispose)
    }

    /// A disposable that runs `dispose` with `state` the first time it is disposed
    /// (`Disposable.Create<TState>`).
    pub fn create_with_state<S: 'static>(state: S, dispose: impl FnOnce(S) + 'static) -> Rc<dyn IDisposable> {
        AnonymousDisposable::new(move || dispose(state))
    }
}

/// A disposable that runs an action the first time it is disposed (`AnonymousDisposable`).
pub struct AnonymousDisposable {
    dispose: RefCell<Option<Box<dyn FnOnce()>>>,
    disposed: Cell<bool>,
}

impl AnonymousDisposable {
    /// A disposable that runs `dispose` when it is disposed.
    pub fn new(dispose: impl FnOnce() + 'static) -> Rc<Self> {
        Rc::new(Self { dispose: RefCell::new(Some(Box::new(dispose))), disposed: Cell::new(false) })
    }
}

impl IDisposable for AnonymousDisposable {
    fn dispose(&self) {
        self.disposed.set(true);
        let action = self.dispose.borrow_mut().take();
        if let Some(action) = action {
            action();
        }
    }
}

impl ICancelable for AnonymousDisposable {
    fn is_disposed(&self) -> bool {
        self.disposed.get()
    }
}

/// `DisposableMixins.DisposeWith`: adds a disposable to a composite disposable and hands it back.
pub trait DisposableExt {
    /// Adds the disposable to `composite`, which disposes it with itself, and returns it.
    fn dispose_with(self, composite: &CompositeDisposable) -> Self;
}

impl DisposableExt for Rc<dyn IDisposable> {
    fn dispose_with(self, composite: &CompositeDisposable) -> Self {
        composite.add(self.clone());
        self
    }
}

impl<D: IDisposable + 'static> DisposableExt for Rc<D> {
    fn dispose_with(self, composite: &CompositeDisposable) -> Self {
        composite.add(self.clone());
        self
    }
}
