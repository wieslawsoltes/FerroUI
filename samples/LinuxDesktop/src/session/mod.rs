//! A pager over the X11 connection and event pump that FerroUI already owns. The window manager
//! owns focus, stacking, workspaces and client lifetime. We never reparent or draw client windows.

pub mod commands;
mod decorations;
mod placement;

pub use placement::SnapSide;

use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{FerroLocator, LocatorExtensions, PixelRect, Ref};
use ferroui_controls::platform::{X11NetWmWindowType, X11Properties};
use ferroui_controls::{Window, WindowDecorations};
use ferroui_x11::x11_structs::{EventMask, XEventName};
use ferroui_x11::x11_window_info::X11WindowInfo;
use ferroui_x11::xlib::{self, Atom, PropertyMode, XID};
use ferroui_x11::FerroX11Platform;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientWindow {
    pub id: u32,
    pub title: String,
    /// `None` denotes a sticky window visible on every workspace.
    pub workspace: Option<usize>,
    pub active: bool,
    pub minimized: bool,
    pub maximized: bool,
    partially_maximized: bool,
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
        Self { current_workspace: 0, workspace_names: vec!["Workspace 1".into()], windows: Vec::new() }
    }
}

#[derive(Default)]
struct Dirty {
    membership: bool,
    workspaces: bool,
    active: bool,
    showing: bool,
    clients: HashSet<XID>,
    titles: HashSet<XID>,
}

impl Dirty {
    fn all() -> Self {
        Self { membership: true, workspaces: true, active: true, showing: true, ..Self::default() }
    }
}

#[derive(Clone, Copy)]
pub struct ShortcutOrigin {
    workspace: usize,
    window: XID,
}

pub struct Session {
    commands: RefCell<Option<Rc<commands::Endpoint>>>,
    platform: Rc<FerroX11Platform>,
    order: RefCell<Vec<XID>>,
    // Some managed clients are docks or intentionally hidden from task lists. Observe those too
    // because a client can change its type or SKIP_TASKBAR flag after mapping.
    clients: RefCell<HashMap<XID, Option<ClientWindow>>>,
    active: Cell<XID>,
    activation_sequence: Cell<u64>,
    pending_snaps: RefCell<HashMap<XID, SnapSide>>,
    move_action: Atom,
    resize_action: Atom,
    close_action: Atom,
    workspace_action: Atom,
    above_action: Atom,
    snapshot: RefCell<Snapshot>,
    dirty: RefCell<Dirty>,
    queued: Cell<bool>,
    root_subscription: Cell<u64>,
    changed: RefCell<Option<Rc<dyn Fn()>>>,
    revealed: RefCell<Option<Rc<dyn Fn()>>>,
    hidden: RefCell<Option<Rc<dyn Fn()>>>,
    desktop: Cell<XID>,
    showing: Cell<bool>,
    screens_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    screen_bounds: Cell<Option<(PixelRect, f64, i32)>>,
    panel_insets: Cell<(f64, f64)>,
}

impl Session {
    pub fn connect() -> Option<Rc<Self>> {
        let platform = FerroLocator::current().get_service::<FerroX11Platform>()?;
        let display = platform.display();
        let session = Rc::new(Self {
            commands: RefCell::default(),
            platform,
            order: RefCell::default(),
            clients: RefCell::default(),
            active: Cell::new(0),
            activation_sequence: Cell::new(0),
            pending_snaps: RefCell::default(),
            move_action: xlib::x_intern_atom(display, "_NET_WM_ACTION_MOVE", false),
            resize_action: xlib::x_intern_atom(display, "_NET_WM_ACTION_RESIZE", false),
            close_action: xlib::x_intern_atom(display, "_NET_WM_ACTION_CLOSE", false),
            workspace_action: xlib::x_intern_atom(display, "_NET_WM_ACTION_CHANGE_DESKTOP", false),
            above_action: xlib::x_intern_atom(display, "_NET_WM_ACTION_ABOVE", false),
            snapshot: RefCell::default(),
            dirty: RefCell::new(Dirty::all()),
            queued: Cell::new(false),
            root_subscription: Cell::new(0),
            changed: RefCell::default(),
            revealed: RefCell::default(),
            hidden: RefCell::default(),
            desktop: Cell::new(0),
            showing: Cell::new(false),
            screens_subscription: RefCell::default(),
            screen_bounds: Cell::new(None),
            panel_insets: Cell::new((44.0, 0.0)),
        });
        let weak = Rc::downgrade(&session);
        let token = session.platform.globals().root_property_changed.subscribe(move |atom| {
            if let Some(session) = weak.upgrade() {
                session.root_changed(atom);
            }
        });
        session.root_subscription.set(token);
        session.refresh();
        Some(session)
    }

