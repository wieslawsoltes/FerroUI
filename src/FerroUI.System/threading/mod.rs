//! `System.Threading`: the cancellation of work, and tasks ([`tasks`]).

mod cancellation_token;
mod synchronization_context;
pub mod tasks;

pub use cancellation_token::{CancellationGuard, CancellationToken, DropGuard, WaitForCancellation};
#[allow(deprecated)]
pub use cancellation_token::{
    CancellationTokenRegistration, CancellationTokenSource, LocalCancellationTokenRegistration,
};
pub use synchronization_context::{current_synchronization_context, set_synchronization_context, SynchronizationContext};
