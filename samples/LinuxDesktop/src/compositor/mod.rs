//! Scene host for the FerroUI-composited desktop. Protocol dispatch runs on
//! its own thread. Only the UI dispatcher touches controls and composition visuals.
pub mod scene;

use crate::compositor_server::{Input, Scene, Server};
use crate::session::{ClientWindow, Session, SnapSide, Snapshot};
use crate::shell::appearance::ThemeController;
use crate::{services::ApplicationCatalog, shell};
use ferroui_base::Ref;
use ferroui_controls::{Application, Control, Window, WindowState};
use ferroui_freedesktop::ui_thread_object::UiThreadHandle;
use ferroui_view::Element;
use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::io;
use std::rc::{Rc, Weak};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct Desktop {
    server: Rc<Server>,
    scene: Rc<scene::ClientScene>,
    windows: RefCell<Snapshot>,
    changed: RefCell<Option<Rc<dyn Fn()>>>,
    extent: Cell<(i32, i32)>,
    work_area: Cell<(i32, i32)>,
}

impl Desktop {
    fn update(&self) {
        let frame = self.server.snapshot();
        self.scene.refresh(&frame);
        self.update_windows(&frame);
    }

    // Pixel commits should not rebuild taskbar buttons or workspace controls.
    fn update_windows(&self, frame: &Scene) {
        let mut model = self.windows.borrow_mut();
        let matching = model.current_workspace == frame.current_workspace
            && model.workspace_names.len() == frame.workspace_count
            && model.windows.len() == frame.windows.len()
            && model.windows.iter().zip(&frame.windows).all(|(a, b)| {
                u64::from(a.id) == b.id
                    && a.title == b.title
                    && a.active == b.focused
                    && a.minimized == b.minimized
                    && a.maximized == b.maximized
                    && a.fullscreen == b.fullscreen
                    && a.above == b.above
                    && a.workspace == Some(b.workspace)
            });
        if matching {
            return;
        }
        model.current_workspace = frame.current_workspace;
        if model.workspace_names.len() != frame.workspace_count {
            model.workspace_names = (1..=frame.workspace_count)
                .map(|i| format!("Workspace {i}"))
                .collect();
        }
        model.windows = frame
            .windows
            .iter()
            .filter_map(|w| {
                Some(ClientWindow {
                    id: u32::try_from(w.id).ok()?,
                    title: w.title.clone(),
                    workspace: Some(w.workspace),
                    active: w.focused,
                    minimized: w.minimized,
                    maximized: w.maximized,
                    partially_maximized: false,
                    fullscreen: w.fullscreen,
                    can_fullscreen: true,
                    above: w.above,
                    can_raise: true,
                    last_activation: u64::from(w.focused),
                    can_maximize: true,
                    can_snap: true,
                    can_move_workspace: true,
                    can_minimize: true,
                    can_close: true,
                })
            })
            .collect();
        drop(model);
        if let Some(changed) = self.changed.borrow().clone() {
            changed();
        }
    }

    pub fn element(&self) -> Element {
        self.scene.element()
    }
    pub fn client_has_focus(&self) -> bool {
        self.scene.has_keyboard_focus()
    }
    pub fn focus_client(&self) {
        self.scene.focus();
    }
    pub fn display(&self) -> OsString {
        self.server.socket_name().into()
    }
    pub fn snapshot(&self) -> Snapshot {
        self.windows.borrow().clone()
    }
    pub fn subscribe(&self, f: impl Fn() + 'static) {
        self.changed.replace(Some(Rc::new(f)));
    }

    fn request(&self, id: u32, command: impl FnOnce(u64) -> Input) -> bool {
        if !self.windows.borrow().windows.iter().any(|w| w.id == id) {
            return false;
        }
        self.server.send(command(id.into()));
        true
    }
    pub fn close(&self, id: u32) -> bool {
        self.request(id, Input::Close)
    }
    pub fn activate(&self, id: u32) -> bool {
        self.request(id, Input::Activate)
    }
    pub fn minimize(&self, id: u32) -> bool {
        self.request(id, Input::Minimize)
    }
    pub fn toggle_maximized(&self, id: u32) -> bool {
        self.request(id, Input::Maximize)
    }

