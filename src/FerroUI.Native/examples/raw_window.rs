//! Smoke test for the raw native bindings: opens a window, fills it with a
//! solid colour through the software render target and logs the events the
//! native side delivers.
//!
//! ```text
//! cargo run -p ferroui-native --example raw_window
//! FERROUI_SMOKE_EXIT_MS=1500 cargo run -p ferroui-native --example raw_window
//! ```
//!
//! With `FERROUI_SMOKE_EXIT_MS=<n>` the window is closed and the loop
//! cancelled after `n` milliseconds; the process then exits with code 0 only
//! if at least one `Paint` and one `Resized` callback were observed.

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("raw_window: this example only runs on macOS");
}

#[cfg(target_os = "macos")]
fn main() -> std::process::ExitCode {
    macos::run()
}

#[cfg(target_os = "macos")]
mod macos {
    use std::cell::{Cell, RefCell};
    use std::ffi::{c_void, CStr};
    use std::process::ExitCode;
    use std::rc::Rc;
    use std::sync::Mutex;

    use ferroui_microcom::{ComPtr, HResult};
    use ferroui_native::interop::*;

    /// Everything the callbacks need; shared through `Rc`.
    #[derive(Default)]
    struct State {
        threading: RefCell<Option<ComPtr<IFrnPlatformThreadingInterface>>>,
        cancel: RefCell<Option<ComPtr<IFrnLoopCancellation>>>,
        window: RefCell<Option<ComPtr<IFrnWindow>>>,
        render_target: RefCell<Option<ComPtr<IFrnSoftwareRenderTarget>>>,
        pixels: RefCell<Vec<u32>>,
        paints: Cell<u32>,
        resizes: Cell<u32>,
        mouse_moves: Cell<u32>,
        closed: Cell<bool>,
        posted: PostQueue,
    }

    impl State {
        fn cancel_loop(&self) {
            if let Some(cancel) = self.cancel.borrow().as_ref() {
                cancel.cancel();
            }
        }

        /// Fills the whole client area with one colour and presents it.
        fn paint(&self) -> Result<(), HResult> {
            let window = self.window.borrow();
            let target = self.render_target.borrow();
            let (Some(window), Some(target)) = (window.as_ref(), target.as_ref()) else {
                return Ok(());
            };
            let size = window.get_client_size()?;
            let scaling = window.get_scaling()?;
            let width = ((size.width * scaling).round() as i32).max(1);
            let height = ((size.height * scaling).round() as i32).max(1);

            // BGRA8888, little endian: 0xAARRGGBB. Hue shifts with every frame
            // so repaints are visible.
            let n = self.paints.get();
            let color = 0xFF00_0000 | (0x20 + (n * 8) % 0x80) << 16 | 0x70 << 8 | 0xC0;
            let mut pixels = self.pixels.borrow_mut();
            pixels.clear();
            pixels.resize(width as usize * height as usize, color);

            let mut fb = FrnFramebuffer {
                data: pixels.as_mut_ptr() as *mut c_void,
                width,
                height,
                stride: width * 4,
                dpi: FrnVector { x: 96.0 * scaling, y: 96.0 * scaling },
                pixel_format: FrnPixelFormat::kFrnBgra8888,
            };
            // SAFETY: `fb` and the pixel buffer outlive the call; the native
            // side copies the pixels before returning.
            unsafe { target.set_frame(&mut fb)? };

            self.paints.set(n + 1);
            if n < 3 {
                println!("Paint #{n}: {width}x{height} px (client {}x{}, scaling {scaling})", size.width, size.height);
            }
            Ok(())
        }
    }

    /// `IFrnDispatcher::Post` may be called from any thread, so the queue is
    /// locked; the callbacks themselves are only run on the UI thread.
    #[derive(Default)]
    struct PostQueue(Mutex<Vec<SendPtr>>);
    struct SendPtr(*mut IFrnActionCallback);
    // SAFETY: the pointer is only AddRef'ed/queued off-thread and used on the UI thread.
    unsafe impl Send for SendPtr {}

    struct GcHandleDeallocator;
    impl IFrnGCHandleDeallocatorCallbackImpl for GcHandleDeallocator {
        fn free_gc_handle(&self, handle: *mut c_void) {
            println!("FreeGCHandle({handle:p})");
        }
    }

