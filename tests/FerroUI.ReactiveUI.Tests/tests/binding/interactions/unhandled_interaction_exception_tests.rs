//! `UnhandledInteractionExceptionTests` (ReactiveUI.Binding.Tests,
//! `Interactions/UnhandledInteractionExceptionTests.cs`). The parameterless constructor is
//! `UnhandledInteractionException::empty`.

use crate::mocks::*;
use ferroui_reactiveui::*;
use std::rc::Rc;

#[test]
fn parameterless_constructor_creates_instance() {
    let ex = UnhandledInteractionException::<String, bool>::empty();

    assert!(ex.interaction().is_none());
    assert!(!ex.message().is_empty());
}

#[test]
fn message_constructor_sets_message() {
    let ex = UnhandledInteractionException::<String, bool>::with_message("test message");

    assert_eq!(ex.message(), "test message");
    assert!(ex.interaction().is_none());
}

#[test]
fn message_and_inner_exception_constructor_sets_both() {
    let inner = InvalidOperationException::error("inner");

    let ex = UnhandledInteractionException::<String, bool>::with_inner("outer", inner.clone());

    assert_eq!(ex.message(), "outer");
    assert!(ex.inner_exception().is_some_and(|error| Rc::ptr_eq(error, &inner)));
}
