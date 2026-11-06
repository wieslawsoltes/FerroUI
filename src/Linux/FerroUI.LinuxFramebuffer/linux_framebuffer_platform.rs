//! The platform, its lifetime and the functions that start an application
//! on it (the port of `LinuxFramebufferPlatform.cs`).

use crate::epoll_dispatcher_impl::EpollDispatcherImpl;
use crate::framebuffer_toplevel_impl::FramebufferToplevelImpl;
use crate::input::ev_dev::EvDevBackend;
use crate::input::IInputBackend;
use crate::linux_framebuffer_platform_options::LinuxFramebufferPlatformOptions;
use crate::output::{DrmOutput, FbDevOutputOptions, FbdevOutput, IOutputBackend};
use crate::DrmOutputOptions;
use crate::raw_event_grouping::{
    IRawEventGrouperDispatchQueue, ManualRawEventGrouperDispatchQueue,
    ManualRawEventGrouperDispatchQueueDispatcherInputProvider,
};
use crate::stubs::CursorFactoryStub;
use ferroui_base::input::platform::{KeyGestureFormatInfo, PlatformHotkeyConfiguration};
use ferroui_base::input::{IKeyboardDevice, InputElement, KeyModifiers, KeyboardDevice};
use ferroui_base::platform::{
    DefaultPlatformSettings, ICursorFactory, IPlatformGraphics, IPlatformSettings, PixelFormat, SharedBitmapImpl,
};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::rendering::{IRenderLoop, IRenderTimer, RenderLoop, SleepLoopRenderTimer, UiThreadRenderTimer};
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{FerroLocator, LocatorExtensions, Ref};
use ferroui_controls::application_lifetimes::{
    ControlledApplicationLifetimeExitEventArgs, ControlledApplicationLifetimeStartupEventArgs, IApplicationLifetime,
    IControlledApplicationLifetime, ISingleTopLevelApplicationLifetime, ISingleViewApplicationLifetime,
};
use ferroui_controls::embedding::EmbeddableControlRoot;
use ferroui_controls::platform::{IPlatformIconLoader, ITopLevelImpl, IWindowIconImpl};
use ferroui_controls::{AppBuilder, Control, TopLevel};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io;
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// The icon loader of a platform without window icons.
#[derive(Default)]
pub(crate) struct LinuxFramebufferIconLoaderStub;

struct IconStub;

impl IWindowIconImpl for IconStub {
    fn save(&self, _output_stream: &mut dyn io::Write) -> io::Result<()> {
        Ok(())
    }
}

impl IPlatformIconLoader for LinuxFramebufferIconLoaderStub {
    fn load_icon_from_file(&self, _file_name: &str) -> io::Result<Rc<dyn IWindowIconImpl>> {
        Ok(Rc::new(IconStub))
    }

    fn load_icon_from_stream(&self, _stream: &mut dyn io::Read) -> io::Result<Rc<dyn IWindowIconImpl>> {
        Ok(Rc::new(IconStub))
    }

    fn load_icon_from_bitmap(&self, _bitmap: Arc<SharedBitmapImpl>) -> Rc<dyn IWindowIconImpl> {
        Rc::new(IconStub)
    }
}

ferroui_base::ferro_thread_local! {
    /// `EventGrouperDispatchQueue`: the raw input of the top-level, which
    /// the dispatcher implementation hands out one event at a time.
    static EVENT_GROUPER_DISPATCH_QUEUE: Rc<ManualRawEventGrouperDispatchQueue> =
        Rc::new(ManualRawEventGrouperDispatchQueue::new());
    /// `Compositor`.
    static COMPOSITOR: RefCell<Option<Rc<Compositor>>> = const { RefCell::new(None) };
    /// The keyboard device that is registered with the platform.
    static KEYBOARD_DEVICE: RefCell<Option<Rc<KeyboardDevice>>> = const { RefCell::new(None) };
}

pub struct LinuxFramebufferPlatform {
    fb: Arc<dyn IOutputBackend>,
}

