//! `MessageBusTest.SendMessage` (`ReactiveUI.Tests/MessageBus/MessageBusTest.SendMessage.cs`).

use super::with_message_bus;
use crate::mocks::collect_items;
use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use std::cell::RefCell;
use std::rc::Rc;

const MESSAGE_VALUE: i32 = 42;
const SECOND_MESSAGE_VALUE: i32 = 100;
const CONTRACT1: &str = "Contract1";
const CONTRACT2: &str = "Contract2";
const HELLO_MESSAGE: &str = "Hello";
const WORLD_MESSAGE: &str = "World";

#[test]
fn send_message_listen_receives_message() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        let messages = collect_items(&message_bus.listen::<String>());

        message_bus.send_message(HELLO_MESSAGE.to_owned());
        message_bus.send_message(WORLD_MESSAGE.to_owned());

        assert_eq!(*messages.borrow(), [HELLO_MESSAGE, WORLD_MESSAGE]);
    });
}

#[test]
fn send_message_listen_with_contract_distinguishes_messages() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        let messages1 = collect_items(&message_bus.listen_with_contract::<String>(Some(CONTRACT1)));
        let messages2 = collect_items(&message_bus.listen_with_contract::<String>(Some(CONTRACT2)));

        message_bus.send_message_with_contract("Message1".to_owned(), Some(CONTRACT1));
        message_bus.send_message_with_contract("Message2".to_owned(), Some(CONTRACT2));
        message_bus.send_message_with_contract("Message3".to_owned(), Some(CONTRACT1));

        assert_eq!(*messages1.borrow(), ["Message1", "Message3"]);
        assert_eq!(*messages2.borrow(), ["Message2"]);
    });
}

#[test]
fn send_message_multiple_subscribers_all_receive_message() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        let subscriber1 = collect_items(&message_bus.listen::<i32>());
        let subscriber2 = collect_items(&message_bus.listen::<i32>());
        let subscriber3 = collect_items(&message_bus.listen::<i32>());

        message_bus.send_message(MESSAGE_VALUE);

        assert_eq!(*subscriber1.borrow(), [MESSAGE_VALUE]);
        assert_eq!(*subscriber2.borrow(), [MESSAGE_VALUE]);
        assert_eq!(*subscriber3.borrow(), [MESSAGE_VALUE]);
    });
}

#[test]
fn send_message_nullable_value_type_works_correctly() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        message_bus.register_scheduler::<Option<i32>>(ImmediateScheduler::instance());
        let messages = Rc::new(RefCell::new(Vec::new()));
        let log = messages.clone();
        let _subscription = message_bus.listen::<Option<i32>>().subscribe_next(move |message| log.borrow_mut().push(message));

        message_bus.send_message(Some(MESSAGE_VALUE));
        message_bus.send_message::<Option<i32>>(None);
        message_bus.send_message(Some(SECOND_MESSAGE_VALUE));

        assert_eq!(*messages.borrow(), [Some(MESSAGE_VALUE), None, Some(SECOND_MESSAGE_VALUE)]);
    });
}

#[test]
fn send_message_null_reference_type_works_correctly() {
    with_message_bus(|| {
        let message_bus = MessageBus::new();
        message_bus.register_scheduler::<Option<String>>(ImmediateScheduler::instance());
        let messages = Rc::new(RefCell::new(Vec::new()));
        let log = messages.clone();
        let _subscription = message_bus.listen::<Option<String>>().subscribe_next(move |message| log.borrow_mut().push(message));

        message_bus.send_message(Some(HELLO_MESSAGE.to_owned()));
        message_bus.send_message::<Option<String>>(None);
        message_bus.send_message(Some(WORLD_MESSAGE.to_owned()));

        assert_eq!(*messages.borrow(), [Some(HELLO_MESSAGE.to_owned()), None, Some(WORLD_MESSAGE.to_owned())]);
    });
}
