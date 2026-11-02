//! Tests of the dispatcher as the schedule of its thread: step A5 of phase C4 of #423
//! (docs/idiomatic-rust-async.md, sections 3.1 and 3.5, finding F3).
//!
//! A task of `ferroui-system` spawned in a job takes the dispatcher at the priority of the job: a
//! completion in a job of the same priority resumes it inside the completing call, any other
//! completion (in a job of another priority, outside of a job, from another thread) queues a job
//! at its priority. A task spawned outside of a job stays in the queue of the thread and resumes
//! inside the completing call on the thread (G9); its wakes from other threads, and the callbacks
//! that other threads send to the thread, are jobs at `Default`, without `run_pending_wakes`.
//!
//! A child module of `dispatcher_tests`, whose simulated platform implementation, helpers and
//! services it uses.

use super::*;
use ferroui_system::task::{current_schedule, is_current_schedule, thread_schedule};
use ferroui_system::threading::tasks::{run, spawn_local as spawn_system_task, Task, TaskCompletionSource};
use ferroui_system::ObservableError;

/// What the callbacks of a test did, in order.
#[derive(Clone, Default)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
    fn new() -> Self {
        Self::default()
    }

    fn push(&self, entry: impl Into<String>) {
        self.0.lock().unwrap().push(entry.into());
    }

    fn entries(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

/// Spawns a task of `ferroui-system` that awaits `awaited` and logs `name`, the value and the
/// priority it resumed at.
fn log_resumption(log: &Log, name: &'static str, awaited: Task<i32>) -> Task<()> {
    let log = log.clone();
    spawn_system_task(async move {
        let value = awaited.await?;
        log.push(format!("{name} resumed with {value} at {}", current_priority()));
        Ok::<_, ObservableError>(())
    })
}

/// Posts a job at `priority` that completes `source` with `value`, logging around it.
fn post_completion(dispatcher: &Dispatcher, log: &Log, source: &TaskCompletionSource<i32>, value: i32, priority: DispatcherPriority) {
    let (log, source) = (log.clone(), source.clone());
    dispatcher.post_local(
        move || {
            log.push(format!("completing at {priority}"));
            source.set_result(value);
            log.push("completed");
        },
        priority,
    );
}

// ---------------------------------------------------------------------------
// The current priority and the schedule of the thread.
// ---------------------------------------------------------------------------

#[test]
fn the_current_priority_is_the_one_of_the_running_job_and_none_outside_of_one() {
    let t = DispatcherTests::new();
    assert_eq!(Dispatcher::current_priority(), None);
    assert!(current_schedule().is_none());

    let seen = Rc::new(RefCell::new(Vec::new()));
    for priority in [DispatcherPriority::BACKGROUND, DispatcherPriority::RENDER] {
        let (s, d) = (seen.clone(), t.ui_thread.clone());
        t.ui_thread.post_local(
            move || {
                // The current schedule of a job is the dispatcher at the priority of the job.
                let schedule = d.task_schedule(priority);
                s.borrow_mut().push((Dispatcher::current_priority(), is_current_schedule(&schedule)));
            },
            priority,
        );
    }
    t.ui_thread.run_jobs(None);
    let s = seen.clone();
    t.ui_thread
        .invoke_local_with_priority(move || s.borrow_mut().push((Dispatcher::current_priority(), true)), DispatcherPriority::SEND)
        .unwrap();
    assert_eq!(
        *seen.borrow(),
        [
            (Some(DispatcherPriority::RENDER), true),
            (Some(DispatcherPriority::BACKGROUND), true),
            (Some(DispatcherPriority::SEND), true)
        ]
    );
    // Restored after each job.
    assert_eq!(Dispatcher::current_priority(), None);
    assert!(current_schedule().is_none());
}

#[test]
fn a_priority_set_outside_of_a_job_is_the_one_of_the_code_there_and_jobs_restore_it() {
    let t = DispatcherTests::new();
    assert_eq!(Dispatcher::set_current_priority(Some(DispatcherPriority::NORMAL)), None);
    assert_eq!(Dispatcher::current_priority(), Some(DispatcherPriority::NORMAL));
    assert!(is_current_schedule(&t.ui_thread.task_schedule(DispatcherPriority::NORMAL)));

    let seen = Rc::new(RefCell::new(None));
    let s = seen.clone();
    t.ui_thread.post_local(move || *s.borrow_mut() = Dispatcher::current_priority(), DispatcherPriority::BACKGROUND);
    t.ui_thread.run_jobs(None);
    assert_eq!(*seen.borrow(), Some(DispatcherPriority::BACKGROUND));
    assert_eq!(Dispatcher::current_priority(), Some(DispatcherPriority::NORMAL));

    assert_eq!(Dispatcher::set_current_priority(None), Some(DispatcherPriority::NORMAL));
    assert_eq!(Dispatcher::current_priority(), None);
    assert!(current_schedule().is_none());
}

#[test]
fn the_dispatcher_is_the_schedule_of_its_thread_until_it_shuts_down() {
    let t = DispatcherTests::new();
    let installed = thread_schedule().expect("the dispatcher is the schedule of its thread");
    // At the default priority, and one schedule per priority on the dispatcher thread.
    assert!(std::ptr::addr_eq(Arc::as_ptr(&installed), Arc::as_ptr(&t.ui_thread.task_schedule(DispatcherPriority::DEFAULT))));

    t.ui_thread.invoke_shutdown();
    assert!(thread_schedule().is_none());

    // The next dispatcher of the thread installs itself.
    Dispatcher::reset_for_unit_tests();
    let next = Dispatcher::current_dispatcher();
    let installed = thread_schedule().expect("the new dispatcher is the schedule of its thread");
    assert!(std::ptr::addr_eq(Arc::as_ptr(&installed), Arc::as_ptr(&next.task_schedule(DispatcherPriority::DEFAULT))));
}

// ---------------------------------------------------------------------------
// Wakes from other threads.
// ---------------------------------------------------------------------------

#[test]
fn a_system_task_woken_from_another_thread_resumes_in_a_dispatcher_job_without_run_pending_wakes() {
    let t = DispatcherTests::new();
    let log = Log::new();
    let (release_outside, gate_outside) = std::sync::mpsc::channel::<()>();
    let (release_inside, gate_inside) = std::sync::mpsc::channel::<()>();

    // Spawned outside of a job: the wake from the worker goes through the schedule of the thread,
    // a job at Default.
    let outside = log_resumption(
        &log,
        "spawned outside of a job",
        run(move || {
            let _ = gate_outside.recv();
            Ok(1)
        }),
    );
    // Spawned in a Background job: the wake is a job at Background.
    let inside = Rc::new(RefCell::new(None));
    {
        let (l, slot) = (log.clone(), inside.clone());
        t.ui_thread.post_local(
            move || {
                let work = run(move || {
                    let _ = gate_inside.recv();
                    Ok(2)
                });
                *slot.borrow_mut() = Some(log_resumption(&l, "spawned at Background", work));
            },
            DispatcherPriority::BACKGROUND,
        );
    }
    t.ui_thread.run_jobs(None);
    assert!(log.entries().is_empty());

    // The work ends after the first polls, so that its end is a wake from the worker.
    release_outside.send(()).unwrap();
    release_inside.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while log.entries().len() < 2 {
        assert!(Instant::now() < deadline, "the wakes did not come back");
        t.ui_thread.run_jobs(None);
        thread::yield_now();
    }
    let mut entries = log.entries();
    entries.sort();
    assert_eq!(entries, ["spawned at Background resumed with 2 at Background", "spawned outside of a job resumed with 1 at Default"]);
    assert!(outside.is_completed_successfully());
    assert!(inside.borrow().as_ref().is_some_and(Task::is_completed_successfully));
}

#[test]
fn a_local_cancellation_callback_canceled_on_another_thread_runs_in_a_dispatcher_job() {
    let t = DispatcherTests::new();
    let log = Log::new();
    let token = CancellationToken::new();
    let l = log.clone();
    let _guard = token.register_local(move || l.push(format!("canceled at {}", current_priority())));

    let remote = token.clone();
    thread::spawn(move || remote.cancel()).join().unwrap();
    assert!(log.entries().is_empty());
    t.ui_thread.run_jobs(None);
    assert_eq!(log.entries(), ["canceled at Default"]);
}

// ---------------------------------------------------------------------------
// Completions on the dispatcher thread (F3).
// ---------------------------------------------------------------------------

#[test]
fn a_system_task_spawned_in_a_job_resumes_inside_a_completion_in_a_job_of_its_priority() {
    let t = DispatcherTests::new();
    let log = Log::new();
    let source = TaskCompletionSource::<i32>::new();
    let task = Rc::new(RefCell::new(None));
    {
        let (l, awaited, slot) = (log.clone(), source.task(), task.clone());
        t.ui_thread.post_local(
            move || *slot.borrow_mut() = Some(log_resumption(&l, "the task", awaited)),
            DispatcherPriority::NORMAL,
        );
    }
    post_completion(&t.ui_thread, &log, &source, 7, DispatcherPriority::NORMAL);
    let l = log.clone();
    t.ui_thread.post_local(move || l.push("queued after the completing job"), DispatcherPriority::NORMAL);

    t.ui_thread.run_jobs(None);
    // The completing code runs where the task was spawned (the same dispatcher and priority): the
    // continuation runs inside it, as an `await` in .NET does in its captured context.
    assert_eq!(
        log.entries(),
        ["completing at Normal", "the task resumed with 7 at Normal", "completed", "queued after the completing job"]
    );
    assert!(task.borrow().as_ref().is_some_and(Task::is_completed_successfully));
}

#[test]
fn a_system_task_spawned_in_a_job_resumes_in_a_job_of_its_priority_when_completed_at_another() {
    let t = DispatcherTests::new();
    let log = Log::new();
    let source = TaskCompletionSource::<i32>::new();
    let task = Rc::new(RefCell::new(None));
    {
        let (l, awaited, slot) = (log.clone(), source.task(), task.clone());
        t.ui_thread.post_local(
            move || *slot.borrow_mut() = Some(log_resumption(&l, "the task", awaited)),
            DispatcherPriority::BACKGROUND,
        );
    }
    t.ui_thread.run_jobs(None);
    let l = log.clone();
    t.ui_thread.post_local(move || l.push("Background job queued before the completion"), DispatcherPriority::BACKGROUND);
    post_completion(&t.ui_thread, &log, &source, 7, DispatcherPriority::NORMAL);

    t.ui_thread.run_jobs(None);
    // A completion in a job of another priority posts the continuation: a job at the task's
    // priority, behind the work of that priority queued before the completion.
    assert_eq!(
        log.entries(),
        [
            "completing at Normal",
            "completed",
            "Background job queued before the completion",
            "the task resumed with 7 at Background"
        ]
    );
    assert!(task.borrow().as_ref().is_some_and(Task::is_completed_successfully));
}

#[test]
fn a_system_task_spawned_in_a_job_and_completed_outside_of_one_resumes_in_a_job() {
    let t = DispatcherTests::new();
    let log = Log::new();
    let source = TaskCompletionSource::<i32>::new();
    let task = Rc::new(RefCell::new(None));
    {
        let (l, awaited, slot) = (log.clone(), source.task(), task.clone());
        t.ui_thread.post_local(
            move || *slot.borrow_mut() = Some(log_resumption(&l, "the task", awaited)),
            DispatcherPriority::NORMAL,
        );
    }
    t.ui_thread.run_jobs(None);

    source.set_result(3);
    assert!(log.entries().is_empty());
    t.ui_thread.run_jobs(None);
    assert_eq!(log.entries(), ["the task resumed with 3 at Normal"]);
    assert!(task.borrow().as_ref().is_some_and(Task::is_completed_successfully));
}

#[test]
fn a_system_task_spawned_outside_of_a_job_resumes_inside_the_completion_wherever_it_runs() {
    // G9 on a dispatcher thread: no schedule is current where the task is spawned, so it stays in
    // the queue of the thread, which resumes it inside the completing call.
    let t = DispatcherTests::new();
    let log = Log::new();
    let source = TaskCompletionSource::<i32>::new();
    let task = log_resumption(&log, "the task", source.task());
    post_completion(&t.ui_thread, &log, &source, 5, DispatcherPriority::BACKGROUND);

    t.ui_thread.run_jobs(None);
    assert_eq!(log.entries(), ["completing at Background", "the task resumed with 5 at Background", "completed"]);
    assert!(task.is_completed_successfully());
}

#[test]
fn a_system_task_spawned_in_a_job_resumes_in_a_nested_frame_by_the_rule_of_its_priority() {
    let _t = DispatcherTests::new();
    let _services = DispatcherServices::new(SimpleDispatcherImpl::new(SimpleKind::Controlled));
    let disp = Dispatcher::ui_thread();
    let log = Log::new();
    let outer = CancellationToken::new();
    let nested = DispatcherFrame::new();
    let (first, second) = (TaskCompletionSource::<i32>::new(), TaskCompletionSource::<i32>::new());
    let task = Rc::new(RefCell::new(None));
    {
        let (l, d, frame, outer, slot) = (log.clone(), disp.clone(), nested.clone(), outer.clone(), task.clone());
        let (a, b) = (first.task(), second.task());
        let (first, second) = (first.clone(), second.clone());
        disp.post_local(
            move || {
                let lt = l.clone();
                *slot.borrow_mut() = Some(spawn_system_task(async move {
                    let value = a.await?;
                    lt.push(format!("resumed with {value} at {}", current_priority()));
                    let value = b.await?;
                    lt.push(format!("resumed with {value} at {}", current_priority()));
                    Ok::<_, ObservableError>(())
                }));
                l.push("J pushes a frame (Normal)");
                post_completion(&d, &l, &first, 1, DispatcherPriority::NORMAL);
                post_completion(&d, &l, &second, 2, DispatcherPriority::BACKGROUND);
                {
                    let (l, frame) = (l.clone(), frame.clone());
                    d.post_local(
                        move || {
                            l.push("exiting the frame (Background)");
                            frame.set_continue(false);
                        },
                        DispatcherPriority::BACKGROUND,
                    );
                }
                d.push_frame(&frame);
                l.push("J ends");
                outer.cancel();
            },
            DispatcherPriority::NORMAL,
        );
    }

    disp.main_loop(&outer);
    // In the frame, the completion in a job of the task's priority resumes it inline; the one in a
    // Background job queues a job at Normal, which the frame runs before the rest of the
    // Background work.
    assert_eq!(
        log.entries(),
        [
            "J pushes a frame (Normal)",
            "completing at Normal",
            "resumed with 1 at Normal",
            "completed",
            "completing at Background",
            "completed",
            "resumed with 2 at Normal",
            "exiting the frame (Background)",
            "J ends"
        ]
    );
    assert!(task.borrow().as_ref().is_some_and(Task::is_completed_successfully));
}
