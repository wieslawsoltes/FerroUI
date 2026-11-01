use super::{buffers, ClientWindow, Input, Pixels, Scene, Surface};
use smithay::{
    delegate_compositor, delegate_output, delegate_shm, delegate_xdg_shell, delegate_xdg_decoration,
    desktop::{PopupKind, PopupManager},
    input::{Seat, SeatState},
    output::{Mode, Output, PhysicalProperties, Scale, Subpixel},
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::{
            backend::{ClientData as WaylandClientData, ClientId, DisconnectReason},
            protocol::{
                wl_buffer::WlBuffer, wl_callback::WlCallback, wl_output::WlOutput, wl_seat::WlSeat,
                wl_surface::WlSurface,
            },
            Client, DisplayHandle, Resource,
        },
    },
    utils::{Logical, Point, Rectangle, Serial, Transform, SERIAL_COUNTER},
    wayland::{
        buffer::BufferHandler,
        compositor::{
            with_states, with_surface_tree_upward, BufferAssignment, CompositorClientState,
            CompositorHandler, CompositorState, SubsurfaceCachedState, SurfaceAttributes,
            TraversalAction,
        },
        output::{OutputHandler, OutputManagerState},
        selection::data_device::DataDeviceState,
        shell::xdg::{
            PopupSurface, PositionerState, SurfaceCachedState, ToplevelSurface, XdgShellHandler,
            XdgShellState, XdgToplevelSurfaceData,
        },
        shm::{ShmHandler, ShmState},
    },
};
use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode as DecorationMode;
use smithay::wayland::shell::xdg::decoration::{XdgDecorationState, XdgDecorationHandler};
use std::{
    collections::HashMap,
    io,
    sync::Arc,
    time::Instant,
};

#[derive(Default)]
pub(super) struct ClientData {
    compositor: CompositorClientState,
}
impl WaylandClientData for ClientData {
    fn initialized(&self, _: ClientId) {}
    fn disconnected(&self, _: ClientId, _: DisconnectReason) {}
}

struct Image {
    id: u64,
    source: Arc<Pixels>,
    shown: Arc<Pixels>,
    transform: Transform,
    scale: i32,
}

pub(super) struct Window {
    pub role: ToplevelSurface,
    pub id: u64,
    pub location: Point<i32, Logical>,
    saved: Option<Rectangle<i32, Logical>>,
    pub hidden: bool,
    pub fullscreen: bool,
    pub maximized: bool,
    pub above: bool,
    pub workspace: usize,
}

pub(super) struct Hit {
    pub wl: WlSurface,
    pub view: Surface,
}

struct PendingFrame {
    scene: u64,
    surface: WlSurface,
    callbacks: Vec<WlCallback>,
}

pub(super) struct State {
    pub display: DisplayHandle,
    pub running: bool,
    compositor: CompositorState,
    shm: ShmState,
    shell: XdgShellState,
    pub(super) seats: SeatState<Self>,
    pub(super) seat: Seat<Self>,
    pub(super) selection: DataDeviceState,
    output: Output,
    mode: Mode,
    work_area: (i32, i32),
    current_workspace: usize,
    workspace_count: usize,
    pub(super) popups: PopupManager,
    images: HashMap<WlSurface, Image>,
    pub(super) windows: Vec<Window>,
    pub(super) hits: Vec<Hit>,
    pub(super) focused: Option<u64>,
    pub(super) pointer_at: Option<Point<f64, Logical>>,
    pub(super) drag: Option<super::seat::Drag>,
    pending_frames: Vec<PendingFrame>,
    visible_since: HashMap<WlSurface, u64>,
    pub(super) dirty: bool,
    serial: u64,
    next: u64,
    start: Instant,
}

