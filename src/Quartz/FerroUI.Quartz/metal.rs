//! Direct Metal presentation. Drawing surfaces and retained layers stay on the GPU.
use crate::{bitmap::Bitmap, ffi};
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

pub struct MetalContext {
    device: Rc<dyn IMetalDevice>,
    generation: u64,
    external: Option<Rc<crate::external_objects::ExternalObjects>>,
}
impl MetalContext {
    pub fn new(graphics: Option<Rc<dyn IPlatformGraphicsContext>>) -> Result<Self, String> {
        let graphics = graphics.ok_or("No Metal graphics context is available")?;
        let device = graphics
            .try_get_feature(TypeId::of::<dyn IMetalDevice>())
            .and_then(|f| f.downcast_ref::<Rc<dyn IMetalDevice>>().cloned())
            .ok_or("The graphics context does not expose a Metal device")?;
        let generation = unsafe { ffi::qm_configure(device.device()) };
        if generation == 0 || unsafe { ffi::qm_attach_queue(device.command_queue()) == 0 } {
            return Err("Quartz could not initialize the Metal device".into());
        }
        static GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let external_generation = GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let external = device
            .try_get_feature(TypeId::of::<dyn ferroui_metal::IMetalExternalObjectsFeature>())
            .and_then(|f| {
                f.downcast_ref::<Rc<dyn ferroui_metal::IMetalExternalObjectsFeature>>()
                    .cloned()
            })
            .map(|inner| {
                Rc::new(crate::external_objects::ExternalObjects::new(
                    device.clone(),
                    external_generation,
                    generation,
                    inner,
                ))
            });
        Ok(Self {
            device,
            generation,
            external,
        })
    }
}
impl IOptionalFeatureProvider for MetalContext {
    fn try_get_feature(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
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
}
impl IPlatformRenderInterfaceContext for MetalContext {
    fn public_features(&self) -> std::collections::HashMap<TypeId, Rc<dyn Any>> {
        let mut features = std::collections::HashMap::new();
        let kind = TypeId::of::<dyn IExternalObjectsRenderInterfaceContextFeature>();
        if let Some(external) = self.try_get_feature(kind) {
            features.insert(kind, external);
        }
        features
    }

    fn is_ready_to_create_render_target(
        &self,
        surfaces: &[Arc<dyn IPlatformRenderSurface>],
    ) -> bool {
        surfaces
            .iter()
            .any(|s| s.is_ready() && try_get_metal_surface(&**s).is_some())
    }
    fn create_render_target(
        &self,
        surfaces: &[Arc<dyn IPlatformRenderSurface>],
    ) -> Rc<dyn IRenderTarget> {
        let Some(surface) = surfaces.iter().find_map(|s| try_get_metal_surface(&**s)) else {
            return Rc::new(crate::UnavailableTarget);
        };
        Rc::new(MetalTarget {
            target: surface.create_metal_render_target(self.device.clone()),
            device: self.device.clone(),
            frame: RefCell::new(None),
            generation: self.generation,
        })
    }
    fn create_offscreen_render_target(
        &self,
        size: PixelSize,
        scaling: Vector,
        _: bool,
    ) -> Rc<dyn IDrawingContextLayerImpl> {
        Rc::new(Bitmap::new(size, scaling * 96.))
    }
    fn is_lost(&self) -> bool {
        self.device.is_lost() || unsafe { ffi::qm_is_lost(self.generation) != 0 }
    }
    fn max_offscreen_render_target_pixel_size(&self) -> Option<PixelSize> {
        Some(PixelSize::new(16384, 16384))
    }
    fn dispose(&self) {
        unsafe { ffi::qm_finish_context(self.generation) }
    }
}
struct MetalTarget {
    target: Rc<dyn IMetalPlatformSurfaceRenderTarget>,
    device: Rc<dyn IMetalDevice>,
    frame: RefCell<Option<Bitmap>>,
    generation: u64,
}
impl MetalTarget {
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
        // Platform drawables are not retained between frames, matching the
        // Skia Metal target contract. Unsupported formats keep a private frame.
        let direct = if !session.is_y_flipped() {
            unsafe { ffi::qm_surface_external(session.texture()) }
        } else {
            std::ptr::null_mut()
        };
        let mut frame = self.frame.borrow_mut();
        if !direct.is_null() {
            *frame = Some(Bitmap::from_context(
                session.size(),
                Vector::new(96., 96.) * session.scaling(),
                unsafe { ffi::Owned::new(direct) },
            ));
        } else {
            let bitmap =
                match Bitmap::try_new(session.size(), Vector::new(96., 96.) * session.scaling()) {
                    Ok(bitmap) => bitmap,
                    Err(error) => {
                        session.dispose();
                        return Err(error.to_string());
                    }
                };
            *frame = Some(bitmap);
        }
        let bitmap = frame.as_ref().unwrap().clone();
        let output = bitmap.clone();
        let device = self.device.clone();
        let done = Box::new(move || {
            let presented = output.context(|p| unsafe {
                ffi::qm_present(
                    p,
                    session.texture(),
                    device.command_queue(),
                    session.is_y_flipped() as i32,
                )
            });
            if presented == 0 {
                eprintln!("Quartz: Metal frame presentation failed");
            }
            session.dispose();
        });
        Ok((
            Box::new(bitmap.drawing(false, Some(done))) as Box<dyn IDrawingContextImpl>,
            RenderTargetDrawingContextProperties {
                previous_frame_is_retained: false,
            },
        ))
    }
}
impl IRenderTarget for MetalTarget {
    fn properties(&self) -> RenderTargetProperties {
        RenderTargetProperties {
            retains_previous_frame_contents: false,
            is_suitable_for_direct_rendering: true,
        }
    }
    fn create_drawing_context(
        &self,
        scene: &RenderTargetSceneInfo,
    ) -> (
        Box<dyn IDrawingContextImpl>,
        RenderTargetDrawingContextProperties,
    ) {
        self.drawing_context()
            .unwrap_or_else(|_| crate::UnavailableTarget.create_drawing_context(scene))
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
        self.drawing_context().map_err(|message| {
            RenderTargetError::Corrupted(RenderTargetCorruptedError::new_with_message(message))
        })
    }
    fn platform_render_target_state(&self) -> PlatformRenderTargetState {
        if unsafe { ffi::qm_is_lost(self.generation) != 0 } {
            PlatformRenderTargetState::CORRUPTED
        } else {
            self.target.state()
        }
    }
    fn dispose(&self) {
        self.frame.borrow_mut().take();
        self.target.dispose();
    }
}
/// Cumulative GPU counters: surfaces, paint operations, image operations, presentations,
/// counted before draw batching (not physical encoder/dispatch counts),
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
