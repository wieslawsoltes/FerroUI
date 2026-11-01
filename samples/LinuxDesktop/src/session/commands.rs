//! One manager per X11 screen. A short-lived sender targets the owner's private window;
//! the same FerroUI X11 dispatcher that serves the shell delivers the fixed action.
use ferroui_x11::x11_structs::XEventName;
use ferroui_x11::x11_window_info::X11WindowInfo;
use ferroui_x11::xlib::{self, Atom, XDisplay, XID};
use ferroui_x11::FerroX11Platform;
use std::cell::RefCell;
use std::io;
use std::rc::Rc;

#[repr(i32)]
#[derive(Clone, Copy)]
pub enum Command {
    Applications = 1,
    Workspaces = 2,
    QuickSettings = 3,
    Notifications = 4,
}

impl Command {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "applications" => Some(Self::Applications),
            "workspaces" => Some(Self::Workspaces),
            "settings" => Some(Self::QuickSettings),
            "notifications" => Some(Self::Notifications),
            _ => None,
        }
    }

    fn decode(value: std::ffi::c_long) -> Option<Self> {
        match value {
            1 => Some(Self::Applications),
            2 => Some(Self::Workspaces),
            3 => Some(Self::QuickSettings),
            4 => Some(Self::Notifications),
            _ => None,
        }
    }
}

const VERSION: std::ffi::c_long = 1;

fn selection(display: XDisplay, existing: bool) -> Atom {
    let screen = xlib::x_default_screen(display);
    xlib::x_intern_atom(display, &format!("_FERRO_DESKTOP_COMMAND_S{screen}"), existing)
}

/// Selection inspection and ownership transfer must be atomic relative to a second shell.
/// Only short nonblocking X operations, no external process or callback, may run under this grab.
struct ServerGrab(XDisplay);
impl ServerGrab {
    fn new(display: XDisplay) -> Self {
        xlib::x_grab_server(display);
        Self(display)
    }
}
impl Drop for ServerGrab {
    fn drop(&mut self) {
        xlib::x_ungrab_server(self.0);
        xlib::x_flush(self.0);
    }
}

fn with_owner<T>(f: impl FnOnce(XDisplay, Atom, XID) -> io::Result<T>) -> io::Result<T> {
    xlib::x_init_threads();
    let display = xlib::x_open_display().ok_or_else(|| io::Error::other("Cannot connect to the X11 display"))?;
    let atom = selection(display, true);
    let _grab = ServerGrab::new(display);
    let owner = if atom == 0 {
        0
    } else {
        xlib::x_get_selection_owner(display, atom)
    };
    if owner == 0 {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No Ferro Desktop session owns this X11 screen",
        ));
    }
    f(display, atom, owner)
}

pub fn current_scene() -> io::Result<String> {
    with_owner(|display, _, owner| {
        let scene = xlib::x_intern_atom(display, "_FERRO_DESKTOP_SCENE", true);
        let utf8 = xlib::x_intern_atom(display, "UTF8_STRING", true);
        if scene != 0 && utf8 != 0 {
            let p = xlib::x_get_window_property(display, owner, scene, 0, 16, false, utf8);
            if p.status == 0 && p.actual_type == utf8 && p.actual_format == 8 && p.bytes_after == 0 {
                return String::from_utf8(p.data).map_err(io::Error::other);
            }
        }
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The desktop scene has not been published yet",
        ))
    })
}

pub fn send(command: Command) -> io::Result<()> {
    with_owner(|display, atom, owner| {
        let mut event = xlib::new_event();
        let message = xlib::client_message_event_mut(&mut event);
        message.type_ = XEventName::ClientMessage as i32;
        message.window = owner;
        message.message_type = atom;
        message.format = 32;
        message.data.set_long(0, VERSION);
        message.data.set_long(1, command as _);
        if xlib::x_send_event(display, owner, false, 0, &mut event) == 0 {
            return Err(io::Error::other("Could not send the desktop command"));
        }
        Ok(())
    })
}

type Handler = Rc<dyn Fn(Command)>;

pub(super) struct Endpoint {
    platform: Rc<FerroX11Platform>,
    window: XID,
    scene_atom: Atom,
    handler: RefCell<Option<Handler>>,
}

impl Endpoint {
    pub(super) fn claim(platform: &Rc<FerroX11Platform>) -> io::Result<Rc<Self>> {
        let display = platform.display();
        let atom = selection(display, false);
        let _grab = ServerGrab::new(display);
        if xlib::x_get_selection_owner(display, atom) != 0 {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Ferro Desktop already owns this X11 screen",
            ));
        }
        let window = xlib::x_create_simple_window(display, platform.info().root_window(), 0, 0, 1, 1, 0, 0, 0);
        xlib::x_set_selection_owner(display, atom, window, 0);
        if xlib::x_get_selection_owner(display, atom) != window {
            xlib::x_destroy_window(display, window);
            return Err(io::Error::other("Could not claim the desktop command selection"));
        }
        let this = Rc::new(Self {
            platform: platform.clone(),
            window,
            scene_atom: xlib::x_intern_atom(display, "_FERRO_DESKTOP_SCENE", false),
            handler: RefCell::default(),
        });
        let weak = Rc::downgrade(&this);
        platform.set_window(
            window,
            X11WindowInfo::new(
                Rc::new(move |event| {
                    if xlib::event_type(event) != XEventName::ClientMessage as i32 {
                        return;
                    }
                    let message = xlib::client_message_event(event);
                    if message.window != window
                        || message.message_type != atom
                        || message.format != 32
                        || message.data.get_long(0) != VERSION
                        || xlib::x_get_selection_owner(display, atom) != window
                    {
                        return;
                    }
                    if let (Some(command), Some(endpoint)) = (Command::decode(message.data.get_long(1)), weak.upgrade())
                    {
                        if let Some(handler) = endpoint.handler.borrow().clone() {
                            handler(command);
                        }
                    }
                }),
                None,
            ),
        );
        Ok(this)
    }

    pub(super) fn publish_scene(&self, scene: &str) {
        let info = self.platform.info();
        xlib::x_change_property_bytes(
            info.display(),
            self.window,
            self.scene_atom,
            info.atoms().UTF8_STRING,
            xlib::PropertyMode::Replace,
            scene.as_bytes(),
        );
        xlib::x_flush(info.display());
    }

    pub(super) fn set_handler(&self, handler: Handler) {
        self.handler.replace(Some(handler));
    }
}

impl Drop for Endpoint {
    fn drop(&mut self) {
        self.platform.remove_window(self.window);
        // Destroying our own window releases the selection, even on abnormal process exit.
        // Never clear another process's selection if ownership was transferred.
        xlib::x_destroy_window(self.platform.display(), self.window);
        xlib::x_flush(self.platform.display());
    }
}
