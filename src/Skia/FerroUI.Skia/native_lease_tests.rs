//! Native API features may be cached by custom drawing operations. Acquiring a
//! lease must use live context state and cannot resurrect a completed recording.

use crate::drawing_context_impl::CreateInfo;
use crate::gpu::{
    ISkiaGpu, ISkiaGpuRenderSession, ISkiaGpuRenderTarget, ISkiaSurface, ScopedGrContext,
};
use crate::skia_sharp_extensions::to_sk_matrix44;
use crate::{DrawingContextImpl, ISkiaApiLeaseFeature, PictureRenderTarget, SkiaOptions};
use ferroui_base::media::imaging::BitmapBlendingMode;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    IDrawingContextImpl, IOptionalFeatureProvider, IPlatformGraphicsContext,
};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{FerroLocator, Matrix, PixelSize, Rect, Size, Vector};
use std::any::{Any, TypeId};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;
use std::sync::Arc;

fn feature(context: &mut DrawingContextImpl) -> Rc<dyn ISkiaApiLeaseFeature> {
    context
        .get_feature(TypeId::of::<dyn ISkiaApiLeaseFeature>())
        .expect("native API feature")
        .downcast::<Rc<dyn ISkiaApiLeaseFeature>>()
        .ok()
        .expect("native API feature type")
        .as_ref()
        .clone()
}

fn contexts() -> [DrawingContextImpl; 2] {
    let dpi = Vector::new(96.0, 96.0);
    let surface = skia_safe::surfaces::raster_n32_premul((32, 32)).unwrap();
    [
        DrawingContextImpl::new(
            CreateInfo {
                surface: Some(surface),
                dpi,
                ..CreateInfo::default()
            },
            Vec::new(),
        ),
        PictureRenderTarget::new(None, None, dpi)
            .create_export_drawing_context(Size::new(32.0, 32.0), false),
    ]
}

#[test]
fn retained_recorded_feature_uses_live_transform_opacity_dpi_and_restore() {
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable().bind_to_self(Rc::new(SkiaOptions {
        use_opacity_save_layer: false,
        ..SkiaOptions::default()
    }));
    let target = PictureRenderTarget::new(None, None, Vector::new(192.0, 144.0));
    let mut context = target.create_export_drawing_context(Size::new(100.0, 100.0), true);
    let feature = feature(&mut context);
    let transform = Matrix::create_translation(10.0, 20.0);
    let device_transform = to_sk_matrix44(transform * Matrix::create_scale(2.0, 1.5));

    context.set_transform(transform);
    context.push_opacity(0.5, None);
    let lease = feature.lease();
    assert_eq!(lease.current_opacity(), 0.5);
    lease.with_sk_canvas(&mut |canvas| {
        assert_eq!(canvas.local_to_device(), device_transform);
        let mut paint = skia_safe::Paint::default();
        paint.set_color(skia_safe::Color::from_argb(128, 255, 0, 0));
        canvas.draw_rect(skia_safe::Rect::from_wh(10.0, 10.0), &paint);
    });
    lease.dispose();

    context.push_clip(Rect::new(0.0, 0.0, 40.0, 40.0));
    context.set_transform(Matrix::create_translation(40.0, 40.0));
    context.pop_clip();
    context.pop_opacity();
    let lease = feature.lease();
    assert_eq!(lease.current_opacity(), 1.0);
    lease.with_sk_canvas(&mut |canvas| {
        assert_eq!(canvas.local_to_device(), device_transform);
        let mut paint = skia_safe::Paint::default();
        paint.set_color(skia_safe::Color::BLUE);
        canvas.draw_rect(skia_safe::Rect::from_xywh(15.0, 0.0, 10.0, 10.0), &paint);
    });
    lease.dispose();
    context.dispose();

    let mut surface = skia_safe::surfaces::raster_n32_premul((200, 150)).unwrap();
    surface.canvas().clear(skia_safe::Color::TRANSPARENT);
    surface
        .canvas()
        .draw_picture(&target.get_picture(), None, None);
    let pixels = surface.peek_pixels().unwrap();
    assert_eq!(
        pixels.get_color((25, 35)),
        skia_safe::Color::from_argb(128, 255, 0, 0)
    );
    assert_eq!(pixels.get_color((55, 35)), skia_safe::Color::BLUE);
    assert_eq!(pixels.get_color((5, 5)), skia_safe::Color::TRANSPARENT);
    scope.dispose();
}

#[test]
fn retained_features_observe_compositing_opacity_transitions() {
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable().bind_to_self(Rc::new(SkiaOptions {
        use_opacity_save_layer: false,
        ..SkiaOptions::default()
    }));
    for mut context in contexts() {
        let feature = feature(&mut context);
        context.push_opacity(0.25, None);
        let lease = feature.lease();
        assert_eq!(lease.current_opacity(), 0.25);
        lease.dispose();
        assert!(context.try_push_compositing_layer(
            BitmapBlendingMode::SourceOver,
            0.8,
            Rect::new(0.0, 0.0, 32.0, 32.0),
        ));
        let lease = feature.lease();
        assert_eq!(lease.current_opacity(), 1.0);
        lease.dispose();
        context.pop_compositing_layer();
        let lease = feature.lease();
        assert_eq!(lease.current_opacity(), 0.25);
        lease.dispose();
        context.pop_opacity();
        let lease = feature.lease();
        assert_eq!(lease.current_opacity(), 1.0);
        lease.dispose();
        context.dispose();
    }
    scope.dispose();
}

