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
//! woken.

pub(crate) mod executor;
mod join;
mod schedule;
mod shared;

pub use executor::{block_on, run_pending, set_thread_schedule, spawn_local, spawn_local_on};
pub use join::{JoinError, JoinHandle};
pub use schedule::{FirstPoll, Schedule};
pub use shared::Shared;
