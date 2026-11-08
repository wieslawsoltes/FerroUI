//! Direct Metal presentation. Drawing surfaces and retained layers stay on the GPU.
use crate::{bitmap::Bitmap, ffi, warn_once};
use ferroui_base::{
    platform::{surfaces::*, *},
    PixelSize, RenderTargetCorruptedError, Vector,
};
use ferroui_metal::{try_get_metal_surface, IMetalDevice, IMetalPlatformSurfaceRenderTarget};
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    rc::Rc,
    sync::Arc,
};

/// The Metal half of the Quartz render context: the platform's device, configured as the GPU
/// of new surfaces.
pub struct MetalContext {
    device: Rc<dyn IMetalDevice>,
    external: Option<Rc<crate::external_objects::ExternalObjects>>,
}
impl MetalContext {
    /// Adopts the platform's Metal device. Fails when there is no Metal graphics context, or
    /// the device cannot run the renderer (no tier-2 read/write textures, failed shaders).
    pub fn new(graphics: Option<Rc<dyn IPlatformGraphicsContext>>) -> Result<Self, String> {
        let graphics = graphics.ok_or("the platform supplied no graphics context")?;
        let device = graphics
            .try_get_feature(TypeId::of::<dyn IMetalDevice>())
            .and_then(|f| f.downcast_ref::<Rc<dyn IMetalDevice>>().cloned())
            .ok_or("the platform graphics context is not a Metal device")?;
        // SAFETY: the platform keeps its MTLDevice alive for the device's lifetime.
        let status = unsafe { ffi::qm_configure(device.device()) };
        if status != ffi::STATUS_OK {
            return Err(ffi::status_message(status).to_string());
        }
        static GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let generation = GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let external = device
            .try_get_feature(TypeId::of::<dyn ferroui_metal::IMetalExternalObjectsFeature>())
            .and_then(|f| {
                f.downcast_ref::<Rc<dyn ferroui_metal::IMetalExternalObjectsFeature>>()
                    .cloned()
            })
            .map(|inner| {
                Rc::new(crate::external_objects::ExternalObjects::new(
                    device.clone(),
                    generation,
                    inner,
                ))
            });
        Ok(Self { device, external })
    }
    pub fn try_get_feature(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        if kind == TypeId::of::<dyn IExternalObjectsHandleWrapRenderInterfaceContextFeature>() {
            return self.device.try_get_feature(kind);
        }
        if kind == TypeId::of::<dyn IExternalObjectsRenderInterfaceContextFeature>() {
            let external: Rc<dyn IExternalObjectsRenderInterfaceContextFeature> =
                self.external.clone()?;
            return Some(Rc::new(external));
        }
        None
    }
    pub fn public_features(&self) -> std::collections::HashMap<TypeId, Rc<dyn Any>> {
        let mut features = std::collections::HashMap::new();
        let kind = TypeId::of::<dyn IExternalObjectsRenderInterfaceContextFeature>();
        if let Some(external) = self.try_get_feature(kind) {
            features.insert(kind, external);
        }
        features
    }
    pub fn is_ready(&self, surfaces: &[Arc<dyn IPlatformRenderSurface>]) -> bool {
        surfaces
            .iter()
            .any(|s| s.is_ready() && try_get_metal_surface(&**s).is_some())
    }
    /// A render target for the first Metal surface, or `None` when there is none.
    pub fn create_render_target(
        &self,
        surfaces: &[Arc<dyn IPlatformRenderSurface>],
    ) -> Option<Rc<dyn IRenderTarget>> {
        let surface = surfaces.iter().find_map(|s| try_get_metal_surface(&**s))?;
        Some(Rc::new(MetalTarget {
            target: surface.create_metal_render_target(self.device.clone()),
            device: self.device.clone(),
            frame: RefCell::new(None),
        }))
    }
    pub fn is_lost(&self) -> bool {
        self.device.is_lost()
    }
}
struct MetalTarget {
    target: Rc<dyn IMetalPlatformSurfaceRenderTarget>,
    device: Rc<dyn IMetalDevice>,
    frame: RefCell<Option<Bitmap>>,
}
impl MetalTarget {
    /// The retained frame for this session, recreated on a size change. Fails for a size the
    /// device cannot hold or when GPU allocation fails.
    fn frame(&self, size: PixelSize, scaling: f64) -> Result<(Bitmap, bool), String> {
        let mut frame = self.frame.borrow_mut();
        if let Some(existing) = frame.as_ref().filter(|f| f.pixel_size() == size) {
            return Ok((existing.clone(), true));
        }
        frame.take();
        let bitmap = Bitmap::try_new_gpu(size, Vector::new(96., 96.) * scaling)
            .map_err(|error| error.to_string())?;
        *frame = Some(bitmap.clone());
        Ok((bitmap, false))
    }
    fn drawing_context(
        &self,
    ) -> Result<
        (
            Box<dyn IDrawingContextImpl>,
            RenderTargetDrawingContextProperties,
        ),
        String,
    > {
        let session = self.target.begin_rendering();
        let (bitmap, retained) = match self.frame(session.size(), session.scaling()) {
            Ok(frame) => frame,
            Err(error) => {
                session.dispose();
                return Err(error);
            }
        };
        let output = bitmap.clone();
        let device = self.device.clone();
        let done = Box::new(move || {
            let status = output.context(|p| unsafe {
                ffi::qm_present(
                    p,
                    session.texture(),
                    device.command_queue(),
                    session.is_y_flipped() as i32,
                )
            });
            if status != ffi::STATUS_OK {
                warn_once("a Metal frame could not be presented");
            }
            session.dispose();
        });
        Ok((
            Box::new(bitmap.drawing(false, Some(done))),
            RenderTargetDrawingContextProperties {
                previous_frame_is_retained: retained,
            },
        ))
    }
}
impl IRenderTarget for MetalTarget {
    fn properties(&self) -> RenderTargetProperties {
        RenderTargetProperties {
            retains_previous_frame_contents: true,
            is_suitable_for_direct_rendering: true,
        }
    }
    fn create_drawing_context(
        &self,
        _: &RenderTargetSceneInfo,
    ) -> (
        Box<dyn IDrawingContextImpl>,
        RenderTargetDrawingContextProperties,
    ) {
        self.drawing_context().unwrap_or_else(|_| {
            warn_once("a Metal frame could not be allocated; the frame is skipped");
            let bitmap = Bitmap::new(PixelSize::new(1, 1), Vector::new(96., 96.));
            (
                Box::new(bitmap.drawing(false, None)),
                RenderTargetDrawingContextProperties::default(),
            )
        })
    }
    fn try_create_drawing_context(
        &self,
        _: &RenderTargetSceneInfo,
    ) -> Result<
        (
            Box<dyn IDrawingContextImpl>,
            RenderTargetDrawingContextProperties,
        ),
        RenderTargetError,
    > {
        self.drawing_context().map_err(|error| {
            RenderTargetError::Corrupted(RenderTargetCorruptedError::new_with_message(format!(
                "Quartz Metal frame: {error}"
            )))
        })
    }
    fn platform_render_target_state(&self) -> PlatformRenderTargetState {
        self.target.state()
    }
    fn dispose(&self) {
        self.frame.borrow_mut().take();
        self.target.dispose();
    }
}
/// Cumulative GPU counters: surfaces, path dispatches, image dispatches, presentations,
/// explicit readbacks, image uploads, asynchronous command-buffer errors.
pub fn statistics() -> [u64; 7] {
    let mut result = [0; 7];
    unsafe { ffi::qm_statistics(result.as_mut_ptr(), result.len()) };
    result
}

/// Submitted Metal command buffers, including explicit diagnostic fences.
pub fn submission_count() -> u64 {
    let mut values = [0; 8];
    unsafe { ffi::qm_statistics(values.as_mut_ptr(), values.len()) };
    values[7]
}
