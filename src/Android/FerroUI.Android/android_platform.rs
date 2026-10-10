//! The Android platform: its options, the selection of the platform with
//! the application builder, and the initialization of its services.

/// Represents the rendering mode for platform graphics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum AndroidRenderingMode {
    /// The framework is rendered into a framebuffer.
    Software = 1,

    /// Enables android EGL rendering.
    Egl = 2,

    /// Enables Vulkan rendering
    Vulkan = 3,
}

/// The options of the Android platform, registered with the application
/// builder (`AppBuilder::with`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AndroidPlatformOptions {
    /// Gets or sets the rendering modes with fallbacks.
    /// The first element in the array has the highest priority.
    /// The default value is: [`AndroidRenderingMode::Egl`], [`AndroidRenderingMode::Software`].
    ///
    /// If the application should work on as wide a range of devices as
    /// possible, at least add [`AndroidRenderingMode::Software`] as a
    /// fallback value. The platform fails to initialize when no value was
    /// matched.
    pub rendering_mode: Vec<AndroidRenderingMode>,
}

impl Default for AndroidPlatformOptions {
    fn default() -> Self {
        Self { rendering_mode: vec![AndroidRenderingMode::Egl, AndroidRenderingMode::Software] }
    }
}

/// The loop over the rendering modes (`InitializeGraphics`): the graphics
/// of the first mode that initializes, `None` for the software mode.
///
/// `try_create_egl` creates the platform graphics of EGL. The Vulkan mode
/// waits for the Vulkan project of the port
/// (docs/porting/android-platform.md, section 6) and is passed over like a
/// mode that fails to initialize.
///
/// # Panics
/// Panics, as the reference throws, when the list is empty and when none
/// of its modes could be applied.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) fn initialize_graphics_core<G>(
    opts: &AndroidPlatformOptions,
    try_create_egl: &mut dyn FnMut() -> Option<G>,
) -> Option<G> {
    if opts.rendering_mode.is_empty() {
        panic!("AndroidPlatformOptions.rendering_mode must not be empty or null");
    }

    for rendering_mode in &opts.rendering_mode {
        match rendering_mode {
            AndroidRenderingMode::Software => return None,
            AndroidRenderingMode::Egl => {
                if let Some(egl) = try_create_egl() {
                    return Some(egl);
                }
            }
            AndroidRenderingMode::Vulkan => {}
        }
    }

    panic!(
        "AndroidPlatformOptions.rendering_mode has a value of \"{}\", but no options were applied.",
        opts.rendering_mode.iter().map(|mode| format!("{mode:?}")).collect::<Vec<_>>().join(", ")
    );
}

#[cfg(target_os = "android")]
pub use imp::{AndroidApplicationExtensions, AndroidPlatform};

#[cfg(target_os = "android")]
mod imp {
    use super::{initialize_graphics_core, AndroidPlatformOptions};
    use crate::android_dispatcher_impl::AndroidDispatcherImpl;
    use crate::android_egl::AndroidEglPlatformGraphics;
    use crate::android_runtime_platform::AndroidRuntimePlatformServices;
    use crate::choreographer_timer::ChoreographerTimer;
    use crate::cursor_factory::CursorFactory;
    use crate::platform::AndroidActivatableLifetime;
    use crate::stubs::{PlatformIconLoaderStub, WindowingPlatformStub};
    use ferroui_base::input::platform::{KeyGestureFormatInfo, PlatformHotkeyConfiguration};
    use ferroui_base::input::{IKeyboardDevice, KeyboardDevice};
    use ferroui_base::platform::{DefaultPlatformSettings, ICursorFactory, IPlatformGraphics, IPlatformSettings};
    use ferroui_base::rendering::composition::Compositor;
    use ferroui_base::rendering::{IRenderLoop, IRenderTimer, RenderLoop};
    use ferroui_base::threading::Dispatcher;
    use ferroui_base::{FerroLocator, LocatorExtensions};
    use ferroui_controls::application_lifetimes::IActivatableLifetime;
    use ferroui_controls::platform::{IPlatformIconLoader, IWindowingPlatform};
    use ferroui_controls::AppBuilder;
    use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
    use ferroui_skia::SkiaApplicationExtensions;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;
    use std::sync::Arc;

