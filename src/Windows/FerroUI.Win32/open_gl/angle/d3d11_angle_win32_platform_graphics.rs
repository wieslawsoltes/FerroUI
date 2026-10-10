//! The platform graphics of ANGLE on Direct3D 11: every context has a
//! display, and a device of Direct3D, of its own.

use super::angle_d3d_texture_feature::AngleD3DTextureFeature;
use super::angle_external_objects_feature::AngleExternalObjectsFeature;
use super::{AngleWin32EglDisplay, D3D11Adapter, Win32AngleEglInterface};
use crate::win32_platform_options::GraphicsAdapterSelectionCallback;
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::platform::{IPlatformGraphics, IPlatformGraphicsContext};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_opengl::egl::{EglContext, EglContextFeature, EglContextFeatureFactory, EglContextOptions};
use ferroui_opengl::{
    GlProfileType, GlVersion, IGlContext, IGlContextExternalObjectsFeature, IGlPlatformSurfaceRenderTargetFactory,
    IPlatformGraphicsOpenGlContextFactory, OpenGlException,
};
use std::any::TypeId;
use std::collections::HashMap;
use std::rc::Rc;

/// The display of ANGLE of a context, with the context as its shared
/// handle: a feature of every context of these graphics
/// (`try_get_feature(TypeId::of::<AngleContextDisplay>())`).
#[allow(dead_code)] // Read by the render target of the DXGI swap chain mode, which is Windows only.
pub(crate) struct AngleContextDisplay {
    pub display: Rc<AngleWin32EglDisplay>,
    pub context: std::rc::Weak<EglContext>,
}

/// The platform graphics of ANGLE on Direct3D 11.
///
/// The reference keeps the EGL interface and the display it probed with,
/// and gives that display to the first context. Both are objects of one
/// thread here (`Rc`), and the platform graphics are shared with the render
/// thread, which creates the contexts: so the graphics hold what the two
/// threads may share (the versions of OpenGL ES the options ask for and the
/// adapter that was chosen when the graphics were probed), the display of
/// the probe is disposed, and a context loads the interface and creates its
/// display on the thread that asks for it. A display more is created at
/// start; what a context is, is the same.
#[derive(Clone, Debug)]
pub struct D3D11AngleWin32PlatformGraphics {
    gl_versions: Option<Vec<GlVersion>>,
    /// The identifier of the adapter the probe chose; `None` for the first
    /// adapter of the system.
    adapter_luid: Option<u64>,
}

impl D3D11AngleWin32PlatformGraphics {
    pub fn new(gl_versions: Option<Vec<GlVersion>>, adapter_luid: Option<u64>) -> D3D11AngleWin32PlatformGraphics {
        D3D11AngleWin32PlatformGraphics { gl_versions, adapter_luid }
    }

    /// The identifier of the adapter the devices are created on, when one
    /// was chosen.
    pub fn adapter_luid(&self) -> Option<u64> {
        self.adapter_luid
    }

    #[cfg(windows)]
    fn create_display(&self) -> Result<AngleWin32EglDisplay, OpenGlException> {
        let egl = Rc::new(Win32AngleEglInterface::new()?);
        let (display, _) =
            AngleWin32EglDisplay::create_d3d11_display(&egl, self.gl_versions.clone(), &D3D11Adapter::Chosen(self.adapter_luid))?;
        Ok(display)
    }

    #[cfg(not(windows))]
    fn create_display(&self) -> Result<AngleWin32EglDisplay, OpenGlException> {
        Err(OpenGlException::new("Direct3D 11 exists on Windows only"))
    }

