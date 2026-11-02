//! Smoke test for the macOS platform implementation: brings the platform up,
//! creates a window through the windowing platform contract, draws into it
//! with the Skia render backend and logs the events the window delivers.
//!
//! ```text
//! cargo run -p ferroui-native --example platform_window            # software framebuffer
//! cargo run -p ferroui-native --example platform_window -- --metal # Metal through Graphite
//! RUSTFLAGS="--cfg ferro_skia_apple_ganesh" \
//!     cargo run -p ferroui-native --example platform_window -- --opengl # OpenGL (CGL) through Ganesh
//! FERROUI_SMOKE_EXIT_MS=1500 cargo run -p ferroui-native --example platform_window
//! ```
//!
//! `--opengl` needs the Skia backend built with Ganesh on OpenGL, which on
//! macOS is the configuration `ferro_skia_apple_ganesh` (docs/porting/apple-opengl.md);
//! the default build of Skia (Graphite on Metal) refuses an OpenGL context at
//! the first frame. With `--opengl` the first paint is preceded by a probe of
//! the OpenGL render target of the window: a frame cleared to a known colour
//! with OpenGL itself and read back before it is presented. With
//! `FERROUI_SMOKE_CAPTURE=<file.png>` the scene is also drawn into an
//! offscreen layer of the render backend (a GPU surface of the context the
//! window renders with) and written to the file: a picture of the frame the
//! application draws, not of the screen.
//!
//! With `FERROUI_SMOKE_EXIT_MS=<n>` the window is closed after `n`
//! milliseconds; the process then exits with code 0 only if at least one
//! paint and one resize were observed and the window reported being closed.
//!
//! With `FERROUI_SMOKE_INPUT=1` the example also posts synthetic events
//! (mouse move, click, key down/up) to its own window through the
//! application event queue, to exercise the input path without a user. The
//! events are posted in-process, which needs no system permission and does
//! not move the real pointer.
//!
//! Everything here goes through the ported platform and the toolkit
//! contracts; there is no raw interop. (The synthetic events are made with
//! the Objective-C runtime, from the window handle the platform exposes.)

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("platform_window: this example only runs on macOS");
}

#[cfg(target_os = "macos")]
fn main() -> std::process::ExitCode {
    macos::run()
}

#[cfg(target_os = "macos")]
mod macos {
    use ferroui_base::input::raw::{
        IRawInputEventArgs, RawKeyEventArgs, RawMouseWheelEventArgs, RawPointerEventArgs, RawPointerEventType,
        RawPointerGestureEventArgs, RawTextInputEventArgs,
    };
    use ferroui_base::input::{FocusManager, IInputRoot, InputElement};
    use ferroui_base::media::immutable::{
        ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutablePen, ImmutableSolidColorBrush,
    };
    use ferroui_base::media::{BoxShadows, Color, Colors, GradientSpreadMethod};
    use ferroui_base::media::imaging::{BitmapEncoderOptions, PngBitmapEncoderOptions};
    use ferroui_base::platform::{
        IDrawingContextImpl, IPlatformGraphics, IPlatformGraphicsContext, IPlatformRenderInterface,
        IPlatformRenderInterfaceContext, IRenderTarget, RenderTargetSceneInfo,
    };
    use ferroui_base::rendering::composition::CompositionTransparencyLevel;
    use ferroui_base::threading::{CancellationTokenSource, Dispatcher, DispatcherPriority, DispatcherTimer};
    use ferroui_base::{
        FerroLocator, LocatorExtensions, Matrix, PixelPoint, PixelSize, Rect, Ref, RelativePoint, RelativeUnit,
        RoundedRect, Size, Vector,
    };
    use ferroui_controls::platform::{IWindowImpl, IWindowingPlatform};
    use ferroui_controls::WindowResizeReason;
    use ferroui_native::{FerroNativePlatform, FerroNativePlatformOptions, FerroNativeRenderingMode};
    use ferroui_opengl::gl_consts::{GL_COLOR_BUFFER_BIT, GL_RGBA, GL_UNSIGNED_BYTE};
    use ferroui_opengl::surfaces::try_get_gl_surface;
    use ferroui_opengl::IGlContext;
    use ferroui_skia::SkiaPlatform;
    use std::cell::{Cell, RefCell};
    use std::process::ExitCode;
    use std::rc::Rc;
    use std::time::Duration;

