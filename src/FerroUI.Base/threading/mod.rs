//! The dispatcher: the per-thread work queue that everything UI-related
//! runs on, and the one place where other threads can hand work to a UI
//! thread.

mod dispatcher;
mod dispatcher_event_args;
mod dispatcher_exceptions;
mod dispatcher_frame;
mod dispatcher_invoke;
mod dispatcher_main_loop;
mod dispatcher_operation;
mod dispatcher_options;
mod dispatcher_priority;
mod dispatcher_priority_awaitable;
mod dispatcher_priority_queue;
mod dispatcher_spawn;
mod dispatcher_task;
mod dispatcher_queue;
mod dispatcher_thread_storage;
mod dispatcher_timer;
mod dispatcher_timers;
mod dispatcher_unhandled_exception_event_args;
mod dispatcher_unhandled_exception_filter_event_args;
mod ferro_synchronization_context;
mod i_dispatcher;
mod i_dispatcher_impl;

pub use ferroui_system::task::Canceled;
// The handle of a task of the dispatcher (`Dispatcher::spawn_local`, `spawn`, `start_local`) and how
// it ends without an output: the types of `ferroui-system`, for the crates that use the dispatcher.
pub use ferroui_system::task::{AbortHandle, JoinError, JoinHandle};
pub use ferroui_system::threading::{CancellationGuard, CancellationToken, DropGuard, WaitForCancellation};
#[allow(deprecated)]
pub use ferroui_system::threading::{CancellationTokenRegistration, CancellationTokenSource};
pub use dispatcher::{
    Dispatcher, DispatcherUnhandledExceptionEventHandler, DispatcherUnhandledExceptionFilterEventHandler,
};
pub use dispatcher_event_args::DispatcherEventArgs;
pub use dispatcher_frame::DispatcherFrame;
pub use dispatcher_main_loop::DispatcherProcessingDisabled;
pub use dispatcher_operation::{DispatcherException, DispatcherOperation, DispatcherOperationStatus};
#[allow(deprecated)]
pub use dispatcher_operation::OperationCanceledError;
pub use dispatcher_options::DispatcherOptions;
pub use dispatcher_priority::{DispatcherPriority, DispatcherPriorityOutOfRangeError};
pub use dispatcher_invoke::DispatcherInvokeError;
pub use dispatcher_priority_awaitable::{DispatcherPriorityAwaitable, DispatcherPriorityTaskAwaitable};
#[allow(deprecated)]
pub use dispatcher_task::DispatcherTask;
#[allow(deprecated)]
pub use ferro_synchronization_context::{DispatcherTaskScheduler, FerroSynchronizationContext, RestoreContext};
mod thread_safe_object_pool;
pub use thread_safe_object_pool::ThreadSafeObjectPool;
pub use dispatcher_thread_storage::UnitTestDispatcherScope;
pub use dispatcher_timer::DispatcherTimer;
pub use dispatcher_unhandled_exception_event_args::DispatcherUnhandledExceptionEventArgs;
pub use dispatcher_unhandled_exception_filter_event_args::DispatcherUnhandledExceptionFilterEventArgs;
pub use i_dispatcher::IDispatcher;
pub use i_dispatcher_impl::{
    DispatcherImplEvent, IControlledDispatcherImpl, IDispatcherImpl, IDispatcherImplWithExplicitBackgroundProcessing,
    IDispatcherImplWithPendingInput, IDispatcherSignal, IPlatformThreadingSignal,
};

#[cfg(test)]
mod dispatcher_priority_queue_tests;
#[cfg(test)]
mod dispatcher_tests;
#[cfg(test)]
mod dispatcher_tests_exception;
