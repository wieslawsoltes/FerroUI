//! Port of `LocalMachineMetricsServiceTests.cs`: the real local process sampler and its periodic
//! observable.

use super::support::{run_until, start_application};
use crate::models::MachineSnapshot;
use crate::services::{ILocalMachineMetricsService, LocalMachineMetricsService, TimeProvider};
use ferroui_base::utilities::DateTimeOffset;
use ferroui_reactive::ObservableExt;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// The test sample interval (`SampleInterval`).
const SAMPLE_INTERVAL: Duration = Duration::from_millis(20);

/// A clock fixed to one instant, for the paths of an elapsed time of zero (`FixedTimeProvider`).
struct FixedTimeProvider {
    instant: DateTimeOffset,
}

impl TimeProvider for FixedTimeProvider {
    fn get_utc_now(&self) -> DateTimeOffset {
        self.instant
    }
}

#[test]
fn watch_produces_periodic_process_measurements() {
    const REQUIRED_SAMPLES: usize = 2;
    const MAXIMUM_PERCENTAGE: f64 = 100.0;
    let _app = start_application();
    let service = LocalMachineMetricsService::new();
    let samples: Rc<RefCell<Vec<MachineSnapshot>>> = Rc::new(RefCell::new(Vec::new()));
    let received = samples.clone();
    let _subscription = service.watch(SAMPLE_INTERVAL).take(REQUIRED_SAMPLES).subscribe_next_error(
        move |snapshot| received.borrow_mut().push(snapshot),
        |error| panic!("the process could not be measured: {error}"),
    );

    assert!(run_until(|| samples.borrow().len() == REQUIRED_SAMPLES), "the watch produced {} samples", samples.borrow().len());
    let snapshot = samples.borrow().last().cloned().expect("the last sample");
    assert!(snapshot.working_set_megabytes() > 0.0);
    assert!(snapshot.managed_megabytes() > 0.0);
    assert!(snapshot.thread_count() > 0);
    assert!(snapshot.processor_percent() >= 0.0);
    assert!(snapshot.processor_percent() <= MAXIMUM_PERCENTAGE);
}

#[test]
fn read_snapshot_non_advancing_clock_produces_zero_processor_percentage() {
    let _app = start_application();
    let service = LocalMachineMetricsService::with_time_provider(Rc::new(FixedTimeProvider { instant: DateTimeOffset::UNIX_EPOCH }));
    let _ = service.read_snapshot().expect("a first snapshot");

    assert_eq!(service.read_snapshot().expect("a second snapshot").processor_percent(), 0.0);
}
