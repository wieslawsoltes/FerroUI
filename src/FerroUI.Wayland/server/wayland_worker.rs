//! The worker of the backend (the port of `WaylandWorker.cs`): the thread
//! that owns the connection to the compositor, dispatches its events, runs
//! the commands of the UI thread and renders.
//!
//! The reference has one class whose members are used from both threads by
//! convention. Here the convention is in the types: [`WaylandWorker`] is what
//! every thread may hold (the queue of commands, the wake-up descriptor, the
//! render timer, the platform graphics), and [`WaylandWorkerThread`] is what
//! the worker thread alone has (the connection, the globals, the surfaces and
//! cursors). The second lives in a value of the worker thread, because the
//! render targets, which the compositor of the framework calls during a tick
//! of the render loop, reach it without a reference to the worker.
//!
//! The protocol tracer of the reference (`Tracer`) belongs to stage 2 of
//! `docs/porting/wayland-platform.md`; `WAYLAND_DEBUG=1` makes the library
//! print the same messages.

use super::interop::wakeup_fd::WakeupFd;
use super::interop::wayland_connection::{DispatchResult, WaylandConnection};
use super::persistent::i_persistent_object::{ConnectionContext, IPersistentWaylandObject};
use super::persistent::w_surface::{
    shell_surface_of, shell_surface_of_mut, PopupAttach, PopupParent, WSurfaceId, WXdgPopup, WXdgShellSurface, WXdgTopLevel,
};
use super::persistent::xdg_popup_positioner_params::XdgPopupPositionerParams;
use super::persistent::wayland_bitmap_cursor::WaylandBitmapCursor;
use super::persistent::wayland_cursor::{WaylandCursor, WaylandCursorId, WaylandCursors};
use super::transient::rendering::i_wayland_framebuffer_surface::{IWaylandFramebufferSurface, WaylandRenderSurfaceTarget};
use super::transient::wayland_globals::{RegistryBootstrap, WaylandGlobals};
use super::transient::wayland_input_dispatcher::InputContext;
use super::wayland_dispatch_priority::WaylandDispatchPriority;
use super::wayland_platform_graphics::WaylandPlatformGraphics;
use super::wayland_worker_render_timer::{RenderLoopImpl, WaylandRenderTimer};
use crate::screens::i_wayland_outputs_sink::WaylandOutputsSinkProxy;
use crate::ferro_wayland_exception::FerroWaylandException;
use crate::wayland_platform_options::WaylandWorkerOptions;
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::platform::RenderTargetSceneInfo;
use ferroui_base::rendering::composition::server::LockedServerCompositor;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::ThreadId;
use std::time::Duration;
use wayland_client::Connection;

/// A command for the worker thread.
pub type WorkerJob = Box<dyn FnOnce(&mut WaylandWorkerThread) + Send>;

/// The marshaller of the proxies of the UI thread: posts a command out of band or with the
/// next commit of the compositor. An object of the UI thread, like the compositor.
pub type WorkerMarshaller = Rc<dyn Fn(WorkerJob, WaylandDispatchPriority)>;

/// The name of the worker thread.
const THREAD_NAME: &str = "FerroWayland";

thread_local! {
    /// The state of the worker, on the worker thread.
    static WORKER_THREAD: RefCell<Option<WaylandWorkerThread>> = const { RefCell::new(None) };
}

/// Calls `f` with the state of the worker. `None` on a thread other than the worker, and
/// while the state is in use further up the stack (which the worker avoids: nothing that
/// borrows the state calls out to the render loop).
pub fn with_worker_thread<R>(f: impl FnOnce(&mut WaylandWorkerThread) -> R) -> Option<R> {
    WORKER_THREAD.with(|cell| {
        let mut worker = cell.try_borrow_mut().ok()?;
        worker.as_mut().map(f)
    })
}

/// A persistent object of the worker by its number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum PersistentKey {
    Surface(WSurfaceId),
    Cursor(WaylandCursorId),
}