impl LinuxFramebufferPlatform {
    fn new(backend: Arc<dyn IOutputBackend>) -> Self {
        Self { fb: backend }
    }

    pub fn event_grouper_dispatch_queue() -> Rc<ManualRawEventGrouperDispatchQueue> {
        EVENT_GROUPER_DISPATCH_QUEUE.with(Rc::clone)
    }

    /// The compositor of the platform, once the platform is initialised.
    pub(crate) fn compositor() -> Option<Rc<Compositor>> {
        COMPOSITOR.with(|compositor| compositor.borrow().clone())
    }

    /// The keyboard device of the platform, once the platform is
    /// initialised.
    pub(crate) fn keyboard_device() -> Option<Rc<KeyboardDevice>> {
        KEYBOARD_DEVICE.with(|keyboard| keyboard.borrow().clone())
    }

    fn initialize_platform(&self) {
        let locator = FerroLocator::current_mutable();
        let graphics = self.fb.as_gl_output_backend().map(|gl| gl.platform_graphics());
        if let Some(graphics) = &graphics {
            locator.bind::<Arc<dyn IPlatformGraphics>>().to_constant(Rc::new(graphics.clone()));
        }

        let opts = FerroLocator::current()
            .get_service::<LinuxFramebufferPlatformOptions>()
            .map_or_else(LinuxFramebufferPlatformOptions::default, |opts| *opts);

        let timer: Arc<dyn IRenderTimer> = if opts.should_render_on_ui_thread {
            Arc::new(UiThreadRenderTimer::new(opts.fps))
        } else {
            // The reference uses its default render timer here, which
            // ticks on threads of a pool. The compositor of the port is
            // confined to one render thread, and the default timer starts
            // a thread each time it is started; the timer that keeps one
            // thread is used (DEVIATIONS.md).
            Arc::new(SleepLoopRenderTimer::new(opts.fps))
        };

        Dispatcher::initialize_ui_thread_dispatcher(EpollDispatcherImpl::new(Box::new(
            ManualRawEventGrouperDispatchQueueDispatcherInputProvider::new(Self::event_grouper_dispatch_queue()),
        )));

        let render_loop: Arc<dyn IRenderLoop> = RenderLoop::from_timer(timer);
        let keyboard_device = KeyboardDevice::new();
        KEYBOARD_DEVICE.with(|keyboard| *keyboard.borrow_mut() = Some(keyboard_device.clone()));
        let keyboard_device: Rc<dyn IKeyboardDevice> = keyboard_device;
        let platform_settings: Rc<dyn IPlatformSettings> = Rc::new(DefaultPlatformSettings::new());
        locator
            .bind::<Arc<dyn IRenderLoop>>()
            .to_constant(Rc::new(render_loop))
            .bind::<dyn ICursorFactory>()
            .to_transient::<CursorFactoryStub>(|stub| stub)
            .bind::<dyn IKeyboardDevice>()
            .to_constant(keyboard_device)
            .bind::<dyn IPlatformIconLoader>()
            .to_singleton::<LinuxFramebufferIconLoaderStub>(|stub| stub)
            .bind::<dyn IPlatformSettings>()
            .to_constant(platform_settings)
            .bind_to_self(Rc::new(PlatformHotkeyConfiguration::new(KeyModifiers::CONTROL)))
            .bind_to_self(Rc::new(KeyGestureFormatInfo::new(Some(HashMap::new()), "Super", "Ctrl", "Alt", "Shift")));

        let compositor = Compositor::new(graphics, false);
        COMPOSITOR.with(|slot| *slot.borrow_mut() = Some(compositor));
    }

