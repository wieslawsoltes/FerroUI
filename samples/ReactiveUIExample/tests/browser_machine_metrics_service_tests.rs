//! Port of `BrowserMachineMetricsServiceTests.cs`: the browser-safe metrics sampler, which must not
//! depend on the measurements of a process.

use super::support::{run_until, start_application};
use crate::models::MachineSnapshot;
use crate::services::{BrowserMachineMetricsService, ILocalMachineMetricsService};
use ferroui_reactive::ObservableExt;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// The test sample interval (`SampleInterval`).
const SAMPLE_INTERVAL: Duration = Duration::from_millis(20);

#[test]
fn read_snapshot_returns_runtime_only_values() {
    let _app = start_application();
    let snapshot = BrowserMachineMetricsService::new().read_snapshot().expect("a browser snapshot");

    assert!(snapshot.managed_megabytes() > 0.0);
    assert!(snapshot.working_set_megabytes() > 0.0);
    assert_eq!(snapshot.thread_count(), 1);
    assert_eq!(snapshot.processor_percent(), 0.0);
    assert!(snapshot.status().starts_with("Browser data:"), "{}", snapshot.status());
}

#[test]
fn watch_produces_periodic_snapshots() {
    const REQUIRED_SAMPLES: usize = 2;
    let _app = start_application();
    let samples: Rc<RefCell<Vec<MachineSnapshot>>> = Rc::new(RefCell::new(Vec::new()));
    let received = samples.clone();
    let _subscription = BrowserMachineMetricsService::new()
        .watch(SAMPLE_INTERVAL)
        .take(REQUIRED_SAMPLES)
        .subscribe_next(move |snapshot| received.borrow_mut().push(snapshot));

    assert!(run_until(|| samples.borrow().len() == REQUIRED_SAMPLES), "the watch produced {} samples", samples.borrow().len());
    let snapshot = samples.borrow().last().cloned().expect("the last sample");
    assert!(snapshot.managed_megabytes() > 0.0);
}
