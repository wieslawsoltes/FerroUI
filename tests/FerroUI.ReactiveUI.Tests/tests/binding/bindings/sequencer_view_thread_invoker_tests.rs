//! `SequencerViewThreadInvokerTests` (`ReactiveUI.Binding.Tests/Bindings/SequencerViewThreadInvokerTests.cs`).
//!
//! The state handed to a posted callback is what the callback captures.

use crate::tests::binding::test_models::ManualSequencer;
use ferroui_reactive::concurrency::{IScheduler, ScheduledAction};
use ferroui_reactiveui::*;
use ferroui_system::IDisposable;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, SystemTime};

/// A scheduler bound to a thread, whose access the test sets, queuing its work on a manual
/// scheduler (`AffineSequencer`).
struct AffineSequencer {
    inner: Rc<ManualSequencer>,
    has_access: Cell<bool>,
}

impl AffineSequencer {
    fn new(has_access: bool) -> Rc<Self> {
        Rc::new(Self { inner: ManualSequencer::new(), has_access: Cell::new(has_access) })
    }
}

impl IScheduler for AffineSequencer {
    fn now(&self) -> SystemTime {
        self.inner.now()
    }

    fn schedule(&self, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.inner.schedule(action)
    }

    fn schedule_after(&self, due_time: Duration, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.inner.schedule_after(due_time, action)
    }

    fn schedule_at(&self, due_time: SystemTime, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.inner.schedule_at(due_time, action)
    }
}

impl IThreadAffineSequencer for AffineSequencer {
    fn check_access(&self) -> bool {
        self.has_access.get()
    }
}

/// An object owned by the thread of its scheduler, or by none (`OwnedTarget`).
struct OwnedTarget(Option<Rc<AffineSequencer>>);

/// The invoker of the owned targets (`OwnedTargetInvoker`).
fn owned_target_invoker() -> SequencerViewThreadInvoker<OwnedTarget, AffineSequencer> {
    SequencerViewThreadInvoker::new(|target: &OwnedTarget| target.0.clone())
}

fn owned(sequencer: Option<Rc<AffineSequencer>>) -> Rc<dyn Any> {
    Rc::new(OwnedTarget(sequencer))
}

#[test]
fn claims_matches_the_target_type() {
    let invoker = owned_target_invoker();

    assert!(invoker.claims(&owned(None)));
    assert!(!invoker.claims(&(Rc::new(()) as Rc<dyn Any>)));
}

/// `CheckAccess_WithAnOwningSequencer_FollowsTheSequencer(bool hasAccess)`, for one row.
fn check_access_with_an_owning_sequencer_follows_the_sequencer(has_access: bool) {
    let target = owned(Some(AffineSequencer::new(has_access)));

    assert_eq!(owned_target_invoker().check_access(&target), has_access);
}

#[test]
fn check_access_with_an_owning_sequencer_follows_the_sequencer_with_access() {
    check_access_with_an_owning_sequencer_follows_the_sequencer(true);
}

#[test]
fn check_access_with_an_owning_sequencer_follows_the_sequencer_without_access() {
    check_access_with_an_owning_sequencer_follows_the_sequencer(false);
}

#[test]
fn check_access_with_no_owning_sequencer_is_true() {
    assert!(owned_target_invoker().check_access(&owned(None)));
}

#[test]
fn post_with_an_owning_sequencer_runs_the_callback_on_the_sequencer() {
    let sequencer = AffineSequencer::new(false);
    let state: Rc<dyn Any> = Rc::new(());
    let received: Rc<RefCell<Option<Rc<dyn Any>>>> = Rc::new(RefCell::new(None));

    let (sent, sink) = (state.clone(), received.clone());
    owned_target_invoker().post(&owned(Some(sequencer.clone())), Box::new(move || *sink.borrow_mut() = Some(sent)));
    let before_run = received.borrow().is_some();
    let _ = sequencer.inner.run_pending();

    assert!(!before_run);
    assert!(received.borrow().as_ref().is_some_and(|received| Rc::ptr_eq(received, &state)));
}

#[test]
fn post_with_no_owning_sequencer_runs_the_callback_inline() {
    let state: Rc<dyn Any> = Rc::new(());
    let received: Rc<RefCell<Option<Rc<dyn Any>>>> = Rc::new(RefCell::new(None));

    let (sent, sink) = (state.clone(), received.clone());
    owned_target_invoker().post(&owned(None), Box::new(move || *sink.borrow_mut() = Some(sent)));

    assert!(received.borrow().as_ref().is_some_and(|received| Rc::ptr_eq(received, &state)));
}

// Not ported: Post_WithANullCallback_Throws - argument validation
