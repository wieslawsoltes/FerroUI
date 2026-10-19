//! `MessageBusTest` (`ReactiveUI.Tests/MessageBus/MessageBusTest.cs`).

use super::with_message_bus;
use crate::mocks::collect_items;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{duration_to_ticks, TestScheduler};
use ferroui_reactiveui::*;
use ferroui_system::IObserver;
use std::any::TypeId;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use std::time::Duration;

const MESSAGE_VALUE: i32 = 42;
const SECOND_MESSAGE_VALUE: i32 = 100;
const TEST_MESSAGE1: &str = "Test";
const CONTRACT1: &str = "Contract1";
const CONTRACT2: &str = "Contract2";
const BEFORE_MESSAGE: &str = "Before";
const AFTER_MESSAGE: &str = "After";
const FIRST_MESSAGE: &str = "First";
const SECOND_TEXT_MESSAGE: &str = "Second";
const THIRD_MESSAGE: &str = "Third";
const HELLO_MESSAGE: &str = "Hello";
const WORLD_MESSAGE: &str = "World";
const TEST_CONTRACT: &str = "TestContract";

/// A message of several fields (`TestMessage`), compared by identity as the class of upstream.
#[derive(Clone, Debug, Default)]
struct TestMessage {
    id: i32,
    text: Option<String>,
}

#[derive(Debug)]
struct TestError;

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Test error")
    }
}

impl std::error::Error for TestError {}

/// One tick of virtual time (`TimeSpan.FromTicks(1)`).
fn one_tick() -> i64 {
    duration_to_ticks(Duration::from_nanos(100))
}

#[test]
fn current_can_get_and_set() {
    with_message_bus(|| {
        let custom_bus: Rc<dyn IMessageBus> = MessageBus::new();
        let original = MessageBus::current();

        MessageBus::set_current(custom_bus.clone());
        assert!(Rc::ptr_eq(&MessageBus::current(), &custom_bus));

        MessageBus::set_current(original);
    });
}

#[test]
fn is_registered_after_listen_returns_true() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        let _ = message_bus.listen::<i32>().subscribe_ignore();
        assert!(message_bus.is_registered(TypeId::of::<i32>()));
    });
}

#[test]
fn is_registered_after_send_message_returns_true() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        message_bus.send_message(TEST_MESSAGE1.to_owned());
        assert!(message_bus.is_registered(TypeId::of::<String>()));
    });
}

#[test]
fn is_registered_before_messages_returns_false() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        assert!(!message_bus.is_registered(TypeId::of::<String>()));
    });
}

#[test]
fn is_registered_different_type_returns_false() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        message_bus.send_message(MESSAGE_VALUE);
        assert!(message_bus.is_registered(TypeId::of::<i32>()));
        assert!(!message_bus.is_registered(TypeId::of::<String>()));
    });
}

#[test]
fn is_registered_with_contract_distinguishes_contracts() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        message_bus.send_message_with_contract(TEST_MESSAGE1.to_owned(), Some(CONTRACT1));
        assert!(message_bus.is_registered_with_contract(TypeId::of::<String>(), Some(CONTRACT1)));
        assert!(!message_bus.is_registered_with_contract(TypeId::of::<String>(), Some(CONTRACT2)));
        assert!(!message_bus.is_registered(TypeId::of::<String>()));
    });
}

#[test]
fn listen_cold_observable_no_side_effects() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        let observable = message_bus.listen::<String>();
        assert!(!message_bus.is_registered(TypeId::of::<String>()));

        let _ = observable.subscribe_ignore();
        assert!(message_bus.is_registered(TypeId::of::<String>()));
    });
}

#[test]
fn listen_messages_sent_before_subscription_are_not_received() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        message_bus.send_message("Before1".to_owned());
        message_bus.send_message("Before2".to_owned());

        let messages = collect_items(&message_bus.listen::<String>());
        message_bus.send_message(AFTER_MESSAGE.to_owned());

        assert_eq!(*messages.borrow(), [AFTER_MESSAGE]);
    });
}

#[test]
fn listen_skips_initial_value() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        let messages = collect_items(&message_bus.listen::<i32>());
        assert!(messages.borrow().is_empty());

        message_bus.send_message(MESSAGE_VALUE);
        assert_eq!(*messages.borrow(), [MESSAGE_VALUE]);
    });
}

