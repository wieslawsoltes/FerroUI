//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/DeferAsyncTest.cs` of Rx.NET.
//! Adapted: `Defer(Func<Task<IObservable<T>>>)` is `Observable::defer_async` and
//! `DeferAsync(Func<CancellationToken, Task<IObservable<T>>>)` is `defer_async_with_token`. The work
//! that `Task.Factory.StartNew` runs on a worker thread is split: the worker (`tasks::run`) does
//! what upstream's work does apart from building the sequence, which is built on the test thread
//! when the work ends, as the sequences are not `Send`. `ToEnumerable()` is `block_on` of
//! `to_list().to_task()`; `ManualResetEvent` is a local event of a mutex and a condition variable.

use ferroui_reactive::{Observable, ObservableError, ObservableExt};
use ferroui_system::threading::tasks::{block_on, run};
use ferroui_system::threading::CancellationToken;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex};

/// `ManualResetEvent`: an event that stays set, for threads.
#[derive(Clone, Default)]
struct ManualResetEvent(Arc<(Mutex<bool>, Condvar)>);

impl ManualResetEvent {
    fn set(&self) {
        let (lock, arrived) = &*self.0;
        *lock.lock().unwrap() = true;
        arrived.notify_all();
    }

    fn wait_one(&self) {
        let (lock, arrived) = &*self.0;
        let mut set = lock.lock().unwrap();
        while !*set {
            set = arrived.wait(set).unwrap();
        }
    }
}

#[test]
fn defer_async_simple() {
    let xs = Observable::defer_async(|| async {
        let value = run(|| Ok(42)).await?;
        Ok::<_, ObservableError>(Observable::return_(value))
    });

    let res = block_on(xs.to_list().to_task()).unwrap();

    assert_eq!(vec![42], res);
}

#[test]
fn defer_async_with_cancel_simple() {
    let xs = Observable::defer_async_with_token(|_ct: CancellationToken| async {
        let value = run(|| Ok(42)).await?;
        Ok::<_, ObservableError>(Observable::return_(value))
    });

    let res = block_on(xs.to_list().to_task()).unwrap();

    assert_eq!(vec![42], res);
}

#[test]
fn defer_async_with_cancel_cancel() {
    let n = 10; // 0000;
    for _ in 0..n {
        let e = ManualResetEvent::default();
        let called = Rc::new(Cell::new(false));

        let (event, flag) = (e.clone(), called.clone());
        let xs = Observable::defer_async_with_token(move |ct: CancellationToken| {
            let e = event.clone();
            let called = flag.clone();
            async move {
                run(move || {
                    e.set();

                    while !ct.is_cancellation_requested() {
                        std::hint::spin_loop();
                    }

                    Ok(())
                })
                .await?;

                Ok::<_, ObservableError>(Observable::defer(move || {
                    called.set(true);
                    Observable::return_(42)
                }))
            }
        });

        let d = xs.subscribe_next(|_| {});

        e.wait_one();
        d.dispose();

        assert!(!called.get());
    }
}

// Not ported: DeferAsync_ArgumentChecking - argument validation
