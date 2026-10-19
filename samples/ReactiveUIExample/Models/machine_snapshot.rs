//! Port of `Models/MachineSnapshot.cs`: one local machine measurement sample.

use ferroui_base::ferro_markup_type;
use ferroui_base::utilities::DateTimeOffset;

/// Represents one local machine measurement sample (`MachineSnapshot`).
///
/// Deviation (DEVIATIONS.md, samples): the record of the reference is a value here (`Clone` and
/// `PartialEq`, as a record compares by value); `with { Status = .. }` is
/// [`with_status`](Self::with_status). The managed heap of the reference is the heap of the process
/// as the allocator of the example counts it ([`heap_bytes`](crate::services::heap_bytes)).
#[derive(Clone, PartialEq)]
pub struct MachineSnapshot {
    captured_at: DateTimeOffset,
    processor_percent: f64,
    working_set_megabytes: f64,
    managed_megabytes: f64,
    thread_count: i32,
    status: String,
}

impl MachineSnapshot {
    /// A sample (`MachineSnapshot(DateTimeOffset, double, double, double, int, string)`).
    pub fn new(
        captured_at: DateTimeOffset,
        processor_percent: f64,
        working_set_megabytes: f64,
        managed_megabytes: f64,
        thread_count: i32,
        status: impl Into<String>,
    ) -> Self {
        Self { captured_at, processor_percent, working_set_megabytes, managed_megabytes, thread_count, status: status.into() }
    }

    /// An empty sample used before the first live update arrives (`Empty`).
    pub fn empty() -> Self {
        Self::new(DateTimeOffset::UNIX_EPOCH, 0.0, 0.0, 0.0, 0, "Waiting for the first local sample.")
    }

    /// The local time when the sample was captured (`CapturedAt`).
    pub fn captured_at(&self) -> DateTimeOffset {
        self.captured_at
    }

    /// The estimated process CPU percentage (`ProcessorPercent`).
    pub fn processor_percent(&self) -> f64 {
        self.processor_percent
    }

    /// The process working-set size in megabytes (`WorkingSetMegabytes`).
    pub fn working_set_megabytes(&self) -> f64 {
        self.working_set_megabytes
    }

    /// The heap size in megabytes (`ManagedMegabytes`).
    pub fn managed_megabytes(&self) -> f64 {
        self.managed_megabytes
    }

    /// The process thread count (`ThreadCount`).
    pub fn thread_count(&self) -> i32 {
        self.thread_count
    }

    /// The feed status text (`Status`).
    pub fn status(&self) -> String {
        self.status.clone()
    }

    /// The same sample with another status (`this with { Status = status }`).
    pub fn with_status(&self, status: impl Into<String>) -> Self {
        Self { status: status.into(), ..self.clone() }
    }
}

ferro_markup_type!(struct MachineSnapshot {
    handles: [MachineSnapshot],
    properties: [
        CapturedAt: DateTimeOffset { get: |this: &MachineSnapshot| this.captured_at() },
        ProcessorPercent: f64 { get: |this: &MachineSnapshot| this.processor_percent() },
        WorkingSetMegabytes: f64 { get: |this: &MachineSnapshot| this.working_set_megabytes() },
        ManagedMegabytes: f64 { get: |this: &MachineSnapshot| this.managed_megabytes() },
        ThreadCount: i32 { get: |this: &MachineSnapshot| this.thread_count() },
        Status: String { get: |this: &MachineSnapshot| this.status() },
    ],
});