/// The part of the worker every thread may hold.
pub struct WaylandWorker {
    queue: Mutex<VecDeque<WorkerJob>>,
    wakeup_fd: Arc<WakeupFd>,
    has_pending_server_jobs: AtomicBool,
    platform_graphics: Arc<WaylandPlatformGraphics>,
    render_timer: Arc<WaylandRenderTimer>,
    thread: Mutex<Option<ThreadId>>,
    server_compositor: Mutex<Option<Arc<LockedServerCompositor>>>,
}

impl WaylandWorker {
    pub fn new() -> Result<Arc<Self>, FerroWaylandException> {
        let wakeup_fd = Arc::new(
            WakeupFd::new().map_err(|error| FerroWaylandException::with_inner("Unable to create the wake-up pipe", error))?,
        );
        Ok(Arc::new_cyclic(|this: &std::sync::Weak<WaylandWorker>| {
            let wake: Arc<dyn Fn() + Send + Sync> = {
                let wakeup_fd = wakeup_fd.clone();
                Arc::new(move || wakeup_fd.set())
            };
            let post_oob = {
                let this = this.clone();
                move |job: Box<dyn FnOnce() + Send>| {
                    if let Some(this) = this.upgrade() {
                        this.post_oob(Box::new(move |_| job()));
                    }
                }
            };
            let render_timer = Arc::new(WaylandRenderTimer::new(wake, post_oob));
            render_timer.start_starvation_timer();
            Self {
                queue: Mutex::new(VecDeque::new()),
                wakeup_fd,
                has_pending_server_jobs: AtomicBool::new(false),
                platform_graphics: WaylandPlatformGraphics::new(),
                render_timer,
                thread: Mutex::new(None),
                server_compositor: Mutex::new(None),
            }
        }))
    }

    pub fn platform_graphics(&self) -> &Arc<WaylandPlatformGraphics> {
        &self.platform_graphics
    }

    pub fn render_loop(&self) -> &Arc<RenderLoopImpl> {
        self.render_timer.render_loop()
    }

    /// Gives the worker the server side of the compositor of the framework, which it resets
    /// and invalidates when a connection is lost and made.
    pub fn set_server_compositor(&self, server: Arc<LockedServerCompositor>) {
        *self.server_compositor.lock().unwrap_or_else(PoisonError::into_inner) = Some(server);
    }

    fn server_compositor(&self) -> Option<Arc<LockedServerCompositor>> {
        self.server_compositor.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// Notes that a command was posted with the current batch of the compositor
    /// (`PostWithCommit` of the reference, whose other half the worker client has: giving
    /// the command to the compositor, which is an object of the UI thread).
    pub fn note_pending_server_job(&self) {
        self.has_pending_server_jobs.store(true, Ordering::Release);
    }

    /// What the worker does after every commit of the compositor of the UI thread.
    pub fn on_after_commit(&self) {
        let had_pending_server_jobs = self.has_pending_server_jobs.swap(false, Ordering::AcqRel);
        self.render_timer.on_after_commit(had_pending_server_jobs);
    }

    /// Posts a callback to the wayland thread queue directly, bypassing the compositor batch.
    /// Use for rare out-of-band messages that need immediate delivery.
    pub fn post_oob(&self, job: WorkerJob) {
        self.queue.lock().unwrap_or_else(PoisonError::into_inner).push_back(job);
        self.wakeup_fd.set();
    }

    /// Posts a callback to the wayland thread queue out-of-band (bypassing the compositor batch)
    /// and returns the receiver of its result. A callback that panics, and one that is never
    /// run, close the channel.
    pub fn invoke_oob<T: Send + 'static>(
        &self,
        callback: impl FnOnce(&mut WaylandWorkerThread) -> T + Send + 'static,
    ) -> Receiver<T> {
        let (sender, receiver) = channel();
        self.post_oob(Box::new(move |worker| {
            let _ = sender.send(callback(worker));
        }));
        receiver
    }

