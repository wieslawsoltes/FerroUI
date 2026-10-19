//! `ExceptionTests` (`ReactiveUI.Tests/Core/ExceptionTests.cs`): `UnhandledErrorException`.

use ferroui_reactiveui::*;

#[test]
fn unhandled_error_exception_default_constructor_has_default_message() {
    // Act
    let fixture = UnhandledErrorException::new();

    // Assert
    assert_eq!(fixture.message(), "Exception of type 'ReactiveUI.UnhandledErrorException' was thrown.");
}

#[test]
fn unhandled_error_exception_message_and_inner_exception_has_both() {
    // Arrange
    const EXPECTED_MESSAGE: &str = "We are terribly sorry but a unhandled error occured.";
    const INNER_MESSAGE: &str = "Inner Exception added.";
    let inner_exception = PanicException { message: INNER_MESSAGE.to_owned() };

    // Act
    let fixture = UnhandledErrorException::with_inner(EXPECTED_MESSAGE, &inner_exception);

    // Assert
    assert_eq!(fixture.message(), EXPECTED_MESSAGE);
    assert!(fixture.inner_exception().is_some());
    assert_eq!(fixture.inner_exception(), Some(INNER_MESSAGE));
}

#[test]
fn unhandled_error_exception_message_constructor_has_custom_message() {
    // Arrange
    const EXPECTED_MESSAGE: &str = "We are terribly sorry but a unhandled error occured.";

    // Act
    let fixture = UnhandledErrorException::with_message(EXPECTED_MESSAGE);

    // Assert
    assert_eq!(fixture.message(), EXPECTED_MESSAGE);
}
