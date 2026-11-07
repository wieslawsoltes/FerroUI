//! Typed errors out of a scoped load.
//!
//! Some members that a load reaches have no error channel: the contracts the framework calls
//! (`IDeferredContent::build` of a deferred resource, the resource contracts of an include, the
//! `build` of a template) return a value, not a `Result`. A failing one throws, as upstream
//! throws: [`crate::throw`] panics with the message of its [`XamlLoadError`]. A loader that wants
//! the error itself, typed and with its inner exception (the WinUI importer of
//! `ferroui-markup-xaml-winui`), runs its load in [`with_xaml_errors`]: inside the scope a failing
//! member reports its exception to the scope instead of panicking ([`report`],
//! [`crate::throw_or`]), goes on as if it had produced nothing (a deferred resource that is not
//! there, an include without resources, a template without content), and the scope returns the
//! exception as `Err(exception)` when the load is done.
//!
//! Deviation (DEVIATIONS.md, Markup metadata and markup events; #567): an addition; without a
//! scope a throw panics exactly as before.
//!
//! - Nothing unwinds: the scope works the same with `panic = "abort"` (#567, the shipped build
//!   aborts on panic). Until 2026-11 the throw unwound to the scope with a private token; the
//!   scope no longer catches panics.
//! - Only the errors the runtime reports are turned into errors: any panic (of a constructor, an
//!   event handler, a converter of the application) leaves the scope as it came, and closes it.
//! - The first error reported is the error of the scope; later ones (often consequences of the
//!   first, such as a resource that is then not found) are dropped. A load can stop early when
//!   [`has_error`] says the scope already failed; the loader of `ferroui-markup-xaml-loader` stops
//!   at its next member call.
//! - The exception stays on its thread (it holds `Rc` handles).
//! - It is not a transaction: objects the load created before the error are not rolled back, so
//!   a scope is for a load into a fresh root.

use crate::XamlLoadError;
use std::cell::RefCell;

ferroui_base::ferro_thread_local! {
    /// The scopes that are open on the thread, innermost last, each with the first exception
    /// reported in it, if one was.
    static SCOPES: RefCell<Vec<Option<XamlLoadError>>> = const { RefCell::new(Vec::new()) };
}

/// An open scope; closing it, or dropping it when a panic unwinds out of the load, removes it
/// and the scopes opened inside it.
struct OpenScope {
    depth: usize,
}

impl OpenScope {
    fn open() -> Self {
        let depth = SCOPES.with(|scopes| {
            let mut scopes = scopes.borrow_mut();
            scopes.push(None);
            scopes.len() - 1
        });
        Self { depth }
    }

    /// Closes the scope and returns the exception reported in it.
    fn close(self) -> Option<XamlLoadError> {
        let error = SCOPES.with(|scopes| {
            let mut scopes = scopes.borrow_mut();
            let error = scopes.get_mut(self.depth).and_then(Option::take);
            scopes.truncate(self.depth);
            error
        });
        std::mem::forget(self);
        error
    }
}

impl Drop for OpenScope {
    fn drop(&mut self) {
        let _ = SCOPES.try_with(|scopes| scopes.borrow_mut().truncate(self.depth));
    }
}

/// Runs `load`, and returns the first exception a member of the runtime reports inside it as
/// `Err`, instead of panicking. Panics continue to unwind (and close the scope). Scopes nest: an
/// exception goes to the innermost.
#[doc(hidden)]
pub fn with_xaml_errors<T>(load: impl FnOnce() -> Result<T, XamlLoadError>) -> Result<T, XamlLoadError> {
    let scope = OpenScope::open();
    let result = load();
    match scope.close() {
        Some(error) => Err(error),
        None => result,
    }
}

/// Reports `error` to the innermost open scope, which keeps the first error reported in it.
/// Returns the error back when no scope is open, for the caller to throw.
#[doc(hidden)]
pub fn report(error: XamlLoadError) -> Result<(), XamlLoadError> {
    SCOPES.with(|scopes| match scopes.borrow_mut().last_mut() {
        Some(scope) => {
            if scope.is_none() {
                *scope = Some(error);
            }
            Ok(())
        }
        None => Err(error),
    })
}

