//! The tests of the protocol over a loopback socket.
//!
//! The first three are the tests of the upstream designer support test
//! project (`RemoteProtocolTests`), which is where the original tests this
//! library. The upstream test fills the properties of every message class
//! by reflection with random values; here the objects are written out, one
//! of every class, with no property at its default. The tests after them
//! are tests of the port.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::Duration;

use ferroui_remote_protocol::designer::{
    ExceptionDetails, StartDesignerSessionMessage, UpdateXamlMessage, UpdateXamlResultMessage,
};
use ferroui_remote_protocol::input::{
    InputEventMessageBase, InputModifiers, Key, KeyEventMessage, MouseButton, PhysicalKey, PointerEventMessageBase,
    PointerMovedEventMessage, PointerPressedEventMessage, PointerReleasedEventMessage, ScrollEventMessage,
    TextInputEventMessage,
};
use ferroui_remote_protocol::viewport::{
    ClientRenderInfoMessage, ClientSupportedPixelFormatsMessage, ClientViewportAllocatedMessage, FrameMessage,
    FrameReceivedMessage, MeasureViewportMessage, PixelFormat, RequestViewportResizeMessage,
};
use ferroui_remote_protocol::{
    bson_class, exception_handler, ferro_remote_message_guid, message_handler, Assembly, BsonTcpTransport,
    DefaultMessageTypeResolver, DisposableServer, Error, ExportedType, HtmlTransportStartedMessage,
    IFerroRemoteTransportConnection, IMessageTypeResolver, Message, TcpTransportBase, ASSEMBLY,
};

const TIMEOUT_IN_MS: Duration = Duration::from_millis(1000);

const LOOPBACK: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

/// What `Init` of the upstream test sets up: a listening transport, a client
/// connected to it and the messages each end receives. Dropping it disposes
/// both connections and the server.
struct Fixture {
    listener: DisposableServer,
    server: Arc<dyn IFerroRemoteTransportConnection>,
    client: Arc<dyn IFerroRemoteTransportConnection>,
    server_messages: Receiver<Message>,
    client_messages: Receiver<Message>,
}

impl Fixture {
    fn init(
        client_resolver: Option<Arc<dyn IMessageTypeResolver>>,
        server_resolver: Option<Arc<dyn IMessageTypeResolver>>,
    ) -> Fixture {
        let default_resolver = || -> Arc<dyn IMessageTypeResolver> { Arc::new(DefaultMessageTypeResolver::new(&[])) };
        let client_transport = BsonTcpTransport::new(client_resolver.unwrap_or_else(default_resolver));
        let server_transport = BsonTcpTransport::new(server_resolver.unwrap_or_else(default_resolver));

        // The upstream test finds a free port with a listener of its own;
        // here the transport listens on the port the system assigns.
        let (connected_sender, connected) = channel();
        let listener = server_transport
            .listen(LOOPBACK, 0, move |connected| {
                let _ = connected_sender.send(connected);
            })
            .unwrap();
        let client = client_transport.connect(LOOPBACK, listener.local_addr().port()).unwrap();
        let (client_sender, client_messages) = channel();
        client.on_message(message_handler(move |_, m| {
            let _ = client_sender.send(m.clone());
        }));
        let server = connected.recv_timeout(Duration::from_secs(5)).unwrap();
        let (server_sender, server_messages) = channel();
        server.on_message(message_handler(move |_, m| {
            let _ = server_sender.send(m.clone());
        }));
        Fixture { listener, server, client, server_messages, client_messages }
    }

    fn take_server(&self) -> Message {
        self.server_messages.recv_timeout(TIMEOUT_IN_MS).unwrap()
    }

    fn take_client(&self) -> Message {
        self.client_messages.recv_timeout(TIMEOUT_IN_MS).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.client.dispose();
        self.server.dispose();
        self.listener.dispose();
    }
}

fn modifiers() -> InputEventMessageBase {
    InputEventMessageBase { modifiers: Some(vec![InputModifiers::MiddleMouseButton]) }
}

fn pointer() -> PointerEventMessageBase {
    PointerEventMessageBase { base: modifiers(), x: 0.25, y: 0.75 }
}

