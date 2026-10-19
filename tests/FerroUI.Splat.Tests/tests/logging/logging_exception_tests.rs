//! Ported from `Splat.Logging.Tests/LoggingExceptionTests.cs` of Splat.
//! Adapted: the inner exception is a boxed `TestException` (upstream an
//! `InvalidOperationException`); `IsSameReferenceAs` compares the address of the boxed exception
//! with the address of the inner exception the `LoggingException` hands back. Throwing and
//! catching is a panic with the exception as its payload (`std::panic::panic_any`), as the library
//! raises it, caught with `catch_unwind` and downcast.

use crate::tests::common::{panic_payload, TestException};
use ferroui_splat::LoggingException;
use std::error::Error;

const TEST_EXCEPTION_MESSAGE: &str = "Test message";

fn address_of(exception: &(dyn Error + Send + Sync)) -> *const () {
    std::ptr::from_ref(exception).cast::<()>()
}

#[test]
fn constructor_with_message_should_set_message() {
    let exception = LoggingException::with_message(TEST_EXCEPTION_MESSAGE);

    assert_eq!(exception.message(), TEST_EXCEPTION_MESSAGE);
}

#[test]
fn constructor_with_message_and_inner_exception_should_set_both() {
    let inner: Box<dyn Error + Send + Sync> = Box::new(TestException::with_message("Inner"));
    let inner_address = address_of(&*inner);
    let exception = LoggingException::with_inner(TEST_EXCEPTION_MESSAGE, inner);

    assert_eq!(exception.message(), TEST_EXCEPTION_MESSAGE);
    let actual = exception.inner_exception().expect("the inner exception is set");
    assert_eq!(address_of(actual), inner_address);
}

#[test]
fn logging_exception_can_be_thrown_and_caught() {
    let payload = panic_payload(|| std::panic::panic_any(LoggingException::with_message("Test")))
        .expect("the exception is thrown");

    let exception = payload.downcast_ref::<LoggingException>().expect("the payload is a LoggingException");
    assert_eq!(exception.message(), "Test");
}