impl State {
    pub fn new(dh: &DisplayHandle, width: i32, height: i32) -> io::Result<Self> {
        let mut seats = SeatState::new();
        let mut seat = seats.new_wl_seat(dh, "FerroUI");
        seat.add_keyboard(Default::default(), 400, 25)
            .map_err(io::Error::other)?;
        seat.add_pointer();
        let output = Output::new(
            "FERRO-1".into(),
            PhysicalProperties {
                size: (0, 0).into(),
                subpixel: Subpixel::Unknown,
                make: "FerroUI".into(),
                model: "Desktop output".into(),
            },
        );
        output.create_global::<Self>(dh);
        let _manager = OutputManagerState::new_with_xdg_output::<Self>(dh);
        XdgDecorationState::new::<Self>(dh);
        let mode = Mode {
            size: (width, height).into(),
            refresh: 60_000,
        };
        output.change_current_state(
            Some(mode),
            Some(Transform::Normal),
            Some(Scale::Integer(1)),
            Some((0, 0).into()),
        );
        output.set_preferred(mode);
        Ok(Self {
            display: dh.clone(),
            running: true,
            compositor: CompositorState::new::<Self>(dh),
            shm: ShmState::new::<Self>(dh, vec![]),
            shell: XdgShellState::new::<Self>(dh),
            seats,
            seat,
            selection: DataDeviceState::new::<Self>(dh),
            output,
            mode,
            work_area: (0, 0),
            current_workspace: 0,
            workspace_count: super::INITIAL_WORKSPACES,
            popups: PopupManager::default(),
            images: HashMap::new(),
            windows: vec![],
            hits: vec![],
            focused: None,
            pointer_at: None,
            drag: None,
            pending_frames: Vec::new(),
            visible_since: HashMap::new(),
            dirty: false,
            serial: 0,
            next: 1,
            start: Instant::now(),
        })
    }

    fn id(&mut self) -> u64 {
        let id = self.next;
        self.next += 1;
        id
    }

    pub(super) fn geometry(&self, surface: &WlSurface) -> Rectangle<i32, Logical> {
        let declared = with_states(surface, |s| {
            s.cached_state
                .get::<SurfaceCachedState>()
                .current()
                .geometry
        });
        declared.unwrap_or_else(|| {
            let dims = self.images.get(surface).map_or((1, 1), |im| {
                (im.shown.width / im.scale, im.shown.height / im.scale)
            });
            Rectangle::from_size(dims.into())
        })
    }
    fn tree(
        &self,
        root: &WlSurface,
        window: u64,
        origin: Point<i32, Logical>,
        hits: &mut Vec<Hit>,
        frames: &mut Vec<PendingFrame>,
    ) {
        let position = |surface: &WlSurface,
                        states: &smithay::wayland::compositor::SurfaceData,
                        base: Point<i32, Logical>| {
            if surface == root {
                base
            } else {
                base + states
                    .cached_state
                    .get::<SubsurfaceCachedState>()
                    .current()
                    .location
            }
        };
        with_surface_tree_upward(
            root,
            origin,
            |surface, s, base| {
                if self.images.contains_key(surface) {
                    TraversalAction::DoChildren(position(surface, s, *base))
                } else {
                    TraversalAction::SkipChildren
                }
            },
            |surface, s, base| {
                let Some(im) = self.images.get(surface) else {
                    return;
                };
                let loc = position(surface, s, *base);
                let view = Surface {
                    id: im.id,
                    window,
                    x: loc.x,
                    y: loc.y,
                    width: im.shown.width / im.scale,
                    height: im.shown.height / im.scale,
                    pixels: im.shown.clone(),
                };
                let callbacks = std::mem::take(
                    &mut s
                        .cached_state
                        .get::<SurfaceAttributes>()
                        .current()
                        .frame_callbacks,
                );
                if !callbacks.is_empty() {
                    frames.push(PendingFrame {
                        scene: self.serial + 1,
                        surface: surface.clone(),
                        callbacks,
                    });
                }
                hits.push(Hit {
                    wl: surface.clone(),
                    view,
                });
            },
            |_, _, _| true,
        );
    }

