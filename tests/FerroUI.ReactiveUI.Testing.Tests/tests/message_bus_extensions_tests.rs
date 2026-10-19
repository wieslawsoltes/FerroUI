//! `MessageBusExtensionsTests` (`ReactiveUI.Testing.Tests/MessageBusExtensionsTests.cs`).
//!
//! Upstream runs each test with `WithMessageBusExecutor`, which builds the application with a
//! message bus of its own and restores the previous one afterwards; the current message bus is per
//! thread here, and each test runs on a thread of its own.

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::assert_throws;
use ferroui_reactiveui::{IMessageBus, IMessageBusExt, MessageBus};
use ferroui_reactiveui_testing::MessageBusExtensions;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// What the function of the tests returns.
const EXPECTED_FUNCTION_RESULT: i32 = 42;

/// A new message bus (`new MessageBus()`).
fn new_bus() -> Rc<dyn IMessageBus> {
    MessageBus::new()
}

/// Whether `bus` is the current message bus (`IsSameReferenceAs`).
fn is_current(bus: &Rc<dyn IMessageBus>) -> bool {
    Rc::ptr_eq(&MessageBus::current(), bus)
}

#[test]
fn with_message_bus_restores_original_message_bus_when_disposed() {
    // Arrange
    let original_bus = MessageBus::current();
    let test_bus = new_bus();

    // Act
    let restore = test_bus.with_message_bus();
    // Message bus should be changed.
    assert!(is_current(&test_bus));
    restore.dispose();

    // Assert - Original should be restored.
    assert!(is_current(&original_bus));
}

#[test]
fn with_message_bus_allows_message_bus_operations() {
    // Arrange
    let test_bus = new_bus();
    let message_received = Rc::new(Cell::new(false));

    let restore = test_bus.with_message_bus();
    let received = message_received.clone();
    let _subscription = MessageBus::current().listen::<String>().subscribe_next(move |_| received.set(true));

    // Act
    MessageBus::current().send_message("test".to_string());
    restore.dispose();

    // Assert
    assert!(message_received.get());
}

#[test]
fn with_action_executes_action_with_message_bus() {
    // Arrange
    let test_bus = new_bus();
    let executed = Cell::new(false);
    let captured_bus: RefCell<Option<Rc<dyn IMessageBus>>> = RefCell::new(None);

    // Act
    test_bus.with(|| {
        executed.set(true);
        *captured_bus.borrow_mut() = Some(MessageBus::current());
    });

    // Assert
    assert!(executed.get());
    assert!(Rc::ptr_eq(captured_bus.borrow().as_ref().expect("the bus was captured"), &test_bus));
}

#[test]
fn with_action_restores_original_message_bus() {
    // Arrange
    let original_bus = MessageBus::current();
    let test_bus = new_bus();

    // Act
    test_bus.with(|| {
        // Do nothing.
    });

    // Assert
    assert!(is_current(&original_bus));
}

// Not ported: With_Action_ThrowsArgumentNullException_WhenBlockIsNull - argument validation

#[test]
fn with_function_executes_function_with_message_bus() {
    // Arrange
    let test_bus = new_bus();
    let captured_bus: RefCell<Option<Rc<dyn IMessageBus>>> = RefCell::new(None);

    // Act
    let result = test_bus.with(|| {
        *captured_bus.borrow_mut() = Some(MessageBus::current());
        EXPECTED_FUNCTION_RESULT
    });

    // Assert
    assert_eq!(result, EXPECTED_FUNCTION_RESULT);
    assert!(Rc::ptr_eq(captured_bus.borrow().as_ref().expect("the bus was captured"), &test_bus));
}

#[test]
fn with_function_restores_original_message_bus() {
    // Arrange
    let original_bus = MessageBus::current();
    let test_bus = new_bus();

    // Act
    let _ = test_bus.with(|| EXPECTED_FUNCTION_RESULT);

    // Assert
    assert!(is_current(&original_bus));
}

// Not ported: With_Function_ThrowsArgumentNullException_WhenBlockIsNull - argument validation

#[test]
fn with_function_returns_correct_value() {
    // Arrange
    let test_bus = new_bus();

    // Act
    let result = test_bus.with(|| "test result");

    // Assert
    assert_eq!(result, "test result");
}

#[test]
fn with_message_bus_can_be_nested() {
    // Arrange
    let original_bus = MessageBus::current();
    let test_bus1 = new_bus();
    let test_bus2 = new_bus();

    // Act
    let outer = test_bus1.with_message_bus();
    assert!(is_current(&test_bus1));

    let inner = test_bus2.with_message_bus();
    assert!(is_current(&test_bus2));
    inner.dispose();

    assert!(is_current(&test_bus1));
    outer.dispose();

    // Assert
    assert!(is_current(&original_bus));
}

#[test]
fn with_propagates_exceptions() {
    // Arrange
    let original_bus = MessageBus::current();
    let test_bus = new_bus();

    // Act & Assert: the exception is a panic here.
    assert_throws(|| test_bus.with::<()>(|| panic!("Test exception")));

    // The original message bus is restored on the way out.
    assert!(is_current(&original_bus));
}
