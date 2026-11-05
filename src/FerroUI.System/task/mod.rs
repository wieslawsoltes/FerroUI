//! Futures run on the thread that spawns them: the executor of the workspace (named after
//! `std::task`; phase C4 of #423, docs/idiomatic-rust-async.md, section 3).
//!
//! One task table and one poll loop per thread. A spawned future is a task of the thread that
//! spawns it, polled on that thread only, and its handle ([`JoinHandle`]) is a future of its
//! outcome: its output, or a [`JoinError`] when it was aborted or panicked. Where the polls of a
//! woken task run is the task's schedule, fixed when it is spawned:
//!
//! - [`spawn_local`] gives the task to the queue of its thread. A wake on that thread polls the
//!   task at once, inside the call that wakes it; a wake from another thread waits in the queue
//!   of the task's thread until that thread runs [`run_pending`] (or [`block_on`] does), and
//!   is announced to the schedule of the thread when one is set ([`set_thread_schedule`]).
//! - [`spawn_local_on`] gives the task a [`Schedule`]: every wake, from any thread, queues one
//!   poll there (at most one queued poll at a time), and the schedule runs it on the task's
//!   thread in order with its other work. The first poll runs inside the spawning call
//!   ([`FirstPoll::Now`]) or is queued as well ([`FirstPoll::Scheduled`]).
//! - [`spawn_on`] does the same from any thread: a `Send` closure makes the future on the
//!   schedule's thread, inside the first work it gives the schedule, and the future is polled
//!   there at once.
//!
//! The schedule of a thread ([`set_thread_schedule`]; a dispatcher is the schedule of its thread)
//! is where the wakes from other threads of the tasks in the queue of the thread go, and the
//! callbacks that other threads send to it. The work of a schedule may make it the current
//! schedule of its thread while it runs ([`set_current_schedule`], [`current_schedule`]; a
//! dispatcher job is the dispatcher at the priority of the job): [`spawn_local`] in that work
//! gives the task that schedule, whose wakes poll it at once while the same schedule is current
//! and queue a poll on it otherwise (the rule of `await` with a `SynchronizationContext`), and
//! [`Schedule::schedule_local`] posts work that is not `Send` to it.
//!
//! A schedule is told of the tasks given to it ([`Schedule::spawned`]), with an [`AbortHandle`],
//! so that it can end them (a dispatcher aborts its tasks when it shuts down). [`try_id`] is the
//! [`Id`] of the task being polled, and [`poll_now`] polls a task of the thread inside the calling
//! code (a dispatcher job that resumes it at another priority).
//!
//! In both cases a task is never polled inside its own poll: a wake that comes while it is polled
//! polls it again once that poll has returned (the queue of the thread), or queues a poll (a
//! schedule), and a queued poll that runs while the task is polled (a nested loop that runs the
//! schedule's work) marks it, and the running poll schedules it again when it returns.
//!
//! A panic of a future faults its task: the panic is caught, the future is dropped, and the
//! handle ends with [`JoinError::Panicked`], waking whoever awaits it; the code that woke the
//! future goes on.
//!
//! [`Shared`] (from [`JoinHandle::shared`]) is the outcome of a task for several observers, with
//! synchronous continuations ([`Shared::on_finished`]) that run before the awaiting futures are
//! woken. [`Shared::channel`] makes one that code ends by hand ([`SharedSender`]): the
//! `TaskCompletionSource<T>` and `Task<T>` of the reactive libraries, whose asynchronous forms give a
//! `Shared<Result<T, ObservableError>>` ([`Shared::flatten`] is a future of that result,
//! [`Flatten`]).
//!
//! [`oneshot`] is a value sent once to the code that awaits it (the port of
//! `TaskCompletionSource`), from any thread; a sender dropped without sending ends the receiver
//! with [`Canceled`].

mod canceled;
pub(crate) mod executor;
mod join;
pub mod oneshot;
mod schedule;
mod shared;

pub use canceled::Canceled;
pub use executor::{
    block_on, current_schedule, is_current_schedule, poll_now, run_pending, set_current_schedule, set_thread_schedule, spawn_local, spawn_local_on,
    spawn_on, thread_schedule, try_id,
};
pub use join::{AbortHandle, Id, JoinError, JoinHandle};
pub use schedule::{FirstPoll, Schedule};
pub use shared::{Flatten, OnFinishedId, Shared, SharedSender};
