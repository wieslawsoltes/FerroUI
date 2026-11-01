//! Compositor-backed workspace and client management for Sway. The subscription
//! thread sleeps on the IPC socket, coalesces bursts, and reads the tree only
//! when its structure or geometry changed. The view always reads a local model.
#[path = "wayland_ipc.rs"]
mod ipc;
#[path = "wayland_tree.rs"]
mod tree;

use super::{SnapSide, Snapshot};
use ferroui_freedesktop::ui_thread_object::UiThreadHandle;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex};
use std::thread;
use tree::{Frame, Rect, Tree};

#[derive(Clone, Copy)]
pub struct ShortcutOrigin {
    window: u32,
    workspace: u64,
}

#[derive(Default)]
struct Update {
    tree: Option<Tree>,
    titles: HashMap<u32, String>,
    focus: Option<u32>,
    disconnected: bool,
}

#[derive(Default)]
struct Mailbox {
    update: Update,
    posted: bool,
}

struct Channel {
    path: PathBuf,
    pid: libc::pid_t,
    socket: Option<UnixStream>,
}

impl Channel {
    fn with<R>(&mut self, f: impl FnOnce(&mut UnixStream) -> io::Result<R>) -> io::Result<R> {
        if self.socket.is_none() {
            self.socket = Some(ipc::connect(&self.path, self.pid)?);
        }
        let result = f(self.socket.as_mut().unwrap());
        // A truncated or timed-out reply poisons the stream boundary. A subsequent
        // user request gets a new peer-verified connection; never replay a mutation.
        if result.is_err() {
            self.socket.take();
        }
        result
    }
}

#[derive(Clone, Copy)]
struct Maximized {
    before: Frame,
    target: Rect,
}

pub struct WaylandSession {
    tree: RefCell<Tree>,
    changed: RefCell<Option<Rc<dyn Fn()>>>,
    commands: RefCell<Channel>,
    cancel: UnixStream,
    registration: RefCell<Option<UiThreadHandle<Weak<Self>>>>,
    live: Cell<bool>,
    serial: Cell<u64>,
    // Maximizing a floating window is reversible; restoring a tiled one
    // hands layout back to Sway. Do not lose this distinction on tree updates.
    restore: RefCell<HashMap<u32, Maximized>>,
}

impl WaylandSession {
    pub fn connect() -> io::Result<Rc<Self>> {
        let (path, pid) = ipc::compositor_socket()?;
        let mut events = ipc::connect(&path, pid)?;
        let reply = ipc::request(
            &mut events,
            ipc::SUBSCRIBE,
            r#"["workspace","window","output","shutdown","tick"]"#,
        )?;
        if reply["success"] != true {
            return Err(ipc::invalid("Sway rejected the event subscription"));
        }
        events.set_read_timeout(None)?;
        // Subscribe before the first snapshot so a new window cannot be missed in between.
        let mut snapshots = ipc::connect(&path, pid)?;
        let tree = Tree::read(&mut snapshots)?;
        let this = Rc::new(Self {
            tree: RefCell::new(tree),
            commands: RefCell::new(Channel {
                path,
                pid,
                socket: None,
            }),
            cancel: events.try_clone()?,
            changed: RefCell::default(),
            registration: RefCell::default(),
            live: Cell::new(true),
            serial: Cell::new(0),
            restore: RefCell::default(),
        });
        let handle = UiThreadHandle::register(Rc::new(Rc::downgrade(&this)));
        this.registration.replace(Some(handle.clone()));
        thread::Builder::new()
            .name("ferro-sway-events".into())
            .spawn(move || {
                let mailbox = Arc::new(Mutex::new(Mailbox::default()));
                let result = pump(&mut events, &mut snapshots, |update| {
                    post(update, &mailbox, &handle)
                });
                if let Err(e) = result {
                    if !matches!(
                        e.kind(),
                        io::ErrorKind::UnexpectedEof | io::ErrorKind::ConnectionReset
                    ) {
                        eprintln!("Ferro Desktop compositor subscription: {e}");
                    }
                }
                post(
                    Update {
                        disconnected: true,
                        ..Default::default()
                    },
                    &mailbox,
                    &handle,
                );
            })?;
        Ok(this)
    }

    pub fn snapshot(&self) -> Snapshot {
        self.tree.borrow().snapshot.clone()
    }