    /// The context of a display, which the context disposes with itself,
    /// with the two features the reference adds to it: a render target over
    /// a texture of Direct3D 11 (what the composition modes render through)
    /// and the import and export of textures as external objects.
    ///
    /// # Panics
    /// Panics when the feature of the external objects cannot be created
    /// (the exception the reference throws from the creation of the
    /// context).
    pub(super) fn create_context_for_display(display: AngleWin32EglDisplay) -> Result<Rc<EglContext>, OpenGlException> {
        let angle = Rc::new(display);
        let egl_display = angle.display().clone();
        let dispose_display = egl_display.clone();

        let mut extra_features: HashMap<TypeId, EglContextFeatureFactory> = HashMap::new();
        {
            let angle = angle.clone();
            extra_features.insert(
                TypeId::of::<dyn IGlPlatformSurfaceRenderTargetFactory>(),
                Rc::new(move |context: &Rc<EglContext>| {
                    let feature: Rc<dyn IGlPlatformSurfaceRenderTargetFactory> =
                        Rc::new(AngleD3DTextureFeature::new(context, angle.clone()));
                    EglContextFeature { feature: Rc::new(feature), disposable: None }
                }),
            );
        }
        {
            let angle = angle.clone();
            extra_features.insert(
                TypeId::of::<dyn IGlContextExternalObjectsFeature>(),
                Rc::new(move |context: &Rc<EglContext>| {
                    let feature = match AngleExternalObjectsFeature::new(context, &angle) {
                        Ok(feature) => Rc::new(feature),
                        Err(error) => panic!("{error}"),
                    };
                    let disposable = {
                        let feature = feature.clone();
                        Disposable::create(move || feature.dispose())
                    };
                    let feature: Rc<dyn IGlContextExternalObjectsFeature> = feature;
                    EglContextFeature { feature: Rc::new(feature), disposable: Some(disposable) }
                }),
            );
        }

        // The display of ANGLE itself: the reference casts the display of
        // a context where it needs it (the render target of the DXGI swap
        // chain mode); a context of the port has it as a feature.
        {
            let angle = angle.clone();
            extra_features.insert(
                TypeId::of::<AngleContextDisplay>(),
                Rc::new(move |context: &Rc<EglContext>| EglContextFeature {
                    feature: Rc::new(AngleContextDisplay { display: angle.clone(), context: Rc::downgrade(context) }),
                    disposable: None,
                }),
            );
        }

        let result = egl_display.create_context(Some(EglContextOptions {
            dispose_callback: Some(Rc::new(move || dispose_display.dispose())),
            extra_features: Some(extra_features),
            ..Default::default()
        }));
        if result.is_err() {
            egl_display.dispose();
        }
        result
    }

    /// The platform graphics, when a display and a context can be created
    /// and the context made current; `None`, with the failure logged, when
    /// not.
    #[cfg(windows)]
    pub fn try_create(
        egl: &Rc<Win32AngleEglInterface>,
        gl_versions: Option<Vec<GlVersion>>,
        selection_callback: Option<GraphicsAdapterSelectionCallback>,
    ) -> Option<D3D11AngleWin32PlatformGraphics> {
        let result = (|| -> Result<Option<u64>, OpenGlException> {
            let (display, adapter_luid) =
                AngleWin32EglDisplay::create_d3d11_display(egl, gl_versions.clone(), &D3D11Adapter::Select(selection_callback))?;
            let probe = probe_display(&display);
            display.dispose();
            probe.map(|()| adapter_luid)
        })();

        match result {
            Ok(adapter_luid) => Some(D3D11AngleWin32PlatformGraphics::new(gl_versions, adapter_luid)),
            Err(e) => {
                if let Some(logger) = Logger::try_get(LogEventLevel::Error, "OpenGL") {
                    logger.log_with_values(None, "Unable to initialize ANGLE-based rendering with DirectX11 : {0}", &[&e]);
                }
                None
            }
        }
    }
}

/// Creates a context of the display, makes it current once and disposes
/// it: what the reference does to learn that the display works.
pub(super) fn probe_display(display: &AngleWin32EglDisplay) -> Result<(), OpenGlException> {
    let ctx = display.create_context(Some(EglContextOptions::default()))?;
    let result = match ctx.make_current_with_surface(None) {
        Ok(current) => {
            current.dispose();
            Ok(())
        }
        Err(error) => Err(OpenGlException::new(error.to_string())),
    };
    IPlatformGraphicsContext::dispose(&*ctx);
    result
}

