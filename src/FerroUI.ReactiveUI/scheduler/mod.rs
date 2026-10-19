//! `ReactiveUI.Shared/Scheduler`: the subject that delivers on a scheduler.
//!
//! `WaitForDispatcherScheduler` is not ported here: it waits for the dispatcher of the user
//! interface, which the integration with FerroUI provides.

mod scheduled_subject;

pub use scheduled_subject::ScheduledSubject;
