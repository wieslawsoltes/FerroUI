//! The full-frame opt-in must preserve primitive capture without retaining
//! pixels or changing existing target and debug-overlay behavior.

use super::*;
use crate::platform::surfaces::IPlatformRenderSurface;
use crate::platform::{
    IDrawingContextImpl, IDrawingContextLayerImpl, IOptionalFeatureProvider,
    IPlatformGraphicsContext, IPlatformRenderInterfaceContext, IRenderTarget,
    RenderTargetDrawingContextProperties, RenderTargetProperties, RenderTargetSceneInfo,
};
use crate::rendering::testing::{DrawingLog, MockDrawingContextLayerImpl, MockRenderTarget};
use crate::rendering::RendererDebugOverlays;
use std::any::{Any, TypeId};

struct CaptureContext {
    log: DrawingLog,
    full_frame: bool,
    suitable: bool,
    retained: bool,
}

impl IOptionalFeatureProvider for CaptureContext {
    fn try_get_feature(&self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
        None
    }
}

impl IPlatformRenderInterfaceContext for CaptureContext {
    fn create_render_target(
        &self,
        _surfaces: &[Arc<dyn IPlatformRenderSurface>],
    ) -> Rc<dyn IRenderTarget> {
        Rc::new(CaptureTarget {
            inner: MockRenderTarget::new(self.log.clone()),
            full_frame: self.full_frame,
            suitable: self.suitable,
            retained: self.retained,
        })
    }

    fn create_offscreen_render_target(
        &self,
        pixel_size: PixelSize,
        _scaling: Vector,
        _enable_text_antialiasing: bool,
    ) -> Rc<dyn IDrawingContextLayerImpl> {
        Rc::new(MockDrawingContextLayerImpl::new(
            self.log.clone(),
            pixel_size,
        ))
    }

    fn is_lost(&self) -> bool {
        false
    }
    fn max_offscreen_render_target_pixel_size(&self) -> Option<PixelSize> {
        None
    }
    fn dispose(&self) {}
}

struct CaptureTarget {
    inner: Rc<MockRenderTarget>,
    full_frame: bool,
    suitable: bool,
    retained: bool,
}

impl IRenderTarget for CaptureTarget {
    fn properties(&self) -> RenderTargetProperties {
        RenderTargetProperties {
            retains_previous_frame_contents: self.retained,
            is_suitable_for_direct_rendering: self.suitable,
        }
    }

    fn prefers_full_frame_rendering(&self) -> bool {
        self.full_frame
    }

    fn create_drawing_context(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> (
        Box<dyn IDrawingContextImpl>,
        RenderTargetDrawingContextProperties,
    ) {
        let (context, _) = self.inner.create_drawing_context(scene_info);
        (
            context,
            RenderTargetDrawingContextProperties {
                previous_frame_is_retained: self.retained,
            },
        )
    }

    fn dispose(&self) {
        self.inner.dispose();
    }
}

fn capture_canvas(full_frame: bool, suitable: bool, retained: bool) -> CompositorCanvas {
    CompositorCanvas::with_size_and_setup(Size::new(100.0, 100.0), move |render_interface| {
        render_interface.set_backend_context_factory(
            move |log: &DrawingLog, _graphics: Option<Rc<dyn IPlatformGraphicsContext>>| {
                Rc::new(CaptureContext {
                    log: log.clone(),
                    full_frame,
                    suitable,
                    retained,
                }) as Rc<dyn IPlatformRenderInterfaceContext>
            },
        );
    })
}

#[test]
fn full_frame_target_receives_vectors_and_redraws_unchanged_siblings() {
    // An opted-in retaining target must receive the same complete command
    // stream as a fresh recording, regardless of its previous pixels.
    for retained in [false, true] {
        assert_full_frame_target_redraws(retained);
    }
}

fn assert_full_frame_target_redraws(retained: bool) {
    let scene = capture_canvas(true, true, retained);
    let changed = TestBorder::layer(4.0, 4.0, 12.0, 12.0);
    let unchanged = TestBorder::layer(80.0, 80.0, 12.0, 12.0);
    scene.canvas.add(&changed);
    scene.canvas.add(&unchanged);
    scene.run_jobs();
    let log = scene.render_interface.log();
    assert_eq!(0, log.count("CreateLayer"), "{:?}", log.entries());
    assert_eq!(0, log.count("Blit"), "{:?}", log.entries());
    log.clear();
    changed.set_background(Some(Brushes::blue()));
    scene.run_jobs();
    assert_eq!(
        1,
        log.count("DrawRectangle Blue"),
        "{:?}",
        log.entries()
    );
    assert_eq!(
        1,
        log.count("DrawRectangle Red"),
        "a complete frame needs its unchanged sibling too (retained={retained}): {:?}",
        log.entries()
    );
    assert_eq!(0, log.count("CreateLayer"), "{:?}", log.entries());
    assert_eq!(0, log.count("Blit"), "{:?}", log.entries());
}

#[test]
fn full_frame_opt_in_preserves_layers_for_existing_or_unsuitable_targets() {
    for (full_frame, suitable) in [(false, true), (true, false)] {
        let scene = capture_canvas(full_frame, suitable, false);
        let child = TestBorder::filled(4.0, 4.0, 12.0, 12.0);
        scene.canvas.add(&child);
        scene.run_jobs();
        let log = scene.render_interface.log();
        assert!(log.count("CreateLayer") > 0, "{:?}", log.entries());
        assert!(log.count("Blit") > 0, "{:?}", log.entries());
    }
}

#[test]
fn full_frame_target_keeps_the_layer_required_by_dirty_rect_overlays() {
    let scene = capture_canvas(true, true, false);
    let log = scene.render_interface.log();
    log.clear();
    scene
        .renderer
        .composition_target()
        .set_debug_overlays(RendererDebugOverlays::DIRTY_RECTS);
    scene.canvas.add(&TestBorder::filled(4.0, 4.0, 12.0, 12.0));
    scene.run_jobs();
    assert!(log.count("CreateLayer") > 0, "{:?}", log.entries());
    assert!(log.count("Blit") > 0, "{:?}", log.entries());
}