#[test]
fn listen_unsubscribe_stops_receiving_messages() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        let messages = Rc::new(RefCell::new(Vec::new()));
        let log = messages.clone();
        let subscription = message_bus.listen::<String>().subscribe_next(move |message| log.borrow_mut().push(message));

        message_bus.send_message(BEFORE_MESSAGE.to_owned());
        subscription.dispose();
        message_bus.send_message(AFTER_MESSAGE.to_owned());

        assert_eq!(*messages.borrow(), [BEFORE_MESSAGE]);
    });
}

#[test]
fn listen_include_latest_messages_sent_before_subscription_receives_latest() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        message_bus.send_message(FIRST_MESSAGE.to_owned());
        message_bus.send_message(SECOND_TEXT_MESSAGE.to_owned());
        message_bus.send_message(THIRD_MESSAGE.to_owned());

        let messages = collect_items(&message_bus.listen_include_latest::<String>());
        assert_eq!(*messages.borrow(), [THIRD_MESSAGE]);

        message_bus.send_message("Fourth".to_owned());
        assert_eq!(*messages.borrow(), [THIRD_MESSAGE, "Fourth"]);
    });
}

#[test]
fn listen_include_latest_receives_initial_value() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        let messages = collect_items(&message_bus.listen_include_latest::<i32>());
        assert_eq!(*messages.borrow(), [0]);

        message_bus.send_message(MESSAGE_VALUE);
        assert_eq!(*messages.borrow(), [0, MESSAGE_VALUE]);
    });
}

#[test]
fn listen_include_latest_receives_latest_message() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        message_bus.send_message(FIRST_MESSAGE.to_owned());
        message_bus.send_message(SECOND_TEXT_MESSAGE.to_owned());

        let messages = collect_items(&message_bus.listen_include_latest::<String>());

        assert_eq!(*messages.borrow(), [SECOND_TEXT_MESSAGE]);
    });
}

#[test]
fn register_message_source_dispose_stops_sending_messages() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        let source = Subject::<String>::new();
        let messages = collect_items(&message_bus.listen::<String>());

        let subscription = message_bus.register_message_source(source.as_observable());
        source.on_next(BEFORE_MESSAGE.to_owned());
        subscription.dispose();
        source.on_next(AFTER_MESSAGE.to_owned());

        assert_eq!(*messages.borrow(), [BEFORE_MESSAGE]);
    });
}

// Not ported: RegisterMessageSource_NullSource_ThrowsArgumentNullException - argument validation

#[test]
fn register_message_source_observable_complete_unsubscribes_correctly() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        let source = Subject::<String>::new();
        let messages = collect_items(&message_bus.listen::<String>());

        let _ = message_bus.register_message_source(source.as_observable());
        source.on_next(BEFORE_MESSAGE.to_owned());
        source.on_completed();
        message_bus.send_message(AFTER_MESSAGE.to_owned());

        assert_eq!(*messages.borrow(), [BEFORE_MESSAGE, AFTER_MESSAGE]);
    });
}

#[test]
fn register_message_source_observable_error_does_not_break_bus() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        let source = Subject::<String>::new();
        let messages = collect_items(&message_bus.listen::<String>());

        let _ = message_bus.register_message_source(source.as_observable());
        source.on_next(BEFORE_MESSAGE.to_owned());
        source.on_error(Rc::new(TestError));
        message_bus.send_message(AFTER_MESSAGE.to_owned());

        assert_eq!(*messages.borrow(), [BEFORE_MESSAGE, AFTER_MESSAGE]);
    });
}

#[test]
fn register_message_source_sends_messages_from_observable() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        let source = Subject::<String>::new();
        let messages = collect_items(&message_bus.listen::<String>());

        let _ = message_bus.register_message_source(source.as_observable());
        source.on_next(FIRST_MESSAGE.to_owned());
        source.on_next(SECOND_TEXT_MESSAGE.to_owned());
        source.on_next(THIRD_MESSAGE.to_owned());

        assert_eq!(*messages.borrow(), [FIRST_MESSAGE, SECOND_TEXT_MESSAGE, THIRD_MESSAGE]);
    });
}

#[test]
fn register_message_source_with_contract_sends_to_correct_listeners() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        let source = Subject::<i32>::new();
        let contract_messages = collect_items(&message_bus.listen_with_contract::<i32>(Some("MyContract")));
        let no_contract_messages = collect_items(&message_bus.listen::<i32>());
        const SECOND_MESSAGE: i32 = 2;

        let _ = message_bus.register_message_source_with_contract(source.as_observable(), Some("MyContract"));
        source.on_next(1);
        source.on_next(SECOND_MESSAGE);

        assert_eq!(*contract_messages.borrow(), [1, SECOND_MESSAGE]);
        assert!(no_contract_messages.borrow().is_empty());
    });
}

