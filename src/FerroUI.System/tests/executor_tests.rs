//! Tests of `task`: the executor of the crate (steps A1, A2 and A5 of phase C4 of #423,
//! docs/idiomatic-rust-async.md, section 3.1): spawning and polling, the reentrancy rule, wakes
//! from other threads, schedules, the current schedule, aborts, panics, and the handles of tasks.

use crate::task::{
    block_on, current_schedule, is_current_schedule, poll_now, run_pending, set_current_schedule, set_thread_schedule, spawn_local, spawn_local_on,
    spawn_local_on_result, spawn_on, thread_schedule, try_id, AbortHandle, FirstPoll, JoinError, JoinHandle, Schedule,
};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::future::Future;
use std::rc::Rc;
use std::sync::{mpsc, Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread::{self, ThreadId};

/// A future that is pending until the gate is opened, from any thread.
#[derive(Clone, Default)]
struct Gate {
    state: Arc<Mutex<GateState>>,
}

#[derive(Default)]
struct GateState {
    open: bool,
    waker: Option<Waker>,
}

impl Gate {
    fn open(&self) {
        let waker = {
            let mut state = self.state.lock().unwrap();
            state.open = true;
            state.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    fn wait(&self) -> impl Future<Output = ()> {
        let gate = self.clone();
        std::future::poll_fn(move |context| {
            let mut state = gate.state.lock().unwrap();
            if state.open {
                Poll::Ready(())
            } else {
                state.waker = Some(context.waker().clone());
                Poll::Pending
            }
        })
    }
}

/// A schedule that queues the polls it is given, run when the test asks.
#[derive(Default)]
struct QueueSchedule {
    polls: Mutex<VecDeque<Box<dyn FnOnce() + Send>>>,
}

impl Schedule for QueueSchedule {
    fn schedule(&self, poll: Box<dyn FnOnce() + Send>) {
        self.polls.lock().unwrap().push_back(poll);
    }
}

impl QueueSchedule {
    fn len(&self) -> usize {
        self.polls.lock().unwrap().len()
    }

    /// Runs the queued polls, the ones queued while they run included, in order.
    fn run_all(&self) {
        loop {
            let poll = self.polls.lock().unwrap().pop_front();
            match poll {
                Some(poll) => poll(),
                None => return,
            }
        }
    }
}

/// Sets its flag when it is dropped.
struct DropFlag(Rc<Cell<bool>>);

impl Drop for DropFlag {
    fn drop(&mut self) {
        self.0.set(true);
    }
}

/// Polls `future` once, with a waker that does nothing.
fn now_or_never<F: Future>(future: F) -> Option<F::Output> {
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(output) => Some(output),
        Poll::Pending => None,
    }
}

/// Runs `test` on a new thread, whose queue and schedule no other test sees, and waits for it.
fn on_own_thread(test: impl FnOnce() + Send + 'static) {
    thread::spawn(test).join().unwrap();
}

fn explode() -> i32 {
    panic!("boom")
}

fn describe(outcome: &Result<i32, JoinError>) -> String {
    match outcome {
        Ok(value) => format!("ok {value}"),
        Err(JoinError::Canceled) => "canceled".to_string(),
        Err(JoinError::Panicked(payload)) => format!("panicked: {}", payload.downcast_ref::<String>().cloned().unwrap_or_default()),
    }
}

#[test]
fn a_spawned_future_runs_to_its_first_await_and_its_handle_gives_the_output() {
    let gate = Gate::default();
    let steps = Rc::new(Cell::new(0));
    let (g, s) = (gate.clone(), steps.clone());
    let mut handle = spawn_local(async move {
        s.set(1);
        g.wait().await;
        s.set(2);
        42
    });
    assert_eq!(steps.get(), 1);
    assert!(!handle.is_finished());
    assert!(now_or_never(&mut handle).is_none());

    // A wake on the task's thread polls it inside the call that wakes it.
    gate.open();
    assert_eq!(steps.get(), 2);
    assert!(handle.is_finished());
    assert_eq!(now_or_never(&mut handle).unwrap().unwrap(), 42);
}

#[test]
fn a_wake_from_inside_a_poll_polls_the_task_again_after_that_poll() {
    let log = Rc::new(RefCell::new(Vec::<String>::new()));
    let depth = Rc::new(Cell::new(0));
    let (l, d) = (log.clone(), depth.clone());
    let mut polls = 0;
    let handle = spawn_local(std::future::poll_fn(move |context| {
        d.set(d.get() + 1);
        polls += 1;
        l.borrow_mut().push(format!("poll {polls} at depth {}", d.get()));
        if polls < 3 {
            context.waker().wake_by_ref();
            l.borrow_mut().push(format!("woke in poll {polls}"));
        }
        d.set(d.get() - 1);
        if polls < 3 {
            Poll::Pending
        } else {
            Poll::Ready(polls)
        }
    }));
    assert!(handle.is_finished());
    assert_eq!(*log.borrow(), ["poll 1 at depth 1", "woke in poll 1", "poll 2 at depth 1", "woke in poll 2", "poll 3 at depth 1"]);
}

#[test]
fn a_wake_from_another_thread_waits_for_run_pending() {
    on_own_thread(|| {
        let gate = Gate::default();
        let g = gate.clone();
        let handle = spawn_local(async move {
            g.wait().await;
            "done"
        });
        let remote = gate.clone();
        thread::spawn(move || remote.open()).join().unwrap();

        // The wake waits in the queue of this thread.
        assert!(!handle.is_finished());
        assert!(run_pending());
        assert!(handle.is_finished());
        assert!(!run_pending());
    });
}

#[test]
fn a_wake_from_another_thread_goes_through_the_schedule_of_the_thread() {
    on_own_thread(|| {
        let schedule = Arc::new(QueueSchedule::default());
        set_thread_schedule(Some(schedule.clone()));
        let gate = Gate::default();
        let g = gate.clone();
        let handle = spawn_local(async move { g.wait().await });
        let remote = gate.clone();
        thread::spawn(move || remote.open()).join().unwrap();

        // The schedule of the thread is given one run of the queue, which polls the task.
        assert!(!handle.is_finished());
        assert_eq!(schedule.len(), 1);
        schedule.run_all();
        assert!(handle.is_finished());
        set_thread_schedule(None);
    });
}

#[test]
fn block_on_drives_a_future_that_spawns_others() {
    on_own_thread(|| {
        let order = Rc::new(RefCell::new(Vec::new()));
        let o = order.clone();
        let value = block_on(async move {
            let o1 = o.clone();
            let first = spawn_local(async move {
                o1.borrow_mut().push("first");
                20
            });
            // The second waits for a wake from another thread, which comes while `block_on` waits.
            let gate = Gate::default();
            let (g, o2) = (gate.clone(), o.clone());
            let second = spawn_local(async move {
                g.wait().await;
                o2.borrow_mut().push("second");
                22
            });
            let (release, released) = mpsc::channel::<()>();
            thread::spawn(move || {
                released.recv().unwrap();
                gate.open();
            });
            o.borrow_mut().push("spawned");
            release.send(()).unwrap();
            first.await.unwrap() + second.await.unwrap()
        });
        assert_eq!(value, 42);
        assert_eq!(*order.borrow(), ["first", "spawned", "second"]);
    });
}

#[test]
fn block_on_raises_the_panic_of_its_future_again() {
    on_own_thread(|| {
        let panic = std::panic::catch_unwind(|| block_on(async { explode() })).unwrap_err();
        assert_eq!(panic.downcast_ref::<&str>(), Some(&"boom"));
    });
}

#[test]
fn abort_drops_the_future_and_the_outcome_is_canceled() {
    let dropped = Rc::new(Cell::new(false));
    let guard = DropFlag(dropped.clone());
    let gate = Gate::default();
    let g = gate.clone();
    let mut handle = spawn_local(async move {
        let _guard = guard;
        g.wait().await;
        1
    });
    handle.abort();
    assert!(dropped.get());
    assert!(handle.is_finished());
    assert!(now_or_never(&mut handle).unwrap().unwrap_err().is_canceled());
    // Nothing is left to wake.
    gate.open();

    // The awaiters of an aborted task are woken with the cancellation.
    let shared = spawn_local(std::future::pending::<i32>()).shared();
    let s = shared.clone();
    let watcher = spawn_local(async move { describe(&s.await) });
    assert!(!watcher.is_finished());
    shared.abort();
    assert!(watcher.is_finished());
    assert_eq!(now_or_never(watcher).unwrap().unwrap(), "canceled");
}

#[test]
fn an_abort_from_another_thread_drops_the_future_on_its_thread() {
    struct RecordDrop(Arc<Mutex<Option<ThreadId>>>);

    impl Drop for RecordDrop {
        fn drop(&mut self) {
            *self.0.lock().unwrap() = Some(thread::current().id());
        }
    }

    on_own_thread(|| {
        let dropped_on = Arc::new(Mutex::new(None));
        let record = RecordDrop(dropped_on.clone());
        let handle = spawn_local(async move {
            let _record = record;
            std::future::pending::<()>().await;
        });
        let handle = thread::spawn(move || {
            handle.abort();
            handle
        })
        .join()
        .unwrap();

        // The future belongs to this thread: it is dropped when this thread runs its queue.
        assert!(dropped_on.lock().unwrap().is_none());
        assert!(!handle.is_finished());
        assert!(run_pending());
        assert_eq!(*dropped_on.lock().unwrap(), Some(thread::current().id()));
        assert!(handle.is_finished());
    });
}

#[test]
fn a_panic_faults_the_task_and_wakes_its_awaiters() {
    let gate = Gate::default();
    let g = gate.clone();
    let shared = spawn_local(async move {
        g.wait().await;
        explode()
    })
    .shared();
    let log = Rc::new(RefCell::new(Vec::<String>::new()));
    let l = log.clone();
    shared.on_finished(move |outcome| l.borrow_mut().push(format!("continuation: {}", describe(outcome))));
    let awaiters: Vec<_> = ["first", "second"]
        .into_iter()
        .map(|name| {
            let (l, s) = (log.clone(), shared.clone());
            spawn_local(async move {
                let outcome = s.await;
                l.borrow_mut().push(format!("{name}: {}", describe(&outcome)));
            })
        })
        .collect();

    // The panic is caught in the poll that the wake runs: the call that wakes goes on.
    gate.open();
    assert_eq!(*log.borrow(), ["continuation: panicked: boom", "first: panicked: boom", "second: panicked: boom"]);
    assert!(awaiters.iter().all(JoinHandle::is_finished));
    let error = shared.outcome().unwrap().unwrap_err();
    assert!(error.is_panic());
    assert_eq!(error.to_string(), "the task panicked: boom");

    // The handle of one observer gives the payload of the panic itself.
    let mut handle = spawn_local(async { explode() });
    assert!(handle.is_finished());
    let payload = now_or_never(&mut handle).unwrap().unwrap_err().into_panic();
    assert_eq!(payload.downcast_ref::<&str>(), Some(&"boom"));
}

#[test]
fn dropping_the_handle_detaches_the_task() {
    let gate = Gate::default();
    let ran = Rc::new(Cell::new(false));
    let (g, r) = (gate.clone(), ran.clone());
    drop(spawn_local(async move {
        g.wait().await;
        r.set(true);
    }));
    assert!(!ran.get());
    gate.open();
    assert!(ran.get());
}

#[test]
fn shared_runs_its_continuations_before_it_wakes_its_awaiters() {
    let gate = Gate::default();
    let g = gate.clone();
    let shared = spawn_local(async move {
        g.wait().await;
        5
    })
    .shared();
    let log = Rc::new(RefCell::new(Vec::<String>::new()));
    let mut awaiters = Vec::new();
    for name in ["first", "second"] {
        let (l, s) = (log.clone(), shared.clone());
        awaiters.push(spawn_local(async move {
            let value = s.await.unwrap();
            l.borrow_mut().push(format!("{name} future: {value}"));
        }));
        let l = log.clone();
        shared.on_finished(move |outcome| l.borrow_mut().push(format!("{name} continuation: {}", outcome.as_ref().unwrap())));
    }

    log.borrow_mut().push("completing".to_string());
    gate.open();
    log.borrow_mut().push("completed".to_string());
    assert_eq!(
        *log.borrow(),
        ["completing", "first continuation: 5", "second continuation: 5", "first future: 5", "second future: 5", "completed"]
    );
    assert!(awaiters.iter().all(JoinHandle::is_finished));

    // Once finished, a continuation runs at once and an awaiter gets the outcome at once.
    let l = log.clone();
    shared.on_finished(move |outcome| l.borrow_mut().push(format!("late continuation: {}", outcome.as_ref().unwrap())));
    assert_eq!(log.borrow().last().unwrap(), "late continuation: 5");
    assert_eq!(now_or_never(shared.clone()).unwrap().unwrap(), 5);

    // The handle of a task that has ended gives a finished `Shared`.
    let ended = spawn_local(async { 3 }).shared();
    assert!(ended.is_finished());
    assert_eq!(ended.outcome().unwrap().unwrap(), 3);
}

#[test]
fn a_schedule_receives_the_polls_of_its_tasks_in_the_order_of_the_wakes() {
    let schedule = Arc::new(QueueSchedule::default());
    let log = Rc::new(RefCell::new(Vec::<String>::new()));
    let gates = [Gate::default(), Gate::default()];
    let handles: Vec<_> = (0..2)
        .map(|n| {
            let (g, l) = (gates[n].clone(), log.clone());
            spawn_local_on(
                async move {
                    l.borrow_mut().push(format!("{n}: first poll"));
                    g.wait().await;
                    l.borrow_mut().push(format!("{n}: resumed"));
                    n
                },
                schedule.clone(),
                FirstPoll::Scheduled,
            )
        })
        .collect();

    // Nothing runs inside the spawning calls: two polls are queued.
    assert!(log.borrow().is_empty());
    assert_eq!(schedule.len(), 2);
    schedule.run_all();
    assert_eq!(*log.borrow(), ["0: first poll", "1: first poll"]);

    // A wake queues a poll, also on the task's thread, and never polls inside the call that wakes.
    gates[1].open();
    gates[0].open();
    assert_eq!(log.borrow().len(), 2);
    assert_eq!(schedule.len(), 2);
    schedule.run_all();
    assert_eq!(log.borrow()[2..], ["1: resumed", "0: resumed"]);
    assert!(handles.iter().all(JoinHandle::is_finished));
}

#[test]
fn first_poll_now_polls_inside_the_spawning_call() {
    let schedule = Arc::new(QueueSchedule::default());
    let gate = Gate::default();
    let steps = Rc::new(Cell::new(0));
    let (g, s) = (gate.clone(), steps.clone());
    let handle = spawn_local_on(
        async move {
            s.set(1);
            g.wait().await;
            s.set(2);
        },
        schedule.clone(),
        FirstPoll::Now,
    );
    assert_eq!(steps.get(), 1);
    assert_eq!(schedule.len(), 0);

    gate.open();
    assert_eq!(steps.get(), 1);
    schedule.run_all();
    assert_eq!(steps.get(), 2);
    assert!(handle.is_finished());
}

#[test]
fn a_task_on_a_schedule_takes_its_outcome_from_the_result_of_its_future_without_a_panic() {
    // #567: a future ends its task with a `JoinError` value, which the handle gives as if the
    // task had panicked or been aborted, and nothing unwinds.
    let schedule = Arc::new(QueueSchedule::default());
    let ok = spawn_local_on_result(async { Ok::<_, JoinError>(5) }, schedule.clone(), FirstPoll::Now);
    assert_eq!(now_or_never(ok).unwrap().unwrap(), 5);

    let gate = Gate::default();
    let g = gate.clone();
    let failed = spawn_local_on_result(
        async move {
            g.wait().await;
            Err::<(), _>(JoinError::Panicked(Box::new("an animation could not be run".to_string())))
        },
        schedule.clone(),
        FirstPoll::Now,
    );
    assert!(!failed.is_finished());
    gate.open();
    schedule.run_all();
    assert!(failed.is_finished());
    let error = now_or_never(failed).unwrap().unwrap_err();
    assert!(error.is_panic());
    assert_eq!(error.into_panic().downcast_ref::<String>().unwrap(), "an animation could not be run");

    let canceled = spawn_local_on_result(async { Err::<(), _>(JoinError::Canceled) }, schedule.clone(), FirstPoll::Now);
    assert!(now_or_never(canceled).unwrap().unwrap_err().is_canceled());
    assert_eq!(schedule.len(), 0);
}

#[test]
fn a_task_on_a_schedule_has_one_queued_poll_at_a_time_and_wakes_from_other_threads_queue_there() {
    let schedule = Arc::new(QueueSchedule::default());
    let waker: Arc<Mutex<Option<Waker>>> = Arc::new(Mutex::new(None));
    let polls = Rc::new(Cell::new(0));
    let (w, p) = (waker.clone(), polls.clone());
    let handle = spawn_local_on(
        std::future::poll_fn(move |context| {
            p.set(p.get() + 1);
            *w.lock().unwrap() = Some(context.waker().clone());
            if p.get() == 3 {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }),
        schedule.clone(),
        FirstPoll::Now,
    );
    assert_eq!(polls.get(), 1);

    let task_waker = waker.lock().unwrap().clone().unwrap();
    task_waker.wake_by_ref();
    task_waker.wake_by_ref();
    assert_eq!(schedule.len(), 1);
    assert_eq!(polls.get(), 1);
    schedule.run_all();
    assert_eq!(polls.get(), 2);

    let remote = task_waker.clone();
    thread::spawn(move || remote.wake()).join().unwrap();
    assert_eq!(schedule.len(), 1);
    assert_eq!(polls.get(), 2);
    schedule.run_all();
    assert_eq!(polls.get(), 3);
    assert!(handle.is_finished());
}

#[test]
fn a_queued_poll_that_runs_inside_the_poll_of_its_task_schedules_the_task_again_after_it() {
    // A nested loop inside the poll (a pumping wait of a dispatcher) runs the poll that a wake
    // queued: it must not poll the task inside its own poll, nor lose the wake (F1).
    let schedule = Arc::new(QueueSchedule::default());
    let log = Rc::new(RefCell::new(Vec::<String>::new()));
    let depth = Rc::new(Cell::new(0));
    let (s, l, d) = (schedule.clone(), log.clone(), depth.clone());
    let mut polls = 0;
    let handle = spawn_local_on(
        std::future::poll_fn(move |context| {
            d.set(d.get() + 1);
            polls += 1;
            l.borrow_mut().push(format!("poll {polls} at depth {}", d.get()));
            if polls == 1 {
                context.waker().wake_by_ref();
                l.borrow_mut().push(format!("queued {}", s.len()));
                s.run_all();
                l.borrow_mut().push("nested loop ran".to_string());
            }
            d.set(d.get() - 1);
            if polls == 2 {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }),
        schedule.clone(),
        FirstPoll::Now,
    );
    assert_eq!(*log.borrow(), ["poll 1 at depth 1", "queued 1", "nested loop ran"]);
    assert_eq!(schedule.len(), 1);
    schedule.run_all();
    assert_eq!(log.borrow()[3], "poll 2 at depth 1");
    assert!(handle.is_finished());
}

#[test]
fn a_closure_is_a_schedule() {
    let queue: Arc<Mutex<Vec<Box<dyn FnOnce() + Send>>>> = Arc::new(Mutex::new(Vec::new()));
    let q = queue.clone();
    let schedule: Arc<dyn Schedule> = Arc::new(move |poll: Box<dyn FnOnce() + Send>| q.lock().unwrap().push(poll));
    let handle = spawn_local_on(async { 7 }, schedule, FirstPoll::Scheduled);
    assert!(!handle.is_finished());
    let polls = std::mem::take(&mut *queue.lock().unwrap());
    assert_eq!(polls.len(), 1);
    for poll in polls {
        poll();
    }
    assert!(handle.is_finished());
}

/// A schedule that keeps the abort handles of its tasks, as a dispatcher does.
#[derive(Default)]
struct TrackingSchedule {
    queue: QueueSchedule,
    spawned: Mutex<Vec<AbortHandle>>,
}

impl Schedule for TrackingSchedule {
    fn schedule(&self, poll: Box<dyn FnOnce() + Send>) {
        self.queue.schedule(poll);
    }

    fn spawned(&self, task: AbortHandle) {
        self.spawned.lock().unwrap().push(task);
    }
}

/// A schedule that aborts every task it is given.
struct Refusing;

impl Schedule for Refusing {
    fn schedule(&self, _poll: Box<dyn FnOnce() + Send>) {}

    fn spawned(&self, task: AbortHandle) {
        task.abort();
    }
}

#[test]
fn a_schedule_is_told_of_its_tasks_and_can_abort_them() {
    let schedule = Arc::new(TrackingSchedule::default());
    let dropped = Rc::new(Cell::new(false));
    let flag = DropFlag(dropped.clone());
    let handle = spawn_local_on(
        async move {
            let _flag = flag;
            std::future::pending::<()>().await;
        },
        schedule.clone(),
        FirstPoll::Now,
    );
    let tasks = schedule.spawned.lock().unwrap().clone();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].id(), handle.id());
    assert!(!tasks[0].is_finished());
    tasks[0].abort();
    assert!(dropped.get());
    assert!(tasks[0].is_finished());
    assert!(matches!(now_or_never(handle).unwrap(), Err(ref error) if error.is_canceled()));

    // Aborted when the schedule is told of it: the future is never polled.
    let refused = spawn_local_on(async { explode() }, Arc::new(Refusing), FirstPoll::Now);
    assert_eq!(describe(&now_or_never(refused).unwrap()), "canceled");
}

#[test]
fn spawn_on_makes_the_future_on_the_thread_of_the_schedule_and_polls_it_there() {
    let schedule = Arc::new(TrackingSchedule::default());
    let s = schedule.clone();
    let handle = thread::spawn(move || {
        spawn_on(
            || {
                // Not `Send`: made on the thread that runs the work of the schedule.
                let made_on = Rc::new(thread::current().id());
                async move { (*made_on, try_id()) }
            },
            s,
        )
    })
    .join()
    .unwrap();
    assert!(!handle.is_finished());
    assert_eq!(schedule.queue.len(), 1);
    assert!(schedule.spawned.lock().unwrap().is_empty());

    // The closure and the first poll run in that one piece of work.
    schedule.queue.run_all();
    let id = handle.id();
    assert!(handle.is_finished());
    assert_eq!(schedule.spawned.lock().unwrap().iter().map(AbortHandle::id).collect::<Vec<_>>(), [id]);
    assert_eq!(now_or_never(handle).unwrap().unwrap(), (thread::current().id(), Some(id)));
}

#[test]
fn a_task_of_spawn_on_is_canceled_when_its_first_work_is_dropped_or_it_is_aborted_before() {
    let schedule = Arc::new(QueueSchedule::default());
    let dropped = spawn_on(|| async { 1 }, schedule.clone());
    let aborted = spawn_on(|| async { 2 }, schedule.clone());
    aborted.abort();
    assert!(!aborted.is_finished());
    assert_eq!(schedule.len(), 2);

    // A schedule that drops the work (a dispatcher that has shut down) cancels the task.
    let mut polls = std::mem::take(&mut *schedule.polls.lock().unwrap());
    drop(polls.pop_front());
    assert_eq!(describe(&now_or_never(dropped).unwrap()), "canceled");
    for poll in polls {
        poll();
    }
    assert_eq!(describe(&now_or_never(aborted).unwrap()), "canceled");

    // A panic of the closure faults the task.
    let panicked = spawn_on(explode_closure, schedule.clone());
    schedule.run_all();
    let error = now_or_never(panicked).unwrap().unwrap_err();
    assert_eq!(error.to_string(), "the task panicked: boom");
}

fn explode_closure() -> std::future::Ready<i32> {
    panic!("boom")
}

#[test]
fn try_id_is_the_task_being_polled_and_poll_now_polls_a_task_inside_the_caller() {
    assert!(try_id().is_none());
    let schedule = Arc::new(QueueSchedule::default());
    let seen = Rc::new(RefCell::new(Vec::new()));
    let gate = Gate::default();
    let (s, g) = (seen.clone(), gate.clone());
    let handle = spawn_local_on(
        async move {
            s.borrow_mut().push(try_id());
            g.wait().await;
            s.borrow_mut().push(try_id());
        },
        schedule.clone(),
        FirstPoll::Now,
    );
    let id = handle.id();
    assert_eq!(*seen.borrow(), [Some(id)]);
    assert!(try_id().is_none());

    // The wake queues a poll; `poll_now` polls the task inside this call instead.
    gate.open();
    assert_eq!(schedule.len(), 1);
    assert!(poll_now(id));
    assert!(handle.is_finished());
    assert_eq!(*seen.borrow(), [Some(id), Some(id)]);
    // The queued poll finds no task, and neither does `poll_now`.
    schedule.run_all();
    assert!(!poll_now(id));

    // Inside its own poll, the task is marked and polled again through its schedule.
    let mut polls = 0;
    let handle = spawn_local_on(
        std::future::poll_fn(move |_| {
            polls += 1;
            if polls == 1 {
                assert!(poll_now(try_id().unwrap()));
                Poll::Pending
            } else {
                Poll::Ready(())
            }
        }),
        schedule.clone(),
        FirstPoll::Now,
    );
    assert!(!handle.is_finished());
    assert_eq!(schedule.len(), 1);
    schedule.run_all();
    assert!(handle.is_finished());
}

// ---------------------------------------------------------------------------
// The current schedule (A5): the schedule of the work that runs, which a task spawned in it takes.
// ---------------------------------------------------------------------------

#[test]
fn a_task_spawned_in_the_work_of_a_schedule_resumes_inline_while_it_is_current_and_queued_otherwise() {
    let schedule = Arc::new(QueueSchedule::default());
    let as_schedule: Arc<dyn Schedule> = schedule.clone();
    let other: Arc<dyn Schedule> = Arc::new(QueueSchedule::default());
    let previous = set_current_schedule(Some(as_schedule.clone()));
    let (first, second) = (Gate::default(), Gate::default());
    let log = Rc::new(RefCell::new(Vec::new()));
    let (l, f, s) = (log.clone(), first.clone(), second.clone());
    let handle = spawn_local(async move {
        f.wait().await;
        l.borrow_mut().push("after the first gate");
        s.wait().await;
        l.borrow_mut().push("after the second gate");
    });

    // The schedule the task was spawned in is current: the wake polls it inside the call.
    first.open();
    assert_eq!(*log.borrow(), ["after the first gate"]);
    assert_eq!(schedule.len(), 0);

    // Another schedule is current: the wake queues one poll on the task's schedule.
    drop(set_current_schedule(Some(other)));
    second.open();
    assert_eq!(*log.borrow(), ["after the first gate"]);
    assert_eq!(schedule.len(), 1);
    schedule.run_all();
    assert_eq!(*log.borrow(), ["after the first gate", "after the second gate"]);
    assert!(handle.is_finished());
    drop(set_current_schedule(previous));
}

#[test]
fn a_task_spawned_in_the_work_of_a_schedule_is_woken_from_another_thread_through_that_schedule() {
    let schedule = Arc::new(QueueSchedule::default());
    let previous = set_current_schedule(Some(schedule.clone() as Arc<dyn Schedule>));
    let gate = Gate::default();
    let done = Rc::new(Cell::new(false));
    let (g, d) = (gate.clone(), done.clone());
    let handle = spawn_local(async move {
        g.wait().await;
        d.set(true);
    });

    let opener = gate.clone();
    thread::spawn(move || opener.open()).join().unwrap();
    assert!(!done.get());
    // The wake is a poll on the schedule, not an entry of the queue of the thread.
    assert!(!run_pending());
    assert_eq!(schedule.len(), 1);
    schedule.run_all();
    assert!(done.get());
    assert!(handle.is_finished());
    drop(set_current_schedule(previous));
}

#[test]
fn a_task_spawned_outside_of_the_work_of_a_schedule_stays_in_the_queue_of_the_thread() {
    // A schedule of the thread is not a current schedule: the task resumes inside the waking call
    // wherever the wake comes from on this thread, as before.
    let schedule = Arc::new(QueueSchedule::default());
    set_thread_schedule(Some(schedule.clone()));
    assert!(current_schedule().is_none());
    assert!(thread_schedule().is_some());
    let gate = Gate::default();
    let done = Rc::new(Cell::new(false));
    let (g, d) = (gate.clone(), done.clone());
    let handle = spawn_local(async move {
        g.wait().await;
        d.set(true);
    });
    let other: Arc<dyn Schedule> = Arc::new(QueueSchedule::default());
    let previous = set_current_schedule(Some(other));
    gate.open();
    assert!(done.get());
    assert!(handle.is_finished());
    assert_eq!(schedule.len(), 0);
    drop(set_current_schedule(previous));
    set_thread_schedule(None);
    assert!(thread_schedule().is_none());
}

#[test]
fn block_on_runs_its_future_with_no_current_schedule() {
    let schedule: Arc<dyn Schedule> = Arc::new(QueueSchedule::default());
    let previous = set_current_schedule(Some(schedule.clone()));
    let inside = block_on(async { current_schedule().is_none() });
    assert!(inside);
    assert!(is_current_schedule(&schedule));
    drop(set_current_schedule(previous));
    assert!(!is_current_schedule(&schedule));
}

#[test]
fn work_that_is_not_send_is_scheduled_through_a_callback_of_its_thread() {
    let schedule = QueueSchedule::default();
    let ran = Rc::new(Cell::new(0));
    let r = ran.clone();
    schedule.schedule_local(Box::new(move || r.set(r.get() + 1)));
    assert_eq!(ran.get(), 0);
    assert_eq!(schedule.len(), 1);
    schedule.run_all();
    assert_eq!(ran.get(), 1);

    // Work that the schedule drops without running it is forgotten, and never runs.
    let r = ran.clone();
    schedule.schedule_local(Box::new(move || r.set(r.get() + 10)));
    let dropped: Vec<_> = schedule.polls.lock().unwrap().drain(..).collect();
    drop(dropped);
    assert!(!run_pending());
    assert_eq!(ran.get(), 1);
}