#[test]
fn native_features_cannot_lease_disposed_or_dropped_contexts() {
    for mut context in contexts() {
        let feature = feature(&mut context);
        context.dispose();
        assert!(catch_unwind(AssertUnwindSafe(|| feature.lease())).is_err());
    }
    for mut context in contexts() {
        let feature = feature(&mut context);
        drop(context);
        assert!(catch_unwind(AssertUnwindSafe(|| feature.lease())).is_err());
    }
}

#[test]
fn disposed_native_leases_cannot_draw_or_release_a_new_lease() {
    for mut context in contexts() {
        let feature = feature(&mut context);
        let old = feature.lease();
        old.dispose();
        let current = feature.lease();
        old.dispose();
        assert!(catch_unwind(AssertUnwindSafe(|| feature.lease())).is_err());
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                old.with_sk_canvas(&mut |_| {});
            }))
            .is_err()
        );
        assert!(catch_unwind(AssertUnwindSafe(|| old.current_opacity())).is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| old.gr_context())).is_err());
        current.with_sk_canvas(&mut |_| {});
        current.dispose();
        context.dispose();
    }
}

#[test]
fn live_native_leases_become_invalid_when_context_is_dropped() {
    for mut context in contexts() {
        let feature = feature(&mut context);
        let lease = feature.lease();
        drop(context);
        assert!(catch_unwind(AssertUnwindSafe(|| lease.current_opacity())).is_err());
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                lease.with_sk_canvas(&mut |_| {});
            }))
            .is_err()
        );
        // Cleanup remains idempotent even when ownership has already ended.
        lease.dispose();
        lease.dispose();
    }
}

#[test]
fn disposing_an_orphaned_surface_lease_cannot_change_a_later_frame() {
    let surface = skia_safe::surfaces::raster_n32_premul((32, 32)).unwrap();
    let make_context = || {
        DrawingContextImpl::new(
            CreateInfo {
                surface: Some(surface.clone()),
                dpi: Vector::new(96.0, 96.0),
                ..CreateInfo::default()
            },
            Vec::new(),
        )
    };
    let mut first = make_context();
    let lease = feature(&mut first).lease();
    drop(first);
    let mut second = make_context();
    let transform = Matrix::create_translation(17.0, 23.0);
    second.set_transform(transform);
    assert_eq!(second.canvas().local_to_device(), to_sk_matrix44(transform));
    lease.dispose();
    assert_eq!(second.canvas().local_to_device(), to_sk_matrix44(transform));
    second.dispose();
}

/// No native GPU is needed to exercise nesting and exclusive ownership. The
/// mock only advertises a platform context; every drawing target stays raster.
struct MockGpu;

impl IOptionalFeatureProvider for MockGpu {
    fn try_get_feature(&self, _: TypeId) -> Option<Rc<dyn Any>> {
        None
    }
}

impl IPlatformGraphicsContext for MockGpu {
    fn is_lost(&self) -> bool {
        false
    }
    fn ensure_current(&self) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
    fn dispose(&self) {}
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ISkiaGpu for MockGpu {
    fn platform_graphics_context(&self) -> Option<Rc<dyn IPlatformGraphicsContext>> {
        Some(Rc::new(Self))
    }
    fn try_create_render_target(
        &self,
        _: &[Arc<dyn IPlatformRenderSurface>],
    ) -> Option<Rc<dyn ISkiaGpuRenderTarget>> {
        None
    }
    fn is_ready_to_create_render_target(&self, _: &[Arc<dyn IPlatformRenderSurface>]) -> bool {
        false
    }
    fn try_create_surface(
        &self,
        _: PixelSize,
        _: Option<&Rc<dyn ISkiaGpuRenderSession>>,
        _: &skia_safe::SurfaceProps,
    ) -> Option<Rc<dyn ISkiaSurface>> {
        None
    }
    fn try_get_gr_context(&self) -> Option<ScopedGrContext> {
        None
    }
}

#[test]
fn nested_platform_leases_cannot_release_their_parent_or_a_new_lease() {
    let gpu: Rc<dyn ISkiaGpu> = Rc::new(MockGpu);
    let dpi = Vector::new(96.0, 96.0);
    let contexts = [
        DrawingContextImpl::new(
            CreateInfo {
                surface: Some(skia_safe::surfaces::raster_n32_premul((32, 32)).unwrap()),
                dpi,
                gpu: Some(gpu.clone()),
                ..CreateInfo::default()
            },
            Vec::new(),
        ),
        PictureRenderTarget::new(Some(gpu), None, dpi)
            .create_export_drawing_context(Size::new(32.0, 32.0), false),
    ];
    for mut context in contexts {
        let feature = feature(&mut context);
        let lease = feature.lease();
        let old = lease.try_lease_platform_graphics_api().unwrap();
        assert!(catch_unwind(AssertUnwindSafe(|| lease.dispose())).is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| feature.lease())).is_err());
        old.dispose();
        let current = lease.try_lease_platform_graphics_api().unwrap();
        old.dispose();
        assert!(catch_unwind(AssertUnwindSafe(|| lease.with_sk_canvas(&mut |_| {}))).is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| old.context())).is_err());
        current.dispose();
        lease.with_sk_canvas(&mut |_| {});
        lease.dispose();
        context.dispose();
    }
}