    /// The smallest input root: one input element and no focus manager.
    struct InputRoot {
        root: Ref<InputElement>,
        pointer_over_element: RefCell<Option<Ref<InputElement>>>,
        cursor_element: RefCell<Option<Ref<InputElement>>>,
    }

    impl IInputRoot for InputRoot {
        fn focus_manager(&self) -> Option<Rc<FocusManager>> {
            None
        }

        fn pointer_over_element(&self) -> Option<Ref<InputElement>> {
            self.pointer_over_element.borrow().clone()
        }

        fn set_pointer_over_element(&self, value: Option<Ref<InputElement>>) {
            let old = self.pointer_over_element.replace(value);
            drop(old);
        }

        fn cursor_element(&self) -> Option<Ref<InputElement>> {
            self.cursor_element.borrow().clone()
        }

        fn set_cursor_element(&self, value: Option<Ref<InputElement>>) {
            let old = self.cursor_element.replace(value);
            drop(old);
        }

        fn root_element(&self) -> Ref<InputElement> {
            self.root.clone()
        }

        fn focus_root(&self) -> Ref<InputElement> {
            self.root.clone()
        }

        fn pointer_over_invalidated(&self) {}
    }

    /// What the callbacks share.
    struct State {
        window: Rc<dyn IWindowImpl>,
        context: Rc<dyn IPlatformRenderInterfaceContext>,
        /// The OpenGL context the window renders with (`--opengl`).
        gl_context: Option<Rc<dyn IGlContext>>,
        /// Whether the probe of the OpenGL render target read back the colour
        /// it cleared the frame to.
        gl_probe: Cell<Option<bool>>,
        /// Where `FERROUI_SMOKE_CAPTURE` asks the offscreen picture to go.
        capture: Option<String>,
        captured: Cell<Option<bool>>,
        render_target: RefCell<Option<Rc<dyn IRenderTarget>>>,
        paints: Cell<u32>,
        resizes: Cell<u32>,
        inputs: Cell<u32>,
        mouse_moves: Cell<u32>,
        closed: Cell<bool>,
    }

    impl State {
        fn paint(&self, rect: Rect) {
            let n = self.paints.get();
            let client_size = self.window.client_size();
            let scaling = self.window.render_scaling();
            let pixel_size = PixelSize::from_size(client_size, scaling);
            if pixel_size.width <= 0 || pixel_size.height <= 0 {
                return;
            }

            if self.gl_probe.get().is_none() {
                if let Some(gl_context) = &self.gl_context {
                    self.gl_probe.set(Some(self.probe_gl_render_target(gl_context, pixel_size, scaling)));
                }
            }

            let render_target = {
                let mut slot = self.render_target.borrow_mut();
                match slot.as_ref() {
                    Some(render_target) => render_target.clone(),
                    None => {
                        let render_target = self.context.create_render_target(&self.window.surfaces());
                        println!("Render target created: {:?}", render_target.properties());
                        *slot = Some(render_target.clone());
                        render_target
                    }
                }
            };

            let scene_info = RenderTargetSceneInfo::new(pixel_size, scaling, CompositionTransparencyLevel::None);
            let (mut context, _properties) = render_target.create_drawing_context(&scene_info);

            draw_scene(&mut *context, client_size, scaling);
            context.dispose();

            if self.captured.get().is_none() {
                if let Some(path) = &self.capture {
                    self.captured.set(Some(self.capture_offscreen(path, pixel_size, client_size, scaling)));
                }
            }

            self.paints.set(n + 1);
            if n < 3 {
                println!(
                    "Paint #{n}: {}x{} px (client {}x{}, scaling {scaling}, dirty {}x{})",
                    pixel_size.width, pixel_size.height, client_size.width, client_size.height, rect.width, rect.height
                );
            }
        }

