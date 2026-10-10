//! The low-latency DXGI swap chain mode: a render timer that ticks with the
//! vertical blank of the output with the highest refresh rate, and the
//! factory of the swap chain surfaces of windows.

use std::collections::HashMap;

/// `ENUM_CURRENT_SETTINGS`.
#[allow(dead_code)] // The constant of the reference; the call that takes it is behind `enum_current_display_settings`.
pub const ENUM_CURRENT_SETTINGS: u32 = u32::MAX;

/// The output to wait on: the one whose monitor has the highest refresh
/// rate, the first of them when several have it. `outputs` are the monitor
/// handles of the outputs in the order DXGI enumerates them. An output
/// whose monitor has no known frequency, or a frequency of zero, is never
/// better than what was found before it.
// Note: Defining best as display with highest refresh rate on
pub(crate) fn best_output_to_v_wait_on(outputs: &[isize], monitor_frequencies: &HashMap<isize, u32>) -> Option<usize> {
    let mut highest_refresh_rate = 0.0f64;
    let mut best = None;
    for (index, monitor) in outputs.iter().enumerate() {
        let frequency = monitor_frequencies.get(monitor).map_or(highest_refresh_rate, |frequency| f64::from(*frequency));

        if highest_refresh_rate < frequency {
            // ooh I like this output!
            best = Some(index);
            highest_refresh_rate = frequency;
        }
    }
    best
}

#[cfg(windows)]
pub(crate) use imp::DxgiConnection;

#[cfg(windows)]
mod imp {
    use super::super::dxgi_swapchain_window::DxgiSwapchainWindow;
    use super::super::{DirectXUnmanagedMethods, IDXGIAdapter, IDXGIOutput};
    use super::best_output_to_v_wait_on;
    use crate::d_composition::CompositionTimerTick;
    use crate::i_windows_surface_factory::{IWindowsSurfaceFactory, WindowsSurface};
    use crate::interop::unmanaged_methods::{
        co_initialize_apartment_threaded, co_uninitialize, dwm_flush, enum_current_display_settings, get_monitor_info,
    };
    use crate::screen_impl::ScreenImpl;
    use crate::sync_root::SyncRoot;
    use ferroui_base::logging::{LogEventLevel, Logger};
    use ferroui_base::platform::surfaces::IPlatformRenderSurface;
    use ferroui_base::rendering::{IRenderLoop, IRenderTimer, RenderLoop, RenderTimerTick};
    use ferroui_base::FerroLocator;
    use ferroui_microcom::{ComPtr, HResult};
    use ferroui_opengl::egl::IEglWindowGlPlatformSurfaceInfo;
    use std::collections::HashMap;
    use std::rc::Rc;
    use std::sync::{mpsc, Arc};
    use std::time::Instant;

    const LOG_AREA: &str = "DXGI";

    pub(crate) struct DxgiConnection {
        tick: CompositionTimerTick,
        sync_lock: Arc<SyncRoot>,
    }

    impl IRenderTimer for DxgiConnection {
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
    /// thread to it.
    pub(crate) struct DxgiSurfaceFactory(pub Arc<DxgiConnection>);

    impl IWindowsSurfaceFactory for DxgiSurfaceFactory {
        fn requires_no_redirection_bitmap(&self) -> bool {
            false
        }

        fn create_surface(&self, info: Arc<dyn IEglWindowGlPlatformSurfaceInfo>) -> WindowsSurface {
            let surface: Arc<dyn IPlatformRenderSurface> = DxgiSwapchainWindow::new(self.0.clone(), info);
            WindowsSurface { surface, effects: None, dispose: None }
        }
    }

    fn log_vblank_failure(error: HResult) {
        if let Some(logger) = Logger::try_get(LogEventLevel::Error, LOG_AREA) {
            logger.log(None, &format!("Failed to wait for vblank, Exception: {error}, HRESULT = {:#010X}", error.0));
        }
    }

    impl DxgiConnection {
        pub fn new(sync_lock: Arc<SyncRoot>) -> DxgiConnection {
            DxgiConnection { tick: CompositionTimerTick::new(), sync_lock }
        }