    pub fn take_scene(&mut self) -> Option<Scene> {
        if !std::mem::take(&mut self.dirty) {
            return None;
        }
        self.popups.cleanup();
        let mut hits = vec![];
        let mut frames = vec![];
        let mut windows = vec![];
        for win in &self.windows {
            let surface = win.role.wl_surface();
            if !self.images.contains_key(surface) {
                continue;
            }
            let geometry = self.geometry(surface);
            let (title, app_id) = with_states(surface, |s| {
                let attrs = s
                    .data_map
                    .get::<XdgToplevelSurfaceData>()
                    .unwrap()
                    .lock()
                    .unwrap();
                (
                    attrs.title.clone().unwrap_or_default(),
                    attrs.app_id.clone().unwrap_or_default(),
                )
            });
            windows.push(ClientWindow {
                id: win.id,
                title,
                app_id,
                x: win.location.x,
                y: win.location.y,
                width: geometry.size.w,
                height: geometry.size.h,
                focused: self.focused == Some(win.id),
                minimized: win.hidden,
                maximized: win.maximized,
                fullscreen: win.fullscreen,
                workspace: win.workspace,
                above: win.above,
            });
            if win.hidden || win.workspace != self.current_workspace {
                continue;
            }
            self.tree(
                surface,
                win.id,
                win.location - geometry.loc,
                &mut hits,
                &mut frames,
            );
            let popups: Vec<_> = PopupManager::popups_for_surface(surface).collect();
            for (popup, offset) in popups.into_iter().rev() {
                self.tree(
                    popup.wl_surface(),
                    win.id,
                    win.location + offset - popup.geometry().loc,
                    &mut hits,
                    &mut frames,
                );
            }
        }
        self.serial += 1;
        self.pending_frames.extend(frames);
        let surfaces = hits.iter().map(|h| h.view.clone()).collect();
        let visible: std::collections::HashSet<_> = hits.iter().map(|h| &h.wl).collect();
        self.visible_since.retain(|surface, _| {
            if visible.contains(surface) {
                true
            } else {
                self.output.leave(surface);
                false
            }
        });
        for hit in &hits {
            self.visible_since.entry(hit.wl.clone()).or_insert_with(|| {
                self.output.enter(&hit.wl);
                self.serial
            });
        }
        self.hits = hits;
        Some(Scene {
            serial: self.serial,
            surfaces,
            windows,
            workspace_count: self.workspace_count,
            current_workspace: self.current_workspace,
        })
    }

    pub(super) fn area(&self) -> Rectangle<i32, Logical> {
        let (top, bottom) = self.work_area;
        Rectangle::new(
            (0, top).into(),
            (self.mode.size.w, (self.mode.size.h - top - bottom).max(1)).into(),
        )
    }

    fn refit(&mut self) {
        let area = self.area();
        for win in self
            .windows
            .iter_mut()
            .filter(|w| w.maximized || w.fullscreen)
        {
            let rect = if win.fullscreen {
                Rectangle::from_size((self.mode.size.w, self.mode.size.h).into())
            } else {
                area
            };
            win.location = rect.loc;
            win.role.with_pending_state(|s| s.size = Some(rect.size));
            win.role.send_pending_configure();
        }
        self.dirty = true;
    }

    fn maximize(&mut self, id: u64, value: bool, fullscreen: bool) {
        let area = self.area();
        let bounds = Rectangle::from_size((self.mode.size.w, self.mode.size.h).into());
        let Some(idx) = self.windows.iter().position(|w| w.id == id) else {
            return;
        };
        let geometry = self
            .images
            .contains_key(self.windows[idx].role.wl_surface())
            .then(|| self.geometry(self.windows[idx].role.wl_surface()));
        let w = &mut self.windows[idx];
        if value && !w.maximized && !w.fullscreen {
            w.saved = geometry.map(|g| Rectangle::new(w.location, g.size));
        }
        if fullscreen {
            w.fullscreen = value;
        } else {
            w.maximized = value;
        }
        let frame = if w.fullscreen {
            Some(bounds)
        } else if w.maximized {
            Some(area)
        } else {
            w.saved.take()
        };
        if let Some(frame) = frame {
            w.location = frame.loc;
        }
        w.role.with_pending_state(|s| {
            s.size = frame.map(|r| r.size);
            s.states.unset(xdg_toplevel::State::Maximized);
            s.states.unset(xdg_toplevel::State::Fullscreen);
            if w.maximized {
                s.states.set(xdg_toplevel::State::Maximized);
            }
            if w.fullscreen {
                s.states.set(xdg_toplevel::State::Fullscreen);
            }
        });
        if w.role.is_initial_configure_sent() {
            w.role.send_pending_configure();
        }
        self.dirty = true;
    }

