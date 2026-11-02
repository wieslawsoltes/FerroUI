//! An application on the framebuffer platform that checks itself: the
//! Skia renderer and the Simple theme, one view filled with one colour with
//! a marker of another colour in its top-left corner, on
//! a framebuffer that is a plain file or a (virtual) framebuffer device
//! (drawn in software), or on a KMS output of a (virtual) card (drawn with
//! OpenGL through EGL on GBM), with input from virtual devices it creates.
//!
//! ```text
//! cargo run -p ferroui-linuxframebuffer --features example --example linuxfb_smoke -- --file /tmp/fb.raw
//! sudo .../linuxfb_smoke --fbdev /dev/fb1 --expect-id vkms --uinput
//! sudo .../linuxfb_smoke --drm /dev/dri/card1 --expect-driver vkms
//! ```
//!
//! - `--file <path>`: the framebuffer is a file of 640 by 400 pixels
//!   (`--size=WxH`, `--format=bgra|rgba|rgb565`); works on every Linux
//!   machine and needs no privileges.
//! - `--fbdev <node> --expect-id <prefix>`: a framebuffer device of the
//!   kernel. The example writes to it only when the identification the
//!   device reports begins with the prefix, so that a virtual device
//!   (`Virtual FB` of `vfb`, `vkms` of the emulation of `vkms`) cannot be
//!   mistaken for the console of the machine.
//! - `--drm <card> --expect-driver <name>`: the DRM output on the card.
//!   The example sets a mode on it only when the name of its driver is the
//!   one given (`vkms` for the virtual KMS driver). The frame is read back
//!   from the framebuffer the CRTC scans out (`output::drm_scanout`), so
//!   the check sees what the card shows. With `--orientation=90|180|270`
//!   the scene is drawn rotated (the rotation pass of the output), and the
//!   check expects the marker in the corner of the card's frame that the
//!   rotation turns the scene's top-left corner into.
//! - `--direct`, `--async`: draw directly into the mapped memory; copy
//!   frames on the blit thread.
//! - `--uinput`: create a touch screen, a keyboard, a mouse and a
//!   multi-touch screen through `/dev/uinput` (needs access to it), read
//!   them with the evdev backend and check that the events written to
//!   them arrive as raw input.
//! - `--libinput` (with `--uinput`): read the multi-touch screen through
//!   the `libinput` backend instead (the library must be installed).
//! - `--dump <path>`: write the content of the framebuffer, as raw bytes,
//!   when the checks are done.
//!
//! Every check prints `[ ok ]` or `[FAILED]`; the last line is
//! `SMOKE PASSED` or `SMOKE FAILED` and the exit code follows it.

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("linuxfb_smoke: this example needs Linux");
}

#[cfg(target_os = "linux")]
fn main() -> std::process::ExitCode {
    app::run()
}

