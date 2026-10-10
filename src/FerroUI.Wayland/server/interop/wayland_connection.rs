//! A connection to a compositor with the event queue of the backend, and the
//! wait of the worker (the port of `WaylandConnection.cs`).

use super::unsafe_native_methods::poll_two;
use crate::server::wayland_worker::WaylandWorkerState;
use crate::ferro_wayland_exception::{
    FerroWaylandException, FerroWaylandFlushException, FerroWaylandPollException, FerroWaylandProtocolErrorException,
    FerroWaylandReadException,
};
use std::ffi::c_void;
use std::io;
use std::os::fd::{BorrowedFd, FromRawFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use wayland_client::backend::WaylandError;
use wayland_client::{Connection, DispatchError, EventQueue, QueueHandle};

/// What one iteration of the wait ended with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DispatchResult {
    Dispatched,
    Wakeup,
    ConnectionReset,
}

pub struct WaylandConnection {
    display: Connection,
    queue: EventQueue<WaylandWorkerState>,
    queue_handle: QueueHandle<WaylandWorkerState>,
    is_connected: bool,
    id: u64,
}

impl WaylandConnection {
    /// Connects to the display of a name, or of the environment when there is none
    /// (`wl_display_connect`): `WAYLAND_SOCKET`, then `WAYLAND_DISPLAY`, then `wayland-0`; a
    /// name that is not an absolute path is a socket of `XDG_RUNTIME_DIR`.
    ///
    /// The result is the display alone, which any thread may hold: the event queue of the
    /// backend is made by the worker thread ([`WaylandConnection::new`]).
    pub fn connect(path: Option<&str>) -> Result<Connection, FerroWaylandException> {
        let display = match path {
            None if std::env::var_os("WAYLAND_SOCKET").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some() => {
                Connection::connect_to_env().map_err(|error| FerroWaylandException::new(error.to_string()))?
            }
            _ => {
                let socket_path = Self::socket_path(path.unwrap_or("wayland-0"))?;
                let stream = UnixStream::connect(&socket_path).map_err(|error| {
                    FerroWaylandException::new(format!("Unable to connect to {}: {error}", socket_path.display()))
                })?;
                Connection::from_socket(stream).map_err(|error| FerroWaylandException::new(error.to_string()))?
            }
        };
        Ok(display)
    }

    fn socket_path(name: &str) -> Result<PathBuf, FerroWaylandException> {
        let name = PathBuf::from(name);
        if name.is_absolute() {
            return Ok(name);
        }
        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .ok_or_else(|| FerroWaylandException::new("XDG_RUNTIME_DIR is not set or is not an absolute path"))?;
        Ok(runtime_dir.join(name))
    }

    /// Connects over an open descriptor of the socket of the display (`wl_display_connect_to_fd`),
    /// which the connection owns from then on.
    pub fn connect_to_fd(fd: i32) -> Result<Connection, FerroWaylandException> {
        if fd < 0 {
            return Err(FerroWaylandException::new("The descriptor of the display is not valid"));
        }
        // SAFETY: the options state that the descriptor is an open socket that is handed over
        // to the backend, which is what the library call of the reference does with it.
        let stream = UnixStream::from(unsafe { OwnedFd::from_raw_fd(fd) });
        Connection::from_socket(stream).map_err(|error| FerroWaylandException::new(error.to_string()))
    }

