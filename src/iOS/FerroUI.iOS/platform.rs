//! The platform: its options, `use_ios`, and the registration of the
//! services of the platform.

use std::fmt;

/// Represents the rendering mode for platform graphics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum IosRenderingMode {
    /// Enables EAGL rendering for iOS and tvOS. Not supported on Mac
    /// Catalyst.
    OpenGl = 1,

    /// Enables Metal rendering for all Apple targets.
    Metal,
}

impl fmt::Display for IosRenderingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            IosRenderingMode::OpenGl => "OpenGl",
            IosRenderingMode::Metal => "Metal",
        })
    }
}

/// iOS backend options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IosPlatformOptions {
    /// The rendering modes with fallbacks. The first element has the
    /// highest priority. The default value is Metal, then OpenGL.
    ///
    /// The platform fails to start when no value could be applied.
    pub rendering_mode: Vec<IosRenderingMode>,
}

impl Default for IosPlatformOptions {
    fn default() -> Self {
        Self { rendering_mode: vec![IosRenderingMode::Metal, IosRenderingMode::OpenGl] }
    }
}

/// Walks the rendering modes of the options and returns the graphics of
/// the first one that can be created.
///
/// `try_create` answers for one mode. The errors are the messages the
/// platform fails with.
#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
pub(crate) fn initialize_graphics_with<T>(
    opts: &IosPlatformOptions,
    try_create: impl Fn(IosRenderingMode) -> Option<T>,
) -> Result<T, String> {
    if opts.rendering_mode.is_empty() {
        return Err("IosPlatformOptions.rendering_mode must not be empty or null".to_string());
    }

    for rendering_mode in &opts.rendering_mode {
        if let Some(graphics) = try_create(*rendering_mode) {
            return Ok(graphics);
        }
    }

    let modes: Vec<String> = opts.rendering_mode.iter().map(ToString::to_string).collect();
    Err(format!(
        "IosPlatformOptions.rendering_mode has a value of \"{}\", but no options were applied.",
        modes.join(", ")
    ))
}

#[cfg(target_os = "ios")]
pub use uikit::{IosApplicationExtensions, Platform};

#[cfg(target_os = "ios")]
mod uikit {
    use super::{initialize_graphics_with, IosPlatformOptions, IosRenderingMode};
    use crate::activatable_lifetime::ActivatableLifetime;
    use crate::dispatcher_impl::DispatcherImpl;
    use crate::display_link_timer::DisplayLinkTimer;
    use crate::ferro_app_delegate::IFerroAppDelegate;
    use crate::ios_screens::IosScreens;
    use crate::metal::MetalPlatformGraphics;
    use crate::platform_settings::PlatformSettings;
    use crate::stubs::{CursorFactoryStub, PlatformIconLoaderStub, WindowingPlatformStub};
    use ferroui_base::input::platform::{KeyGestureFormatInfo, PlatformHotkeyConfiguration};
    use ferroui_base::input::{IKeyboardDevice, Key, KeyModifiers, KeyboardDevice};
    use ferroui_base::platform::{ICursorFactory, IPlatformGraphics, IPlatformSettings};
    use ferroui_base::rendering::composition::Compositor;
    use ferroui_base::rendering::{IRenderLoop, IRenderTimer, RenderLoop};
    use ferroui_base::threading::Dispatcher;
    use ferroui_base::{FerroLocator, LocatorExtensions};
    use ferroui_controls::application_lifetimes::IActivatableLifetime;
    use ferroui_controls::platform::{IPlatformIconLoader, IScreenImpl, IWindowingPlatform};
    use ferroui_controls::AppBuilder;
    use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
    use ferroui_skia::SkiaApplicationExtensions;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;
    use std::sync::Arc;

    /// Selects the iOS platform for an application.
    pub trait IosApplicationExtensions {
        /// Uses the iOS platform, with the Skia renderer and the HarfBuzz
        /// text shaper, without an application delegate (an application
        /// that embeds views in a UIKit application of its own).
        fn use_ios(&self) -> AppBuilder;

        /// Uses the iOS platform with the application delegate whose
        /// activations the activatable lifetime of the application reports.
        fn use_ios_with_delegate(&self, app_delegate: Rc<dyn IFerroAppDelegate>) -> AppBuilder;
    }

