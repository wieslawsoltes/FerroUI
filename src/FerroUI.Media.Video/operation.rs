//! Cancellation belongs to the player, including an unpolled native creation.
//! Slots hold provisional futures, never UI objects or a strong player owner.
use crate::{MediaError, MediaFuture};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    task::{Poll, Waker},
};
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum OperationScope {
    Source,
    Transport,
    Seek,
}
trait PendingOperation {
    fn scope(&self) -> OperationScope;
    fn cancel(&self);
    fn live(&self) -> bool;
}
struct Slot<T> {
    scope: OperationScope,
    pending: RefCell<Option<MediaFuture<T>>>,
    valid: Box<dyn Fn() -> bool>,
    cancelled: Cell<bool>,
    finished: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}
impl<T> PendingOperation for Slot<T> {
    fn scope(&self) -> OperationScope {
        self.scope
    }
    fn live(&self) -> bool {
        !self.cancelled.get() && !self.finished.get()
    }
    fn cancel(&self) {
        if !self.live() {
            return;
        }
        self.cancelled.set(true);
        let pending = self.pending.borrow_mut().take();
        let waker = self.waker.borrow_mut().take();
        drop(pending);
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}
#[derive(Default)]
pub(crate) struct OperationSet(RefCell<Vec<Weak<dyn PendingOperation>>>);
impl OperationSet {
    pub fn cancel(&self, scope: Option<OperationScope>) {
        let active: Vec<_> = self.0.borrow().iter().filter_map(Weak::upgrade).collect();
        for operation in active {
            if scope.is_none_or(|s| s == operation.scope()) {
                operation.cancel();
            }
        }
        self.0
            .borrow_mut()
            .retain(|weak| weak.upgrade().is_some_and(|op| op.live()));
    }
    pub fn register<T: 'static>(
        &self,
        pending: MediaFuture<T>,
        scope: OperationScope,
        valid: impl Fn() -> bool + 'static,
    ) -> MediaFuture<T> {
        let slot = Rc::new(Slot {
            scope,
            pending: RefCell::new(Some(pending)),
            valid: Box::new(valid),
            cancelled: Cell::new(false),
            finished: Cell::new(false),
            waker: RefCell::new(None),
        });
        let erased: Rc<dyn PendingOperation> = slot.clone();
        self.0
            .borrow_mut()
            .retain(|weak| weak.upgrade().is_some_and(|op| op.live()));
        self.0.borrow_mut().push(Rc::downgrade(&erased));
        Box::pin(std::future::poll_fn(move |cx| {
            if slot.cancelled.get() || !(slot.valid)() {
                slot.cancel();
                return Poll::Ready(Err(MediaError::cancelled()));
            }
            let Some(mut pending) = slot.pending.borrow_mut().take() else {
                return Poll::Ready(Err(MediaError::cancelled()));
            };
            *slot.waker.borrow_mut() = Some(cx.waker().clone());
            // The native future can synchronously replace/dispose its source.
            // Keep no RefCell borrow while invoking it, so cancellation reenters.
            let result = pending.as_mut().poll(cx);
            if slot.cancelled.get() || !(slot.valid)() {
                slot.cancel();
                return Poll::Ready(Err(MediaError::cancelled()));
            }
            match result {
                Poll::Pending => {
                    *slot.pending.borrow_mut() = Some(pending);
                    Poll::Pending
                }
                Poll::Ready(result) => {
                    slot.finished.set(true);
                    slot.waker.borrow_mut().take();
                    Poll::Ready(result)
                }
            }
        }))
    }
}
impl Drop for OperationSet {
    fn drop(&mut self) {
        self.cancel(None);
    }
}