#[cfg(target_os = "linux")]
mod app {
    use ferroui_base::input::raw::{
        RawKeyEventArgs, RawKeyEventType, RawMouseWheelEventArgs, RawPointerEventArgs, RawTextInputEventArgs,
        RawTouchEventArgs,
    };
    use ferroui_base::input::IInputRoot;
    use ferroui_base::media::immutable::ImmutableSolidColorBrush;
    use ferroui_base::media::Color;
    use ferroui_base::platform::{PixelFormat, SurfaceOrientation};
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::styling::Styles;
    use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
    use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
    use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Ref};
    use ferroui_controls::application_lifetimes::IApplicationLifetime;
    use ferroui_controls::{
        AppBuilder, Application, ApplicationImpl, ApplicationImplExt, Border, Control, NewApplication,
    };
    use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
    use ferroui_linuxframebuffer::input::ev_dev::{
        EvDevBackend, EvDevDeviceDescription, EvDevKeyboardDeviceDescription, EvDevMouseDeviceDescription,
        EvDevMultiTouchScreenDeviceDescription, EvDevTouchScreenDeviceDescription,
    };
    use ferroui_linuxframebuffer::input::lib_input::{LibInputBackend, LibInputBackendOptions};
    use ferroui_linuxframebuffer::input::null_input::NullInputBackend;
    use ferroui_linuxframebuffer::input::{IInputBackend, IScreenInfoProvider};
    use ferroui_linuxframebuffer::native_unsafe_methods::{
        abs_axis, axis_event_code, ev_key, ev_type, fb_bitfield, fb_fix_screeninfo, fb_var_screeninfo, ioc, syn_code,
        IOC_READ, IOC_WRITE,
    };
    use ferroui_linuxframebuffer::output::{
        drm_scanout, DrmCard, DrmOutput, FbDevOutputOptions, FbdevOutput, FileFbDevice, IFbDevice, IOutputBackend,
        LinuxFbDevice,
    };
    use ferroui_linuxframebuffer::raw_event_grouping::RawEvent;
    use ferroui_linuxframebuffer::{DrmOutputOptions, LinuxFramebufferPlatformExtensions};
    use ferroui_skia::SkiaApplicationExtensions;
    use ferroui_themes_simple::SimpleTheme;
    use std::cell::{Cell, RefCell};
    use std::fs::File;
    use std::io::{Read, Write};
    use std::os::fd::AsRawFd;
    use std::process::ExitCode;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    /// The colour the view is filled with.
    const FILL: (u8, u8, u8) = (0x33, 0x66, 0x99);
    /// The colour of the marker in the top-left corner of the view, and its
    /// size in device pixels (the runs use the scaling 1). The marker makes
    /// the frame asymmetric, so that the checks see where the scene's
    /// top-left corner was put (a rotation of the DRM output moves it).
    const MARKER: (u8, u8, u8) = (0xCC, 0x33, 0x00);
    const MARKER_WIDTH: f64 = 96.0;
    const MARKER_HEIGHT: f64 = 64.0;
    const RUN_TIMEOUT: Duration = Duration::from_secs(60);

    /// Whether every check passed.
    static SMOKE_PASSED: AtomicBool = AtomicBool::new(false);

    /// Where the frame is read back from.
    enum FrameSource {
        /// The node of a framebuffer device, or the file that stands for one.
        Fb { node: String, var_info: fb_var_screeninfo, fixed_info: fb_fix_screeninfo },
        /// The framebuffer the CRTC of a DRM output scans out.
        Drm(Arc<DrmOutput>),
    }

    /// A frame as read back: rows of `pitch` bytes.
    struct Frame {
        data: Vec<u8>,
        width: u32,
        height: u32,
        pitch: u32,
        bits_per_pixel: u32,
        /// Red in the low byte (else blue, as `BGRA8888` and `XRGB8888`).
        rgba: bool,
    }

    /// What the application needs of the command line and of `run`.
    struct Smoke {
        source: FrameSource,
        /// How the output turns the scene (the DRM output's option; the
        /// other outputs do not turn it).
        orientation: SurfaceOrientation,
        /// The size of the frame in pixels.
        width: u32,
        height: u32,
        dump: Option<String>,
        devices: Option<VirtualDevices>,
        /// Whether the devices are read through the `libinput` backend.
        libinput: bool,
        log: Rc<RefCell<Vec<Logged>>>,
        failed: Cell<u32>,
        passed: Cell<u32>,
    }

    ferroui_base::ferro_thread_local! {
        static SMOKE: RefCell<Option<Rc<Smoke>>> = const { RefCell::new(None) };
        /// The timer of the checks, kept for the length of the run.
        static TIMER: RefCell<Option<Rc<dyn IDisposable>>> = const { RefCell::new(None) };
    }

    impl Smoke {
        fn check(&self, name: &str, passed: bool, detail: String) {
            println!("  [{}] {name}: {detail}", if passed { " ok " } else { "FAILED" });
            let counter = if passed { &self.passed } else { &self.failed };
            counter.set(counter.get() + 1);
        }
    }

    #[repr(C)]
    pub struct App {
        base: Application,
    }

    ferro_class!(App: Application);
    ferro_impl_classes!(App: FerroObjectImpl);

    impl NewApplication for App {
        fn new_application() -> Ref<Self> {
            instantiate(Self { base: Application::construct() })
        }
    }

    impl ApplicationImpl for App {
        fn initialize(this: &Self) {
            Self::parent_initialize(this);
            this.styles().add(SimpleTheme::new().upcast::<Styles>());
        }

        fn on_framework_initialization_completed(this: &Self) {
            let lifetime = this.application_lifetime();
            if let Some(single_view) =
                lifetime.as_ref().and_then(|lifetime| lifetime.as_single_view_application_lifetime())
            {
                let border = Border::new();
                border.set_background(Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(
                    FILL.0, FILL.1, FILL.2,
                )))));
                let marker = Border::new();
                marker.set_background(Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(
                    MARKER.0, MARKER.1, MARKER.2,
                )))));
                marker.set_width(MARKER_WIDTH);
                marker.set_height(MARKER_HEIGHT);
                marker.set_horizontal_alignment(HorizontalAlignment::Left);
                marker.set_vertical_alignment(VerticalAlignment::Top);
                border.set_child(Some(marker.upcast::<Control>()));
                single_view.set_main_view(Some(border.upcast::<Control>()));
            }

            if let (Some(smoke), Some(lifetime)) = (SMOKE.with(|smoke| smoke.borrow().clone()), lifetime) {
                start_checks(smoke, lifetime);
            }

            Self::parent_on_framework_initialization_completed(this);
        }
    }

    /// A raw input event as the checks compare it.
    #[derive(Clone, Debug)]
    struct Logged {
        what: String,
        position: Option<Point>,
    }

    fn logged(what: impl Into<String>, x: f64, y: f64) -> Logged {
        Logged { what: what.into(), position: Some(Point::new(x, y)) }
    }

    fn logged_without_position(what: impl Into<String>) -> Logged {
        Logged { what: what.into(), position: None }
    }

    fn same(expected: &[Logged], actual: &[Logged]) -> bool {
        expected.len() == actual.len()
            && expected.iter().zip(actual).all(|(expected, actual)| {
                expected.what == actual.what
                    && match (expected.position, actual.position) {
                        (Some(a), Some(b)) => (a.x - b.x).abs() <= 1.0 && (a.y - b.y).abs() <= 1.0,
                        (None, None) => true,
                        _ => false,
                    }
            })
    }

    fn describe(e: &RawEvent) -> Logged {
        if let Some(touch) = e.downcast_ref::<RawTouchEventArgs>() {
            let position = touch.position();
            return logged(format!("touch {:?} {}", touch.type_(), touch.raw_pointer_id()), position.x, position.y);
        }
        if let Some(wheel) = e.downcast_ref::<RawMouseWheelEventArgs>() {
            let delta = wheel.delta();
            return logged_without_position(format!("wheel {} {}", delta.x, delta.y));
        }
        if let Some(pointer) = e.downcast_ref::<RawPointerEventArgs>() {
            let position = pointer.position();
            return logged(format!("pointer {:?}", pointer.type_()), position.x, position.y);
        }
        if let Some(key) = e.downcast_ref::<RawKeyEventArgs>() {
            let direction = if key.type_() == RawKeyEventType::KeyDown { "down" } else { "up" };
            return logged_without_position(format!("key {direction} {:?}", key.key()));
        }
        if let Some(text) = e.downcast_ref::<RawTextInputEventArgs>() {
            return logged_without_position(format!("text {:?}", text.text()));
        }
        logged_without_position("other")
    }

    /// An input backend that notes every raw event of another one.
    struct RecordingBackend {
        inner: Rc<dyn IInputBackend>,
        log: Rc<RefCell<Vec<Logged>>>,
    }

    impl IInputBackend for RecordingBackend {
        fn initialize(&self, info: Rc<dyn IScreenInfoProvider>, on_input: Rc<dyn Fn(RawEvent)>) {
            let log = self.log.clone();
            self.inner.initialize(
                info,
                Rc::new(move |e: RawEvent| {
                    log.borrow_mut().push(describe(&e));
                    on_input(e);
                }),
            );
        }

        fn set_input_root(&self, root: Rc<dyn IInputRoot>) {
            self.inner.set_input_root(root);
        }
    }

    /// A virtual input device, created through `/dev/uinput`.
    struct UinputDevice {
        file: File,
        event_node: String,
    }

    const UI_DEV_CREATE: u32 = ioc(0, b'U', 1, 0);
    const UI_DEV_DESTROY: u32 = ioc(0, b'U', 2, 0);
    const UI_DEV_SETUP: u32 = ioc(IOC_WRITE, b'U', 3, std::mem::size_of::<libc::uinput_setup>());
    const UI_ABS_SETUP: u32 = ioc(IOC_WRITE, b'U', 4, std::mem::size_of::<libc::uinput_abs_setup>());
    const UI_SET_EVBIT: u32 = ioc(IOC_WRITE, b'U', 100, 4);
    const UI_SET_KEYBIT: u32 = ioc(IOC_WRITE, b'U', 101, 4);
    const UI_SET_RELBIT: u32 = ioc(IOC_WRITE, b'U', 102, 4);
    const UI_SET_ABSBIT: u32 = ioc(IOC_WRITE, b'U', 103, 4);
    const UI_SET_PROPBIT: u32 = ioc(IOC_WRITE, b'U', 110, 4);
    /// `INPUT_PROP_DIRECT`: the device is on the screen it controls.
    const INPUT_PROP_DIRECT: u16 = 1;

    impl UinputDevice {
        fn set_bit(file: &File, request: u32, bit: u16) -> std::io::Result<()> {
            // SAFETY: the request takes an integer argument.
            let rv = unsafe { libc::ioctl(file.as_raw_fd(), request as _, bit as libc::c_int) };
            if rv < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        }

        fn create(name: &str, keys: &[u16], rel: &[u16], abs: &[(u16, i32, i32)]) -> std::io::Result<Self> {
            let file = std::fs::OpenOptions::new().write(true).open("/dev/uinput")?;
            let fd = file.as_raw_fd();
            Self::set_bit(&file, UI_SET_EVBIT, ev_type::EV_SYN)?;
            if !keys.is_empty() {
                Self::set_bit(&file, UI_SET_EVBIT, ev_type::EV_KEY)?;
            }
            for &key in keys {
                Self::set_bit(&file, UI_SET_KEYBIT, key)?;
            }
            if !rel.is_empty() {
                Self::set_bit(&file, UI_SET_EVBIT, ev_type::EV_REL)?;
            }
            for &axis in rel {
                Self::set_bit(&file, UI_SET_RELBIT, axis)?;
            }
            if !abs.is_empty() {
                Self::set_bit(&file, UI_SET_EVBIT, ev_type::EV_ABS)?;
                Self::set_bit(&file, UI_SET_PROPBIT, INPUT_PROP_DIRECT)?;
            }
            for &(axis, minimum, maximum) in abs {
                Self::set_bit(&file, UI_SET_ABSBIT, axis)?;
                // SAFETY: an all-zero structure is valid; it is local and
                // the request reads one of its size.
                unsafe {
                    let mut setup: libc::uinput_abs_setup = std::mem::zeroed();
                    setup.code = axis;
                    setup.absinfo.minimum = minimum;
                    setup.absinfo.maximum = maximum;
                    if axis != abs_axis::ABS_MT_SLOT && axis != abs_axis::ABS_MT_TRACKING_ID {
                        setup.absinfo.resolution = 10;
                    }
                    if libc::ioctl(fd, UI_ABS_SETUP as _, &setup) < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                }
            }

            // SAFETY: as above, for the description of the device; then
            // the request that creates it, which has no argument.
            unsafe {
                let mut setup: libc::uinput_setup = std::mem::zeroed();
                setup.id.bustype = 0x06; // BUS_VIRTUAL
                setup.id.vendor = 0x1209;
                setup.id.product = 0x0001;
                for (target, source) in setup.name.iter_mut().zip(name.bytes().take(79)) {
                    *target = source as _;
                }
                if libc::ioctl(fd, UI_DEV_SETUP as _, &setup) < 0 || libc::ioctl(fd, UI_DEV_CREATE as _) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }

            // The name of the device in sysfs, and from it the event node.
            let mut sysname = [0u8; 64];
            // SAFETY: the length of the buffer is part of the request.
            let rv = unsafe { libc::ioctl(fd, ioc(IOC_READ, b'U', 44, sysname.len()) as _, sysname.as_mut_ptr()) };
            if rv < 0 {
                return Err(std::io::Error::last_os_error());
            }
            let sysname =
                String::from_utf8_lossy(&sysname[..sysname.iter().position(|&b| b == 0).unwrap_or(0)]).into_owned();
            let started = Instant::now();
            loop {
                let event = std::fs::read_dir(format!("/sys/devices/virtual/input/{sysname}"))?
                    .filter_map(|entry| entry.ok().map(|entry| entry.file_name().to_string_lossy().into_owned()))
                    .find(|name| name.starts_with("event"));
                if let Some(event) = event {
                    let event_node = format!("/dev/input/{event}");
                    if std::path::Path::new(&event_node).exists() {
                        return Ok(Self { file, event_node });
                    }
                }
                if started.elapsed() > Duration::from_secs(5) {
                    return Err(std::io::Error::other(format!("no event node appeared for {sysname}")));
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }

        fn emit(&self, type_: u16, code: u16, value: i32) {
            // SAFETY: an all-zero event is valid.
            let mut ev: libc::input_event = unsafe { std::mem::zeroed() };
            ev.type_ = type_;
            ev.code = code;
            ev.value = value;
            // SAFETY: the bytes of a plain structure, for its size.
            let bytes = unsafe {
                std::slice::from_raw_parts((&ev as *const libc::input_event).cast::<u8>(), std::mem::size_of_val(&ev))
            };
            let _ = (&self.file).write_all(bytes);
        }

        /// One report: the events and the `SYN_REPORT` that ends it.
        fn report(&self, events: &[(u16, u16, i32)]) {
            for &(type_, code, value) in events {
                self.emit(type_, code, value);
            }
            self.emit(ev_type::EV_SYN, syn_code::SYN_REPORT, 0);
        }
    }

    impl Drop for UinputDevice {
        fn drop(&mut self) {
            // SAFETY: the request has no argument.
            unsafe {
                libc::ioctl(self.file.as_raw_fd(), UI_DEV_DESTROY as _);
            }
        }
    }

    struct VirtualDevices {
        touch: UinputDevice,
        keyboard: UinputDevice,
        mouse: UinputDevice,
        multi_touch: UinputDevice,
    }

    const KEY_A: u16 = 30;
    const KEY_LEFTSHIFT: u16 = 42;
    const KEY_ENTER: u16 = 28;

    impl VirtualDevices {
        fn create() -> std::io::Result<Self> {
            let touch = UinputDevice::create(
                "FerroUI smoke touch screen",
                &[ev_key::BTN_TOUCH],
                &[],
                &[(abs_axis::ABS_X, 0, 1000), (abs_axis::ABS_Y, 0, 1000)],
            )?;
            let keys: Vec<u16> = (1..=120).collect();
            let keyboard = UinputDevice::create("FerroUI smoke keyboard", &keys, &[], &[])?;
            let mouse = UinputDevice::create(
                "FerroUI smoke mouse",
                &[ev_key::BTN_LEFT, ev_key::BTN_RIGHT, ev_key::BTN_MIDDLE],
                &[axis_event_code::REL_X, axis_event_code::REL_Y, axis_event_code::REL_WHEEL],
                &[],
            )?;
            let multi_touch = UinputDevice::create(
                "FerroUI smoke multi-touch screen",
                &[ev_key::BTN_TOUCH],
                &[],
                &[
                    (abs_axis::ABS_MT_SLOT, 0, 9),
                    (abs_axis::ABS_MT_TRACKING_ID, 0, 65535),
                    (abs_axis::ABS_MT_POSITION_X, 0, 1000),
                    (abs_axis::ABS_MT_POSITION_Y, 0, 1000),
                ],
            )?;
            Ok(Self { touch, keyboard, mouse, multi_touch })
        }

        fn descriptions(&self) -> Vec<EvDevDeviceDescription> {
            vec![
                EvDevDeviceDescription::TouchScreen(EvDevTouchScreenDeviceDescription {
                    path: Some(self.touch.event_node.clone()),
                    ..Default::default()
                }),
                EvDevDeviceDescription::Keyboard(EvDevKeyboardDeviceDescription {
                    path: Some(self.keyboard.event_node.clone()),
                }),
                EvDevDeviceDescription::Mouse(EvDevMouseDeviceDescription {
                    path: Some(self.mouse.event_node.clone()),
                }),
                EvDevDeviceDescription::MultiTouchScreen(EvDevMultiTouchScreenDeviceDescription {
                    path: Some(self.multi_touch.event_node.clone()),
                    ..Default::default()
                }),
            ]
        }
    }

    /// The frame as the device shows it: the bytes of the framebuffer,
    /// read from its node, or the framebuffer the CRTC scans out.
    fn read_framebuffer(smoke: &Smoke) -> std::io::Result<Frame> {
        match &smoke.source {
            FrameSource::Fb { node, var_info, fixed_info } => {
                let length = fixed_info.line_length as u64 * var_info.yres as u64;
                let mut content = Vec::new();
                File::open(node)?.take(length).read_to_end(&mut content)?;
                Ok(Frame {
                    data: content,
                    width: var_info.xres,
                    height: var_info.yres,
                    pitch: fixed_info.line_length,
                    bits_per_pixel: var_info.bits_per_pixel,
                    rgba: var_info.blue.offset == 16,
                })
            }
            FrameSource::Drm(output) => {
                let scanout = drm_scanout::read_scanout(output.card().fd(), output.crtc_id())?;
                Ok(Frame {
                    data: scanout.data,
                    width: scanout.width,
                    height: scanout.height,
                    pitch: scanout.pitch,
                    bits_per_pixel: scanout.bits_per_pixel,
                    rgba: false,
                })
            }
        }
    }

    /// The colour of a pixel of the framebuffer as red, green and blue.
    fn pixel(frame: &Frame, x: u32, y: u32) -> Option<(u8, u8, u8)> {
        let content = &frame.data;
        let bytes = frame.bits_per_pixel as usize / 8;
        let offset = y as usize * frame.pitch as usize + x as usize * bytes;
        let px = content.get(offset..offset + bytes)?;
        if bytes == 2 {
            // The 16-bit format of the renderer: five bits of red at the top.
            let value = u16::from_le_bytes([px[0], px[1]]);
            let expand = |v: u16, bits: u32| ((v as u32 * 255) / ((1 << bits) - 1)) as u8;
            return Some((expand(value >> 11, 5), expand((value >> 5) & 0x3f, 6), expand(value & 0x1f, 5)));
        }
        if frame.rgba {
            Some((px[0], px[1], px[2]))
        } else {
            Some((px[2], px[1], px[0]))
        }
    }

    fn is_colour(colour: Option<(u8, u8, u8)>, expected: (u8, u8, u8)) -> bool {
        let close = |a: u8, b: u8| (a as i32 - b as i32).abs() <= 8;
        colour.is_some_and(|(r, g, b)| close(r, expected.0) && close(g, expected.1) && close(b, expected.2))
    }

    /// A corner of the frame as the device holds it.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Corner {
        TopLeft,
        TopRight,
        BottomRight,
        BottomLeft,
    }

    /// The corner of the device's frame the scene's top-left corner is
    /// shown in: the DRM output turns the scene clockwise by its
    /// orientation (`rotation_vertices` of `drm_output.rs`); the other
    /// outputs do not turn it.
    fn marker_corner(orientation: SurfaceOrientation) -> Corner {
        match orientation {
            SurfaceOrientation::Rotation0 => Corner::TopLeft,
            SurfaceOrientation::Rotation90 => Corner::TopRight,
            SurfaceOrientation::Rotation180 => Corner::BottomRight,
            SurfaceOrientation::Rotation270 => Corner::BottomLeft,
        }
    }

    /// A pixel well inside the marker in a corner of a frame of `width` by
    /// `height` (the marker is at least 64 pixels on each side, turned or
    /// not), and the corner it is in.
    fn corner_points(width: u32, height: u32) -> [(Corner, (u32, u32)); 4] {
        let inset = 16;
        let (right, bottom) = (width.saturating_sub(1 + inset), height.saturating_sub(1 + inset));
        [
            (Corner::TopLeft, (inset, inset)),
            (Corner::TopRight, (right, inset)),
            (Corner::BottomRight, (right, bottom)),
            (Corner::BottomLeft, (inset, bottom)),
        ]
    }

    /// The steps of the run, one per tick of a timer on the UI thread.
    fn start_checks(smoke: Rc<Smoke>, lifetime: Rc<dyn IApplicationLifetime>) {
        println!("Phase: frame");
        let started = Instant::now();
        let step = Cell::new(0u32);
        let waited = Cell::new(0u32);
        let (width, height) = (smoke.width, smoke.height);
        let (w, h) = (width as f64, height as f64);

        let finish = {
            let smoke = smoke.clone();
            move || {
                if let Some(dump) = &smoke.dump {
                    match read_framebuffer(&smoke).and_then(|frame| std::fs::write(dump, &frame.data).map(|()| frame)) {
                        Ok(frame) => println!(
                            "The framebuffer was written to {dump} ({}x{}, {} bits, line length {})",
                            frame.width, frame.height, frame.bits_per_pixel, frame.pitch
                        ),
                        Err(error) => smoke.check("dump", false, format!("{error}")),
                    }
                }
                SMOKE_PASSED.store(smoke.failed.get() == 0 && smoke.passed.get() > 0, Ordering::SeqCst);
                if let Some(controlled) = lifetime.as_controlled_application_lifetime() {
                    controlled.shutdown(0);
                }
            }
        };

        // What is written to a device in a step, and what must have
        // arrived a few ticks later.
        let expect = {
            let smoke = smoke.clone();
            move |name: &str, expected: Vec<Logged>| {
                let actual = std::mem::take(&mut *smoke.log.borrow_mut());
                let passed = same(&expected, &actual);
                let detail = if passed {
                    format!("{} raw events, as expected", actual.len())
                } else {
                    format!("expected {expected:?}, received {actual:?}")
                };
                smoke.check(name, passed, detail);
            }
        };

        let timer = DispatcherTimer::run(
            move || {
                match step.get() {
                    0 => {
                        // The frame: the middle and the corners have the colour of the view.
                        let frame = read_framebuffer(&smoke);
                        // The points of the frame as the device holds it (a rotated scene is
                        // shown on a frame of the device's own size): the middle has the colour
                        // of the view, the corner the orientation puts the scene's top-left
                        // corner in has the marker, the three other corners the view's colour.
                        let marker = marker_corner(smoke.orientation);
                        let mut colours: Vec<(String, Option<(u8, u8, u8)>, (u8, u8, u8))> = Vec::new();
                        if let Ok(frame) = &frame {
                            let (fw, fh) = (frame.width.max(1), frame.height.max(1));
                            colours.push(("middle".to_owned(), pixel(frame, fw / 2, fh / 2), FILL));
                            for (corner, (x, y)) in corner_points(fw, fh) {
                                let expected = if corner == marker { MARKER } else { FILL };
                                colours.push((format!("{corner:?}"), pixel(frame, x, y), expected));
                            }
                        }
                        let arrived = !colours.is_empty()
                            && colours.iter().all(|(_, colour, expected)| is_colour(*colour, *expected));
                        // The DRM output creates its EGL objects with the first frame, which
                        // takes a while on a software renderer.
                        let limit = if matches!(smoke.source, FrameSource::Drm(_)) { 30 } else { 10 };
                        if !arrived && started.elapsed() < Duration::from_secs(limit) {
                            return true;
                        }
                        if let Err(error) = &frame {
                            println!("  the frame could not be read back: {error}");
                        }
                        smoke.check(
                            "frame",
                            arrived,
                            format!(
                                "the pixels (where, read, expected) are {colours:?} after {} ms (the view is filled with {FILL:?}, its top-left marker {MARKER:?} is expected at {marker:?} for {:?})",
                                started.elapsed().as_millis(),
                                smoke.orientation
                            ),
                        );
                        if smoke.devices.is_none() {
                            finish();
                            return false;
                        }
                        println!("Phase: input");
                        smoke.log.borrow_mut().clear();
                    }
                    step_number => {
                        let Some(devices) = &smoke.devices else {
                            return false;
                        };
                        // Odd steps write, even steps look (three ticks later).
                        // The library takes a moment to add its device.
                        if smoke.libinput && step_number == 7 && waited.get() < 20 {
                            waited.set(waited.get() + 1);
                            return true;
                        }
                        if step_number % 2 == 0 {
                            if waited.get() < 3 {
                                waited.set(waited.get() + 1);
                                return true;
                            }
                            waited.set(0);
                        }
                        match step_number {
                            1 => {
                                let touch = &devices.touch;
                                touch.report(&[
                                    (ev_type::EV_ABS, abs_axis::ABS_X, 500),
                                    (ev_type::EV_ABS, abs_axis::ABS_Y, 250),
                                    (ev_type::EV_KEY, ev_key::BTN_TOUCH, 1),
                                ]);
                                touch.report(&[(ev_type::EV_ABS, abs_axis::ABS_X, 600)]);
                                touch.report(&[(ev_type::EV_KEY, ev_key::BTN_TOUCH, 0)]);
                            }
                            2 => expect(
                                "touch screen",
                                vec![
                                    logged("touch TouchBegin 1", w * 0.5, h * 0.25),
                                    logged("touch TouchUpdate 1", w * 0.6, h * 0.25),
                                    logged("touch TouchEnd 1", w * 0.6, h * 0.25),
                                ],
                            ),
                            3 => {
                                let keyboard = &devices.keyboard;
                                keyboard.report(&[(ev_type::EV_KEY, KEY_A, 1)]);
                                keyboard.report(&[(ev_type::EV_KEY, KEY_A, 0)]);
                                keyboard.report(&[(ev_type::EV_KEY, KEY_LEFTSHIFT, 1)]);
                                keyboard.report(&[(ev_type::EV_KEY, KEY_A, 1)]);
                                keyboard.report(&[(ev_type::EV_KEY, KEY_A, 0)]);
                                keyboard.report(&[(ev_type::EV_KEY, KEY_LEFTSHIFT, 0)]);
                                keyboard.report(&[(ev_type::EV_KEY, KEY_ENTER, 1)]);
                                keyboard.report(&[(ev_type::EV_KEY, KEY_ENTER, 0)]);
                            }
                            4 => expect(
                                "keyboard",
                                [
                                    "key down A",
                                    "text \"a\"",
                                    "key up A",
                                    "key down LeftShift",
                                    "key down A",
                                    "text \"A\"",
                                    "key up A",
                                    "key up LeftShift",
                                    "key down Return",
                                    "key up Return",
                                ]
                                .into_iter()
                                .map(logged_without_position)
                                .collect(),
                            ),
                            5 => {
                                let mouse = &devices.mouse;
                                mouse.report(&[
                                    (ev_type::EV_REL, axis_event_code::REL_X, 40),
                                    (ev_type::EV_REL, axis_event_code::REL_Y, 30),
                                ]);
                                mouse.report(&[(ev_type::EV_KEY, ev_key::BTN_LEFT, 1)]);
                                mouse.report(&[(ev_type::EV_KEY, ev_key::BTN_LEFT, 0)]);
                                mouse.report(&[(ev_type::EV_REL, axis_event_code::REL_WHEEL, -1)]);
                            }
                            6 => expect(
                                "mouse",
                                vec![
                                    logged("pointer Move", 40.0, 30.0),
                                    logged("pointer LeftButtonDown", 40.0, 30.0),
                                    logged("pointer LeftButtonUp", 40.0, 30.0),
                                    logged_without_position("wheel 0 -1"),
                                ],
                            ),
                            7 => {
                                waited.set(0);
                                let touch = &devices.multi_touch;
                                touch.report(&[
                                    (ev_type::EV_ABS, abs_axis::ABS_MT_SLOT, 0),
                                    (ev_type::EV_ABS, abs_axis::ABS_MT_TRACKING_ID, 10),
                                    (ev_type::EV_ABS, abs_axis::ABS_MT_POSITION_X, 100),
                                    (ev_type::EV_ABS, abs_axis::ABS_MT_POSITION_Y, 200),
                                    (ev_type::EV_ABS, abs_axis::ABS_MT_SLOT, 1),
                                    (ev_type::EV_ABS, abs_axis::ABS_MT_TRACKING_ID, 11),
                                    (ev_type::EV_ABS, abs_axis::ABS_MT_POSITION_X, 800),
                                    (ev_type::EV_ABS, abs_axis::ABS_MT_POSITION_Y, 700),
                                ]);
                                touch.report(&[(ev_type::EV_ABS, abs_axis::ABS_MT_POSITION_X, 900)]);
                                touch.report(&[(ev_type::EV_ABS, abs_axis::ABS_MT_TRACKING_ID, -1)]);
                                touch.report(&[
                                    (ev_type::EV_ABS, abs_axis::ABS_MT_SLOT, 0),
                                    (ev_type::EV_ABS, abs_axis::ABS_MT_TRACKING_ID, -1),
                                ]);
                            }
                            8 => expect(
                                "multi-touch screen",
                                vec![
                                    logged("touch TouchBegin 0", w * 0.1, h * 0.2),
                                    logged("touch TouchBegin 1", w * 0.8, h * 0.7),
                                    logged("touch TouchUpdate 1", w * 0.9, h * 0.7),
                                    logged("touch TouchEnd 1", w * 0.9, h * 0.7),
                                    logged("touch TouchEnd 0", w * 0.1, h * 0.2),
                                ],
                            ),
                            _ => {
                                finish();
                                return false;
                            }
                        }
                    }
                }
                // With the library only the multi-touch screen is read.
                step.set(if smoke.libinput && step.get() == 0 { 7 } else { step.get() + 1 });
                true
            },
            Duration::from_millis(50),
            DispatcherPriority::BACKGROUND,
        )
        .detach();
        TIMER.with(|slot| *slot.borrow_mut() = Some(timer));
    }

    fn argument(name: &str) -> Option<String> {
        let args: Vec<String> = std::env::args().collect();
        let prefix = format!("{name}=");
        args.iter()
            .position(|arg| arg == name)
            .and_then(|index| args.get(index + 1).cloned())
            .or_else(|| args.iter().find_map(|arg| arg.strip_prefix(&prefix).map(str::to_owned)))
    }

    fn flag(name: &str) -> bool {
        std::env::args().any(|arg| arg == name)
    }

    fn file_var_info() -> fb_var_screeninfo {
        let size = argument("--size").unwrap_or_else(|| "640x400".to_owned());
        let (width, height) =
            size.split_once('x').and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?))).unwrap_or((640, 400));
        let eight = |offset| fb_bitfield { offset, length: 8, msb_right: 0 };
        let mut var_info = fb_var_screeninfo { xres: width, yres: height, bits_per_pixel: 32, ..Default::default() };
        match argument("--format").as_deref() {
            Some("rgba") => {
                (var_info.red, var_info.green, var_info.blue, var_info.transp) =
                    (eight(0), eight(8), eight(16), eight(24))
            }
            Some("rgb565") => var_info.bits_per_pixel = 16,
            _ => {
                (var_info.red, var_info.green, var_info.blue, var_info.transp) =
                    (eight(16), eight(8), eight(0), eight(24))
            }
        }
        var_info
    }

    /// What a run is drawn on: the output, where its frame is read back
    /// from, and the result of the check of the output.
    type CreatedOutput = (Arc<dyn IOutputBackend>, FrameSource, SurfaceOrientation, (bool, String));

    /// A file or a framebuffer device, drawn in software.
    fn fb_output() -> Result<CreatedOutput, String> {
        let (node, device): (String, Box<dyn IFbDevice>) = if let Some(path) = argument("--file") {
            match FileFbDevice::create(&path, file_var_info()) {
                Ok(device) => (path, Box::new(device)),
                Err(error) => return Err(format!("the file {path} cannot be created: {error}")),
            }
        } else if let Some(node) = argument("--fbdev") {
            let Some(expected) = argument("--expect-id") else {
                return Err("--fbdev needs --expect-id, the beginning of the identification of the device".to_owned());
            };
            let device = match LinuxFbDevice::open(&node) {
                Ok(device) => device,
                Err(error) => return Err(format!("{node}: {error}")),
            };
            // Nothing is written to a device that is not the expected one.
            let id = device.get_fix_screen_info().map(|info| info.id_string()).unwrap_or_default();
            if !id.starts_with(&expected) {
                return Err(format!("{node} is \"{id}\", not a device whose identification begins with \"{expected}\"; nothing was written to it"));
            }
            (node, Box::new(device))
        } else {
            return Err(
                "one of --file <path>, --fbdev <node> --expect-id <prefix> and --drm <card> --expect-driver <name> is needed"
                    .to_owned(),
            );
        };

        let options = FbDevOutputOptions {
            render_directly_to_mapped_memory: flag("--direct"),
            use_async_front_buffer_blit: flag("--async").then_some(true),
            pixel_format: match argument("--request-format").as_deref() {
                Some("bgra") => Some(PixelFormat::BGRA8888),
                Some("rgba") => Some(PixelFormat::RGBA8888),
                _ => None,
            },
            ..Default::default()
        };
        // The device is asked again once the output has it: a requested
        // format changes what it reports.
        let output = match FbdevOutput::with_device(device, options) {
            Ok(output) => output,
            Err(error) => return Err(format!("{node}: {error}")),
        };
        let (var_info, fixed_info) = (output.var_screen_info(), output.fix_screen_info());
        let format = match (var_info.bits_per_pixel, var_info.blue.offset) {
            (16, _) => PixelFormat::RGB565,
            (32, 16) => PixelFormat::RGBA8888,
            _ => PixelFormat::BGRA8888,
        };
        let size = output.pixel_size();
        let check = (
            size.width as u32 == var_info.xres && size.height as u32 == var_info.yres && size.width > 0 && size.height > 0,
            format!(
                "{node} is \"{}\": {}x{}, {} bits per pixel, lines of {} bytes, drawn as {format:?}",
                output.id(),
                size.width,
                size.height,
                var_info.bits_per_pixel,
                fixed_info.line_length
            ),
        );
        let output: Arc<dyn IOutputBackend> = output;
        Ok((output, FrameSource::Fb { node, var_info, fixed_info }, SurfaceOrientation::Rotation0, check))
    }

    /// A KMS output of a card, drawn with OpenGL.
    fn drm_output(card: &str) -> Result<CreatedOutput, String> {
        let Some(expected) = argument("--expect-driver") else {
            return Err("--drm needs --expect-driver, the name of the driver of the card".to_owned());
        };
        let drm_card = DrmCard::new(Some(card)).map_err(|error| format!("{card}: {error}"))?;
        // No mode is set on a card that is not the expected one.
        let driver = drm_scanout::driver_name(drm_card.fd()).unwrap_or_default();
        if driver != expected {
            return Err(format!("{card} is driven by \"{driver}\", not by \"{expected}\"; nothing was written to it"));
        }
        match drm_card.get_resources(false) {
            Ok(resources) => resources.dump(),
            Err(error) => return Err(format!("{card}: {error}")),
        }
        let orientation = match argument("--orientation").as_deref() {
            Some("90") => SurfaceOrientation::Rotation90,
            Some("180") => SurfaceOrientation::Rotation180,
            Some("270") => SurfaceOrientation::Rotation270,
            _ => SurfaceOrientation::Rotation0,
        };
        let options = DrmOutputOptions { orientation, ..Default::default() };
        let output = DrmOutput::from_card(drm_card, false, Some(options)).map_err(|error| format!("{card}: {error}"))?;
        let size = output.pixel_size();
        let check = (
            size.width > 0 && size.height > 0,
            format!(
                "{card} is driven by \"{driver}\": CRTC {}, {}x{} as the top-level sees it ({orientation:?})",
                output.crtc_id(),
                size.width,
                size.height
            ),
        );
        let backend: Arc<dyn IOutputBackend> = output.clone();
        Ok((backend, FrameSource::Drm(output), orientation, check))
    }

    pub fn run() -> ExitCode {
        let fail = |message: String| {
            println!("  [FAILED] setup: {message}");
            println!("SMOKE FAILED");
            ExitCode::FAILURE
        };

        println!("Phase: output");
        let (output, source, orientation, output_check) = if let Some(card) = argument("--drm") {
            match drm_output(&card) {
                Ok(created) => created,
                Err(message) => return fail(message),
            }
        } else {
            match fb_output() {
                Ok(created) => created,
                Err(message) => return fail(message),
            }
        };
        let size = output.pixel_size();

        let log = Rc::new(RefCell::new(Vec::new()));
        let devices = if flag("--uinput") {
            match VirtualDevices::create() {
                Ok(devices) => Some(devices),
                Err(error) => {
                    return fail(format!("the virtual input devices cannot be created through /dev/uinput: {error}"))
                }
            }
        } else {
            None
        };
        let input_backend: Rc<dyn IInputBackend> = match &devices {
            Some(devices) => {
                for description in devices.descriptions() {
                    println!("  virtual input device {}", description.path().unwrap_or_default());
                }
                let inner: Rc<dyn IInputBackend> = if flag("--libinput") {
                    Rc::new(LibInputBackend::with_options(LibInputBackendOptions {
                        events: Some(vec![devices.multi_touch.event_node.clone()]),
                    }))
                } else {
                    Rc::new(EvDevBackend::new(devices.descriptions()))
                };
                Rc::new(RecordingBackend { inner, log: log.clone() })
            }
            None => Rc::new(NullInputBackend::new()),
        };

        let smoke = Rc::new(Smoke {
            source,
            orientation,
            width: size.width as u32,
            height: size.height as u32,
            dump: argument("--dump"),
            devices,
            libinput: flag("--libinput"),
            log,
            failed: Cell::new(0),
            passed: Cell::new(0),
        });
        smoke.check("output", output_check.0, output_check.1);
        SMOKE.with(|slot| *slot.borrow_mut() = Some(smoke));

        // The run must end by itself.
        std::thread::spawn(|| {
            std::thread::sleep(RUN_TIMEOUT);
            println!("  [FAILED] run: the smoke run did not finish within {} seconds", RUN_TIMEOUT.as_secs());
            println!("SMOKE FAILED");
            std::process::exit(1);
        });

        let args: Vec<String> = std::env::args().collect();
        let exit_code = AppBuilder::configure::<App>()
            .log_to_text_writer(std::io::stderr(), ferroui_base::logging::LogEventLevel::Warning, &[])
            .use_harfbuzz()
            .use_skia()
            .start_linux_direct(&args, output, Some(input_backend));
        println!("The application ended with exit code {exit_code}");

        // The virtual devices are removed with the state of the run.
        SMOKE.with(|slot| slot.borrow_mut().take());
        if exit_code == 0 && SMOKE_PASSED.load(Ordering::SeqCst) {
            println!("SMOKE PASSED");
            ExitCode::SUCCESS
        } else {
            println!("SMOKE FAILED");
            ExitCode::FAILURE
        }
    }
}
