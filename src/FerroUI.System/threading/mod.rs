//! `System.Threading`: the cancellation of work, and tasks ([`tasks`]).

mod cancellation_token;
pub mod tasks;

pub use cancellation_token::{CancellationToken, CancellationTokenRegistration, CancellationTokenSource, LocalCancellationTokenRegistration};
