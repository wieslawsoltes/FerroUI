//! Port of `Services/BrowserMachineMetricsService.cs`: samples the few runtime measurements
//! available in a browser, where the measurements of a process are not.

use super::{heap_allocation_count, heap_bytes, ILocalMachineMetricsService, SystemTimeProvider, TimeProvider};
use crate::models::MachineSnapshot;
use ferroui_base::utilities::DateTimeOffset;
use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactiveui::RxSchedulers;
use ferroui_system::{IObservable, ObservableError};
use std::rc::Rc;
use std::time::Duration;

/// The number of bytes in one megabyte.
const BYTES_PER_MEGABYTE: f64 = 1024.0 * 1024.0;

/// Samples the few runtime measurements available in a browser (`BrowserMachineMetricsService`).
///
/// Deviation (DEVIATIONS.md, samples): the managed heap is the heap the allocator of the example
/// counts, and its allocations stand for the collections of the garbage collector of the reference
/// (`GC.CollectionCount(0)`); the uptime is the one of the service, since `Environment.TickCount64`
/// has no counterpart in a browser. The interval ticks on the scheduler of the user-interface thread,
/// as in [`LocalMachineMetricsService`](super::LocalMachineMetricsService).
pub struct BrowserMachineMetricsService {
    sampler: Rc<Sampler>,
}

/// The clock and the start of the service.
struct Sampler {
    /// The time provider (`_timeProvider`).
    time_provider: Rc<dyn TimeProvider>,
    /// The scheduler the interval ticks on; `None` for the scheduler of the user-interface thread.
    scheduler: Option<Rc<dyn IScheduler>>,
    /// When the service was created.
    started: DateTimeOffset,
}

impl Default for BrowserMachineMetricsService {
    fn default() -> Self {
        Self::new()
    }
}

impl BrowserMachineMetricsService {
    /// A service on the clock of the system (`BrowserMachineMetricsService()`).
    pub fn new() -> Self {
        Self::with_time_provider(Rc::new(SystemTimeProvider))
    }

    /// A service on `time_provider` (`BrowserMachineMetricsService(TimeProvider)`).
    pub fn with_time_provider(time_provider: Rc<dyn TimeProvider>) -> Self {
        Self::with_time_provider_and_scheduler(time_provider, None)
    }

    /// A service on `time_provider` whose interval ticks on `scheduler` (`None` for the scheduler of
    /// the user-interface thread).
    pub fn with_time_provider_and_scheduler(time_provider: Rc<dyn TimeProvider>, scheduler: Option<Rc<dyn IScheduler>>) -> Self {
        let started = time_provider.get_utc_now();
        Self { sampler: Rc::new(Sampler { time_provider, scheduler, started }) }
    }
}

impl ILocalMachineMetricsService for BrowserMachineMetricsService {
    fn read_snapshot(&self) -> Result<MachineSnapshot, ObservableError> {
        Ok(self.sampler.read_snapshot())
    }

    fn watch(&self, interval: Duration) -> Rc<dyn IObservable<MachineSnapshot>> {
        let scheduler = self.sampler.scheduler.clone().unwrap_or_else(RxSchedulers::main_thread_scheduler);
        let sampler = self.sampler.clone();
        Observable::interval_on(interval, scheduler).start_with([0_i64]).select(move |_| sampler.read_snapshot())
    }
}

impl Sampler {
    /// `ReadSnapshot()`: the heap, no processor reading and one thread.
    fn read_snapshot(&self) -> MachineSnapshot {
        let managed_megabytes = heap_bytes() as f64 / BYTES_PER_MEGABYTE;
        let uptime_seconds = self.time_provider.get_utc_now().subtract_date_time_offset(self.started).total_seconds().max(0.0) as i64;
        MachineSnapshot::new(
            self.time_provider.get_local_now(),
            0.0,
            managed_megabytes,
            managed_megabytes,
            1,
            format!("Browser data: CPU and threads are not measured. {} heap allocations, up {uptime_seconds} s.", heap_allocation_count()),
        )
    }
}
