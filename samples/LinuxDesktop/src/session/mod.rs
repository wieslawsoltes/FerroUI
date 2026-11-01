//! The desktop shell's compositor-backed workspace and toplevel model.
pub mod commands;
mod wayland;
pub mod wayland_commands;
mod x11;

use crate::shell::appearance::Appearance;
use ferroui_base::Ref;
use ferroui_controls::Window;
use std::{cell::RefCell, io, rc::Rc};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientWindow {
    pub id: u32,
    pub title: String,
    /// `None` denotes a sticky window visible on every workspace.
    pub workspace: Option<usize>,
    pub active: bool,
    pub minimized: bool,
    pub maximized: bool,
    pub(crate) partially_maximized: bool,
    pub fullscreen: bool,
    pub can_fullscreen: bool,
    pub above: bool,
    pub can_raise: bool,
    pub last_activation: u64,
    pub can_maximize: bool,
    pub can_snap: bool,
    pub can_move_workspace: bool,
    pub can_minimize: bool,
    pub can_close: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub current_workspace: usize,
    pub workspace_names: Vec<String>,
    pub windows: Vec<ClientWindow>,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            current_workspace: 0,
            workspace_names: vec!["Workspace 1".into()],
            windows: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapSide {
    Left,
    Right,
}

#[derive(Clone, Copy)]
pub enum ShortcutOrigin {
    X11(x11::ShortcutOrigin),
    Wayland(wayland::ShortcutOrigin),
}

enum Backend {
    X11(Rc<x11::Session>),
    Wayland(Rc<wayland::WaylandSession>),
    Composed(Rc<crate::compositor::Desktop>),
}

pub struct Session {
    backend: Backend,
    wayland_commands: RefCell<Option<Rc<wayland_commands::Endpoint>>>,
}

// These operations have the same semantics and arguments on both managers. The
// platform implementations decide capability and report the compositor's reply.
macro_rules! dispatch {
    ($(fn $name:ident(&self $(, $arg:ident: $ty:ty)*) -> $result:ty;)*) => {$(
        pub fn $name(&self, $($arg: $ty),*) -> $result {
            match &self.backend {
                Backend::X11(x) => x.$name($($arg),*),
                Backend::Wayland(w) => w.$name($($arg),*),
                Backend::Composed(c) => c.$name($($arg),*),
            }
        }
    )*}
}

impl Session {
    pub fn connect() -> Option<Rc<Self>> {
        Some(Rc::new(Self {
            backend: Backend::X11(x11::Session::connect()?),
            wayland_commands: RefCell::default(),
        }))
    }
    pub fn connect_wayland() -> io::Result<Rc<Self>> {
        Ok(Rc::new(Self {
            backend: Backend::Wayland(wayland::WaylandSession::connect()?),
            wayland_commands: RefCell::default(),
        }))
    }

    pub fn composed(manager: Rc<crate::compositor::Desktop>) -> Rc<Self> {
        Rc::new(Self {
            backend: Backend::Composed(manager),
            wayland_commands: RefCell::default(),
        })
    }
    pub fn compositor(&self) -> Option<&Rc<crate::compositor::Desktop>> {
        match &self.backend {
            Backend::Composed(c) => Some(c),
            _ => None,
        }
    }
    dispatch! {
        fn snapshot(&self) -> Snapshot;
        fn activate(&self, id: u32) -> bool;
        fn close(&self, id: u32) -> bool;
        fn minimize(&self, id: u32) -> bool;
        fn toggle_fullscreen(&self, id: u32) -> bool;
        fn toggle_maximized(&self, id: u32) -> bool;
        fn toggle_above(&self, id: u32) -> bool;
        fn snap(&self, id: u32, side: SnapSide) -> bool;
        fn move_to_workspace(&self, id: u32, index: usize) -> bool;
        fn switch_workspace(&self, index: usize) -> bool;
        fn add_workspace(&self) -> bool;
        fn remove_empty_last_workspace(&self) -> bool;
    }
    pub fn focused_output(&self) -> Option<String> {
        match &self.backend {
            Backend::X11(_) => None,
            Backend::Wayland(w) => w.focused_output(),
            Backend::Composed(_) => None,
        }
    }
    pub fn can_remove_empty_last_workspace(&self, snapshot: &Snapshot) -> bool {
        match &self.backend {
            Backend::X11(_) => {
                snapshot.workspace_names.len() > 1
                    && !snapshot
                        .windows
                        .iter()
                        .any(|w| w.workspace == Some(snapshot.workspace_names.len() - 1))
            }
            Backend::Wayland(w) => w.can_remove_last(),
            Backend::Composed(c) => c.can_remove_last(),
        }
    }
    pub fn subscribe(&self, callback: impl Fn() + 'static) {
        match &self.backend {
            Backend::X11(x) => x.subscribe(callback),
            Backend::Wayland(w) => w.subscribe(callback),
            Backend::Composed(c) => c.subscribe(callback),
        }
    }
    pub fn claim_commands(&self, lease: Option<wayland_commands::DisplayLease>) -> io::Result<()> {
        match &self.backend {
            Backend::X11(x) => x.claim_commands(),
            Backend::Wayland(_) | Backend::Composed(_) => {
                self.wayland_commands
                    .replace(Some(wayland_commands::Endpoint::bind(lease.ok_or_else(
                        || io::Error::other("Wayland display ownership required"),
                    )?)?));
                Ok(())
            }
        }
    }
    pub fn publish_scene(&self, name: &str) {
        match &self.backend {
            Backend::X11(x) => x.publish_scene(name),
            Backend::Wayland(_) | Backend::Composed(_) => {
                if let Some(endpoint) = self.wayland_commands.borrow().as_ref() {
                    endpoint.publish_scene(name);
                }
            }
        }
    }
    pub fn on_command(&self, f: impl Fn(commands::Command) + 'static) {
        match &self.backend {
            Backend::X11(x) => x.on_command(f),
            Backend::Wayland(_) | Backend::Composed(_) => {
                if let Some(endpoint) = self.wayland_commands.borrow().as_ref() {
                    endpoint.set_handler(Rc::new(f));
                }
            }
        }
    }
    pub fn shortcut_origin(&self) -> Option<ShortcutOrigin> {
        match &self.backend {
            Backend::X11(x) => x.shortcut_origin().map(ShortcutOrigin::X11),
            Backend::Wayland(w) => w.shortcut_origin().map(ShortcutOrigin::Wayland),
            Backend::Composed(_) => None,
        }
    }
    pub fn restore_shortcut_origin(&self, origin: ShortcutOrigin) {
        match (&self.backend, origin) {
            (Backend::X11(x), ShortcutOrigin::X11(origin)) => x.restore_shortcut_origin(origin),
            (Backend::Wayland(w), ShortcutOrigin::Wayland(origin)) => {
                w.restore_shortcut_origin(origin)
            }
            _ => {}
        }
    }
    pub fn configure_desktop(&self, window: &Ref<Window>) {
        if let Backend::X11(x) = &self.backend {
            x.configure_desktop(window);
        }
    }
    pub fn set_panel_insets(&self, top: f64, bottom: f64) {
        match &self.backend {
            Backend::X11(x) => x.set_panel_insets(top, bottom),
            Backend::Composed(c) => c.set_panel_insets(top, bottom),
            Backend::Wayland(_) => {}
        }
    }
    pub fn set_window_appearance(&self, appearance: Appearance) -> io::Result<()> {
        match &self.backend {
            Backend::X11(x) => x.set_window_appearance(appearance),
            Backend::Wayland(w) if self.wayland_commands.borrow().is_some() => {
                w.set_window_appearance(appearance)
            }
            Backend::Wayland(_) | Backend::Composed(_) => Ok(()),
        }
    }
    pub fn on_reveal(&self, f: impl Fn() + 'static) {
        if let Backend::X11(x) = &self.backend {
            x.on_reveal(f);
        }
    }
    pub fn on_hidden(&self, f: impl Fn() + 'static) {
        if let Backend::X11(x) = &self.backend {
            x.on_hidden(f);
        }
    }
    pub fn reveal_desktop(&self) -> bool {
        match &self.backend {
            Backend::X11(x) => x.reveal_desktop(),
            Backend::Wayland(_) | Backend::Composed(_) => true,
        }
    }
    pub fn restore_windows(&self) -> bool {
        match &self.backend {
            Backend::X11(x) => x.restore_windows(),
            Backend::Wayland(_) | Backend::Composed(_) => true,
        }
    }
}