#[test]
fn register_scheduler_affects_message_delivery() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        let scheduler = TestScheduler::new();
        message_bus.register_scheduler::<String>(scheduler.scheduler());
        let messages = collect_items(&message_bus.listen::<String>());

        message_bus.send_message(TEST_MESSAGE1.to_owned());
        assert!(messages.borrow().is_empty());

        scheduler.advance_by(one_tick());
        assert_eq!(*messages.borrow(), [TEST_MESSAGE1]);
    });
}

#[test]
fn register_scheduler_multiple_overwrites_previous() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        let scheduler1 = TestScheduler::new();
        let scheduler2 = TestScheduler::new();
        message_bus.register_scheduler::<String>(scheduler1.scheduler());
        message_bus.register_scheduler::<String>(scheduler2.scheduler());
        let messages = collect_items(&message_bus.listen::<String>());

        message_bus.send_message(TEST_MESSAGE1.to_owned());
        assert!(messages.borrow().is_empty());

        scheduler1.advance_by(one_tick());
        assert!(messages.borrow().is_empty());

        scheduler2.advance_by(one_tick());
        assert_eq!(*messages.borrow(), [TEST_MESSAGE1]);
    });
}

#[test]
fn register_scheduler_with_contract_only_affects_contract() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        let scheduler = TestScheduler::new();
        message_bus.register_scheduler_with_contract::<String>(scheduler.scheduler(), Some(TEST_CONTRACT));
        let contract_messages = collect_items(&message_bus.listen_with_contract::<String>(Some(TEST_CONTRACT)));
        let normal_messages = collect_items(&message_bus.listen::<String>());

        message_bus.send_message_with_contract("Contract".to_owned(), Some(TEST_CONTRACT));
        message_bus.send_message("Normal".to_owned());

        assert!(contract_messages.borrow().is_empty());
        assert_eq!(*normal_messages.borrow(), ["Normal"]);

        scheduler.advance_by(one_tick());
        assert_eq!(*contract_messages.borrow(), ["Contract"]);
    });
}

#[test]
fn send_message_complex_object_works_correctly() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        let messages = collect_items(&message_bus.listen::<Rc<TestMessage>>());
        const SECOND_ID: i32 = 2;
        let msg1 = Rc::new(TestMessage { id: 1, text: Some(FIRST_MESSAGE.to_owned()) });
        let msg2 = Rc::new(TestMessage { id: SECOND_ID, text: Some(SECOND_TEXT_MESSAGE.to_owned()) });

        message_bus.send_message(msg1.clone());
        message_bus.send_message(msg2.clone());

        let messages = messages.borrow();
        assert_eq!(messages.len(), 2);
        assert!(Rc::ptr_eq(&messages[0], &msg1));
        assert!(Rc::ptr_eq(&messages[1], &msg2));
        assert_eq!(messages[0].id, 1);
        assert_eq!(messages[0].text.as_deref(), Some(FIRST_MESSAGE));
        assert_eq!(messages[1].id, SECOND_ID);
        assert_eq!(messages[1].text.as_deref(), Some(SECOND_TEXT_MESSAGE));
    });
}

// Not ported: SendMessage_ConcurrentCalls_ThreadSafe - real threads

#[test]
fn send_message_different_types_are_independent() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        let string_messages = collect_items(&message_bus.listen::<String>());
        let int_messages = collect_items(&message_bus.listen::<i32>());

        message_bus.send_message(HELLO_MESSAGE.to_owned());
        message_bus.send_message(MESSAGE_VALUE);
        message_bus.send_message(WORLD_MESSAGE.to_owned());
        message_bus.send_message(SECOND_MESSAGE_VALUE);

        assert_eq!(*string_messages.borrow(), [HELLO_MESSAGE, WORLD_MESSAGE]);
        assert_eq!(*int_messages.borrow(), [MESSAGE_VALUE, SECOND_MESSAGE_VALUE]);
    });
}

#[test]
fn send_message_listen_null_vs_empty_contract_are_different() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        let null_messages = collect_items(&message_bus.listen::<String>());
        let empty_messages = collect_items(&message_bus.listen_with_contract::<String>(Some("")));

        message_bus.send_message("Null".to_owned());
        message_bus.send_message_with_contract("Empty".to_owned(), Some(""));

        assert_eq!(*null_messages.borrow(), ["Null"]);
        assert_eq!(*empty_messages.borrow(), ["Empty"]);
    });
}
