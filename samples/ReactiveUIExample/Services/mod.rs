//! The services of the example (namespace `ReactiveUIExample.Services`): one module per reference
//! file, and what the port adds for them (the clock, the errors, the measurements of the process and
//! the counted heap).

mod browser_machine_metrics_service;
mod errors;
mod heap_usage;
mod i_local_machine_metrics_service;
mod local_machine_metrics_service;
mod process_metrics;
mod showcase_session;
mod time_provider;

pub use browser_machine_metrics_service::BrowserMachineMetricsService;
pub use errors::{InvalidOperationError, PlatformNotSupportedError};
pub use heap_usage::{heap_allocation_count, heap_bytes};
pub use i_local_machine_metrics_service::ILocalMachineMetricsService;
pub use local_machine_metrics_service::LocalMachineMetricsService;
pub use showcase_session::ShowcaseSession;
pub use time_provider::{SystemTimeProvider, TimeProvider};