    /// This only clones cached state. Rendering never performs X server round trips.
    pub fn snapshot(&self) -> Snapshot {
        self.snapshot.borrow().clone()
    }

    pub fn subscribe(&self, changed: impl Fn() + 'static) {
        self.changed.replace(Some(Rc::new(changed)));
    }
    pub fn claim_commands(&self) -> std::io::Result<()> {
        self.commands.replace(Some(commands::Endpoint::claim(&self.platform)?));
        Ok(())
    }

    /// The EWMH focus at the time a global shortcut was invoked. Query the root, since a
    /// just-focused client can precede our next coalesced snapshot. Include transient clients.
    pub fn shortcut_origin(&self) -> Option<ShortcutOrigin> {
        let info = self.platform.info();
        let a = info.atoms();
        let active = self.number(info.root_window(), a._NET_ACTIVE_WINDOW, a.WINDOW)?;
        if active == self.desktop.get() || !self.order.borrow().contains(&active) {
            return None;
        }
        let workspace = self.number(info.root_window(), a._NET_CURRENT_DESKTOP, a.CARDINAL)? as usize;
        Some(ShortcutOrigin { workspace, window: active })
    }

    pub fn restore_shortcut_origin(&self, origin: ShortcutOrigin) {
        let ShortcutOrigin { workspace, window: id } = origin;
        if !self.known_workspace(workspace) || !self.order.borrow().contains(&id) {
            return;
        }
        let info = self.platform.info();
        let a = info.atoms();
        // If the application closed or was moved deliberately, do not chase it on another desk.
        let assigned = self.number(id, a._NET_WM_DESKTOP, a.CARDINAL);
        if !assigned.is_some_and(|desk| desk as usize == workspace || desk as u32 == u32::MAX) {
            return;
        }
        if self.number(info.root_window(), a._NET_CURRENT_DESKTOP, a.CARDINAL) != Some(workspace as XID) {
            self.switch_workspace(workspace);
        }
        self.message(id, a._NET_ACTIVE_WINDOW, &[2, info.last_activity_timestamp(), self.desktop.get()]);
    }

    pub fn publish_scene(&self, name: &str) {
        if let Some(endpoint) = self.commands.borrow().as_ref() {
            endpoint.publish_scene(name);
        }
    }

    pub fn on_command(&self, handler: impl Fn(commands::Command) + 'static) {
        if let Some(endpoint) = self.commands.borrow().as_ref() {
            endpoint.set_handler(Rc::new(handler));
        }
    }

    pub fn on_reveal(&self, revealed: impl Fn() + 'static) {
        self.revealed.replace(Some(Rc::new(revealed)));
    }

    pub fn on_hidden(&self, hidden: impl Fn() + 'static) {
        self.hidden.replace(Some(Rc::new(hidden)));
    }

    fn root_changed(self: &Rc<Self>, atom: Atom) {
        let a = self.platform.info().atoms();
        let mut dirty = self.dirty.borrow_mut();
        if atom == a._NET_CLIENT_LIST {
            dirty.membership = true;
        } else if [a._NET_CURRENT_DESKTOP, a._NET_NUMBER_OF_DESKTOPS, a._NET_DESKTOP_NAMES].contains(&atom) {
            dirty.workspaces = true;
        } else if atom == a._NET_ACTIVE_WINDOW {
            dirty.active = true;
        } else if atom == a._NET_SHOWING_DESKTOP {
            dirty.showing = true;
        } else if atom == a._NET_SUPPORTING_WM_CHECK {
            *dirty = Dirty::all();
        } else {
            return;
        }
        drop(dirty);
        self.queue();
    }