    fn use_ios(builder: &AppBuilder, app_delegate: Option<Rc<dyn IFerroAppDelegate>>) -> AppBuilder {
        builder
            .use_standard_runtime_platform_subsystem()
            .use_windowing_subsystem(move || Platform::register(app_delegate.clone()), "iOS")
            .use_harfbuzz()
            .use_skia()
    }

    impl IosApplicationExtensions for AppBuilder {
        fn use_ios(&self) -> AppBuilder {
            use_ios(self, None)
        }

        fn use_ios_with_delegate(&self, app_delegate: Rc<dyn IFerroAppDelegate>) -> AppBuilder {
            use_ios(self, Some(app_delegate))
        }
    }

    #[derive(Default)]
    struct PlatformState {
        options: Option<Rc<IosPlatformOptions>>,
        graphics: Option<Arc<dyn IPlatformGraphics>>,
        timer: Option<Arc<DisplayLinkTimer>>,
        compositor: Option<Rc<Compositor>>,
        settings: Option<Rc<PlatformSettings>>,
    }

    thread_local! {
        /// The state of the platform: what the reference keeps in static
        /// fields. The platform is registered on the main thread, and its
        /// state is read there.
        static STATE: RefCell<PlatformState> = RefCell::new(PlatformState::default());
    }

    /// The iOS platform.
    pub struct Platform;

    impl Platform {
        /// The options the platform was registered with.
        pub fn options() -> Option<Rc<IosPlatformOptions>> {
            STATE.with(|state| state.borrow().options.clone())
        }

        /// The graphics of the platform.
        pub fn graphics() -> Option<Arc<dyn IPlatformGraphics>> {
            STATE.with(|state| state.borrow().graphics.clone())
        }

        /// The render timer of the platform.
        pub fn timer() -> Option<Arc<DisplayLinkTimer>> {
            STATE.with(|state| state.borrow().timer.clone())
        }

        /// The compositor of the platform.
        pub(crate) fn compositor() -> Option<Rc<Compositor>> {
            STATE.with(|state| state.borrow().compositor.clone())
        }