    pub(super) fn minimize(&mut self, id: u64) {
        if let Some(w) = self.windows.iter_mut().find(|w| w.id == id) {
            w.hidden = true;
            self.dirty = true;
        }
        self.hits.retain(|hit| hit.view.window != id);
        if self.focused == Some(id) {
            self.focus_last();
        }
    }

    fn focus_last(&mut self) {
        let top = self
            .windows
            .iter()
            .rev()
            .find(|w| {
                w.workspace == self.current_workspace
                    && !w.hidden
                    && self.images.contains_key(w.role.wl_surface())
            })
            .map(|w| w.id);
        self.focus(top);
    }

    fn raise(&mut self, id: u64) {
        let Some(idx) = self.windows.iter().position(|w| w.id == id) else {
            return;
        };
        let win = self.windows.remove(idx);
        let position = if win.above {
            self.windows.len()
        } else {
            self.windows.partition_point(|w| !w.above)
        };
        self.windows.insert(position, win);
        if idx != position {
            let order: HashMap<_, _> = self
                .windows
                .iter()
                .enumerate()
                .map(|(i, w)| (w.id, i))
                .collect();
            self.hits
                .sort_by_key(|h| order.get(&h.view.window).copied());
        }
    }

    pub(super) fn focus(&mut self, id: Option<u64>) {
        if self.focused != id {
            // A popup grab belongs to its toplevel and must not continue
            // capturing keys after switching desktop or minimizing that window.
            if let Some(root) = self
                .windows
                .iter()
                .find(|w| Some(w.id) == self.focused)
                .map(|w| w.role.wl_surface().clone())
            {
                let popups: Vec<_> = PopupManager::popups_for_surface(&root)
                    .map(|(p, _)| p)
                    .collect();
                if !popups.is_empty() {
                    for popup in popups {
                        let _ = PopupManager::dismiss_popup(&root, &popup);
                    }
                    if let Some(kbd) = self.seat.get_keyboard() {
                        kbd.unset_grab(self);
                    }
                    if let Some(ptr) = self.seat.get_pointer() {
                        ptr.unset_grab(
                            self,
                            SERIAL_COUNTER.next_serial(),
                            self.start.elapsed().as_millis() as u32,
                        );
                    }
                }
            }
        }
        if let Some(id) = id {
            let Some(w) = self.windows.iter_mut().find(|w| w.id == id) else {
                return;
            };
            w.hidden = false;
            if self.current_workspace != w.workspace {
                self.hits.clear();
            }
            self.current_workspace = w.workspace;
            self.raise(id);
        }
        self.focused = id;
        for w in &self.windows {
            w.role.with_pending_state(|s| {
                if Some(w.id) == id {
                    s.states.set(xdg_toplevel::State::Activated);
                } else {
                    s.states.unset(xdg_toplevel::State::Activated);
                }
            });
            if w.role.is_initial_configure_sent() {
                w.role.send_pending_configure();
            }
        }
        if let Some(keyboard) = self.seat.get_keyboard() {
            let surface = id.and_then(|id| {
                self.windows
                    .iter()
                    .find(|w| w.id == id)
                    .map(|w| w.role.wl_surface().clone())
            });
            keyboard.set_focus(self, surface, SERIAL_COUNTER.next_serial());
        }
        self.dirty = true;
    }

    fn switch_workspace(&mut self, workspace: usize) {
        if workspace >= self.workspace_count || self.current_workspace == workspace {
            return;
        }
        self.current_workspace = workspace;
        self.drag = None;
        self.hits.clear();
        self.focus_last();
    }