/// One object of every message class of the library. For an enumeration the
/// upstream test takes the last member, as these do.
fn entities() -> Vec<Message> {
    vec![
        Arc::new(MeasureViewportMessage { width: 0.125, height: 0.5 }),
        Arc::new(ClientViewportAllocatedMessage { width: 0.1, height: 0.2, dpi_x: 0.3, dpi_y: 0.4 }),
        Arc::new(RequestViewportResizeMessage { width: 0.6, height: 0.7 }),
        Arc::new(ClientSupportedPixelFormatsMessage { formats: Some(vec![PixelFormat::MaxValue]) }),
        Arc::new(ClientRenderInfoMessage { dpi_x: 0.8, dpi_y: 0.9 }),
        Arc::new(FrameReceivedMessage { sequence_id: 1_234_567 }),
        Arc::new(FrameMessage {
            sequence_id: 7_654_321,
            format: PixelFormat::MaxValue,
            data: Some(vec![200]),
            width: 11,
            height: 12,
            stride: 13,
            dpi_x: 0.11,
            dpi_y: 0.12,
        }),
        Arc::new(PointerMovedEventMessage { base: pointer() }),
        Arc::new(PointerPressedEventMessage { base: pointer(), button: MouseButton::Middle }),
        Arc::new(PointerReleasedEventMessage { base: pointer(), button: MouseButton::Middle }),
        Arc::new(ScrollEventMessage { base: pointer(), delta_x: 0.13, delta_y: 0.14 }),
        Arc::new(KeyEventMessage {
            base: modifiers(),
            is_down: true,
            key: Key::DeadCharProcessed,
            physical_key: PhysicalKey::Tab,
            key_symbol: Some("5c6e4d5f-0b0e-4c7c-9a57-3a4a2a0f3a11".to_string()),
        }),
        Arc::new(TextInputEventMessage { base: modifiers(), text: "f2f0a3c9-41f1-4b4c-8d59-6a6d0c6d5b22".to_string() }),
        Arc::new(UpdateXamlMessage {
            xaml: Some("0f8fad5b-d9cb-469f-a165-70867728950e".to_string()),
            assembly_path: Some("7c9e6679-7425-40de-944b-e07fc1f90ae7".to_string()),
            xaml_file_project_path: Some("3f2504e0-4f89-11d3-9a0c-0305e82c3301".to_string()),
        }),
        Arc::new(UpdateXamlResultMessage {
            error: Some("21ec2020-3aea-4069-a2dd-08002b30309d".to_string()),
            handle: Some("9a1b2c3d-0000-4000-8000-123456789abc".to_string()),
            exception: Some(ExceptionDetails {
                exception_type: Some("Exception".to_string()),
                line_number: Some(5),
                line_position: Some(6),
                message: Some("Here".to_string()),
            }),
        }),
        Arc::new(StartDesignerSessionMessage { session_id: Some("a8098c1a-f86e-11da-bd1a-00112444be1e".to_string()) }),
        Arc::new(HtmlTransportStartedMessage { uri: Some("e902893a-9d22-3c7e-a7b8-d6e313b71d9f".to_string()) }),
    ]
}

