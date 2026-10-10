//! The DirectComposition mode: the composition device, the render timer
//! that ticks when the device has committed a frame, and the factory of
//! the surfaces of windows.

use super::{DirectCompositedWindowSurface, DirectCompositionShared};
use crate::i_blur_host::ICompositionEffectsSurface;
use crate::i_windows_surface_factory::{IWindowsSurfaceFactory, WindowsSurface};
use crate::platform_constants::{PlatformConstants, Version};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::rendering::{IRenderTimer, RenderTimerTick};
use ferroui_opengl::egl::IEglWindowGlPlatformSurfaceInfo;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};

/// An `AutoResetEvent`: a signal one waiter takes.
#[derive(Default)]
pub(crate) struct AutoResetEvent {
    signaled: Mutex<bool>,
    changed: Condvar,
}

impl AutoResetEvent {
    pub fn set(&self) {
        *self.signaled.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.changed.notify_one();
    }

    pub fn wait_one(&self) {
        let mut signaled = self.signaled.lock().unwrap_or_else(PoisonError::into_inner);
        while !*signaled {
            signaled = self.changed.wait(signaled).unwrap_or_else(PoisonError::into_inner);
        }
        *signaled = false;
    }
}

/// The tick of a composition timer with the two flags the reference keeps
/// beside it: set and asked for by the render loop from any thread, read by
/// the thread of the timer.
#[derive(Default)]
pub(crate) struct CompositionTimerTick {
    tick: Mutex<Option<RenderTimerTick>>,
    wake_event: AutoResetEvent,
    stopped: AtomicBool,
}

impl CompositionTimerTick {
    pub fn new() -> CompositionTimerTick {
        CompositionTimerTick { tick: Mutex::new(None), wake_event: AutoResetEvent::default(), stopped: AtomicBool::new(true) }
    }

    pub fn tick(&self) -> Option<RenderTimerTick> {
        self.tick.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    pub fn set_tick(&self, value: Option<RenderTimerTick>) {
        let mut tick = self.tick.lock().unwrap_or_else(PoisonError::into_inner);
        if value.is_some() {
            *tick = value;
            self.stopped.store(false, Ordering::SeqCst);
            self.wake_event.set();
        } else {
            self.stopped.store(true, Ordering::SeqCst);
            *tick = None;
        }
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }

    /// Waits until a tick is set.
    pub fn wait_for_wake(&self) {
        self.wake_event.wait_one();
    }
}

pub(crate) struct DirectCompositionConnection {
    tick: CompositionTimerTick,
    shared: Arc<DirectCompositionShared>,
}

impl DirectCompositionConnection {
    pub fn new(shared: Arc<DirectCompositionShared>) -> DirectCompositionConnection {
        DirectCompositionConnection { tick: CompositionTimerTick::new(), shared }
    }

    pub fn is_supported(windows_version: Version) -> bool {
        windows_version >= PlatformConstants::WINDOWS8_1
    }

    /// What the reference logs when the system is too old for the mode.
    pub(crate) fn os_version_notice(windows_version: Version) -> String {
        format!(
            "Windows {} is required. Your machine has Windows {} installed.",
            PlatformConstants::WINDOWS8_1,
            windows_version
        )
    }
}

impl IRenderTimer for DirectCompositionConnection {
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

/// The connection as the surface factory of the UI thread. The reference
/// binds the connection itself; here the connection is shared with its
/// thread, and the factory is the handle of the UI thread to it.
pub(crate) struct DirectCompositionSurfaceFactory(pub Arc<DirectCompositionConnection>);

impl IWindowsSurfaceFactory for DirectCompositionSurfaceFactory {
    fn requires_no_redirection_bitmap(&self) -> bool {
        true
    }

    fn create_surface(&self, info: Arc<dyn IEglWindowGlPlatformSurfaceInfo>) -> WindowsSurface {
        let surface = DirectCompositedWindowSurface::new(self.0.shared.clone(), info);
        let render_surface: Arc<dyn IPlatformRenderSurface> = surface.clone();
        let effects: Arc<dyn ICompositionEffectsSurface> = surface.clone();
        WindowsSurface { surface: render_surface, effects: Some(effects), dispose: Some(Arc::new(move || surface.dispose())) }
    }
}

#[cfg(windows)]
mod imp {
    use super::super::{IDCompositionDesktopDevice, NativeMethods};
    use super::*;
    use crate::interop::unmanaged_methods::{co_initialize_apartment_threaded, co_uninitialize};
    use crate::win32_platform::Win32Platform;
    use ferroui_base::rendering::{IRenderLoop, RenderLoop};
    use ferroui_base::FerroLocator;
    use ferroui_microcom::HResult;
    use std::rc::Rc;
    use std::sync::mpsc;
    use std::time::Instant;