    /// `Initialize`: registers the platform with the builder and creates
    /// the lifetime of its one top-level.
    ///
    /// The reference also selects its Skia backend here; the renderer is
    /// the caller's choice on the builder in this port.
    pub(crate) fn initialize(
        builder: &AppBuilder,
        output_backend: Arc<dyn IOutputBackend>,
        input_backend: Option<Rc<dyn IInputBackend>>,
    ) -> Rc<LinuxFramebufferLifetime> {
        let platform = LinuxFramebufferPlatform::new(output_backend);
        let fb = platform.fb.clone();
        builder
            .use_standard_runtime_platform_subsystem()
            .use_windowing_subsystem(move || platform.initialize_platform(), "fbdev");
        LinuxFramebufferLifetime::with_input(fb, input_backend)
    }
}

/// The lifetime of an application on the platform: one top-level that
/// shows the main view.
pub struct LinuxFramebufferLifetime {
    this: Weak<LinuxFramebufferLifetime>,
    fb: Arc<dyn IOutputBackend>,
    input_backend: Option<Rc<dyn IInputBackend>>,
    top_level: RefCell<Option<Ref<EmbeddableControlRoot>>>,
    main_view: RefCell<Option<Ref<Control>>>,
    cts: CancellationToken,
    exit_code: Cell<i32>,
    startup: HandlerList<dyn Fn(&ControlledApplicationLifetimeStartupEventArgs)>,
    exit: HandlerList<dyn Fn(&ControlledApplicationLifetimeExitEventArgs)>,
}

impl LinuxFramebufferLifetime {
    pub fn new(fb: Arc<dyn IOutputBackend>) -> Rc<Self> {
        Self::with_input(fb, None)
    }

    pub fn with_input(fb: Arc<dyn IOutputBackend>, input: Option<Rc<dyn IInputBackend>>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            fb,
            input_backend: input,
            top_level: RefCell::new(None),
            main_view: RefCell::new(None),
            cts: CancellationToken::new(),
            exit_code: Cell::new(0),
            startup: HandlerList::new(),
            exit: HandlerList::new(),
        })
    }

    /// The token that is cancelled when the application shuts down.
    pub fn token(&self) -> CancellationToken {
        self.cts.clone()
    }

    pub fn exit_code(&self) -> i32 {
        self.exit_code.get()
    }

    pub fn start(&self, args: &[String]) {
        let e = ControlledApplicationLifetimeStartupEventArgs::new(args.iter().cloned());
        for (_, handler) in self.startup.snapshot().iter() {
            handler(&e);
        }
    }

    fn subscribe<F: ?Sized + 'static>(
        &self,
        select: fn(&Self) -> &HandlerList<F>,
        handler: Rc<F>,
    ) -> Rc<dyn IDisposable> {
        let token = select(self).add(handler);
        let weak = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                select(&this).remove(token);
            }
        })
    }

    /// `EnsureTopLevel`.
    ///
    /// # Panics
    /// Panics when no input backend was given and the one of the
    /// environment cannot be created.
    fn ensure_top_level(&self) -> Ref<EmbeddableControlRoot> {
        if let Some(top_level) = self.top_level.borrow().clone() {
            return top_level;
        }

        let input_backend: Rc<dyn IInputBackend> = match &self.input_backend {
            Some(input_backend) => input_backend.clone(),
            None => {
                if std::env::var("FERROUI_USE_EVDEV").is_ok_and(|value| value == "1") {
                    Rc::new(EvDevBackend::create_from_environment().unwrap_or_else(|error| panic!("{error}")))
                } else {
                    Rc::new(crate::input::lib_input::LibInputBackend::new())
                }
            }
        };

        let queue: Rc<dyn IRawEventGrouperDispatchQueue> = LinuxFramebufferPlatform::event_grouper_dispatch_queue();
        let platform_impl: Rc<dyn ITopLevelImpl> = FramebufferToplevelImpl::new(
            self.fb.clone(),
            input_backend,
            Some(queue),
            LinuxFramebufferPlatform::compositor(),
        );
        let tl = EmbeddableControlRoot::with_impl(platform_impl);
        tl.prepare();
        tl.start_rendering();
        *self.top_level.borrow_mut() = Some(tl.clone());

        let scope: Ref<InputElement> = tl.clone().upcast();
        tl.focus_manager().set_focus_scope(&scope);
        tl
    }
}

