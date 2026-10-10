//! The connection to the session bus (the port of `DBusHelper.cs`).

use ferroui_base::logging::{LogEventLevel, Logger};
use std::sync::{Mutex, PoisonError};
use zbus::Connection;

/// The default connection of the process, and whether making it failed.
static DEFAULT_CONNECTION: Mutex<(Option<Connection>, bool)> = Mutex::new((None, false));

pub struct DBusHelper;

impl DBusHelper {
    /// The connection to the session bus the services of the process
    /// share (`DefaultConnection`): made when first asked for; `None` for
    /// good once that failed.
    pub fn default_connection() -> Option<Connection> {
        let mut state = DEFAULT_CONNECTION.lock().unwrap_or_else(PoisonError::into_inner);
        if state.0.is_none() && !state.1 {
            state.0 = Self::try_create_new_connection(None);
            if state.0.is_none() {
                state.1 = true;
            }
        }

        state.0.clone()
    }

    /// Connects to the bus at `dbus_address`, or to the session bus
    /// (`TryCreateNewConnection`). A failure is logged and gives `None`.
    pub fn try_create_new_connection(dbus_address: Option<&str>) -> Option<Connection> {
        // Connect synchronously
        //
        // The reference clears the synchronization context around this so
        // that the wait does not need the UI thread; here the connection is
        // established by the reactor of the D-Bus library, on its own
        // thread, and this thread only waits for it.
        let result = match dbus_address {
            Some(address) => zbus::blocking::connection::Builder::address(address),
            None => zbus::blocking::connection::Builder::session(),
        }
        .and_then(Self::with_object_server)
        .and_then(|builder| builder.build())
        .map(zbus::blocking::Connection::into_inner);

        match result {
            Ok(conn) => Some(conn),
            Err(e) => {
                if let Some(logger) = Logger::try_get(LogEventLevel::Error, "DBUS") {
                    logger.log(None, &format!("Unable to connect to DBus: {e}"));
                }
                None
            }
        }
    }
}

impl DBusHelper {
    /// Makes a connection answer method calls from the moment it is built.
    ///
    /// The services of the crate (a menu, a tray icon) are exported on a
    /// connection after it was made, as the reference adds method
    /// handlers. The object server of the D-Bus library starts with the
    /// first object: when that is given to the builder, the server is
    /// listening before the connection reads its first message; when it is
    /// added later, the server starts in a task of its own, and a call
    /// that arrives before that task ran is never answered (measured in
    /// the tests of the input methods: docs/porting/x11-platform.md,
    /// section 13). So every connection of the crate is built with one
    /// object, the standard object manager at the root.
    pub fn with_object_server(
        builder: zbus::blocking::connection::Builder<'_>,
    ) -> zbus::Result<zbus::blocking::connection::Builder<'_>> {
        builder.serve_at("/", zbus::fdo::ObjectManager)
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this class.
    use super::*;

    #[test]
    fn a_bus_that_is_not_there_gives_no_connection() {
        let address = "unix:path=/nonexistent/ferroui-freedesktop-test-bus";
        assert!(DBusHelper::try_create_new_connection(Some(address)).is_none());
        // An address that cannot be parsed is a failure like any other.
        assert!(DBusHelper::try_create_new_connection(Some("not an address")).is_none());
    }
}