    /// The connection of a display, with a queue of its own for the objects of the backend.
    pub fn new(display: Connection) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let queue = display.new_event_queue();
        let queue_handle = queue.handle();
        Self { display, queue, queue_handle, is_connected: true, id: NEXT_ID.fetch_add(1, Ordering::Relaxed) }
    }

    pub fn display(&self) -> &Connection {
        &self.display
    }

    /// The handle objects are created with: their events come to the queue of this connection.
    pub fn queue_handle(&self) -> &QueueHandle<WaylandWorkerState> {
        &self.queue_handle
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected
    }

    /// A number no other connection of the process has: what the reference compares the
    /// display objects for (a cookie of another connection is not valid on this one).
    pub fn id(&self) -> u64 {
        self.id
    }

    /// The `wl_display` of the C library, for EGL. Valid while the connection lives.
    pub fn display_ptr(&self) -> *mut c_void {
        self.display.backend().display_ptr().cast()
    }

    /// Marks the connection as gone. The display is closed when the value is dropped.
    pub fn dispose(&mut self) {
        self.is_connected = false;
    }

    /// Sends the pending requests and waits for the events they cause to be handled.
    pub fn roundtrip(&mut self, state: &mut WaylandWorkerState) -> Result<(), FerroWaylandException> {
        self.queue.roundtrip(state).map(|_| ()).map_err(|error| Self::dispatch_error(&error))
    }

    /// Sends the pending requests.
    pub fn flush(&self) -> Result<(), FerroWaylandException> {
        match self.display.flush() {
            Ok(()) => Ok(()),
            Err(WaylandError::Io(error)) if error.kind() == io::ErrorKind::WouldBlock => Ok(()),
            Err(error) => Err(Self::flush_error(error)),
        }
    }

    fn dispatch_error(error: &DispatchError) -> FerroWaylandException {
        match error {
            DispatchError::Backend(WaylandError::Protocol(protocol)) => {
                FerroWaylandProtocolErrorException::with_code(protocol.code, protocol.to_string())
            }
            other => FerroWaylandReadException::new(other.to_string()),
        }
    }

    fn flush_error(error: WaylandError) -> FerroWaylandException {
        match error {
            WaylandError::Io(error) => match error.raw_os_error() {
                Some(errno) => FerroWaylandFlushException::from_errno(errno),
                None => FerroWaylandFlushException::new(format!("wl_display_flush failed: {error}")),
            },
            WaylandError::Protocol(protocol) => {
                FerroWaylandProtocolErrorException::with_code(protocol.code, protocol.to_string())
            }
        }
    }

    fn is_connection_reset(error: &io::Error) -> bool {
        matches!(error.kind(), io::ErrorKind::BrokenPipe | io::ErrorKind::ConnectionReset)
    }

    /// Dispatches what the queue has; without anything queued, sends the requests and waits
    /// for events of the connection or for the wake-up descriptor.
    pub fn dispatch_queue_or_wakeup(
        &mut self,
        state: &mut WaylandWorkerState,
        wakeup_fd: BorrowedFd<'_>,
    ) -> Result<DispatchResult, FerroWaylandException> {
        // Events that were read earlier (during a round trip, or by a read that another queue
        // of the display made) are in the queue while the socket is empty: they are handed
        // out before anything waits.
        match self.queue.dispatch_pending(state) {
            Ok(0) => {}
            Ok(_) => return Ok(DispatchResult::Dispatched),
            Err(error) => return Self::classify_dispatch_error(error),
        }

        // This initiates the race of sorts for who will have to use the wayland socket
        let Some(read_guard) = self.queue.prepare_read() else {
            // Other thread has won the race and read from the display, we can safely dispatch pending events
            return match self.queue.dispatch_pending(state) {
                Ok(_) => Ok(DispatchResult::Dispatched),
                Err(error) => Self::classify_dispatch_error(error),
            };
        };

        // We won the race and are now responsible to drive libwayland's flush/poll/read cycle

        // First flush any pending data to the compositor
        match self.display.flush() {
            Ok(()) => {}
            // Not an error, just network buffers being full. We need to read our side of the socket and/or wait
            // for the compositor to process the previous requests, so report success
            Err(WaylandError::Io(error)) if error.kind() == io::ErrorKind::WouldBlock => {}
            Err(error) => {
                // Dropping the guard releases the display read lock.
                drop(read_guard);
                return match error {
                    WaylandError::Io(error) if Self::is_connection_reset(&error) => Ok(DispatchResult::ConnectionReset),
                    other => Err(Self::flush_error(other)),
                };
            }
        }

        let poll_result = match poll_two(read_guard.connection_fd(), wakeup_fd) {
            Ok(result) => result,
            Err(_) => {
                // Release the display read lock
                drop(read_guard);
                // This is quite likely a non-recoverable error
                return Err(FerroWaylandPollException::new());
            }
        };

        if !poll_result.first {
            // poll got woken up by the wakeup fd
            drop(read_guard);
            return Ok(DispatchResult::Wakeup);
        }

        // Actual read call
        match read_guard.read() {
            Ok(_) => {}
            // Another reader emptied the socket between the wait and the read.
            Err(WaylandError::Io(error)) if error.kind() == io::ErrorKind::WouldBlock => {}
            Err(WaylandError::Io(error)) if Self::is_connection_reset(&error) => {
                return Ok(DispatchResult::ConnectionReset);
            }
            Err(WaylandError::Io(error)) => {
                return Err(match error.raw_os_error() {
                    Some(errno) => FerroWaylandReadException::from_errno(errno),
                    None => FerroWaylandReadException::new(format!("wl_display_read_events failed: {error}")),
                });
            }
            Err(WaylandError::Protocol(protocol)) => {
                return Err(FerroWaylandProtocolErrorException::with_code(protocol.code, protocol.to_string()));
            }
        }

        match self.queue.dispatch_pending(state) {
            Ok(_) => Ok(DispatchResult::Dispatched),
            Err(error) => Self::classify_dispatch_error(error),
        }
    }

    fn classify_dispatch_error(error: DispatchError) -> Result<DispatchResult, FerroWaylandException> {
        match error {
            DispatchError::Backend(WaylandError::Io(error)) if Self::is_connection_reset(&error) => {
                Ok(DispatchResult::ConnectionReset)
            }
            other => Err(Self::dispatch_error(&other)),
        }
    }
}