impl IApplicationLifetime for LinuxFramebufferLifetime {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_controlled_application_lifetime(&self) -> Option<&dyn IControlledApplicationLifetime> {
        Some(self)
    }

    fn as_single_view_application_lifetime(&self) -> Option<&dyn ISingleViewApplicationLifetime> {
        Some(self)
    }

    fn as_single_top_level_application_lifetime(&self) -> Option<&dyn ISingleTopLevelApplicationLifetime> {
        Some(self)
    }
}

impl IControlledApplicationLifetime for LinuxFramebufferLifetime {
    fn startup(&self, handler: Rc<dyn Fn(&ControlledApplicationLifetimeStartupEventArgs)>) -> Rc<dyn IDisposable> {
        self.subscribe(|this| &this.startup, handler)
    }

    fn exit(&self, handler: Rc<dyn Fn(&ControlledApplicationLifetimeExitEventArgs)>) -> Rc<dyn IDisposable> {
        self.subscribe(|this| &this.exit, handler)
    }

    fn shutdown(&self, exit_code: i32) {
        self.exit_code.set(exit_code);
        let e = ControlledApplicationLifetimeExitEventArgs::new(exit_code);
        for (_, handler) in self.exit.snapshot().iter() {
            handler(&e);
        }
        self.exit_code.set(e.application_exit_code());
        self.cts.cancel();
    }
}

impl ISingleViewApplicationLifetime for LinuxFramebufferLifetime {
    fn main_view(&self) -> Option<Ref<Control>> {
        self.main_view.borrow().clone()
    }

    fn set_main_view(&self, value: Option<Ref<Control>>) {
        let top_level = self.ensure_top_level();
        *self.main_view.borrow_mut() = value.clone();
        top_level.set_content(value.map(Control::boxed));
    }
}

impl ISingleTopLevelApplicationLifetime for LinuxFramebufferLifetime {
    fn top_level(&self) -> Option<Ref<TopLevel>> {
        Some(self.ensure_top_level().upcast())
    }
}

/// The functions that start an application on the platform.
pub trait LinuxFramebufferPlatformExtensions {
    /// Starts the application on a framebuffer device (`/dev/fb0`, or the
    /// one the environment variable `FRAMEBUFFER` names, when `fbdev` is
    /// not given) and runs it until it shuts down. Returns the exit code.
    fn start_linux_fb_dev(
        &self,
        args: &[String],
        fbdev: Option<&str>,
        scaling: f64,
        input_backend: Option<Rc<dyn IInputBackend>>,
    ) -> io::Result<i32>;

    /// As [`start_linux_fb_dev`](Self::start_linux_fb_dev), asking the
    /// device for a pixel format.
    fn start_linux_fb_dev_with_format(
        &self,
        args: &[String],
        fbdev: &str,
        format: Option<PixelFormat>,
        scaling: f64,
        input_backend: Option<Rc<dyn IInputBackend>>,
    ) -> io::Result<i32>;

    /// As [`start_linux_fb_dev`](Self::start_linux_fb_dev), with the
    /// options of the output.
    fn start_linux_fb_dev_with_options(
        &self,
        args: &[String],
        options: FbDevOutputOptions,
        input_backend: Option<Rc<dyn IInputBackend>>,
    ) -> io::Result<i32>;

    /// Starts the application on a KMS output: the first connected
    /// connector of the card `card` (or of the first usable card of
    /// `/dev/dri`) in its preferred mode, and runs it until it shuts down.
    /// Returns the exit code.
    fn start_linux_drm(
        &self,
        args: &[String],
        card: Option<&str>,
        scaling: f64,
        input_backend: Option<Rc<dyn IInputBackend>>,
    ) -> io::Result<i32>;

