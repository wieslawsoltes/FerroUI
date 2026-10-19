//! `System.Threading.Tasks`: tasks, their completion sources, and the running of futures on the
//! thread that started them (docs/porting/reactive.md, section 5.8).

mod local_executor;
mod task;

pub use local_executor::{block_on, canceled, run, run_pending_wakes, set_thread_poster, spawn_local, TaskWorkError, ThreadPoster};
pub use task::{Task, TaskCanceledError, TaskCompletionSource, TaskStatus};