    /// Asks for a tick of the render loop at the end of the current iteration of the worker.
    pub fn wakeup_render_loop(&self) {
        self.render_timer.wakeup_render_loop();
    }

    /// Asks for a tick of the render loop from any thread.
    pub fn any_thread_wakeup_render_loop(&self) {
        self.render_timer.any_thread_wakeup_render_loop();
    }

    /// Connects to the display of the options. A failure is logged.
    pub fn probe(options: &WaylandWorkerOptions) -> Result<Connection, FerroWaylandException> {
        let connected = match options.display_fd {
            Some(fd) => WaylandConnection::connect_to_fd(fd),
            None => WaylandConnection::connect(options.wl_display_name.as_deref()),
        };
        if let Err(error) = &connected {
            if let Some(logger) = Logger::try_get(LogEventLevel::Error, "Wayland") {
                logger.log_with_values(None, "Failed to connect to Wayland display: {Error}", &[error]);
            }
        }
        connected
    }

    /// Starts the worker on a probed connection. `init` receives `None` once the globals of
    /// the first connection are bound, or the failure, in which case the worker thread exits
    /// without reconnecting.
    pub fn start(
        self: &Arc<Self>,
        options: WaylandWorkerOptions,
        probed_connection: Connection,
        outputs_sink: Option<WaylandOutputsSinkProxy>,
        init: Sender<Option<FerroWaylandException>>,
    ) -> Result<(), FerroWaylandException> {
        let worker = self.clone();
        std::thread::Builder::new()
            .name(THREAD_NAME.to_string())
            .spawn(move || {
                *worker.thread.lock().unwrap_or_else(PoisonError::into_inner) = Some(std::thread::current().id());
                let state = WaylandWorkerState {
                    worker: worker.clone(),
                    options,
                    outputs_sink,
                    globals: None,
                    registry_bootstrap: None,
                    top_levels: HashMap::new(),
                    popups: HashMap::new(),
                    cursors: HashMap::new(),
                    connected_objects: HashSet::new(),
                };
                WORKER_THREAD.with(|cell| *cell.borrow_mut() = Some(WaylandWorkerThread { connection: None, state }));
                worker.run(probed_connection, Some(init));
                // The state holds objects of the connection: it goes with the thread.
                WORKER_THREAD.with(|cell| cell.borrow_mut().take());
            })
            .map(|_| ())
            .map_err(|error| FerroWaylandException::with_inner("Unable to start the Wayland worker thread", error))
    }

    fn verify_access(&self) {
        if *self.thread.lock().unwrap_or_else(PoisonError::into_inner) != Some(std::thread::current().id()) {
            FerroWaylandException::new("Call from invalid thread").throw();
        }
    }

    /// The connection is gone and cannot be made again: the reference lets the exception end
    /// the process. So does the port, after it said why.
    fn fatal(error: &FerroWaylandException) -> ! {
        if let Some(logger) = Logger::try_get(LogEventLevel::Fatal, "Wayland") {
            logger.log_with_values(None, "The Wayland worker cannot go on: {Error}", &[error]);
        }
        eprintln!("ferroui-wayland: the worker cannot go on: {error}");
        std::process::abort();
    }

