//! `ViewModelActivator` (`ReactiveUI.Shared/Activation/ViewModelActivator.cs`).

use ferroui_reactive::disposables::{CompositeDisposable, Disposable};
use ferroui_reactive::subjects::Subject;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::{Rc, Weak};

/// A block registered with an activator: what to set up on activation, returning what to dispose
/// on deactivation.
pub(crate) type ActivationBlock = Rc<dyn Fn() -> Vec<Rc<dyn IDisposable>>>;

struct Inner {
    /// The blocks run on activation (`_blocks`).
    blocks: RefCell<Vec<ActivationBlock>>,
    /// `_activated`.
    activated: Rc<Subject<()>>,
    /// `_deactivated`.
    deactivated: Rc<Subject<()>>,
    /// What the blocks of the current activation returned (`_activationHandle`).
    activation_handle: RefCell<Rc<dyn IDisposable>>,
    /// The activations without a matching deactivation (`_refCount`).
    ref_count: Cell<usize>,
}

/// The helper a view model holds to take part in activation (`ViewModelActivator`).
///
/// The view calls it when it comes on screen: the view model then sets up what should only live
/// while it is shown, such as subscriptions to global objects, and tears it down when the view
/// leaves. Register what to do with
/// [`when_activated`](crate::ViewModelActivationMixins::when_activated).
///
/// Activation is not loading: a view is activated when it enters the visual tree and deactivated
/// when it leaves it, which matters for views that are recycled. Activations are counted: the
/// view model is deactivated when every activation has been matched by a deactivation.
///
/// Create the activator once, in the most basic view model of a hierarchy, and let the others use
/// it. The view that shows the view model must set up its own activation
/// ([`ViewForMixins`](crate::ViewForMixins)).
///
/// The activator is a shared handle: a clone is the same activator.
// Deviation (DEVIATIONS.md, ReactiveUI): the activator is a handle (`Clone`) over shared state,
// since the disposable that `activate` returns deactivates it later.
#[derive(Clone)]
pub struct ViewModelActivator {
    inner: Rc<Inner>,
}

impl Default for ViewModelActivator {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for ViewModelActivator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ViewModelActivator")
            .field("blocks", &self.inner.blocks.borrow().len())
            .field("ref_count", &self.inner.ref_count.get())
            .finish()
    }
}

impl PartialEq for ViewModelActivator {
    /// The same activator.
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl ViewModelActivator {
    /// An activator without blocks, not activated (`ViewModelActivator()`).
    pub fn new() -> Self {
        Self {
            inner: Rc::new(Inner {
                blocks: RefCell::new(Vec::new()),
                activated: Subject::new(),
                deactivated: Subject::new(),
                activation_handle: RefCell::new(Disposable::empty()),
                ref_count: Cell::new(0),
            }),
        }
    }

    /// Ticks each time the activator is activated (`Activated`).
    pub fn activated(&self) -> Rc<dyn IObservable<()>> {
        self.inner.activated.as_observable()
    }

    /// Ticks each time the activator is deactivated (`Deactivated`).
    pub fn deactivated(&self) -> Rc<dyn IObservable<()>> {
        self.inner.deactivated.as_observable()
    }

    /// Activates the view model: on the first activation, runs the registered blocks and ticks
    /// [`activated`](Self::activated) (`Activate()`). The view calls it when it is activated; a
    /// unit test calls it to simulate that.
    ///
    /// Returns a disposable that calls [`deactivate`](Self::deactivate).
    pub fn activate(&self) -> Rc<dyn IDisposable> {
        let count = self.inner.ref_count.get() + 1;
        self.inner.ref_count.set(count);
        if count == 1 {
            let blocks: Vec<ActivationBlock> = self.inner.blocks.borrow().clone();
            let disposable = CompositeDisposable::new();
            for block in blocks {
                for item in block() {
                    disposable.add(item);
                }
            }
            let previous = std::mem::replace(&mut *self.inner.activation_handle.borrow_mut(), disposable as Rc<dyn IDisposable>);
            previous.dispose();
            self.inner.activated.on_next(());
        }
        let inner: Weak<Inner> = Rc::downgrade(&self.inner);
        Disposable::create(move || {
            if let Some(inner) = inner.upgrade() {
                ViewModelActivator { inner }.deactivate();
            }
        })
    }

    /// Deactivates the view model, following the count of activations: the blocks' disposables
    /// are disposed and [`deactivated`](Self::deactivated) ticks when the last activation is
    /// matched (`Deactivate()`). The view calls it when it is deactivated.
    pub fn deactivate(&self) {
        self.deactivate_with(false);
    }

    /// [`deactivate`](Self::deactivate); with `ignore_ref_count`, the view model is deactivated
    /// even when it was activated more than once (`Deactivate(bool)`).
    pub fn deactivate_with(&self, ignore_ref_count: bool) {
        if ignore_ref_count {
            self.inner.ref_count.set(0);
        } else {
            let current = self.inner.ref_count.get();
            if current == 0 {
                return;
            }
            let next = current - 1;
            self.inner.ref_count.set(next);
            if next != 0 {
                return;
            }
        }
        let previous = std::mem::replace(&mut *self.inner.activation_handle.borrow_mut(), Disposable::empty());
        previous.dispose();
        self.inner.deactivated.on_next(());
    }

    /// Adds a block to run on activation; what it returns is disposed on deactivation
    /// (`AddActivationBlock`).
    pub(crate) fn add_activation_block(&self, block: ActivationBlock) {
        self.inner.blocks.borrow_mut().push(block);
    }
}

impl IDisposable for ViewModelActivator {
    /// Disposes what the current activation set up, and the two observables (`Dispose()`).
    fn dispose(&self) {
        let handle = self.inner.activation_handle.borrow().clone();
        handle.dispose();
        self.inner.activated.dispose();
        self.inner.deactivated.dispose();
    }
}
