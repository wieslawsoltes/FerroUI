//! A window of the Windows platform backend: the platform is brought up, a
//! window is created through the windowing contract, and frames of shapes
//! are drawn into its software surface through the render interface.
//!
//! ```text
//! cargo run -p ferroui-win32 --example win32_window              # stays until the window is closed
//! cargo run -p ferroui-win32 --example win32_window -- --smoke   # checks the backend and exits
//! ```
//!
//! With `--smoke` the example runs without a user: it draws a number of
//! frames on a timer (the shapes move from frame to frame), resizes the
//! window half way, maximizes and restores it, posts key and mouse messages
//! to its own window, puts text on the clipboard and reads it back, wakes
//! the message loop from another thread, and closes the window. Every step
//! prints what it saw and whether that is what the backend promises; the
//! exit code is 0 only if every frame was drawn and every check passed.
//! This run is how the backend is proved on a Windows machine without a
//! person, the CI runner first of all, so its output is meant to be read.
//!
//! The renderer is the Vello backend in its CPU mode: it has no C or C++
//! sources, so the example is checked for a Windows target on any host. The
//! window is drawn through the framebuffer surface with
//! `--rendering software` (the default).
//!
//! With `--rendering angle` (the crate built with its feature `angle`) the
//! platform graphics are ANGLE on Direct3D 11, and the frames are drawn
//! with OpenGL ES itself through what the backend gives a renderer: the
//! platform graphics of the services, a context of them, the OpenGL surface
//! among the surfaces of the window, and its render target. Every frame is
//! read back with `glReadPixels` before it is presented and compared with
//! what was drawn, so the path to the GPU is checked, not assumed.
//! `--angle-probe` adds what ANGLE reports of itself.

#[cfg(not(windows))]
fn main() {
    eprintln!("win32_window: this example only runs on Windows");
}

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    windows::run()
}

#[cfg(windows)]
mod windows {
    use ferroui_base::input::platform::IClipboardImpl;
    use ferroui_base::input::raw::{
        IRawInputEventArgs, RawKeyEventArgs, RawKeyEventType, RawMouseWheelEventArgs, RawPointerEventArgs,
        RawPointerEventType, RawTextInputEventArgs,
    };
    use ferroui_base::input::{
        DataTransfer, DataTransferExtensions, DataTransferItem, FocusManager, IDataTransfer, IInputRoot, InputElement, Key,
        PhysicalKey,
    };
    use ferroui_base::logging::LogArea;
    use ferroui_base::media::immutable::{
        ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutablePen, ImmutableSolidColorBrush,
    };
    use ferroui_base::media::{BoxShadows, Color, Colors, GradientSpreadMethod, PenLineCap, PenLineJoin};
    use ferroui_base::platform::{
        IPlatformRenderInterface, IPlatformRenderInterfaceContext, IRenderTarget, RenderTargetSceneInfo,
    };
    use ferroui_base::rendering::composition::CompositionTransparencyLevel;
    use ferroui_base::threading::{CancellationTokenSource, Dispatcher, DispatcherPriority, DispatcherTimer};
    use ferroui_base::{
        FerroLocator, LocatorExtensions, Matrix, PixelPoint, PixelSize, Point, Rect, Ref, RelativePoint, RelativeUnit,
        RoundedRect, Size,
    };
    use ferroui_base::platform::surfaces::IPlatformRenderSurface;
    use ferroui_base::platform::{IPlatformGraphics, IPlatformGraphicsContext};
    use ferroui_controls::platform::{IScreenImpl, IWindowImpl, IWindowingPlatform};
    use ferroui_opengl::gl_consts::{GL_COLOR_BUFFER_BIT, GL_RGBA, GL_SCISSOR_TEST, GL_UNSIGNED_BYTE};
    use ferroui_opengl::surfaces::{try_get_gl_surface, IGlPlatformSurfaceRenderTarget};
    use ferroui_opengl::IGlContext;
    use ferroui_controls::{WindowResizeReason, WindowState};
    use ferroui_vello::{VelloOptions, VelloPlatform, VelloRenderingMode};
    use ferroui_win32::interop::unmanaged_methods::{post_message, WindowsMessage};
    use ferroui_win32::{Win32CompositionMode, Win32Platform, Win32PlatformOptions, Win32RenderingMode};
    use std::cell::{Cell, RefCell};
    use std::future::Future;
    use std::pin::Pin;
    use std::process::ExitCode;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::task::{Context, Poll, Waker};
    use std::time::Duration;

    /// The frames a smoke run draws.
    const SMOKE_FRAMES: u32 = 12;

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

    /// The checks of a smoke run: each is printed when it is made, and the
    /// ones that failed are listed at the end.
    #[derive(Default)]
    struct Report {
        passed: Cell<u32>,
        failures: RefCell<Vec<String>>,
    }

    impl Report {
        fn check(&self, name: &str, ok: bool, detail: impl AsRef<str>) {
            if ok {
                self.passed.set(self.passed.get() + 1);
                println!("[ ok ] {name}: {}", detail.as_ref());
            } else {
                println!("[FAIL] {name}: {}", detail.as_ref());
                self.failures.borrow_mut().push(format!("{name}: {}", detail.as_ref()));
            }
        }

        /// Something the run observed that is not a promise of the backend
        /// (it depends on the session the run is in).
        fn info(&self, name: &str, detail: impl AsRef<str>) {
            println!("[info] {name}: {}", detail.as_ref());
        }
    }

    /// The raw input a smoke run posted to its own window and saw arrive.
    #[derive(Default)]
    struct SeenInput {
        key_down: RefCell<Option<(Key, PhysicalKey, Option<String>)>>,
        key_up: Cell<bool>,
        text: RefCell<Option<String>>,
        mouse_move: Cell<Option<Point>>,
        left_down: Cell<Option<Point>>,
        left_up: Cell<bool>,
        wheel: Cell<Option<f64>>,
    }