    fn run(self: &Arc<Self>, probed_connection: Connection, mut init: Option<Sender<Option<FerroWaylandException>>>) {
        let reconnects_disabled =
            with_worker_thread(|worker| worker.state.options.reconnects_disabled()).unwrap_or(true);
        let mut display = probed_connection;
        loop {
            let initial = init.is_some();
            match self.run_connection(display, &mut init) {
                Ok(()) => {}
                Err(error) if initial && init.is_some() => {
                    // Globals init failed on the initial connection: report to the waiting caller
                    // and exit without entering the reconnect loop
                    if let Some(init) = init.take() {
                        let _ = init.send(Some(error));
                    }
                    with_worker_thread(|worker| worker.disconnect());
                    return;
                }
                Err(error) => Self::fatal(&error),
            }
            init = None;

            with_worker_thread(|worker| worker.disconnect_persistent_objects());
            if let Some(server) = self.server_compositor() {
                server.with(|server| server.reset_all_gpu_resources());
            }
            with_worker_thread(|worker| worker.disconnect());

            if let Some(logger) = Logger::try_get(LogEventLevel::Error, "Wayland") {
                logger.log(None, "Wayland connection lost");
            }
            if reconnects_disabled {
                Self::fatal(&FerroWaylandException::new("Connection lost"));
            }

            display = loop {
                let options = with_worker_thread(|worker| worker.state.options.clone());
                match options.as_ref().map(Self::probe) {
                    Some(Ok(display)) => break display,
                    _ => std::thread::sleep(Duration::from_millis(1000)),
                }
            };
        }
    }

    fn run_queue(&self) {
        loop {
            let job = self.queue.lock().unwrap_or_else(PoisonError::into_inner).pop_front();
            let Some(job) = job else {
                return;
            };
            with_worker_thread(job);
        }
    }

    /// Runs a connection until it is lost (`Ok`) or fails. `init` is taken once the globals
    /// are bound.
    fn run_connection(
        self: &Arc<Self>,
        display: Connection,
        init: &mut Option<Sender<Option<FerroWaylandException>>>,
    ) -> Result<(), FerroWaylandException> {
        self.verify_access();
        let connected = with_worker_thread(|worker| {
            let mut connection = WaylandConnection::new(display);
            let created = WaylandGlobals::create(&mut connection, &mut worker.state);
            worker.connection = Some(connection);
            created
        });
        match connected {
            Some(Ok(())) => {}
            Some(Err(error)) => return Err(error),
            None => return Err(FerroWaylandException::new("The worker thread has no state")),
        }

        // Globals are fully constructed (required globals bound, sanity checks passed),
        // the display is deemed usable
        if let Some(init) = init.take() {
            let _ = init.send(None);
        }

        with_worker_thread(|worker| worker.connect_persistent_objects());

        self.wakeup_render_loop();
        if let Some(server) = self.server_compositor() {
            server.with(|server| server.invalidate_all_composition_targets());
        }

        loop {
            let wakeup_fd = self.wakeup_fd.clone();
            let result = with_worker_thread(|worker| {
                let WaylandWorkerThread { connection, state } = worker;
                match connection {
                    Some(connection) => connection.dispatch_queue_or_wakeup(state, wakeup_fd.poll_fd()),
                    None => Ok(DispatchResult::ConnectionReset),
                }
            })
            .unwrap_or(Ok(DispatchResult::ConnectionReset))?;
            if result == DispatchResult::ConnectionReset {
                return Ok(());
            }
            if result == DispatchResult::Wakeup {
                self.wakeup_fd.clear();
            }
            self.run_queue();
            self.render_timer.tick_render_loop_if_needed();
        }
    }
}

/// What the worker thread alone has.
pub struct WaylandWorkerThread {
    pub connection: Option<WaylandConnection>,
    pub state: WaylandWorkerState,
}

/// The state the events of the connection are dispatched to.
pub struct WaylandWorkerState {
    pub worker: Arc<WaylandWorker>,
    pub(crate) options: WaylandWorkerOptions,
    pub(crate) outputs_sink: Option<WaylandOutputsSinkProxy>,
    pub globals: Option<WaylandGlobals>,
    pub(crate) registry_bootstrap: Option<RegistryBootstrap>,
    /// The top-levels of the worker: persistent objects.
    pub top_levels: HashMap<WSurfaceId, WXdgTopLevel>,
    /// The popups of the worker: persistent objects. A popup names its parent, a top-level
    /// or another popup, by number.
    pub popups: HashMap<WSurfaceId, WXdgPopup>,
    /// The cursors of the worker; the bitmap ones are persistent objects.
    pub cursors: WaylandCursors,
    connected_objects: HashSet<PersistentKey>,
}

