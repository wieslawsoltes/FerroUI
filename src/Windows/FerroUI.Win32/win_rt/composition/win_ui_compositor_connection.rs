//! The Windows.UI.Composition mode: the compositor on a thread with a
//! dispatcher queue, the render timer that ticks when the compositor has
//! committed, and the factory of the surfaces of windows.

use super::win_ui_composition_shared::WinUiCompositionVersions;
use crate::platform_constants::Version;
use std::time::Duration;

/// Whether the system has the mode.
pub(crate) fn is_supported(windows_version: Version) -> bool {
    windows_version >= WinUiCompositionVersions::MIN_WIN_COMPOSITION_VERSION
}

/// What the reference logs when the system is too old for the mode.
pub(crate) fn os_version_notice(windows_version: Version) -> String {
    format!(
        "Windows {} is required. Your machine has Windows {} installed.",
        WinUiCompositionVersions::MIN_WIN_COMPOSITION_VERSION,
        windows_version
    )
}

/// Whether the commit the compositor was asked for is overdue: the watchdog
/// then forces the next tick.
pub(crate) fn commit_timed_out(elapsed: Duration, commit_due_at: Option<Duration>, has_current_commit: bool) -> bool {
    commit_due_at.is_some_and(|due| elapsed > due) && has_current_commit
}

#[cfg(windows)]
pub(crate) use imp::WinUiCompositorConnection;

#[cfg(windows)]
mod imp {
    use super::super::win_ui_composited_window_surface::WinUiCompositedWindowSurface;
    use super::super::win_ui_composition_shared::WinUiCompositionShared;
    use super::super::Required;
    use super::{commit_timed_out, is_supported, os_version_notice};
    use crate::d_composition::CompositionTimerTick;
    use crate::i_blur_host::ICompositionEffectsSurface;
    use crate::i_windows_surface_factory::{IWindowsSurfaceFactory, WindowsSurface};
    use crate::interop::unmanaged_methods::{
        co_initialize_apartment_threaded, co_uninitialize, def_window_proc, dispatch_message, get_last_error, get_message,
        set_timer, WindowsMessage,
    };
    use crate::simple_window::{SimpleWindow, SimpleWndProc};
    use crate::win32_platform::Win32Platform;
    use crate::win_rt::{
        AsyncStatus, DispatcherQueueOptions, IAsyncAction, IAsyncActionCompletedHandler, IAsyncActionCompletedHandlerImpl,
        ICompositor, NativeWinRTMethods, DISPATCHERQUEUE_THREAD_APARTMENTTYPE, DISPATCHERQUEUE_THREAD_TYPE,
    };
    use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
    use ferroui_base::platform::surfaces::IPlatformRenderSurface;
    use ferroui_base::rendering::{IRenderLoop, IRenderTimer, RenderLoop, RenderTimerTick};
    use ferroui_base::FerroLocator;
    use ferroui_microcom::{ComPtr, HResult};
    use ferroui_opengl::egl::IEglWindowGlPlatformSurfaceInfo;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::sync::{mpsc, Arc};
    use std::thread::ThreadId;
    use std::time::{Duration, Instant};

    /// The log area of the mode.
    const LOG_AREA: &str = "WinUIComposition";

    /// `RPC_E_WRONG_THREAD`.
    const RPC_E_WRONG_THREAD: HResult = HResult(0x8001_010E);

    pub(crate) struct WinUiCompositorConnection {
        shared: Arc<WinUiCompositionShared>,
        tick: CompositionTimerTick,
    }

    impl IRenderTimer for WinUiCompositorConnection {
        fn tick(&self) -> Option<RenderTimerTick> {
            self.tick.tick()
        }

        fn set_tick(&self, value: Option<RenderTimerTick>) {
            self.tick.set_tick(value);
        }

        fn runs_in_background(&self) -> bool {
            true
        }
    }

    /// The connection as the surface factory of the UI thread. The
    /// reference binds the connection itself; here the connection is
    /// shared with its thread, and the factory is the handle of the UI
    /// thread to it, with the one option a surface needs.
    pub(crate) struct WinUiCompositorSurfaceFactory {
        connection: Arc<WinUiCompositorConnection>,
        backdrop_corner_radius: Option<f32>,
    }

    impl IWindowsSurfaceFactory for WinUiCompositorSurfaceFactory {
        fn requires_no_redirection_bitmap(&self) -> bool {
            true
        }

