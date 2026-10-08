//! Port of the platform file of the upstream headless project: the
//! headless platform, its options and the `use_headless` extension of the
//! application builder.

use crate::headless_platform_render_interface::HeadlessPlatformRenderInterface;
use crate::headless_platform_stubs::{HeadlessClipboardImplStub, HeadlessCursorFactoryStub, HeadlessIconLoaderStub};
use crate::headless_render_timer::HeadlessRenderTimer;
use crate::headless_window_impl::HeadlessWindowImpl;
use ferroui_base::input::platform::{
    Clipboard, IClipboard, IClipboardImpl, KeyGestureFormatInfo, PlatformHotkeyConfiguration,
};
use ferroui_base::input::{IKeyboardDevice, KeyboardDevice, MouseDevice};
use ferroui_base::platform::{DefaultPlatformSettings, ICursorFactory, IPlatformSettings, PixelFormat, PixelFormats};
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::rendering::{IRenderLoop, IRenderTimer, RenderLoop, SleepLoopRenderTimer};
use ferroui_base::threading::{Dispatcher, DispatcherFrame};
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_controls::platform::{IPlatformIconLoader, ITopLevelImpl, ITrayIconImpl, IWindowImpl, IWindowingPlatform};
use ferroui_controls::AppBuilder;
use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

thread_local! {
    // The two static fields of the original. They are per thread: the compositor is an `Rc`,
    // and a headless application lives on the one thread that initialised the platform.
    static COMPOSITOR: RefCell<Option<Rc<Compositor>>> = const { RefCell::new(None) };
    /// The render timer when it is the headless one (the UI thread timer
    /// that can be ticked by force); `None` for the sleep loop timer.
    static HEADLESS_RENDER_TIMER: RefCell<Option<Arc<HeadlessRenderTimer>>> = const { RefCell::new(None) };
}

/// The headless platform.
pub struct FerroHeadlessPlatform;

struct HeadlessWindowingPlatform {
    options: FerroHeadlessPlatformOptions,
}

impl IWindowingPlatform for HeadlessWindowingPlatform {
    fn create_window(&self) -> Rc<dyn IWindowImpl> {
        HeadlessWindowImpl::new(&self.options)
    }

    fn create_embeddable_top_level(&self) -> Rc<dyn ITopLevelImpl> {
        let window: Rc<dyn ITopLevelImpl> = self.create_embeddable_window();
        window
    }

    /// # Panics
    /// Always: the headless platform has no embeddable windows
    /// (`PlatformNotSupportedException`).
    fn create_embeddable_window(&self) -> Rc<dyn IWindowImpl> {
        panic!("Operation is not supported on this platform.");
    }

    fn create_tray_icon(&self) -> Option<Rc<dyn ITrayIconImpl>> {
        None
    }

    fn get_windows_z_order(&self, windows: &[Rc<dyn IWindowImpl>], z_order: &mut [i64]) {
        for (i, window) in windows.iter().enumerate() {
            z_order[i] = window
                .as_any()
                .downcast_ref::<HeadlessWindowImpl>()
                .map_or(0, |window| i64::from(window.z_order()));
        }
    }
}

impl FerroHeadlessPlatform {
    /// The compositor of the platform; `None` before the platform is
    /// initialised.
    pub(crate) fn compositor() -> Option<Rc<Compositor>> {
        COMPOSITOR.with(|compositor| compositor.borrow().clone())
    }

    pub(crate) fn initialize(opts: &FerroHeadlessPlatformOptions) {
        if opts.use_shared_mouse_device == Some(true) {
            MouseDevice::reset_primary_for_unit_tests();
        }

        let clipboard_impl: Rc<dyn IClipboardImpl> = Rc::new(HeadlessClipboardImplStub::default());
        let clipboard: Rc<dyn IClipboard> = Clipboard::new(clipboard_impl.clone());

        let (render_timer, headless_render_timer): (Arc<dyn IRenderTimer>, Option<Arc<HeadlessRenderTimer>>) =
            if opts.should_render_on_ui_thread {
                let timer = Arc::new(HeadlessRenderTimer::new(opts.fps));
                (timer.clone(), Some(timer))
            } else {
                (Arc::new(SleepLoopRenderTimer::new(opts.fps)), None)
            };
        HEADLESS_RENDER_TIMER.with(|slot| *slot.borrow_mut() = headless_render_timer);

        let keyboard_device: Rc<dyn IKeyboardDevice> = KeyboardDevice::new();
        let render_loop: Arc<dyn IRenderLoop> = RenderLoop::from_timer(render_timer);
        let windowing_platform: Rc<dyn IWindowingPlatform> =
            Rc::new(HeadlessWindowingPlatform { options: opts.clone() });

        FerroLocator::current_mutable()
            .bind::<dyn IClipboardImpl>()
            .to_constant(clipboard_impl)
            .bind::<dyn IClipboard>()
            .to_constant(clipboard)
            .bind::<dyn ICursorFactory>()
            .to_singleton::<HeadlessCursorFactoryStub>(|instance| instance)
            .bind::<dyn IPlatformSettings>()
            .to_singleton::<DefaultPlatformSettings>(|instance| instance)
            .bind::<dyn IPlatformIconLoader>()
            .to_singleton::<HeadlessIconLoaderStub>(|instance| instance)
            .bind::<dyn IKeyboardDevice>()
            .to_constant(keyboard_device)
            .bind::<Arc<dyn IRenderLoop>>()
            .to_constant(Rc::new(render_loop))
            .bind::<dyn IWindowingPlatform>()
            .to_constant(windowing_platform)
            .bind_to_self_singleton::<PlatformHotkeyConfiguration>()
            .bind_to_self(Rc::new(KeyGestureFormatInfo::new(Some(HashMap::new()), "Cmd", "Ctrl", "Alt", "Shift")));

        let compositor = Compositor::new(None, false);
        COMPOSITOR.with(|slot| *slot.borrow_mut() = Some(compositor));
    }