impl WaylandWorkerState {
    /// The object a render surface draws for, with the globals of the connection. `None`
    /// without a connection and for an object that is gone.
    pub fn framebuffer_surface(
        &mut self,
        target: WaylandRenderSurfaceTarget,
    ) -> Option<(&WaylandGlobals, &mut dyn IWaylandFramebufferSurface)> {
        let WaylandWorkerState { globals, top_levels, popups, cursors, .. } = self;
        let globals = globals.as_ref()?;
        let surface: &mut dyn IWaylandFramebufferSurface = match target {
            WaylandRenderSurfaceTarget::Surface(id) => match top_levels.get_mut(&id) {
                Some(top_level) => top_level,
                None => popups.get_mut(&id)?,
            },
            WaylandRenderSurfaceTarget::Cursor(id) => match cursors.get_mut(&id)? {
                WaylandCursor::Bitmap(cursor) => cursor,
                WaylandCursor::Standard(_) => return None,
            },
        };
        Some((globals, surface))
    }

    /// The shell surface of a top-level or of a popup.
    pub fn shell(&self, id: WSurfaceId) -> Option<&WXdgShellSurface> {
        shell_surface_of(&self.top_levels, &self.popups, id)
    }

    /// The shell surface of a top-level or of a popup.
    pub fn shell_mut(&mut self, id: WSurfaceId) -> Option<&mut WXdgShellSurface> {
        shell_surface_of_mut(&mut self.top_levels, &mut self.popups, id)
    }

    /// Stages the per-frame state of the object a render surface draws for, before its
    /// buffer is attached, and attaches the popups that waited for the surface to be mapped
    /// (the loop over the pending child popups in `OnBeforeNewBufferAttached` of the
    /// reference, which reaches the children through the parent object).
    pub fn on_before_new_buffer_attached(&mut self, target: WaylandRenderSurfaceTarget, scene_info: &RenderTargetSceneInfo) {
        match self.framebuffer_surface(target) {
            Some((globals, surface)) => surface.on_before_new_buffer_attached(globals, scene_info),
            None => return,
        }
        if let WaylandRenderSurfaceTarget::Surface(id) = target {
            let children = self.shell_mut(id).map(WXdgShellSurface::take_children_to_attach).unwrap_or_default();
            for child in children {
                self.try_attach_popup_to_parent(child);
            }
        }
    }

    /// What a popup reads of its parent, when both exist.
    fn popup_parent(&self, popup: WSurfaceId) -> Option<(WSurfaceId, Option<PopupParent>)> {
        let parent_id = self.popups.get(&popup)?.parent();
        Some((parent_id, self.shell(parent_id).map(PopupParent::of)))
    }

    fn finish_popup_attach(&mut self, popup: WSurfaceId, parent: WSurfaceId, attach: PopupAttach) {
        if attach == PopupAttach::ParentNotMapped {
            if let Some(parent) = self.shell_mut(parent) {
                parent.unregister_pending_child_popup(popup);
                parent.register_pending_child_popup(popup);
            }
        }
    }

    /// Creates the role object of a popup if its parent is mapped, and has the parent
    /// remember the popup if it is not (`TryAttachToParent`).
    pub fn try_attach_popup_to_parent(&mut self, popup: WSurfaceId) {
        let Some((parent_id, parent)) = self.popup_parent(popup) else {
            return;
        };
        let WaylandWorkerState { globals, popups, .. } = self;
        let Some(popup_surface) = popups.get_mut(&popup) else {
            return;
        };
        let attach = popup_surface.try_attach_to_parent(parent.as_ref(), globals.as_ref());
        self.finish_popup_attach(popup, parent_id, attach);
    }