        fn create_surface(&self, info: Arc<dyn IEglWindowGlPlatformSurfaceInfo>) -> WindowsSurface {
            let surface = WinUiCompositedWindowSurface::new(self.connection.shared.clone(), info, self.backdrop_corner_radius);
            let render_surface: Arc<dyn IPlatformRenderSurface> = surface.clone();
            let effects: Arc<dyn ICompositionEffectsSurface> = surface.clone();
            WindowsSurface { surface: render_surface, effects: Some(effects), dispose: Some(Arc::new(move || surface.dispose())) }
        }
    }

    impl WinUiCompositorConnection {
        /// Activates the compositor on the calling thread, which has to
        /// have a dispatcher queue, and creates the shared state.
        fn new() -> Result<WinUiCompositorConnection, HResult> {
            let compositor = NativeWinRTMethods::create_instance::<ICompositor>("Windows.UI.Composition.Compositor")?;
            Ok(WinUiCompositorConnection { shared: WinUiCompositionShared::new(&compositor)?, tick: CompositionTimerTick::new() })
        }

        /// Creates the connection on a thread of its own, which then runs
        /// the loop of the compositor; the calling thread (the UI thread)
        /// binds the connection as the surface factory and as the render
        /// loop.
        ///
        /// The reference binds both from the new thread, into services
        /// that are shared by the threads; the services of the port are of
        /// the thread that binds, so the connection comes back to the UI
        /// thread first.
        fn try_create_and_register_core() -> Result<(), HResult> {
            let (created, result) = mpsc::channel::<Result<Arc<WinUiCompositorConnection>, HResult>>();
            std::thread::Builder::new()
                .name("DwmRenderTimerLoop".to_owned())
                .spawn(move || {
                    // The single-threaded apartment the reference gives the
                    // thread.
                    let apartment = co_initialize_apartment_threaded() >= 0;
                    // The reference lets go of the controller; the queue
                    // of the thread lives on. Here it is held until the
                    // thread ends.
                    let controller = NativeWinRTMethods::create_dispatcher_queue_controller(DispatcherQueueOptions {
                        apartment_type: DISPATCHERQUEUE_THREAD_APARTMENTTYPE::DQTAT_COM_NONE,
                        dw_size: std::mem::size_of::<DispatcherQueueOptions>() as i32,
                        thread_type: DISPATCHERQUEUE_THREAD_TYPE::DQTYPE_THREAD_CURRENT,
                    });
                    let connect = controller.as_ref().map_err(|error| *error).and_then(|_| Self::new().map(Arc::new));
                    match connect {
                        Ok(connect) => {
                            if created.send(Ok(connect.clone())).is_ok() {
                                drop(created);
                                connect.run_loop();
                            }
                        }
                        Err(error) => {
                            let _ = created.send(Err(error));
                        }
                    }
                    drop(controller);
                    if apartment {
                        co_uninitialize();
                    }
                })
                .map_err(|_| HResult::FAIL)?;

            let connect = result.recv().map_err(|_| HResult::FAIL)??;
            let backdrop_corner_radius = Win32Platform::options().win_ui_composition_backdrop_corner_radius;
            let factory: Rc<dyn IWindowsSurfaceFactory> =
                Rc::new(WinUiCompositorSurfaceFactory { connection: connect.clone(), backdrop_corner_radius });
            let render_loop: Arc<dyn IRenderLoop> = RenderLoop::from_timer(connect);
            FerroLocator::current_mutable()
                .bind::<dyn IWindowsSurfaceFactory>()
                .to_constant(factory)
                .bind::<Arc<dyn IRenderLoop>>()
                .to_constant(Rc::new(render_loop));
            Ok(())
        }