#[test]
fn entities_are_properly_serialized_and_deserialized() {
    let fixture = Fixture::init(None, None);
    fixture.server.on_message(message_handler(|_, _| {}));

    let entities = entities();
    // Every class of the table of the library is among them.
    assert_eq!(entities.len(), ASSEMBLY.exported_types.len());
    for exported in ASSEMBLY.exported_types {
        assert!(entities.iter().any(|o| o.get_type().name == exported.class.name), "{}", exported.class.name);
    }

    for o in entities {
        assert!(matches!(fixture.client.send(o.clone()).wait_timeout(TIMEOUT_IN_MS), Some(Ok(()))));
        let received = fixture.take_server();
        assert!(received.equals(&*o), "{:?} was received as {:?}", o, received);
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SubObject {
    pub foo: i32,
}

bson_class!(SubObject as "SubObject" { "Foo" => foo: i32 });

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExtendedMeasureViewportMessage {
    pub width: f64,

    pub some_new_property: i32,
    pub some_array_property: Option<Vec<i32>>,
    pub sub_object_property: Option<SubObject>,
    pub height: f64,
}

ferro_remote_message_guid!(ExtendedMeasureViewportMessage, "6E3C5310-E2B1-4C3D-8688-01183AA48C5B");
bson_class!(ExtendedMeasureViewportMessage as "ExtendedMeasureViewportMessage" {
    "Width" => width: f64,
    "SomeNewProperty" => some_new_property: i32,
    "SomeArrayProperty" => some_array_property: Option<Vec<i32>>,
    "SubObjectProperty" => sub_object_property: Option<SubObject>,
    "Height" => height: f64,
});

/// The assembly of the test project, as far as the resolver reads it.
static TEST_ASSEMBLY: Assembly =
    Assembly { name: "RemoteProtocolTests", exported_types: &[ExportedType::of::<ExtendedMeasureViewportMessage>()] };

#[test]
fn remote_protocol_should_be_backwards_compatible() {
    let fixture = Fixture::init(Some(Arc::new(DefaultMessageTypeResolver::new(&[&TEST_ASSEMBLY]))), None);
    fixture.client.send(Arc::new(ExtendedMeasureViewportMessage {
        width: 100.0,
        height: 200.0,
        some_new_property: 300,
        some_array_property: Some(vec![1, 2, 3]),
        sub_object_property: Some(SubObject { foo: 543 }),
    }));
    let received = fixture.take_server();
    let received = received.downcast_ref::<MeasureViewportMessage>().unwrap();
    assert_eq!(100.0, received.width);
    assert_eq!(200.0, received.height);
}

#[test]
fn bson_serialization_is_thread_safe() {
    let fixture = Fixture::init(None, None);
    // This test verifies that concurrent serialization doesn't cause infinite loops
    // or corruption in the TypeHelper cache
    let messages: Vec<Message> = (0..100)
        .map(|i| Arc::new(MeasureViewportMessage { width: i as f64, height: (i * 2) as f64 }) as Message)
        .collect();

    // Spawn multiple threads that all try to serialize messages concurrently
    let mut tasks = Vec::new();
    for _ in 0..10 {
        let client = fixture.client.clone();
        let messages = messages.clone();
        tasks.push(std::thread::spawn(move || {
            let mut exceptions = Vec::new();
            for message in messages {
                match client.send(message).wait_timeout(TIMEOUT_IN_MS) {
                    Some(Ok(())) => {}
                    Some(Err(e)) => exceptions.push(e.to_string()),
                    None => exceptions.push("the send timed out".to_string()),
                }
            }
            exceptions
        }));
    }

    // Verify no exceptions occurred
    for task in tasks {
        assert_eq!(task.join().unwrap(), Vec::<String>::new());
    }
}

// ----------------------------------------------------------- tests of the port

fn width_of(message: &Message) -> f64 {
    message.downcast_ref::<MeasureViewportMessage>().unwrap().width
}

fn measure(width: f64) -> Message {
    Arc::new(MeasureViewportMessage { width, height: 1.0 })
}

#[test]
fn messages_sent_both_ways_arrive_intact_and_in_order() {
    let fixture = Fixture::init(None, None);
    for i in 0..200 {
        fixture.client.send(measure(i as f64));
        fixture.server.send(measure(-(i as f64)));
    }
    for i in 0..200 {
        assert_eq!(width_of(&fixture.take_server()), i as f64);
        assert_eq!(width_of(&fixture.take_client()), -(i as f64));
    }
}

#[test]
fn messages_sent_before_a_handler_is_added_are_kept_for_it() {
    let server_transport = BsonTcpTransport::empty();
    let (connected_sender, connected) = channel();
    let listener = server_transport
        .listen(LOOPBACK, 0, move |connected| {
            let _ = connected_sender.send(connected);
        })
        .unwrap();
    let client = BsonTcpTransport::default().connect(LOOPBACK, listener.local_addr().port()).unwrap();
    let server = connected.recv_timeout(Duration::from_secs(5)).unwrap();
    for i in 0..10 {
        assert!(matches!(client.send(measure(i as f64)).wait_timeout(TIMEOUT_IN_MS), Some(Ok(()))));
    }
    // The messages are on their way or stashed; a handler added now gets all
    // of them, the stashed ones first.
    std::thread::sleep(Duration::from_millis(200));
    let (sender, received) = channel();
    server.on_message(message_handler(move |_, m| {
        let _ = sender.send(m.clone());
    }));
    for i in 0..10 {
        assert_eq!(width_of(&received.recv_timeout(TIMEOUT_IN_MS).unwrap()), i as f64);
    }
    client.dispose();
    server.dispose();
    listener.dispose();
}

#[test]
fn many_small_messages_arrive_in_order() {
    let fixture = Fixture::init(None, None);
    let mut last = None;
    for i in 0..5000 {
        last = Some(fixture.client.send(measure(i as f64)));
    }
    assert!(matches!(last.unwrap().wait_timeout(Duration::from_secs(30)), Some(Ok(()))));
    for i in 0..5000 {
        assert_eq!(width_of(&fixture.server_messages.recv_timeout(Duration::from_secs(30)).unwrap()), i as f64);
    }
}

#[test]
fn a_large_message_arrives_intact() {
    let fixture = Fixture::init(None, None);
    let data: Vec<u8> = (0..8 * 1024 * 1024).map(|i| (i % 253) as u8).collect();
    let frame = FrameMessage {
        sequence_id: 1,
        format: PixelFormat::Rgba8888,
        data: Some(data),
        width: 2048,
        height: 1024,
        stride: 8192,
        dpi_x: 96.0,
        dpi_y: 96.0,
    };
    fixture.server.send(Arc::new(frame.clone()));
    let received = fixture.client_messages.recv_timeout(Duration::from_secs(30)).unwrap();
    assert_eq!(received.downcast_ref::<FrameMessage>(), Some(&frame));
    // The connection is in step after it.
    fixture.server.send(measure(5.0));
    assert_eq!(width_of(&fixture.take_client()), 5.0);
}

#[test]
fn a_closed_connection_raises_the_exception_event_once() {
    let fixture = Fixture::init(None, None);
    let (server_sender, server_errors) = channel();
    fixture.server.on_exception(exception_handler(move |_, e| {
        let _ = server_sender.send(e.clone());
    }));
    let (client_sender, client_errors) = channel();
    fixture.client.on_exception(exception_handler(move |_, e| {
        let _ = client_sender.send(e.clone());
    }));

    fixture.client.dispose();
    assert!(matches!(server_errors.recv_timeout(Duration::from_secs(5)), Ok(Error::EndOfStream)));
    assert!(server_errors.recv_timeout(Duration::from_millis(300)).is_err());
    // The end that closed the connection is told nothing.
    assert!(client_errors.recv_timeout(Duration::from_millis(300)).is_err());
}

#[test]
fn a_message_of_a_class_the_resolver_does_not_know_fails_its_send() {
    let fixture = Fixture::init(None, None);
    match fixture.client.send(Arc::new(SubObject { foo: 1 })).wait_timeout(TIMEOUT_IN_MS) {
        Some(Err(Error::KeyNotFound(_))) => {}
        other => panic!("expected a key that is not found, got {:?}", other),
    }
    fixture.client.send(measure(2.0));
    assert_eq!(width_of(&fixture.take_server()), 2.0);
}

#[test]
fn a_server_that_is_disposed_accepts_no_more_clients() {
    let transport = BsonTcpTransport::empty();
    let (connected_sender, connected) = channel();
    let listener = transport
        .listen(LOOPBACK, 0, move |connected| {
            let _ = connected_sender.send(connected);
        })
        .unwrap();
    let port = listener.local_addr().port();
    let client = transport.connect(LOOPBACK, port).unwrap();
    let server = connected.recv_timeout(Duration::from_secs(5)).unwrap();
    listener.dispose();
    listener.dispose();
    // The listener thread closes the socket when it has seen the flag.
    let mut refused = false;
    for _ in 0..50 {
        match transport.connect(LOOPBACK, port) {
            Err(Error::Io(_)) => {
                refused = true;
                break;
            }
            Err(other) => panic!("unexpected error {:?}", other),
            Ok(late) => {
                late.dispose();
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
    assert!(refused, "the port still accepts connections");
    assert!(connected.recv_timeout(Duration::from_millis(100)).is_err());
    // The connection that was accepted before is not affected.
    let (sender, received) = channel();
    server.on_message(message_handler(move |_, m| {
        let _ = sender.send(m.clone());
    }));
    client.send(measure(9.0));
    assert_eq!(width_of(&received.recv_timeout(TIMEOUT_IN_MS).unwrap()), 9.0);
    client.dispose();
    server.dispose();
}

/// Not from upstream: the two key enumerations are compiled into this library from copies of the
/// files of the base library (`input/key.rs`, `input/physical_key.rs`; see `lib.rs`), and the copies
/// are the originals, byte for byte. A change of a key is made in the base library and copied here.
#[test]
fn the_key_enumerations_are_the_files_of_the_base_library() {
    let library = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for file in ["key.rs", "physical_key.rs"] {
        let copy = library.join("input").join(file);
        let original = library.with_file_name("FerroUI.Base").join("input").join(file);
        let read = |path: &std::path::Path| {
            std::fs::read_to_string(path).unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
        };
        assert!(
            read(&copy) == read(&original),
            "{} differs from {}: copy the file of the base library over it",
            copy.display(),
            original.display()
        );
    }
}
