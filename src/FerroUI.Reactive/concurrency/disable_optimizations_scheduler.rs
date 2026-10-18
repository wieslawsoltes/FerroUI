//! `DisableOptimizationsScheduler` and its base `SchedulerWrapper`
//! (`Concurrency/DisableOptimizationsScheduler.cs`, `Concurrency/SchedulerWrapper.cs`), which
//! `Scheduler.DisableOptimizations` returns (`Concurrency/Scheduler.Services.cs`).

use super::{IScheduler, ISchedulerPeriodic, IStopwatchProvider, ScheduledAction};
use ferroui_system::IDisposable;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::time::{Duration, SystemTime};

/// A service of a scheduler that `DisableOptimizations` can hide (an interface of
/// `Scheduler.Optimizations`, which upstream names by its `Type`).
///
/// Deviation (DEVIATIONS.md, Reactive): `ISchedulerLongRunning` is not in the port, so it is not
/// one of the services.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SchedulerOptimization {
    /// `IStopwatchProvider` ([`IScheduler::as_stopwatch_provider`]).
    StopwatchProvider,
    /// `ISchedulerPeriodic` ([`IScheduler::as_periodic`]).
    Periodic,
}

impl SchedulerOptimization {
    /// Every service a scheduler may offer (`Scheduler.Optimizations`).
    pub const ALL: [SchedulerOptimization; 2] = [SchedulerOptimization::StopwatchProvider, SchedulerOptimization::Periodic];
}

/// The wrappers made for the schedulers that run the work of a wrapper, shared by the wrapper and
/// the wrappers it makes (the `ConditionalWeakTable<IScheduler, IScheduler>` of
/// `SchedulerWrapper`). A wrapper is kept while something holds it: holding it here strongly would
/// make a cycle through the cache it holds.
type WrapperCache = Rc<RefCell<Vec<Weak<DisableOptimizationsScheduler>>>>;

/// A scheduler that schedules on another one and hides some of its services
/// (`DisableOptimizationsScheduler`, a `SchedulerWrapper`): the services named are answered with
/// none, the others are those of the scheduler it wraps. The work it schedules receives a wrapper
/// of the scheduler that runs it, with the same services hidden, so that the work scheduled from
/// inside it is wrapped too (`SchedulerWrapper.Wrap`).
pub(crate) struct DisableOptimizationsScheduler {
    scheduler: Rc<dyn IScheduler>,
    optimizations: Rc<[SchedulerOptimization]>,
    cache: WrapperCache,
}

impl DisableOptimizationsScheduler {
    /// `scheduler` without the services `optimizations` (`new DisableOptimizationsScheduler(scheduler, optimizationInterfaces)`).
    pub(crate) fn create(scheduler: Rc<dyn IScheduler>, optimizations: &[SchedulerOptimization]) -> Rc<dyn IScheduler> {
        Rc::new(Self { scheduler, optimizations: optimizations.into(), cache: Rc::new(RefCell::new(Vec::new())) })
    }

    fn is_disabled(&self, optimization: SchedulerOptimization) -> bool {
        self.optimizations.contains(&optimization)
    }

    /// The wrapper of `scheduler` with the same services hidden, made once while it is held
    /// (`GetRecursiveWrapper`, which clones the wrapper for `scheduler`).
    fn recursive_wrapper(scheduler: &Rc<dyn IScheduler>, optimizations: &Rc<[SchedulerOptimization]>, cache: &WrapperCache) -> Rc<dyn IScheduler> {
        let mut wrappers = cache.borrow_mut();
        wrappers.retain(|wrapper| wrapper.strong_count() > 0);
        let found = wrappers
            .iter()
            .filter_map(Weak::upgrade)
            .find(|wrapper| std::ptr::addr_eq(Rc::as_ptr(&wrapper.scheduler), Rc::as_ptr(scheduler)));
        if let Some(wrapper) = found {
            return wrapper;
        }
        let wrapper = Rc::new(Self { scheduler: scheduler.clone(), optimizations: optimizations.clone(), cache: cache.clone() });
        wrappers.push(Rc::downgrade(&wrapper));
        wrapper
    }

    /// The work, handed the wrapper of the scheduler that runs it (`SchedulerWrapper.Wrap`).
    fn wrap(&self, action: ScheduledAction) -> ScheduledAction {
        let (optimizations, cache) = (self.optimizations.clone(), self.cache.clone());
        Box::new(move |scheduler: &Rc<dyn IScheduler>| {
            let wrapper = Self::recursive_wrapper(scheduler, &optimizations, &cache);
            action(&wrapper)
        })
    }
}

impl IScheduler for DisableOptimizationsScheduler {
    fn now(&self) -> SystemTime {
        self.scheduler.now()
    }

    fn schedule(&self, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.scheduler.schedule(self.wrap(action))
    }

    fn schedule_after(&self, due_time: Duration, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.scheduler.schedule_after(due_time, self.wrap(action))
    }

    fn schedule_at(&self, due_time: SystemTime, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.scheduler.schedule_at(due_time, self.wrap(action))
    }

    /// None when the periodic scheduling is hidden, else that of the scheduler wrapped
    /// (`GetService(typeof(ISchedulerPeriodic))`).
    fn as_periodic(&self) -> Option<&dyn ISchedulerPeriodic> {
        if self.is_disabled(SchedulerOptimization::Periodic) {
            return None;
        }
        self.scheduler.as_periodic()
    }

    /// None when the stopwatch is hidden, else that of the scheduler wrapped
    /// (`GetService(typeof(IStopwatchProvider))`).
    fn as_stopwatch_provider(&self) -> Option<&dyn IStopwatchProvider> {
        if self.is_disabled(SchedulerOptimization::StopwatchProvider) {
            return None;
        }
        self.scheduler.as_stopwatch_provider()
    }
}
