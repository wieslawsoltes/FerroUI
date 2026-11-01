//! Ported from `Splat.Logging.Tests/LoggingExceptionCoverageTests.cs` of Splat.
//! Adapted: the inner exception is a boxed `TestException` (upstream an
//! `InvalidOperationException`); `IsSameReferenceAs` compares the address of the boxed exception
//! with the address of the inner exception the `LoggingError` hands back. `IsNotNull` on a
//! value holds by construction.

use crate::tests::common::TestException;
use ferroui_splat::LoggingError;
use std::error::Error;

fn address_of(exception: &(dyn Error + Send + Sync)) -> *const () {
    std::ptr::from_ref(exception).cast::<()>()
}

#[test]
fn default_constructor_should_create_exception() {
    let exception = LoggingError::new();

    assert_eq!(exception.message(), "Exception of type 'LoggingException' was thrown.");
    assert!(exception.inner_exception().is_none());
}

#[test]
fn message_constructor_should_set_message_and_no_inner() {
    const MESSAGE: &str = "logging failed";
    let exception = LoggingError::with_message(MESSAGE);

    assert_eq!(exception.message(), MESSAGE);
    assert!(exception.inner_exception().is_none());
}

#[test]
fn message_and_inner_constructor_should_set_both() {
    const MESSAGE: &str = "logging failed";
    let inner: Box<dyn Error + Send + Sync> = Box::new(TestException::with_message("inner"));
    let inner_address = address_of(&*inner);
    let exception = LoggingError::with_inner(MESSAGE, inner);

    assert_eq!(exception.message(), MESSAGE);
    let actual = exception.inner_exception().expect("the inner exception is set");
    assert_eq!(address_of(actual), inner_address);
}