impl IPlatformGraphics for D3D11AngleWin32PlatformGraphics {
    fn uses_shared_context(&self) -> bool {
        false
    }

    /// # Panics
    /// Panics when the display or the context cannot be created (the
    /// exception of the reference).
    fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        match self.create_display().and_then(Self::create_context_for_display) {
            Ok(context) => context,
            Err(error) => panic!("{error}"),
        }
    }

    /// # Panics
    /// Always: the graphics have no shared context (the
    /// `InvalidOperationException` of the reference).
    fn get_shared_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        panic!("Operation is not valid due to the current state of the object.")
    }
}

/// Whether a context of ANGLE answers a request for `versions`: every
/// context is OpenGL ES 3, so a list without it cannot be met.
pub(crate) fn versions_can_be_met(versions: Option<&[GlVersion]>) -> bool {
    versions.is_none_or(|versions| !versions.iter().all(|v| v.type_() != GlProfileType::OpenGLES || v.major() != 3))
}

impl IPlatformGraphicsOpenGlContextFactory for D3D11AngleWin32PlatformGraphics {
    /// # Panics
    /// Panics when `versions` has no OpenGL ES 3, and when the context
    /// cannot be created (the exceptions of the reference).
    fn create_context(&self, versions: Option<&[GlVersion]>) -> Rc<dyn IGlContext> {
        if !versions_can_be_met(versions) {
            panic!("{}", OpenGlException::new("Unable to create context with requested version"));
        }

        match self.create_display().and_then(Self::create_context_for_display) {
            Ok(context) => context,
            Err(error) => panic!("{error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of the platform
    // graphics.
    use super::super::angle_win32_egl_display::tests as display_tests;
    use super::*;
    use ferroui_opengl::egl::egl_consts::EGL_BAD_DISPLAY;

    #[test]
    fn a_request_for_versions_is_met_when_it_has_open_gl_es_3() {
        let es3 = GlVersion::new(GlProfileType::OpenGLES, 3, 0);
        let es2 = GlVersion::new(GlProfileType::OpenGLES, 2, 0);
        let gl4 = GlVersion::new(GlProfileType::OpenGL, 4, 0);
        let gl3 = GlVersion::new(GlProfileType::OpenGL, 3, 3);

        assert!(versions_can_be_met(None));
        assert!(versions_can_be_met(Some(&[es2, es3])));
        assert!(versions_can_be_met(Some(&[GlVersion::new(GlProfileType::OpenGLES, 3, 1)])));
        assert!(!versions_can_be_met(Some(&[es2, gl4])));
        // OpenGL 3 is not OpenGL ES 3.
        assert!(!versions_can_be_met(Some(&[gl3])));
        // As in the reference, a list without a version cannot be met.
        assert!(!versions_can_be_met(Some(&[])));
    }

    #[test]
    fn the_graphics_have_no_shared_context_and_keep_what_the_probe_chose() {
        let graphics = D3D11AngleWin32PlatformGraphics::new(None, Some(0x1234_5678_9ABC));

        assert!(!graphics.uses_shared_context());
        assert_eq!(Some(0x1234_5678_9ABC), graphics.adapter_luid());
        // Shared with the render thread.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<D3D11AngleWin32PlatformGraphics>();
    }

    #[test]
    fn a_display_whose_context_cannot_be_made_current_fails_the_probe() {
        // The EGL of the tests creates no context (the entry point answers
        // zero), which is the error of the probe.
        let display = AngleWin32EglDisplay::create_shared_d3d11_display(&display_tests::egl()).expect("a display");

        let error = probe_display(&display).err().expect("an error");

        assert_eq!(Some(EGL_BAD_DISPLAY), error.error_code());
        display.dispose();
    }
}