    struct AppEvents(Rc<State>);
    impl IFrnApplicationEventsImpl for AppEvents {
        fn files_opened(&self, _args: Option<&IFrnStringArray>) {}
        fn urls_opened(&self, _urls: Option<&IFrnStringArray>) {}
        fn try_shutdown(&self, is_os_shutdown: bool) -> FrnShutdownReply {
            println!("TryShutdown(isOSShutdown={is_os_shutdown})");
            self.0.cancel_loop();
            FrnShutdownReply::ShutdownReplyDeferToManagedLoop
        }
        fn on_reopen(&self) {}
        fn on_hide(&self) {}
        fn on_unhide(&self) {}
        fn on_activate(&self) {
            println!("App activated");
        }
        fn on_deactivate(&self) {
            println!("App deactivated");
        }
        fn on_terminating(&self) {
            println!("App terminating");
        }
    }

    struct Dispatcher(Rc<State>);
    impl IFrnDispatcherImpl for Dispatcher {
        fn post(&self, cb: Option<&IFrnActionCallback>) {
            let Some(cb) = cb else { return };
            // Keep the callback alive until it has run.
            let owned = ComPtr::from_ref(cb).into_raw();
            self.0.posted.0.lock().unwrap().push(SendPtr(owned));
            if let Some(threading) = self.0.threading.borrow().as_ref() {
                threading.signal();
            }
        }
    }

    struct ThreadingEvents(Rc<State>);
    impl IFrnPlatformThreadingInterfaceEventsImpl for ThreadingEvents {
        fn signaled(&self) {
            let posted = std::mem::take(&mut *self.0.posted.0.lock().unwrap());
            for SendPtr(raw) in posted {
                // SAFETY: reference taken in `Dispatcher::post`.
                if let Some(cb) = unsafe { ComPtr::from_raw(raw) } {
                    cb.run();
                }
            }
        }

        fn timer(&self) {
            println!("Timer fired: closing window");
            let state = &self.0;
            if let Some(threading) = state.threading.borrow().as_ref() {
                threading.update_timer(-1);
            }
            let window = state.window.borrow().clone();
            if let Some(window) = window {
                if let Err(e) = window.close() {
                    println!("Close failed: {e}");
                }
            }
            state.cancel_loop();
        }

        fn ready_for_background_processing(&self) {}
    }

    struct WindowEvents(Rc<State>);

    impl IFrnTopLevelEventsImpl for WindowEvents {
        fn closed(&self) {
            println!("Closed");
            let state = &self.0;
            state.closed.set(true);
            // Break the window <-> events reference cycle.
            let _target = state.render_target.borrow_mut().take();
            let _window = state.window.borrow_mut().take();
            state.cancel_loop();
        }

        fn paint(&self) -> Result<(), HResult> {
            self.0.paint()
        }

        fn resized(&self, size: &FrnSize, reason: FrnPlatformResizeReason) {
            self.0.resizes.set(self.0.resizes.get() + 1);
            println!("Resized: {}x{} ({reason:?})", size.width, size.height);
        }

        fn raw_mouse_event(
            &self,
            type_: FrnRawMouseEventType,
            device_type: FrnPointerDeviceType,
            time_stamp: u64,
            modifiers: FrnInputModifiers,
            point: FrnPoint,
            delta: FrnVector,
            _pressure: f32,
            _x_tilt: f32,
            _y_tilt: f32,
        ) {
            if type_ == FrnRawMouseEventType::Move {
                // Log only every 20th move to keep the output readable.
                let n = self.0.mouse_moves.get();
                self.0.mouse_moves.set(n + 1);
                if n % 20 != 0 {
                    return;
                }
            }
            println!(
                "RawMouseEvent: {type_:?} {device_type:?} at ({:.1}, {:.1}) delta ({:.1}, {:.1}) modifiers {} t={time_stamp}",
                point.x, point.y, delta.x, delta.y, modifiers.0
            );
        }

        fn raw_key_event(
            &self,
            type_: FrnRawKeyEventType,
            _time_stamp: u64,
            modifiers: FrnInputModifiers,
            key: FrnKey,
            physical_key: FrnPhysicalKey,
            key_symbol: Option<&CStr>,
        ) -> bool {
            println!("RawKeyEvent: {type_:?} {key:?} {physical_key:?} symbol={key_symbol:?} modifiers {}", modifiers.0);
            false
        }

        fn raw_text_input_event(&self, _time_stamp: u64, text: Option<&CStr>) -> bool {
            println!("RawTextInputEvent: {text:?}");
            false
        }

        fn scaling_changed(&self, scaling: f64) {
            println!("ScalingChanged: {scaling}");
        }

        fn run_render_priority_jobs(&self) {}

        fn lost_focus(&self) {
            println!("LostFocus");
        }

        fn get_automation_peer(&self) -> Option<ComPtr<IFrnAutomationPeer>> {
            None
        }

