//! An opt-in error bridge for a fresh-root runtime load. Deferred resource interfaces cannot
//! return errors, so they traditionally throw. This scope preserves their exact XAML errors
//! while leaving programmer panics and normal loader behavior unchanged.
//!
//! This is not a transaction: never use it to claim rollback of an existing root instance.
//! Recovering a deferred error requires a Rust build with panic unwinding enabled.

use crate::XamlLoadException;
use std::cell::RefCell;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

ferroui_base::ferro_thread_local! {
    static SCOPES: RefCell<Vec<Option<XamlLoadException>>> = const { RefCell::new(Vec::new()) };
}

// The exception owns UI-thread Rc handles and cannot be a Send panic payload. Keep it on
// its originating thread; only this private marker crosses the unwind boundary.
struct XamlErrorToken;

/// Handles only errors raised by the XAML runtime's fallible throw helper. All other
/// panic payloads resume unwinding, including user control/event/constructor panics.
pub fn with_xaml_errors<T>(
    load: impl FnOnce() -> Result<T, XamlLoadException>,
) -> Result<T, XamlLoadException> {
    SCOPES.with(|scopes| scopes.borrow_mut().push(None));
    let result = catch_unwind(AssertUnwindSafe(load));
    let error = SCOPES.with(|scopes| scopes.borrow_mut().pop().flatten());
    match result {
        Ok(result) => result,
        Err(payload) if payload.is::<XamlErrorToken>() => {
            Err(error.expect("a XAML error token has an error in its active scope"))
        }
        Err(payload) => resume_unwind(payload),
    }
}

pub(crate) fn throw(error: XamlLoadException) -> ! {
    let scoped = SCOPES.with(|scopes| {
        let mut scopes = scopes.borrow_mut();
        if let Some(scope) = scopes.last_mut() {
            *scope = Some(error.clone());
            true
        } else {
            false
        }
    });
    if scoped {
        // This is an expected, typed error transfer. It does not invoke a process-wide panic
        // hook or change that hook for other code running concurrently.
        resume_unwind(Box::new(XamlErrorToken));
    }
    panic!("{error}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_errors_preserve_inner_error_and_nested_scopes_are_independent() {
        let result: Result<(), _> = with_xaml_errors(|| {
            let inner: Result<(), _> =
                with_xaml_errors(|| crate::throw(Err(XamlLoadException::with_message("inner"))));
            assert_eq!(inner.unwrap_err().message(), "inner");
            crate::throw(Err(XamlLoadException::with_inner(
                "outer",
                XamlLoadException::with_message("cause"),
            )))
        });
        let error = result.unwrap_err();
        assert_eq!(error.message(), "outer");
        assert_eq!(error.inner_exception().unwrap().to_string(), "cause");
    }

    #[test]
    fn unrelated_panics_resume_and_the_scope_is_removed() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _: Result<(), _> = with_xaml_errors(|| panic!("application panic"));
        }));
        assert_eq!(
            result.unwrap_err().downcast_ref::<&str>(),
            Some(&"application panic")
        );
        assert!(SCOPES.with(|scopes| scopes.borrow().is_empty()));
    }

    #[test]
    fn ordinary_throw_still_panics_with_its_original_message() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            crate::throw::<()>(Err(XamlLoadException::with_message("ordinary")))
        }));
        assert_eq!(
            result
                .unwrap_err()
                .downcast_ref::<String>()
                .map(String::as_str),
            Some("ordinary")
        );
    }
}