        /// Clears a frame of the OpenGL render target of the window to a
        /// known colour with OpenGL itself and reads a pixel back before the
        /// frame is presented. The render target the backend creates next
        /// replaces this one in the view.
        fn probe_gl_render_target(&self, gl_context: &Rc<dyn IGlContext>, pixel_size: PixelSize, scaling: f64) -> bool {
            let surfaces = self.window.surfaces();
            let Some(surface) = surfaces.iter().find_map(|surface| try_get_gl_surface(&**surface)) else {
                println!("OpenGL probe: the window has no OpenGL surface");
                return false;
            };
            let target = surface.create_gl_render_target(gl_context);
            let scene_info = RenderTargetSceneInfo::new(pixel_size, scaling, CompositionTransparencyLevel::None);
            let session = target.begin_draw(&scene_info);
            let gl = session.context().gl_interface();
            gl.clear_color(0.2, 0.4, 0.6, 1.0);
            gl.clear(GL_COLOR_BUFFER_BIT);
            let mut pixel = [0u8; 4];
            let size = session.size();
            // SAFETY: the storage holds the one pixel of RGBA bytes asked
            // for; the context of the session is current on this thread.
            unsafe { gl.read_pixels(size.width / 2, size.height / 2, 1, 1, GL_RGBA, GL_UNSIGNED_BYTE, pixel.as_mut_ptr().cast()) };
            println!(
                "OpenGL probe: {:?}, session {}x{} px, scaling {}, y flipped {}, read back {pixel:?}",
                gl_context.version(),
                size.width,
                size.height,
                session.scaling(),
                session.is_y_flipped()
            );
            session.dispose();
            target.dispose();
            let expected = [51u8, 102, 153, 255];
            pixel.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1)
        }