    pub fn subscribe(&self, changed: impl Fn() + 'static) {
        self.changed.replace(Some(Rc::new(changed)));
    }

    pub fn focused_output(&self) -> Option<String> {
        let tree = self.tree.borrow();
        tree.spaces
            .get(tree.snapshot.current_workspace)
            .map(|w| w.output.clone())
    }

    fn apply(&self, update: Update) {
        let mut tree = self.tree.borrow_mut();
        let mut changed = false;
        if update.disconnected {
            self.live.set(false);
            changed = !tree.snapshot.windows.is_empty() || !tree.spaces.is_empty();
            *tree = Tree::default();
        } else {
            if let Some(mut fresh) = update.tree {
                let previous: HashMap<_, _> = tree
                    .snapshot
                    .windows
                    .iter()
                    .map(|w| (w.id, (w.last_activation, w.active)))
                    .collect();
                let mut restore = self.restore.borrow_mut();
                restore.retain(|id, _| fresh.frames.contains_key(id));
                for w in &mut fresh.snapshot.windows {
                    let (serial, was_active) = previous.get(&w.id).copied().unwrap_or_default();
                    w.last_activation = if w.active && !was_active {
                        self.next_serial()
                    } else {
                        serial
                    };
                    w.maximized = restore.get(&w.id).is_some_and(|saved| {
                        fresh
                            .frames
                            .get(&w.id)
                            .is_some_and(|frame| frame.floating && frame.rect == saved.target)
                    });
                }
                changed = tree.snapshot != fresh.snapshot;
                *tree = fresh;
            }
            if !update.titles.is_empty() || update.focus.is_some() {
                for w in &mut tree.snapshot.windows {
                    if let Some(title) = update.titles.get(&w.id) {
                        if w.title != *title {
                            w.title.clone_from(title);
                            changed = true;
                        }
                    }
                    if let Some(id) = update.focus {
                        if w.active != (w.id == id) {
                            w.active = w.id == id;
                            if w.active {
                                w.last_activation = self.next_serial();
                            }
                            changed = true;
                        }
                    }
                }
            }
        }
        drop(tree);
        if changed {
            if let Some(cb) = self.changed.borrow().clone() {
                cb();
            }
        }
    }

    fn next_serial(&self) -> u64 {
        let serial = self.serial.get().saturating_add(1);
        self.serial.set(serial);
        serial
    }

    fn command(&self, command: String) -> bool {
        if !self.live.get() {
            return false;
        }
        let reply = self
            .commands
            .borrow_mut()
            .with(|s| ipc::request(s, ipc::RUN, &command));
        match reply {
            Ok(value) => value.as_array().is_some_and(|items| {
                !items.is_empty() && items.iter().all(|item| item["success"] == true)
            }),
            Err(e) => {
                eprintln!("Ferro Desktop compositor request: {e}");
                false
            }
        }
    }

    /// Change only Sway's server-side frames. GTK and Qt may draw their own titlebars.
    pub fn set_window_appearance(
        &self,
        appearance: crate::shell::appearance::Appearance,
    ) -> io::Result<()> {
        let p = appearance.palette();
        let rgba = |value: &str| -> io::Result<String> {
            let c = ferroui_base::media::Color::try_parse(value)
                .ok_or_else(|| io::Error::other("Invalid window decoration color"))?;
            Ok(format!("#{:02x}{:02x}{:02x}{:02x}", c.r, c.g, c.b, c.a))
        };
        let (accent, surface, text) = (rgba(p.accent)?, rgba(p.surface)?, rgba(p.text)?);
        let (edge, tile, muted) = (rgba(p.edge)?, rgba(p.tile)?, rgba(p.muted)?);
        let colors = format!(
            "client.focused {accent} {surface} {text} {accent} {accent}; \
             client.focused_inactive {edge} {tile} {muted} {edge} {edge}; \
             client.unfocused {edge} {tile} {muted} {edge} {edge}"
        );
        if self.command(colors) {
            Ok(())
        } else {
            Err(io::Error::other(
                "Sway rejected the window decoration colors",
            ))
        }
    }