    /// Selects the Android platform for an application.
    pub trait AndroidApplicationExtensions {
        /// Uses the Android platform, with the [`AndroidPlatformOptions`]
        /// registered with the builder or the default ones, the HarfBuzz
        /// text shaper and the Skia renderer.
        fn use_android(&self) -> AppBuilder;
    }

    impl AndroidApplicationExtensions for AppBuilder {
        fn use_android(&self) -> AppBuilder {
            AndroidRuntimePlatformServices::use_android_runtime_platform_subsystem(self)
                .use_windowing_subsystem(AndroidPlatform::initialize, "Android")
                .use_harfbuzz()
                .use_skia()
        }
    }

    thread_local! {
        static OPTIONS: RefCell<Option<Rc<AndroidPlatformOptions>>> = const { RefCell::new(None) };
        static COMPOSITOR: RefCell<Option<Rc<Compositor>>> = const { RefCell::new(None) };
        static TIMER: RefCell<Option<Arc<ChoreographerTimer>>> = const { RefCell::new(None) };
    }

    /// The Android platform.
    pub struct AndroidPlatform;

    impl AndroidPlatform {
        /// The options the platform was initialized with.
        pub fn options() -> Option<Rc<AndroidPlatformOptions>> {
            OPTIONS.with(|options| options.borrow().clone())
        }

        pub(crate) fn compositor() -> Option<Rc<Compositor>> {
            COMPOSITOR.with(|compositor| compositor.borrow().clone())
        }

        #[allow(dead_code)]
        pub(crate) fn timer() -> Option<Arc<ChoreographerTimer>> {
            TIMER.with(|timer| timer.borrow().clone())
        }

        /// Registers the services of the platform with the current service
        /// locator and installs the dispatcher of the main looper.
        ///
        /// # Panics
        /// Panics on another thread than the main thread of the
        /// application, and when the rendering modes of the options are
        /// empty or none of them applies.
        pub fn initialize() {
            let options = FerroLocator::current().get_service::<AndroidPlatformOptions>().unwrap_or_default();
            OPTIONS.with(|slot| *slot.borrow_mut() = Some(options.clone()));

            Dispatcher::initialize_ui_thread_dispatcher(AndroidDispatcherImpl::new());
            let timer = Arc::new(ChoreographerTimer::new());
            TIMER.with(|slot| *slot.borrow_mut() = Some(timer.clone()));

            let windowing_platform: Rc<dyn IWindowingPlatform> = Rc::new(WindowingPlatformStub);
            // Stage 2 of docs/porting/android-platform.md: the keyboard device of the
            // platform (`AndroidKeyboardDevice`) and its settings (`AndroidPlatformSettings`)
            // are not built; the keyboard device and the default settings of the framework
            // answer until then.
            let keyboard_device: Rc<dyn IKeyboardDevice> = KeyboardDevice::new();
            let platform_settings: Rc<dyn IPlatformSettings> = Rc::new(DefaultPlatformSettings::new());
            let render_timer: Arc<dyn IRenderTimer> = timer;
            let render_loop: Arc<dyn IRenderLoop> = RenderLoop::from_timer(render_timer);
            let activatable_lifetime = AndroidActivatableLifetime::new();
            let activatable: Rc<dyn IActivatableLifetime> = activatable_lifetime.clone();

            let locator = FerroLocator::current_mutable();
            locator
                .bind::<dyn ICursorFactory>()
                .to_transient::<CursorFactory>(|instance| instance)
                .bind::<dyn IWindowingPlatform>()
                .to_constant(windowing_platform)
                .bind::<dyn IKeyboardDevice>()
                .to_constant(keyboard_device)
                .bind::<dyn IPlatformSettings>()
                .to_constant(platform_settings)
                .bind::<dyn IPlatformIconLoader>()
                .to_singleton::<PlatformIconLoaderStub>(|instance| instance)
                .bind::<Arc<dyn IRenderLoop>>()
                .to_constant(Rc::new(render_loop))
                .bind_to_self_singleton::<PlatformHotkeyConfiguration>()
                .bind_to_self(Rc::new(KeyGestureFormatInfo::new(Some(HashMap::new()), "Cmd", "Ctrl", "Alt", "Shift")))
                .bind::<dyn IActivatableLifetime>()
                .to_constant(activatable)
                // With its concrete type, for the activities of this backend.
                .bind_to_self(activatable_lifetime);

            let graphics = Self::initialize_graphics(&options);
            if let Some(graphics) = &graphics {
                locator.bind::<Arc<dyn IPlatformGraphics>>().to_constant(Rc::new(graphics.clone()));
            }

            let compositor = Compositor::new(graphics, false);
            COMPOSITOR.with(|slot| *slot.borrow_mut() = Some(compositor.clone()));
            locator.bind_to_self(compositor);
        }

