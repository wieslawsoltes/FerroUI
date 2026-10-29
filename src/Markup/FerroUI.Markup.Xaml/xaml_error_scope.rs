//! Typed errors out of a scoped load.
//!
//! The members that markup calls through metadata (the deferred construction of a resource, the
//! invokers of markup members) have no error channel, so a fallible one throws: [`crate::throw`]
//! panics with the message of its [`XamlLoadException`], as upstream throws. A loader that wants
//! the error itself, typed and with its inner exception (the WinUI importer of
//! `ferroui-markup-xaml-winui`), runs its load in [`with_xaml_errors`]: inside the scope the throw
//! records the exception and unwinds with a private token, and the scope turns the token back into
//! `Err(exception)`.
//!
//! Deviation (DEVIATIONS.md, Markup metadata and markup events): an addition; without a scope a
//! throw panics exactly as before.
//!
//! - Only the throws of the runtime are turned into errors: any other panic (of a constructor, an
//!   event handler, a converter of the application) leaves the scope as it came.
//! - The token unwinds with `resume_unwind`, which does not run the panic hook: an expected error
//!   is not printed as a panic. Recovering it needs a build that unwinds (`panic = "unwind"`, which
//!   FerroUI needs anyway, `docs/binary-size.md` section 5.3); with `panic = "abort"` the throw
//!   aborts, as a panic does.
//! - The exception stays on its thread (it holds `Rc` handles, so it cannot be a panic payload);
//!   only the token crosses the unwinding.
//! - A `catch_unwind` between the throw and the scope that catches the token and carries on has
//!   handled the error: the scope then returns what the load returns, and the exception it had
//!   recorded is dropped.
//! - It is not a transaction: objects the load created before the error are not rolled back, so
//!   a scope is for a load into a fresh root.

use crate::XamlLoadException;
use std::cell::RefCell;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

ferroui_base::ferro_thread_local! {
    /// The scopes that are open on the thread, innermost last, each with the exception thrown in
    /// it, if one was.
    static SCOPES: RefCell<Vec<Option<XamlLoadException>>> = const { RefCell::new(Vec::new()) };
}

/// The payload of the unwinding of a throw inside a scope.
struct XamlErrorToken;

/// Runs `load`, and returns the exception a throw of the runtime raises inside it as `Err`,
/// instead of panicking. Panics that are not throws of the runtime continue to unwind. Scopes
/// nest: a throw goes to the innermost.
#[doc(hidden)]
pub fn with_xaml_errors<T>(load: impl FnOnce() -> Result<T, XamlLoadException>) -> Result<T, XamlLoadException> {
    SCOPES.with(|scopes| scopes.borrow_mut().push(None));
    let result = catch_unwind(AssertUnwindSafe(load));
    let error = SCOPES.with(|scopes| scopes.borrow_mut().pop().flatten());
    match result {
        Ok(result) => result,
        Err(payload) if payload.is::<XamlErrorToken>() => match error {
            Some(error) => Err(error),
            // A token without an exception: one of an inner scope that was not caught there,
            // which cannot happen; keep unwinding rather than invent an error.
            None => resume_unwind(payload),
        },
        Err(payload) => resume_unwind(payload),
    }
}

/// Throws `error`: into the innermost open scope, or else as a panic with its message.
pub(crate) fn throw(error: XamlLoadException) -> ! {
    let unthrown = SCOPES.with(|scopes| match scopes.borrow_mut().last_mut() {
        Some(scope) => {
            *scope = Some(error);
            None
        }
        None => Some(error),
    });
    match unthrown {
        None => resume_unwind(Box::new(XamlErrorToken)),
        Some(error) => panic!("{error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_throw_in_a_scope_is_its_error_with_the_inner_exception_and_scopes_nest() {
        let result: Result<(), _> = with_xaml_errors(|| {
            let inner: Result<(), _> = with_xaml_errors(|| crate::throw(Err(XamlLoadException::with_message("inner"))));
            assert_eq!(inner.unwrap_err().message(), "inner");
            crate::throw(Err(XamlLoadException::with_inner("outer", XamlLoadException::with_message("cause"))))
        });
        let error = result.unwrap_err();
        assert_eq!(error.message(), "outer");
        assert_eq!(error.inner_exception().unwrap().to_string(), "cause");
        assert!(SCOPES.with(|scopes| scopes.borrow().is_empty()));
    }

    #[test]
    fn a_load_that_succeeds_returns_its_value() {
        assert_eq!(42, with_xaml_errors(|| Ok(42)).unwrap());
        let error = with_xaml_errors::<()>(|| Err(XamlLoadException::with_message("returned"))).unwrap_err();
        assert_eq!(error.message(), "returned");
    }

    #[test]
    fn other_panics_continue_and_the_scope_is_closed() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _: Result<(), _> = with_xaml_errors(|| panic!("application panic"));
        }));
        assert_eq!(result.unwrap_err().downcast_ref::<&str>(), Some(&"application panic"));
        assert!(SCOPES.with(|scopes| scopes.borrow().is_empty()));
    }

    #[test]
    fn a_throw_without_a_scope_panics_with_its_message() {
        let result = catch_unwind(AssertUnwindSafe(|| crate::throw::<()>(Err(XamlLoadException::with_message("ordinary")))));
        assert_eq!(result.unwrap_err().downcast_ref::<String>().map(String::as_str), Some("ordinary"));
    }

    #[test]
    fn a_throw_caught_inside_the_scope_is_handled_there() {
        let result = with_xaml_errors(|| {
            let caught = catch_unwind(AssertUnwindSafe(|| crate::throw::<()>(Err(XamlLoadException::with_message("caught")))));
            assert!(caught.is_err());
            Ok("carried on")
        });
        assert_eq!("carried on", result.unwrap());
    }
}
