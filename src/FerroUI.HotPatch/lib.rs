//! Explicit synchronous patch boundaries and conservative source admission.
//! A supervisor must validate the entire build input set, not just a changed function.
#[cfg(debug_assertions)]
mod supervisor;
#[cfg(debug_assertions)]
pub use subsecond::{aslr_reference, JumpTable};
#[cfg(debug_assertions)]
pub use supervisor::{AdmittedPatch, PatchSupervisor, SourceSnapshot};
#[cfg(debug_assertions)]
mod registration;
#[cfg(debug_assertions)]
pub use registration::FunctionRegistration;
mod admission;
pub use admission::{admit, Admission};
/// Invoke at the callback boundary, each time the callback runs.
#[inline(always)]
pub fn hot_reloadable<R>(function: impl FnMut() -> R) -> R {
    #[cfg(debug_assertions)]
    {
        let _exit = CallbackGuard::enter();
        subsecond::call(function)
    }
    #[cfg(not(debug_assertions))]
    {
        let mut function = function;
        function()
    }
}

/// Invoke a registered synchronous function with a handler or command argument.
/// Pass a tuple for multiple arguments. Call this at every invocation of the
/// existing subscription; the function pointer is resolved through the current
/// patch table each time. Argument and return types must live in stable code.
/// This does not admit a function or rebuild its view: the owning supervisor
/// still validates the complete source revision before installing a patch.
#[inline(always)]
pub fn hot_reloadable_with<A, R>(function: fn(A) -> R, argument: A) -> R {
    #[cfg(debug_assertions)]
    {
        let _exit = CallbackGuard::enter();
        subsecond::HotFn::current(function).call((argument,))
    }
    #[cfg(not(debug_assertions))]
    {
        function(argument)
    }
}

#[cfg(debug_assertions)]
struct CallbackGuard;
#[cfg(debug_assertions)]
impl CallbackGuard {
    fn enter() -> Self {
        DEPTH.with(|depth| depth.set(depth.get() + 1));
        Self
    }
}
#[cfg(debug_assertions)]
impl Drop for CallbackGuard {
    fn drop(&mut self) {
        DEPTH.with(|depth| depth.set(depth.get() - 1));
    }
}

#[cfg(debug_assertions)]
ferroui_system::ferro_thread_local! { static DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
#[cfg(debug_assertions)]
fn in_callback() -> bool {
    DEPTH.with(|depth| depth.get() != 0)
}

#[cfg(debug_assertions)]
pub mod restart;

#[cfg(debug_assertions)]
pub mod inputs;

#[cfg(all(test, debug_assertions))]
mod callback_tests {
    use super::*;

    #[test]
    fn typed_callback_preserves_arguments_and_nested_quiescence() {
        fn command((mut text, delta): (String, usize)) -> String {
            assert!(in_callback());
            hot_reloadable(|| assert!(in_callback()));
            assert!(in_callback());
            text.push_str(&delta.to_string());
            text
        }
        assert!(!in_callback());
        assert_eq!(hot_reloadable_with(command, ("count=".into(), 3)), "count=3");
        assert!(!in_callback());
        fn handler(value: &mut i32) {
            assert!(in_callback());
            *value += 1;
        }
        let mut value = 7;
        hot_reloadable_with(handler, &mut value);
        assert_eq!(value, 8);
        assert!(!in_callback());
    }

    #[test]
    fn panicking_typed_callback_releases_quiescence_guard() {
        fn handler(_: ()) {
            assert!(in_callback());
            panic!("application handler failed");
        }
        assert!(std::panic::catch_unwind(|| hot_reloadable_with(handler, ())).is_err());
        assert!(!in_callback());
    }
}
