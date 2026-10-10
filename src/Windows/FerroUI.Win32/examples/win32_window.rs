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
    // `--teardown-trace`: what the end of the process does is written to the standard error
    // stream. The values of the main thread are destroyed while the process ends, after the
    // system has ended every other thread: a destructor that waits for one of them never
    // returns (docs/porting/win32-platform.md, section 11.2: runs through DirectComposition
    // printed that they passed and then did not end). The trace tells how far the end got.
    let trace = std::env::args().any(|argument| argument == "--teardown-trace");
    if trace {
        teardown::at_start();
    }
    let code = windows::run();
    // The last line the program prints itself: what follows is the end of the process (the
    // values of the thread, the libraries).
    println!("main returns");
    if trace {
        teardown::at_return();
    }
    code
}

/// Diagnostics of the end of the process: the threads that are alive when `main` returns,
/// a line per second while they still run (none once the system has ended them), and a line
/// when the handlers of the C runtime and the destructors of the values of the main thread
/// run.
#[cfg(windows)]
mod teardown {
    use std::ffi::c_void;

    #[repr(C)]
    struct ThreadEntry32 {
        size: u32,
        usage: u32,
        thread_id: u32,
        owner_process_id: u32,
        base_priority: i32,
        delta_priority: i32,
        flags: u32,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateToolhelp32Snapshot(flags: u32, process_id: u32) -> isize;
        fn Thread32First(snapshot: isize, entry: *mut ThreadEntry32) -> i32;
        fn Thread32Next(snapshot: isize, entry: *mut ThreadEntry32) -> i32;
        fn CloseHandle(handle: isize) -> i32;
        fn GetCurrentProcessId() -> u32;
        fn GetCurrentThreadId() -> u32;
        fn GetCurrentProcess() -> isize;
        fn TerminateProcess(process: isize, code: u32) -> i32;
        fn OpenThread(access: u32, inherit: i32, thread_id: u32) -> isize;
        fn GetThreadDescription(thread: isize, description: *mut *mut u16) -> i32;
        fn LocalFree(memory: *mut c_void) -> *mut c_void;
        fn GetModuleHandleExW(flags: u32, address: *const c_void, module: *mut isize) -> i32;
        fn GetModuleFileNameW(module: isize, name: *mut u16, size: u32) -> u32;
    }

    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn NtQueryInformationThread(thread: isize, class: u32, information: *mut c_void, length: u32, returned: *mut u32) -> i32;
    }

    unsafe extern "C" {
        fn atexit(callback: extern "C" fn()) -> i32;
    }

    struct Marker(&'static str);

    impl Drop for Marker {
        fn drop(&mut self) {
            eprintln!("teardown: {}", self.0);
        }
    }

    thread_local! {
        static FIRST: Marker = const { Marker("the value of the main thread that was registered first is destroyed (the last of them)") };
        static LAST: Marker = const { Marker("the values of the main thread are being destroyed (the one registered last is the first)") };
    }

    extern "C" fn at_exit() {
        eprintln!("teardown: the handlers of the C runtime run (the one registered when main started)");
    }

    /// The name and the module of the start address of every thread of the process.
    fn threads() -> Vec<String> {
        let mut list = Vec::new();
        // SAFETY (the block): calls of the system with values and buffers of this function;
        // every handle that is opened is closed, and the description is freed.
        unsafe {
            let (process, current) = (GetCurrentProcessId(), GetCurrentThreadId());
            let snapshot = CreateToolhelp32Snapshot(0x4, 0);
            if snapshot == -1 {
                return list;
            }
            let mut entry: ThreadEntry32 = std::mem::zeroed();
            entry.size = size_of::<ThreadEntry32>() as u32;
            let mut more = Thread32First(snapshot, &mut entry);
            while more != 0 {
                if entry.owner_process_id == process {
                    let mut line = format!("thread {}", entry.thread_id);
                    if entry.thread_id == current {
                        line.push_str(" (this thread)");
                    }
                    // THREAD_QUERY_INFORMATION
                    let thread = OpenThread(0x0040, 0, entry.thread_id);
                    if thread != 0 {
                        let mut description: *mut u16 = std::ptr::null_mut();
                        if GetThreadDescription(thread, &mut description) >= 0 && !description.is_null() {
                            let mut length = 0;
                            while *description.add(length) != 0 {
                                length += 1;
                            }
                            let name = String::from_utf16_lossy(std::slice::from_raw_parts(description, length));
                            if !name.is_empty() {
                                line.push_str(&format!(" \"{name}\""));
                            }
                            LocalFree(description.cast());
                        }
                        // ThreadQuerySetWin32StartAddress
                        let mut start: usize = 0;
                        let status = NtQueryInformationThread(
                            thread,
                            9,
                            (&raw mut start).cast(),
                            size_of::<usize>() as u32,
                            std::ptr::null_mut(),
                        );
                        if status >= 0 {
                            let mut module = 0isize;
                            // GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT
                            if GetModuleHandleExW(0x4 | 0x2, start as *const c_void, &mut module) != 0 {
                                let mut name = [0u16; 512];
                                let length = GetModuleFileNameW(module, name.as_mut_ptr(), name.len() as u32) as usize;
                                let path = String::from_utf16_lossy(&name[..length]);
                                let file = path.rsplit(['\\', '/']).next().unwrap_or(&path).to_owned();
                                line.push_str(&format!(", started in {file}+{:#x}", start.wrapping_sub(module as usize)));
                            } else {
                                line.push_str(&format!(", started at {start:#x}"));
                            }
                        }
                        CloseHandle(thread);
                    }
                    list.push(line);
                }
                more = Thread32Next(snapshot, &mut entry);
            }
            CloseHandle(snapshot);
        }
        list
    }

    pub fn at_start() {
        FIRST.with(|_| {});
        // SAFETY: a function of this program that takes nothing and returns.
        unsafe { atexit(at_exit) };
    }

    pub fn at_return() {
        let alive = threads();
        eprintln!("teardown: {} thread(s) when main returns", alive.len());
        for thread in &alive {
            eprintln!("teardown:   {thread}");
        }
        LAST.with(|_| {});
        let _ = std::thread::Builder::new().name("teardown watchdog".to_owned()).spawn(|| {
            for second in 1..=20 {
                std::thread::sleep(std::time::Duration::from_secs(1));
                eprintln!("teardown: {second} s after main returned the threads of the process still run");
                if second == 5 {
                    for thread in threads() {
                        eprintln!("teardown:   {thread}");
                    }
                }
            }
            eprintln!("teardown: the process did not end in 20 s while its threads ran: it is terminated with the code 97");
            // SAFETY: ends this process.
            unsafe { TerminateProcess(GetCurrentProcess(), 97) };
        });
    }
}

