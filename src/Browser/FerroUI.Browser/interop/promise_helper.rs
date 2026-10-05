//! Promises of the page awaited by the framework.
//!
//! Upstream marshals a `Task` across the boundary for every promise an
//! import returns. Here an asynchronous import returns the promise as an
//! opaque object; [`JsTask::new`] registers it under a request id and
//! `PromiseHelper.track` settles it into `PromiseHelper_OnResolved` or
//! `PromiseHelper_OnRejected` with that id. The completion wakes the task
//! that awaits it, which the dispatcher then resumes, so the framework never
//! blocks and never hands a closure to the page.
//!
//! Not an upstream file: it is the single completion mechanism of the
//! boundary (section 5 rule 2 of the browser design).

use super::JsObject;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "./ferroui.js")]
extern "C" {
    #[wasm_bindgen(js_namespace = PromiseHelper, js_name = track)]
    fn track(promise: &JsObject, request_id: u32);
}

/// The reason a promise was rejected: the message of the error it was
/// rejected with (C# `JSException.Message`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsError {
    message: String,
}

impl JsError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for JsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for JsError {}

impl From<JsError> for std::io::Error {
    fn from(error: JsError) -> Self {
        std::io::Error::other(error)
    }
}

/// The state of one request: its result once settled, and the task waiting
/// for it.
struct Pending<T> {
    result: Option<T>,
    waker: Option<Waker>,
}

/// Requests waiting for their completion, by id.
pub(crate) struct Completions<T> {
    last_id: u32,
    pending: HashMap<u32, Rc<RefCell<Pending<T>>>>,
}

impl<T> Completions<T> {
    pub(crate) fn new() -> Self {
        Self { last_id: 0, pending: HashMap::new() }
    }

