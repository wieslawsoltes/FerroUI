//! `PlatformRegistrations` of the target without a user interface
//! (`ReactiveUI/Platforms/net/PlatformRegistrations.cs`), the platform module that
//! `with_platform_services` of the builder applies.

use super::{IRegistrar, IWantsToRegisterStuff};
use crate::RxSchedulers;
use ferroui_reactive::concurrency::SchedulerDefaults;
use ferroui_splat::ModeDetector;

/// The registrations of the platform (`PlatformRegistrations`): outside a unit test runner, the
/// schedulers of [`RxSchedulers`] become the default ones.
///
/// This is the module of upstream's plain .NET target, which has no user interface; the
/// integration of FerroUI's user interface registers its own module, which sets the scheduler of
/// the dispatcher.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream sets the task pool scheduler to
/// `TaskPoolScheduler.Default`; a thread pool scheduler cannot run the single-threaded observables
/// (`docs/porting/reactive.md`, section 5.1), so both become the default scheduler of the thread.
#[derive(Debug, Default, Clone, Copy)]
pub struct PlatformRegistrations;

impl IWantsToRegisterStuff for PlatformRegistrations {
    fn register(&self, _registrar: &dyn IRegistrar) {
        if ModeDetector::in_unit_test_runner() {
            return;
        }

        RxSchedulers::set_task_pool_scheduler(SchedulerDefaults::default_scheduler());
        RxSchedulers::set_main_thread_scheduler(SchedulerDefaults::default_scheduler());
    }
}
