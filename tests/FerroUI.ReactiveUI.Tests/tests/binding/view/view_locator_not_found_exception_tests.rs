//! `ViewLocatorNotFoundExceptionTests` (ReactiveUI.Binding.Tests,
//! `View/ViewLocatorNotFoundExceptionTests.cs`).

use ferroui_reactiveui::*;
use std::error::Error;
use std::fmt;

/// An error that stands for the inner `InvalidOperationException`.
#[derive(Debug)]
struct InnerError;

impl fmt::Display for InnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("inner")
    }
}

impl Error for InnerError {}

#[test]
fn default_constructor_has_message() {
    let ex = ViewLocatorNotFoundException::new();

    assert!(!ex.message().is_empty());
    assert!(ex.message().contains("IViewLocator"));
}

#[test]
fn message_constructor_preserves_message() {
    let ex = ViewLocatorNotFoundException::with_message("custom message");

    assert_eq!(ex.message(), "custom message");
}

#[test]
fn message_and_inner_constructor_preserves_both() {
    let inner = InnerError;
    let ex = ViewLocatorNotFoundException::with_inner("outer", &inner);

    assert_eq!(ex.message(), "outer");
    assert_eq!(ex.inner_exception(), Some("inner"));
}
