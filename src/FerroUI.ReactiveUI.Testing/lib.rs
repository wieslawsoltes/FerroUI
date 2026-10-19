//! ferroui-reactiveui-testing
//!
//! A port of ReactiveUI.Testing and ReactiveUI.Testing.Reactive
//! (<https://github.com/reactiveui/ReactiveUI>) for `ferroui-reactiveui`: the helpers of the tests
//! of view models and of the code that runs on ReactiveUI's schedulers.
//!
//! - [`SchedulerExtensions`]: runs code with a scheduler, typically a
//!   [`TestScheduler`](ferroui_reactive_testing::TestScheduler), as both schedulers of
//!   [`RxSchedulers`](ferroui_reactiveui::RxSchedulers) (`SchedulerExtensions`).
//! - [`TestSchedulerExtensions`]: the virtual time of the test scheduler in milliseconds, and
//!   recorded notifications at a time in milliseconds (`TestSchedulerExtensions`, from
//!   ReactiveUI.Testing.Reactive).
//! - [`MessageBusExtensions`]: runs code with a message bus as
//!   [`MessageBus::current`](ferroui_reactiveui::MessageBus::current) (`MessageBusExtensions`).
//! - [`IBuilder`] and [`IBuilderExtensions`]: the builders of test data (`IBuilder`,
//!   `IBuilderExtensions`).
//! - [`RxTest`] and [`AppBuilderTestBase`]: run a test of the application builder of Splat from a
//!   clean builder state, one at a time and within a time limit (`RxTest`, `AppBuilderTestBase`).
//! - [`TestSequencer`]: two participants of a test that wait for each other phase by phase
//!   (`TestSequencer`).
//!
//! A test that runs a view model on a test scheduler:
//!
//! ```
//! use ferroui_reactive_testing::TestScheduler;
//! use ferroui_reactiveui::RxSchedulers;
//! use ferroui_reactiveui_testing::{SchedulerExtensions, TestSchedulerExtensions};
//! use std::rc::Rc;
//!
//! let scheduler = TestScheduler::new();
//! scheduler.with(|scheduler| {
//!     assert!(Rc::ptr_eq(&RxSchedulers::main_thread_scheduler(), &scheduler.scheduler()));
//!     scheduler.advance_by_ms(250.0);
//!     assert_eq!(scheduler.clock(), TestScheduler::from_time_span(std::time::Duration::from_millis(250)));
//! });
//! ```
//!
//! Everything is single-threaded, as `ferroui-reactiveui` is (`docs/porting/reactive.md`, section
//! 5.1): the state these helpers set (the schedulers, the message bus, the state of the
//! application builder) is the state of the current thread.

mod app_builder_test_base;
mod i_builder;
mod i_builder_extensions;
mod message_bus_extensions;
mod rx_test;
mod scheduler_extensions;
mod test_scheduler_extensions;
mod test_sequencer;

pub use app_builder_test_base::AppBuilderTestBase;
pub use i_builder::IBuilder;
pub use i_builder_extensions::IBuilderExtensions;
pub use message_bus_extensions::MessageBusExtensions;
pub use rx_test::RxTest;
pub use scheduler_extensions::SchedulerExtensions;
pub use test_scheduler_extensions::TestSchedulerExtensions;
pub use test_sequencer::TestSequencer;
