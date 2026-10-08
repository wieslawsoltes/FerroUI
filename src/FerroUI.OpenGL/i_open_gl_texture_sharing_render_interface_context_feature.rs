use crate::{GlVersion, IGlContext};
use ferroui_base::rendering::composition::ICompositionImportableSharedGpuContextImage;
use ferroui_base::PixelSize;
use std::rc::Rc;
use std::sync::Arc;

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
    ) -> Arc<dyn ICompositionImportableOpenGlSharedTexture>;
}

/// An OpenGL texture of a context of the share group of the compositor, which the compositor
/// can import.
///
/// The texture is shared between the thread of the context it was made on and the thread that
/// renders (`Send + Sync`, from the image contract). The identifier, the internal format and the
/// size are read by both: plain values, the identifier behind a lock or an atomic since the
/// disposal clears it. The context the texture was made on is an object of its thread: an
/// implementation keeps it bound to that thread (`ThreadBound`) and deletes the texture with
/// the context of the thread that disposes it, one of the same share group, as the original
/// does.
pub trait ICompositionImportableOpenGlSharedTexture: ICompositionImportableSharedGpuContextImage {
    /// The identifier of the texture, 0 once the texture is disposed.
    fn texture_id(&self) -> i32;

    fn internal_format(&self) -> i32;

    fn size(&self) -> PixelSize;
}
