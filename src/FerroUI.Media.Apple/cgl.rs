//! A borrowed CGL rectangle view of the same retained camera/decoder storage.
use crate::*;
use ferroui_base::platform::{
    IOptionalFeatureProvider, PlatformGraphicsExternalImageFormat,
    PlatformGraphicsExternalImageProperties,
};
use ferroui_opengl::{IGlContext, IGlContextExternalObjectsFeature, IGlExternalImageTexture};

/// Import and use on the compositor's GL render thread. Dropping this view
/// schedules a real consumer fence before releasing the alias. No pixel storage
/// is allocated or copied. Texture coordinates are in pixels, not normalized.
pub struct CglFrameView {
    context: Rc<dyn IGlContext>,
    texture: Rc<dyn IGlExternalImageTexture>,
    completion: GpuCompletion,
}
impl CglFrameView {
    pub fn import(context: &Rc<dyn IGlContext>, frame: &FrameLease) -> Result<Self, MediaError> {
        let fail = |text| MediaError::new(MediaErrorKind::Unsupported, text);
        let m = frame.metadata();
        m.validate(8192).map_err(|e| fail(e.to_string()))?;
        if !frame.is_current() || m.device_generation != context.shared_frame_context_generation() {
            return Err(MediaError::cancelled());
        }
        let features: &dyn IOptionalFeatureProvider = &**context;
        let feature = features
            .try_get::<dyn IGlContextExternalObjectsFeature>()
            .ok_or_else(|| fail("No CGL external-image importer".into()))?;
        if context.shared_frame_context_id().is_none()
            || m.device.is_none()
            || m.device != feature.device_luid().map(DeviceIdentity::Luid)
            || m.format != FrameFormat::Bgra8
            || !m.top_left_origin
        {
            return Err(fail(
                "Incompatible CGL source device, format or origin".into(),
            ));
        }
        let current = context.ensure_current();
        let texture = feature.import_image(
            frame_surface(frame)?,
            PlatformGraphicsExternalImageProperties {
                width: m.width as i32,
                height: m.height as i32,
                format: PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm,
                top_left_origin: true,
                memory_size: m.retained_bytes,
                ..Default::default()
            },
        );
        current.dispose();
        let texture = texture.map_err(|e| fail(e.to_string()))?;
        let retained = match texture.acquire_frame_lease(frame.clone()) {
            Ok(frame) => frame,
            Err(e) => {
                texture.dispose();
                return Err(fail(e.to_string()));
            }
        };
        let completion = GpuCompletion::new();
        if let Err(e) = completion.retain(retained) {
            texture.dispose();
            return Err(fail(e.to_string()));
        }
        frame.counters().gpu_import();
        Ok(Self {
            context: context.clone(),
            texture,
            completion,
        })
    }
    pub fn texture(&self) -> u32 {
        self.texture.texture_id() as u32
    }
    pub fn target(&self) -> u32 {
        self.texture.texture_type() as u32
    }
}
impl Drop for CglFrameView {
    fn drop(&mut self) {
        // One source allocation may also be sampled by the ordinary presenter.
        // Its independently owned native views retire with their own fences.
        self.context
            .schedule_frame_completion(self.completion.clone());
        self.texture.dispose();
    }
}
