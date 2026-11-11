use crate::{
    bitmap::Bitmap,
    ffi::{self, Owned},
};
use ferroui_base::{
    platform::{surfaces::IPlatformRenderSurface, *},
    PixelSize, Vector,
};
use ferroui_controls::platform::INativePlatformHandleSurface;
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    rc::Rc,
    sync::Arc,
};

pub struct Context {
    generation: u64,
}
impl Context {
    pub fn new() -> Self {
        Self {
            generation: crate::device_generation(),
        }
    }
}
impl IOptionalFeatureProvider for Context {
    fn try_get_feature(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        if kind == TypeId::of::<dyn IExternalObjectsRenderInterfaceContextFeature>() {
            let feature: Rc<dyn IExternalObjectsRenderInterfaceContextFeature> =
                Rc::new(crate::external_objects::ExternalObjects {
                    generation: self.generation,
                });
            Some(Rc::new(feature))
        } else {
            None
        }
    }
}
fn handle(surface: &dyn IPlatformRenderSurface) -> Option<Rc<dyn INativePlatformHandleSurface>> {
    surface
        .try_get_surface_kind(TypeId::of::<dyn INativePlatformHandleSurface>())?
        .downcast_ref::<Rc<dyn INativePlatformHandleSurface>>()
        .cloned()
}
impl IPlatformRenderInterfaceContext for Context {
    fn public_features(&self) -> std::collections::HashMap<TypeId, Rc<dyn Any>> {
        let kind = TypeId::of::<dyn IExternalObjectsRenderInterfaceContextFeature>();
        std::collections::HashMap::from([(kind, self.try_get_feature(kind).unwrap())])
    }
    fn create_render_target(
        &self,
        surfaces: &[Arc<dyn IPlatformRenderSurface>],
    ) -> Rc<dyn IRenderTarget> {
        let surface = surfaces
            .iter()
            .find_map(|s| handle(&**s))
            .expect("Direct2D requires a Windows HWND surface");
        let window = unsafe { Owned::new(ffi::d2_window(surface.handle())) };
        Rc::new(Target {
            window: Rc::new(window),
            bitmap: RefCell::new(None),
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
        let current = crate::device_generation();
        current == 0 || current != self.generation
    }
    fn max_offscreen_render_target_pixel_size(&self) -> Option<PixelSize> {
        Some(PixelSize::new(16384, 16384))
    }
    fn is_ready_to_create_render_target(
        &self,
        surfaces: &[Arc<dyn IPlatformRenderSurface>],
    ) -> bool {
        surfaces.iter().any(|s| {
            s.is_ready() && handle(&**s).is_some_and(|s| s.size().width > 0 && s.size().height > 0)
        })
    }
    fn dispose(&self) {}
}
struct Target {
    window: Rc<Owned>,
    bitmap: RefCell<Option<Bitmap>>,
}
pub(crate) fn compositor_drawing(
    bitmap: &Bitmap,
    finished: Option<Box<dyn FnOnce()>>,
) -> Box<dyn IDrawingContextImpl> {
    // The compositor supplies a transform in device pixels, including DPI.
    // RenderTargetBitmap's immediate path applies its own logical-to-pixel scale.
    Box::new(bitmap.drawing(false, finished))
}
impl IRenderTarget for Target {
    fn properties(&self) -> RenderTargetProperties {
        RenderTargetProperties {
            retains_previous_frame_contents: true,
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
        unsafe {ffi::d2_window_frame_begin(self.window.0);}
        let mut stored = self.bitmap.borrow_mut();
        let retained = stored.as_ref().is_some_and(|b| {
            b.pixel_size() == scene.size
                && b.dpi() == Vector::new(scene.scaling * 96., scene.scaling * 96.)
        });
        if !retained {
            *stored = Some(Bitmap::new(
                scene.size,
                Vector::new(scene.scaling * 96., scene.scaling * 96.),
            ));
        }
        let bitmap = stored.as_ref().unwrap().clone();
        let output = bitmap.clone();
        let window = self.window.clone();
        let done = Box::new(move || {
            output.context(|p| unsafe {
                ffi::d2_present(window.0, p);
            })
        });
        (
            compositor_drawing(&bitmap, Some(done)),
            RenderTargetDrawingContextProperties {
                previous_frame_is_retained: retained,
            },
        )
    }
    fn platform_render_target_state(&self) -> PlatformRenderTargetState {
        match unsafe { ffi::d2_window_state(self.window.0) } {
            -1 => PlatformRenderTargetState::CORRUPTED,
            0 => PlatformRenderTargetState::NOT_READY_TRY_LATER,
            _ => PlatformRenderTargetState::READY,
        }
    }
    fn dispose(&self) {
        self.bitmap.borrow_mut().take();
        unsafe {ffi::d2_window_dispose(self.window.0);}
    }
}
