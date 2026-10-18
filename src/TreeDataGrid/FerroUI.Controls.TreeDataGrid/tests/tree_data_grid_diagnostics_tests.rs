//! `TreeDataGridDiagnosticsTests.cs`: the diagnostic tracing of the grid is opt-in and written in
//! debug builds only.
//!
//! The reference checks by reflection that the `Trace` methods of the presenter base and of
//! `RealizedStackElements` carry `[Conditional("DEBUG")]`. Rust has no attribute reflection, and
//! those methods are private to their modules: in the port they write only when
//! `cfg!(debug_assertions)` holds and `TreeDataGridDiagnostics::enable_tracing()` is set, the gate
//! of `TreeDataGridDiagnostics::trace`. The test checks that gate through the public `trace`: a
//! trace does not even build its message unless tracing is enabled, and builds it then only in a
//! debug build.

use crate::TreeDataGridDiagnostics;
use std::cell::Cell;

/// Restores the thread-local tracing flag when the test ends, also when an assertion fails.
struct RestoreTracing(bool);

impl Drop for RestoreTracing {
    fn drop(&mut self) {
        TreeDataGridDiagnostics::set_enable_tracing(self.0);
    }
}

#[test]
fn tracing_is_opt_in_and_trace_calls_are_debug_only() {
    assert!(!TreeDataGridDiagnostics::enable_tracing());
    let _restore = RestoreTracing(false);

    let messages_built = Cell::new(0);
    let trace = || {
        TreeDataGridDiagnostics::trace(|| {
            messages_built.set(messages_built.get() + 1);
            "diagnostics test".to_string()
        })
    };

    // Opt-in: without tracing enabled, nothing is traced.
    trace();
    assert_eq!(0, messages_built.get());

    // Debug only: with tracing enabled, the message is built in a debug build and not otherwise.
    TreeDataGridDiagnostics::set_enable_tracing(true);
    assert!(TreeDataGridDiagnostics::enable_tracing());
    trace();
    let expected = if cfg!(debug_assertions) { 1 } else { 0 };
    assert_eq!(expected, messages_built.get());

    TreeDataGridDiagnostics::set_enable_tracing(false);
    trace();
    assert_eq!(expected, messages_built.get());
}
