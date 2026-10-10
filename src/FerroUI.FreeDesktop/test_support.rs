//! What the tests of the crate share: the dispatcher of a test thread, and
//! two connections with a service on one of them.
//!
//! The services of the tests are doubles on a second connection. Two
//! set-ups:
//!
//! - By default the two connections are the two ends of a socket pair,
//!   without a bus, and the second end also answers for the bus itself
//!   (who owns a name, and the signal that the owner changed). This runs
//!   on every Unix system.
//! - With `FERROUI_FREEDESKTOP_TEST_BUS=session` both connect to the
//!   session bus of the environment (a private one: `dbus-run-session`),
//!   and the double takes and releases the well-known names for real. The
//!   names are the real ones, so these runs need `--test-threads=1`.

use crate::dbus_helper::DBusHelper;
use ferroui_base::threading::{Dispatcher, UnitTestDispatcherScope};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Runs the jobs of the dispatcher of the test thread until `done` says
/// so. Completions that come from the threads of the D-Bus library wake
/// the dispatcher by posting to it, so the loop sleeps between rounds.
///
/// # Panics
/// Panics when `done` is not true within ten seconds.
pub(crate) fn pump_until(mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        Dispatcher::ui_thread().run_jobs(None);
        if done() {
            return;
        }
        assert!(Instant::now() < deadline, "the condition was not met within ten seconds");
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// Makes the test thread its own UI thread for the duration of a test.
pub(crate) fn scope() -> UnitTestDispatcherScope {
    Dispatcher::unit_test_scope()
}

/// The unique name the double of the bus gives the services, and the name
/// of the bus itself.
pub(crate) const SERVICE_UNIQUE_NAME: &str = ":1.42";
pub(crate) const BUS_NAME: &str = "org.freedesktop.DBus";

/// A signal as it arrives through a bus: with the unique name of its
/// sender. Without a bus a connection has no name, and a signal would
/// carry none; with one, the name is the one the bus gave the connection.
/// The proxies of the D-Bus library take the signals of a service by the
/// unique name of the owner of its well-known name.
pub(crate) fn signal<B>(
    connection: &zbus::Connection,
    sender: &str,
    path: &str,
    interface: &str,
    name: &str,
    body: &B,
) -> zbus::Message
where
    B: zbus::export::serde::ser::Serialize + zbus::zvariant::DynamicType,
{
    let builder = zbus::Message::signal(path, interface, name).unwrap();
    match connection.unique_name() {
        Some(own) => builder.sender(own.as_str()).unwrap(),
        None => builder.sender(sender).unwrap(),
    }
    .build(body)
    .unwrap()
}

/// Emits a signal from inside a method of a double.
pub(crate) async fn emit_from_service<B>(
    connection: &zbus::Connection,
    path: &str,
    interface: &str,
    name: &str,
    body: &B,
) where
    B: zbus::export::serde::ser::Serialize + zbus::zvariant::DynamicType,
{
    connection.send(&signal(connection, SERVICE_UNIQUE_NAME, path, interface, name, body)).await.unwrap();
}

/// The bus, when there is none.
struct FakeBus {
    owners: Arc<Mutex<HashMap<String, String>>>,
}

#[zbus::interface(name = "org.freedesktop.DBus")]
impl FakeBus {
    fn get_name_owner(&self, name: &str) -> zbus::fdo::Result<String> {
        if name == BUS_NAME {
            return Ok(BUS_NAME.to_string());
        }
        self.owners
            .lock()
            .unwrap()
            .get(name)
            .cloned()
            .ok_or_else(|| zbus::fdo::Error::NameHasNoOwner(format!("Could not get owner of name '{name}': no such name")))
    }
}

pub(crate) fn on_session_bus() -> bool {
    std::env::var("FERROUI_FREEDESKTOP_TEST_BUS").as_deref() == Ok("session")
}

/// The builder of the connection the doubles are served on.
pub(crate) type ServiceBuilder = zbus::blocking::connection::Builder<'static>;

/// Two connections: the one the code under test uses, and the one of the
/// doubles.
pub(crate) struct TestConnections {
    pub(crate) client: zbus::Connection,
    pub(crate) service: zbus::blocking::Connection,
    /// The owners the double of the bus answers with; `None` on a real bus.
    owners: Option<Arc<Mutex<HashMap<String, String>>>>,
    /// The path of an object of the doubles, for the round trip of `settle`.
    ping_path: String,
}

impl TestConnections {
    /// `serve` adds the doubles to the connection. They are part of the
    /// connection from the moment it is built: an object server that is
    /// started later, by the first object that is added, was seen not to
    /// answer calls now and then. `ping_path` is the path of one of them.
    pub(crate) fn new(
        ping_path: &str,
        serve: impl FnOnce(ServiceBuilder) -> zbus::Result<ServiceBuilder> + Send + 'static,
    ) -> TestConnections {
        let (client, service, owners) = if on_session_bus() {
            let client = DBusHelper::try_create_new_connection(None).expect("a session bus");
            let service = serve(zbus::blocking::connection::Builder::session().unwrap()).unwrap().build().unwrap();
            (client, service, None)
        } else {
            let (client_end, service_end) = std::os::unix::net::UnixStream::pair().unwrap();
            let guid = zbus::Guid::generate();
            let owners = Arc::new(Mutex::new(HashMap::new()));
            // Both ends take part in the handshake, so one is built on
            // another thread.
            let service = {
                let owners = owners.clone();
                std::thread::spawn(move || {
                    let builder = zbus::blocking::connection::Builder::async_io_unix_stream(service_end)
                        .server(guid)
                        .unwrap()
                        .p2p()
                        .serve_at("/org/freedesktop/DBus", FakeBus { owners })
                        .unwrap();
                    serve(builder).unwrap().build().unwrap()
                })
            };
            let client = DBusHelper::with_object_server(
                zbus::blocking::connection::Builder::async_io_unix_stream(client_end).p2p(),
            )
            .unwrap()
            .build()
            .unwrap()
            .into_inner();
            let service = service.join().unwrap();
            (client, service, Some(owners))
        };

        TestConnections { client, service, owners, ping_path: ping_path.to_string() }
    }

    /// The service appears on the bus under `name`.
    pub(crate) fn start(&self, name: &str) {
        match &self.owners {
            Some(owners) => {
                owners.lock().unwrap().insert(name.to_string(), SERVICE_UNIQUE_NAME.to_string());
                self.name_owner_changed(&(name, "", SERVICE_UNIQUE_NAME));
            }
            None => self.service.request_name(name).unwrap(),
        }
    }

    /// The service is gone from the bus (it crashed).
    pub(crate) fn stop(&self, name: &str) {
        match &self.owners {
            Some(owners) => {
                owners.lock().unwrap().remove(name);
                self.name_owner_changed(&(name, SERVICE_UNIQUE_NAME, ""));
            }
            None => {
                self.service.release_name(name).unwrap();
            }
        }
    }

    /// Emits a signal of a double.
    pub(crate) fn emit<B>(&self, path: &str, interface: &str, name: &str, body: &B)
    where
        B: zbus::export::serde::ser::Serialize + zbus::zvariant::DynamicType,
    {
        self.service.send(&signal(self.service.inner(), SERVICE_UNIQUE_NAME, path, interface, name, body)).unwrap();
    }

    /// The signal of the double of the bus that the owner of a name changed.
    fn name_owner_changed(&self, body: &(&str, &str, &str)) {
        let message =
            signal(self.service.inner(), BUS_NAME, "/org/freedesktop/DBus", BUS_NAME, "NameOwnerChanged", body);
        self.service.send(&message).unwrap();
    }

    /// Lets what is in flight arrive: a round trip to the service and
    /// back, behind everything that was sent before it in either
    /// direction, and the jobs that follow from what arrived.
    pub(crate) fn settle(&self) {
        for _ in 0..2 {
            let done = Rc::new(RefCell::new(false));
            let (client, flag) = (self.client.clone(), done.clone());
            let service_name = self.service.unique_name().map(|name| name.to_string());
            let ping_path = self.ping_path.clone();
            drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                let peer = zbus::fdo::PeerProxy::builder(&client)
                    .destination(service_name.unwrap_or_else(|| BUS_NAME.to_string()))
                    .unwrap()
                    .path(ping_path)
                    .unwrap()
                    .build()
                    .await
                    .unwrap();
                peer.ping().await.unwrap();
                *flag.borrow_mut() = true;
            }));
            pump_until(|| *done.borrow());
            Dispatcher::ui_thread().run_jobs(None);
        }
    }
}

