use super::composition_gl_context::CompositionGlContext;
use super::{CompositionGlContextOptions, ICompositionGlContext};
use crate::{
    IGlContextExternalObjectsFeature, IOpenGlTextureSharingRenderInterfaceContextFeature,
    IPlatformGraphicsOpenGlContextFactory,
};
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::platform::{IOptionalFeatureProvider, KnownPlatformGraphicsExternalImageHandleTypes};
use ferroui_base::rendering::composition::{Compositor, RenderInterfaceFeature};
use ferroui_base::{FerroLocator, LocatorExtensions};
use std::any::{Any, TypeId};
use std::rc::Rc;

/// The creation of OpenGL contexts that draw into the surfaces of a compositor.
pub struct OpenGlCompositionInterop;

impl OpenGlCompositionInterop {
    /// Attempts to create an OpenGL context that is capable of drawing to a
    /// `CompositionDrawingSurface` of the compositor (`TryCreateCompatibleGlContextAsync`).
    /// The context is either shared with the compositor's rendering context or uses GPU
    /// memory interop to transfer frames. Returns `None` if the current platform or
    /// rendering backend does not support any compatible way of OpenGL interop.
    ///
    /// The original awaits the answers of the render thread; here this thread enters the
    /// compositor lock for them, so the result is given directly. The sharing feature is an
    /// object of the server side: it is asked inside the lock, through the handle the
    /// compositor hands out, and the context it creates for this thread is what leaves it.
    ///
    /// `options` are optional parameters for the context creation.
    // A failure of the context factory or of the sharing feature is a panic of that call
    // here: the original catches it, logs it and returns null.
    pub fn try_create_compatible_gl_context(
        compositor: &Rc<Compositor>,
        options: Option<&CompositionGlContextOptions>,
    ) -> Option<Rc<dyn ICompositionGlContext>> {
        compositor.dispatcher().verify_access();

        let sharing_feature = compositor
            .try_get_render_interface_feature(TypeId::of::<dyn IOpenGlTextureSharingRenderInterfaceContextFeature>())
            .filter(|feature| feature.is::<Rc<dyn IOpenGlTextureSharingRenderInterfaceContextFeature>>());
        let Some(interop) = compositor.try_get_composition_gpu_interop() else {
            // Expected on platforms without GPU interop support, so not an error
            log_unsupported("Compositor backend doesn't support GPU interop");
            return None;
        };

        let gl_profiles = options.and_then(|options| options.gl_profiles());
        if let Some(sharing_feature) = sharing_feature.filter(can_create_shared_context) {
            let context = sharing_feature
                .with::<dyn IOpenGlTextureSharingRenderInterfaceContextFeature, _>(|feature| {
                    feature.create_shared_context(gl_profiles)
                })
                .flatten();
            match context {
                None => log_error("Unable to create a context shared with the compositor"),
                Some(context) => {
                    return Some(CompositionGlContext::new(
                        compositor.clone(),
                        context,
                        interop,
                        Some(sharing_feature),
                        None,
                    ));
                }
            }
        }

        let Some(context_factory) = FerroLocator::current().get_service::<dyn IPlatformGraphicsOpenGlContextFactory>()
        else {
            log_unsupported("The current platform doesn't provide an OpenGL context factory");
            return None;
        };

        let ctx = context_factory.create_context(gl_profiles);
        let external_objects = {
            let provider: &dyn IOptionalFeatureProvider = &*ctx;
            provider.try_get::<dyn IGlContextExternalObjectsFeature>()
        };
        let handle_type = KnownPlatformGraphicsExternalImageHandleTypes::D3D11_TEXTURE_GLOBAL_SHARED_HANDLE;
        if let Some(external_objects) = external_objects {
            if interop.supported_image_handle_types().iter().any(|t| t == handle_type)
                && external_objects.supported_exportable_external_image_types().iter().any(|t| t == handle_type)
            {
                return Some(CompositionGlContext::new(compositor.clone(), ctx, interop, None, Some(external_objects)));
            }
        }

        log_unsupported(
            "The current platform doesn't support OpenGL context sharing with the compositor \
             or a compatible shared memory type",
        );
        ctx.dispose();
        None
    }
}

/// Whether the sharing feature behind `feature` can create a context shared with the
/// compositor: asked inside the compositor lock.
fn can_create_shared_context(feature: &RenderInterfaceFeature) -> bool {
    feature
        .with::<dyn IOpenGlTextureSharingRenderInterfaceContextFeature, _>(|feature| feature.can_create_shared_context())
        .unwrap_or(false)
}

fn log_unsupported(message: &str) {
    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, "OpenGL") {
        let source: &dyn Any = &"OpenGlCompositionInterop";
        logger.log(Some(source), message);
    }
}

fn log_error(message: &str) {
    if let Some(logger) = Logger::try_get(LogEventLevel::Error, "OpenGL") {
        let source: &dyn Any = &"OpenGlCompositionInterop";
        logger.log(Some(source), message);
    }
}