    /// What draws the frames with OpenGL ES in the rendering mode ANGLE.
    struct GlPainter {
        context: Rc<dyn IPlatformGraphicsContext>,
        render_target: Rc<dyn IGlPlatformSurfaceRenderTarget>,
        /// The frames whose pixels were not the ones drawn, with what was
        /// read.
        mismatches: RefCell<Vec<String>>,
        /// The frame that was read back last: its size, the pixels that
        /// are not blank, a checksum.
        last_frame: Cell<Option<(PixelSize, usize, u64)>>,
    }

    type Scissor = unsafe extern "system" fn(x: i32, y: i32, width: i32, height: i32);

    impl GlPainter {
        /// The context and the render target of the window, through the
        /// services of the platform and the surfaces of the window.
        fn new(window: &Rc<dyn IWindowImpl>, report: &Report) -> Option<GlPainter> {
            let Some(graphics) = FerroLocator::current().get_service::<Arc<dyn IPlatformGraphics>>() else {
                report.check("platform graphics", false, "the platform registered no platform graphics");
                return None;
            };
            let surfaces: Vec<Arc<dyn IPlatformRenderSurface>> = window.surfaces();
            let Some(gl_surface) = surfaces.iter().find_map(|surface| try_get_gl_surface(&**surface)) else {
                report.check("OpenGL surface", false, "the window has no OpenGL surface among its surfaces");
                return None;
            };
            let context = graphics.create_context();
            let features: &dyn ferroui_base::platform::IOptionalFeatureProvider = &*context;
            let Some(gl_context) = features.try_get::<dyn IGlContext>() else {
                report.check("OpenGL context", false, "the context of the platform graphics is not an OpenGL context");
                return None;
            };
            let current = gl_context.make_current();
            let gl = gl_context.gl_interface();
            report.check(
                "OpenGL context",
                true,
                format!(
                    "{:?} by {:?} on {:?}; OpenGL ES {}.{}, {} sample(s), {} stencil bit(s)",
                    gl.version(),
                    gl.vendor(),
                    gl.renderer(),
                    gl_context.version().major(),
                    gl_context.version().minor(),
                    gl_context.sample_count(),
                    gl_context.stencil_size()
                ),
            );
            current.dispose();
            let render_target = gl_surface.create_gl_render_target(&gl_context);
            Some(GlPainter { context, render_target, mismatches: RefCell::new(Vec::new()), last_frame: Cell::new(None) })
        }

        /// Draws a frame of rectangles (the number of the frame moves one of
        /// them), reads it back and presents it.
        fn paint(&self, frame: u32, pixel_size: PixelSize, scaling: f64) {
            let scene_info = RenderTargetSceneInfo::new(pixel_size, scaling, CompositionTransparencyLevel::None);
            let session = self.render_target.begin_draw(&scene_info);
            let size = session.size();
            let (width, height) = (size.width, size.height);
            let gl = session.context().gl_interface();
            let scissor = gl.get_proc_address("glScissor");
            if scissor.is_null() {
                self.mismatches.borrow_mut().push("the context has no glScissor".to_string());
                session.dispose();
                return;
            }
            // SAFETY: the address is the entry point `glScissor` of the
            // context, whose signature is the one of the type.
            let scissor = unsafe { std::mem::transmute::<*const std::ffi::c_void, Scissor>(scissor) };

            // A rectangle from the top-left corner, as the window sees it:
            // the rows of OpenGL count from the bottom.
            let fill = |x: i32, y: i32, w: i32, h: i32, color: [u8; 3]| {
                // SAFETY: plain values; the context of the session is
                // current.
                unsafe { scissor(x, height - y - h, w, h) };
                gl.clear_color(f32::from(color[0]) / 255.0, f32::from(color[1]) / 255.0, f32::from(color[2]) / 255.0, 1.0);
                gl.clear(GL_COLOR_BUFFER_BIT);
            };

            gl.viewport(0, 0, width, height);
            gl.disable(GL_SCISSOR_TEST);
            gl.clear_color(1.0, 1.0, 1.0, 1.0);
            gl.clear(GL_COLOR_BUFFER_BIT);
            gl.enable(GL_SCISSOR_TEST);
            // Bands from blue to amber, a card, and a rectangle that moves
            // with every frame and does not come back.
            let bands = 8;
            for band in 0..bands {
                let t = band * 255 / (bands - 1);
                let color = [(0x1e + (0xf5 - 0x1e) * t / 255) as u8, (0x3a + (0x9e - 0x3a) * t / 255) as u8, (0x8a - (0x8a - 0x0b) * t / 255) as u8];
                fill(width * band / bands, 0, width / bands + 1, height, color);
            }
            let (card_x, card_y, card_w, card_h) = (width * 15 / 100, height / 5, width * 70 / 100, height * 3 / 5);
            fill(card_x, card_y, card_w, card_h, [0xf0, 0xf0, 0xf0]);
            let step = (frame as i32).min(48);
            let (box_w, box_h) = (60.min(card_w / 4).max(4), 40.min(card_h / 4).max(4));
            let box_x = card_x + (card_w - box_w) * step / 48;
            let box_y = card_y + (card_h - box_h) / 2;
            let box_color = [0xdc, 0x26, 0x26];
            fill(box_x, box_y, box_w, box_h, box_color);
            gl.disable(GL_SCISSOR_TEST);

            let mut pixels = vec![0u8; width as usize * height as usize * 4];
            // SAFETY: the buffer holds four bytes for every pixel of the
            // rectangle that is read.
            unsafe { gl.read_pixels(0, 0, width, height, GL_RGBA, GL_UNSIGNED_BYTE, pixels.as_mut_ptr().cast()) };
            let error = gl.get_error();
            let mut painted = 0usize;
            let mut checksum = 0xcbf2_9ce4_8422_2325_u64;
            for pixel in pixels.chunks_exact(4) {
                if pixel != [0, 0, 0, 0] {
                    painted += 1;
                }
                for &byte in pixel {
                    checksum = (checksum ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
                }
            }
            // The pixel at a point of the window, in the rows of OpenGL.
            let pixel_at = |x: i32, y: i32| -> [u8; 4] {
                let index = ((height - 1 - y) as usize * width as usize + x as usize) * 4;
                [pixels[index], pixels[index + 1], pixels[index + 2], pixels[index + 3]]
            };
            let near = |pixel: [u8; 4], color: [u8; 3]| {
                pixel[3] == 255 && (0..3).all(|i| (i32::from(pixel[i]) - i32::from(color[i])).abs() <= 1)
            };
            let in_box = pixel_at(box_x + box_w / 2, box_y + box_h / 2);
            let in_card = pixel_at(card_x + card_w / 2, card_y + 2);
            let in_first_band = pixel_at(1, 1);
            if error != 0 || !near(in_box, box_color) || !near(in_card, [0xf0, 0xf0, 0xf0]) || !near(in_first_band, [0x1e, 0x3a, 0x8a]) {
                self.mismatches.borrow_mut().push(format!(
                    "frame {frame} ({width}x{height}): the moving rectangle reads {in_box:?} (drawn {box_color:?}), the card {in_card:?} (drawn [240, 240, 240]), the first band {in_first_band:?} (drawn [30, 58, 138]), GL error {error:#x}"
                ));
            }
            if frame < 3 {
                println!("Frame #{frame}: {width}x{height} px by OpenGL ES; the moving rectangle reads back as {in_box:?}");
            }
            self.last_frame.set(Some((size, painted, checksum)));

            // Presents the frame.
            session.dispose();
        }

        fn dispose(&self) {
            self.render_target.dispose();
            self.context.dispose();
        }
    }