/// Whether the innermost open scope has an error already: the load has failed, and a loader can
/// stop instead of going on with what the failing member produced.
#[doc(hidden)]
pub fn has_error() -> bool {
    SCOPES.with(|scopes| scopes.borrow().last().is_some_and(Option::is_some))
}

/// Throws `error`: panics with its message. The fatal fallback of a member without an error
/// channel and without a value to go on with; members that a scoped load reaches report to the
/// scope instead ([`report`], [`crate::throw_or`]). Public for the members of
/// `ferroui-markup-xaml-winui`, which throw as the runtime does.
#[doc(hidden)]
pub fn throw(error: XamlLoadError) -> ! {
    panic!("{error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    fn fail(message: &str) -> Result<u32, XamlLoadError> {
        Err(XamlLoadError::with_message(message))
    }

    #[test]
    fn a_report_in_a_scope_is_its_error_with_the_inner_exception_and_scopes_nest() {
        let result: Result<(), _> = with_xaml_errors(|| {
            let inner: Result<(), _> = with_xaml_errors(|| {
                assert_eq!(0, crate::throw_or(fail("inner"), || 0));
                Ok(())
            });
            assert_eq!(inner.unwrap_err().message(), "inner");
            assert!(!has_error());
            let outer = Err(XamlLoadError::with_inner("outer", XamlLoadError::with_message("cause")));
            crate::throw_or(outer, || ());
            assert!(has_error());
            Ok(())
        });
        let error = result.unwrap_err();
        assert_eq!(error.message(), "outer");
        assert_eq!(error.inner_exception().unwrap().to_string(), "cause");
        assert!(SCOPES.with(|scopes| scopes.borrow().is_empty()));
    }

    #[test]
    fn the_first_error_of_a_scope_is_kept_and_the_load_goes_on_with_the_fallback() {
        let mut went_on = false;
        let error = with_xaml_errors(|| {
            assert_eq!(7, crate::throw_or(fail("first"), || 7));
            assert_eq!(8, crate::throw_or(fail("second"), || 8));
            went_on = true;
            Ok(())
        })
        .unwrap_err();
        assert_eq!(error.message(), "first");
        assert!(went_on);
    }

    #[test]
    fn a_load_that_succeeds_returns_its_value() {
        assert_eq!(42, with_xaml_errors(|| Ok(42)).unwrap());
        let error = with_xaml_errors::<()>(|| Err(XamlLoadError::with_message("returned"))).unwrap_err();
        assert_eq!(error.message(), "returned");
        // A reported error wins over the error the load returns because of it.
        let error = with_xaml_errors::<()>(|| {
            report(XamlLoadError::with_message("cause")).unwrap();
            Err(XamlLoadError::with_message("consequence"))
        })
        .unwrap_err();
        assert_eq!(error.message(), "cause");
    }

    #[test]
    fn panics_continue_and_the_scope_is_closed() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _: Result<(), _> = with_xaml_errors(|| {
                let _: Result<(), _> = with_xaml_errors(|| panic!("application panic"));
                Ok(())
            });
        }));
        assert_eq!(result.unwrap_err().downcast_ref::<&str>(), Some(&"application panic"));
        assert!(SCOPES.with(|scopes| scopes.borrow().is_empty()));
    }

    #[test]
    fn without_a_scope_a_failing_member_panics_with_its_message() {
        assert!(!has_error());
        assert_eq!(report(XamlLoadError::with_message("returned")).unwrap_err().message(), "returned");
        let result = catch_unwind(AssertUnwindSafe(|| crate::throw_or(fail("ordinary"), || 0)));
        assert_eq!(result.unwrap_err().downcast_ref::<String>().map(String::as_str), Some("ordinary"));
        let result = catch_unwind(AssertUnwindSafe(|| crate::throw::<()>(Err(XamlLoadError::with_message("fatal")))));
        assert_eq!(result.unwrap_err().downcast_ref::<String>().map(String::as_str), Some("fatal"));
    }
}