        /// Draws the scene into an offscreen layer of the render backend and
        /// writes the layer to `path` as PNG.
        fn capture_offscreen(&self, path: &str, pixel_size: PixelSize, client_size: Size, scaling: f64) -> bool {
            let layer = self.context.create_offscreen_render_target(pixel_size, Vector::new(scaling, scaling), true);
            let mut context = layer.create_drawing_context();
            draw_scene(&mut *context, client_size, scaling);
            context.dispose();
            // The pixels of a GPU layer are read through its context, which
            // has to be current for that.
            let current = self.gl_context.as_ref().map(|gl_context| gl_context.make_current());
            let result = std::fs::File::create(path).and_then(|mut file| {
                layer.save(&mut file, &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::DEFAULT))
            });
            if let Some(current) = current {
                current.dispose();
            }
            match result {
                Ok(()) => {
                    println!("Capture: {path} ({}x{} px, an offscreen layer of the render backend)", pixel_size.width, pixel_size.height);
                    true
                }
                Err(error) => {
                    println!("Capture: {path} could not be written: {error}");
                    false
                }
            }
        }

        fn input(&self, e: &dyn IRawInputEventArgs) {
            self.inputs.set(self.inputs.get() + 1);
            if let Some(e) = e.downcast_ref::<RawKeyEventArgs>() {
                println!(
                    "Input: key {:?} {:?} physical {:?} symbol {:?} modifiers {:?} t={}",
                    e.type_(),
                    e.key(),
                    e.physical_key(),
                    e.key_symbol(),
                    e.modifiers(),
                    e.timestamp()
                );
            } else if let Some(e) = e.downcast_ref::<RawTextInputEventArgs>() {
                println!("Input: text {:?}", e.text());
            } else if let Some(e) = e.downcast_ref::<RawMouseWheelEventArgs>() {
                println!("Input: wheel at {:?} delta {:?} modifiers {:?}", e.position(), e.delta(), e.input_modifiers());
            } else if let Some(e) = e.downcast_ref::<RawPointerGestureEventArgs>() {
                println!("Input: gesture {:?} at {:?} delta {:?}", e.type_(), e.position(), e.delta());
            } else if let Some(e) = e.downcast_ref::<RawPointerEventArgs>() {
                if e.type_() == RawPointerEventType::Move {
                    // Log only every 20th move to keep the output readable.
                    let n = self.mouse_moves.get();
                    self.mouse_moves.set(n + 1);
                    if n % 20 != 0 {
                        return;
                    }
                }
                println!(
                    "Input: pointer {:?} at ({:.1}, {:.1}) modifiers {:?} t={}",
                    e.type_(),
                    e.position().x,
                    e.position().y,
                    e.input_modifiers(),
                    e.timestamp()
                );
            } else {
                println!("Input: other raw event");
            }
        }
    }

    pub fn run() -> ExitCode {
        let metal = std::env::args().any(|arg| arg == "--metal");
        let opengl = std::env::args().any(|arg| arg == "--opengl");
        let exit_ms = std::env::var("FERROUI_SMOKE_EXIT_MS").ok().and_then(|v| v.parse::<u64>().ok());
        let mode = if metal {
            "metal"
        } else if opengl {
            "opengl"
        } else {
            "software"
        };

        // --- platform and render backend -----------------------------------
        let options = FerroNativePlatformOptions {
            rendering_mode: if metal {
                vec![FerroNativeRenderingMode::Metal]
            } else if opengl {
                vec![FerroNativeRenderingMode::OpenGl]
            } else {
                vec![FerroNativeRenderingMode::Software]
            },
            ..FerroNativePlatformOptions::default()
        };
        let platform = FerroNativePlatform::initialize(options);
        platform.setup_application_name_with(Some("FerroUI platform window"));
        SkiaPlatform::initialize();

        let locator = FerroLocator::current();
        let graphics = locator.get_service::<std::sync::Arc<dyn IPlatformGraphics>>();
        println!("Mode: {mode}, platform graphics: {}", graphics.is_some());
        if (metal || opengl) && graphics.is_none() {
            eprintln!("platform_window failed: {mode} is not available");
            return ExitCode::FAILURE;
        }

        let graphics_context: Option<Rc<dyn IPlatformGraphicsContext>> =
            graphics.as_ref().map(|graphics| graphics.create_context());
        let gl_context = graphics_context.as_ref().and_then(|context| {
            let features: &dyn ferroui_base::platform::IOptionalFeatureProvider = &**context;
            features.try_get::<dyn IGlContext>()
        });
        let render_interface = locator.get_required_service::<dyn IPlatformRenderInterface>();
        let context = render_interface.create_backend_context(graphics_context.clone());

        // --- window ---------------------------------------------------------
        let windowing_platform = locator.get_required_service::<dyn IWindowingPlatform>();
        let window = windowing_platform.create_window();
        let state = Rc::new(State {
            window: window.clone(),
            context,
            gl_context,
            gl_probe: Cell::new(None),
            capture: std::env::var("FERROUI_SMOKE_CAPTURE").ok(),
            captured: Cell::new(None),
            render_target: RefCell::new(None),
            paints: Cell::new(0),
            resizes: Cell::new(0),
            inputs: Cell::new(0),
            mouse_moves: Cell::new(0),
            closed: Cell::new(false),
        });
        let loop_cancellation = CancellationTokenSource::new();

        window.set_input_root(Rc::new(InputRoot {
            root: InputElement::new(),
            pointer_over_element: RefCell::new(None),
            cursor_element: RefCell::new(None),
        }));

        // The callbacks hold the state weakly: the state owns the window.
        let weak = Rc::downgrade(&state);
        window.set_paint(Some(Rc::new(move |rect| {
            if let Some(state) = weak.upgrade() {
                state.paint(rect);
            }
        })));
        let weak = Rc::downgrade(&state);
        window.set_resized(Some(Rc::new(move |size: Size, reason: WindowResizeReason| {
            if let Some(state) = weak.upgrade() {
                state.resizes.set(state.resizes.get() + 1);
                println!("Resized: {}x{} ({reason:?})", size.width, size.height);
            }
        })));
        let weak = Rc::downgrade(&state);
        window.set_input(Some(Rc::new(move |e: Rc<dyn IRawInputEventArgs>| {
            if let Some(state) = weak.upgrade() {
                state.input(&*e);
            }
        })));
        window.set_scaling_changed(Some(Rc::new(|scaling| println!("ScalingChanged: {scaling}"))));
        window.set_position_changed(Some(Rc::new(|position: PixelPoint| {
            println!("PositionChanged: ({}, {})", position.x, position.y)
        })));
        window.set_activated(Some(Rc::new(|| println!("Activated"))));
        window.set_deactivated(Some(Rc::new(|| println!("Deactivated"))));
        window.set_lost_focus(Some(Rc::new(|| println!("LostFocus"))));
        window.set_window_state_changed(Some(Rc::new(|state| println!("WindowStateChanged: {state:?}"))));
        window.set_closing(Some(Rc::new(|reason| {
            println!("Closing ({reason:?})");
            // `true` lets the window close.
            true
        })));
        let weak = Rc::downgrade(&state);
        let cancel = loop_cancellation.clone();
        window.set_closed(Some(Rc::new(move || {
            println!("Closed");
            if let Some(state) = weak.upgrade() {
                state.closed.set(true);
                // The render target belongs to the window that is going away.
                let render_target = state.render_target.borrow_mut().take();
                if let Some(render_target) = render_target {
                    render_target.dispose();
                }
            }
            cancel.cancel();
        })));

        window.set_title(Some(if metal {
            "FerroUI platform window (Metal)"
        } else if opengl {
            "FerroUI platform window (OpenGL)"
        } else {
            "FerroUI platform window"
        }));
        window.resize(Size::new(640.0, 400.0), WindowResizeReason::Application);
        window.move_(PixelPoint::new(200, 200));
        window.show(true, false);
        println!(
            "Window shown: client size {:?}, frame size {:?}, scaling {}, position {:?}, handle {:?}",
            window.client_size(),
            window.frame_size(),
            window.render_scaling(),
            window.position(),
            window.handle().and_then(|handle| handle.handle_descriptor().map(str::to_owned)),
        );
        // The dirty state of a freshly shown window is the platform's
        // business: it asks for the first paint itself.

        let _exit_timer = exit_ms.map(|ms| {
            println!("Will exit after {ms} ms");
            let window = window.clone();
            DispatcherTimer::run_once(
                move || {
                    println!("Timer fired: closing window");
                    window.dispose();
                },
                Duration::from_millis(ms),
                DispatcherPriority::NORMAL,
            )
            .detach()
        });

        let _input_timer = std::env::var_os("FERROUI_SMOKE_INPUT").map(|_| {
            let ns_window = window.handle().map_or(0, |handle| handle.handle());
            DispatcherTimer::run_once(
                move || {
                    println!("Posting synthetic input");
                    synthetic_input::post_all(ns_window);
                },
                Duration::from_millis(700),
                DispatcherPriority::NORMAL,
            )
            .detach()
        });

        // --- main loop ------------------------------------------------------
        println!("Entering main loop");
        Dispatcher::ui_thread().main_loop(&loop_cancellation.token());
        println!("Main loop exited");

        println!(
            "Summary: {} paint(s), {} resize(s), {} input event(s), closed={}",
            state.paints.get(),
            state.resizes.get(),
            state.inputs.get(),
            state.closed.get()
        );

        // Release what is still alive, in dependency order.
        let (paints, resizes, closed) = (state.paints.get(), state.resizes.get(), state.closed.get());
        let (gl_probe, captured) = (state.gl_probe.get(), state.captured.get());
        println!("OpenGL probe: {gl_probe:?}; capture: {captured:?}");
        // The backend context gives its GPU resources back through the
        // graphics context, which has to be current for that (OpenGL).
        let current = graphics_context.as_ref().map(|graphics_context| graphics_context.ensure_current());
        state.context.dispose();
        if let Some(current) = current {
            current.dispose();
        }
        drop(state);
        drop(window);
        if let Some(graphics_context) = graphics_context {
            graphics_context.dispose();
        }
        platform.dispose();

        if exit_ms.is_some() && (paints == 0 || resizes == 0 || !closed) {
            eprintln!("smoke test failed: expected paint, resize and closed callbacks");
            return ExitCode::FAILURE;
        }
        if exit_ms.is_some() && opengl && gl_probe != Some(true) {
            eprintln!("smoke test failed: the OpenGL render target did not read back the colour it was cleared to");
            return ExitCode::FAILURE;
        }
        if exit_ms.is_some() && captured == Some(false) {
            eprintln!("smoke test failed: the offscreen picture could not be written");
            return ExitCode::FAILURE;
        }
        ExitCode::SUCCESS
    }

    /// The scene of the example: a gradient across the client area and a
    /// rounded card in the middle.
    fn draw_scene(context: &mut dyn IDrawingContextImpl, client_size: Size, scaling: f64) {
        // The backend draws in device pixels; the scene is laid out in
        // logical units.
        context.clear(Colors::WHITE);
        context.set_transform(Matrix::create_scale(scaling, scaling));

        let (width, height) = (client_size.width, client_size.height);

        // A gradient across the whole client area...
        let gradient = ImmutableLinearGradientBrush::new(
            &[
                ImmutableGradientStop::new(0.0, Color::from_rgb(0x1e, 0x3a, 0x8a)),
                ImmutableGradientStop::new(0.5, Color::from_rgb(0x7c, 0x3a, 0xed)),
                ImmutableGradientStop::new(1.0, Color::from_rgb(0xf5, 0x9e, 0x0b)),
            ],
            1.0,
            None,
            None,
            GradientSpreadMethod::Pad,
            Some(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative)),
            Some(RelativePoint::new(1.0, 1.0, RelativeUnit::Relative)),
            None,
        );
        context.draw_rectangle(
            Some(&gradient),
            None,
            RoundedRect::from_rect(Rect::new(0.0, 0.0, width, height)),
            &BoxShadows::default(),
        );

        // ...and a rounded rectangle with an outline in the middle.
        let fill = ImmutableSolidColorBrush::new(Color::from_argb(0xe6, 0xff, 0xff, 0xff));
        let pen = ImmutablePen::from_uint32(0xff11_1827, 3.0);
        let card = Rect::new(width * 0.2, height * 0.25, width * 0.6, height * 0.5);
        context.draw_rectangle(Some(&fill), Some(&pen), RoundedRect::from_radius(card, 24.0), &BoxShadows::default());
    }

    /// Synthetic input for the smoke test: events are created with AppKit
    /// and posted to the event queue of this application only.
    mod synthetic_input {
        use std::ffi::{c_char, c_void, CStr};

        type Id = *mut c_void;
        type Sel = *mut c_void;

        #[repr(C)]
        #[derive(Clone, Copy)]
        struct NSPoint {
            x: f64,
            y: f64,
        }

        #[link(name = "objc")]
        extern "C" {
            fn sel_registerName(name: *const c_char) -> Sel;
            fn objc_getClass(name: *const c_char) -> Id;
            fn objc_msgSend();
        }

        const LEFT_MOUSE_DOWN: usize = 1;
        const LEFT_MOUSE_UP: usize = 2;
        const MOUSE_MOVED: usize = 5;
        const KEY_DOWN: usize = 10;
        const KEY_UP: usize = 11;

        fn sel(name: &CStr) -> Sel {
            // SAFETY: `name` is NUL-terminated.
            unsafe { sel_registerName(name.as_ptr()) }
        }

        fn class(name: &CStr) -> Id {
            // SAFETY: `name` is NUL-terminated.
            unsafe { objc_getClass(name.as_ptr()) }
        }

        // SAFETY (all `send_*`): `objc_msgSend` is called through a pointer
        // with the C signature of the method being sent, as the Objective-C
        // ABI requires; receivers are live objects or classes.
        unsafe fn send_id(receiver: Id, selector: Sel) -> Id {
            let f: unsafe extern "C" fn(Id, Sel) -> Id = std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            f(receiver, selector)
        }

        unsafe fn send_isize(receiver: Id, selector: Sel) -> isize {
            let f: unsafe extern "C" fn(Id, Sel) -> isize = std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            f(receiver, selector)
        }

        unsafe fn send_f64(receiver: Id, selector: Sel) -> f64 {
            let f: unsafe extern "C" fn(Id, Sel) -> f64 = std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            f(receiver, selector)
        }

        unsafe fn ns_string(s: &CStr) -> Id {
            let f: unsafe extern "C" fn(Id, Sel, *const c_char) -> Id =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            f(class(c"NSString"), sel(c"stringWithUTF8String:"), s.as_ptr())
        }

        unsafe fn uptime() -> f64 {
            let process_info = send_id(class(c"NSProcessInfo"), sel(c"processInfo"));
            send_f64(process_info, sel(c"systemUptime"))
        }

        unsafe fn post(event: Id) {
            if event.is_null() {
                println!("Synthetic event could not be created");
                return;
            }
            let app = send_id(class(c"NSApplication"), sel(c"sharedApplication"));
            let f: unsafe extern "C" fn(Id, Sel, Id, bool) =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            f(app, sel(c"postEvent:atStart:"), event, false);
        }

        unsafe fn mouse(window_number: isize, type_: usize, location: NSPoint, click_count: isize, pressure: f32) {
            #[allow(clippy::type_complexity)]
            let f: unsafe extern "C" fn(Id, Sel, usize, NSPoint, usize, f64, isize, Id, isize, isize, f32) -> Id =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            let event = f(
                class(c"NSEvent"),
                sel(c"mouseEventWithType:location:modifierFlags:timestamp:windowNumber:context:eventNumber:clickCount:pressure:"),
                type_,
                location,
                0,
                uptime(),
                window_number,
                std::ptr::null_mut(),
                0,
                click_count,
                pressure,
            );
            post(event);
        }

        unsafe fn key(window_number: isize, type_: usize, characters: &CStr, key_code: u16) {
            #[allow(clippy::type_complexity)]
            let f: unsafe extern "C" fn(Id, Sel, usize, NSPoint, usize, f64, isize, Id, Id, Id, bool, u16) -> Id =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            let characters = ns_string(characters);
            let event = f(
                class(c"NSEvent"),
                sel(c"keyEventWithType:location:modifierFlags:timestamp:windowNumber:context:characters:charactersIgnoringModifiers:isARepeat:keyCode:"),
                type_,
                NSPoint { x: 0.0, y: 0.0 },
                0,
                uptime(),
                window_number,
                std::ptr::null_mut(),
                characters,
                characters,
                false,
                key_code,
            );
            post(event);
        }

        /// Posts a mouse move, a left click and an "a" key press to the
        /// window behind the `NSWindow` handle.
        pub fn post_all(ns_window: isize) {
            if ns_window == 0 {
                println!("No NSWindow handle: synthetic input skipped");
                return;
            }
            // SAFETY: the handle is the live NSWindow of the example window;
            // the messages sent are plain AppKit API (see `send_*`).
            unsafe {
                let window_number = send_isize(ns_window as Id, sel(c"windowNumber"));
                // Window coordinates: origin at the bottom-left corner.
                let location = NSPoint { x: 120.0, y: 90.0 };
                mouse(window_number, MOUSE_MOVED, location, 0, 0.0);
                mouse(window_number, LEFT_MOUSE_DOWN, location, 1, 1.0);
                mouse(window_number, LEFT_MOUSE_UP, location, 1, 0.0);
                // Key code 0 is the "A" key of an ANSI keyboard.
                key(window_number, KEY_DOWN, c"a", 0);
                key(window_number, KEY_UP, c"a", 0);
            }
        }
    }
}
