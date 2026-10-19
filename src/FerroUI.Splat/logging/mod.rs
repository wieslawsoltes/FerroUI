//! Logging (Splat.Logging): the logger interfaces, the loggers Splat ships, the log managers that
//! hand out a logger per type, [`IEnableLogger`] and [`LogHost`].
//!
//! # Types
//!
//! Upstream passes the `Type` of the class that logs; the port passes the name of the type, as
//! `std::any::type_name` gives it (`&'static str`). The generic forms (`debug_for::<T>`, ...)
//! take the type as a type parameter.
//!
//! # Formatting
//!
//! The generated overloads of upstream with one to ten arguments (`Debug<TArgument>(string
//! messageFormat, TArgument argument)`, ...) and the `params object[]` ones are one method each
//! that takes [`std::fmt::Arguments`], made with `format_args!`:
//!
//! ```ignore
//! logger.debug_args(format_args!("{} of {}", index, count));
//! ```
//!
//! `format_args!` formats nothing: the arguments are formatted when the logger writes the message,
//! so a method that checks the level first (the `_args` methods, as the allocation-free overloads
//! of upstream do) formats only when the level is enabled. The format string is checked at
//! compile time.
//!
//! # Exceptions
//!
//! An exception is a `&dyn std::error::Error`, written with its `Display`.

mod action_logger;
mod console_logger;
mod debug_logger;
mod default_log_manager;
mod full_logger;
mod log_host;
mod log_level;
mod log_manager;
mod logger;
mod logging_exception;
mod memoizing_mru_cache;
mod modules;
mod null_logger;
mod static_full_logger;
mod type_name;
mod wrapping_loggers;

pub use action_logger::ActionLogger;
pub use console_logger::ConsoleLogger;
pub use debug_logger::DebugLogger;
pub use default_log_manager::DefaultLogManager;
pub use full_logger::{FullLoggerExt, IAllocationFreeErrorLogger, IAllocationFreeLogger, IFullLogger};
pub use log_host::{IEnableLogger, LogHost};
pub use log_level::LogLevel;
pub use log_manager::{FuncLogManager, ILogManager, LogManagerExt};
pub use logger::ILogger;
pub use logging_exception::LoggingException;
pub use memoizing_mru_cache::{AggregateReleaseError, MemoizingMRUCache};
pub use modules::{SplatLogManagerModule, SplatLoggerModule};
pub use null_logger::NullLogger;
pub use static_full_logger::{IStaticFullLogger, StaticFullLogger, StaticFullLoggerExt};
pub use type_name::short_type_name;
pub use wrapping_loggers::{WrappingFullLogger, WrappingLogLevelLogger, WrappingPrefixLogger};