        /// The loop of the thread of the connection: the messages of the
        /// thread, among them the completion of a commit, which ticks. It
        /// ends with the process: the reference cancels the loop when the
        /// process exits; a thread of the port is ended by the system then
        /// (docs/porting/win32-platform.md, section 11.3).
        fn run_loop(self: &Arc<Self>) {
            let handler = RunLoopHandler::new(self.clone());
            handler.start();

            const WATCH_DOG_INTERVAL_IN_MS: u32 = 1000;

            let wnd_proc: SimpleWndProc = {
                let handler = handler.clone();
                Rc::new(move |hwnd, msg, w, l| {
                    if msg == WindowsMessage::WM_TIMER {
                        handler.watch_dog();
                        set_timer(hwnd, 0, WATCH_DOG_INTERVAL_IN_MS);
                    }
                    def_window_proc(hwnd, msg, w, l)
                })
            };
            let dw = SimpleWindow::new(Some(wnd_proc));
            set_timer(dw.handle(), 0, WATCH_DOG_INTERVAL_IN_MS);

            // Warning: the completion callback (RunLoopHandler.Invoke) from ICompositor5.RequestCommitAsync()
            // is called in DispatchMessage() on Windows 10, but in GetMessage() on Windows 11!
            // Be careful when changing the scope of the shared lock.

            let result = loop {
                let (result, msg) = get_message();
                if result <= 0 {
                    break result;
                }
                dispatch_message(&msg);
                handler.on_after_message_without_lock();
            };

            if result < 0 {
                if let Some(logger) = Logger::try_get(LogEventLevel::Error, LOG_AREA) {
                    logger.log(None, &format!("Unmanaged error in run_loop. Error Code: {}", get_last_error()));
                }
            }
            dw.dispose();
        }

        pub fn is_supported() -> bool {
            is_supported(Win32Platform::windows_version())
        }

        pub fn try_create_and_register() -> bool {
            if Self::is_supported() {
                match Self::try_create_and_register_core() {
                    Ok(()) => return true,
                    Err(error) => {
                        if let Some(logger) = Logger::try_get(LogEventLevel::Error, LOG_AREA) {
                            logger.log_with_values(None, "Unable to initialize WinUI compositor: {0}", &[&error]);
                        }
                    }
                }
            } else if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LOG_AREA) {
                logger.log(
                    None,
                    &format!("Unable to initialize WinUI compositor: {}", os_version_notice(Win32Platform::windows_version())),
                );
            }

