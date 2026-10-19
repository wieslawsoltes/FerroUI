//! Port of `ViewModels/ViewModelBase.cs`: the base view model with ReactiveUI activation support.

use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactiveui::{ReactiveObject, ViewModelActivator};
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

/// The base view model with ReactiveUI activation support (`ViewModelBase`): the part of a view model
/// that makes it a reactive object, its activator, and the disposables it owns.
///
/// Deviation (DEVIATIONS.md, samples): the base class of the reference is a part that a view model
/// holds (the `ReactiveObject` of the port is one too); [`reactive_object!`] is invoked for the view
/// model with the field that holds this part, and the view model implements `IActivatableViewModel`
/// and `IDisposable` by forwarding to it. `Dispose(bool disposing)` of a derived class is its own
/// `dispose`, which disposes what it owns, then this part.
///
/// [`reactive_object!`]: ferroui_reactiveui::reactive_object
pub struct ViewModelBase {
    reactive: ReactiveObject,
    /// The activator (`Activator`).
    activator: ViewModelActivator,
    /// The disposables owned by the view model (`_disposables`).
    disposables: Rc<CompositeDisposable>,
    /// A latch raised by the first call to dispose this instance (`_disposed`).
    disposed: Cell<bool>,
}

impl Default for ViewModelBase {
    fn default() -> Self {
        Self::new()
    }
}

impl AsRef<ReactiveObject> for ViewModelBase {
    fn as_ref(&self) -> &ReactiveObject {
        &self.reactive
    }
}

impl ViewModelBase {
    /// The part of a new view model, which owns its activator (`ViewModelBase()`).
    pub fn new() -> Self {
        let activator = ViewModelActivator::new();
        let disposables = CompositeDisposable::new();
        disposables.add(Rc::new(activator.clone()));
        Self { reactive: ReactiveObject::new(), activator, disposables, disposed: Cell::new(false) }
    }

    /// The activator of the view model (`Activator`).
    pub fn activator(&self) -> &ViewModelActivator {
        &self.activator
    }

    /// Tracks a disposable for the lifetime of the view model, and gives it back
    /// (`Track<TDisposable>`): a shared handle such as a command.
    pub fn track<D: IDisposable + Clone + 'static>(&self, disposable: D) -> D {
        self.disposables.add(Rc::new(disposable.clone()));
        disposable
    }

    /// Tracks a subscription or another shared disposable for the lifetime of the view model
    /// (`Track<TDisposable>`), and gives it back.
    pub fn track_disposable(&self, disposable: Rc<dyn IDisposable>) -> Rc<dyn IDisposable> {
        self.disposables.add(disposable.clone());
        disposable
    }

    /// Whether the view model was disposed.
    pub fn is_disposed(&self) -> bool {
        self.disposed.get()
    }

    /// Disposes what the view model owns, once (`Dispose()`).
    pub fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }

        self.disposables.dispose();
    }
}