/// Calls a method of an object the code under test exports on the client
/// connection, as a peer would: from the connection of the doubles, on
/// another thread, while this thread runs the dispatcher (the exported
/// objects answer from the UI thread).
pub(crate) fn peer_call<B>(
    connections: &TestConnections,
    path: &str,
    interface: &str,
    method: &str,
    body: B,
) -> zbus::Result<zbus::Message>
where
    B: zbus::export::serde::ser::Serialize + zbus::zvariant::DynamicType + Send + 'static,
{
    let service = connections.service.clone();
    let destination = connections.client.unique_name().map(|name| name.to_string());
    let (path, interface, method) = (path.to_string(), interface.to_string(), method.to_string());
    let worker = std::thread::spawn(move || {
        service.call_method(destination.as_deref(), path.as_str(), Some(interface.as_str()), method.as_str(), &body)
    });
    pump_until(|| worker.is_finished());
    worker.join().unwrap()
}

/// A property of such an object, shown as text (see [`show`]); `None`
/// when the object does not answer.
pub(crate) fn peer_property(connections: &TestConnections, path: &str, interface: &str, name: &str) -> Option<String> {
    let body = (interface.to_string(), name.to_string());
    let reply = peer_call(connections, path, "org.freedesktop.DBus.Properties", "Get", body).ok()?;
    let value: zbus::zvariant::OwnedValue = reply.body().deserialize().ok()?;
    Some(show(&value))
}

