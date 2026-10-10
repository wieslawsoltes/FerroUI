//! Chooses the platform graphics: the first rendering mode of the options
//! that can be initialized, and with ANGLE the first composition mode.

use crate::angle_options::AngleOptions;
use crate::open_gl::angle::{AnglePlatformGraphics, AngleWin32PlatformGraphicsFactory, D3D11AngleWin32PlatformGraphics};
use crate::win32_platform_options::{Win32CompositionMode, Win32PlatformOptions, Win32RenderingMode};
use ferroui_base::platform::IPlatformGraphics;
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_opengl::IPlatformGraphicsOpenGlContextFactory;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

/// What the platform graphics the manager registered are. The reference
/// tests the type of the registered object where it needs to know (a window
/// creates the surface that fits it); the contract of the port does not
/// give the type back, so the manager remembers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Win32PlatformGraphicsKind {
    /// ANGLE on Direct3D 11: a window is rendered through an EGL window
    /// surface.
    AngleD3D11,
}

thread_local! {
    static KIND: Cell<Option<Win32PlatformGraphicsKind>> = const { Cell::new(None) };
}

pub(crate) struct Win32GlManager;

impl Win32GlManager {
    /// The platform graphics of the first rendering mode that initializes,
    /// registered with the services; `None` for the software mode.
    ///
    /// # Panics
    /// Panics, as the reference throws, when the list of rendering modes is
    /// empty or none of its modes could be applied, and likewise for the
    /// composition modes once ANGLE is chosen.
    pub fn initialize(opts: &Win32PlatformOptions) -> Option<Arc<dyn IPlatformGraphics>> {
        let locator = FerroLocator::current();
        let angle_options = locator.get_service::<AngleOptions>().map_or_else(AngleOptions::default, |options| (*options).clone());
        let selection_callback = opts.graphics_adapter_selection_callback.clone();

        let gl = Self::initialize_core(
            opts,
            &mut || AngleWin32PlatformGraphicsFactory::try_create(Some(&angle_options), selection_callback.clone()),
            &mut Self::try_create_and_register_composition,
        )?;

        KIND.with(|kind| kind.set(Some(Win32PlatformGraphicsKind::AngleD3D11)));
        let open_gl_factory: Rc<dyn IPlatformGraphicsOpenGlContextFactory> = Rc::new(gl.clone());
        let gl: Arc<dyn IPlatformGraphics> = Arc::new(gl);
        FerroLocator::current_mutable()
            .bind::<Arc<dyn IPlatformGraphics>>()
            .to_constant(Rc::new(gl.clone()))
            .bind::<dyn IPlatformGraphicsOpenGlContextFactory>()
            .to_constant(open_gl_factory);

        Some(gl)
    }

    /// What the registered platform graphics are; `None` without platform
    /// graphics of this backend (the software mode, custom graphics).
    pub fn platform_graphics_kind() -> Option<Win32PlatformGraphicsKind> {
        KIND.with(Cell::get)
    }

    /// Registers a composition mode that presents through a surface of its
    /// own, when the system supports it and it initializes.
    ///
    #[cfg(windows)]
    fn try_create_and_register_composition(composition_mode: Win32CompositionMode) -> bool {
        use crate::d_composition::DirectCompositionConnection;
        use crate::direct_x::DxgiConnection;
        use crate::win32_platform::Win32Platform;
        use crate::win_rt::composition::WinUiCompositorConnection;

        match composition_mode {
            Win32CompositionMode::WinUIComposition => {
                WinUiCompositorConnection::is_supported() && WinUiCompositorConnection::try_create_and_register()
            }
            Win32CompositionMode::DirectComposition => {
                DirectCompositionConnection::is_supported(Win32Platform::windows_version())
                    && DirectCompositionConnection::try_create_and_register()
            }
            Win32CompositionMode::LowLatencyDxgiSwapChain => DxgiConnection::try_create_and_register(),
            Win32CompositionMode::RedirectionSurface => false,
        }
    }

    #[cfg(not(windows))]
    fn try_create_and_register_composition(_composition_mode: Win32CompositionMode) -> bool {
        false
    }

    /// The loop over the rendering modes. `try_create_angle` creates the
    /// platform graphics of ANGLE; `try_register_composition` registers a
    /// composition mode other than the redirection surface.
    ///
    /// The modes that are not built yet are passed over like a mode that
    /// fails to initialize: the OpenGL of the system (WGL, a later step of
    /// stage 2b) and Vulkan (after the Vulkan project of the port).
    pub(crate) fn initialize_core(
        opts: &Win32PlatformOptions,
        try_create_angle: &mut dyn FnMut() -> Option<AnglePlatformGraphics>,
        try_register_composition: &mut dyn FnMut(Win32CompositionMode) -> bool,
    ) -> Option<D3D11AngleWin32PlatformGraphics> {
        if opts.rendering_mode.is_empty() {
            panic!("Win32PlatformOptions.rendering_mode must not be empty or null");
        }

        for rendering_mode in &opts.rendering_mode {
            match rendering_mode {
                Win32RenderingMode::Software => return None,
                Win32RenderingMode::AngleEgl => match try_create_angle() {
                    Some(AnglePlatformGraphics::D3D11(egl)) => {
                        Self::try_register_composition(opts, try_register_composition);
                        return Some(egl);
                    }
                    // As in the reference, the graphics of Direct3D 9 are
                    // not taken; they are released here, where the
                    // reference leaves them to the collector.
                    Some(AnglePlatformGraphics::D3D9(d3d9)) => d3d9.dispose(),
                    None => {}
                },
                Win32RenderingMode::Wgl | Win32RenderingMode::Vulkan => {}
            }
        }

        panic!(
            "Win32PlatformOptions.rendering_mode has a value of \"{}\", but no options were applied.",
            opts.rendering_mode.iter().map(|mode| format!("{mode:?}")).collect::<Vec<_>>().join(", ")
        );
    }

