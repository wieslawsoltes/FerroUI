//! `ReactiveUI.Shared/Suspension`: the suspension and resumption of an application.
//!
//! A [`SuspensionHost`] (or the typed [`SuspensionHostOf<T>`]) gathers the events of the life of
//! the process that a platform helper pushes into it, and the state of the application;
//! [`SuspensionHostExtensions::setup_default_suspend_resume`] loads and saves the state through an
//! [`ISuspensionDriver`](crate::ISuspensionDriver), and [`DummySuspensionDriver`] saves nothing.

mod dummy_suspension_driver;
mod suspension_host;
mod suspension_host_extensions;
mod suspension_host_t_app_state;

pub use dummy_suspension_driver::DummySuspensionDriver;
pub use suspension_host::SuspensionHost;
pub use suspension_host_extensions::{SuspensionHostExtensions, SuspensionHostOfExtensions};
pub use suspension_host_t_app_state::SuspensionHostOf;
