//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/UsingAsyncTest.cs` of Rx.NET.
//! Adapted: `Using(Func<CancellationToken, Task<TResource>>, Func<TResource, CancellationToken,
//! Task<IObservable<TResult>>>)` is `Observable::using_async`. The work that
//! `Task.Factory.StartNew` runs on a worker thread is split: the worker (`tasks::run`) does what
//! upstream's work does apart from building the resource or the sequence, which is built on the
//! test thread when the work ends, as they are not `Send`. `ToEnumerable()` is `block_on` of
//! `to_list().to_task()`; `ManualResetEvent` and `CountdownEvent` are a local event of a mutex and
//! a condition variable and a flag; the resource is disposed on the test thread, so the loop that
//! waits for it under a lock is an assertion.

use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{IDisposable, Observable, ObservableError, ObservableExt};
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
fn using_async_simple() {
    let done = Rc::new(Cell::new(false));

    let signal = done.clone();
    let xs = Observable::using_async(
        move |_ct: CancellationToken| {
            let signal = signal.clone();
            async move {
                run(|| Ok(())).await?;
                Ok::<Rc<dyn IDisposable>, ObservableError>(Disposable::create(move || signal.set(true)))
            }
        },
        |_, _ct| async {
            let value = run(|| Ok(42)).await?;
            Ok::<_, ObservableError>(Observable::return_(value))
        },
    );

    let res = block_on(xs.to_list().to_task()).unwrap();

    assert_eq!(vec![42], res);
    assert!(done.get(), "done.Wait(5000)");
}

#[test]
fn using_async_cancel_resource() {
    let n = 10; // 0000;
    for _ in 0..n {
        let called = Rc::new(Cell::new(false));

        let s = ManualResetEvent::default();
        let e = ManualResetEvent::default();
        let x = ManualResetEvent::default();

        let events = (s.clone(), e.clone(), x.clone());
        let flag = called.clone();
        let xs = Observable::using_async(
            move |ct: CancellationToken| {
                let (s, e, x) = events.clone();
                async move {
                    run(move || {
                        s.set();
                        e.wait_one();
                        while !ct.is_cancellation_requested() {
                            std::hint::spin_loop();
                        }

                        x.set();
                        Ok(())
                    })
                    .await?;
                    Ok::<Rc<dyn IDisposable>, ObservableError>(Disposable::empty())
                }
            },
            move |_, _ct| {
                flag.set(true);
                async {
                    let value = run(|| Ok(42)).await?;
                    Ok::<_, ObservableError>(Observable::return_(value))
                }
            },
        );

        let d = xs.subscribe_next(|_| {});

        s.wait_one();
        d.dispose();

        e.set();
        x.wait_one();

        assert!(!called.get());
    }
}

#[test]
fn using_async_cancel_factory() {
    let n = 10; // 0000;
    for i in 0..n {
        let disposed = Rc::new(Cell::new(false));
        let called = Rc::new(Cell::new(false));

        let s = ManualResetEvent::default();
        let e = ManualResetEvent::default();
        let x = ManualResetEvent::default();

        let released = disposed.clone();
        let events = (s.clone(), e.clone(), x.clone());
        let flag = called.clone();
        let xs = Observable::using_async(
            move |_ct: CancellationToken| {
                let released = released.clone();
                async move {
                    run(|| Ok(())).await?;
                    Ok::<Rc<dyn IDisposable>, ObservableError>(Disposable::create(move || released.set(true)))
                }
            },
            move |_, ct: CancellationToken| {
                let (s, e, x) = events.clone();
                let called = flag.clone();
                async move {
                    run(move || {
                        s.set();
                        e.wait_one();
                        while !ct.is_cancellation_requested() {
                            std::hint::spin_loop();
                        }

                        x.set();
                        Ok(())
                    })
                    .await?;
                    Ok::<_, ObservableError>(Observable::defer(move || {
                        called.set(true);
                        Observable::return_(42)
                    }))
                }
            },
        );

        let d = xs.subscribe_next(|_| {});

        // `s.WaitOne()`: the resource arrives through the queue of the test thread, which `block_on`
        // runs while it waits.
        let started = s.clone();
        block_on(run(move || {
            started.wait_one();
            Ok(())
        }))
        .unwrap();

        //
        // This will *eventually* set the CancellationToken. There's a fundamental race between observing the CancellationToken
        // and returning the IDisposable that will set the CancellationTokenSource. Notice this is reflected in the code above,
        // by looping until the CancellationToken is set.
        //
        d.dispose();

        e.set();
        x.wait_one();

        assert!(disposed.get());

        assert!(!called.get(), "{i}");
    }
}

// Not ported: UsingAsync_ArgumentChecking - argument validation