    /// What the callbacks share.
    struct State {
        window: Rc<dyn IWindowImpl>,
        /// The painter of the rendering mode ANGLE; `None` in the software
        /// mode, whose frames the render interface draws.
        gl: Option<GlPainter>,
        context: Rc<dyn IPlatformRenderInterfaceContext>,
        render_target: RefCell<Option<Rc<dyn IRenderTarget>>>,
        frames: Cell<u32>,
        /// The checksum of the pixels of every frame that was read back.
        checksums: RefCell<Vec<u64>>,
        resizes: RefCell<Vec<Size>>,
        states: RefCell<Vec<WindowState>>,
        closed: Cell<bool>,
        seen: SeenInput,
        /// The ticks the run has waited for the work another thread posted.
        wake_waits: Cell<u32>,
        report: Report,
        verbose_input: bool,
    }

    /// What the timer does after a step of a smoke run.
    enum Step {
        /// The next tick runs the next step.
        Next,
        /// The next tick runs this step again: what it waits for has not
        /// happened yet.
        Again,
        /// The run is over.
        Done,
    }

    impl State {
        /// Draws a frame of shapes; the number of the frame moves them.
        fn paint(&self) {
            if self.closed.get() {
                return;
            }
            let frame = self.frames.get();
            let client_size = self.window.client_size();
            let scaling = self.window.render_scaling();
            let pixel_size = PixelSize::from_size(client_size, scaling);
            if pixel_size.width <= 0 || pixel_size.height <= 0 {
                return;
            }

            if let Some(gl) = &self.gl {
                gl.paint(frame, pixel_size, scaling);
                self.frames.set(frame + 1);
                return;
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

            context.clear(Colors::WHITE);
            context.set_transform(Matrix::create_scale(scaling, scaling));

            let (width, height) = (client_size.width, client_size.height);
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

            // A card with rounded corners, clipped content and a layer of
            // half opacity inside it.
            let card = Rect::new(width * 0.15, height * 0.2, width * 0.7, height * 0.6);
            let fill = ImmutableSolidColorBrush::new(Color::from_argb(0xe6, 0xff, 0xff, 0xff));
            let outline = ImmutablePen::from_uint32(0xff11_1827, 3.0);
            context.draw_rectangle(Some(&fill), Some(&outline), RoundedRect::from_radius(card, 24.0), &BoxShadows::default());

            context.push_clip_rounded(RoundedRect::from_radius(card, 24.0));
            context.push_opacity(0.5, None);
            // The window also paints when the system asks it to, so the
            // number of a frame is not the number of a tick of the timer:
            // the shapes move with every frame and do not come back.
            let step = (f64::from(frame) / 48.0).min(1.0);
            let ellipse = Rect::new(card.x + card.width * step - 40.0, card.y + card.height * 0.5 - 40.0, 120.0, 80.0);
            context.draw_ellipse(Some(&ImmutableSolidColorBrush::new(Color::from_rgb(0x0e, 0x96, 0x88))), None, ellipse);
            context.draw_rectangle(
                Some(&ImmutableSolidColorBrush::new(Color::from_rgb(0xdc, 0x26, 0x26))),
                None,
                RoundedRect::from_rect(Rect::new(ellipse.x + 60.0, ellipse.y + 30.0, 90.0, 60.0)),
                &BoxShadows::default(),
            );
            context.pop_opacity();
            context.pop_clip();

            let dashed = ImmutablePen::new(
                Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(0x11, 0x18, 0x27)))),
                4.0,
                None,
                PenLineCap::Round,
                PenLineJoin::Round,
                10.0,
            );
            context.draw_line(
                Some(&dashed),
                Point::new(width * 0.15, height * 0.9),
                Point::new(width * (0.15 + 0.7 * step), height * 0.9),
            );

            context.dispose();

