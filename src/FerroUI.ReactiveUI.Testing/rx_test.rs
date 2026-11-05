//! `RxTest` (`ReactiveUI.Testing/RxTest.cs`).

use ferroui_reactive::linq::TimeoutError;
use ferroui_reactiveui::PanicError;
use ferroui_splat::AppBuilder;
use ferroui_system::task::{Shared, SharedSender};
use ferroui_system::threading::tasks::run;
use ferroui_system::ObservableError;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::future::{poll_fn, Future};
use std::pin::Pin;
use std::rc::Rc;
use std::task::Poll;
use std::time::Duration;

/// How long a test waits for the gate, and runs, by default (`DefaultMaxWaitMs`).
const DEFAULT_MAX_WAIT_MS: u64 = 60_000;

/// The gate that lets one test of the application builder run at a time (`TestGate`, a
/// `SemaphoreSlim` of one): whether a test holds it, and the tests waiting for it in order.
#[derive(Default)]
struct TestGate {
    held: Cell<bool>,
    /// Each waiting test: the turn it waits for, and what gives it.
    waiters: RefCell<VecDeque<(Shared<()>, SharedSender<()>)>>,
}

ferroui_system::ferro_thread_local! {
    /// The gate of the thread.
    static TEST_GATE: Rc<TestGate> = Rc::new(TestGate::default());
}

impl TestGate {
    fn current() -> Rc<Self> {
        TEST_GATE.with(Rc::clone)
    }

    /// Takes the gate, waiting at most `max_wait` for the test that holds it; false when the time
    /// ran out (`TestGate.WaitAsync(maxWaitMs)`).
    async fn wait(self: Rc<Self>, max_wait: Duration) -> bool {
        if !self.held.get() {
            self.held.set(true);
            return true;
        }
        let (turn, granted) = Shared::<()>::channel();
        self.waiters.borrow_mut().push_back((granted.clone(), turn));
        match first_of(granted.clone(), delay(max_wait)).await {
            First::Left(_) => true,
            First::Right(_) => {
                let removed: VecDeque<_> = {
                    let mut waiters = self.waiters.borrow_mut();
                    let (kept, removed): (VecDeque<_>, VecDeque<_>) =
                        std::mem::take(&mut *waiters).into_iter().partition(|(waiter, _)| !Shared::ptr_eq(waiter, &granted));
                    *waiters = kept;
                    removed
                };
                // Dropped outside of the borrow.
                drop(removed);
                granted.is_finished()
            }
        }
    }

    /// Gives the gate to the next waiting test, or opens it (`TestGate.Release()`).
    fn release(&self) {
        let next = self.waiters.borrow_mut().pop_front();
        match next {
            Some((_, next)) => next.send(()),
            None => self.held.set(false),
        }
    }
}

/// Which of two tasks ended first.
enum First<A, B> {
    Left(A),
    Right(B),
}

/// The first of two tasks to end, `left` when both have (`Task.WhenAny`).
async fn first_of<A: Future + Unpin, B: Future + Unpin>(mut left: A, mut right: B) -> First<A::Output, B::Output> {
    poll_fn(move |context| {
        if let Poll::Ready(outcome) = Pin::new(&mut left).poll(context) {
            return Poll::Ready(First::Left(outcome));
        }
        if let Poll::Ready(outcome) = Pin::new(&mut right).poll(context) {
            return Poll::Ready(First::Right(outcome));
        }
        Poll::Pending
    })
    .await
}

/// A task that ends after `duration` (`Task.Delay`).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): `ferroui-system` has no timer of tasks; the time passes on
/// a worker thread ([`run`]), and the task ends on the current thread.
fn delay(duration: Duration) -> ferroui_system::task::Flatten<()> {
    run(move || {
        std::thread::sleep(duration);
        Ok(())
    })
    .flatten()
}

/// Runs a test of the application builder of Splat from a clean builder state, one test at a time
/// and within a time limit (`RxTest`).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the state of the application builder is per thread, as
/// the rest of the state of ReactiveUI and Splat is, so the gate lets one test of the thread run
/// at a time, not one of the process.
pub struct RxTest;

impl RxTest {
    /// [`app_builder_test_async_with`](Self::app_builder_test_async_with) with the default time
    /// limit of 60 seconds (`AppBuilderTestAsync(Func<Task>)`).
    pub fn app_builder_test_async<Fut: Future<Output = Result<(), ObservableError>> + 'static>(
        test_body: impl FnOnce() -> Fut + 'static,
    ) -> Shared<Result<(), ObservableError>> {
        Self::app_builder_test_async_with(test_body, DEFAULT_MAX_WAIT_MS)
    }

    /// Runs `test_body` once no other test of the application builder runs, with the state of the
    /// application builder reset before and after it, and gives a task that ends as the test does,
    /// the [`Shared`] of its result (`AppBuilderTestAsync(Func<Task>, int)`). The test body gives a
    /// future of a `Result<(), ObservableError>`, which runs as a task of the thread.
    ///
    /// The task ends with a [`TimeoutError`] when the gate is not free within `max_wait_ms`
    /// milliseconds, or when the test runs longer than that; with the error of the test when it
    /// fails, and with the error of a panic ([`PanicError`]) when `test_body` panics, as an
    /// exception thrown in an `async` method faults its task.
    ///
    /// Not ported: the `ArgumentException` of a null test body - argument validation.
    pub fn app_builder_test_async_with<Fut: Future<Output = Result<(), ObservableError>> + 'static>(
        test_body: impl FnOnce() -> Fut + 'static,
        max_wait_ms: u64,
    ) -> Shared<Result<(), ObservableError>> {
        let max_wait = Duration::from_millis(max_wait_ms);
        ferroui_system::task::spawn_local(async move {
            // Takes the gate, with a time limit so that a test that never ends cannot block the
            // others forever.
            let gate = TestGate::current();
            if !gate.clone().wait(max_wait).await {
                return Err(Rc::new(TimeoutError) as ObservableError);
            }

            // Resets any previous builder state.
            AppBuilder::reset_builder_state_for_tests();

            // Runs the test with a time limit, so that it cannot hang forever, and passes on its
            // error.
            let test_task = match PanicError::catch(test_body) {
                Ok(future) => ferroui_system::task::spawn_local(future).shared(),
                Err((error, _payload)) => Shared::ready(Err(error)),
            };
            let outcome = match first_of(test_task.flatten(), delay(max_wait)).await {
                First::Left(outcome) => outcome,
                First::Right(_) => Err(Rc::new(TimeoutError) as ObservableError),
            };

            // The final reset after the test.
            AppBuilder::reset_builder_state_for_tests();
            gate.release();
            outcome
        })
        .shared()
    }
}
