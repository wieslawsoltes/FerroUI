//! The part of the connection class of the reference's D-Bus library
//! that the accessibility server uses (`DBusConnection.cs`,
//! `DBusConnection.Worker.cs`): objects registered at paths, the
//! dispatch of a method call to the handler of its path and interface,
//! the answers for what is not registered, and messages sent in order.
//!
//! The connection itself is one of `zbus`, built without an object
//! server (`docs/porting/atspi.md`, "Threading"): nothing answers a
//! method call but the loop of this file, which is a task of the UI
//! dispatcher.

use super::built_in_introspection_handler::{self, IntrospectionData};
use super::built_in_properties_handler;
use super::interface::{
    CallResult, DBusError, DBusInterface, ERROR_UNKNOWN_INTERFACE, ERROR_UNKNOWN_OBJECT,
};
use crate::signal_watch::watch_stream;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::Dispatcher;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::{Rc, Weak};
use zbus::export::serde::ser::Serialize;
use zbus::zvariant::DynamicType;

pub(crate) const INTERFACE_PROPERTIES: &str = "org.freedesktop.DBus.Properties";
pub(crate) const INTERFACE_INTROSPECTABLE: &str = "org.freedesktop.DBus.Introspectable";

/// The handlers of a connection by path and interface (`_handlers`).
#[derive(Default)]
pub(crate) struct ObjectTable {
    handlers: RefCell<BTreeMap<(String, String), (u64, Rc<dyn DBusInterface>)>>,
    next_registration: Cell<u64>,
}

impl ObjectTable {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Registers `targets` at `path` (`RegisterObjects`). The built-in
    /// handlers of properties and introspection answer for a path as
    /// long as it has a target. Disposing the result removes what this
    /// call registered and was not replaced since.
    pub(crate) fn register_objects(
        self: &Rc<Self>,
        path: &str,
        targets: Vec<Rc<dyn DBusInterface>>,
    ) -> Rc<dyn IDisposable> {
        let registration = self.next_registration.get() + 1;
        self.next_registration.set(registration);
        {
            let mut handlers = self.handlers.borrow_mut();
            for target in targets {
                let key = (path.to_string(), target.description().name.to_string());
                handlers.insert(key, (registration, target));
            }
        }

        let weak = Rc::downgrade(self);
        Disposable::create(move || {
            if let Some(table) = weak.upgrade() {
                table.handlers.borrow_mut().retain(|_, (owner, _)| *owner != registration);
            }
        })
    }

    pub(crate) fn has_path(&self, path: &str) -> bool {
        self.handlers.borrow().keys().any(|(registered, _)| registered == path)
    }

    fn handler(&self, path: &str, interface: &str) -> Option<Rc<dyn DBusInterface>> {
        self.handlers.borrow().get(&(path.to_string(), interface.to_string())).map(|(_, handler)| handler.clone())
    }

    /// The handlers of a path, in the order of their interface names.
    pub(crate) fn interfaces_of(&self, path: &str) -> Vec<Rc<dyn DBusInterface>> {
        self.handlers
            .borrow()
            .iter()
            .filter(|((registered, _), _)| registered == path)
            .map(|(_, (_, handler))| handler.clone())
            .collect()
    }

    /// What introspection of `path` shows (`ResolveIntrospectionData`):
    /// its interfaces, and the next path element of every registered
    /// path below it.
    pub(crate) fn resolve_introspection_data(&self, path: &str) -> IntrospectionData {
        let prefix = if path == "/" { "/".to_string() } else { format!("{path}/") };
        let child_segments: BTreeSet<String> = self
            .handlers
            .borrow()
            .keys()
            .filter_map(|(registered, _)| registered.strip_prefix(prefix.as_str()))
            .filter_map(|rest| rest.split('/').next())
            .filter(|segment| !segment.is_empty())
            .map(str::to_string)
            .collect();

        IntrospectionData { interfaces: self.interfaces_of(path), child_segments }
    }

    /// Answers a method call (`DispatchMethodCallAsync`). `None` for a
    /// call the reference does not answer: one without a path or an
    /// interface.
    pub(crate) fn dispatch_method_call(&self, message: &zbus::Message) -> Option<CallResult> {
        let header = message.header();
        let path = header.path()?.as_str().to_string();
        let interface = header.interface()?.as_str().to_string();
        let member = header.member().map(|member| member.as_str().to_string()).unwrap_or_default();
        let body = message.body();
        let has_path = self.has_path(&path);

        if has_path && interface == INTERFACE_PROPERTIES {
            return Some(built_in_properties_handler::handle(self, &path, &member, &body));
        }

        if has_path && interface == INTERFACE_INTROSPECTABLE {
            let data = self.resolve_introspection_data(&path);
            return Some(built_in_introspection_handler::handle(&path, &member, &data));
        }

        if let Some(handler) = self.handler(&path, &interface) {
            return Some(handler.call(&member, &body));
        }

        if interface == INTERFACE_INTROSPECTABLE && member == "Introspect" {
            // A path that only has registered paths below it
            // (`HandleVirtualIntrospectionAsync`).
            let data = self.resolve_introspection_data(&path);
            if !data.child_segments.is_empty() {
                return Some(built_in_introspection_handler::handle(&path, &member, &data));
            }
        }

        Some(Err(Self::missing_handler(&path, &interface, has_path)))
    }