    /// As [`start_linux_drm`](Self::start_linux_drm), probing the
    /// connectors when asked to and with the options of the output.
    fn start_linux_drm_with_options(
        &self,
        args: &[String],
        card: Option<&str>,
        connectors_force_probe: bool,
        options: Option<DrmOutputOptions>,
        input_backend: Option<Rc<dyn IInputBackend>>,
    ) -> io::Result<i32>;

    /// Starts the application on an output that is already created and
    /// runs it until it shuts down. Returns the exit code.
    fn start_linux_direct(
        &self,
        args: &[String],
        output_backend: Arc<dyn IOutputBackend>,
        input_backend: Option<Rc<dyn IInputBackend>>,
    ) -> i32;

    /// Registers the platform with the builder and returns the lifetime
    /// of its top-level, without setting the application up or running
    /// it. An addition: the reference keeps this step internal
    /// (`LinuxFramebufferPlatform.Initialize`).
    fn use_linux_framebuffer(
        &self,
        output_backend: Arc<dyn IOutputBackend>,
        input_backend: Option<Rc<dyn IInputBackend>>,
    ) -> Rc<LinuxFramebufferLifetime>;
}

impl LinuxFramebufferPlatformExtensions for AppBuilder {
    fn start_linux_fb_dev(
        &self,
        args: &[String],
        fbdev: Option<&str>,
        scaling: f64,
        input_backend: Option<Rc<dyn IInputBackend>>,
    ) -> io::Result<i32> {
        let output = FbdevOutput::with_format(fbdev, None)?;
        output.set_scaling(scaling);
        Ok(self.start_linux_direct(args, output, input_backend))
    }

    fn start_linux_fb_dev_with_format(
        &self,
        args: &[String],
        fbdev: &str,
        format: Option<PixelFormat>,
        scaling: f64,
        input_backend: Option<Rc<dyn IInputBackend>>,
    ) -> io::Result<i32> {
        let output = FbdevOutput::with_format(Some(fbdev), format)?;
        output.set_scaling(scaling);
        Ok(self.start_linux_direct(args, output, input_backend))
    }

    fn start_linux_fb_dev_with_options(
        &self,
        args: &[String],
        options: FbDevOutputOptions,
        input_backend: Option<Rc<dyn IInputBackend>>,
    ) -> io::Result<i32> {
        let output = FbdevOutput::with_options(options)?;
        Ok(self.start_linux_direct(args, output, input_backend))
    }

    fn start_linux_drm(
        &self,
        args: &[String],
        card: Option<&str>,
        scaling: f64,
        input_backend: Option<Rc<dyn IInputBackend>>,
    ) -> io::Result<i32> {
        let output = DrmOutput::new(card, false, None)?;
        output.set_scaling(scaling);
        Ok(self.start_linux_direct(args, output, input_backend))
    }

    fn start_linux_drm_with_options(
        &self,
        args: &[String],
        card: Option<&str>,
        connectors_force_probe: bool,
        options: Option<DrmOutputOptions>,
        input_backend: Option<Rc<dyn IInputBackend>>,
    ) -> io::Result<i32> {
        let output = DrmOutput::new(card, connectors_force_probe, options)?;
        Ok(self.start_linux_direct(args, output, input_backend))
    }

    fn start_linux_direct(
        &self,
        args: &[String],
        output_backend: Arc<dyn IOutputBackend>,
        input_backend: Option<Rc<dyn IInputBackend>>,
    ) -> i32 {
        let lifetime = LinuxFramebufferPlatform::initialize(self, output_backend, input_backend);
        self.setup_with_lifetime(lifetime.clone());
        lifetime.start(args);
        let instance = self.instance().unwrap_or_else(|| panic!("The application was not created"));
        instance.run_with_token(&lifetime.token());
        lifetime.exit_code()
    }

    fn use_linux_framebuffer(
        &self,
        output_backend: Arc<dyn IOutputBackend>,
        input_backend: Option<Rc<dyn IInputBackend>>,
    ) -> Rc<LinuxFramebufferLifetime> {
        LinuxFramebufferPlatform::initialize(self, output_backend, input_backend)
    }
}