    /// Forces renderer to process a rendering timer tick.
    /// Use this method before calling
    /// [`get_last_rendered_frame`](crate::HeadlessWindowExtensions::get_last_rendered_frame).
    ///
    /// `count` is the count of frames to be ticked on the timer (1 in the
    /// original when it is not given).
    ///
    /// # Panics
    /// Panics if the render timer is not the headless one and the
    /// compositor is not initialized (`InvalidOperationException`).
    pub fn force_render_timer_tick(count: i32) {
        let timer = HEADLESS_RENDER_TIMER.with(|timer| timer.borrow().clone());
        if let Some(timer) = timer {
            for _ in 0..count {
                timer.force_tick();
            }
        } else {
            let Some(compositor) = Self::compositor() else {
                panic!("Compositor is not initialized.");
            };

            let frame = DispatcherFrame::new();
            // `RequestCommitAsync` of the original is the processed signal of the batch.
            let batch = compositor.request_commit_async();
            let continuation_frame = frame.clone();
            batch.processed().on_completed(move || continuation_frame.set_continue(false));
            Dispatcher::current_dispatcher().push_frame(&frame);
        }
    }
}

/// Options for configuring the headless platform.
#[derive(Clone, Debug, PartialEq)]
pub struct FerroHeadlessPlatformOptions {
    /// Gets or sets the number of frames per second at which the renderer should run.
    /// Default 60.
    pub fps: i32,

    /// Render directly on the UI thread instead of using a dedicated render thread.
    /// This can be usable if your device doesn't have multiple cores to begin with.
    /// This setting is true by default (the documentation of the original says false).
    pub should_render_on_ui_thread: bool,

    /// Gets or sets a value indicating whether to use headless drawing mode, which allows
    /// rendering without creating an actual window.
    ///
    /// Disable this option if you are using the Skia backend or another drawing backend.
    pub use_headless_drawing: bool,

    /// Gets or sets the pixel format to be used for the headless window framebuffers.
    pub frame_buffer_format: PixelFormat,

    /// Embeds popups to the window when set to true. The default value is true.
    pub overlay_popups: bool,

    /// Shares a single mouse device between all top-levels when set to true, so that pointer
    /// capture and click counting are global rather than per-window, as they are on the other
    /// platforms. When `None` or false, every top-level gets its own mouse device.
    pub use_shared_mouse_device: Option<bool>,
}

impl Default for FerroHeadlessPlatformOptions {
    fn default() -> Self {
        Self {
            fps: 60,
            should_render_on_ui_thread: true,
            use_headless_drawing: true,
            frame_buffer_format: PixelFormats::RGBA8888,
            overlay_popups: true,
            use_shared_mouse_device: None,
        }
    }
}

/// The `use_headless` extension of the application builder.
pub trait FerroHeadlessPlatformExtensions {
    /// Uses the headless platform: its windowing subsystem, the standard
    /// runtime platform, the HarfBuzz text shaper and, with
    /// [`use_headless_drawing`](FerroHeadlessPlatformOptions::use_headless_drawing),
    /// its rendering subsystem.
    fn use_headless(&self, opts: FerroHeadlessPlatformOptions) -> AppBuilder;
}

impl FerroHeadlessPlatformExtensions for AppBuilder {
    fn use_headless(&self, opts: FerroHeadlessPlatformOptions) -> AppBuilder {
        if opts.use_headless_drawing {
            // The builder is a handle: the call configures the builder it is made on.
            drop(self.use_rendering_subsystem(HeadlessPlatformRenderInterface::initialize, "Headless"));
        }
        self.use_standard_runtime_platform_subsystem()
            .use_windowing_subsystem(move || FerroHeadlessPlatform::initialize(&opts), "Headless")
            .use_harfbuzz()
    }
}
