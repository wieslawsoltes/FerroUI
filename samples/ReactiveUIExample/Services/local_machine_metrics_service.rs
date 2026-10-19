//! Port of `Services/LocalMachineMetricsService.cs`: samples process-local measurements from the
//! current machine.

use super::process_metrics::read_process;
use super::{heap_bytes, ILocalMachineMetricsService, SystemTimeProvider, TimeProvider};
use crate::models::MachineSnapshot;
use ferroui_base::utilities::DateTimeOffset;
use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactiveui::RxSchedulers;
use ferroui_system::{IObservable, ObservableError};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// The number of bytes in one megabyte.
const BYTES_PER_MEGABYTE: f64 = 1024.0 * 1024.0;

/// The maximum percentage value.
const MAXIMUM_PERCENTAGE: f64 = 100.0;

/// Samples process-local measurements from the current machine (`LocalMachineMetricsService`).
///
/// Deviation (DEVIATIONS.md, samples): the measurements come from the operating system
/// (`process_metrics.rs`) where the reference uses `System.Diagnostics.Process`, and the managed heap
/// is the heap the allocator of the example counts. The interval of [`watch`](Self::watch) ticks on
/// the scheduler of the user-interface thread (`RxSchedulers::main_thread_scheduler`), or on the
/// scheduler the service is given, where the reference ticks on the thread pool: the port has no
/// thread pool for observables (docs/porting/reactive.md, section 5.1).
pub struct LocalMachineMetricsService {
    sampler: Rc<Sampler>,
}

/// The state the measurements share: the clock and the previous processor sample.
struct Sampler {
    /// The time provider (`_timeProvider`).
    time_provider: Rc<dyn TimeProvider>,
    /// The scheduler the interval ticks on; `None` for the scheduler of the user-interface thread.
    scheduler: Option<Rc<dyn IScheduler>>,
    /// The previous total processor time and the time it was sampled (`_lastProcessorTime`,
    /// `_lastSampleTime`); `None` before the first sample.
    last: RefCell<Option<(Duration, DateTimeOffset)>>,
}

impl Default for LocalMachineMetricsService {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalMachineMetricsService {
    /// A service on the clock of the system (`LocalMachineMetricsService()`).
    pub fn new() -> Self {
        Self::with_time_provider(Rc::new(SystemTimeProvider))
    }

    /// A service on `time_provider` (`LocalMachineMetricsService(TimeProvider)`).
    pub fn with_time_provider(time_provider: Rc<dyn TimeProvider>) -> Self {
        Self::with_time_provider_and_scheduler(time_provider, None)
    }

    /// A service on `time_provider` whose interval ticks on `scheduler` (`None` for the scheduler of
    /// the user-interface thread).
    pub fn with_time_provider_and_scheduler(time_provider: Rc<dyn TimeProvider>, scheduler: Option<Rc<dyn IScheduler>>) -> Self {
        Self { sampler: Rc::new(Sampler { time_provider, scheduler, last: RefCell::new(None) }) }
    }
}

impl ILocalMachineMetricsService for LocalMachineMetricsService {
    fn read_snapshot(&self) -> Result<MachineSnapshot, ObservableError> {
        self.sampler.read_snapshot()
    }

    fn watch(&self, interval: Duration) -> Rc<dyn IObservable<MachineSnapshot>> {
        let scheduler = self.sampler.scheduler.clone().unwrap_or_else(RxSchedulers::main_thread_scheduler);
        let sampler = self.sampler.clone();
        Observable::interval_on(interval, scheduler).start_with([0_i64]).try_select(move |_| sampler.read_snapshot())
    }
}

impl Sampler {
    /// `ReadSnapshot()`.
    fn read_snapshot(&self) -> Result<MachineSnapshot, ObservableError> {
        let process = read_process()?;
        let now = self.time_provider.get_local_now();
        let cpu_percent = self.calculate_processor_percent(process.processor_time, now);

        Ok(MachineSnapshot::new(
            now,
            cpu_percent,
            process.working_set_bytes as f64 / BYTES_PER_MEGABYTE,
            heap_bytes() as f64 / BYTES_PER_MEGABYTE,
            process.thread_count,
            "Live local process data",
        ))
    }

    /// Calculates the process CPU percentage from two local samples
    /// (`CalculateProcessorPercent`): zero for the first sample, and when the clock did not advance.
    fn calculate_processor_percent(&self, processor_time: Duration, sample_time: DateTimeOffset) -> f64 {
        // The reference takes a lock (`_gate`); the samples of the port are taken on one thread.
        let previous = self.last.replace(Some((processor_time, sample_time)));

        let Some((last_processor_time, last_sample_time)) = previous else {
            return 0.0;
        };

        let elapsed_processor = processor_time.as_secs_f64() - last_processor_time.as_secs_f64();
        let elapsed_wall_milliseconds = sample_time.subtract_date_time_offset(last_sample_time).total_milliseconds();
        if elapsed_wall_milliseconds <= 0.0 {
            return 0.0;
        }

        let processor_count = std::thread::available_parallelism().map(|count| count.get()).unwrap_or(1).max(1) as f64;
        let percentage = elapsed_processor * 1000.0 / elapsed_wall_milliseconds / processor_count * MAXIMUM_PERCENTAGE;
        percentage.clamp(0.0, MAXIMUM_PERCENTAGE)
    }
}
