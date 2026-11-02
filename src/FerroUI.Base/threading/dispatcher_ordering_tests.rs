//! Characterization tests of the ordering guarantees of the dispatcher: step A0 of phase C4 of
//! #423 (docs/idiomatic-rust-async.md, section 2). They pin what the dispatcher does today, so
//! that replacing `Task`, `TaskCompletionSource` and `SynchronizationContext` with futures on the
//! dispatcher's executor keeps it, or changes it on purpose with a DEVIATIONS row and a new test.
//!
//! A child module of `dispatcher_tests`, whose simulated platform implementation, helpers and
//! services it uses.

use super::*;
use crate::threading::DispatcherProcessingDisabled;
use ferroui_system::threading::tasks::{spawn_local as spawn_system_task, TaskCompletionSource};
use ferroui_system::ObservableError;

/// What the callbacks of a test did, in order; shared by local and `Send` callbacks.
#[derive(Clone, Default)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
    fn new() -> Self {
        Self::default()
    }

    fn push(&self, entry: impl Into<String>) {
        self.0.lock().unwrap().push(entry.into());
    }

    /// A callback that writes `entry`.
    fn add(&self, entry: &'static str) -> impl FnOnce() + Send + 'static {
        let log = self.clone();
        move || log.push(entry)
    }

    fn entries(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

// ---------------------------------------------------------------------------
// The queue: priorities and the order of arrival.
// ---------------------------------------------------------------------------

#[test]
fn jobs_of_one_priority_run_in_the_order_they_were_queued_whatever_queued_them() {
    let t = DispatcherTests::new();
    let log = Log::new();
    t.ui_thread.post_local(log.add("post_local 1"), DispatcherPriority::NORMAL);
    t.ui_thread.post(log.add("post 2"), DispatcherPriority::NORMAL);
    let local =
        t.ui_thread.invoke_async_local_with_priority(log.add("invoke_async_local 3"), DispatcherPriority::NORMAL);
    let send = t.ui_thread.invoke_async_with_priority(log.add("invoke_async 4"), DispatcherPriority::NORMAL);
    t.ui_thread.post_local(log.add("post_local 5"), DispatcherPriority::NORMAL);

    // Nothing that is queued runs inside the call that queues it.
    assert!(log.entries().is_empty());
    assert_eq!(local.status(), DispatcherOperationStatus::Pending);

    t.ui_thread.run_jobs(None);
    assert_eq!(log.entries(), ["post_local 1", "post 2", "invoke_async_local 3", "invoke_async 4", "post_local 5"]);
    assert_eq!(local.status(), DispatcherOperationStatus::Completed);
    assert_eq!(send.status(), DispatcherOperationStatus::Completed);
}

#[test]
fn the_highest_priority_runs_first_also_for_jobs_queued_while_the_queue_runs() {
    let t = DispatcherTests::new();
    let log = Log::new();
    let d = t.ui_thread.clone();
    let l = log.clone();
    t.ui_thread.post_local(
        move || {
            l.push("A (Default)");
            d.post_local(l.add("D (Normal, queued by A)"), DispatcherPriority::NORMAL);
            d.post_local(l.add("E (Default, queued by A)"), DispatcherPriority::DEFAULT);
        },
        DispatcherPriority::DEFAULT,
    );
    t.ui_thread.post_local(log.add("B (Background)"), DispatcherPriority::BACKGROUND);
    t.ui_thread.post_local(log.add("C (Default)"), DispatcherPriority::DEFAULT);
    // An asynchronous invoke at Send priority is queued, not run inline, even on the dispatcher
    // thread; it is the highest priority, so it runs first.
    let send = t.ui_thread.invoke_async_with_priority(log.add("S (Send)"), DispatcherPriority::SEND);
    assert_eq!(send.status(), DispatcherOperationStatus::Pending);
    assert!(log.entries().is_empty());

    t.ui_thread.run_jobs(None);
    assert_eq!(
        log.entries(),
        [
            "S (Send)",
            "A (Default)",
            "D (Normal, queued by A)",
            "C (Default)",
            "E (Default, queued by A)",
            "B (Background)"
        ]
    );
}

#[test]
fn work_queued_from_another_thread_joins_the_same_queue_in_the_order_of_arrival() {
    let t = DispatcherTests::new();
    let log = Log::new();
    t.ui_thread.post_local(log.add("dispatcher thread 1"), DispatcherPriority::NORMAL);
    let d = t.ui_thread.clone();
    let l = log.clone();
    let (first, last) = thread::spawn(move || {
        let first = d.invoke_async_with_priority(l.add("other thread 2"), DispatcherPriority::NORMAL);
        d.post(l.add("other thread 3"), DispatcherPriority::NORMAL);
        let last = d.invoke_async_with_priority(l.add("other thread 4"), DispatcherPriority::NORMAL);
        (first, last)
    })
    .join()
    .unwrap();
    t.ui_thread.post_local(log.add("dispatcher thread 5"), DispatcherPriority::NORMAL);
    assert!(log.entries().is_empty());

    t.ui_thread.run_jobs(None);
    assert_eq!(
        log.entries(),
        ["dispatcher thread 1", "other thread 2", "other thread 3", "other thread 4", "dispatcher thread 5"]
    );
    assert_eq!(first.status(), DispatcherOperationStatus::Completed);
    assert_eq!(last.status(), DispatcherOperationStatus::Completed);
}

#[test]
fn a_synchronous_invoke_from_another_thread_is_queued_at_send_priority_and_waits_for_the_queue() {
    let t = DispatcherTests::new();
    let log = Log::new();
    t.ui_thread.post_local(log.add("normal, queued first"), DispatcherPriority::NORMAL);
    let d = t.ui_thread.clone();
    let l = log.clone();
    let worker = thread::spawn(move || {
        d.invoke(move || {
            l.push("invoked from another thread");
            thread::current().id()
        })
    });

    let deadline = Instant::now() + Duration::from_secs(10);
    while !t.ui_thread.has_jobs_with_priority(DispatcherPriority::SEND) {
        assert!(Instant::now() < deadline, "the invoke of the other thread was not queued");
        thread::yield_now();
    }
    // Queued, and not run until the dispatcher thread runs its jobs.
    assert!(log.entries().is_empty());

    t.ui_thread.run_jobs(None);
    assert_eq!(worker.join().unwrap(), Ok(thread::current().id()));
    assert_eq!(log.entries(), ["invoked from another thread", "normal, queued first"]);
}

#[test]
fn waiting_on_the_dispatcher_thread_runs_only_the_jobs_ahead_of_the_operation() {
    let t = DispatcherTests::new();
    // A platform with run loops: the wait for a foreground operation pumps the queue down to the
    // priority of the operation and stops when it has run.
    Dispatcher::initialize_ui_thread_dispatcher(SimpleDispatcherImpl::new(SimpleKind::Controlled));
    let log = Log::new();
    t.ui_thread.post_local(log.add("normal, before"), DispatcherPriority::NORMAL);
    t.ui_thread.post_local(log.add("background"), DispatcherPriority::BACKGROUND);
    t.ui_thread.post_local(log.add("send"), DispatcherPriority::SEND);
    let waited = t.ui_thread.invoke_async_local_with_priority(log.add("waited for"), DispatcherPriority::NORMAL);
    t.ui_thread.post_local(log.add("normal, after"), DispatcherPriority::NORMAL);

    waited.wait().unwrap();
    assert_eq!(log.entries(), ["send", "normal, before", "waited for"]);

    t.ui_thread.run_jobs(None);
    assert_eq!(log.entries(), ["send", "normal, before", "waited for", "normal, after", "background"]);
}

// ---------------------------------------------------------------------------
// Futures on the dispatcher: where and when a continuation runs.
// ---------------------------------------------------------------------------

#[test]
fn a_task_awaiting_an_operation_resumes_in_a_job_queued_at_its_priority_when_the_operation_completes() {
    let t = DispatcherTests::new();
    let log = Log::new();
    let operation = t.ui_thread.invoke_async_local_with_priority(log.add("operation"), DispatcherPriority::NORMAL);
    let l = log.clone();
    let task = t.ui_thread.to_task_scheduler_with_priority(DispatcherPriority::NORMAL).start_local(async move {
        l.push("task started");
        operation.await.unwrap();
        l.push("task resumed");
    });
    t.ui_thread.post_local(log.add("queued after the operation"), DispatcherPriority::NORMAL);
    // `start_local` polls the future up to its first pending await before it returns.
    assert_eq!(log.entries(), ["task started"]);

    t.ui_thread.run_jobs(None);
    // The completion does not resume the task inline: it queues a poll at the task's priority,
    // behind the work that was queued before the operation completed.
    assert_eq!(log.entries(), ["task started", "operation", "queued after the operation", "task resumed"]);
    assert!(task.is_completed_successfully());
}

#[test]
fn tasks_woken_together_resume_in_the_order_they_awaited_each_at_its_own_priority() {
    let t = DispatcherTests::new();
    let log = Log::new();
    let operation = t.ui_thread.invoke_async_local_with_priority(log.add("operation"), DispatcherPriority::NORMAL);
    let mut tasks = Vec::new();
    for (name, priority) in [
        ("t1", DispatcherPriority::BACKGROUND),
        ("t2", DispatcherPriority::NORMAL),
        ("t3", DispatcherPriority::NORMAL),
    ] {
        let (l, awaited) = (log.clone(), operation.clone());
        tasks.push(t.ui_thread.to_task_scheduler_with_priority(priority).start_local(async move {
            awaited.await.unwrap();
            // The current context of a poll is the one of the task's priority.
            l.push(format!("{name} resumed at {}", current_priority()));
        }));
    }
    t.ui_thread.post_local(log.add("queued after the operation"), DispatcherPriority::NORMAL);

    t.ui_thread.run_jobs(None);
    assert_eq!(
        log.entries(),
        [
            "operation",
            "queued after the operation",
            "t2 resumed at Normal",
            "t3 resumed at Normal",
            "t1 resumed at Background"
        ]
    );
    assert!(tasks.iter().all(DispatcherTask::is_completed_successfully));
}

#[test]
fn a_resumption_at_another_priority_changes_the_priority_of_one_continuation_only() {
    let t = DispatcherTests::new();
    let log = Log::new();
    let d = t.ui_thread.clone();
    let l = log.clone();
    let task = t.ui_thread.to_task_scheduler_with_priority(DispatcherPriority::BACKGROUND).start_local(async move {
        d.resume_with_priority(DispatcherPriority::SEND).await;
        l.push(format!("after resume_with_priority: {}", current_priority()));
        let operation = d.invoke_async_local_with_priority(|| {}, DispatcherPriority::NORMAL);
        operation.await.unwrap();
        l.push(format!("after an awaited operation: {}", current_priority()));
    });

    t.ui_thread.run_jobs(None);
    assert_eq!(log.entries(), ["after resume_with_priority: Send", "after an awaited operation: Background"]);
    assert!(task.is_completed_successfully());
}

#[test]
fn a_completion_source_runs_its_continuations_and_system_tasks_inline_and_queues_dispatcher_tasks() {
    // The two executors of a dispatcher thread today: a task of `ferroui-system`
    // (`tasks::spawn_local`) is polled again inside the call that wakes it, a task of the
    // dispatcher in a job queued at its priority. Continuations (`continue_with`) run first, then
    // the awaiting futures are woken in the order they awaited.
    let t = DispatcherTests::new();
    let log = Log::new();
    let source = TaskCompletionSource::<i32>::new();

    let (l, awaited) = (log.clone(), source.task());
    let system_task = spawn_system_task(async move {
        let value = awaited.await?;
        l.push(format!("system task resumed with {value}"));
        Ok::<_, ObservableError>(())
    });
    let (l, awaited) = (log.clone(), source.task());
    let dispatcher_task =
        t.ui_thread.to_task_scheduler_with_priority(DispatcherPriority::NORMAL).start_local(async move {
            let value = awaited.await.unwrap();
            l.push(format!("dispatcher task resumed with {value}"));
        });
    let l = log.clone();
    let _ = source.task().continue_with(move |task| l.push(format!("continuation saw {}", task.result().unwrap())));

    let (l, completing) = (log.clone(), source.clone());
    t.ui_thread.post_local(
        move || {
            l.push("completing");
            completing.set_result(7);
            l.push("completed");
        },
        DispatcherPriority::NORMAL,
    );
    t.ui_thread.post_local(log.add("queued after the completing job"), DispatcherPriority::NORMAL);

    t.ui_thread.run_jobs(None);
    assert_eq!(
        log.entries(),
        [
            "completing",
            "continuation saw 7",
            "system task resumed with 7",
            "completed",
            "queued after the completing job",
            "dispatcher task resumed with 7"
        ]
    );
    assert!(system_task.is_completed_successfully());
    assert!(dispatcher_task.is_completed_successfully());
}

// ---------------------------------------------------------------------------
// Nested frames and disabled processing.
// ---------------------------------------------------------------------------

#[test]
fn a_nested_frame_runs_the_queue_in_priority_order_before_the_job_that_pushed_it_continues() {
    let _t = DispatcherTests::new();
    let _services = DispatcherServices::new(SimpleDispatcherImpl::new(SimpleKind::Controlled));
    let disp = Dispatcher::ui_thread();
    let log = Log::new();
    let outer = CancellationToken::new();
    let nested = DispatcherFrame::new();
    {
        let (l, d, frame, outer) = (log.clone(), disp.clone(), nested.clone(), outer.clone());
        disp.post_local(
            move || {
                l.push("J starts (Normal)");
                d.post_local(l.add("X (Default, queued by J)"), DispatcherPriority::DEFAULT);
                d.push_frame(&frame);
                l.push("J ends");
                outer.cancel();
            },
            DispatcherPriority::NORMAL,
        );
    }
    disp.post_local(log.add("C (Normal)"), DispatcherPriority::NORMAL);
    {
        let (l, frame) = (log.clone(), nested.clone());
        disp.post_local(
            move || {
                l.push("B (Background) exits the nested frame");
                frame.set_continue(false);
            },
            DispatcherPriority::BACKGROUND,
        );
    }

    disp.main_loop(&outer);
    // Inside the frame the queue runs as it does outside: the highest priority first, the order of
    // arrival within a priority, background work included; the job that pushed the frame
    // continues when the frame exits.
    assert_eq!(
        log.entries(),
        [
            "J starts (Normal)",
            "C (Normal)",
            "X (Default, queued by J)",
            "B (Background) exits the nested frame",
            "J ends"
        ]
    );
}

#[test]
fn work_queued_while_processing_is_disabled_waits_and_keeps_its_order() {
    let t = DispatcherTests::new();
    let impl_ = SimpleDispatcherImpl::new(SimpleKind::Simple);
    Dispatcher::initialize_ui_thread_dispatcher(impl_.clone());
    let log = Log::new();
    t.ui_thread.post_local(log.add("before"), DispatcherPriority::NORMAL);
    let disabled: DispatcherProcessingDisabled = t.ui_thread.disable_processing();
    t.ui_thread.post_local(log.add("while disabled, background"), DispatcherPriority::BACKGROUND);
    t.ui_thread.post_local(log.add("while disabled, normal"), DispatcherPriority::NORMAL);

    // Waiting on the dispatcher thread pumps the queue, which is refused while processing is
    // disabled; the operation stays queued.
    let waited = t.ui_thread.invoke_async_local_with_priority(log.add("waited for"), DispatcherPriority::NORMAL);
    let w = waited.clone();
    let result = catch_unwind(AssertUnwindSafe(|| w.wait()));
    assert_eq!(
        message_of(&*result.unwrap_err()),
        Some("Cannot perform this operation while dispatcher processing is suspended.")
    );
    assert_eq!(waited.status(), DispatcherOperationStatus::Pending);
    assert!(log.entries().is_empty());

    drop(disabled);
    t.ui_thread.run_jobs(None);
    assert_eq!(log.entries(), ["before", "while disabled, normal", "waited for", "while disabled, background"]);
}

#[test]
fn a_platform_signal_runs_jobs_while_processing_is_disabled() {
    // What upstream does as well: only `run_jobs`, `push_frame` and the waits that pump check the
    // counter; the signal of the platform does not (docs/idiomatic-rust-async.md, section 1.6).
    let t = DispatcherTests::new();
    let impl_ = SimpleDispatcherImpl::new(SimpleKind::Simple);
    Dispatcher::initialize_ui_thread_dispatcher(impl_.clone());
    let log = Log::new();
    let _disabled = t.ui_thread.disable_processing();
    t.ui_thread.post_local(log.add("posted while disabled"), DispatcherPriority::NORMAL);
    assert!(impl_.asked_for_signal());

    impl_.execute_signal();
    assert_eq!(log.entries(), ["posted while disabled"]);
}

// ---------------------------------------------------------------------------
// Timers.
// ---------------------------------------------------------------------------

#[test]
fn a_due_timer_runs_in_the_place_where_it_was_started_among_the_jobs_of_its_priority() {
    let _t = DispatcherTests::new();
    let impl_ = SimpleDispatcherImpl::new(SimpleKind::Simple);
    Dispatcher::reset_for_unit_tests();
    let dispatcher = Dispatcher::new(Some(impl_.clone()));
    impl_.set_now(100);
    let log = Log::new();

    dispatcher.post_local(log.add("posted before the timer started"), DispatcherPriority::NORMAL);
    let timer = DispatcherTimer::with_interval(Duration::from_millis(50), DispatcherPriority::NORMAL, &dispatcher);
    let l = log.clone();
    let _tick = timer.tick(move |timer| {
        l.push("tick");
        timer.stop();
    });
    timer.start();
    dispatcher.post_local(log.add("posted after the timer started"), DispatcherPriority::NORMAL);
    dispatcher.post_local(log.add("render"), DispatcherPriority::RENDER);

    // The timer's operation is queued (inactive) when the timer starts; when it is due it is
    // promoted to the timer's priority and keeps its place in the order of arrival.
    assert_eq!(impl_.next_timer(), Some(150));
    impl_.execute_timer();
    dispatcher.run_jobs(None);
    assert_eq!(
        log.entries(),
        ["posted before the timer started", "tick", "posted after the timer started", "render"]
    );
}

#[test]
fn a_timer_that_becomes_due_during_a_job_runs_before_the_next_job_of_its_priority() {
    let _t = DispatcherTests::new();
    let impl_ = SimpleDispatcherImpl::new(SimpleKind::Simple);
    Dispatcher::reset_for_unit_tests();
    let dispatcher = Dispatcher::new(Some(impl_.clone()));
    impl_.set_now(100);
    let log = Log::new();

    let timer = DispatcherTimer::with_interval(Duration::from_millis(50), DispatcherPriority::NORMAL, &dispatcher);
    let l = log.clone();
    let _tick = timer.tick(move |timer| {
        l.push("tick");
        timer.stop();
    });
    timer.start();
    let (l, clock) = (log.clone(), impl_.clone());
    dispatcher.post_local(
        move || {
            l.push("A moves the clock past the due time");
            clock.set_now(150);
        },
        DispatcherPriority::NORMAL,
    );
    dispatcher.post_local(log.add("B"), DispatcherPriority::NORMAL);

    // The due timers are promoted after every job, before the next one is chosen.
    dispatcher.run_jobs(None);
    assert_eq!(log.entries(), ["A moves the clock past the due time", "tick", "B"]);
}

// ---------------------------------------------------------------------------
// Cancellation and panics.
// ---------------------------------------------------------------------------

#[test]
fn a_canceled_operation_leaves_the_queue_and_its_awaiter_resumes_in_its_own_job() {
    let t = DispatcherTests::new();
    let log = Log::new();
    let source = CancellationToken::new();
    t.ui_thread.post_local(log.add("first"), DispatcherPriority::NORMAL);
    let canceled = t.ui_thread.invoke_async_local_with_cancellation(
        log.add("canceled"),
        DispatcherPriority::NORMAL,
        &source,
    );
    t.ui_thread.post_local(log.add("last"), DispatcherPriority::NORMAL);

    let aborted_on = Arc::new(Mutex::new(None));
    let a = aborted_on.clone();
    let _aborted = canceled.aborted(move || *a.lock().unwrap() = Some(thread::current().id()));
    let (l, awaited) = (log.clone(), canceled.clone());
    let task = t.ui_thread.to_task_scheduler_with_priority(DispatcherPriority::NORMAL).start_local(async move {
        let outcome = awaited.await;
        l.push(format!("the awaiting task saw {outcome:?}"));
    });

    let canceller = thread::spawn(move || {
        source.cancel();
        thread::current().id()
    })
    .join()
    .unwrap();
    // The operation is aborted inside `cancel`, and its `aborted` event runs on the thread that
    // canceled; the awaiting task is woken from there, which queues its poll.
    assert_eq!(canceled.status(), DispatcherOperationStatus::Aborted);
    assert_eq!(*aborted_on.lock().unwrap(), Some(canceller));
    assert!(log.entries().is_empty());

    t.ui_thread.run_jobs(None);
    assert_eq!(
        log.entries(),
        ["first", "last", "the awaiting task saw Err(Canceled)"]
    );
    assert_eq!(canceled.result(), Err(Canceled));
    assert!(task.is_completed_successfully());
}

#[test]
fn an_unhandled_panic_of_a_posted_job_leaves_the_rest_of_the_queue_in_order() {
    let t = DispatcherTests::new();
    let log = Log::new();
    t.ui_thread.post_local(log.add("first"), DispatcherPriority::NORMAL);
    t.ui_thread.post_local(|| panic!("the posted job panics"), DispatcherPriority::NORMAL);
    t.ui_thread.post_local(log.add("second"), DispatcherPriority::NORMAL);
    t.ui_thread.post_local(log.add("background"), DispatcherPriority::BACKGROUND);

    let d = t.ui_thread.clone();
    let result = catch_unwind(AssertUnwindSafe(|| d.run_jobs(None)));
    assert_eq!(message_of(&*result.unwrap_err()), Some("the posted job panics"));
    assert_eq!(log.entries(), ["first"]);

    t.ui_thread.run_jobs(None);
    assert_eq!(log.entries(), ["first", "second", "background"]);
}

#[test]
fn a_panic_of_an_awaited_operation_faults_the_awaiting_task_in_its_own_job() {
    let t = DispatcherTests::new();
    let log = Log::new();
    let operation = t
        .ui_thread
        .invoke_async_local_with_priority(|| -> i32 { panic!("the operation panics") }, DispatcherPriority::NORMAL);
    let l = log.clone();
    let task = t.ui_thread.to_task_scheduler_with_priority(DispatcherPriority::NORMAL).start_local(async move {
        l.push("awaiting");
        let _ = operation.await;
        l.push("not reached");
    });
    t.ui_thread.post_local(log.add("queued after the operation"), DispatcherPriority::NORMAL);

    // The panic of an `invoke_async` callback is kept for whoever observes the operation; the
    // queue goes on.
    t.ui_thread.run_jobs(None);
    assert_eq!(log.entries(), ["awaiting", "queued after the operation"]);
    assert!(task.is_faulted());
    let result = catch_unwind(AssertUnwindSafe(|| task.wait()));
    assert_eq!(message_of(&*result.unwrap_err()), Some("the operation panics"));
}

#[test]
fn the_completed_handlers_of_an_operation_run_on_the_dispatcher_thread_in_order_after_its_wakers() {
    let t = DispatcherTests::new();
    let log = Log::new();
    let dispatcher_thread = thread::current().id();
    let mut operation = t.ui_thread.invoke_async_with_priority(log.add("operation"), DispatcherPriority::NORMAL);

    let waker = CountingWaker::new();
    let l = log.clone();
    *waker.on_wake.lock().unwrap() = Some(Box::new(move || l.push("waker")));
    assert!(poll_once(&mut operation, &waker).is_pending());
    for name in ["completed 1", "completed 2"] {
        let l = log.clone();
        let _registration = operation.completed(move || {
            l.push(format!("{name} on the dispatcher thread: {}", thread::current().id() == dispatcher_thread));
        });
    }
    let (l, remote) = (log.clone(), operation.clone());
    thread::spawn(move || {
        let _registration = remote.completed(move || {
            l.push(format!(
                "completed 3 (added on another thread) on the dispatcher thread: {}",
                thread::current().id() == dispatcher_thread
            ));
        });
    })
    .join()
    .unwrap();

    t.ui_thread.run_jobs(None);
    assert_eq!(
        log.entries(),
        [
            "operation",
            "waker",
            "completed 1 on the dispatcher thread: true",
            "completed 2 on the dispatcher thread: true",
            "completed 3 (added on another thread) on the dispatcher thread: true"
        ]
    );
    assert_eq!(poll_once(&mut operation, &waker), Poll::Ready(Ok(())));
}