        /// The settings of the platform, once the platform is registered.
        /// They are created when first asked for, here or through the
        /// locator. The reference finds them in the locator and casts
        /// them to their class; the contract of the port has no cast, so
        /// the platform keeps the object it registers.
        pub(crate) fn settings() -> Option<Rc<PlatformSettings>> {
            STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.options.as_ref()?;
                Some(state.settings.get_or_insert_with(PlatformSettings::new).clone())
            })
        }

        /// Registers the services of the platform.
        ///
        /// # Panics
        /// Panics when none of the rendering modes of the options can be
        /// applied.
        pub fn register(app_delegate: Option<Rc<dyn IFerroAppDelegate>>) {
            let options =
                FerroLocator::current().get_service::<IosPlatformOptions>().unwrap_or_default();

            let graphics = Self::initialize_graphics(&options);
            let timer = match Self::timer() {
                Some(timer) => timer,
                None => DisplayLinkTimer::new(),
            };
            let keyboard = KeyboardDevice::new();

            STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.options = Some(options);
                state.graphics = Some(graphics.clone());
                state.timer = Some(timer.clone());
            });

            Dispatcher::initialize_ui_thread_dispatcher(DispatcherImpl::instance());

            let cursor_factory: Rc<dyn ICursorFactory> = Rc::new(CursorFactoryStub);
            let windowing_platform: Rc<dyn IWindowingPlatform> = Rc::new(WindowingPlatformStub);
            let icon_loader: Rc<dyn IPlatformIconLoader> = Rc::new(PlatformIconLoaderStub);
            let key_names = HashMap::from([
                (Key::Back, "\u{232B}".to_string()),
                (Key::Down, "\u{2193}".to_string()),
                (Key::End, "\u{2198}".to_string()),
                (Key::Escape, "\u{238B}".to_string()),
                (Key::Home, "\u{2196}".to_string()),
                (Key::Left, "\u{2190}".to_string()),
                (Key::Return, "\u{21A9}".to_string()),
                (Key::PageDown, "\u{21DF}".to_string()),
                (Key::PageUp, "\u{21DE}".to_string()),
                (Key::Right, "\u{2192}".to_string()),
                (Key::Space, "\u{2423}".to_string()),
                (Key::Tab, "\u{21E5}".to_string()),
                (Key::Up, "\u{2191}".to_string()),
            ]);
            let render_timer: Arc<dyn IRenderTimer> = timer;
            let render_loop: Arc<dyn IRenderLoop> = RenderLoop::from_timer(render_timer);
            let keyboard: Rc<dyn IKeyboardDevice> = keyboard;

            let locator = FerroLocator::current_mutable();
            locator
                .bind::<Arc<dyn IPlatformGraphics>>()
                .to_constant(Rc::new(graphics.clone()))
                .bind::<dyn ICursorFactory>()
                .to_constant(cursor_factory)
                .bind::<dyn IWindowingPlatform>()
                .to_constant(windowing_platform)
                .bind::<dyn IPlatformSettings>()
                .to_lazy(|| Platform::settings().map(|settings| settings as Rc<dyn IPlatformSettings>))
                .bind::<dyn IPlatformIconLoader>()
                .to_constant(icon_loader)
                .bind::<dyn IScreenImpl>()
                .to_lazy(|| Some(IosScreens::new() as Rc<dyn IScreenImpl>))
                .bind_to_self(Rc::new(PlatformHotkeyConfiguration::new(KeyModifiers::CONTROL)))
                .bind_to_self(Rc::new(KeyGestureFormatInfo::new(
                    Some(key_names),
                    "\u{2318}",
                    "\u{2303}",
                    "\u{2325}",
                    "\u{21E7}",
                )))
                .bind::<Arc<dyn IRenderLoop>>()
                .to_constant(Rc::new(render_loop))
                .bind::<dyn IKeyboardDevice>()
                .to_constant(keyboard);

            if let Some(app_delegate) = app_delegate {
                let lifetime: Rc<dyn IActivatableLifetime> = ActivatableLifetime::new(&*app_delegate);
                locator.bind::<dyn IActivatableLifetime>().to_constant(lifetime);
            }

            let compositor = Compositor::new(Some(graphics), false);
            STATE.with(|state| state.borrow_mut().compositor = Some(compositor.clone()));
            locator.bind_to_self(compositor);
        }

        fn initialize_graphics(opts: &IosPlatformOptions) -> Arc<dyn IPlatformGraphics> {
            let graphics = initialize_graphics_with(opts, |rendering_mode| match rendering_mode {
                // Stage 4 of docs/porting/ios-platform.md: OpenGL ES over
                // an EAGL layer is not built, so the mode counts as one
                // that could not be created and the next mode is tried.
                IosRenderingMode::OpenGl => None,
                IosRenderingMode::Metal => {
                    MetalPlatformGraphics::try_create().map(|graphics| graphics as Arc<dyn IPlatformGraphics>)
                }
            });
            match graphics {
                Ok(graphics) => graphics,
                Err(message) => panic!("{message}"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this file.
    use super::*;

    #[test]
    fn the_default_options_are_those_of_the_reference() {
        let options = IosPlatformOptions::default();
        assert_eq!(options.rendering_mode, vec![IosRenderingMode::Metal, IosRenderingMode::OpenGl]);
    }

    #[test]
    fn the_modes_have_the_values_of_the_reference() {
        assert_eq!(1, IosRenderingMode::OpenGl as i32);
        assert_eq!(2, IosRenderingMode::Metal as i32);
    }

    #[test]
    fn the_first_mode_that_can_be_created_is_used() {
        let options = IosPlatformOptions { rendering_mode: vec![IosRenderingMode::OpenGl, IosRenderingMode::Metal] };
        let graphics =
            initialize_graphics_with(&options, |mode| (mode == IosRenderingMode::Metal).then_some("metal"));
        assert_eq!(Ok("metal"), graphics);

        let graphics = initialize_graphics_with(&IosPlatformOptions::default(), |mode| Some(mode.to_string()));
        assert_eq!(Ok("Metal".to_string()), graphics);
    }

    #[test]
    fn no_mode_is_an_error() {
        let options = IosPlatformOptions { rendering_mode: Vec::new() };
        let graphics = initialize_graphics_with(&options, |_| Some(()));
        assert_eq!(Err("IosPlatformOptions.rendering_mode must not be empty or null".to_string()), graphics);
    }

    #[test]
    fn modes_of_which_none_applies_are_an_error_that_names_them() {
        let graphics = initialize_graphics_with(&IosPlatformOptions::default(), |_| None::<()>);
        assert_eq!(
            Err("IosPlatformOptions.rendering_mode has a value of \"Metal, OpenGl\", but no options were applied."
                .to_string()),
            graphics
        );
    }
}