    pub fn input(&mut self, event: Input) {
        match event {
            Input::Presented(serial) => {
                let now = self.start.elapsed().as_millis() as u32;
                self.pending_frames.retain_mut(|batch| {
                    if !batch.surface.is_alive() {
                        return false;
                    }
                    // Several commits can coalesce into one composed frame. A
                    // hidden surface (or one just revealed in a newer scene)
                    // must not receive credit for an unrelated presentation.
                    if batch.scene <= serial
                        && self
                            .visible_since
                            .get(&batch.surface)
                            .is_some_and(|since| *since <= serial)
                    {
                        for callback in batch.callbacks.drain(..) {
                            callback.done(now);
                        }
                        false
                    } else {
                        true
                    }
                });
            }
            Input::Resize { width, height }
                if width > 0 && height > 0 && self.mode.size != (width, height).into() =>
            {
                let old = self.mode;
                self.mode.size = (width, height).into();
                self.output
                    .change_current_state(Some(self.mode), None, None, None);
                self.output.set_preferred(self.mode);
                self.output.delete_mode(old);
                self.refit();
            }
            Input::Resize { .. } => {}
            Input::WorkArea { top, bottom } => {
                if self.work_area != (top, bottom) {
                    self.work_area = (top.max(0), bottom.max(0));
                    self.refit();
                }
            }
            Input::Activate(id) => self.focus(Some(id)),
            Input::Minimize(id) => self.minimize(id),
            Input::SwitchWorkspace(index) => self.switch_workspace(index),
            Input::AddWorkspace => {
                self.workspace_count += 1;
                self.switch_workspace(self.workspace_count - 1);
            }
            Input::RemoveWorkspace => {
                let last = self.workspace_count - 1;
                if last > 0 && !self.windows.iter().any(|w| w.workspace == last) {
                    if self.current_workspace == last {
                        self.switch_workspace(last - 1);
                    }
                    self.workspace_count -= 1;
                    self.dirty = true;
                }
            }
            Input::MoveToWorkspace { id, workspace } => {
                if workspace < self.workspace_count {
                    if let Some(w) = self.windows.iter_mut().find(|w| w.id == id) {
                        w.workspace = workspace;
                        if workspace != self.current_workspace {
                            self.hits.retain(|h| h.view.window != id);
                            if self.focused == Some(id) {
                                self.focus_last();
                            }
                        }
                        self.dirty = true;
                    }
                }
            }
            Input::Fullscreen(id) => {
                if let Some(w) = self.windows.iter().find(|w| w.id == id) {
                    self.maximize(id, !w.fullscreen, true);
                }
            }
            Input::AlwaysOnTop(id) => {
                if let Some(w) = self.windows.iter_mut().find(|w| w.id == id) {
                    w.above = !w.above;
                    self.raise(id);
                    self.dirty = true;
                }
            }
            Input::Maximize(id) => {
                if let Some(w) = self.windows.iter().find(|w| w.id == id) {
                    self.maximize(id, !w.maximized, false)
                }
            }
            Input::Close(id) => {
                if let Some(w) = self.windows.iter().find(|w| w.id == id) {
                    w.role.send_close();
                }
            }
            Input::Move { id, x, y } => {
                if let Some(w) = self.windows.iter_mut().find(|w| w.id == id) {
                    w.location = (x, y).into();
                    self.dirty = true;
                }
            }
            Input::Configure { id, width, height } => {
                if let Some(w) = self.windows.iter().find(|w| w.id == id) {
                    w.role.with_pending_state(|s| {
                        s.size = Some((width.max(1), height.max(1)).into())
                    });
                    w.role.send_pending_configure();
                }
            }
            _ => self.seat_event(event),
        }
    }
}

