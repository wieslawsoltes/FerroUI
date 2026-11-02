//! `System.Threading`: the cancellation of work, and tasks ([`tasks`]).

mod cancellation_token;
mod synchronization_context;
pub mod tasks;

pub use cancellation_token::{
    CancellationGuard, CancellationToken, CancellationTokenRegistration, CancellationTokenSource,
    DropGuard, LocalCancellationTokenRegistration, WaitForCancellation,
};
pub use synchronization_context::{current_synchronization_context, set_synchronization_context, SynchronizationContext};
