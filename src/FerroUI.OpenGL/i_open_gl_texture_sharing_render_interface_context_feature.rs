use crate::{GlVersion, IGlContext};
use ferroui_base::rendering::composition::ICompositionImportableSharedGpuContextImage;
use ferroui_base::PixelSize;
use std::rc::Rc;

/// The feature of a render interface context that shares OpenGL textures with other contexts
/// of its share group.
///
/// A render interface context announces the feature through its optional features:
/// `try_get_feature(TypeId::of::<dyn IOpenGlTextureSharingRenderInterfaceContextFeature>())`
/// returns an `Rc<dyn IOpenGlTextureSharingRenderInterfaceContextFeature>`.
pub trait IOpenGlTextureSharingRenderInterfaceContextFeature {
    fn can_create_shared_context(&self) -> bool;

    fn create_shared_context(&self, preferred_versions: Option<&[GlVersion]>) -> Option<Rc<dyn IGlContext>>;

    fn create_shared_texture_for_composition(
        &self,
        context: &Rc<dyn IGlContext>,
        size: PixelSize,
    ) -> Rc<dyn ICompositionImportableOpenGlSharedTexture>;
}

/// An OpenGL texture of a context of the share group of the compositor, which the compositor
/// can import.
pub trait ICompositionImportableOpenGlSharedTexture: ICompositionImportableSharedGpuContextImage {
    fn texture_id(&self) -> i32;

    fn internal_format(&self) -> i32;

    fn size(&self) -> PixelSize;
}