    impl DirectCompositionConnection {
        /// Creates the device and the connection on a thread of their own,
        /// which then runs the timer; the calling thread (the UI thread)
        /// binds the connection as the surface factory and as the render
        /// loop.
        ///
        /// The reference binds both from the new thread, into services that
        /// are shared by the threads; the services of the port are of the
        /// thread that binds, so the connection comes back to the UI thread
        /// first.
        fn try_create_and_register_core() -> Result<(), HResult> {
            let (created, result) = mpsc::channel::<Result<Arc<DirectCompositionConnection>, HResult>>();
            std::thread::Builder::new()
                .name("DwmRenderTimerLoop".to_owned())
                .spawn(move || {
                    // The single-threaded apartment the reference gives the
                    // thread.
                    let apartment = co_initialize_apartment_threaded() >= 0;
                    let connect = NativeMethods::d_composition_create_device2::<IDCompositionDesktopDevice>(None)
                        .map(|device| Arc::new(DirectCompositionConnection::new(DirectCompositionShared::new(&device))));
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
                    if apartment {
                        co_uninitialize();
                    }
                })
                .map_err(|_| HResult::FAIL)?;

            let connect = result.recv().map_err(|_| HResult::FAIL)??;
            let factory: Rc<dyn IWindowsSurfaceFactory> = Rc::new(DirectCompositionSurfaceFactory(connect.clone()));
            let render_loop: Arc<dyn IRenderLoop> = RenderLoop::from_timer(connect);
            FerroLocator::current_mutable()
                .bind::<dyn IWindowsSurfaceFactory>()
                .to_constant(factory)
                .bind::<Arc<dyn IRenderLoop>>()
                .to_constant(Rc::new(render_loop));
            Ok(())
        }

        /// The loop of the timer thread: a tick for every commit of the
        /// device the system has completed. It ends with the process, as
        /// the background thread of the reference does.
        fn run_loop(&self) {
            let stopwatch = Instant::now();
            let device = self.shared.device().clone();

            loop {
                if self.tick.is_stopped() {
                    self.tick.wait_for_wake();
                }
                match device.wait_for_commit_completion() {
                    Ok(()) => {
                        if let Some(tick) = self.tick.tick() {
                            tick(stopwatch.elapsed());
                        }
                    }
                    Err(error) => {
                        if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::WIN32_PLATFORM) {
                            logger.log(
                                None,
                                &format!("Failed to wait for vblank, Exception: {error}, HRESULT = {:#010X}", error.0),
                            );
                        }
                    }
                }
            }
        }

        pub fn try_create_and_register() -> bool {
            let windows_version = Win32Platform::windows_version();
            if Self::is_supported(windows_version) {
                match Self::try_create_and_register_core() {
                    Ok(()) => return true,
                    Err(error) => {
                        if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::WIN32_PLATFORM) {
                            logger.log_with_values(None, "Unable to initialize WinUI compositor: {0}", &[&error]);
                        }
                    }
                }
            } else if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::WIN32_PLATFORM) {
                logger.log(
                    None,
                    &format!("Unable to initialize WinUI compositor: {}", Self::os_version_notice(windows_version)),
                );
            }

            false
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of the connection.
    use super::*;
    use std::time::Duration;

    #[test]
    fn the_timer_is_stopped_until_a_tick_is_set_and_wakes_its_thread() {
        let tick = Arc::new(CompositionTimerTick::new());
        assert!(tick.is_stopped());
        assert!(tick.tick().is_none());

        let woke = {
            let tick = tick.clone();
            std::thread::spawn(move || {
                tick.wait_for_wake();
                tick.tick().is_some()
            })
        };
        std::thread::sleep(Duration::from_millis(20));
        tick.set_tick(Some(Arc::new(|_| {})));

        assert!(woke.join().unwrap());
        assert!(!tick.is_stopped());

        tick.set_tick(None);
        assert!(tick.is_stopped());
        assert!(tick.tick().is_none());
    }

    #[test]
    fn the_mode_needs_windows_8_1() {
        assert!(!DirectCompositionConnection::is_supported(PlatformConstants::WINDOWS8));
        assert!(DirectCompositionConnection::is_supported(PlatformConstants::WINDOWS8_1));
        assert!(DirectCompositionConnection::is_supported(PlatformConstants::WINDOWS10));
        assert_eq!(
            "Windows 6.3 is required. Your machine has Windows 6.2 installed.",
            DirectCompositionConnection::os_version_notice(PlatformConstants::WINDOWS8)
        );
    }
}