            self.frames.set(frame + 1);
            if frame < 3 {
                println!(
                    "Frame #{frame}: {}x{} px (client {}x{}, scaling {scaling})",
                    pixel_size.width, pixel_size.height, client_size.width, client_size.height
                );
            }
        }

        /// Reads the frame that was just drawn back from the framebuffer of
        /// the window (the surface keeps its memory from frame to frame
        /// while the size of the window stays): its size, how many of its
        /// pixels are not blank and a checksum of its bytes.
        fn read_back(&self) -> Option<(PixelSize, usize, u64)> {
            // A frame of OpenGL was read back before it was presented.
            if let Some(gl) = &self.gl {
                return gl.last_frame.get();
            }
            let surfaces = self.window.surfaces();
            let framebuffer_surface = surfaces.iter().find_map(|surface| surface.as_framebuffer_surface())?;
            let render_target = framebuffer_surface.create_framebuffer_render_target();
            let client_size = self.window.client_size();
            let scaling = self.window.render_scaling();
            let scene_info = RenderTargetSceneInfo::new(
                PixelSize::from_size(client_size, scaling),
                scaling,
                CompositionTransparencyLevel::None,
            );
            let (framebuffer, _) = render_target.lock(&scene_info);
            let size = framebuffer.size();
            let mut painted = 0usize;
            let mut checksum = 0xcbf2_9ce4_8422_2325_u64;
            framebuffer.with_data(&mut |data| {
                for pixel in data.chunks_exact(4) {
                    if pixel != [0, 0, 0, 0] {
                        painted += 1;
                    }
                    for &byte in pixel {
                        checksum = (checksum ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
                    }
                }
            });
            // Disposing presents the same pixels again and releases the
            // surface.
            framebuffer.dispose();
            render_target.dispose();
            Some((size, painted, checksum))
        }

        fn input(&self, e: &dyn IRawInputEventArgs) {
            if let Some(e) = e.downcast_ref::<RawKeyEventArgs>() {
                if self.verbose_input {
                    println!(
                        "Input: key {:?} {:?} physical {:?} symbol {:?} modifiers {:?}",
                        e.type_(),
                        e.key(),
                        e.physical_key(),
                        e.key_symbol(),
                        e.modifiers()
                    );
                }
                if e.type_() == RawKeyEventType::KeyDown {
                    *self.seen.key_down.borrow_mut() = Some((e.key(), e.physical_key(), e.key_symbol()));
                } else {
                    self.seen.key_up.set(true);
                }
            } else if let Some(e) = e.downcast_ref::<RawTextInputEventArgs>() {
                if self.verbose_input {
                    println!("Input: text {:?}", e.text());
                }
                *self.seen.text.borrow_mut() = Some(e.text().to_string());
            } else if let Some(e) = e.downcast_ref::<RawMouseWheelEventArgs>() {
                if self.verbose_input {
                    println!("Input: wheel at {:?} delta {:?}", e.position(), e.delta());
                }
                self.seen.wheel.set(Some(e.delta().y));
            } else if let Some(e) = e.downcast_ref::<RawPointerEventArgs>() {
                if self.verbose_input && e.type_() != RawPointerEventType::Move {
                    println!("Input: pointer {:?} at {:?} modifiers {:?}", e.type_(), e.position(), e.input_modifiers());
                }
                match e.type_() {
                    RawPointerEventType::Move => self.seen.mouse_move.set(Some(e.position())),
                    RawPointerEventType::LeftButtonDown => self.seen.left_down.set(Some(e.position())),
                    RawPointerEventType::LeftButtonUp => self.seen.left_up.set(true),
                    _ => {}
                }
            }
        }
    }