#[cfg(windows)]
mod windows {
    use ferroui_base::input::platform::IClipboardImpl;
    use ferroui_base::input::raw::{
        IRawInputEventArgs, RawKeyEventArgs, RawKeyEventType, RawMouseWheelEventArgs, RawPointerEventArgs,
        RawPointerEventType, RawTextInputEventArgs,
    };
    use ferroui_base::platform::storage::file_io::{BclStorageItemHandle, StorageProviderHelpers};
    use ferroui_base::platform::storage::IStorageItem;
    use ferroui_base::input::{
        DataFormat, DataTransfer, DataTransferExtensions, DataTransferItem, FocusManager, IDataTransfer, IInputRoot, InputElement, Key,
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

    /// Whether the run presents through Windows.UI.Composition: the checks of the transparency
    /// levels expect the effects of that mode then.
    static WIN_UI_COMPOSITION: AtomicBool = AtomicBool::new(false);
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
        /// Where in its texture the last frame was read back from.
        read_offset: Cell<(i32, i32)>,
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
            // The OpenGL surface of the window (the redirection surface),
            // or, in a composition mode, the surface the context renders
            // to through its render target factory, as a renderer does.
            let render_target = match surfaces.iter().find_map(|surface| try_get_gl_surface(&**surface)) {
                Some(gl_surface) => gl_surface.create_gl_render_target(&gl_context),
                None => {
                    let factory = features.try_get::<dyn ferroui_opengl::IGlPlatformSurfaceRenderTargetFactory>();
                    let surface = factory
                        .as_ref()
                        .and_then(|factory| surfaces.iter().find(|surface| factory.can_render_to_surface(&gl_context, surface)));
                    match (&factory, surface) {
                        (Some(factory), Some(surface)) => {
                            report.check(
                                "composition surface",
                                true,
                                "the context renders to a surface of the window through its render target factory",
                            );
                            factory.create_render_target(&gl_context, surface)
                        }
                        _ => {
                            report.check(
                                "OpenGL surface",
                                false,
                                format!(
                                    "the window has no OpenGL surface among its surfaces, and the context has {} it can render to one of them with",
                                    if factory.is_some() { "a render target factory but no surface" } else { "no render target factory" }
                                ),
                            );
                            return None;
                        }
                    }
                }
            };
            Some(GlPainter {
                context,
                render_target,
                mismatches: RefCell::new(Vec::new()),
                last_frame: Cell::new(None),
                read_offset: Cell::new((0, 0)),
            })
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
            // the rows of OpenGL count from the bottom, unless the session
            // says that its surface is flipped (a texture of Direct3D in a
            // composition mode), where they count from the top.
            let flipped = session.is_y_flipped();
            let fill = |x: i32, y: i32, w: i32, h: i32, color: [u8; 3]| {
                // SAFETY: plain values; the context of the session is
                // current.
                unsafe { scissor(x, if flipped { y } else { height - y - h }, w, h) };
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
            let (mut blank_in_column, mut blank_in_row) = (vec![0i32; width as usize], vec![0i32; height as usize]);
            for (index, pixel) in pixels.chunks_exact(4).enumerate() {
                if pixel != [0, 0, 0, 0] {
                    painted += 1;
                } else {
                    blank_in_column[index % width as usize] += 1;
                    blank_in_row[index / width as usize] += 1;
                }
                for &byte in pixel {
                    checksum = (checksum ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
                }
            }
            // In a composition mode the frame is a rectangle of a texture of the system, at
            // an offset the system chooses for every frame (the atlas of a virtual surface of
            // DirectComposition). ANGLE applies that offset to what is drawn (the viewport,
            // the scissor rectangle, a clear, the target of a blit) and not to glReadPixels,
            // which reads the texture from its origin: the frame is then read back moved by
            // the offset, with blank columns and rows before it and its last columns and rows
            // out of reach (the first run at scaling 2, in the virtual machine, read one blank
            // column and one blank row from the fifth frame on, while the window the system
            // composed had every pixel). The offset is what is blank at the start; the pixels
            // it hides count as painted, and the places that are compared move with it.
            let leading = |blank: &[i32], full: i32| blank.iter().take_while(|&&count| count == full).count() as i32;
            let (offset_x, offset_y) =
                if flipped { (leading(&blank_in_column, height), leading(&blank_in_row, width)) } else { (0, 0) };
            if (offset_x, offset_y) != self.read_offset.get() {
                self.read_offset.set((offset_x, offset_y));
                println!(
                    "[info] frame {frame}: the frame is read back at the offset ({offset_x}, {offset_y}) of its texture (glReadPixels of ANGLE reads the texture, not the rectangle of the frame)"
                );
            }
            if offset_x < width && offset_y < height {
                painted += (width as usize * height as usize)
                    - ((width - offset_x) as usize * (height - offset_y) as usize);
            }
            // The pixel at a point of the window, in the rows of OpenGL.
            let pixel_at = |x: i32, y: i32| -> [u8; 4] {
                let (x, y) = ((x + offset_x).min(width - 1), (y + offset_y).min(height - 1));
                let row = if flipped { y } else { height - 1 - y };
                let index = (row as usize * width as usize + x as usize) * 4;
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

    /// The client area of the window as the system holds it after a frame
    /// was presented: the window prints itself into a bitmap, with what the
    /// desktop window manager composed of it, so a frame presented through
    /// the swap chain of ANGLE is there as well as one copied with GDI.
    /// Nothing but the window of the example is captured.
    mod presented {
        use std::ffi::c_void;

        #[repr(C)]
        #[derive(Default)]
        struct Rect {
            left: i32,
            top: i32,
            right: i32,
            bottom: i32,
        }

        /// `BITMAPINFOHEADER`, which is all of a `BITMAPINFO` for 32 bits
        /// a pixel without compression.
        #[repr(C)]
        struct BitmapInfoHeader {
            size: u32,
            width: i32,
            height: i32,
            planes: u16,
            bit_count: u16,
            compression: u32,
            size_image: u32,
            x_pels_per_meter: i32,
            y_pels_per_meter: i32,
            clr_used: u32,
            clr_important: u32,
        }

        #[link(name = "user32", kind = "raw-dylib")]
        extern "system" {
            fn GetClientRect(hwnd: isize, rect: *mut Rect) -> i32;
            fn GetDC(hwnd: isize) -> isize;
            fn ReleaseDC(hwnd: isize, dc: isize) -> i32;
            fn PrintWindow(hwnd: isize, dc: isize, flags: u32) -> i32;
        }

        #[link(name = "gdi32", kind = "raw-dylib")]
        extern "system" {
            fn CreateCompatibleDC(dc: isize) -> isize;
            fn CreateDIBSection(
                dc: isize,
                info: *const BitmapInfoHeader,
                usage: u32,
                bits: *mut *mut c_void,
                section: isize,
                offset: u32,
            ) -> isize;
            fn SelectObject(dc: isize, object: isize) -> isize;
            fn DeleteObject(object: isize) -> i32;
            fn DeleteDC(dc: isize) -> i32;
            fn GdiFlush() -> i32;
        }

        /// `PW_CLIENTONLY | PW_RENDERFULLCONTENT`: the client area, with
        /// what the desktop window manager holds of the window.
        const FLAGS: u32 = 1 | 2;

        /// The pixels of the client area: four bytes a pixel, blue first,
        /// rows from the top.
        pub struct Capture {
            pub pixels: Vec<u8>,
            pub width: i32,
            pub height: i32,
        }

        impl Capture {
            /// Red, green and blue of the pixel of a column and a row.
            pub fn at(&self, x: i32, y: i32) -> [u8; 3] {
                let index = (y as usize * self.width as usize + x as usize) * 4;
                [self.pixels[index + 2], self.pixels[index + 1], self.pixels[index]]
            }
        }

        pub fn capture(hwnd: isize) -> Result<Capture, String> {
            let mut rect = Rect::default();
            // SAFETY: a rectangle of this frame the system writes to; a
            // handle that is not a window makes the call fail.
            if unsafe { GetClientRect(hwnd, &mut rect) } == 0 {
                return Err("GetClientRect failed".to_string());
            }
            let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
            if width < 1 || height < 1 {
                return Err(format!("the client area is empty ({width} by {height})"));
            }
            let header = BitmapInfoHeader {
                size: std::mem::size_of::<BitmapInfoHeader>() as u32,
                width,
                // A negative height: the rows run from the top.
                height: -height,
                planes: 1,
                bit_count: 32,
                compression: 0,
                size_image: 0,
                x_pels_per_meter: 0,
                y_pels_per_meter: 0,
                clr_used: 0,
                clr_important: 0,
            };
            // SAFETY: every object created here is released before the
            // function returns. The section is `width * height * 4` bytes
            // the system owns until the bitmap is deleted; they are copied
            // out, after the drawing of the system was flushed, while the
            // bitmap lives.
            unsafe {
                let window_dc = GetDC(hwnd);
                if window_dc == 0 {
                    return Err("GetDC failed".to_string());
                }
                let memory_dc = CreateCompatibleDC(window_dc);
                let mut bits: *mut c_void = std::ptr::null_mut();
                let bitmap = if memory_dc == 0 { 0 } else { CreateDIBSection(memory_dc, &header, 0, &mut bits, 0, 0) };
                let result = if memory_dc == 0 || bitmap == 0 || bits.is_null() {
                    Err("the bitmap of the capture could not be created".to_string())
                } else {
                    let previous = SelectObject(memory_dc, bitmap);
                    let printed = PrintWindow(hwnd, memory_dc, FLAGS) != 0;
                    GdiFlush();
                    let pixels = std::slice::from_raw_parts(bits as *const u8, (width * height * 4) as usize).to_vec();
                    SelectObject(memory_dc, previous);
                    if printed {
                        Ok(Capture { pixels, width, height })
                    } else {
                        Err("PrintWindow failed".to_string())
                    }
                };
                if bitmap != 0 {
                    DeleteObject(bitmap);
                }
                if memory_dc != 0 {
                    DeleteDC(memory_dc);
                }
                ReleaseDC(hwnd, window_dc);
                result
            }
        }
    }

    /// Checks that what was presented reaches the last column and the last
    /// row of the client area: the capture has the size of the client
    /// area, no pixel of its last column and of its last row is blank
    /// (white, which is what the system shows where nothing was presented,
    /// or black), and its last pixel has the colour both painters end
    /// with. The desktop window manager may compose a frame a moment after
    /// it was presented, so a capture that fails is taken again.
    fn check_presented(state: &State, hwnd: isize, name: &str) {
        const LAST: [u8; 3] = [0xf5, 0x9e, 0x0b];
        let expected = PixelSize::from_size(state.window.client_size(), state.window.render_scaling());
        let mut detail = String::new();
        for attempt in 0..6 {
            if attempt > 0 {
                std::thread::sleep(Duration::from_millis(100));
            }
            match presented::capture(hwnd) {
                Err(error) => detail = error,
                Ok(capture) => {
                    let (width, height) = (capture.width, capture.height);
                    let blank = |pixel: [u8; 3]| pixel.iter().all(|&c| c >= 250) || pixel.iter().all(|&c| c <= 5);
                    let blank_in_column = (0..height).filter(|&y| blank(capture.at(width - 1, y))).count();
                    let blank_in_row = (0..width).filter(|&x| blank(capture.at(x, height - 1))).count();
                    let last = capture.at(width - 1, height - 1);
                    let last_ok = (0..3).all(|i| (i32::from(last[i]) - i32::from(LAST[i])).abs() <= 12);
                    let size_ok = width == expected.width && height == expected.height;
                    detail = format!(
                        "the window prints {width} by {height} pixels (client area {} by {}); {blank_in_column} blank pixel(s) in the last column, {blank_in_row} in the last row; the last pixel is {last:?} (drawn {LAST:?}), the first {:?}",
                        expected.width,
                        expected.height,
                        capture.at(0, 0)
                    );
                    if size_ok && blank_in_column == 0 && blank_in_row == 0 && last_ok {
                        state.report.check(name, true, detail);
                        return;
                    }
                }
            }
        }
        state.report.check(name, false, detail);
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

    /// The bytes of an icon file with one image: 16 by 16 pixels of 32
    /// bits, an opaque blue square with a white border.
    fn icon_file() -> Vec<u8> {
        const SIZE: u32 = 16;
        let image_size = 40 + SIZE * SIZE * 4 + SIZE * 4;
        let mut data = vec![0, 0, 1, 0, 1, 0];
        // The directory entry: width, height, colours, reserved, planes,
        // bits, the size of the image and where it starts.
        data.extend_from_slice(&[SIZE as u8, SIZE as u8, 0, 0, 1, 0, 32, 0]);
        data.extend_from_slice(&image_size.to_le_bytes());
        data.extend_from_slice(&22u32.to_le_bytes());
        // The bitmap header: the height counts the colour rows and the
        // mask rows.
        data.extend_from_slice(&40u32.to_le_bytes());
        data.extend_from_slice(&SIZE.to_le_bytes());
        data.extend_from_slice(&(SIZE * 2).to_le_bytes());
        data.extend_from_slice(&[1, 0, 32, 0]);
        data.extend_from_slice(&[0; 24]);
        for y in 0..SIZE {
            for x in 0..SIZE {
                let border = x == 0 || y == 0 || x == SIZE - 1 || y == SIZE - 1;
                data.extend_from_slice(if border { &[0xff, 0xff, 0xff, 0xff] } else { &[0xd7, 0x78, 0x00, 0xff] });
            }
        }
        // The mask: nothing is transparent.
        data.extend_from_slice(&vec![0; (SIZE * 4) as usize]);
        data
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
                // The clipboard goes through OLE: the data transfer is a
                // data object the system asks for its formats, and what is
                // read back is the data object of the system.
                let clipboard = FerroLocator::current().get_required_service::<dyn IClipboardImpl>();
                let text = format!("FerroUI win32_window {} za\u{17c}\u{f3}\u{142}\u{107}", std::process::id());
                let string_format = DataFormat::create_string_application_format("win32-window-smoke.text");
                let bytes_format = DataFormat::create_bytes_application_format("win32-window-smoke.bytes");
                let bytes: Rc<[u8]> = Rc::from(vec![0u8, 1, 2, 0, 254, 255]);
                let executable = std::env::current_exe().ok().map(|path| path.to_string_lossy().to_string());
                let file = StorageProviderHelpers::try_create_bcl_storage_item(executable.as_deref()).map(|item| match item {
                    BclStorageItemHandle::Folder(folder) => folder as Rc<dyn IStorageItem>,
                    BclStorageItemHandle::File(file) => file as Rc<dyn IStorageItem>,
                });
                // The path as the storage item of the file has it.
                let executable = file.as_ref().and_then(|file| file.try_get_local_path());
                let item = DataTransferItem::create_text(Some(&text));
                item.set(&string_format, Some("a string of the application".to_string()));
                item.set(&bytes_format, Some(bytes.clone()));
                let data_transfer = DataTransfer::new();
                data_transfer.add(item);
                data_transfer.add(DataTransferItem::create_file(file));
                let data_transfer: Rc<dyn IDataTransfer> = data_transfer;
                match poll_once(clipboard.set_data_async(data_transfer.to_asynchronous())) {
                    Some(Ok(())) => match poll_once(clipboard.try_get_data_async()) {
                        Some(Ok(Some(read))) => {
                            let read = read.to_synchronous(LogArea::WIN32_PLATFORM);
                            let read_text = read.try_get_text();
                            report.check("clipboard text", read_text.as_deref() == Some(text.as_str()), format!("{read_text:?}"));
                            let read_string = read.try_get_value(&string_format);
                            report.check(
                                "clipboard string of an application format",
                                read_string.as_deref() == Some("a string of the application"),
                                format!("{read_string:?}"),
                            );
                            let read_bytes = read.try_get_value(&bytes_format);
                            report.check(
                                "clipboard bytes of an application format",
                                read_bytes.as_ref().is_some_and(|read| read.len() >= bytes.len() && read[..bytes.len()] == *bytes),
                                format!("{read_bytes:?}"),
                            );
                            let read_files: Vec<Option<String>> =
                                read.try_get_files().unwrap_or_default().iter().map(|file| file.try_get_local_path()).collect();
                            report.check(
                                "clipboard file",
                                executable.is_some() && read_files == [executable.clone()],
                                format!("{read_files:?} (set: {executable:?})"),
                            );
                            report.info("clipboard formats", format!("{:?}", read.formats()));
                            read.dispose();

                            let owned = clipboard.as_owned_clipboard_impl().and_then(|owned| poll_once(owned.is_current_owner_async()));
                            report.check("clipboard owner", matches!(owned, Some(Ok(true))), format!("{owned:?} after setting"));
                            let cleared = poll_once(clipboard.clear_async());
                            let owned = clipboard.as_owned_clipboard_impl().and_then(|owned| poll_once(owned.is_current_owner_async()));
                            let empty = poll_once(clipboard.try_get_data_async());
                            report.check(
                                "clipboard cleared",
                                matches!(cleared, Some(Ok(()))) && matches!(owned, Some(Ok(false))) && matches!(empty, Some(Ok(None))),
                                format!(
                                    "cleared: {cleared:?}, owner: {owned:?}, data afterwards: {}",
                                    match &empty {
                                        Some(Ok(None)) => "none".to_string(),
                                        Some(Ok(Some(data))) => format!("{:?}", data.formats()),
                                        Some(Err(error)) => format!("{error:?}"),
                                        None => "not read".to_string(),
                                    }
                                ),
                            );
                        }
                        Some(Ok(None)) => report.check("clipboard text", false, "the clipboard has no data after it was set"),
                        Some(Err(error)) => report.check("clipboard text", false, format!("reading failed: {error:?}")),
                        None => report.info("clipboard text", "the clipboard is held open by another process"),
                    },
                    Some(Err(error)) => report.check("clipboard text", false, format!("setting failed: {error:?}")),
                    None => report.info("clipboard text", "the clipboard is held open by another process"),
                }
                report.check(
                    "drag source",
                    FerroLocator::current().get_service::<dyn ferroui_base::input::platform::IPlatformDragSource>().is_some(),
                    "the platform registered its drag source",
                );

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

                println!("-- window icon");
                // An icon file with one image of 16 by 16 pixels, made here:
                // the loader of the platform makes the icon, the window
                // takes it, and the window answers the system's question
                // for its small and its big icon with an icon handle.
                let loader = FerroLocator::current().get_required_service::<dyn ferroui_controls::platform::IPlatformIconLoader>();
                match loader.load_icon_from_stream(&mut std::io::Cursor::new(icon_file())) {
                    Ok(icon) => {
                        let mut saved = Vec::new();
                        let saved_ok = icon.save(&mut saved).is_ok() && saved == icon_file();
                        report.check("icon loaded", saved_ok, format!("the icon saves the {} bytes it was loaded from", saved.len()));
                        window.set_icon(Some(icon));
                        let send = ferroui_win32::interop::unmanaged_methods::send_message;
                        let small = send(hwnd, WindowsMessage::WM_GETICON, 0, 0);
                        let big = send(hwnd, WindowsMessage::WM_GETICON, 1, 0);
                        let small_at_192 = send(hwnd, WindowsMessage::WM_GETICON, 2, 192);
                        report.check(
                            "icon of the window",
                            small != 0 && big != 0 && small_at_192 != 0,
                            format!("WM_GETICON: small {small:#x}, big {big:#x}, small at 192 DPI {small_at_192:#x}"),
                        );
                        report.check(
                            "icon cached",
                            send(hwnd, WindowsMessage::WM_GETICON, 0, 0) == small,
                            "the same question gets the same icon handle",
                        );
                        window.set_icon(None);
                        let none = send(hwnd, WindowsMessage::WM_GETICON, 0, 0);
                        report.check("icon removed", none == 0, format!("WM_GETICON after the icon was removed: {none:#x}"));
                    }
                    Err(error) => report.check("icon loaded", false, format!("{error}")),
                }

                println!("-- tray icon");
                // The platform creates a tray icon, which is given an icon
                // and a tip and added to the notification area. The shell
                // tells the message window of the platform about the mouse
                // over an icon; the message of a left button released over
                // the first icon of the thread, sent here, has to reach the
                // click action, and no longer once the icon is disposed.
                let platform = FerroLocator::current().get_required_service::<dyn IWindowingPlatform>();
                match platform.create_tray_icon() {
                    Some(tray) => {
                        let clicks = Rc::new(Cell::new(0u32));
                        {
                            let clicks = clicks.clone();
                            tray.set_on_clicked(Some(Rc::new(move || clicks.set(clicks.get() + 1))));
                        }
                        let icon = loader.load_icon_from_stream(&mut std::io::Cursor::new(icon_file())).ok();
                        let has_icon = icon.is_some();
                        tray.set_icon(icon);
                        tray.set_tool_tip_text(Some("FerroUI win32_window"));
                        tray.set_is_visible(true);
                        let send = ferroui_win32::interop::unmanaged_methods::send_message;
                        let tray_mouse = WindowsMessage::WM_USER + 1024;
                        send(Win32Platform::message_window(), tray_mouse, 1, WindowsMessage::WM_LBUTTONUP as isize);
                        send(Win32Platform::message_window(), tray_mouse, 1, WindowsMessage::WM_MOUSEMOVE as isize);
                        report.check(
                            "tray icon",
                            has_icon && clicks.get() == 1 && tray.menu_exporter().is_some(),
                            format!(
                                "created with an icon ({has_icon}), a tip and a menu exporter; a left button released over it reached the click action {} time(s)",
                                clicks.get()
                            ),
                        );
                        tray.set_is_visible(false);
                        tray.set_is_visible(true);
                        tray.dispose();
                        send(Win32Platform::message_window(), tray_mouse, 1, WindowsMessage::WM_LBUTTONUP as isize);
                        report.check(
                            "tray icon disposed",
                            clicks.get() == 1,
                            format!("{} click(s) after a message for the icon that was disposed", clicks.get()),
                        );
                    }
                    None => report.check("tray icon", false, "the platform creates no tray icon"),
                }

                println!("-- native control host");
                // The window offers a host of native controls. The host
                // makes a child window of its own kind (the default
                // child), which is attached to the window through a
                // holder; shown in bounds the child has their size in
                // pixels and is visible, hidden it is not.
                {
                    use ferroui_controls::platform::INativeControlHostImpl;
                    use ferroui_win32::interop::unmanaged_methods::{get_client_rect, is_window_visible};

                    let host = window
                        .try_get_feature(std::any::TypeId::of::<dyn INativeControlHostImpl>())
                        .and_then(|feature| feature.downcast::<Rc<dyn INativeControlHostImpl>>().ok());
                    match (host, window.handle()) {
                        (Some(host), Some(parent)) => {
                            let host: Rc<dyn INativeControlHostImpl> = (*host).clone();
                            let compatible = host.is_compatible_with(&*parent);
                            let child = host.create_default_child(parent);
                            let child_handle: Rc<dyn ferroui_controls::platform::IPlatformHandle> = child.clone();
                            let attachment = host.create_new_attachment(child_handle);
                            let scaling = window.render_scaling();
                            attachment.show_in_bounds(Rect::new(20.0, 30.0, 120.0, 60.0));
                            let shown = get_client_rect(child.handle());
                            let visible = is_window_visible(child.handle());
                            let expected = ((120.0 * scaling) as i32, (60.0 * scaling) as i32);
                            attachment.hide_with_size(Size::new(120.0, 60.0));
                            let hidden = is_window_visible(child.handle());
                            report.check(
                                "native control host",
                                compatible
                                    && child.handle() != 0
                                    && (shown.right - shown.left, shown.bottom - shown.top) == expected
                                    && visible
                                    && !hidden
                                    && attachment.attached_to().is_some(),
                                format!(
                                    "a child window {:#x} attached; shown in 120 by 60 at scaling {scaling} it is {} by {} pixels (visible: {visible}); visible after hiding: {hidden}",
                                    child.handle(),
                                    shown.right - shown.left,
                                    shown.bottom - shown.top
                                ),
                            );
                            attachment.dispose();
                            child.destroy();
                        }
                        (host, _) => report.check(
                            "native control host",
                            false,
                            format!("the window offers a host of native controls: {}", host.is_some()),
                        ),
                    }
                }

                println!("-- extended client area");
                // The client area extended into the frame: the window
                // reports it with a margin for the title bar, the caption
                // becomes client area, and the hit test of the frame is
                // the one of the custom caption procedure (the top resize
                // border, the title bar below it, the left border, the
                // client area in the middle). Taken back, the frame is the
                // one of the system again.
                {
                    use ferroui_win32::interop::unmanaged_methods::{
                        get_client_rect, get_window_rect, send_message, HitTestValues, WindowsMessage,
                    };

                    let changes = Rc::new(RefCell::new(Vec::new()));
                    window.set_extend_client_area_to_decorations_changed(Some(Rc::new({
                        let changes = changes.clone();
                        move |extended| changes.borrow_mut().push(extended)
                    })));
                    let hit = |x: i32, y: i32| send_message(hwnd, WindowsMessage::WM_NCHITTEST, 0, make_l_param(x, y)) as i32;
                    let height = |rect: ferroui_win32::interop::unmanaged_methods::RECT| rect.bottom - rect.top;
                    let scaling = window.render_scaling();
                    let client_before = get_client_rect(hwnd);

                    window.set_extend_client_area_to_decorations_hint(true);

                    let extended = window.is_client_area_extended_to_decorations();
                    let margins = window.extended_margins();
                    let bounds = get_window_rect(hwnd);
                    let client = get_client_rect(hwnd);
                    // The width of the resize border at a side.
                    let frame = ((bounds.right - bounds.left) - (client.right - client.left)) / 2;
                    let center_x = (bounds.left + bounds.right) / 2;
                    let center_y = (bounds.top + bounds.bottom) / 2;
                    let top_edge = hit(center_x, bounds.top + 1);
                    let title = hit(center_x, bounds.top + frame + 4);
                    let left_edge = hit(bounds.left + 1, center_y);
                    let middle = hit(center_x, center_y);
                    report.check(
                        "extended client area",
                        extended
                            && margins.top > 0.0
                            && height(client) > height(client_before)
                            && height(client) == height(bounds) - frame,
                        format!(
                            "extended: {extended}; margins {margins:?} at scaling {scaling}; the client area is {} pixels high (before: {}), the window {} with a border of {frame}",
                            height(client),
                            height(client_before),
                            height(bounds)
                        ),
                    );
                    report.check(
                        "hit test of the extended frame",
                        top_edge == HitTestValues::HTTOP
                            && title == HitTestValues::HTCAPTION
                            && left_edge == HitTestValues::HTLEFT
                            && middle == HitTestValues::HTCLIENT,
                        format!(
                            "top edge {top_edge} (HTTOP is 12), title bar {title} (HTCAPTION is 2), left edge {left_edge} (HTLEFT is 10), middle {middle} (HTCLIENT is 1)"
                        ),
                    );

                    window.set_extend_client_area_to_decorations_hint(false);

                    let client_after = get_client_rect(hwnd);
                    let title_after = hit(center_x, bounds.top + frame + 4);
                    let changes = changes.borrow().clone();
                    report.check(
                        "extended client area taken back",
                        !window.is_client_area_extended_to_decorations()
                            && window.extended_margins().top == 0.0
                            && height(client_after) == height(client_before)
                            && title_after == HitTestValues::HTCAPTION
                            && changes.first() == Some(&true)
                            && changes.last() == Some(&false),
                        format!(
                            "the client area is {} pixels high again (before: {}); the caption answers {title_after}; the window reported {changes:?}",
                            height(client_after),
                            height(client_before)
                        ),
                    );
                    window.set_extend_client_area_to_decorations_changed(None);
                }

                println!("-- transparency levels and the theme of the frame");
                // The levels of a composition surface (blur, acrylic blur,
                // mica) are not taken by a window that presents through its
                // redirection bitmap: of a list that names them the window
                // takes the transparent level, which the desktop window
                // manager gives such a window, or keeps none. A list of
                // "none" alone is none.
                {
                    use ferroui_base::platform::PlatformThemeVariant;
                    use ferroui_controls::WindowTransparencyLevel;

                    window.set_transparency_level_hint(&[
                        WindowTransparencyLevel::mica(),
                        WindowTransparencyLevel::acrylic_blur(),
                        WindowTransparencyLevel::blur(),
                        WindowTransparencyLevel::transparent(),
                    ]);
                    let taken = window.transparency_level();
                    window.set_transparency_level_hint(&[WindowTransparencyLevel::none()]);
                    let none = window.transparency_level();
                    // Through Windows.UI.Composition the window has the
                    // effects of that mode: mica on Windows 11, acrylic
                    // blur before.
                    let expected = if WIN_UI_COMPOSITION.load(Ordering::SeqCst) {
                        if Win32Platform::windows_version().build >= 22000 {
                            vec![WindowTransparencyLevel::mica()]
                        } else {
                            vec![WindowTransparencyLevel::acrylic_blur()]
                        }
                    } else {
                        vec![WindowTransparencyLevel::transparent(), WindowTransparencyLevel::none()]
                    };
                    report.check(
                        "transparency levels",
                        expected.contains(&taken) && none == WindowTransparencyLevel::none(),
                        format!(
                            "of mica, acrylic blur, blur and transparent the window took {taken:?} (expected one of {expected:?}); of none, {none:?}"
                        ),
                    );

                    // The frame in the dark and in the light theme, and
                    // back to the theme of the system: the attribute of the
                    // desktop window manager is set (Windows 11); nothing
                    // reads the frame back.
                    window.set_frame_theme_variant(Some(PlatformThemeVariant::Dark));
                    window.set_frame_theme_variant(Some(PlatformThemeVariant::Light));
                    window.set_frame_theme_variant(None);
                    report.info("theme of the frame", "set to dark, to light and to the theme of the system");
                }

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
        if smoke {
            // A run without a person must end: a panic inside a callback of the dispatcher is
            // caught there and would leave the steps of the run waiting until the job is
            // stopped (run 38065160851 waited five minutes after one). The message and the
            // backtrace are printed, then the process ends with the code of a panic.
            let default_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                default_hook(info);
                std::process::exit(101);
            }));
        }
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

        // `--composition redirection|dcomp|winui`: the one composition mode of
        // the run (with ANGLE), without a fallback. The default is the
        // redirection surface of the window.
        let composition = arguments
            .iter()
            .position(|argument| argument == "--composition")
            .and_then(|index| arguments.get(index + 1))
            .map_or("redirection", String::as_str);
        let composition_mode = match composition {
            "redirection" => Win32CompositionMode::RedirectionSurface,
            "dcomp" => Win32CompositionMode::DirectComposition,
            "winui" => Win32CompositionMode::WinUIComposition,
            other => {
                eprintln!("win32_window: unknown composition mode '{other}' (redirection, dcomp, winui)");
                return ExitCode::from(2);
            }
        };
        if rendering_mode == Win32RenderingMode::AngleEgl {
            println!("Composition mode: {composition_mode:?}");
        }
        let composed = rendering_mode == Win32RenderingMode::AngleEgl && composition_mode != Win32CompositionMode::RedirectionSurface;

        let options = Win32PlatformOptions {
            rendering_mode: vec![rendering_mode],
            composition_mode: vec![composition_mode],
            ..Win32PlatformOptions::default()
        };
        Win32Platform::initialize(options);
        VelloPlatform::initialize_with_options(VelloOptions::with_rendering_mode(VelloRenderingMode::Cpu));

        // The compositor of the Windows Runtime commits what was changed when its thread asks it
        // to, and the thread of the mode asks while the render loop has something to render: a
        // renderer of the framework renders in the tick of that loop. This example draws on the
        // UI thread from a timer of the dispatcher, so it gives the loop a task that always wants
        // the next tick; its frames then reach the screen with the next commit.
        if composed && composition_mode == Win32CompositionMode::WinUIComposition {
            WIN_UI_COMPOSITION.store(true, Ordering::SeqCst);
            struct KeepCommitting;
            impl ferroui_base::rendering::IRenderLoopTask for KeepCommitting {
                fn render(&self) -> bool {
                    true
                }
            }
            match FerroLocator::current().get_service::<Arc<dyn ferroui_base::rendering::IRenderLoop>>() {
                Some(render_loop) => render_loop.add(Arc::new(KeepCommitting)),
                None => {
                    eprintln!("win32_window: the composition mode registered no render loop");
                    return ExitCode::from(1);
                }
            }
        }

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
            // With ANGLE a window has a third surface: its OpenGL surface,
            // or in a composition mode the surface of that mode, which is
            // not an OpenGL surface itself.
            let expected_gl_surfaces = usize::from(rendering_mode == Win32RenderingMode::AngleEgl && !composed);
            let expected_surfaces = 2 + usize::from(rendering_mode == Win32RenderingMode::AngleEgl);
            report.check(
                "surfaces",
                surfaces.len() == expected_surfaces
                    && gl_surfaces == expected_gl_surfaces
                    && surfaces.iter().any(|surface| surface.as_framebuffer_surface().is_some()),
                format!(
                    "{} surface(s): the window handle, {gl_surfaces} OpenGL surface(s){} and the framebuffer",
                    surfaces.len(),
                    if composed { ", the surface of the composition mode" } else { "" }
                ),
            );
            if composed {
                // A window of a composition mode has no redirection bitmap.
                const GWL_EXSTYLE: i32 = -20;
                const WS_EX_NOREDIRECTIONBITMAP: isize = 0x0020_0000;
                #[link(name = "user32")]
                extern "system" {
                    fn GetWindowLongPtrW(hwnd: isize, index: i32) -> isize;
                }
                // SAFETY: a window handle of this thread and an index of
                // the system.
                let ex_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
                report.check(
                    "no redirection bitmap",
                    ex_style & WS_EX_NOREDIRECTIONBITMAP != 0,
                    format!("the extended styles of the window are {ex_style:#x}"),
                );
            }
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
                            check_presented(&state, hwnd, "presented pixels reach the last column and row after showing");
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
                            drop(resizes);
                            check_presented(&state, hwnd, "presented pixels reach the last column and row after the resize");
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
        println!("the window is released");

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
