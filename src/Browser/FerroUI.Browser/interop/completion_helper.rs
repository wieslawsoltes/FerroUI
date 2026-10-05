//! Completion of the asynchronous calls into the page.
//!
//! Not in the original, where the runtime turns a promise of the page into a
//! task. Here a function of the script module that answers with a promise is
//! imported as returning the promise object; [`await_promise`] gives the
//! promise a request id, hands both to `CompletionHelper.track` and returns a
//! future. When the promise settles, the script module calls
//! `CompletionHelper_OnResolved` or `CompletionHelper_OnRejected` with the id,
//! which completes the future and wakes its task. The task is polled again by
//! whoever drives it (on the UI thread, the dispatcher), never from inside the
//! callback.
//!
//! This is the one way an asynchronous answer crosses the boundary: no Rust
//! closure is passed to the page.

use super::JsObject;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "./ferroui.js")]
extern "C" {
    #[wasm_bindgen(js_namespace = CompletionHelper, js_name = track)]
    fn js_track(promise: &JsObject, request_id: u32);
}

/// Why a promise of the page was rejected: the name and the message of the
/// error it was rejected with (for a reason that is not an error, an empty
/// name and the reason as text).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromiseError {
    name: String,
    message: String,
}

impl PromiseError {
    /// Creates the error of a rejection.
    pub fn new(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self { name: name.into(), message: message.into() }
    }

    /// The name of the error, such as `NotAllowedError`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The message of the error.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for PromiseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.name.is_empty() {
            f.write_str(&self.message)
        } else {
            write!(f, "{}: {}", self.name, self.message)
        }
    }
}

impl std::error::Error for PromiseError {}

impl From<PromiseError> for std::io::Error {
    /// The error as an I/O error whose payload is the rejection (read it back
    /// with `get_ref` and `downcast_ref::<PromiseError>` for its name and
    /// message). The kind follows the name of the DOM exception, so that a
    /// caller can tell a missing item, a refused permission and a cancelled
    /// request apart; any other name is [`std::io::ErrorKind::Other`].
    fn from(error: PromiseError) -> Self {
        let kind = match error.name() {
            "NotFoundError" => std::io::ErrorKind::NotFound,
            "NotAllowedError" | "SecurityError" => std::io::ErrorKind::PermissionDenied,
            "AbortError" => std::io::ErrorKind::Interrupted,
            _ => std::io::ErrorKind::Other,
        };
        std::io::Error::new(kind, error)
    }
}

/// The outcome of a promise: its value or its rejection.
pub type PromiseOutcome<V> = Result<V, PromiseError>;

/// The call that hands a promise and its request id to the page; replaced
/// by a recorder in the tests.
trait IPromiseTracker<V> {
    fn track(&self, promise: &V, request_id: u32);
}

struct PagePromiseTracker;

impl IPromiseTracker<JsObject> for PagePromiseTracker {
    fn track(&self, promise: &JsObject, request_id: u32) {
        js_track(promise, request_id);
    }
}

struct PendingRequest<V> {
    outcome: Option<PromiseOutcome<V>>,
    waker: Option<Waker>,
}

/// The requests waiting for the page, by id.
struct Completions<V> {
    last_request_id: Cell<u32>,
    pending: RefCell<HashMap<u32, PendingRequest<V>>>,
}

impl<V> Completions<V> {
    fn new() -> Rc<Self> {
        Rc::new(Self { last_request_id: Cell::new(0), pending: RefCell::new(HashMap::new()) })
    }

    /// Registers a request for `promise`, hands it to the page and returns
    /// the future of its outcome.
    fn await_promise(self: &Rc<Self>, tracker: &dyn IPromiseTracker<V>, promise: &V) -> PromiseFuture<V> {
        let mut request_id = self.last_request_id.get();
        loop {
            // Ids are reused after 2^32 requests; one still waiting is skipped.
            request_id = request_id.wrapping_add(1);
            if !self.pending.borrow().contains_key(&request_id) {
                break;
            }
        }
        self.last_request_id.set(request_id);
        self.pending.borrow_mut().insert(request_id, PendingRequest { outcome: None, waker: None });

        tracker.track(promise, request_id);

        PromiseFuture { completions: self.clone(), request_id }
    }

    /// Completes a request. Returns `false` when no request has the id: it
    /// was completed before, or its future was dropped.
    fn complete(&self, request_id: u32, outcome: PromiseOutcome<V>) -> bool {
        let waker = {
            let mut pending = self.pending.borrow_mut();
            let Some(request) = pending.get_mut(&request_id) else {
                return false;
            };
            if request.outcome.is_some() {
                return false;
            }
            request.outcome = Some(outcome);
            request.waker.take()
        };

        // Outside the borrow: waking may poll the future right away.
        if let Some(waker) = waker {
            waker.wake();
        }
        true
    }

    fn pending_count(&self) -> usize {
        self.pending.borrow().len()
    }
}

