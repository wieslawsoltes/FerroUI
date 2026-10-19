//! Port of `Services/ILocalMachineMetricsService.cs`: provides live local machine measurements.

use crate::models::MachineSnapshot;
use ferroui_system::{IObservable, ObservableError};
use std::any::Any;
use std::rc::Rc;
use std::time::Duration;

/// Provides live local machine measurements (`ILocalMachineMetricsService`).
///
/// Deviation (DEVIATIONS.md, samples): a measurement that fails is an `Err` where the reference
/// throws. The trait derives from `Any`, so that a service can be told by its type
/// (`service.GetType()`).
pub trait ILocalMachineMetricsService: Any {
    /// Reads a single local machine measurement (`ReadSnapshot()`).
    fn read_snapshot(&self) -> Result<MachineSnapshot, ObservableError>;

    /// Watches local machine measurements at the requested interval (`Watch(TimeSpan)`): one at once,
    /// then one per interval; a measurement that fails ends the stream with its error.
    fn watch(&self, interval: Duration) -> Rc<dyn IObservable<MachineSnapshot>>;
}
