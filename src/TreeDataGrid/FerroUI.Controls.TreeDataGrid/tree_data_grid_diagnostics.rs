//! `TreeDataGridDiagnostics.cs`: the diagnostic settings of the grid.

use std::cell::Cell;

ferroui_base::ferro_thread_local! {
    static ENABLE_TRACING: Cell<bool> = const { Cell::new(false) };
}

/// Provides diagnostic settings for the grid (`static class TreeDataGridDiagnostics`).
pub struct TreeDataGridDiagnostics;

impl TreeDataGridDiagnostics {
    /// Gets a value indicating whether diagnostic tracing is enabled.
    ///
    /// When enabled, detailed trace output is written to the debug output for viewport changes,
    /// element realization, and attach/detach operations. This setting is only active in debug
    /// builds.
    pub fn enable_tracing() -> bool {
        ENABLE_TRACING.with(Cell::get)
    }

    /// Sets [`enable_tracing`](Self::enable_tracing).
    pub fn set_enable_tracing(value: bool) {
        ENABLE_TRACING.with(|cell| cell.set(value));
    }

    /// Writes a trace line when tracing is enabled, in debug builds only.
    pub fn trace(message: impl FnOnce() -> String) {
        if cfg!(debug_assertions) && Self::enable_tracing() {
            eprintln!("[TreeDataGrid] {}", message());
        }
    }
}
