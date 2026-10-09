//! The upstream unit tests of the headless platform
//! (`tests/<the headless unit test project>` upstream), one module per
//! upstream file, and the upstream tests of the controls that run on the
//! headless platform (`headless_probe_tests`).
//!
//! The tests of the platform run as upstream's do: on the dispatcher thread
//! of the [`HeadlessUnitTestSession`](crate::HeadlessUnitTestSession) of
//! their test assembly, with the application of `test_application`, which
//! has the Simple theme and renders with the Skia backend.

use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use std::future::Future;
use std::pin::{pin, Pin};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::thread::Thread;
use std::time::Duration;

mod test_application;

mod async_setup_tests;
mod headless_unit_test_session_tests;
mod input_tests;
mod isolation_tests;
mod leak_tests;
mod mouse_device_tests;
mod popup_tests;
mod rendering_tests;
mod second_window_tests;
mod services_tests;
mod setup_tests;
mod threading_tests;

/// A future that is completed by hand: the stand-in for a task completion
/// source.
struct Oneshot<T> {
    shared: Arc<Mutex<(Option<T>, Option<Waker>)>>,
}

struct OneshotSender<T> {
    shared: Arc<Mutex<(Option<T>, Option<Waker>)>>,
}

fn oneshot<T>() -> (OneshotSender<T>, Oneshot<T>) {
    let shared = Arc::new(Mutex::new((None, None)));
    (OneshotSender { shared: shared.clone() }, Oneshot { shared })
}

impl<T> OneshotSender<T> {
    fn send(&self, value: T) {
        let waker = {
            let mut shared = self.shared.lock().unwrap();
            shared.0 = Some(value);
            shared.1.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl<T> Future for Oneshot<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut shared = self.shared.lock().unwrap();
        match shared.0.take() {
            Some(value) => Poll::Ready(value),
            None => {
                shared.1 = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// Completes after `duration` of dispatcher time: the stand-in for a timer
/// task (`Task.Delay`).
fn delay(duration: Duration) -> Oneshot<()> {
    let (sender, receiver) = oneshot();
    DispatcherTimer::run_once(move || sender.send(()), duration, DispatcherPriority::NORMAL);
    receiver
}

struct ThreadWaker(Thread);

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

/// Waits for a future on the calling thread, which is not a dispatcher
/// thread: what awaiting does in a test of the original that does not run
/// on the session.
fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let waker = Waker::from(Arc::new(ThreadWaker(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::park(),
        }
    }
}