    /// Gives a popup new positioner parameters (`UpdatePositioner`).
    pub fn update_popup_positioner(&mut self, popup: WSurfaceId, positioner: XdgPopupPositionerParams) {
        let Some((parent_id, parent)) = self.popup_parent(popup) else {
            return;
        };
        let has_attached_children = self.popups.values().any(|child| child.parent() == popup && child.is_attached());
        let WaylandWorkerState { globals, popups, .. } = self;
        let Some(popup_surface) = popups.get_mut(&popup) else {
            return;
        };
        let attach = popup_surface.update_positioner(positioner, parent.as_ref(), globals.as_ref(), has_attached_children);
        self.finish_popup_attach(popup, parent_id, attach);
    }

    /// Refreshes the cursor of the pointers that are over a surface.
    pub fn notify_cursor_changed(&self, surface: WSurfaceId) {
        let Some(globals) = &self.globals else {
            return;
        };
        let cx = InputContext {
            top_levels: &self.top_levels,
            popups: &self.popups,
            cursors: &self.cursors,
            cursor_manager: &globals.cursor_manager,
            connection_id: globals.connection_id,
        };
        globals.input_dispatcher.notify_cursor_changed(surface, &cx);
    }
}

impl WaylandWorkerThread {
    /// Sends the pending requests and waits until the compositor handled them.
    pub fn roundtrip(&mut self) {
        let WaylandWorkerThread { connection, state } = self;
        if let Some(connection) = connection {
            // A failure shows again in the wait of the worker, which classifies it.
            let _ = connection.roundtrip(state);
        }
    }

    fn is_connected(&self) -> bool {
        self.connection.as_ref().is_some_and(WaylandConnection::is_connected) && self.state.globals.is_some()
    }

    fn connect_persistent_object(&mut self, key: PersistentKey) {
        let mut connected_popup = None;
        let enforce_roundtrip = {
            let WaylandWorkerThread { connection, state } = self;
            let Some(connection) = connection else {
                return;
            };
            let WaylandWorkerState { globals, top_levels, popups, cursors, connected_objects, .. } = state;
            let Some(globals) = globals.as_ref() else {
                return;
            };
            let cx = ConnectionContext { connection_id: connection.id(), queue_handle: connection.queue_handle(), globals };
            let enforce_roundtrip = match key {
                PersistentKey::Surface(id) => match (top_levels.get_mut(&id), popups.get_mut(&id)) {
                    (Some(top_level), _) => {
                        top_level.on_connected(&cx);
                        top_level.enforce_buffer_creation_roundtrip()
                    }
                    (None, Some(popup)) => {
                        popup.on_connected(&cx);
                        connected_popup = Some(id);
                        popup.enforce_buffer_creation_roundtrip()
                    }
                    (None, None) => return,
                },
                PersistentKey::Cursor(id) => match cursors.get_mut(&id) {
                    Some(WaylandCursor::Bitmap(cursor)) => {
                        cursor.on_connected(&cx);
                        cursor.enforce_buffer_creation_roundtrip()
                    }
                    _ => return,
                },
            };
            connected_objects.insert(key);
            enforce_roundtrip
        };
        // `OnConnected` of a popup ends with the attempt to attach it to its parent.
        if let Some(popup) = connected_popup {
            self.state.try_attach_popup_to_parent(popup);
        }
        if enforce_roundtrip {
            self.roundtrip();
        }
    }

    fn persistent_keys(&self) -> Vec<PersistentKey> {
        let mut keys: Vec<PersistentKey> = self.state.top_levels.keys().map(|id| PersistentKey::Surface(*id)).collect();
        // Popups after the top-levels and in the order they were made, so that a parent
        // comes before its children.
        let mut popups: Vec<WSurfaceId> = self.state.popups.keys().copied().collect();
        popups.sort();
        keys.extend(popups.into_iter().map(PersistentKey::Surface));
        keys.extend(self.state.cursors.iter().filter_map(|(id, cursor)| match cursor {
            WaylandCursor::Bitmap(_) => Some(PersistentKey::Cursor(*id)),
            WaylandCursor::Standard(_) => None,
        }));
        keys
    }