    fn queue(self: &Rc<Self>) {
        if self.queued.replace(true) {
            return;
        }
        let weak = Rc::downgrade(self);
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(session) = weak.upgrade() {
                    session.refresh();
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    }

    fn refresh(self: &Rc<Self>) {
        self.queued.set(false);
        let mut dirty = self.dirty.take();
        let info = self.platform.info();
        let a = info.atoms();
        let root = info.root_window();
        let mut reorder = dirty.membership;
        let mut windows_changed = false;
        if dirty.membership {
            let ids = self.longs(root, a._NET_CLIENT_LIST, a.WINDOW, 16_384);
            let live: HashSet<_> = ids.iter().copied().collect();
            let previous = self.order.replace(ids);
            for id in previous.into_iter().filter(|id| !live.contains(id)) {
                self.pending_snaps.borrow_mut().remove(&id);
                if self.clients.borrow_mut().remove(&id).is_some() {
                    self.unwatch(id);
                }
            }
            for &id in self.order.borrow().iter() {
                if !self.clients.borrow().contains_key(&id) && self.platform.get_window(id).is_none() {
                    self.watch(id);
                    dirty.clients.insert(id);
                }
            }
        }
        for id in dirty.clients {
            dirty.titles.remove(&id);
            if self.clients.borrow().contains_key(&id) {
                let client = self.read_client(id);
                let ready_to_snap = client.as_ref().is_some_and(|w| !w.partially_maximized && !w.fullscreen);
                let mut clients = self.clients.borrow_mut();
                let slot = clients.get_mut(&id).expect("watched client");
                reorder |= slot.is_some() != client.is_some();
                windows_changed |= *slot != client;
                *slot = client;
                drop(clients);
                if ready_to_snap {
                    if let Some(side) = self.pending_snaps.borrow_mut().remove(&id) {
                        self.place_half(id, side);
                    }
                }
            }
        }
        // Editors and terminals can update their title repeatedly while typing. A title event
        // must not query state, workspace, allowed actions and window type again.
        for id in dirty.titles {
            if let Some(Some(client)) = self.clients.borrow_mut().get_mut(&id) {
                let title = self.client_title(id);
                windows_changed |= client.title != title;
                client.title = title;
            }
        }
        if dirty.active {
            let active = self.number(root, a._NET_ACTIVE_WINDOW, a.WINDOW).unwrap_or(0);
            let previous = self.active.replace(active);
            if previous != active {
                let mut clients = self.clients.borrow_mut();
                if let Some(Some(client)) = clients.get_mut(&previous) {
                    client.active = false;
                    windows_changed = true;
                }
                if let Some(Some(client)) = clients.get_mut(&active) {
                    client.active = true;
                    windows_changed = true;
                    let serial = self.activation_sequence.get().saturating_add(1);
                    self.activation_sequence.set(serial);
                    client.last_activation = serial;
                }
            }
        }
        let mut snapshot = self.snapshot.borrow_mut();
        let mut changed = false;
        if dirty.workspaces {
            let count = self.number(root, a._NET_NUMBER_OF_DESKTOPS, a.CARDINAL).unwrap_or(1);
            let names = workspace_names(count as usize, &self.bytes(root, a._NET_DESKTOP_NAMES, a.UTF8_STRING));
            let current = self.number(root, a._NET_CURRENT_DESKTOP, a.CARDINAL).unwrap_or(0) as usize;
            let current = current.min(names.len() - 1);
            changed = snapshot.current_workspace != current || snapshot.workspace_names != names;
            snapshot.current_workspace = current;
            snapshot.workspace_names = names;
        }
        if reorder {
            let clients = self.clients.borrow();
            let windows: Vec<_> = self.order.borrow().iter().filter_map(|id| clients.get(id)?.clone()).collect();
            changed |= windows != snapshot.windows;
            snapshot.windows = windows;
        } else if windows_changed {
            // Keep the list and strings of unaffected windows across title/state notifications.
            let clients = self.clients.borrow();
            for published in &mut snapshot.windows {
                if let Some(Some(client)) = clients.get(&XID::from(published.id)) {
                    if published != client {
                        published.clone_from(client);
                        changed = true;
                    }
                }
            }
        }
        drop(snapshot);
        if changed {
            if let Some(callback) = self.changed.borrow().clone() {
                callback();
            }
        }
        if dirty.showing {
            let showing = self.number(root, a._NET_SHOWING_DESKTOP, a.CARDINAL) == Some(1);
            if self.showing.replace(showing) != showing && self.desktop.get() != 0 {
                let callback = if showing { &self.revealed } else { &self.hidden };
                if let Some(callback) = callback.borrow().clone() {
                    callback();
                }
            }
        }
    }

    fn watch(self: &Rc<Self>, id: XID) {
        let weak = Rc::downgrade(self);
        self.platform.set_window(
            id,
            X11WindowInfo::new(
                Rc::new(move |event| {
                    if xlib::event_type(event) != XEventName::PropertyNotify as i32 {
                        return;
                    }
                    if let Some(session) = weak.upgrade() {
                        let a = session.platform.info().atoms();
                        let atom = xlib::property_event(event).atom;
                        if atom == a._NET_WM_NAME || atom == a.WM_NAME {
                            session.dirty.borrow_mut().titles.insert(id);
                            session.queue();
                        } else if [a._NET_WM_ALLOWED_ACTIONS, a._NET_WM_DESKTOP, a._NET_WM_STATE, a._NET_WM_WINDOW_TYPE]
                            .contains(&atom)
                        {
                            session.dirty.borrow_mut().clients.insert(id);
                            session.queue();
                        }
                    }
                }),
                None,
            ),
        );
        xlib::x_select_input(self.platform.display(), id, EventMask::PROPERTY_CHANGE_MASK.bits() as _);
        self.clients.borrow_mut().insert(id, None);
    }

    fn unwatch(&self, id: XID) {
        // A destroyed XID can be reused. Never unregister one of the framework's own top levels.
        if self.platform.get_window(id).is_some_and(|entry| entry.window().is_none()) {
            self.platform.remove_window(id);
            xlib::x_select_input(self.platform.display(), id, 0);
        }
    }

    fn bytes(&self, id: XID, atom: Atom, kind: Atom) -> Vec<u8> {
        let p = xlib::x_get_window_property(self.platform.display(), id, atom, 0, 4096, false, kind);
        if p.status == 0 && p.actual_type == kind && p.actual_format == 8 {
            p.data
        } else {
            Vec::new()
        }
    }

    fn longs(&self, id: XID, atom: Atom, kind: Atom, limit: i64) -> Vec<XID> {
        let p = xlib::x_get_window_property(self.platform.display(), id, atom, 0, limit as _, false, kind);
        if p.status == 0 && p.actual_type == kind && p.actual_format == 32 {
            p.longs()
        } else {
            Vec::new()
        }
    }

    fn number(&self, id: XID, atom: Atom, kind: Atom) -> Option<XID> {
        xlib::x_get_window_property_as_int_ptr(self.platform.display(), id, atom, kind)
    }

    fn read_client(&self, id: XID) -> Option<ClientWindow> {
        let a = self.platform.info().atoms();
        let kinds = self.longs(id, a._NET_WM_WINDOW_TYPE, a.ATOM, 32);
        let states = self.longs(id, a._NET_WM_STATE, a.ATOM, 64);
        if kinds.iter().any(|k| {
            [
                a._NET_WM_WINDOW_TYPE_DOCK,
                a._NET_WM_WINDOW_TYPE_DESKTOP,
                a._NET_WM_WINDOW_TYPE_SPLASH,
                a._NET_WM_WINDOW_TYPE_MENU,
            ]
            .contains(k)
        }) || states.contains(&a._NET_WM_STATE_SKIP_TASKBAR)
        {
            return None;
        }
        // A missing hint leaves the decision to the WM. A present, empty list means that
        // this particular client does not allow any window-manager operations.
        let actions =
            xlib::x_get_window_property(self.platform.display(), id, a._NET_WM_ALLOWED_ACTIONS, 0, 128, false, a.ATOM);
        let allowed = (actions.actual_type == a.ATOM && actions.actual_format == 32).then(|| actions.longs());
        let permits = |action| allowed.as_ref().is_none_or(|list| list.contains(&action));
        let horz = states.contains(&a._NET_WM_STATE_MAXIMIZED_HORZ);
        let vert = states.contains(&a._NET_WM_STATE_MAXIMIZED_VERT);
        let desktop = self.number(id, a._NET_WM_DESKTOP, a.CARDINAL).unwrap_or(0) as u32;
        Some(ClientWindow {
            id: id as u32,
            title: self.client_title(id),
            workspace: (desktop != u32::MAX).then_some(desktop as usize),
            active: self.active.get() == id,
            minimized: states.contains(&a._NET_WM_STATE_HIDDEN),
            maximized: horz && vert,
            partially_maximized: horz || vert,
            fullscreen: states.contains(&a._NET_WM_STATE_FULLSCREEN),
            can_fullscreen: permits(a._NET_WM_ACTION_FULLSCREEN),
            above: states.contains(&a._NET_WM_STATE_ABOVE),
            can_raise: permits(self.above_action),
            last_activation: self
                .clients
                .borrow()
                .get(&id)
                .and_then(Option::as_ref)
                .map_or(0, |old| old.last_activation),
            can_maximize: permits(a._NET_WM_ACTION_MAXIMIZE_HORZ) && permits(a._NET_WM_ACTION_MAXIMIZE_VERT),
            can_snap: permits(self.move_action) && permits(self.resize_action),
            can_move_workspace: permits(self.workspace_action),
            can_minimize: permits(a._NET_WM_ACTION_MINIMIZE),
            can_close: permits(self.close_action),
        })
    }

    fn client_title(&self, id: XID) -> String {
        let a = self.platform.info().atoms();
        let mut title = String::from_utf8_lossy(&self.bytes(id, a._NET_WM_NAME, a.UTF8_STRING)).into_owned();
        if title.trim().is_empty() {
            title = xlib::x_fetch_name(self.platform.display(), id).unwrap_or_default();
        }
        if title.trim().is_empty() {
            title = "Untitled window".into();
        }
        title
    }

    fn message(&self, id: XID, atom: Atom, data: &[XID]) -> bool {
        let info = self.platform.info();
        if !self.platform.globals().net_supported().is_some_and(|atoms| atoms.contains(&atom)) {
            return false;
        }
        let mut event = xlib::new_event();
        let message = xlib::client_message_event_mut(&mut event);
        message.type_ = XEventName::ClientMessage as i32;
        message.display = info.display().as_ptr().cast();
        message.window = id;
        message.message_type = atom;
        message.format = 32;
        for (i, &value) in data.iter().enumerate() {
            message.data.set_long(i, value as _);
        }
        let mask = EventMask::SUBSTRUCTURE_NOTIFY_MASK | EventMask::SUBSTRUCTURE_REDIRECT_MASK;
        let sent = xlib::x_send_event(info.display(), info.root_window(), false, mask.bits() as _, &mut event);
        xlib::x_flush(info.display());
        sent != 0
    }

    fn known_client(&self, id: u32) -> bool {
        self.clients.borrow().get(&XID::from(id)).is_some_and(Option::is_some)
    }
    fn known_workspace(&self, index: usize) -> bool {
        index < self.snapshot.borrow().workspace_names.len()
    }

    pub fn switch_workspace(&self, index: usize) -> bool {
        let info = self.platform.info();
        self.known_workspace(index)
            && self.message(
                info.root_window(),
                info.atoms()._NET_CURRENT_DESKTOP,
                &[index as XID, info.last_activity_timestamp()],
            )
    }

    pub fn add_workspace(&self) -> bool {
        let snapshot = self.snapshot.borrow();
        let count = snapshot.workspace_names.len();
        count < 256 && self.set_workspace_count(count + 1)
    }

    pub fn remove_empty_last_workspace(&self) -> bool {
        let snapshot = self.snapshot.borrow();
        let count = snapshot.workspace_names.len();
        if count <= 1 || snapshot.windows.iter().any(|client| client.workspace == Some(count - 1)) {
            return false;
        }
        // A newly mapped or skip-taskbar client may not have made it into this snapshot.
        // Recheck actual membership before asking the WM to retire the workspace.
        let info = self.platform.info();
        let a = info.atoms();
        let members = xlib::x_get_window_property(info.display(), info.root_window(), a._NET_CLIENT_LIST,
            0, 16_384, false, a.WINDOW);
        if members.status != 0 || members.actual_type != a.WINDOW || members.actual_format != 32 || members.bytes_after != 0 {
            return false;
        }
        if members.longs().into_iter().any(|id| {
            self.number(id, a._NET_WM_DESKTOP, a.CARDINAL) == Some((count - 1) as XID)
        }) {
            return false;
        }
        self.set_workspace_count(count - 1)
    }

    fn set_workspace_count(&self, count: usize) -> bool {
        let info = self.platform.info();
        let atom = info.atoms()._NET_NUMBER_OF_DESKTOPS;
        // If the server changed first, or the actual count exceeds the UI's bound,
        // a delta based on our displayed list must not remove additional workspaces.
        let displayed = self.snapshot.borrow().workspace_names.len() as XID;
        self.number(info.root_window(), atom, info.atoms().CARDINAL) == Some(displayed)
            && self.message(info.root_window(), atom, &[count as XID])
    }

    pub fn activate(&self, id: u32) -> bool {
        let info = self.platform.info();
        self.known_client(id)
            && self.message(
                id.into(),
                info.atoms()._NET_ACTIVE_WINDOW,
                &[2, info.last_activity_timestamp(), self.active.get()],
            )
    }

    pub fn close(&self, id: u32) -> bool {
        let info = self.platform.info();
        self.known_client(id)
            && self.message(id.into(), info.atoms()._NET_CLOSE_WINDOW, &[info.last_activity_timestamp(), 2])
    }

    pub fn move_to_workspace(&self, id: u32, index: usize) -> bool {
        self.known_client(id)
            && self.known_workspace(index)
            && self.message(id.into(), self.platform.info().atoms()._NET_WM_DESKTOP, &[index as XID, 2])
    }

    pub fn minimize(&self, id: u32) -> bool {
        if !self.known_client(id) {
            return false;
        }
        let info = self.platform.info();
        let result = xlib::x_iconify_window(info.display(), id.into(), info.default_screen());
        xlib::x_flush(info.display());
        result != 0
    }

    pub fn reveal_desktop(&self) -> bool {
        self.set_showing(true)
    }
    pub fn restore_windows(&self) -> bool {
        self.set_showing(false)
    }

    fn set_showing(&self, showing: bool) -> bool {
        let info = self.platform.info();
        self.desktop.get() != 0
            && self.message(info.root_window(), info.atoms()._NET_SHOWING_DESKTOP, &[XID::from(showing)])
    }

    /// Change decorations only on the Openbox instance that the login supervisor started.
    /// Check its live root property as well as the shell mode so nested sessions cannot
    /// reload a surrounding desktop's window manager.
    pub fn set_window_appearance(&self, appearance: crate::shell::appearance::Appearance) -> std::io::Result<()> {
        use std::os::unix::ffi::OsStrExt;
        let Some(state) = std::env::var_os("FERRO_DESKTOP_SESSION_DIR").map(std::path::PathBuf::from) else {
            return Ok(());
        };
        if self.desktop.get() == 0 {
            return Ok(());
        }
        let info = self.platform.info();
        let property = xlib::x_intern_atom(info.display(), "_OB_CONFIG_FILE", true);
        if property == 0
            || self.bytes(info.root_window(), property, info.atoms().UTF8_STRING)
                != state.join("openbox.xml").as_os_str().as_bytes()
        {
            return Ok(());
        }
        decorations::apply(&state, appearance)
    }

    /// Apply these hints before the WM sees MapRequest. A fullscreen application sits above
    /// real clients, whereas a desktop-type window stays below them and survives Show Desktop.
    pub fn configure_desktop(self: &Rc<Self>, window: &Ref<Window>) {
        let Some(handle) = window.try_get_platform_handle() else {
            return;
        };
        self.desktop.set(handle.handle() as XID);
        X11Properties::set_net_wm_window_type(window, X11NetWmWindowType::Desktop);
        X11Properties::set_wm_class(window, Some("FerroDesktop".to_owned()));
        window.set_window_decorations(WindowDecorations::None);
        window.set_show_in_taskbar(false);
        window.set_show_activated(false);
        let info = self.platform.info();
        let a = info.atoms();
        xlib::x_change_property_longs(
            info.display(),
            self.desktop.get(),
            a._NET_WM_DESKTOP,
            a.CARDINAL,
            PropertyMode::Replace,
            &[u32::MAX.into()],
        );
        self.screen_bounds.set(None);
        self.fit_primary(window);
        {
            let screens = window.screens();
            let weak_window = window.downgrade();
            let weak_session = Rc::downgrade(self);
            let token = screens.changed(move || {
                if let (Some(session), Some(window)) = (weak_session.upgrade(), weak_window.upgrade()) {
                    session.fit_primary(&window);
                }
            });
            if let Some(previous) = self.screens_subscription.replace(Some(token)) {
                previous.dispose();
            }
        }
    }

    fn fit_primary(&self, window: &Window) {
        let screens = window.screens();
        let Some(screen) = screens.primary().or_else(|| screens.all().into_iter().next()) else {
            return;
        };
        let bounds = screen.bounds();
        let scale = screen.scaling().max(f64::EPSILON);
        let root_height = screens.all().iter().map(|screen| screen.bounds().bottom()).max().unwrap_or(bounds.bottom());
        let geometry = (bounds, scale, root_height);
        if self.screen_bounds.replace(Some(geometry)) == Some(geometry) {
            return;
        }
        window.set_position(bounds.position());
        window.set_width(f64::from(bounds.width) / scale);
        window.set_height(f64::from(bounds.height) / scale);
        self.update_strut();
    }

    /// Reserve the active theme's panel/dock in DIPs. Windowed previews never change the WM's work area.
    pub fn set_panel_insets(&self, top: f64, bottom: f64) {
        let insets = (top.max(0.0), bottom.max(0.0));
        if self.panel_insets.replace(insets) != insets {
            self.update_strut();
        }
    }

    fn update_strut(&self) {
        let Some((bounds, scale, root_height)) = self.screen_bounds.get() else {
            return;
        };
        let info = self.platform.info();
        let a = info.atoms();
        let strut = panel_strut(bounds, scale, root_height, self.panel_insets.get());
        xlib::x_change_property_longs(
            info.display(),
            self.desktop.get(),
            a._NET_WM_STRUT_PARTIAL,
            a.CARDINAL,
            PropertyMode::Replace,
            &strut,
        );
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.platform.globals().root_property_changed.unsubscribe(self.root_subscription.get());
        if let Some(token) = self.screens_subscription.take() {
            token.dispose();
        }
        for id in self.clients.take().into_keys() {
            self.unwatch(id);
        }
    }
}

fn workspace_names(count: usize, bytes: &[u8]) -> Vec<String> {
    let mut names = bytes.split(|&byte| byte == 0);
    (0..count.clamp(1, 256))
        .map(|i| {
            let name = names.next().map(String::from_utf8_lossy).unwrap_or_default();
            if name.trim().is_empty() {
                format!("Workspace {}", i + 1)
            } else {
                name.into_owned()
            }
        })
        .collect()
}

fn panel_strut(bounds: PixelRect, scale: f64, root_height: i32, (top, bottom): (f64, f64)) -> [XID; 12] {
    let mut result = [0; 12];
    // A strut extends from the virtual root edge. Reserving an interior monitor edge would
    // exclude valid window space on a monitor above/below the shell.
    if bounds.width <= 0 {
        return result;
    }
    let x_start = bounds.x.max(0) as XID;
    let x_end = (bounds.right() - 1).max(0) as XID;
    let height = f64::from(bounds.height.max(0));
    if bounds.y == 0 && top > 0.0 {
        result[2] = (top * scale).ceil().min(height) as XID;
        result[8] = x_start;
        result[9] = x_end;
    }
    if bounds.bottom() == root_height && bottom > 0.0 {
        result[3] = (bottom * scale).ceil().min(height - result[2] as f64) as XID;
        result[10] = x_start;
        result[11] = x_end;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workspace_names_follow_count_and_accept_partial_utf8_lists() {
        assert_eq!(workspace_names(3, b"Code\0\0"), ["Code", "Workspace 2", "Workspace 3"]);
        assert_eq!(workspace_names(0, b""), ["Workspace 1"]);
        assert_eq!(workspace_names(usize::MAX, b"A").len(), 256);
    }
    #[test]
    fn struts_use_physical_pixels_and_inclusive_root_coordinates() {
        let strut = panel_strut(PixelRect::new(1920, 0, 2560, 1440), 1.5, 1440, (34.0, 64.0));
        assert_eq!([strut[2], strut[8], strut[9]], [51, 1920, 4479]);
        assert_eq!([strut[3], strut[10], strut[11]], [96, 1920, 4479]);
        assert_eq!(panel_strut(PixelRect::new(0, 1080, 1280, 720), 1.0, 2160, (44.0, 52.0)), [0; 12]);
    }
}

#[cfg(test)]
mod integration;
