//! A managed shell owns one command endpoint per Wayland display. A Unix socket's
//! device/inode pair identifies the same display through relative and absolute names,
//! including when several compositors share a session bus.
use super::commands::Command;
use ferroui_freedesktop::ui_thread_object::UiThreadHandle;
use std::cell::RefCell;
use std::env;
use std::fs;
use std::io;
use std::os::fd::FromRawFd;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;
use zbus::blocking::{connection::Builder, proxy::Builder as ProxyBuilder, Connection, Proxy};
use zbus::fdo::{Error as BusError, RequestNameFlags, RequestNameReply};
use zbus::proxy::CacheProperties;

const PATH: &str = "/org/ferroui/Desktop";
const INTERFACE: &str = "org.ferroui.Desktop1";

fn inherited_display() -> io::Result<Option<PathBuf>> {
    let Some(value) = env::var_os("WAYLAND_SOCKET") else {
        return Ok(None);
    };
    let fd: i32 = value
        .to_str()
        .and_then(|text| text.parse().ok())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid WAYLAND_SOCKET"))?;
    // Duplicate first: the backend owns the inherited descriptor. fcntl validates
    // the environment-provided number before we turn it into an owned Rust handle.
    let copy = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 3) };
    if copy < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: F_DUPFD_CLOEXEC returned a new descriptor owned by this function.
    let stream = unsafe { UnixStream::from_raw_fd(copy) };
    let address = stream.peer_addr()?;
    address
        .as_pathname()
        .map(|p| Some(p.to_owned()))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Unsupported,
                "Managed Wayland commands require a named compositor socket",
            )
        })
}

/// wayland-client consumes WAYLAND_SOCKET as it opens the display. Preserve its
/// named peer for subsequent compositor IPC discovery and child launchers.
pub fn prepare_inherited_display() -> io::Result<()> {
    if let Some(path) = inherited_display()? {
        env::set_var("WAYLAND_DISPLAY", path);
    }
    Ok(())
}

fn display_name() -> io::Result<String> {
    let display = inherited_display()?.unwrap_or_else(|| {
        PathBuf::from(env::var_os("WAYLAND_DISPLAY").unwrap_or_else(|| "wayland-0".into()))
    });
    let path = if display.is_absolute() {
        display
    } else {
        PathBuf::from(
            env::var_os("XDG_RUNTIME_DIR")
                .ok_or_else(|| io::Error::other("Missing XDG_RUNTIME_DIR"))?,
        )
        .join(display)
    };
    let info = fs::metadata(&path)?;
    if !info.file_type().is_socket() {
        return Err(io::Error::other(format!(
            "{} is not a Wayland socket",
            path.display()
        )));
    }
    Ok(format!(
        "org.ferroui.Desktop.Wayland.d{:x}_{:x}",
        info.dev(),
        info.ino()
    ))
}

fn client() -> io::Result<Proxy<'static>> {
    let connection = Builder::session()
        .map_err(io::Error::other)?
        .method_timeout(Duration::from_secs(4))
        .build()
        .map_err(io::Error::other)?;
    let proxy = ProxyBuilder::new(&connection)
        .destination(display_name()?)
        .map_err(io::Error::other)?
        .path(PATH)
        .map_err(io::Error::other)?
        .interface(INTERFACE)
        .map_err(io::Error::other)?
        .cache_properties(CacheProperties::No)
        .build()
        .map_err(io::Error::other)?;
    Ok(proxy)
}

pub fn send(command: Command) -> io::Result<()> {
    let name = match command {
        Command::Applications => "applications",
        Command::Workspaces => "workspaces",
        Command::QuickSettings => "settings",
        Command::Notifications => "notifications",
    };
    client()?.call("Show", &(name,)).map_err(io::Error::other)
}

pub fn current_scene() -> io::Result<String> {
    client()?
        .call("CurrentScene", &())
        .map_err(io::Error::other)
}

type Handler = Rc<dyn Fn(Command)>;

#[derive(Default)]
struct State {
    scene: RefCell<Option<String>>,
    handler: RefCell<Option<Handler>>,
}

struct Interface(UiThreadHandle<State>);

#[zbus::interface(name = "org.ferroui.Desktop1", spawn = false)]
impl Interface {
    async fn show(&self, name: &str) -> zbus::fdo::Result<()> {
        let command = Command::parse(name)
            .ok_or_else(|| BusError::InvalidArgs("Unknown desktop command".into()))?;
        self.0
            .call(move |state| {
                let handler = state.handler.borrow().clone()?;
                handler(command);
                Some(())
            })
            .await
            .flatten()
            .ok_or_else(|| BusError::Failed("Shell is not ready".into()))
    }

    async fn current_scene(&self) -> zbus::fdo::Result<String> {
        self.0
            .call(|state| state.scene.borrow().clone())
            .await
            .flatten()
            .ok_or_else(|| BusError::Failed("No published shell scene".into()))
    }
}

pub(super) struct Endpoint {
    state: Rc<State>,
    handle: UiThreadHandle<State>,
    // This dedicated connection owns the name; disconnect also releases it on abnormal exit.
    _connection: Connection,
}

impl Endpoint {
    pub(super) fn claim() -> io::Result<Rc<Self>> {
        let name = display_name()?;
        let state = Rc::new(State::default());
        let handle = UiThreadHandle::register(state.clone());
        let build = || -> io::Result<Connection> {
            let bus = Builder::session()
                .map_err(io::Error::other)?
                .method_timeout(Duration::from_secs(4))
                .serve_at(PATH, Interface(handle.clone()))
                .map_err(io::Error::other)?
                .build()
                .map_err(io::Error::other)?;
            match bus.request_name_with_flags(name.as_str(), RequestNameFlags::DoNotQueue.into()) {
                Ok(RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner) => {}
                Ok(_) | Err(zbus::Error::NameTaken) => {
                    return Err(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        "Ferro Desktop already owns this Wayland display",
                    ))
                }
                Err(error) => return Err(io::Error::other(error)),
            }
            Ok(bus)
        };
        match build() {
            Ok(connection) => Ok(Rc::new(Self {
                state,
                handle,
                _connection: connection,
            })),
            Err(error) => {
                handle.unregister();
                Err(error)
            }
        }
    }

    pub(super) fn set_handler(&self, handler: Handler) {
        self.state.handler.replace(Some(handler));
    }

    pub(super) fn publish_scene(&self, scene: &str) {
        self.state.scene.replace(Some(scene.to_owned()));
    }
}

impl Drop for Endpoint {
    fn drop(&mut self) {
        self.handle.unregister();
    }
}
