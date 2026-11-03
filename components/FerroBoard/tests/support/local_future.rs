#![allow(dead_code)] // Shared test fixture; each test binary uses a different subset.
//! Test-only native future fixtures. No executor, Task identity/status or runtime emulation.
use std::{
    cell::RefCell,
    future::{Future, poll_fn},
    pin::Pin,
    rc::Rc,
    sync::Arc,
    task::{Context, Poll, Waker},
};
pub type Error = Arc<dyn std::error::Error + Send + Sync>;
pub fn poll<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}
pub fn ready<F: Future>(future: F) -> F::Output {
    match poll(std::pin::pin!(future)) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("fixture expected a ready native future"),
    }
}
#[derive(Clone)]
pub struct Completion<T> {
    state: Rc<State<T>>,
}
struct State<T> {
    result: RefCell<Option<Result<T, Error>>>,
    wakers: RefCell<Vec<Waker>>,
}
impl<T> Default for Completion<T> {
    fn default() -> Self {
        Self {
            state: Rc::new(State {
                result: RefCell::new(None),
                wakers: RefCell::new(Vec::new()),
            }),
        }
    }
}
impl<T: Clone + 'static> Completion<T> {
    pub fn future(&self) -> Pin<Box<dyn Future<Output = Result<T, Error>>>> {
        let state = self.state.clone();
        Box::pin(poll_fn(move |cx| {
            if let Some(result) = state.result.borrow().clone() {
                return Poll::Ready(result);
            }
            let mut wakers = state.wakers.borrow_mut();
            if !wakers.iter().any(|w| w.will_wake(cx.waker())) {
                wakers.push(cx.waker().clone());
            }
            Poll::Pending
        }))
    }
    pub fn complete(&self, value: T) {
        self.finish(Ok(value));
    }
    pub fn fail(&self, error: Error) {
        self.finish(Err(error));
    }
    pub fn cancel(&self) {
        self.fail(Arc::new(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            "fixture canceled",
        )));
    }
    fn finish(&self, value: Result<T, Error>) {
        *self.state.result.borrow_mut() = Some(value);
        let wakers = std::mem::take(&mut *self.state.wakers.borrow_mut());
        for waker in wakers {
            waker.wake();
        }
    }
}