impl BufferHandler for State {
    fn buffer_destroyed(&mut self, _: &WlBuffer) {}
}
impl OutputHandler for State {}
impl ShmHandler for State {
    fn shm_state(&self) -> &ShmState {
        &self.shm
    }
}
impl CompositorHandler for State {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor
    }
    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client
            .get_data::<ClientData>()
            .expect("server-created client")
            .compositor
    }
    fn commit(&mut self, surface: &WlSurface) {
        self.popups.commit(surface);
        let (assignment, scale, transform) = with_states(surface, |s| {
            let mut cached = s.cached_state.get::<SurfaceAttributes>();
            let attrs = cached.current();
            attrs.damage.clear();
            attrs.buffer_delta.take();
            (
                attrs.buffer.take(),
                attrs.buffer_scale.max(1),
                Transform::from(attrs.buffer_transform),
            )
        });
        let previously_mapped = self.images.contains_key(surface);
        match assignment {
            Some(BufferAssignment::NewBuffer(buffer)) => {
                let revision = self.id();
                let source = buffers::read(&buffer, revision);
                buffer.release();
                if let Some(source) = source {
                    let id = self.images.get(surface).map_or(revision, |s| s.id);
                    let shown = buffers::oriented(&source, transform, revision);
                    self.images.insert(
                        surface.clone(),
                        Image {
                            id,
                            source,
                            shown,
                            transform,
                            scale,
                        },
                    );
                }
            }
            Some(BufferAssignment::Removed) => {
                self.images.remove(surface);
            }
            None => {
                let revision = self.id();
                if let Some(im) = self.images.get_mut(surface) {
                    im.scale = scale;
                    if im.transform != transform {
                        im.shown = buffers::oriented(&im.source, transform, revision);
                        im.transform = transform;
                    }
                }
            }
        }
        if let Some(w) = self.windows.iter().find(|w| w.role.wl_surface() == surface) {
            let mapped = self.images.contains_key(surface);
            let id = w.id;
            if !mapped && previously_mapped {
                w.role.reset_initial_configure_sent();
                if self.focused == Some(id) {
                    self.focus_last();
                }
            } else if mapped {
                if !w.role.ensure_configured() {
                    self.images.remove(surface);
                } else if !previously_mapped && w.workspace == self.current_workspace {
                    self.focus(Some(id));
                }
            } else if !w.role.is_initial_configure_sent() {
                w.role.send_configure();
            }
        } else if let Some(popup) = self
            .shell
            .popup_surfaces()
            .iter()
            .find(|p| p.wl_surface() == surface)
        {
            let mapped = self.images.contains_key(surface);
            if previously_mapped && !mapped {
                popup.reset_initial_configure_sent();
            } else if mapped {
                if !popup.ensure_configured() {
                    self.images.remove(surface);
                }
            } else if !popup.is_initial_configure_sent() {
                let _ = popup.send_configure();
            }
        }
        // Idle cursor/drag-icon clients do not invalidate the output. Their
        // buffers still have the same release and lifetime guarantees.
        if matches!(
            smithay::wayland::compositor::get_role(surface),
            Some(
                smithay::wayland::shell::xdg::XDG_TOPLEVEL_ROLE
                    | smithay::wayland::shell::xdg::XDG_POPUP_ROLE
                    | smithay::wayland::compositor::SUBSURFACE_ROLE
            )
        ) {
            self.dirty = true;
        }
    }
    fn destroyed(&mut self, surface: &WlSurface) {
        self.images.remove(surface);
        self.pending_frames.retain(|f| f.surface != *surface);
        self.dirty = true;
    }
}

