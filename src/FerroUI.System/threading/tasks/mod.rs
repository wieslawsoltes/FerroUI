//! `System.Threading.Tasks`: tasks, their completion sources, and the running of futures on the
//! thread that started them (docs/porting/reactive.md, section 5.8). The futures run on the executor
//! of the crate ([`crate::task`]).
//!
//! Since 0.5.0 the tasks are the types of [`crate::task`]: [`run`] (`Task.Run`) gives a
//! [`Shared`](crate::task::Shared) of its result, and [`canceled`] and [`TaskWorkError`] stay.
//! `Task`, `TaskCompletionSource`, `TaskStatus`, `ContinuationId`, `TaskCanceledError`,
//! `spawn_local`, `block_on`, `run_pending_wakes`, `set_thread_poster` and `ThreadPoster`, the
//! wrappers over them deprecated in 0.5.0, were removed in 0.7.0 (#625;
//! docs/idiomatic-rust-async.md, section 4).

pub(crate) mod local_executor;

pub use local_executor::{canceled, run, TaskWorkError};