    /// The loop over the composition modes: the redirection surface needs
    /// nothing; any other mode has to register itself.
    fn try_register_composition(
        opts: &Win32PlatformOptions,
        try_register_composition: &mut dyn FnMut(Win32CompositionMode) -> bool,
    ) {
        if opts.composition_mode.is_empty() {
            panic!("Win32PlatformOptions.composition_mode must not be empty or null");
        }

        for composition_mode in &opts.composition_mode {
            if *composition_mode == Win32CompositionMode::RedirectionSurface {
                return;
            }

            if try_register_composition(*composition_mode) {
                return;
            }
        }

        panic!(
            "Win32PlatformOptions.composition_mode has a value of \"{}\", but no options were applied.",
            opts.composition_mode.iter().map(|mode| format!("{mode:?}")).collect::<Vec<_>>().join(", ")
        );
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of the manager.
    use super::*;

    fn options(rendering_mode: Vec<Win32RenderingMode>) -> Win32PlatformOptions {
        Win32PlatformOptions { rendering_mode, ..Default::default() }
    }

    fn d3d11() -> Option<AnglePlatformGraphics> {
        Some(AnglePlatformGraphics::D3D11(D3D11AngleWin32PlatformGraphics::new(None, Some(7))))
    }

    #[test]
    fn the_default_order_takes_angle_when_it_initializes() {
        let mut asked = 0;
        let mut composition = Vec::new();
        let gl = Win32GlManager::initialize_core(
            &Win32PlatformOptions::default(),
            &mut || {
                asked += 1;
                d3d11()
            },
            &mut |mode| {
                composition.push(mode);
                false
            },
        );

        assert_eq!(Some(7), gl.expect("the graphics of ANGLE").adapter_luid());
        assert_eq!(1, asked);
        // The default order, up to the redirection surface, which needs
        // nothing registered.
        assert_eq!(vec![Win32CompositionMode::WinUIComposition, Win32CompositionMode::DirectComposition], composition);
    }

    #[test]
    fn the_default_order_falls_back_to_software_when_angle_does_not_initialize() {
        let mut asked = 0;
        let gl = Win32GlManager::initialize_core(
            &Win32PlatformOptions::default(),
            &mut || {
                asked += 1;
                None
            },
            &mut |_| panic!("no composition without ANGLE"),
        );

        assert!(gl.is_none());
        assert_eq!(1, asked);
    }

    #[test]
    fn software_first_never_asks_for_angle() {
        let mut asked = 0;
        let gl = Win32GlManager::initialize_core(
            &options(vec![Win32RenderingMode::Software, Win32RenderingMode::AngleEgl]),
            &mut || {
                asked += 1;
                d3d11()
            },
            &mut |_| false,
        );

        assert!(gl.is_none());
        assert_eq!(0, asked);
    }

    #[test]
    #[should_panic(expected = "has a value of \"AngleEgl, Wgl, Vulkan\", but no options were applied.")]
    fn a_list_in_which_nothing_initializes_is_an_error() {
        Win32GlManager::initialize_core(
            &options(vec![Win32RenderingMode::AngleEgl, Win32RenderingMode::Wgl, Win32RenderingMode::Vulkan]),
            &mut || None,
            &mut |_| false,
        );
    }

    #[test]
    #[should_panic(expected = "rendering_mode must not be empty or null")]
    fn an_empty_list_is_an_error() {
        Win32GlManager::initialize_core(&options(Vec::new()), &mut || None, &mut |_| false);
    }

    #[test]
    #[should_panic(expected = "composition_mode has a value of \"WinUIComposition, DirectComposition\", but no options were applied.")]
    fn angle_without_a_composition_mode_that_applies_is_an_error() {
        let opts = Win32PlatformOptions {
            composition_mode: vec![Win32CompositionMode::WinUIComposition, Win32CompositionMode::DirectComposition],
            ..Default::default()
        };

        Win32GlManager::initialize_core(&opts, &mut d3d11, &mut |_| false);
    }

    #[test]
    fn the_first_composition_mode_that_registers_ends_the_list() {
        let opts = Win32PlatformOptions {
            composition_mode: vec![
                Win32CompositionMode::LowLatencyDxgiSwapChain,
                Win32CompositionMode::DirectComposition,
                Win32CompositionMode::WinUIComposition,
            ],
            ..Default::default()
        };
        let mut asked = Vec::new();

        let gl = Win32GlManager::initialize_core(&opts, &mut d3d11, &mut |mode| {
            asked.push(mode);
            mode == Win32CompositionMode::DirectComposition
        });

        assert!(gl.is_some());
        assert_eq!(vec![Win32CompositionMode::LowLatencyDxgiSwapChain, Win32CompositionMode::DirectComposition], asked);
    }

    #[test]
    fn the_redirection_surface_ends_the_list_without_registering() {
        let opts = Win32PlatformOptions {
            composition_mode: vec![Win32CompositionMode::RedirectionSurface, Win32CompositionMode::DirectComposition],
            ..Default::default()
        };

        let gl = Win32GlManager::initialize_core(&opts, &mut d3d11, &mut |_| panic!("nothing is registered"));

        assert!(gl.is_some());
    }

    #[test]
    #[should_panic(expected = "composition_mode must not be empty or null")]
    fn angle_with_an_empty_list_of_composition_modes_is_an_error() {
        let opts = Win32PlatformOptions { composition_mode: Vec::new(), ..Default::default() };

        Win32GlManager::initialize_core(&opts, &mut d3d11, &mut |_| false);
    }
}