impl XdgShellHandler for State {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.shell
    }
    fn new_toplevel(&mut self, role: ToplevelSurface) {
        let id = self.id();
        let n = (self.windows.len() % 7) as i32;
        let area = self.area();
        role.with_pending_state(|s| s.bounds = Some(area.size));
        let workspace = role
            .parent()
            .and_then(|surface| {
                self.windows
                    .iter()
                    .find(|w| *w.role.wl_surface() == surface)
            })
            .map_or(self.current_workspace, |w| w.workspace);
        let index = self.windows.partition_point(|w| !w.above);
        self.windows.insert(
            index,
            Window {
                role,
                id,
                location: (32 + 26 * n, area.loc.y + 36 + 26 * n).into(),
                saved: None,
                hidden: false,
                fullscreen: false,
                maximized: false,
                above: false,
                workspace,
            },
        );
    }
    fn toplevel_destroyed(&mut self, role: ToplevelSurface) {
        if let Some(i) = self.windows.iter().position(|w| w.role == role) {
            let w = self.windows.remove(i);
            if self.focused == Some(w.id) {
                self.focus_last();
            }
            self.dirty = true;
        }
    }
    fn title_changed(&mut self, _: ToplevelSurface) {
        self.dirty = true;
    }
    fn app_id_changed(&mut self, _: ToplevelSurface) {
        self.dirty = true;
    }
    fn new_popup(&mut self, role: PopupSurface, positioner: PositionerState) {
        self.position_popup(&role, positioner);
        let _ = self.popups.track_popup(PopupKind::Xdg(role));
    }
    fn popup_destroyed(&mut self, _: PopupSurface) {
        self.dirty = true;
    }
    fn reposition_request(&mut self, role: PopupSurface, positioner: PositionerState, token: u32) {
        self.position_popup(&role, positioner);
        role.send_repositioned(token);
        self.dirty = true;
    }
    fn grab(&mut self, role: PopupSurface, seat: WlSeat, serial: Serial) {
        self.grab_popup(role, seat, serial);
    }
    fn move_request(&mut self, role: ToplevelSurface, seat: WlSeat, serial: Serial) {
        self.begin_drag(role, seat, serial, None);
    }
    fn resize_request(
        &mut self,
        role: ToplevelSurface,
        seat: WlSeat,
        serial: Serial,
        edges: xdg_toplevel::ResizeEdge,
    ) {
        self.begin_drag(role, seat, serial, Some(edges));
    }
    fn maximize_request(&mut self, role: ToplevelSurface) {
        if let Some(w) = self.windows.iter().find(|w| w.role == role) {
            self.maximize(w.id, true, false)
        }
    }
    fn unmaximize_request(&mut self, role: ToplevelSurface) {
        if let Some(w) = self.windows.iter().find(|w| w.role == role) {
            self.maximize(w.id, false, false)
        }
    }
    fn fullscreen_request(&mut self, role: ToplevelSurface, _: Option<WlOutput>) {
        if let Some(w) = self.windows.iter().find(|w| w.role == role) {
            self.maximize(w.id, true, true)
        }
    }
    fn unfullscreen_request(&mut self, role: ToplevelSurface) {
        if let Some(w) = self.windows.iter().find(|w| w.role == role) {
            self.maximize(w.id, false, true)
        }
    }
    fn minimize_request(&mut self, role: ToplevelSurface) {
        if let Some(w) = self.windows.iter().find(|w| w.role == role) {
            self.minimize(w.id)
        }
    }
}
impl State {
    fn position_popup(&self, popup: &PopupSurface, positioner: PositionerState) {
        let parent_location = popup
            .get_parent_surface()
            .and_then(|wl| self.hits.iter().find(|h| h.wl == wl))
            .map(|h| Point::from((h.view.x, h.view.y)) + self.geometry(&h.wl).loc)
            .unwrap_or_default();
        let mut bounds = self.area();
        bounds.loc -= parent_location;
        let geometry = positioner.get_unconstrained_geometry(bounds);
        popup.with_pending_state(|s| {
            s.positioner = positioner;
            s.geometry = geometry;
        });
    }
}

impl XdgDecorationHandler for State {
    fn new_decoration(&mut self, role: ToplevelSurface) {
        self.unset_mode(role);
    }
    fn request_mode(&mut self, role: ToplevelSurface, _: DecorationMode) {
        self.unset_mode(role);
    }
    fn unset_mode(&mut self, role: ToplevelSurface) {
        // The client draws its frame; the FerroUI window overview provides
        // independent access to move, maximize, minimize and close.
        role.with_pending_state(|s| s.decoration_mode = Some(DecorationMode::ClientSide));
        if role.is_initial_configure_sent() {
            role.send_pending_configure();
        }
    }
}
delegate_xdg_decoration!(State);
delegate_compositor!(State);
delegate_shm!(State);
delegate_output!(State);
delegate_xdg_shell!(State);