/// A value as text: variants unwrapped, the entries of a dictionary
/// sorted by key.
pub(crate) fn show(value: &zbus::zvariant::Value<'_>) -> String {
    use zbus::zvariant::Value;
    match value {
        Value::Value(inner) => show(inner),
        Value::Str(text) => text.to_string(),
        Value::ObjectPath(path) => path.to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::U8(number) => number.to_string(),
        Value::I32(number) => number.to_string(),
        Value::U32(number) => number.to_string(),
        Value::Array(items) => format!("[{}]", items.iter().map(show).collect::<Vec<_>>().join(",")),
        Value::Structure(fields) => format!("({})", fields.fields().iter().map(show).collect::<Vec<_>>().join(",")),
        Value::Dict(entries) => {
            let mut entries: Vec<String> =
                entries.iter().map(|(key, value)| format!("{}={}", show(key), show(value))).collect();
            entries.sort();
            format!("{{{}}}", entries.join(","))
        }
        other => format!("{other:?}"),
    }
}

/// The calls the doubles got, in order.
pub(crate) type Log = Arc<Mutex<Vec<String>>>;

pub(crate) fn log(log: &Log, entry: impl Into<String>) {
    log.lock().unwrap().push(entry.into());
}

/// Runs the dispatcher until the doubles were called with `entries`, in
/// that order and nothing else, and forgets those calls.
pub(crate) fn expect_calls(connections: &TestConnections, log: &Log, entries: &[&str]) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        Dispatcher::ui_thread().run_jobs(None);
        if log.lock().unwrap().len() >= entries.len() {
            break;
        }
        assert!(Instant::now() < deadline, "expected the calls {entries:?}, got {:?}", log.lock().unwrap());
        std::thread::sleep(Duration::from_millis(1));
    }
    connections.settle();
    assert_eq!(std::mem::take(&mut *log.lock().unwrap()), entries);
}
