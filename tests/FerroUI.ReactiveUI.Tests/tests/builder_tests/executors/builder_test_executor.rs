//! `BuilderTestExecutor` (`ReactiveUI.Builder.Tests/Executors/BuilderTestExecutor.cs`), the
//! executor of every test of the assembly that names no other one (`AssemblyHooks.Parallel.cs`).

use super::{configure_builder, execute_test, reset_state};

/// Runs `action` after resetting the state and building the core services, and resets the state
/// after it (`BuilderTestExecutor`).
pub fn builder_test<R>(action: impl FnOnce() -> R) -> R {
    execute_test(reset_state, configure_builder, action)
}
