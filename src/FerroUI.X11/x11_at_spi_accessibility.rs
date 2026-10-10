//! The port of `X11AtSpiAccessibility.cs`: the accessibility server of
//! the platform, started when the platform is, and the windows it is to
//! be told about.

use crate::x11_window::X11Window;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::Ref;
use ferroui_controls::automation::peers::{AutomationPeer, ControlAutomationPeer};
use ferroui_controls::Control;
use ferroui_freedesktop::at_spi::{AtSpiAccessibilityWatcher, AtSpiServer};
use ferroui_freedesktop::signal_watch::CancellationFlag;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

pub(crate) struct X11AtSpiAccessibility {
    this: Weak<X11AtSpiAccessibility>,
    tracked_windows: RefCell<Vec<Weak<X11Window>>>,
    watcher: RefCell<Option<Rc<AtSpiAccessibilityWatcher>>>,
    server: RefCell<Option<Rc<AtSpiServer>>>,
    server_started_unconditionally: Cell<bool>,
}

impl X11AtSpiAccessibility {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            tracked_windows: RefCell::new(Vec::new()),
            watcher: RefCell::new(None),
            server: RefCell::new(None),
            server_started_unconditionally: Cell::new(false),
        })
    }

    pub(crate) fn server(&self) -> Option<Rc<AtSpiServer>> {
        self.server.borrow().clone()
    }

    pub(crate) fn initialize(&self) {
        *self.watcher.borrow_mut() = Some(AtSpiAccessibilityWatcher::new());
        let Some(this) = self.this.upgrade() else { return };
        drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            this.initialize_async().await;
        }));
    }

    pub(crate) fn track_window(&self, window: &X11Window) {
        self.tracked_windows.borrow_mut().push(window.this.clone());
    }

    pub(crate) fn untrack_window(&self, window: &X11Window) {
        let mut tracked = self.tracked_windows.borrow_mut();
        if let Some(index) = tracked.iter().position(|other| other.ptr_eq(&window.this)) {
            tracked.remove(index);
        }
    }

    async fn initialize_async(self: &Rc<Self>) {
        Self::wait_for_ui_thread_settle_async().await;

        // Path A: try unconditional connection first (GTK4 approach).
        // This avoids delaying startup on watcher/session-bus property calls.
        if self.try_start_server_async().await {
            self.server_started_unconditionally.set(true);
            return;
        }

        // Path A failed - fall back to watcher-driven enablement.
        let Some(watcher) = self.watcher.borrow().clone() else { return };
        watcher.init_async().await;
        let weak = Rc::downgrade(self);
        watcher.is_enabled_changed().subscribe(move |enabled| {
            if let Some(this) = weak.upgrade() {
                this.on_accessibility_enabled_changed(enabled);
            }
        });

        if watcher.is_enabled() {
            self.enable_accessibility_async().await;
        }
    }

    async fn wait_for_ui_thread_settle_async() {
        // Wait until UI work is drained to context-idle so AT-SPI handlers
        // are responsive when clients start querying immediately after embed.
        let settle = Dispatcher::ui_thread()
            .invoke_async_task_local_with_priority(|| async {}, DispatcherPriority::CONTEXT_IDLE);

        // Keep startup bounded in case the UI thread never reaches idle
        // (e.g., continuous high-priority work).
        let timed_out = CancellationFlag::new();
        let flag = timed_out.clone();
        let timer =
            DispatcherTimer::run_once(move || flag.cancel(), Duration::from_millis(100), DispatcherPriority::DEFAULT);
        if timed_out.run(settle).await.is_none() {
            if let Some(logger) = Logger::try_get(LogEventLevel::Debug, LogArea::X11_PLATFORM) {
                logger.log(None, "AT-SPI startup wait timed out before UI thread reached idle");
            }
        }
        timer.dispose();
    }

    fn on_accessibility_enabled_changed(self: &Rc<Self>, enabled: bool) {
        let this = self.clone();
        drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            if enabled {
                this.enable_accessibility_async().await;
            } else if !this.server_started_unconditionally.get() {
                // Only tear down if server wasn't started unconditionally.
                // When started unconditionally, event listener tracking handles suppression.
                this.disable_accessibility();
            }
        }));
    }

    async fn try_start_server_async(&self) -> bool {
        if self.server.borrow().is_some() {
            return true;
        }

        let server = AtSpiServer::new();
        if let Err(e) = server.start_async().await {
            if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::X11_PLATFORM) {
                logger.log(None, &format!("AT-SPI server startup attempt failed: {e}"));
            }
            return false;
        }
        *self.server.borrow_mut() = Some(server.clone());

        // Register any already-tracked windows.
        let tracked: Vec<Rc<X11Window>> = self.tracked_windows.borrow().iter().filter_map(Weak::upgrade).collect();
        for window in tracked {
            if let Some(peer) = Self::try_get_window_peer(&window) {
                server.add_window(&peer);
            }
        }

        true
    }

    /// The automation peer of the content of a window, created when it
    /// does not exist: the root of its automation tree, or itself. `None`
    /// for a window that has no input root yet (it can be tracked before
    /// it has one).
    pub(crate) fn try_get_window_peer(window: &X11Window) -> Option<Ref<AutomationPeer>> {
        let control = window.input_root_or_none()?.focus_root().cast::<Control>()?;
        let peer = ControlAutomationPeer::create_peer_for_element(&control);
        Some(peer.get_automation_root().unwrap_or(peer))
    }

    /// As [`try_get_window_peer`](Self::try_get_window_peer), without
    /// creating a peer (`GetAutomationPeer`).
    pub(crate) fn existing_window_peer(window: &X11Window) -> Option<Ref<AutomationPeer>> {
        let control = window.input_root_or_none()?.focus_root().cast::<Control>()?;
        let peer = ControlAutomationPeer::from_element(&control)?;
        Some(peer.get_automation_root().unwrap_or(peer))
    }

    async fn enable_accessibility_async(&self) {
        self.try_start_server_async().await;
    }

    fn disable_accessibility(&self) {
        if let Some(server) = self.server.borrow_mut().take() {
            server.dispose();
        }
    }
}
