//! A private native Wayland display. Smithay handles the wire protocol and seat;
//! FerroUI receives immutable, bottom-to-top surface trees for composition.
mod buffers;
mod seat;
mod state;

use smithay::reexports::calloop::{
    channel::{channel, Event, Sender},
    generic::Generic,
    EventLoop, Interest, Mode, PostAction,
};
use smithay::reexports::wayland_server::Display;
use smithay::wayland::socket::ListeningSocketSource;
use state::{ClientData, State};
use std::{
    ffi::{OsStr, OsString},
    io,
    os::fd::AsFd,
    sync::{mpsc, Arc, Mutex},
    thread,
};

#[derive(Clone, Debug)]
pub struct Pixels {
    pub width: i32,
    pub height: i32,
    /// Tightly packed premultiplied BGRA.
    pub bytes: Vec<u8>,
    pub revision: u64,
}

#[derive(Clone, Debug)]
pub struct Surface {
    pub id: u64,
    pub window: u64,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub pixels: Arc<Pixels>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClientWindow {
    pub id: u64,
    pub title: String,
    pub app_id: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub focused: bool,
    pub minimized: bool,
    pub maximized: bool,
    pub fullscreen: bool,
    pub above: bool,
    pub workspace: usize,
}

#[derive(Clone, Debug)]
pub struct Scene {
    pub serial: u64,
    pub surfaces: Vec<Surface>,
    pub windows: Vec<ClientWindow>,
    pub workspace_count: usize,
    pub current_workspace: usize,
}

const INITIAL_WORKSPACES: usize = 4;
impl Default for Scene {
    fn default() -> Self {
        Self {
            serial: 0,
            surfaces: Vec::new(),
            windows: Vec::new(),
            workspace_count: INITIAL_WORKSPACES,
            current_workspace: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Input {
    Motion {
        x: f64,
        y: f64,
        time: u32,
    },
    Leave {
        time: u32,
    },
    KeyboardLeave {
        time: u32,
    },
    Button {
        code: u32,
        pressed: bool,
        time: u32,
    },
    Axis {
        horizontal: f64,
        vertical: f64,
        time: u32,
    },
    Key {
        code: u32,
        pressed: bool,
        time: u32,
    },
    WaylandKey {
        event: ferroui_wayland::WaylandKeyEvent,
        pressed: bool,
        time: u32,
    },
    WorkArea {
        top: i32,
        bottom: i32,
    },
    Resize {
        width: i32,
        height: i32,
    },
    Move {
        id: u64,
        x: i32,
        y: i32,
    },
    Configure {
        id: u64,
        width: i32,
        height: i32,
    },
    Activate(u64),
    Close(u64),
    SwitchWorkspace(usize),
    MoveToWorkspace {
        id: u64,
        workspace: usize,
    },
    AddWorkspace,
    RemoveWorkspace,
    Fullscreen(u64),
    AlwaysOnTop(u64),
    Maximize(u64),
    Minimize(u64),
    Presented(u64),
}

enum Command {
    Input(Input),
    Stop,
}

pub struct Server {
    socket: OsString,
    input: Sender<Command>,
    latest: Arc<Mutex<Scene>>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Server {
    pub fn start(
        width: i32,
        height: i32,
        name: Option<&str>,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> io::Result<Self> {
        if width <= 0 || height <= 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid compositor output size",
            ));
        }
        let listener = match name {
            Some(name)
                if !name.is_empty()
                    && !name.starts_with('.')
                    && !name.ends_with(".lock")
                    && name
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.')) =>
            {
                let dir = std::env::var_os("XDG_RUNTIME_DIR").ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::NotFound,
                        "XDG_RUNTIME_DIR is required for a child display",
                    )
                })?;
                match std::fs::symlink_metadata(std::path::Path::new(&dir).join(name)) {
                    Ok(_) => {
                        return Err(io::Error::new(
                            io::ErrorKind::AddrInUse,
                            "Private display path already exists",
                        ))
                    }
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                    Err(e) => return Err(e),
                }
                ListeningSocketSource::with_name(name)
            }
            Some(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Expected a Wayland socket basename",
                ))
            }
            None => ListeningSocketSource::new_auto(),
        }
        .map_err(io::Error::other)?;
        let socket = listener.socket_name().to_os_string();
        let latest = Arc::new(Mutex::new(Scene::default()));
        let published = latest.clone();
        let (input, receiver) = channel();
        let (started, ready) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("ferro-wayland-server".into())
            .spawn(move || {
                let run = || -> io::Result<()> {
                    let mut events = EventLoop::<State>::try_new().map_err(io::Error::other)?;
                    let mut display = Display::<State>::new().map_err(io::Error::other)?;
                    let dh = display.handle();
                    let mut state = State::new(&dh, width, height)?;
                    let handle = events.handle();
                    handle
                        .insert_source(listener, move |stream, _, state| {
                            if let Err(error) = state
                                .display
                                .insert_client(stream, Arc::new(ClientData::default()))
                            {
                                eprintln!("Wayland client connection: {error}");
                            }
                        })
                        .map_err(|e| io::Error::other(e.to_string()))?;
                    handle
                        .insert_source(
                            Generic::new(
                                display.as_fd().try_clone_to_owned()?,
                                Interest::READ,
                                Mode::Level,
                            ),
                            |_, _, _| Ok(PostAction::Continue),
                        )
                        .map_err(io::Error::other)?;
                    handle
                        .insert_source(receiver, |event, _, state| match event {
                            Event::Msg(Command::Input(event)) => state.input(event),
                            Event::Msg(Command::Stop) | Event::Closed => state.running = false,
                        })
                        .map_err(|e| io::Error::other(e.to_string()))?;
                    let _ = started.send(Ok(()));
                    while state.running {
                        events
                            .dispatch(None, &mut state)
                            .map_err(io::Error::other)?;
                        display.dispatch_clients(&mut state)?;
                        if let Some(scene) = state.take_scene() {
                            *published.lock().unwrap() = scene;
                            wake();
                        }
                        display.flush_clients()?;
                    }
                    Ok(())
                };
                if let Err(error) = run() {
                    let description = error.to_string();
                    if started.send(Err(error)).is_err() {
                        eprintln!("FerroUI Wayland server stopped: {description}");
                    }
                }
            })?;
        ready.recv().map_err(io::Error::other)??;
        Ok(Self {
            socket,
            input,
            latest,
            worker: Some(worker),
        })
    }
    pub fn socket_name(&self) -> &OsStr {
        &self.socket
    }
    pub fn snapshot(&self) -> Scene {
        self.latest.lock().unwrap().clone()
    }
    pub fn send(&self, event: Input) {
        let _ = self.input.send(Command::Input(event));
    }
    pub fn present_callback(&self) -> impl FnOnce(u64) + Send + 'static {
        let input = self.input.clone();
        move |serial| {
            let _ = input.send(Command::Input(Input::Presented(serial)));
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.input.send(Command::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests;