    fn connect_persistent_objects(&mut self) {
        for key in self.persistent_keys() {
            self.connect_persistent_object(key);
        }
    }

    fn disconnect_persistent_object(&mut self, key: PersistentKey) {
        match key {
            PersistentKey::Surface(id) => {
                if let Some(top_level) = self.state.top_levels.get_mut(&id) {
                    top_level.on_disconnected();
                } else if let Some(popup) = self.state.popups.get_mut(&id) {
                    let parent = popup.parent();
                    popup.on_disconnected();
                    // Unregister from the parent's pending list in case we were
                    // deferred and the connection went down before we got attached.
                    if let Some(parent) = self.state.shell_mut(parent) {
                        parent.unregister_pending_child_popup(id);
                    }
                }
            }
            PersistentKey::Cursor(id) => {
                if let Some(WaylandCursor::Bitmap(cursor)) = self.state.cursors.get_mut(&id) {
                    cursor.on_disconnected();
                }
            }
        }
    }

    /// Tells every persistent object that the connection is lost.
    fn disconnect_persistent_objects(&mut self) {
        for key in self.persistent_keys() {
            self.disconnect_persistent_object(key);
        }
        self.state.connected_objects.clear();
    }

    /// Releases the globals and the connection.
    fn disconnect(&mut self) {
        if let Some(globals) = self.state.globals.take() {
            globals.dispose(&self.state);
        }
        if let Some(mut connection) = self.connection.take() {
            connection.dispose();
        }
    }

    /// Registers a top-level (`RegisterPersistentObject`): it is connected at once when a
    /// connection exists, and with every later one.
    pub fn register_top_level(&mut self, top_level: WXdgTopLevel) {
        let id = top_level.shell().surface().id();
        self.state.top_levels.insert(id, top_level);
        if self.is_connected() {
            self.connect_persistent_object(PersistentKey::Surface(id));
        }
    }

    /// Registers a popup (`RegisterPersistentObject`), like a top-level.
    pub fn register_popup(&mut self, popup: WXdgPopup) {
        let id = popup.shell().surface().id();
        self.state.popups.insert(id, popup);
        if self.is_connected() {
            self.connect_persistent_object(PersistentKey::Surface(id));
        }
    }

    /// Destroys a top-level or a popup and unregisters it from the worker (`Disconnect` of
    /// a surface).
    pub fn unregister_surface(&mut self, id: WSurfaceId) {
        let key = PersistentKey::Surface(id);
        if self.state.connected_objects.remove(&key) {
            self.disconnect_persistent_object(key);
        }
        self.state.top_levels.remove(&id);
        if let Some(popup) = self.state.popups.remove(&id) {
            if let Some(parent) = self.state.shell_mut(popup.parent()) {
                parent.unregister_pending_child_popup(id);
            }
        }
    }

    /// Registers a themed cursor: a name for a cursor of the theme of whatever connection.
    pub fn register_standard_cursor(&mut self, id: WaylandCursorId, cursor: super::persistent::wayland_cursor::WaylandStandardCursor) {
        self.state.cursors.insert(id, WaylandCursor::Standard(cursor));
    }

    /// Registers a bitmap cursor, a persistent object.
    pub fn register_bitmap_cursor(&mut self, id: WaylandCursorId, cursor: WaylandBitmapCursor) {
        self.state.cursors.insert(id, WaylandCursor::Bitmap(cursor));
        if self.is_connected() {
            self.connect_persistent_object(PersistentKey::Cursor(id));
        }
    }

    /// Releases a cursor (`Destroy` of a cursor): nothing to release for a themed one, whose
    /// surfaces the cursor manager owns; a bitmap cursor is unregistered.
    pub fn destroy_cursor(&mut self, id: WaylandCursorId) {
        let key = PersistentKey::Cursor(id);
        if self.state.connected_objects.remove(&key) {
            self.disconnect_persistent_object(key);
        }
        self.state.cursors.remove(&id);
    }
}