    /// Registers a request and returns its id and the future of its result.
    pub(crate) fn begin(this: &'static std::thread::LocalKey<RefCell<Self>>) -> (u32, Completion<T>)
    where
        T: 'static,
    {
        this.with(|completions| {
            let mut completions = completions.borrow_mut();
            let pending = Rc::new(RefCell::new(Pending { result: None, waker: None }));
            // Ids wrap; an id is free again once its request has settled or been dropped.
            let mut id = completions.last_id;
            loop {
                id = id.wrapping_add(1);
                if id != 0 && !completions.pending.contains_key(&id) {
                    break;
                }
            }
            completions.last_id = id;
            completions.pending.insert(id, pending.clone());
            (id, Completion { id, pending, registry: this })
        })
    }

    /// Settles the request `id`. A request nobody waits for any more is
    /// ignored.
    pub(crate) fn complete(this: &'static std::thread::LocalKey<RefCell<Self>>, id: u32, result: T) {
        let pending = this.with(|completions| completions.borrow_mut().pending.remove(&id));
        if let Some(pending) = pending {
            let waker = {
                let mut pending = pending.borrow_mut();
                pending.result = Some(result);
                pending.waker.take()
            };
            if let Some(waker) = waker {
                waker.wake();
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

/// The result of one request, available once [`Completions::complete`] was
/// called with its id. Dropping it withdraws the request.
pub(crate) struct Completion<T: 'static> {
    id: u32,
    pending: Rc<RefCell<Pending<T>>>,
    registry: &'static std::thread::LocalKey<RefCell<Completions<T>>>,
}

impl<T: 'static> Completion<T> {
    #[cfg(test)]
    pub(crate) fn id(&self) -> u32 {
        self.id
    }
}

impl<T: 'static> Future for Completion<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut pending = self.pending.borrow_mut();
        match pending.result.take() {
            Some(result) => Poll::Ready(result),
            None => {
                pending.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

impl<T: 'static> Drop for Completion<T> {
    fn drop(&mut self) {
        // Still registered only while unsettled; `try_with` because the registry may already be
        // gone when the thread ends.
        let id = self.id;
        let _ = self.registry.try_with(|completions| {
            if let Ok(mut completions) = completions.try_borrow_mut() {
                if completions.pending.get(&id).is_some_and(|pending| Rc::ptr_eq(pending, &self.pending)) {
                    completions.pending.remove(&id);
                }
            }
        });
    }
}

type PromiseResult = Result<JsObject, JsError>;

thread_local! {
    static PROMISES: RefCell<Completions<PromiseResult>> = RefCell::new(Completions::new());
}

/// A promise of the page, awaited by the framework (C# `Task<JSObject>`
/// returned by a `[JSImport]`).
pub struct JsTask {
    completion: Completion<PromiseResult>,
}

impl JsTask {
    /// Awaits `promise`. A value that is not a promise resolves to itself.
    pub fn new(promise: JsObject) -> Self {
        let (id, completion) = Completions::begin(&PROMISES);
        track(&promise, id);
        Self { completion }
    }
}

impl Future for JsTask {
    type Output = PromiseResult;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<PromiseResult> {
        Pin::new(&mut self.completion).poll(cx)
    }
}

/// The promise of request `request_id` was fulfilled with `value`.
#[wasm_bindgen(js_name = PromiseHelper_OnResolved)]
pub fn on_resolved(request_id: u32, value: JsObject) {
    Completions::complete(&PROMISES, request_id, Ok(value));
}

/// The promise of request `request_id` was rejected; `message` is the
/// message of the error.
#[wasm_bindgen(js_name = PromiseHelper_OnRejected)]
pub fn on_rejected(request_id: u32, message: String) {
    Completions::complete(&PROMISES, request_id, Err(JsError::new(message)));
}

#[cfg(test)]
mod tests {
    // Not from upstream: the completion mechanism is specific to the port.
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::task::Wake;

    thread_local! {
        static TEST: RefCell<Completions<i32>> = RefCell::new(Completions::new());
    }

    struct CountingWaker(AtomicUsize);

    impl Wake for CountingWaker {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn poll<F: Future + Unpin>(future: &mut F, waker: &Arc<CountingWaker>) -> Poll<F::Output> {
        let waker = Waker::from(waker.clone());
        Pin::new(future).poll(&mut Context::from_waker(&waker))
    }

    #[test]
    fn completion_resolves_after_complete_and_wakes_the_waiting_task() {
        let waker = Arc::new(CountingWaker(AtomicUsize::new(0)));
        let (id, mut completion) = Completions::begin(&TEST);
        assert_eq!(id, completion.id());
        assert_eq!(poll(&mut completion, &waker), Poll::Pending);

        Completions::complete(&TEST, id, 42);

        assert_eq!(waker.0.load(Ordering::SeqCst), 1);
        assert_eq!(poll(&mut completion, &waker), Poll::Ready(42));
        assert_eq!(TEST.with(|t| t.borrow().pending_count()), 0);
    }

    #[test]
    fn completion_settled_before_the_first_poll_is_ready() {
        let waker = Arc::new(CountingWaker(AtomicUsize::new(0)));
        let (id, mut completion) = Completions::begin(&TEST);
        Completions::complete(&TEST, id, 7);
        assert_eq!(poll(&mut completion, &waker), Poll::Ready(7));
        assert_eq!(waker.0.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn requests_get_distinct_ids_and_complete_independently() {
        let waker = Arc::new(CountingWaker(AtomicUsize::new(0)));
        let (first_id, mut first) = Completions::begin(&TEST);
        let (second_id, mut second) = Completions::begin(&TEST);
        assert_ne!(first_id, second_id);

        Completions::complete(&TEST, second_id, 2);
        assert_eq!(poll(&mut first, &waker), Poll::Pending);
        assert_eq!(poll(&mut second, &waker), Poll::Ready(2));

        Completions::complete(&TEST, first_id, 1);
        assert_eq!(poll(&mut first, &waker), Poll::Ready(1));
    }

    #[test]
    fn dropped_request_is_withdrawn_and_its_late_completion_ignored() {
        let (id, completion) = Completions::begin(&TEST);
        assert_eq!(TEST.with(|t| t.borrow().pending_count()), 1);
        drop(completion);
        assert_eq!(TEST.with(|t| t.borrow().pending_count()), 0);
        Completions::complete(&TEST, id, 3);
        assert_eq!(TEST.with(|t| t.borrow().pending_count()), 0);
    }

    #[test]
    fn rejection_converts_to_an_io_error_with_the_message() {
        let error: std::io::Error = JsError::new("The user aborted a request.").into();
        assert_eq!(error.kind(), std::io::ErrorKind::Other);
        assert_eq!(error.to_string(), "The user aborted a request.");
    }
}