    /// Polls a future once: the clipboard completes at once unless another
    /// process holds it open.
    fn poll_once<T>(mut future: Pin<Box<dyn Future<Output = T>>>) -> Option<T> {
        let mut context = Context::from_waker(Waker::noop());
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => Some(value),
            Poll::Pending => None,
        }
    }

    fn make_l_param(x: i32, y: i32) -> isize {
        (((y as u16 as u32) << 16) | (x as u16 as u32)) as isize
    }

    /// The steps of a smoke run after its frames, one a tick of the timer:
    /// each acts, and the next checks what the action did once the messages
    /// it caused have been processed.
    fn smoke_step(state: &Rc<State>, step: u32, hwnd: isize, woken: &Arc<AtomicBool>) -> Step {
        let report = &state.report;
        let window = &state.window;
        match step {
            0 => {
                println!("-- window state");
                window.set_window_state(WindowState::Maximized);
                Step::Next
            }
            1 => {
                let size = window.client_size();
                report.check(
                    "maximize",
                    state.states.borrow().last() == Some(&WindowState::Maximized) && size.width > 800.0,
                    format!("states seen {:?}, client size {}x{}", state.states.borrow(), size.width, size.height),
                );
                let screens = FerroLocator::current().get_required_service::<dyn IScreenImpl>();
                match screens.screen_from_window(&**window) {
                    Some(screen) => {
                        let working_area = screen.working_area();
                        let pixel_size = PixelSize::from_size(size, window.render_scaling());
                        report.check(
                            "maximized client area fits the working area",
                            pixel_size.width == working_area.width && pixel_size.height < working_area.height,
                            format!(
                                "client {}x{} px, working area {}x{} px (the caption is above the client area)",
                                pixel_size.width, pixel_size.height, working_area.width, working_area.height
                            ),
                        );
                    }
                    None => report.check("screen of the window", false, "the window is on no screen"),
                }
                state.paint();
                window.set_window_state(WindowState::Normal);
                Step::Next
            }
            2 => {
                let size = window.client_size();
                report.check(
                    "restore",
                    state.states.borrow().last() == Some(&WindowState::Normal)
                        && (size.width - 800.0).abs() <= 1.0
                        && (size.height - 520.0).abs() <= 1.0,
                    format!("states seen {:?}, client size {}x{}", state.states.borrow(), size.width, size.height),
                );
                state.paint();

                println!("-- input (messages posted to the window)");
                // The A key: scan code 0x1E, repeat count 1; and its
                // release, with the transition bits set.
                post_message(hwnd, WindowsMessage::WM_KEYDOWN, 0x41, 0x001E_0001);
                post_message(hwnd, WindowsMessage::WM_KEYUP, 0x41, 0xC01E_0001_u32 as isize);
                let scaling = window.render_scaling();
                let (x, y) = ((100.0 * scaling) as i32, (50.0 * scaling) as i32);
                post_message(hwnd, WindowsMessage::WM_MOUSEMOVE, 0, make_l_param(x, y));
                post_message(hwnd, WindowsMessage::WM_LBUTTONDOWN, 0x0001, make_l_param(x, y));
                post_message(hwnd, WindowsMessage::WM_LBUTTONUP, 0, make_l_param(x, y));
                // One notch of the wheel; its position is in screen
                // coordinates.
                let screen_point = window.point_to_screen(Point::new(100.0, 50.0));
                post_message(
                    hwnd,
                    WindowsMessage::WM_MOUSEWHEEL,
                    (120usize) << 16,
                    make_l_param(screen_point.x, screen_point.y),
                );
                Step::Next
            }
            3 => {
                let seen = &state.seen;
                let key_down = seen.key_down.borrow().clone();
                report.check(
                    "key down",
                    matches!(&key_down, Some((Key::A, PhysicalKey::A, _))),
                    format!("{key_down:?} (expected the key A on the physical key A)"),
                );
                report.check("key up", seen.key_up.get(), format!("seen: {}", seen.key_up.get()));
                let text = seen.text.borrow().clone();
                // The character depends on the keyboard layout and the lock
                // keys of the session; that one arrives is the promise.
                report.check("text input", text.as_deref().is_some_and(|text| !text.is_empty()), format!("{text:?}"));
                let near = |point: Option<Point>| {
                    point.is_some_and(|point| (point.x - 100.0).abs() <= 1.0 && (point.y - 50.0).abs() <= 1.0)
                };
                report.check("mouse move", near(seen.mouse_move.get()), format!("{:?}", seen.mouse_move.get()));
                report.check("left button down", near(seen.left_down.get()), format!("{:?}", seen.left_down.get()));
                report.check("left button up", seen.left_up.get(), format!("seen: {}", seen.left_up.get()));
                report.check("mouse wheel", seen.wheel.get() == Some(1.0), format!("{:?} notch(es)", seen.wheel.get()));

                println!("-- clipboard");
                let clipboard = FerroLocator::current().get_required_service::<dyn IClipboardImpl>();
                let text = format!("FerroUI win32_window {}", std::process::id());
                let data_transfer = DataTransfer::new();
                data_transfer.add(DataTransferItem::create_text(Some(&text)));
                let data_transfer: Rc<dyn IDataTransfer> = data_transfer;
                match poll_once(clipboard.set_data_async(data_transfer.to_asynchronous())) {
                    Some(Ok(())) => match poll_once(clipboard.try_get_data_async()) {
                        Some(Ok(Some(read))) => {
                            let read = read.to_synchronous(LogArea::WIN32_PLATFORM).try_get_text();
                            report.check("clipboard text", read.as_deref() == Some(text.as_str()), format!("{read:?}"));
                        }
                        Some(Ok(None)) => report.check("clipboard text", false, "the clipboard has no text after it was set"),
                        Some(Err(error)) => report.check("clipboard text", false, format!("reading failed: {error:?}")),
                        None => report.info("clipboard text", "the clipboard is held open by another process"),
                    },
                    Some(Err(error)) => report.check("clipboard text", false, format!("setting failed: {error:?}")),
                    None => report.info("clipboard text", "the clipboard is held open by another process"),
                }

                println!("-- platform settings");
                // The colours and the language are read through the Windows
                // Runtime on a system that has its types: the settings
                // answer either way, and what they answer is printed.
                let settings = FerroLocator::current().get_required_service::<dyn ferroui_base::platform::IPlatformSettings>();
                let color_values = settings.get_color_values();
                report.check(
                    "colour values",
                    color_values == settings.get_color_values(),
                    format!(
                        "theme {:?}, contrast {:?}, accent {:?}",
                        color_values.theme_variant(),
                        color_values.contrast_preference(),
                        color_values.accent_color1()
                    ),
                );
                let language = settings.preferred_application_language();
                report.check("preferred language", !language.is_empty(), format!("{language:?}"));
                // A change of a setting of the system is a message to the
                // message window of the platform: one that names the colours
                // makes the settings read them again, and nothing changed,
                // so no change is reported.
                let changes = Rc::new(Cell::new(0u32));
                let subscription = {
                    let changes = changes.clone();
                    settings.color_values_changed(Rc::new(move |_| changes.set(changes.get() + 1)))
                };
                let setting: Vec<u16> = "ImmersiveColorSet\0".encode_utf16().collect();
                ferroui_win32::interop::unmanaged_methods::send_message(
                    Win32Platform::message_window(),
                    WindowsMessage::WM_SETTINGCHANGE,
                    0,
                    setting.as_ptr() as isize,
                );
                subscription.dispose();
                report.check(
                    "setting change",
                    changes.get() == 0 && settings.get_color_values() == color_values,
                    format!("a colour setting change with the same colours reported {} change(s)", changes.get()),
                );

                println!("-- dispatcher");
                // Work posted from another thread has to wake the message
                // loop: the signal of the dispatcher.
                let woken = woken.clone();
                std::thread::spawn(move || {
                    Dispatcher::ui_thread().post(move || woken.store(true, Ordering::SeqCst), DispatcherPriority::NORMAL);
                });
                Step::Next
            }
            4 => {
                // The other thread has to start and post first: up to two
                // seconds of ticks are given to it.
                if !woken.load(Ordering::SeqCst) && state.wake_waits.get() < 50 {
                    state.wake_waits.set(state.wake_waits.get() + 1);
                    return Step::Again;
                }
                report.check(
                    "work posted from another thread",
                    woken.load(Ordering::SeqCst),
                    format!("ran on the UI thread: {}", woken.load(Ordering::SeqCst)),
                );
                println!("-- closing the window");
                window.dispose();
                Step::Done
            }
            _ => Step::Done,
        }
    }

    /// What the ANGLE of this build does on this machine, display by
    /// display: the strings of EGL and of the context, and a frame cleared
    /// to a colour on a window surface and read back before it is
    /// presented. Run with `--angle-probe`; without the feature `angle` of
    /// the crate there is no ANGLE to load, and the probe says so.
    fn angle_probe(report: &Report) {
        use ferroui_base::reactive::IDisposable;
        use ferroui_base::platform::IPlatformGraphicsContext;
        use ferroui_opengl::gl_consts::{GL_COLOR_BUFFER_BIT, GL_RGBA, GL_UNSIGNED_BYTE};
        use ferroui_opengl::IGlContext;
        use ferroui_win32::open_gl::angle::{AngleWin32EglDisplay, Win32AngleEglInterface};

        const EGL_VENDOR: i32 = 0x3053;
        const EGL_VERSION: i32 = 0x3054;
        const EGL_EXTENSIONS: i32 = 0x3055;

        println!("-- ANGLE");
        let egl = match Win32AngleEglInterface::new() {
            Ok(egl) => egl,
            Err(error) => {
                report.check("ANGLE loads", false, format!("{error}"));
                return;
            }
        };
        report.check("ANGLE loads", true, "the entry points of EGL are resolved");
        report.info("EGL client extensions", format!("{:?}", egl.egl().query_string(0, EGL_EXTENSIONS)));
        report.info("eglCreateDeviceANGLE", format!("available: {}", egl.is_create_device_angle_available()));

        // A window of its own: a window that was drawn to through its
        // device context is not given to a swap chain.
        let windowing_platform = FerroLocator::current().get_required_service::<dyn IWindowingPlatform>();

        type Create = fn(&Win32AngleEglInterface) -> Result<AngleWin32EglDisplay, ferroui_opengl::OpenGlException>;
        let displays: [(&str, bool, Create); 2] = [
            ("Direct3D 11", true, AngleWin32EglDisplay::create_shared_d3d11_display),
            ("Direct3D 9", false, AngleWin32EglDisplay::create_d3d9_display),
        ];
        for (name, required, create) in displays {
            let outcome = |ok: bool, detail: String| {
                if required {
                    report.check(&format!("ANGLE on {name}"), ok, detail);
                } else {
                    report.info(&format!("ANGLE on {name}"), format!("{}: {detail}", if ok { "works" } else { "does not work" }));
                }
            };
            let display = match create(&egl) {
                Ok(display) => display,
                Err(error) => {
                    outcome(false, format!("the display: {error}"));
                    continue;
                }
            };
            let handle = display.handle();
            report.info(
                &format!("EGL on {name}"),
                format!(
                    "vendor {:?}, version {:?}, extensions {:?}",
                    egl.egl().query_string(handle, EGL_VENDOR),
                    egl.egl().query_string(handle, EGL_VERSION),
                    egl.egl().query_string(handle, EGL_EXTENSIONS)
                ),
            );
            report.info(&format!("Direct3D device of {name}"), format!("{:?}", display.get_direct3d_device().map(|device| device != 0)));
            let context = match display.create_context(None) {
                Ok(context) => context,
                Err(error) => {
                    outcome(false, format!("the context: {error}"));
                    display.dispose();
                    continue;
                }
            };
            let window = windowing_platform.create_window();
            window.resize(Size::new(200.0, 120.0), WindowResizeReason::Application);
            window.show(false, false);
            let hwnd = window.handle().map_or(0, |handle| handle.handle());
            let result = (|| -> Result<String, String> {
                let surface = display.create_window_surface(hwnd).map_err(|error| format!("the window surface: {error}"))?;
                let current = context.make_current_with_surface(Some(&surface)).map_err(|error| format!("make current: {error}"))?;
                let gl = context.gl_interface();
                let strings = format!(
                    "{:?} by {:?} on {:?}, OpenGL ES {}.{}",
                    gl.version(),
                    gl.vendor(),
                    gl.renderer(),
                    context.version().major(),
                    context.version().minor()
                );
                gl.viewport(0, 0, 200, 120);
                gl.clear_color(1.0, 0.5, 0.0, 1.0);
                gl.clear(GL_COLOR_BUFFER_BIT);
                let mut pixel = [0u8; 4];
                // SAFETY: one pixel of four bytes is read into four bytes.
                unsafe { gl.read_pixels(10, 10, 1, 1, GL_RGBA, GL_UNSIGNED_BYTE, pixel.as_mut_ptr().cast()) };
                let error = gl.get_error();
                surface.swap_buffers();
                current.dispose();
                surface.dispose();
                if pixel == [255, 128, 0, 255] || pixel == [255, 127, 0, 255] {
                    Ok(format!("{strings}; a frame cleared to orange reads back as {pixel:?} (GL error {error:#x})"))
                } else {
                    Err(format!("{strings}; a frame cleared to orange reads back as {pixel:?} (GL error {error:#x})"))
                }
            })();
            match result {
                Ok(detail) => outcome(true, detail),
                Err(detail) => outcome(false, detail),
            }
            context.dispose();
            display.dispose();
            window.dispose();
        }
    }

    pub fn run() -> ExitCode {
        let smoke = std::env::args().any(|argument| argument == "--smoke");
        let probe_angle = std::env::args().any(|argument| argument == "--angle-probe");

        // `--rendering software|angle`: the one rendering mode of the run,
        // without a fallback, so that a mode that does not initialise fails
        // the run instead of being passed over.
        let arguments: Vec<String> = std::env::args().collect();
        let rendering = arguments
            .iter()
            .position(|argument| argument == "--rendering")
            .and_then(|index| arguments.get(index + 1))
            .map_or("software", String::as_str);
        let rendering_mode = match rendering {
            "software" => Win32RenderingMode::Software,
            "angle" => Win32RenderingMode::AngleEgl,
            other => {
                eprintln!("win32_window: unknown rendering mode '{other}' (software, angle)");
                return ExitCode::from(2);
            }
        };
        println!("Rendering mode: {rendering_mode:?}");

        let options = Win32PlatformOptions {
            rendering_mode: vec![rendering_mode],
            // The window is presented through its redirection surface: the
            // composition modes are a later stage.
            composition_mode: vec![Win32CompositionMode::RedirectionSurface],
            ..Win32PlatformOptions::default()
        };
        Win32Platform::initialize(options);
        VelloPlatform::initialize_with_options(VelloOptions::with_rendering_mode(VelloRenderingMode::Cpu));

        let version = Win32Platform::windows_version();
        println!("Windows {}.{} build {}", version.major, version.minor, version.build);

        let locator = FerroLocator::current();
        let screens = locator.get_required_service::<dyn IScreenImpl>();
        let all_screens = screens.all_screens();
        println!("Screens: {} (the system counts {})", all_screens.len(), screens.screen_count());
        for screen in &all_screens {
            println!(
                "  {:?}: bounds {:?}, working area {:?}, scaling {}, primary {}, orientation {:?}",
                screen.display_name(),
                screen.bounds(),
                screen.working_area(),
                screen.scaling(),
                screen.is_primary(),
                screen.current_orientation()
            );
        }

        let render_interface = locator.get_required_service::<dyn IPlatformRenderInterface>();
        let context = render_interface.create_backend_context(None);

        let windowing_platform = locator.get_required_service::<dyn IWindowingPlatform>();
        let window = windowing_platform.create_window();
        let early_report = Report::default();
        let gl = if rendering_mode == Win32RenderingMode::AngleEgl {
            println!("-- platform graphics");
            GlPainter::new(&window, &early_report)
        } else {
            None
        };
        let gl_failed = rendering_mode == Win32RenderingMode::AngleEgl && gl.is_none();
        let state = Rc::new(State {
            window: window.clone(),
            gl,
            context,
            render_target: RefCell::new(None),
            frames: Cell::new(0),
            checksums: RefCell::new(Vec::new()),
            resizes: RefCell::new(Vec::new()),
            states: RefCell::new(Vec::new()),
            closed: Cell::new(false),
            seen: SeenInput::default(),
            wake_waits: Cell::new(0),
            report: early_report,
            verbose_input: true,
        });
        if gl_failed {
            eprintln!("win32_window: the rendering mode ANGLE has no painter; see the failed check above");
            return ExitCode::FAILURE;
        }
        let report = &state.report;
        let loop_cancellation = CancellationTokenSource::new();

        let handle = window.handle();
        let hwnd = handle.as_ref().map_or(0, |handle| handle.handle());
        if smoke {
            println!("-- platform");
            report.check(
                "screens",
                !all_screens.is_empty() && all_screens.iter().filter(|screen| screen.is_primary()).count() == 1,
                format!("{} screen(s), one of them primary", all_screens.len()),
            );
            report.check(
                "window handle",
                hwnd != 0 && handle.as_ref().and_then(|handle| handle.handle_descriptor().map(str::to_owned)).as_deref() == Some("HWND"),
                format!("{hwnd:#x}"),
            );
            let surfaces = window.surfaces();
            let gl_surfaces = surfaces.iter().filter(|surface| try_get_gl_surface(&***surface).is_some()).count();
            let expected_gl_surfaces = usize::from(rendering_mode == Win32RenderingMode::AngleEgl);
            report.check(
                "surfaces",
                surfaces.len() == 2 + expected_gl_surfaces
                    && gl_surfaces == expected_gl_surfaces
                    && surfaces.iter().any(|surface| surface.as_framebuffer_surface().is_some()),
                format!(
                    "{} surface(s): the window handle, {gl_surfaces} OpenGL surface(s) and the framebuffer",
                    surfaces.len()
                ),
            );
        }

        window.set_input_root(Rc::new(InputRoot {
            root: InputElement::new(),
            pointer_over_element: RefCell::new(None),
            cursor_element: RefCell::new(None),
        }));

        let weak = Rc::downgrade(&state);
        window.set_paint(Some(Rc::new(move |_rect| {
            if let Some(state) = weak.upgrade() {
                state.paint();
            }
        })));
        let weak = Rc::downgrade(&state);
        window.set_resized(Some(Rc::new(move |size: Size, reason: WindowResizeReason| {
            if let Some(state) = weak.upgrade() {
                state.resizes.borrow_mut().push(size);
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
        let weak = Rc::downgrade(&state);
        window.set_window_state_changed(Some(Rc::new(move |window_state| {
            println!("WindowStateChanged: {window_state:?}");
            if let Some(state) = weak.upgrade() {
                state.states.borrow_mut().push(window_state);
            }
        })));
        let weak = Rc::downgrade(&state);
        let cancel = loop_cancellation.clone();
        window.set_closed(Some(Rc::new(move || {
            println!("Closed");
            if let Some(state) = weak.upgrade() {
                state.closed.set(true);
                let render_target = state.render_target.borrow_mut().take();
                if let Some(render_target) = render_target {
                    render_target.dispose();
                }
                if let Some(gl) = &state.gl {
                    gl.dispose();
                }
            }
            cancel.cancel();
        })));

        window.set_title(Some("FerroUI Win32 window"));
        window.show_taskbar_icon(true);
        window.resize(Size::new(640.0, 400.0), WindowResizeReason::Application);
        window.move_(PixelPoint::new(60, 60));
        window.show(true, false);
        println!(
            "Window shown: client size {:?}, frame size {:?}, position {:?}, scaling {}",
            window.client_size(),
            window.frame_size(),
            window.position(),
            window.render_scaling()
        );
        if smoke {
            println!("-- window");
            let size = window.client_size();
            report.check(
                "client size after resize and show",
                (size.width - 640.0).abs() <= 1.0 && (size.height - 400.0).abs() <= 1.0,
                format!("{}x{} (asked for 640x400)", size.width, size.height),
            );
            let position = window.position();
            report.check("position", position == PixelPoint::new(60, 60), format!("{position:?} (asked for 60, 60)"));
            report.check(
                "frame is larger than the client area",
                window.frame_size().is_some_and(|frame| frame.height > size.height && frame.width >= size.width),
                format!("{:?}", window.frame_size()),
            );
            report.info("window state before any change", format!("{:?}", window.window_state()));
        }

        // The frames of a smoke run: one every 40 ms, a resize half way;
        // the steps that follow them act on one tick and check on the next.
        let smoke_frames = Rc::new(Cell::new(0u32));
        let woken = Arc::new(AtomicBool::new(false));
        let _smoke_timer = smoke.then(|| {
            let (state, window, drawn) = (state.clone(), window.clone(), smoke_frames.clone());
            let woken = woken.clone();
            let step = Cell::new(0u32);
            println!("-- frames");
            DispatcherTimer::run(
                move || {
                    if state.closed.get() {
                        return false;
                    }
                    if drawn.get() < SMOKE_FRAMES {
                        if drawn.get() == SMOKE_FRAMES / 2 {
                            window.resize(Size::new(800.0, 520.0), WindowResizeReason::Application);
                        }
                        let before = state.frames.get();
                        state.paint();
                        if state.frames.get() == before + 1 {
                            drawn.set(drawn.get() + 1);
                            if let Some((size, painted, checksum)) = state.read_back() {
                                let pixels = size.width as usize * size.height as usize;
                                if painted != pixels {
                                    state.report.check(
                                        "frame covers the window",
                                        false,
                                        format!("frame {}: {painted} of {pixels} pixels painted", drawn.get()),
                                    );
                                }
                                state.checksums.borrow_mut().push(checksum);
                            }
                        }
                        if drawn.get() == SMOKE_FRAMES {
                            let checksums = state.checksums.borrow();
                            let mut distinct = checksums.clone();
                            distinct.sort_unstable();
                            distinct.dedup();
                            state.report.check(
                                "frames",
                                checksums.len() == SMOKE_FRAMES as usize && distinct.len() == checksums.len(),
                                format!(
                                    "{} frame(s) drawn and read back from the {}, {} of them different (the shapes move)",
                                    checksums.len(),
                                    if state.gl.is_some() { "default framebuffer of OpenGL ES before the swap" } else { "framebuffer" },
                                    distinct.len()
                                ),
                            );
                            if let Some(gl) = &state.gl {
                                let mismatches = gl.mismatches.borrow();
                                state.report.check(
                                    "frames of the GPU hold what was drawn",
                                    mismatches.is_empty(),
                                    if mismatches.is_empty() {
                                        "every frame read back with glReadPixels has the colours drawn at three places".to_string()
                                    } else {
                                        mismatches.join("; ")
                                    },
                                );
                            }
                            let resizes = state.resizes.borrow();
                            state.report.check(
                                "resize half way",
                                resizes.iter().any(|size| (size.width - 800.0).abs() <= 1.0 && (size.height - 520.0).abs() <= 1.0),
                                format!("sizes reported: {:?}", *resizes),
                            );
                        }
                        return true;
                    }

                    let current = step.get();
                    match smoke_step(&state, current, hwnd, &woken) {
                        Step::Next => {
                            step.set(current + 1);
                            true
                        }
                        Step::Again => true,
                        Step::Done => false,
                    }
                },
                Duration::from_millis(40),
                DispatcherPriority::NORMAL,
            )
        });

        println!("Entering main loop");
        Dispatcher::ui_thread().main_loop(&loop_cancellation.token());
        println!("Main loop exited");

        let (frames, closed) = (state.frames.get(), state.closed.get());
        let lost = state.context.is_lost();
        println!("Summary: {frames} frame(s), {} resize(s), closed={closed}, lost={lost}", state.resizes.borrow().len());

        if smoke {
            report.check("closed", closed, format!("the window reported being closed: {closed}"));
            report.check("render context", !lost, format!("lost: {lost}"));
            report.check(
                "every frame was drawn",
                smoke_frames.get() == SMOKE_FRAMES,
                format!("{} of {SMOKE_FRAMES}", smoke_frames.get()),
            );
        }

        if probe_angle {
            angle_probe(&state.report);
        }

        let failures = state.report.failures.borrow().clone();
        let passed = state.report.passed.get();
        state.context.dispose();
        drop(state);
        drop(window);

        if smoke {
            if failures.is_empty() {
                println!("smoke run passed: {passed} check(s)");
            } else {
                eprintln!("smoke run failed: {} of {} check(s)", failures.len(), failures.len() as u32 + passed);
                for failure in &failures {
                    eprintln!("  {failure}");
                }
                return ExitCode::FAILURE;
            }
        }
        ExitCode::SUCCESS
    }
}
