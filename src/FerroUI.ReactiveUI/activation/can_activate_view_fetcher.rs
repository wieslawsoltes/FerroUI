//! `CanActivateViewFetcher` (`ReactiveUI.Shared/Activation/CanActivateViewFetcher.cs`).

use super::IActivationForViewFetcher;
use crate::{BindingAffinity, IActivatableView};
use ferroui_reactive::disposables::SingleAssignmentDisposable;
use ferroui_reactive::Observable;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::rc::Rc;

/// The activation of the views that say themselves when they are activated, through
/// [`ICanActivate`](crate::ICanActivate) (`CanActivateViewFetcher`).
///
/// A view says that it implements `ICanActivate` with `IActivatableView::as_can_activate`.
#[derive(Clone, Copy, Debug, Default)]
pub struct CanActivateViewFetcher;

impl CanActivateViewFetcher {
    /// The fetcher.
    pub fn new() -> Self {
        Self
    }
}

impl IActivationForViewFetcher for CanActivateViewFetcher {
    /// [`BindingAffinity::EXACT_TYPE`] for a view that implements `ICanActivate`, zero otherwise.
    fn get_affinity_for_view(&self, view: &dyn IActivatableView) -> i32 {
        if view.as_can_activate().is_some() {
            BindingAffinity::EXACT_TYPE
        } else {
            0
        }
    }

    /// `true` on each tick of `Activated` and `false` on each tick of `Deactivated`; for a view
    /// that does not implement `ICanActivate`, a single `false`.
    fn get_activation_for_view(&self, view: &Rc<dyn IActivatableView>) -> Rc<dyn IObservable<bool>> {
        match view.as_can_activate() {
            None => Observable::return_(false),
            Some(can_activate) => Rc::new(ActivationStateObservable {
                activated: can_activate.activated(),
                deactivated: can_activate.deactivated(),
            }),
        }
    }
}

/// `true` on each tick of `activated` and `false` on each tick of `deactivated`, completing when
/// both complete (`ActivationStateObservable`).
struct ActivationStateObservable {
    activated: Rc<dyn IObservable<()>>,
    deactivated: Rc<dyn IObservable<()>>,
}

impl IObservable<bool> for ActivationStateObservable {
    fn subscribe(&self, observer: Rc<dyn IObserver<bool>>) -> Rc<dyn IDisposable> {
        let sink = Rc::new(Sink {
            downstream: observer,
            activated: SingleAssignmentDisposable::new(),
            deactivated: SingleAssignmentDisposable::new(),
            completed: Cell::new(0),
            done: Cell::new(false),
        });
        let activated = self.activated.subscribe(Rc::new(BranchObserver { parent: sink.clone(), value: true }));
        sink.activated.set_disposable(Some(activated));
        let deactivated = self.deactivated.subscribe(Rc::new(BranchObserver { parent: sink.clone(), value: false }));
        sink.deactivated.set_disposable(Some(deactivated));
        sink
    }
}

/// The number of sources that complete before the sink completes.
const SOURCE_COUNT: usize = 2;

/// Forwards the values of both sources and completes when both have completed (`Sink`).
struct Sink {
    downstream: Rc<dyn IObserver<bool>>,
    activated: Rc<SingleAssignmentDisposable>,
    deactivated: Rc<SingleAssignmentDisposable>,
    completed: Cell<usize>,
    done: Cell<bool>,
}

impl Sink {
    fn on_branch_next(&self, value: bool) {
        if self.done.get() {
            return;
        }
        self.downstream.on_next(value);
    }

    fn on_branch_error(&self, error: ObservableError) {
        if self.done.replace(true) {
            return;
        }
        self.downstream.on_error(error);
        self.activated.dispose();
        self.deactivated.dispose();
    }

    fn on_branch_completed(&self) {
        if self.done.get() {
            return;
        }
        let completed = self.completed.get() + 1;
        self.completed.set(completed);
        if completed < SOURCE_COUNT {
            return;
        }
        self.done.set(true);
        self.downstream.on_completed();
    }
}

impl IDisposable for Sink {
    fn dispose(&self) {
        if self.done.replace(true) {
            return;
        }
        self.activated.dispose();
        self.deactivated.dispose();
    }
}

/// Maps each tick of one source to its value (`BranchObserver`).
struct BranchObserver {
    parent: Rc<Sink>,
    value: bool,
}

impl IObserver<()> for BranchObserver {
    fn on_next(&self, _value: ()) {
        self.parent.on_branch_next(self.value);
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.on_branch_error(error);
    }

    fn on_completed(&self) {
        self.parent.on_branch_completed();
    }
}
