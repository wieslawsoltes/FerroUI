//! Direct Metal presentation. Drawing surfaces and retained layers stay on the GPU.
use crate::{bitmap::Bitmap, ffi};
use ferroui_base::{
    platform::{surfaces::*, *},
    PixelSize, Vector,
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
    external: Option<Rc<crate::external_objects::ExternalObjects>>,
}
impl MetalContext {
    pub fn new(graphics: Option<Rc<dyn IPlatformGraphicsContext>>) -> Self {
        let graphics = graphics.expect("Quartz Metal requires a platform Metal graphics context");
        let device = graphics
            .try_get_feature(TypeId::of::<dyn IMetalDevice>())
            .and_then(|f| f.downcast_ref::<Rc<dyn IMetalDevice>>().cloned())
            .expect("Quartz Metal requires an IMetalDevice; select macOS Metal rendering");
        unsafe { ffi::qm_configure(device.device()) };
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
        Self { device, external }
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
        let surface = surfaces
            .iter()
            .find_map(|s| try_get_metal_surface(&**s))
            .expect("Quartz Metal requires a Metal surface");
        Rc::new(MetalTarget {
            target: surface.create_metal_render_target(self.device.clone()),
            device: self.device.clone(),
            frame: RefCell::new(None),
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
        self.device.is_lost()
    }
    fn max_offscreen_render_target_pixel_size(&self) -> Option<PixelSize> {
        Some(PixelSize::new(16384, 16384))
    }
    fn dispose(&self) {}
}
struct MetalTarget {
    target: Rc<dyn IMetalPlatformSurfaceRenderTarget>,
    device: Rc<dyn IMetalDevice>,
    frame: RefCell<Option<Bitmap>>,
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
        let session = self.target.begin_rendering();
        let mut frame = self.frame.borrow_mut();
        let retained = frame
            .as_ref()
            .is_some_and(|f| f.pixel_size() == session.size());
        if !retained {
            *frame = Some(Bitmap::new(
                session.size(),
                Vector::new(96., 96.) * session.scaling(),
            ));
        }
        let bitmap = frame.as_ref().unwrap().clone();
        let output = bitmap.clone();
        let device = self.device.clone();
        let done = Box::new(move || {
            output.context(|p| unsafe {
                ffi::qm_present(
                    p,
                    session.texture(),
                    device.command_queue(),
                    session.is_y_flipped() as i32,
                )
            });
            session.dispose();
        });
        (
            Box::new(bitmap.drawing(false, Some(done))),
            RenderTargetDrawingContextProperties {
                previous_frame_is_retained: retained,
            },
        )
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
