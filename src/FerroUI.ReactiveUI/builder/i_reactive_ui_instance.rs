//! `IReactiveUIInstance` (`ReactiveUI.Shared/Builder/IReactiveUIInstance.cs`).

use ferroui_reactive::concurrency::IScheduler;
use ferroui_splat::IAppInstance;
use std::rc::Rc;

/// A built ReactiveUI application (`IReactiveUIInstance`): its resolvers, and the schedulers the
/// builder was given.
pub trait IReactiveUIInstance: IAppInstance {
    /// The scheduler of the work of the user-interface thread that the builder was given, if any
    /// (`MainThreadScheduler`).
    fn main_thread_scheduler(&self) -> Option<Rc<dyn IScheduler>>;

    /// The scheduler of background work that the builder was given, if any (`TaskpoolScheduler`).
    fn task_pool_scheduler(&self) -> Option<Rc<dyn IScheduler>>;
}