        pub fn try_create_and_register() -> bool {
            match Self::try_create_and_register_core() {
                Ok(()) => true,
                Err(error) => {
                    if let Some(logger) = Logger::try_get(LogEventLevel::Error, LOG_AREA) {
                        logger.log_with_values(None, "Unable to establish Dxgi: {0}", &[&error]);
                    }
                    false
                }
            }
        }

        /// The loop of the timer thread: a tick after every vertical blank
        /// of the output, inside the lock. It ends with the process, as
        /// the background thread of the reference does.
        fn run_loop(&self) {
            let stopwatch = Instant::now();
            let mut output = match Self::get_best_output_to_v_wait_on() {
                Ok(output) => output,
                Err(error) => {
                    log_vblank_failure(error);
                    None
                }
            };

            loop {
                if self.tick.is_stopped() {
                    self.tick.wait_for_wake();
                }

                let _lock = self.sync_lock.lock();
                if let Some(waited) = output.as_ref().map(|output| output.wait_for_v_blank()) {
                    if let Err(error) = waited {
                        log_vblank_failure(error);
                        output = None;
                        match Self::get_best_output_to_v_wait_on() {
                            Ok(best) => output = best,
                            Err(error) => {
                                // The exception of the reference leaves the
                                // pass without a tick.
                                log_vblank_failure(error);
                                continue;
                            }
                        }
                    }
                } else {
                    // well since that obviously didn't work, then let's use the lowest-common-denominator instead
                    // for reference, this has never happened on my machine,
                    // but theoretically someone could have a weirder setup out there
                    dwm_flush();
                }
                if let Some(tick) = self.tick.tick() {
                    tick(stopwatch.elapsed());
                }
            }
        }

        /// The output with the highest refresh rate among the outputs of
        /// every adapter; `None` when no output has a monitor with a known
        /// frequency.
        pub(super) fn get_best_output_to_v_wait_on() -> Result<Option<ComPtr<IDXGIOutput>>, HResult> {
            let fact = DirectXUnmanagedMethods::create_dxgi_factory()?.ok_or(HResult::POINTER)?;

            let monitor_frequencies = Self::get_all_monitor_frequencies();

            let mut outputs = Vec::new();
            let mut monitors = Vec::new();
            // this looks odd, but that's just how one enumerates adapters in DXGI
            let mut adapter_index = 0;
            loop {
                let mut adapter_pointer = std::ptr::null_mut::<std::ffi::c_void>();
                // SAFETY: the place of one pointer, of this frame.
                if unsafe { fact.enum_adapters(adapter_index, (&raw mut adapter_pointer).cast()) } != 0 {
                    break;
                }
                // SAFETY: the call succeeded, so the pointer is null or an
                // adapter whose reference this function owns.
                let Some(adapter) = (unsafe { ComPtr::<IDXGIAdapter>::from_raw(adapter_pointer.cast()) }) else {
                    break;
                };
                let mut output_index = 0;
                loop {
                    let mut output_pointer = std::ptr::null_mut::<std::ffi::c_void>();
                    // SAFETY: as above.
                    if unsafe { adapter.enum_outputs(output_index, (&raw mut output_pointer).cast()) } != 0 {
                        break;
                    }
                    // SAFETY: as above, an output.
                    let Some(output) = (unsafe { ComPtr::<IDXGIOutput>::from_raw(output_pointer.cast()) }) else {
                        break;
                    };
                    let output_desc = output.get_desc()?;
                    monitors.push(output_desc.monitor.0);
                    outputs.push(output);
                    // and then increment index to move onto the next monitor
                    output_index += 1;
                }
                // and then increment index to move onto the next display adapater
                adapter_index += 1;
            }

            Ok(best_output_to_v_wait_on(&monitors, &monitor_frequencies).map(|index| outputs.swap_remove(index)))
        }

        pub(super) fn get_all_monitor_frequencies() -> HashMap<isize, u32> {
            let monitor_handlers = ScreenImpl::get_all_display_monitor_handlers();
            let mut dictionary = HashMap::with_capacity(monitor_handlers.len());

            for monitor_handler in monitor_handlers {
                // A monitor or a device the system does not answer for has
                // the frequency zero, as the zeroed structures of the
                // reference have.
                let frequency = get_monitor_info(monitor_handler)
                    .and_then(|info| enum_current_display_settings(&info.sz_device))
                    .map_or(0, |(frequency, _orientation)| frequency);

                dictionary.insert(monitor_handler, frequency);
            }

            dictionary
        }