    fn refresh_after_geometry(&self) {
        // Floating moves/resizes do not emit a window event on every Sway
        // version. A tick orders a background snapshot after our commands.
        let _ = self
            .commands
            .borrow_mut()
            .with(|s| ipc::request(s, 10, "ferro-desktop:geometry"));
    }

    fn target(&self, id: u32, action: &str) -> bool {
        self.tree.borrow().frames.contains_key(&id)
            && self.command(format!("[con_id={id}] {action}"))
    }

    pub fn activate(&self, id: u32) -> bool {
        let minimized = self
            .tree
            .borrow()
            .snapshot
            .windows
            .iter()
            .any(|w| w.id == id && w.minimized);
        self.target(
            id,
            if minimized {
                "scratchpad show, focus"
            } else {
                "focus"
            },
        )
    }
    pub fn close(&self, id: u32) -> bool {
        self.target(id, "kill")
    }
    pub fn minimize(&self, id: u32) -> bool {
        self.target(id, "move scratchpad")
    }
    pub fn toggle_fullscreen(&self, id: u32) -> bool {
        self.target(id, "fullscreen toggle")
    }
    pub fn toggle_above(&self, _id: u32) -> bool {
        false
    } // Sway has no always-on-top state.

    fn frame_and_area(&self, id: u32) -> Option<(Frame, Rect)> {
        // Geometry of floating windows may change without an IPC event on some
        // Sway versions. Refresh on the explicit geometry operation, never on paint.
        let tree = self.commands.borrow_mut().with(Tree::read).ok()?;
        if !tree
            .snapshot
            .windows
            .iter()
            .any(|w| w.id == id && w.can_snap)
        {
            return None;
        }
        let frame = *tree.frames.get(&id)?;
        let target = tree.spaces.get(frame.workspace?)?.area;
        (target.valid() && frame.rect.valid()).then_some((frame, target))
    }

    fn place(&self, id: u32, area: Rect) -> bool {
        area.valid() && self.target(id, &format!(
            "floating enable, resize set width {} px height {} px, move absolute position {} px {} px",
            area.width, area.height, area.x, area.y))
    }

    pub fn snap(&self, id: u32, side: SnapSide) -> bool {
        let Some((_, area)) = self.frame_and_area(id) else {
            return false;
        };
        if self.place(id, area.half(side == SnapSide::Right)) {
            self.restore.borrow_mut().remove(&id);
            self.refresh_after_geometry();
            true
        } else {
            false
        }
    }

    pub fn toggle_maximized(&self, id: u32) -> bool {
        let Some((frame, area)) = self.frame_and_area(id) else {
            return false;
        };
        let previous = self.restore.borrow().get(&id).copied();
        if let Some(saved) = previous.filter(|saved| frame.floating && frame.rect == saved.target) {
            let old = saved.before;
            let restored = if old.floating {
                self.place(id, old.rect)
            } else {
                self.target(id, "floating disable")
            };
            if restored {
                self.restore.borrow_mut().remove(&id);
                self.refresh_after_geometry();
            }
            return restored;
        }
        if self.place(id, area) {
            self.restore.borrow_mut().insert(
                id,
                Maximized {
                    before: frame,
                    target: area,
                },
            );
            self.refresh_after_geometry();
            true
        } else {
            false
        }
    }

    pub fn switch_workspace(&self, index: usize) -> bool {
        let tree = self.tree.borrow();
        tree.spaces
            .get(index)
            .and_then(|w| w.target(&tree.spaces))
            .is_some_and(|target| {
                self.command(format!("workspace --no-auto-back-and-forth {target}"))
            })
    }

    pub fn move_to_workspace(&self, id: u32, index: usize) -> bool {
        let tree = self.tree.borrow();
        tree.spaces
            .get(index)
            .and_then(|w| w.target(&tree.spaces))
            .is_some_and(|target| {
                self.target(
                    id,
                    &format!("move --no-auto-back-and-forth container to workspace {target}"),
                )
            })
    }

    pub fn add_workspace(&self) -> bool {
        let tree = self.tree.borrow();
        let number = (1..=256).find(|n| tree.spaces.iter().all(|w| w.number != *n));
        tree.spaces.len() < 256
            && number.is_some_and(|n| {
                self.command(format!("workspace --no-auto-back-and-forth number {n}"))
            })
    }