        fn initialize_graphics(opts: &AndroidPlatformOptions) -> Option<Arc<dyn IPlatformGraphics>> {
            initialize_graphics_core(opts, &mut AndroidEglPlatformGraphics::try_create)
                .map(|egl| Arc::new(egl) as Arc<dyn IPlatformGraphics>)
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of the platform.
    use super::*;

    fn options(rendering_mode: Vec<AndroidRenderingMode>) -> AndroidPlatformOptions {
        AndroidPlatformOptions { rendering_mode }
    }

    #[test]
    fn the_default_order_is_egl_then_software() {
        assert_eq!(
            AndroidPlatformOptions::default().rendering_mode,
            vec![AndroidRenderingMode::Egl, AndroidRenderingMode::Software]
        );
        assert_eq!(AndroidRenderingMode::Software as i32, 1);
        assert_eq!(AndroidRenderingMode::Egl as i32, 2);
        assert_eq!(AndroidRenderingMode::Vulkan as i32, 3);
    }

    #[test]
    fn the_default_order_takes_egl_when_it_initializes() {
        let mut asked = 0;
        let graphics = initialize_graphics_core(&AndroidPlatformOptions::default(), &mut || {
            asked += 1;
            Some(7)
        });
        assert_eq!(graphics, Some(7));
        assert_eq!(asked, 1);
    }

    #[test]
    fn the_default_order_ends_at_software_when_egl_does_not_initialize() {
        let mut asked = 0;
        let graphics = initialize_graphics_core::<i32>(&AndroidPlatformOptions::default(), &mut || {
            asked += 1;
            None
        });
        assert_eq!(graphics, None);
        assert_eq!(asked, 1);
    }

    #[test]
    fn software_first_does_not_ask_for_egl() {
        let graphics = initialize_graphics_core::<i32>(
            &options(vec![AndroidRenderingMode::Software, AndroidRenderingMode::Egl]),
            &mut || panic!("EGL must not be asked for"),
        );
        assert_eq!(graphics, None);
    }

    #[test]
    #[should_panic(expected = "must not be empty or null")]
    fn an_empty_list_fails() {
        initialize_graphics_core::<i32>(&options(vec![]), &mut || None);
    }

    #[test]
    #[should_panic(expected = "has a value of \"Egl, Vulkan\", but no options were applied.")]
    fn a_list_in_which_nothing_applies_fails() {
        initialize_graphics_core::<i32>(
            &options(vec![AndroidRenderingMode::Egl, AndroidRenderingMode::Vulkan]),
            &mut || None,
        );
    }
}
