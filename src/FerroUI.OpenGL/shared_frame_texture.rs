use ferroui_base::{
    platform::{
        IPlatformHandle, KnownPlatformGraphicsExternalImageHandleTypes, LeasedPlatformHandle,
    },
    rendering::composition::frames::{FrameError, FrameLease},
};
use std::{any::Any, rc::Rc};

/// Immutable storage owned by a producer, sampled in its admitted GL context.
/// Importing creates a borrowed view; it never adopts or copies this texture.
pub struct SharedFrameTexture {
    context_id: u64,
    generation: u64,
    texture: u32,
    _owner: Box<dyn Any + Send + Sync>,
}
impl SharedFrameTexture {
    /// # Safety
    /// `texture` must name RGBA8 GL_TEXTURE_2D storage in the given context or
    /// proven share group. The owner must keep it alive and immutable after
    /// producer readiness until the last frame lease/GPU completion retires.
    /// Native destruction must use the owning context, retaining allocation
    /// accounting until any queued destruction actually completes.
    pub unsafe fn new<T: Any + Send + Sync>(
        context_id: u64,
        generation: u64,
        texture: u32,
        owner: T,
    ) -> Self {
        Self {
            context_id,
            generation,
            texture,
            _owner: Box::new(owner),
        }
    }
    pub fn context_id(&self) -> u64 {
        self.context_id
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn texture(&self) -> u32 {
        self.texture
    }
    pub fn handle(frame: &FrameLease) -> Result<Rc<dyn IPlatformHandle>, FrameError> {
        let source = frame
            .resource::<Self>()
            .ok_or(FrameError::MissingImporter)?;
        Ok(Rc::new(LeasedPlatformHandle::new(
            source.texture as isize,
            Some(KnownPlatformGraphicsExternalImageHandleTypes::SHARED_GL_TEXTURE),
            frame.clone(),
        )))
    }
}
