//! `ResolverExceptionHelpers` and `ObjectDisposedExceptionHelper`.

use std::panic::{catch_unwind, AssertUnwindSafe};

/// Runs `action` and swallows a panic from it (`RunSwallowingExceptions`): the callbacks and the
/// disposals a resolver runs while it is disposed or notifies must not stop the others. The panic
/// hook has reported the panic already, as `Debug.WriteLine` does upstream.
// Deviation (DEVIATIONS.md, Splat): an exception is a panic, caught with `catch_unwind`.
pub(crate) fn run_swallowing_exceptions(action: impl FnOnce()) {
    let _ = catch_unwind(AssertUnwindSafe(action));
}

/// Panics as `ObjectDisposedException` is thrown for a resolver that was disposed.
pub(crate) fn throw_object_disposed(object_name: &str) -> ! {
    panic!("Cannot access a disposed object.\nObject name: '{object_name}'.")
}