    pub fn can_remove_last(&self) -> bool {
        let tree = self.tree.borrow();
        let Some(last) = tree.spaces.len().checked_sub(1) else {
            return false;
        };
        last > 0
            && tree.snapshot.current_workspace == last
            && !tree.frames.values().any(|f| f.workspace == Some(last))
            && tree.spaces[..last]
                .iter()
                .any(|w| w.output == tree.spaces[last].output)
    }

    pub fn remove_empty_last_workspace(&self) -> bool {
        if !self.can_remove_last() {
            return false;
        }
        // Sway retires empty workspaces when focus leaves. It never deletes a
        // workspace containing a client, including a client mapped after our snapshot.
        let tree = self.tree.borrow();
        let last = tree.spaces.last().unwrap();
        tree.spaces
            .iter()
            .rposition(|w| w.id != last.id && w.output == last.output)
            .is_some_and(|index| self.switch_workspace(index))
    }

    pub fn shortcut_origin(&self) -> Option<ShortcutOrigin> {
        let tree = self.commands.borrow_mut().with(Tree::read).ok()?;
        let active = tree.snapshot.windows.iter().find(|w| w.active)?;
        Some(ShortcutOrigin {
            workspace: tree.spaces.get(tree.snapshot.current_workspace)?.id,
            window: active.id,
        })
    }

    pub fn restore_shortcut_origin(&self, origin: ShortcutOrigin) {
        let Ok(tree) = self.commands.borrow_mut().with(Tree::read) else {
            return;
        };
        if let Some(frame) = tree.frames.get(&origin.window) {
            if frame
                .workspace
                .and_then(|i| tree.spaces.get(i))
                .is_some_and(|w| w.id == origin.workspace)
            {
                self.activate(origin.window);
            }
        }
    }
}

impl Drop for WaylandSession {
    fn drop(&mut self) {
        if let Some(handle) = self.registration.take() {
            handle.unregister();
        }
        let _ = self.cancel.shutdown(Shutdown::Both);
    }
}

fn post(
    update: Update,
    mailbox: &Arc<Mutex<Mailbox>>,
    handle: &UiThreadHandle<Weak<WaylandSession>>,
) {
    let mut queued = mailbox.lock().unwrap_or_else(|e| e.into_inner());
    if update.tree.is_some() || update.disconnected {
        queued.update = update;
    } else {
        queued.update.titles.extend(update.titles);
        if update.focus.is_some() {
            queued.update.focus = update.focus;
        }
    }
    if queued.posted {
        return;
    }
    queued.posted = true;
    drop(queued);
    let mailbox = mailbox.clone();
    drop(handle.call(move |weak| {
        let mut pending = mailbox.lock().unwrap_or_else(|e| e.into_inner());
        let update = std::mem::take(&mut pending.update);
        pending.posted = false;
        drop(pending);
        if let Some(session) = weak.upgrade() {
            session.apply(update);
        }
    }));
}

fn pump(
    events: &mut UnixStream,
    snapshots: &mut UnixStream,
    mut deliver: impl FnMut(Update),
) -> io::Result<()> {
    loop {
        let mut update = Update::default();
        let mut reload = false;
        // Bound the drain to prevent a flooding application from delaying unrelated events.
        for _ in 0..128 {
            let (kind, data) = ipc::receive(&mut *events)?;
            match kind {
                k if k == ipc::EVENT | 6 => return Ok(()),
                k if k == ipc::EVENT | 7 => reload |= data["payload"] == "ferro-desktop:geometry",
                k if k == ipc::EVENT | 3 => {
                    let container = &data["container"];
                    let id = container["id"]
                        .as_u64()
                        .and_then(|id| u32::try_from(id).ok());
                    match (data["change"].as_str(), id) {
                        (Some("title"), Some(id)) => {
                            update.titles.insert(
                                id,
                                container["name"].as_str().unwrap_or("Window").to_owned(),
                            );
                        }
                        (Some("focus"), Some(id)) => update.focus = Some(id),
                        _ => reload = true,
                    }
                }
                _ => reload = true,
            }
            if !ipc::ready(events) {
                break;
            }
        }
        if reload {
            update.tree = Some(Tree::read(snapshots)?);
            update.titles.clear();
            update.focus = None;
        }
        deliver(update);
    }
}

#[cfg(test)]
#[path = "wayland_tests.rs"]
mod tests;