        fn drag_event(
            &self,
            _type_: FrnDragEventType,
            _position: FrnPoint,
            _modifiers: FrnInputModifiers,
            _effects: FrnDragDropEffects,
            _clipboard: Option<&IFrnClipboard>,
            _data_transfer_handle: *mut c_void,
        ) -> FrnDragDropEffects {
            FrnDragDropEffects::None
        }
    }

    impl IFrnWindowBaseEventsImpl for WindowEvents {
        fn activated(&self) {
            println!("Activated");
        }
        fn deactivated(&self) {
            println!("Deactivated");
        }
        fn position_changed(&self, position: FrnPoint) {
            println!("PositionChanged: ({}, {})", position.x, position.y);
        }
    }

    impl IFrnWindowEventsImpl for WindowEvents {
        fn closing(&self) -> bool {
            println!("Closing");
            true
        }
        fn window_state_changed(&self, state: FrnWindowState) {
            println!("WindowStateChanged: {state:?}");
        }
        fn got_input_when_disabled(&self) {}
    }

    fn start(state: &Rc<State>, exit_ms: Option<i32>) -> Result<(), HResult> {
        let factory = create_ferro_native().ok_or(HResult::FAIL)?;

        // Callback objects: Rust values wrapped into COM objects.
        let deallocator = IFrnGCHandleDeallocatorCallback::from_impl(GcHandleDeallocator);
        let app_events = IFrnApplicationEvents::from_impl(AppEvents(state.clone()));
        let dispatcher = IFrnDispatcher::from_impl(Dispatcher(state.clone()));
        factory.initialize(Some(&deallocator), Some(&app_events), Some(&dispatcher))?;

        if let Some(options) = factory.get_mac_options() {
            options.set_show_in_dock(1)?;
            options.set_application_title(Some(c"FerroUI raw window"))?;
        }

        let threading = factory.create_platform_threading_interface()?.ok_or(HResult::POINTER)?;
        let threading_events = IFrnPlatformThreadingInterfaceEvents::from_impl(ThreadingEvents(state.clone()));
        threading.set_events(Some(&threading_events));
        println!("Loop thread: {}", threading.get_current_thread_is_loop_thread());
        *state.threading.borrow_mut() = Some(threading.clone());

        let window_events = IFrnWindowEvents::from_impl(WindowEvents(state.clone()));
        let window = factory.create_window(Some(&window_events))?.ok_or(HResult::POINTER)?;
        *state.window.borrow_mut() = Some(window.clone());
        *state.render_target.borrow_mut() = window.create_software_render_target()?;

        window.set_title(Some(c"FerroUI raw window"))?;
        window.resize(640.0, 400.0, FrnPlatformResizeReason::ResizeApplication)?;
        window.set_position(FrnPoint { x: 200.0, y: 200.0 })?;
        window.show(true, false)?;
        window.invalidate()?;
        println!(
            "Window shown: client size {:?}, frame size {:?}, scaling {}, state {:?}",
            window.get_client_size()?,
            window.get_frame_size()?,
            window.get_scaling()?,
            window.get_window_state()?
        );

        let cancel = threading.create_loop_cancellation().ok_or(HResult::POINTER)?;
        *state.cancel.borrow_mut() = Some(cancel.clone());
        if let Some(ms) = exit_ms {
            println!("Will exit after {ms} ms");
            threading.update_timer(ms);
        }
        drop(window);

        println!("Entering run loop");
        threading.run_loop(Some(&cancel));
        println!("Run loop exited");
        Ok(())
    }

    pub fn run() -> ExitCode {
        let exit_ms = std::env::var("FERROUI_SMOKE_EXIT_MS").ok().and_then(|v| v.parse::<i32>().ok());
        let state = Rc::new(State::default());
        if let Err(e) = start(&state, exit_ms) {
            eprintln!("raw_window failed: {e}");
            return ExitCode::FAILURE;
        }
        println!(
            "Summary: {} paint(s), {} resize(s), closed={}",
            state.paints.get(),
            state.resizes.get(),
            state.closed.get()
        );
        // Drop what the state still references before leaving.
        state.render_target.borrow_mut().take();
        state.window.borrow_mut().take();
        state.cancel.borrow_mut().take();
        state.threading.borrow_mut().take();

        if exit_ms.is_some() && (state.paints.get() == 0 || state.resizes.get() == 0) {
            eprintln!("smoke test failed: expected Paint and Resized callbacks");
            return ExitCode::FAILURE;
        }
        ExitCode::SUCCESS
    }
}
