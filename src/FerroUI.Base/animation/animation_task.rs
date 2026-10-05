//! The completion of an animation run.
//!
//! Animations are driven by clock ticks on the UI thread, so the result of
//! running one is completed from within a tick (or a cancellation) and needs
//! no executor: it can be polled as a [`Future`], queried, or given
//! continuations, which run synchronously at completion.

use crate::animation::AnimationError;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

#[derive(Default)]
struct State {
    result: Option<Result<(), AnimationError>>,
    wakers: Vec<Waker>,
    continuations: Vec<Box<dyn FnOnce()>>,
}

/// The pending or completed result of running an animation or a page
/// transition.
///
/// It completes with `Ok` when the animation has ended or was cancelled,
/// and with an error when the animation could not be run.
#[derive(Clone, Default)]
pub struct AnimationTask {
    state: Rc<RefCell<State>>,
}

impl AnimationTask {
    /// A task that has not completed yet.
    pub(crate) fn pending() -> Self {
        Self::default()
    }

    /// A successfully completed task.
    pub fn completed() -> Self {
        let task = Self::default();
        task.state.borrow_mut().result = Some(Ok(()));
        task
    }

    /// Whether the task has completed, successfully or not.
    pub fn is_completed(&self) -> bool {
        self.state.borrow().result.is_some()
    }

    /// The result of the task; `None` while it is pending.
    pub fn result(&self) -> Option<Result<(), AnimationError>> {
        self.state.borrow().result.clone()
    }

    /// Runs `continuation` when the task completes; immediately if it has
    /// completed already.
    pub fn on_completed(&self, continuation: impl FnOnce() + 'static) {
        if self.is_completed() {
            continuation();
        } else {
            self.state.borrow_mut().continuations.push(Box::new(continuation));
        }
    }

    /// Completes the task unless it has completed already. Returns whether
    /// it did.
    pub(crate) fn try_set(&self, result: Result<(), AnimationError>) -> bool {
        let (wakers, continuations) = {
            let mut state = self.state.borrow_mut();
            if state.result.is_some() {
                return false;
            }
            state.result = Some(result);
            (std::mem::take(&mut state.wakers), std::mem::take(&mut state.continuations))
        };
        for continuation in continuations {
            continuation();
        }
        for waker in wakers {
            waker.wake();
        }
        true
    }

    pub(crate) fn try_set_result(&self) -> bool {
        self.try_set(Ok(()))
    }

    /// A task that completes when all of `tasks` have completed: with the
    /// first error among them, if any.
    pub fn when_all(tasks: impl IntoIterator<Item = AnimationTask>) -> AnimationTask {
        let tasks: Vec<AnimationTask> = tasks.into_iter().collect();
        let all = AnimationTask::pending();
        let complete = {
            let all = all.clone();
            let tasks = tasks.clone();
            Rc::new(move || {
                if tasks.iter().all(AnimationTask::is_completed) {
                    let error = tasks.iter().find_map(|task| task.result().and_then(Result::err));
                    all.try_set(match error {
                        Some(error) => Err(error),
                        None => Ok(()),
                    });
                }
            })
        };
        if tasks.is_empty() {
            all.try_set_result();
        }
        for task in &tasks {
            let complete = complete.clone();
            task.on_completed(move || complete());
        }
        all
    }
}

impl Future for AnimationTask {
    type Output = Result<(), AnimationError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.state.borrow_mut();
        match &state.result {
            Some(result) => Poll::Ready(result.clone()),
            None => {
                if !state.wakers.iter().any(|waker| waker.will_wake(cx.waker())) {
                    state.wakers.push(cx.waker().clone());
                }
                Poll::Pending
            }
        }
    }
}