    /// `ReplyMissingHandler`.
    fn missing_handler(path: &str, interface: &str, has_path: bool) -> DBusError {
        if has_path {
            DBusError::new(
                ERROR_UNKNOWN_INTERFACE,
                format!("No handler registered for interface '{interface}' on '{path}'."),
            )
        } else {
            DBusError::new(ERROR_UNKNOWN_OBJECT, format!("No handler registered for object '{path}'."))
        }
    }
}

/// A connection to the accessibility bus with its objects.
pub(crate) struct AtSpiConnection {
    connection: zbus::Connection,
    objects: Rc<ObjectTable>,
    outbox: RefCell<VecDeque<zbus::Message>>,
    sending: Cell<bool>,
    message_loop: RefCell<Option<Rc<dyn IDisposable>>>,
    weak_self: Weak<AtSpiConnection>,
}

impl AtSpiConnection {
    /// Takes a connection that has no object server and starts answering
    /// its method calls on the UI thread.
    ///
    /// # Panics
    /// Panics when called from a thread other than the UI thread.
    pub(crate) fn new(connection: zbus::Connection) -> Rc<Self> {
        let this = Rc::new_cyclic(|weak_self| Self {
            connection,
            objects: ObjectTable::new(),
            outbox: RefCell::new(VecDeque::new()),
            sending: Cell::new(false),
            message_loop: RefCell::new(None),
            weak_self: weak_self.clone(),
        });

        // The stream exists before this function returns, so no message
        // that arrives from now on is missed.
        let stream = zbus::MessageStream::from(&this.connection);
        let weak = Rc::downgrade(&this);
        let subscription = watch_stream(stream, move |item: zbus::Result<zbus::Message>| {
            let (Some(this), Ok(message)) = (weak.upgrade(), item) else { return };
            if message.message_type() == zbus::message::Type::MethodCall {
                this.on_method_call(&message);
            }
        });
        *this.message_loop.borrow_mut() = Some(subscription);
        this
    }

    pub(crate) fn connection(&self) -> &zbus::Connection {
        &self.connection
    }

    /// The name the bus gave the connection (`GetUniqueNameAsync`);
    /// empty without a bus.
    pub(crate) fn unique_name(&self) -> String {
        self.connection.unique_name().map(|name| name.to_string()).unwrap_or_default()
    }

    pub(crate) fn register_objects(&self, path: &str, targets: Vec<Rc<dyn DBusInterface>>) -> Rc<dyn IDisposable> {
        self.objects.register_objects(path, targets)
    }

    fn on_method_call(&self, message: &zbus::Message) {
        let Some(result) = self.objects.dispatch_method_call(message) else { return };
        let header = message.header();
        if header.primary().flags().contains(zbus::message::Flags::NoReplyExpected) {
            return;
        }

        let built = match result {
            Ok(body) => zbus::Message::method_return(&header).and_then(|builder| body.build(self.with_sender(builder))),
            Err(error) => zbus::Message::error(&header, error.name.as_str())
                .and_then(|builder| self.with_sender(builder).build(&(error.message.as_str(),))),
        };
        match built {
            Ok(reply) => self.send(reply),
            Err(e) => Self::log(&format!("AT-SPI reply could not be built: {e}")),
        }
    }

    fn with_sender<'a>(&self, builder: zbus::message::Builder<'a>) -> zbus::message::Builder<'a> {
        match self.connection.unique_name() {
            Some(name) => match builder.clone().sender(name.as_str().to_string()) {
                Ok(with_sender) => with_sender,
                Err(_) => builder,
            },
            None => builder,
        }
    }

    /// Sends a signal of an object of this connection
    /// (`DBusMessage.CreateSignal` and `SendMessageAsync`).
    pub(crate) fn emit_signal<B>(&self, path: &str, interface: &str, member: &str, body: &B)
    where
        B: Serialize + DynamicType,
    {
        let built =
            zbus::Message::signal(path, interface, member).and_then(|builder| self.with_sender(builder).build(body));
        match built {
            Ok(message) => self.send(message),
            Err(e) => Self::log(&format!("AT-SPI signal {member} could not be built: {e}")),
        }
    }

    /// Queues a message; one task of the UI dispatcher sends the queue in order.
    fn send(&self, message: zbus::Message) {
        self.outbox.borrow_mut().push_back(message);
        if self.sending.replace(true) {
            return;
        }

        let Some(this) = self.weak_self.upgrade() else { return };
        drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            loop {
                let next = this.outbox.borrow_mut().pop_front();
                let Some(message) = next else { break };
                if let Err(e) = this.connection.send(&message).await {
                    Self::log(&format!("AT-SPI message could not be sent: {e}"));
                }
            }
            this.sending.set(false);
        }));
    }

    /// Stops answering (`DisposeAsync`). Messages already queued are still sent.
    pub(crate) fn dispose(&self) {
        if let Some(subscription) = self.message_loop.borrow_mut().take() {
            subscription.dispose();
        }
    }

    fn log(message: &str) {
        if let Some(logger) = Logger::try_get(LogEventLevel::Debug, LogArea::FREE_DESKTOP_PLATFORM) {
            logger.log(None, message);
        }
    }
}