            false
        }
    }

    /// The handler of the loop: asks the compositor for a commit, ticks
    /// when the commit completed and asks for the next. An object of the
    /// thread of the connection; the object the system calls
    /// ([`CompletedHandler`]) is a reference to it.
    struct RunLoopHandler {
        parent: Arc<WinUiCompositorConnection>,
        st: Instant,
        thread: ThreadId,
        commit_due_at: Cell<Option<Duration>>,
        current_commit: RefCell<Option<ComPtr<IAsyncAction>>>,
        commit_completed: Cell<bool>,
        /// The handler as the object given to a commit (`this` of the
        /// reference). It refers back to the handler, which lives as long
        /// as the loop: to the end of the process.
        completed_handler: RefCell<Option<ComPtr<IAsyncActionCompletedHandler>>>,
    }

    /// The handler as `IAsyncActionCompletedHandler`.
    struct CompletedHandler(Rc<RunLoopHandler>);

    impl IAsyncActionCompletedHandlerImpl for CompletedHandler {
        fn invoke(&self, async_info: Option<&IAsyncAction>, _async_status: AsyncStatus) -> Result<(), HResult> {
            self.0.invoke(async_info)
        }
    }

    impl RunLoopHandler {
        fn new(parent: Arc<WinUiCompositorConnection>) -> Rc<RunLoopHandler> {
            let handler = Rc::new(RunLoopHandler {
                parent,
                st: Instant::now(),
                thread: std::thread::current().id(),
                commit_due_at: Cell::new(None),
                current_commit: RefCell::new(None),
                commit_completed: Cell::new(false),
                completed_handler: RefCell::new(None),
            });
            *handler.completed_handler.borrow_mut() = Some(IAsyncActionCompletedHandler::from_impl(CompletedHandler(handler.clone())));
            handler
        }

        fn invoke(&self, async_info: Option<&IAsyncAction>) -> Result<(), HResult> {
            // The handler is state of the thread of the connection, where
            // the dispatcher queue of the thread delivers the completion.
            if std::thread::current().id() != self.thread {
                return Err(RPC_E_WRONG_THREAD);
            }
            let _lock = self.parent.shared.sync_root().lock();
            let current = self.current_commit.borrow().as_ref().map(|commit| commit.as_ptr() as usize);
            let completed = async_info.map(|info| std::ptr::from_ref(info) as usize);
            if current.is_none() || current != completed {
                return Ok(());
            }
            self.on_commit_completed();
            Ok(())
        }

        fn on_commit_completed(&self) {
            debug_assert!(self.parent.shared.sync_root().is_entered(), "Lock should be held");

            let current_commit = self.current_commit.borrow_mut().take();
            drop(current_commit);
            if let Some(tick) = self.parent.tick.tick() {
                tick(self.st.elapsed());
            }
            // Always schedule a commit so the current frame's work reaches DWM.
            self.schedule_next_commit();
            self.commit_completed.set(true);
        }

        // This method should be called outside the shared lock, as it might wait for a long time.
        fn on_after_message_without_lock(&self) {
            debug_assert!(!self.parent.shared.sync_root().is_entered(), "Lock should NOT be held");

            if !self.commit_completed.get() {
                return;
            }

            self.commit_completed.set(false);

            if self.parent.tick.is_stopped() {
                self.parent.tick.wait_for_wake();
                // Reset the expected commit callback time since we've paused
                // the render loop due to app being idle
                self.commit_due_at.set(Some(self.st.elapsed() + Duration::from_secs(1)));
            }
        }

        fn schedule_next_commit(&self) {
            debug_assert!(self.parent.shared.sync_root().is_entered(), "Lock should be held");

            self.commit_due_at.set(Some(self.st.elapsed() + Duration::from_secs(1)));
            // The reference throws out of the handler when the compositor
            // refuses; here the failure is logged, no commit is current,
            // and so nothing ticks until the mode is registered again.
            let commit = self.parent.shared.compositor5().request_commit_async().required().and_then(|commit| {
                commit.set_completed(self.completed_handler.borrow().as_deref())?;
                Ok(commit)
            });
            match commit {
                Ok(commit) => *self.current_commit.borrow_mut() = Some(commit),
                Err(error) => {
                    if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::VISUAL) {
                        logger.log_with_values(None, "ICompositor5::RequestCommitAsync failed: {HR}", &[&error]);
                    }
                }
            }
        }

        fn watch_dog(&self) {
            let _lock = self.parent.shared.sync_root().lock();
            // This is a workaround for a nasty WinUI composition API bug that prevents
            // RequestCommitAsync to ever complete after D3D device loss event with some systems
            // (A notable example is after pause/resume in Parallels Desktop)
            // We check if we haven't got a commit completion callback for a second
            // And forcefully trigger the next one, which makes the entire thing to unstuck

            let current_commit = self.current_commit.borrow().clone();
            if commit_timed_out(self.st.elapsed(), self.commit_due_at.get(), current_commit.is_some()) {
                if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::VISUAL) {
                    logger.log(
                        None,
                        "windows::UI::Composition::ICompositor5.RequestCommitAsync timed out, force-triggering next tick",
                    );
                }
                if let Some(Err(error)) = current_commit.map(|commit| commit.get_results()) {
                    if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::VISUAL) {
                        logger.log_with_values(None, "ICompositor5::RequestCommitAsync failed: {HR}, {ERR}", &[&error.0, &error]);
                    }
                }

                self.on_commit_completed();
            }
        }

        fn start(&self) {
            let _lock = self.parent.shared.sync_root().lock();
            self.schedule_next_commit();
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of the connection.
    use super::*;
    use crate::platform_constants::PlatformConstants;

    #[test]
    fn the_mode_needs_windows_10_1803() {
        assert!(!is_supported(PlatformConstants::WINDOWS8_1));
        assert!(!is_supported(PlatformConstants::WINDOWS10));
        assert!(!is_supported(Version { major: 10, minor: 0, build: 17133 }));
        assert!(is_supported(Version { major: 10, minor: 0, build: 17134 }));
        assert!(is_supported(Version { major: 10, minor: 0, build: 26100 }));
        assert_eq!(
            "Windows 10.0.17134 is required. Your machine has Windows 10.0.14393 installed.",
            os_version_notice(Version { major: 10, minor: 0, build: 14393 })
        );
    }

    #[test]
    fn the_watchdog_forces_a_tick_when_a_commit_is_a_second_overdue() {
        let second = Duration::from_secs(1);
        // Nothing asked for yet.
        assert!(!commit_timed_out(second * 5, None, false));
        // Asked for and due later.
        assert!(!commit_timed_out(second * 5, Some(second * 6), true));
        assert!(!commit_timed_out(second * 6, Some(second * 6), true));
        // Overdue.
        assert!(commit_timed_out(second * 7, Some(second * 6), true));
        // Overdue, but the commit completed and none is current.
        assert!(!commit_timed_out(second * 7, Some(second * 6), false));
    }
}