        // Used the windows composition as a blueprint for this startup/creation
        /// The reference binds the surface factory and the render loop
        /// from the new thread, into services that are shared by the
        /// threads; the services of the port are of the thread that binds,
        /// so the connection comes back to the UI thread first.
        fn try_create_and_register_core() -> Result<(), HResult> {
            let (created, result) = mpsc::channel::<Arc<DxgiConnection>>();
            let pump_lock = SyncRoot::new();
            std::thread::Builder::new()
                .name("DxgiRenderTimerLoop".to_owned())
                .spawn(move || {
                    // The single-threaded apartment the reference gives the
                    // thread.
                    let apartment = co_initialize_apartment_threaded() >= 0;
                    let connection = Arc::new(DxgiConnection::new(pump_lock));
                    if created.send(connection.clone()).is_ok() {
                        drop(created);
                        connection.run_loop();
                    }
                    if apartment {
                        co_uninitialize();
                    }
                })
                .map_err(|_| HResult::FAIL)?;

            // block until
            let connection = result.recv().map_err(|_| HResult::FAIL)?;
            let factory: Rc<dyn IWindowsSurfaceFactory> = Rc::new(DxgiSurfaceFactory(connection.clone()));
            let render_loop: Arc<dyn IRenderLoop> = RenderLoop::from_timer(connection);
            FerroLocator::current_mutable()
                .bind::<dyn IWindowsSurfaceFactory>()
                .to_constant(factory)
                .bind::<Arc<dyn IRenderLoop>>()
                .to_constant(Rc::new(render_loop));
            Ok(())
        }
    }
}

/// Against the system: where the tests of the crate run on Windows. Each
/// prints what the system answered.
#[cfg(all(test, windows))]
mod system_tests {
    use super::imp::DxgiConnection;

    #[test]
    fn the_outputs_of_the_system_are_enumerated_and_one_is_waited_on() {
        let frequencies = DxgiConnection::get_all_monitor_frequencies();
        println!("the refresh rates of the monitors: {frequencies:?}");
        let output = DxgiConnection::get_best_output_to_v_wait_on();
        println!(
            "the output to wait on: {:?}",
            output.as_ref().map(|output| output.as_ref().map(|output| output.get_desc().map(|desc| desc.monitor.0)))
        );
        // The factory is created and the adapters are enumerated wherever
        // DXGI is; a session without a monitor of a known rate has no
        // output, and the timer then waits on the desktop window manager.
        let output = output.expect("the factory of DXGI and its adapters");
        if let Some(output) = output {
            let monitor = output.get_desc().expect("the description of an output").monitor.0;
            assert!(frequencies.get(&monitor).is_some_and(|frequency| *frequency > 0));
            let waited = output.wait_for_v_blank();
            println!("WaitForVBlank: {waited:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of the connection.
    use super::*;

    #[test]
    fn the_output_with_the_highest_refresh_rate_is_waited_on() {
        let frequencies: HashMap<isize, u32> = [(10, 60), (11, 144), (12, 144), (13, 0)].into_iter().collect();

        assert_eq!(None, best_output_to_v_wait_on(&[], &frequencies));
        assert_eq!(Some(0), best_output_to_v_wait_on(&[10], &frequencies));
        assert_eq!(Some(1), best_output_to_v_wait_on(&[10, 11], &frequencies));
        // The first of two with the same rate.
        assert_eq!(Some(1), best_output_to_v_wait_on(&[10, 11, 12], &frequencies));
        assert_eq!(Some(0), best_output_to_v_wait_on(&[12, 11, 10], &frequencies));
        // A monitor without a known frequency, and one that reports zero,
        // are never chosen.
        assert_eq!(None, best_output_to_v_wait_on(&[99], &frequencies));
        assert_eq!(None, best_output_to_v_wait_on(&[13], &frequencies));
        assert_eq!(Some(1), best_output_to_v_wait_on(&[99, 10, 13], &frequencies));
        assert_eq!(u32::MAX, ENUM_CURRENT_SETTINGS);
    }
}