    pub fn toggle_fullscreen(&self, id: u32) -> bool {
        self.request(id, Input::Fullscreen)
    }
    pub fn toggle_above(&self, id: u32) -> bool {
        self.request(id, Input::AlwaysOnTop)
    }
    pub fn add_workspace(&self) -> bool {
        if self.windows.borrow().workspace_names.len() >= 256 {
            return false;
        }
        self.server.send(Input::AddWorkspace);
        true
    }
    pub fn can_remove_last(&self) -> bool {
        let model = self.windows.borrow();
        model.workspace_names.len() > 1
            && !model
                .windows
                .iter()
                .any(|w| w.workspace == Some(model.workspace_names.len() - 1))
    }
    pub fn remove_empty_last_workspace(&self) -> bool {
        if !self.can_remove_last() {
            return false;
        }
        self.server.send(Input::RemoveWorkspace);
        true
    }
    pub fn switch_workspace(&self, i: usize) -> bool {
        if i >= self.windows.borrow().workspace_names.len() {
            return false;
        }
        self.server.send(Input::SwitchWorkspace(i));
        true
    }
    pub fn move_to_workspace(&self, id: u32, workspace: usize) -> bool {
        if workspace >= self.windows.borrow().workspace_names.len() {
            return false;
        }
        self.request(id, |id| Input::MoveToWorkspace { id, workspace })
    }

    pub fn resize(&self, width: i32, height: i32) {
        let size = (width.max(1), height.max(1));
        if self.extent.replace(size) != size {
            self.server.send(Input::Resize {
                width: size.0,
                height: size.1,
            });
        }
    }
    pub fn set_panel_insets(&self, top: f64, bottom: f64) {
        let insets = (top.round() as i32, bottom.round() as i32);
        if self.work_area.replace(insets) != insets {
            self.server.send(Input::WorkArea {
                top: insets.0,
                bottom: insets.1,
            });
        }
    }
    pub fn snap(&self, id: u32, side: SnapSide) -> bool {
        let (width, height) = self.extent.get();
        let (top, bottom) = self.work_area.get();
        let left = width / 2;
        let (x, w) = match side {
            SnapSide::Left => (0, left),
            SnapSide::Right => (left, width - left),
        };
        if !self.request(id, |id| Input::Move { id, x, y: top }) {
            return false;
        }
        self.server.send(Input::Configure {
            id: id.into(),
            width: w.max(1),
            height: (height - top - bottom).max(1),
        });
        true
    }
}

pub(crate) fn run(
    app: &Ref<Application>,
    initial: shell::Scene,
    catalog: Rc<ApplicationCatalog>,
    themes: Rc<ThemeController>,
    socket: Option<&str>,
    managed: bool,
    lease: Option<crate::session::wayland_commands::DisplayLease>,
) -> io::Result<()> {
    // Publishing a frame replaces the previous snapshot. Multiple protocol
    // commits before the UI tick require only one dispatch and one reconciliation.
    let recipient = Rc::new(RefCell::new(Weak::<Desktop>::new()));
    let handle = UiThreadHandle::register(recipient.clone());
    struct Registration(UiThreadHandle<RefCell<Weak<Desktop>>>);
    impl Drop for Registration {
        fn drop(&mut self) {
            self.0.unregister();
        }
    }
    let _registration = Registration(handle.clone());
    let queued = Arc::new(AtomicBool::new(false));
    let server = Rc::new(Server::start(1280, 800, socket, move || {
        if queued.swap(true, Ordering::AcqRel) {
            return;
        }
        let queued = queued.clone();
        drop(handle.call(move |slot| {
            queued.store(false, Ordering::Release);
            if let Some(host) = slot.borrow().upgrade() {
                host.update();
            }
        }));
    })?);
    let host = Rc::new(Desktop {
        scene: scene::ClientScene::new(server.clone()),
        server,
        windows: RefCell::default(),
        changed: RefCell::default(),
        extent: Cell::new((1280, 800)),
        work_area: Cell::default(),
    });
    recipient.replace(Rc::downgrade(&host));
    let session = Session::composed(host.clone());
    if let Some(lease) = lease {
        lease.include_child(host.server.socket_name())?;
        session.claim_commands(Some(lease))?;
    }
    let window = Window::new();
    window.set_title(Some("Ferro Desktop — FerroUI compositor".into()));
    window.set_width(1280.0);
    window.set_height(800.0);
    if managed {
        window.set_window_state(WindowState::FullScreen);
    }
    let view = shell::view(initial, catalog, Some(session), themes, managed).mount();
    window.set_content(Some(Control::boxed(view)));
    host.update();
    eprintln!(
        "FerroUI compositor ready. WAYLAND_DISPLAY={}",
        host.server.socket_name().to_string_lossy()
    );
    app.run_window(window);
    Ok(())
}