/// The outcome of a promise of the page, as a future. Dropping the future
/// forgets the request: an answer that comes later is ignored.
///
/// # Panics
/// Polling the future again after it returned its outcome panics: the
/// request is gone once its outcome has been taken.
pub struct PromiseFuture<V> {
    completions: Rc<Completions<V>>,
    request_id: u32,
}

impl<V> PromiseFuture<V> {
    /// The id the page answers the request with.
    pub fn request_id(&self) -> u32 {
        self.request_id
    }
}

impl<V> Future for PromiseFuture<V> {
    type Output = PromiseOutcome<V>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut pending = self.completions.pending.borrow_mut();
        let Some(request) = pending.get_mut(&self.request_id) else {
            panic!("The request {} was polled after it completed.", self.request_id);
        };

        if request.outcome.is_some() {
            let request = pending.remove(&self.request_id);
            return Poll::Ready(request.and_then(|request| request.outcome).expect("the outcome of the request"));
        }

        request.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

impl<V> Drop for PromiseFuture<V> {
    fn drop(&mut self) {
        let removed = self.completions.pending.borrow_mut().remove(&self.request_id);
        drop(removed);
    }
}

thread_local! {
    static PAGE_COMPLETIONS: Rc<Completions<JsObject>> = Completions::new();
}

/// The future of the outcome of a promise of the page.
pub fn await_promise(promise: &JsObject) -> PromiseFuture<JsObject> {
    PAGE_COMPLETIONS.with(|completions| completions.await_promise(&PagePromiseTracker, promise))
}

/// The number of requests waiting for the page.
pub fn pending_request_count() -> usize {
    PAGE_COMPLETIONS.with(|completions| completions.pending_count())
}

/// A promise handed to the page by [`await_promise`] was fulfilled with
/// `value`.
#[wasm_bindgen(js_name = CompletionHelper_OnResolved)]
pub fn on_resolved(request_id: u32, value: JsObject) {
    PAGE_COMPLETIONS.with(|completions| completions.complete(request_id, Ok(value)));
}

/// A promise handed to the page by [`await_promise`] was rejected with an
/// error of the given name and message.
#[wasm_bindgen(js_name = CompletionHelper_OnRejected)]
pub fn on_rejected(request_id: u32, name: Option<String>, message: Option<String>) {
    let error = PromiseError::new(name.unwrap_or_default(), message.unwrap_or_default());
    PAGE_COMPLETIONS.with(|completions| completions.complete(request_id, Err(error)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::task::Wake;

    /// Records the requests handed to the page.
    #[derive(Default)]
    struct RecordingTracker {
        tracked: RefCell<Vec<(String, u32)>>,
    }

    impl IPromiseTracker<String> for RecordingTracker {
        fn track(&self, promise: &String, request_id: u32) {
            self.tracked.borrow_mut().push((promise.clone(), request_id));
        }
    }

    #[derive(Default)]
    struct CountingWaker {
        wakes: AtomicUsize,
    }

    impl Wake for CountingWaker {
        fn wake(self: Arc<Self>) {
            self.wakes.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn poll<V>(future: &mut PromiseFuture<V>, waker: &Arc<CountingWaker>) -> Poll<PromiseOutcome<V>> {
        let waker = Waker::from(waker.clone());
        let mut context = Context::from_waker(&waker);
        Pin::new(future).poll(&mut context)
    }

    #[test]
    fn the_promise_is_handed_to_the_page_with_its_request_id() {
        let completions = Completions::new();
        let tracker = RecordingTracker::default();

        let first = completions.await_promise(&tracker, &"first".to_string());
        let second = completions.await_promise(&tracker, &"second".to_string());

        assert_eq!(
            vec![("first".to_string(), first.request_id()), ("second".to_string(), second.request_id())],
            *tracker.tracked.borrow()
        );
        assert_ne!(first.request_id(), second.request_id());
        assert_eq!(2, completions.pending_count());
    }

    #[test]
    fn the_future_waits_until_the_page_answers_and_then_has_the_value() {
        let completions = Completions::new();
        let tracker = RecordingTracker::default();
        let waker = Arc::new(CountingWaker::default());

        let mut future = completions.await_promise(&tracker, &"promise".to_string());
        assert_eq!(Poll::Pending, poll(&mut future, &waker));
        assert_eq!(0, waker.wakes.load(Ordering::SeqCst));

        assert!(completions.complete(future.request_id(), Ok("value".to_string())));
        assert_eq!(1, waker.wakes.load(Ordering::SeqCst));

        assert_eq!(Poll::Ready(Ok("value".to_string())), poll(&mut future, &waker));
        assert_eq!(0, completions.pending_count());
    }

    #[test]
    fn a_rejection_is_the_outcome_of_the_future() {
        let completions = Completions::new();
        let tracker = RecordingTracker::default();
        let waker = Arc::new(CountingWaker::default());

        let mut future = completions.await_promise(&tracker, &"promise".to_string());
        assert_eq!(Poll::Pending, poll(&mut future, &waker));
        let error = PromiseError::new("NotAllowedError", "Read permission denied.");
        assert!(completions.complete(future.request_id(), Err(error.clone())));

        assert_eq!(Poll::Ready(Err(error)), poll(&mut future, &waker));
    }

    #[test]
    fn an_answer_that_comes_before_the_first_poll_is_kept() {
        let completions = Completions::new();
        let tracker = RecordingTracker::default();
        let waker = Arc::new(CountingWaker::default());

        let mut future = completions.await_promise(&tracker, &"promise".to_string());
        assert!(completions.complete(future.request_id(), Ok("early".to_string())));

        assert_eq!(Poll::Ready(Ok("early".to_string())), poll(&mut future, &waker));
        assert_eq!(0, waker.wakes.load(Ordering::SeqCst));
    }

    #[test]
    fn answers_complete_their_own_request_in_any_order() {
        let completions = Completions::new();
        let tracker = RecordingTracker::default();
        let waker = Arc::new(CountingWaker::default());

        let mut first = completions.await_promise(&tracker, &"first".to_string());
        let mut second = completions.await_promise(&tracker, &"second".to_string());
        assert_eq!(Poll::Pending, poll(&mut first, &waker));
        assert_eq!(Poll::Pending, poll(&mut second, &waker));

        assert!(completions.complete(second.request_id(), Ok("2".to_string())));
        assert_eq!(Poll::Pending, poll(&mut first, &waker));
        assert_eq!(Poll::Ready(Ok("2".to_string())), poll(&mut second, &waker));

        assert!(completions.complete(first.request_id(), Ok("1".to_string())));
        assert_eq!(Poll::Ready(Ok("1".to_string())), poll(&mut first, &waker));
    }

    #[test]
    fn a_dropped_future_forgets_its_request_and_a_later_answer_is_ignored() {
        let completions = Completions::new();
        let tracker = RecordingTracker::default();

        let future = completions.await_promise(&tracker, &"promise".to_string());
        let request_id = future.request_id();
        drop(future);

        assert_eq!(0, completions.pending_count());
        assert!(!completions.complete(request_id, Ok("late".to_string())));
    }

    #[test]
    fn an_answer_to_an_unknown_or_answered_request_is_ignored() {
        let completions = Completions::new();
        let tracker = RecordingTracker::default();
        let waker = Arc::new(CountingWaker::default());

        assert!(!completions.complete(7, Ok("stray".to_string())));

        let mut future = completions.await_promise(&tracker, &"promise".to_string());
        assert!(completions.complete(future.request_id(), Ok("first".to_string())));
        assert!(!completions.complete(future.request_id(), Ok("second".to_string())));
        assert_eq!(Poll::Ready(Ok("first".to_string())), poll(&mut future, &waker));
    }

    #[test]
    fn a_request_id_still_waiting_is_not_given_again() {
        let completions = Completions::new();
        let tracker = RecordingTracker::default();

        let waiting = completions.await_promise(&tracker, &"waiting".to_string());
        // The next id would wrap around to the one still waiting.
        completions.last_request_id.set(waiting.request_id().wrapping_sub(1));

        let next = completions.await_promise(&tracker, &"next".to_string());
        assert_ne!(waiting.request_id(), next.request_id());
    }

    #[test]
    #[should_panic(expected = "polled after it completed")]
    fn polling_again_after_the_outcome_panics() {
        let completions = Completions::new();
        let tracker = RecordingTracker::default();
        let waker = Arc::new(CountingWaker::default());

        let mut future = completions.await_promise(&tracker, &"promise".to_string());
        completions.complete(future.request_id(), Ok("value".to_string()));
        assert_eq!(Poll::Ready(Ok("value".to_string())), poll(&mut future, &waker));
        let _ = poll(&mut future, &waker);
    }

    #[test]
    fn a_rejection_is_an_io_error_that_keeps_the_name_and_the_message() {
        for (name, kind) in [
            ("NotFoundError", std::io::ErrorKind::NotFound),
            ("NotAllowedError", std::io::ErrorKind::PermissionDenied),
            ("SecurityError", std::io::ErrorKind::PermissionDenied),
            ("AbortError", std::io::ErrorKind::Interrupted),
            ("TypeError", std::io::ErrorKind::Other),
            ("", std::io::ErrorKind::Other),
        ] {
            let error: std::io::Error = PromiseError::new(name, "The user aborted a request.").into();
            assert_eq!(kind, error.kind(), "{name}");
            let rejection = error.get_ref().and_then(|inner| inner.downcast_ref::<PromiseError>()).expect("the rejection");
            assert_eq!(name, rejection.name());
            assert_eq!("The user aborted a request.", rejection.message());
        }
    }

    #[test]
    fn the_error_is_shown_with_its_name() {
        assert_eq!("NotAllowedError: denied", PromiseError::new("NotAllowedError", "denied").to_string());
        assert_eq!("reason", PromiseError::new("", "reason").to_string());
    }
}
