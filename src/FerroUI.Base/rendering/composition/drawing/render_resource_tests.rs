//! Tests of the mutable media objects as composition render resources: the
//! render-resource tests of the brush, pen, geometry and transform classes,
//! and end-to-end tests through recorded render data.

use super::*;
use crate::media::imaging::Bitmap;
use crate::media::immutable::{ImmutableDashStyle, ImmutableSolidColorBrush, ImmutableTransform};
use crate::media::{
    AlignmentX, AlignmentY, Brush, Brushes, Color, Colors, CombinedGeometry, ConicGradientBrush, DashStyle,
    DrawingBrush, DrawingContext, EllipseGeometry, Geometry, GeometryCombineMode, GeometryDrawing, GeometryGroup,
    GradientBrush, GradientSpreadMethod, GradientStop, GradientStops, IBrush, IImageBrushSource, IPen,
    ISceneBrushContent, ITransform, ImageBrush, LinearGradientBrush, MediaCollection, MediaContext, Pen, PenLineCap,
    PenLineJoin, RadialGradientBrush, RectangleGeometry, RotateTransform, SolidColorBrush, Stretch, TileMode,
    Transform, TranslateTransform, VisualBrush,
};
use crate::rendering::composition::server::{
    IServerObject, IServerRenderResource, IServerRenderResourceObserver, ServerCompositionSimpleConicGradientBrush,
    ServerCompositionSimpleContentBrush, ServerCompositionSimpleGeometry, ServerCompositionSimpleImageBrush,
    ServerCompositionSimpleLinearGradientBrush, ServerCompositionSimpleRadialGradientBrush,
    ServerCompositionSimpleSolidColorBrush, ServerCompositionSimpleTransform, ServerObjectId,
};
use crate::rendering::composition::test_compositor::TestCompositor;
use crate::rendering::composition::{CompositionBrush, Compositor, ICompositorSerializable};
use crate::rendering::testing::{
    DrawingLog, ManualRenderLoop, MockDrawingContextImpl, MockDrawingContextLayerImpl, MockPlatformRenderInterface,
};
use crate::threading::Dispatcher;
use crate::{Matrix, PixelSize, Point, Rect, Ref, RelativePoint, RelativeRect, RelativeScalar, RelativeUnit, Upcast};
use std::cell::Cell;
use std::rc::Rc;

/// A mutable media object viewed as the two interfaces the classes
/// implement upstream.
struct Resource {
    serializable: Rc<dyn ICompositorSerializable>,
    add_ref: Box<dyn Fn(&Rc<Compositor>)>,
    release: Box<dyn Fn(&Rc<Compositor>)>,
}

trait AsResource {
    fn resource(&self) -> Resource;
}

macro_rules! as_resource {
    ($($class:ty),*) => {$(
        impl<T: crate::ObjectType + Upcast<$class>> AsResource for (Ref<T>, std::marker::PhantomData<$class>) {
            fn resource(&self) -> Resource {
                let object: &$class = (*self.0).upcast();
                let (a, r) = (object.to_ref(), object.to_ref());
                Resource {
                    serializable: object.as_compositor_serializable(),
                    add_ref: Box::new(move |c| a.add_ref_on_compositor(c)),
                    release: Box::new(move |c| r.release_on_compositor(c)),
                }
            }
        }
    )*};
}

as_resource!(Brush, Pen, Geometry, Transform);

fn brush_resource<T: crate::ObjectType + Upcast<Brush>>(brush: &Ref<T>) -> Resource {
    (brush.clone(), std::marker::PhantomData::<Brush>).resource()
}

fn pen_resource(pen: &Ref<Pen>) -> Resource {
    (pen.clone(), std::marker::PhantomData::<Pen>).resource()
}

fn geometry_resource<T: crate::ObjectType + Upcast<Geometry>>(geometry: &Ref<T>) -> Resource {
    (geometry.clone(), std::marker::PhantomData::<Geometry>).resource()
}

fn transform_resource<T: crate::ObjectType + Upcast<Transform>>(transform: &Ref<T>) -> Resource {
    (transform.clone(), std::marker::PhantomData::<Transform>).resource()
}

/// The compositor test services and the render resource test helper of the
/// reference tests.
struct RenderResourceTestHelper {
    _dispatcher_scope: crate::threading::UnitTestDispatcherScope,
    locator_scope: Rc<dyn crate::reactive::IDisposable>,
    render_interface: Rc<MockPlatformRenderInterface>,
    render_loop: std::sync::Arc<ManualRenderLoop>,
    compositor: Rc<Compositor>,
}

impl RenderResourceTestHelper {
    fn new() -> RenderResourceTestHelper {
        let dispatcher_scope = Dispatcher::unit_test_scope();
        let (locator_scope, render_interface) = MockPlatformRenderInterface::install();
        let render_loop = ManualRenderLoop::new();
        let compositor = Compositor::with_scheduler(
            render_loop.clone(),
            None,
            false,
            &MediaContext::instance().scheduler(),
            Dispatcher::ui_thread(),
            None,
            None,
        );
        RenderResourceTestHelper {
            _dispatcher_scope: dispatcher_scope,
            locator_scope,
            render_interface,
            render_loop,
            compositor,
        }
    }

    /// Commits the pending changes and runs a frame of the server.
    fn run_jobs(&self) {
        self.compositor.commit();
        self.render_loop.tick();
    }

    fn add_to_compositor(&self, resource: &Resource) {
        (resource.add_ref)(&self.compositor);
    }

    fn is_invalidated(&self, resource: &Resource) -> bool {
        self.compositor.unit_test_is_registered_for_serialization(&*resource.serializable)
    }

    fn try_get_server(&self, resource: &Resource) -> Option<ServerObjectId> {
        resource.serializable.try_get_server(&self.compositor)
    }

    fn assert_exists_on_compositor(&self, resource: &Resource, exists: bool) {
        assert_eq!(self.try_get_server(resource).is_some(), exists);
    }

    fn assert_resource_invalidation(resource: Resource, cb: impl FnOnce()) {
        let helper = RenderResourceTestHelper::new();
        helper.assert_invalidation(&resource, cb);
    }

    fn assert_invalidation(&self, resource: &Resource, cb: impl FnOnce()) {
        (resource.add_ref)(&self.compositor);
        assert!(self.try_get_server(resource).is_some());
        assert!(self.is_invalidated(resource));

        self.run_jobs();

        assert!(!self.is_invalidated(resource));
        cb();
        assert!(self.is_invalidated(resource));
        (resource.release)(&self.compositor);
        assert!(self.try_get_server(resource).is_none());
    }

    fn server<T: IServerObject>(&self, resource: &Resource) -> Rc<T> {
        let id = self.try_get_server(resource).expect("the resource is on the compositor");
        self.compositor.server().get::<T>(id).expect("the server object exists")
    }

    fn record(&self, draw: impl FnOnce(&mut DrawingContext<'_>)) -> Rc<CompositionRenderData> {
        let mut recorder = RenderDataDrawingContext::new(Some(self.compositor.clone()));
        {
            let mut context = DrawingContext::new(&mut recorder);
            draw(&mut context);
        }
        recorder.get_render_results().expect("something was drawn")
    }

    fn replay(&self, render_data: &CompositionRenderData) -> Vec<String> {
        let server =
            self.compositor.server().get::<ServerCompositionRenderData>(render_data.server()).expect("render data");
        let log = DrawingLog::new();
        let mut context = MockDrawingContextImpl::new(log.clone());
        context.log_transforms = false;
        server.render(&mut context);
        log.entries()
    }
}

impl Drop for RenderResourceTestHelper {
    fn drop(&mut self) {
        self.locator_scope.dispose();
    }
}

// --- BrushTests / SolidColorBrushTests / LinearGradientBrushTests -----------------

#[test]
fn brush_changing_opacity_raises_invalidated() {
    let target = SolidColorBrush::new();
    RenderResourceTestHelper::assert_resource_invalidation(brush_resource(&target), || target.set_opacity(0.5));
}

#[test]
fn solid_color_brush_changing_color_raises_invalidated() {
    let target = SolidColorBrush::with_color(Colors::RED);
    RenderResourceTestHelper::assert_resource_invalidation(brush_resource(&target), || target.set_color(Colors::GREEN));
}

#[test]
fn linear_gradient_brush_changing_start_point_raises_invalidated() {
    let target = LinearGradientBrush::new();
    target.set_start_point(RelativePoint::default());
    RenderResourceTestHelper::assert_resource_invalidation(brush_resource(&target), || {
        target.set_start_point(RelativePoint::new(10.0, 10.0, RelativeUnit::Absolute))
    });
}

#[test]
fn linear_gradient_brush_changing_end_point_raises_invalidated() {
    let target = LinearGradientBrush::new();
    target.set_end_point(RelativePoint::default());
    RenderResourceTestHelper::assert_resource_invalidation(brush_resource(&target), || {
        target.set_end_point(RelativePoint::new(10.0, 10.0, RelativeUnit::Absolute))
    });
}

fn stops(color: Color, offset: f64) -> GradientStops {
    let stops = GradientStops::new();
    stops.add(GradientStop::with_color_and_offset(color, offset));
    stops
}

#[test]
fn linear_gradient_brush_changing_gradient_stops_raises_invalidated() {
    let target = LinearGradientBrush::new();
    target.set_gradient_stops(stops(Colors::RED, 0.0));
    RenderResourceTestHelper::assert_resource_invalidation(brush_resource(&target), || {
        target.set_gradient_stops(stops(Colors::GREEN, 0.0))
    });
}

#[test]
fn linear_gradient_brush_adding_gradient_stop_raises_invalidated() {
    let target = LinearGradientBrush::new();
    target.set_gradient_stops(stops(Colors::RED, 0.0));
    RenderResourceTestHelper::assert_resource_invalidation(brush_resource(&target), || {
        target.gradient_stops().add(GradientStop::with_color_and_offset(Colors::GREEN, 1.0))
    });
}

#[test]
fn linear_gradient_brush_changing_gradient_stop_offset_raises_invalidated() {
    let target = LinearGradientBrush::new();
    target.set_gradient_stops(stops(Colors::RED, 0.0));
    RenderResourceTestHelper::assert_resource_invalidation(brush_resource(&target), || {
        target.gradient_stops().get(0).set_offset(0.5)
    });
}

// --- PenTests ------------------------------------------------------------------------

#[test]
fn pen_changing_thickness_raises_invalidated() {
    let target = Pen::new();
    RenderResourceTestHelper::assert_resource_invalidation(pen_resource(&target), || target.set_thickness(18.0));
}

#[test]
fn pen_brush_is_added_to_the_same_compositor() {
    let brush = SolidColorBrush::with_color(Colors::RED);
    let target = Pen::new();
    target.set_brush(Some((&brush).into()));

    let helper = RenderResourceTestHelper::new();
    let brush_resource = brush_resource(&brush);
    helper.assert_exists_on_compositor(&brush_resource, false);
    helper.add_to_compositor(&pen_resource(&target));
    helper.assert_exists_on_compositor(&brush_resource, true);
    assert!(helper.is_invalidated(&brush_resource));
}

#[test]
fn pen_changing_dash_style_dashes_raises_invalidated() {
    let dashes = DashStyle::new();
    let target = Pen::new();
    target.set_dash_style(Some((&dashes).into()));
    RenderResourceTestHelper::assert_resource_invalidation(pen_resource(&target), || {
        dashes.set_dashes(Some(MediaCollection::from_items([0.1, 0.2])))
    });
}

#[test]
fn pen_adding_dash_style_dashes_raises_invalidated() {
    let dashes = DashStyle::new();
    let target = Pen::new();
    target.set_dash_style(Some((&dashes).into()));
    RenderResourceTestHelper::assert_resource_invalidation(pen_resource(&target), || {
        dashes.set_dashes(Some(MediaCollection::from_items([0.3])))
    });
}

#[test]
fn pen_adding_dash_style_dash_raises_invalidated() {
    let dashes = DashStyle::new();
    let target = Pen::new();
    target.set_dash_style(Some((&dashes).into()));
    dashes.set_dashes(Some(MediaCollection::from_items([0.3])));
    RenderResourceTestHelper::assert_resource_invalidation(pen_resource(&target), || {
        dashes.dashes().unwrap().add_range([1.0, 2.0])
    });
}

#[test]
fn pen_replacing_the_brush_moves_the_reference() {
    let helper = RenderResourceTestHelper::new();
    let (first, second) = (SolidColorBrush::with_color(Colors::RED), SolidColorBrush::with_color(Colors::BLUE));
    let target = Pen::new();
    target.set_brush(Some((&first).into()));
    let pen = pen_resource(&target);
    helper.add_to_compositor(&pen);
    helper.assert_exists_on_compositor(&brush_resource(&first), true);

    target.set_brush(Some((&second).into()));
    helper.assert_exists_on_compositor(&brush_resource(&first), false);
    helper.assert_exists_on_compositor(&brush_resource(&second), true);

    (pen.release)(&helper.compositor);
    helper.assert_exists_on_compositor(&brush_resource(&second), false);
}

// --- GeometryRenderResourceTests ----------------------------------------------------

/// Port of `Media/GeometryRenderResourceTests.cs`.
mod geometry_render_resource_tests {
    use super::*;

    #[test]
    fn changing_geometry_property_raises_invalidated() {
        let target = EllipseGeometry::with_rect(Rect::new(0.0, 0.0, 10.0, 10.0));
        RenderResourceTestHelper::assert_resource_invalidation(geometry_resource(&target), || {
            target.set_rect(Rect::new(0.0, 0.0, 20.0, 20.0))
        });
    }

    #[test]
    fn changing_transform_raises_invalidated() {
        let target = EllipseGeometry::with_rect(Rect::new(0.0, 0.0, 10.0, 10.0));
        RenderResourceTestHelper::assert_resource_invalidation(geometry_resource(&target), || {
            target.set_transform(TranslateTransform::with_offset(5.0, 5.0))
        });
    }

    #[test]
    fn changing_transform_value_raises_invalidated() {
        let transform = TranslateTransform::with_offset(5.0, 5.0);
        let target = EllipseGeometry::with_rect(Rect::new(0.0, 0.0, 10.0, 10.0));
        target.set_transform(transform.clone());
        RenderResourceTestHelper::assert_resource_invalidation(geometry_resource(&target), || {
            transform.set_x(10.0)
        });
    }

    #[test]
    fn adding_child_to_geometry_group_raises_invalidated() {
        let target = GeometryGroup::new();
        RenderResourceTestHelper::assert_resource_invalidation(geometry_resource(&target), || {
            target.children().add(EllipseGeometry::with_rect(Rect::new(0.0, 0.0, 10.0, 10.0)).upcast())
        });
    }

    #[test]
    fn changing_child_of_geometry_group_raises_invalidated() {
        let child = EllipseGeometry::with_rect(Rect::new(0.0, 0.0, 10.0, 10.0));
        let target = GeometryGroup::new();
        target.children().add(child.clone().upcast());
        RenderResourceTestHelper::assert_resource_invalidation(geometry_resource(&target), || {
            child.set_rect(Rect::new(0.0, 0.0, 20.0, 20.0))
        });
    }

    #[test]
    fn changing_geometry1_of_combined_geometry_raises_invalidated() {
        let geometry1 = EllipseGeometry::with_rect(Rect::new(0.0, 0.0, 10.0, 10.0));
        let geometry2 = RectangleGeometry::with_rect(Rect::new(5.0, 5.0, 10.0, 10.0));
        let target = CombinedGeometry::with_mode(
            GeometryCombineMode::Union,
            Some(geometry1.clone().upcast()),
            Some(geometry2.upcast()),
        );
        RenderResourceTestHelper::assert_resource_invalidation(geometry_resource(&target), || {
            geometry1.set_rect(Rect::new(0.0, 0.0, 20.0, 20.0))
        });
    }

    #[test]
    fn changing_combine_mode_of_combined_geometry_raises_invalidated() {
        let geometry1 = EllipseGeometry::with_rect(Rect::new(0.0, 0.0, 10.0, 10.0));
        let geometry2 = RectangleGeometry::with_rect(Rect::new(5.0, 5.0, 10.0, 10.0));
        let target =
            CombinedGeometry::with_mode(GeometryCombineMode::Union, Some(geometry1.upcast()), Some(geometry2.upcast()));
        RenderResourceTestHelper::assert_resource_invalidation(geometry_resource(&target), || {
            target.set_geometry_combine_mode(GeometryCombineMode::Intersect)
        });
    }
}

#[test]
fn geometry_platform_impl_reaches_the_server() {
    let helper = RenderResourceTestHelper::new();
    let target = RectangleGeometry::with_rect(Rect::new(1.0, 2.0, 3.0, 4.0));
    let resource = geometry_resource(&target);
    helper.add_to_compositor(&resource);
    helper.run_jobs();
    let server = helper.server::<ServerCompositionSimpleGeometry>(&resource);
    assert_eq!(server.geometry_impl().unwrap().bounds(), Rect::new(1.0, 2.0, 3.0, 4.0));

    target.set_rect(Rect::new(0.0, 0.0, 8.0, 9.0));
    helper.run_jobs();
    assert_eq!(server.geometry_impl().unwrap().bounds(), Rect::new(0.0, 0.0, 8.0, 9.0));
}

// --- RelativeTransformBrushTests (composition) ---------------------------------------

fn test_matrix() -> Matrix {
    Matrix::create_rotation(0.5) * Matrix::create_translation(0.25, 0.5)
}

fn assert_brush_reaches_server<T: crate::ObjectType + Upcast<Brush>>(brush: Ref<T>) {
    let helper = RenderResourceTestHelper::new();
    let relative_transform: Rc<dyn ITransform> = Rc::new(ImmutableTransform::new(test_matrix()));
    Upcast::<Brush>::upcast(&*brush).set_relative_transform(Some(relative_transform));

    let resource = brush_resource(&brush);
    helper.add_to_compositor(&resource);
    helper.run_jobs();

    let id = helper.try_get_server(&resource).unwrap();
    let server = helper.compositor.server().get_object(id).unwrap().as_brush().expect("a server-side brush");
    assert_eq!(server.relative_transform().unwrap().value(), test_matrix());
    (resource.release)(&helper.compositor);
}

fn with_red_stop<T: crate::ObjectType + Upcast<GradientBrush>>(brush: Ref<T>) -> Ref<T> {
    Upcast::<GradientBrush>::upcast(&*brush)
        .gradient_stops()
        .add(GradientStop::with_color_and_offset(Colors::RED, 0.0));
    brush
}

fn assert_composition_brush_reaches_server(factory: impl FnOnce(&Rc<Compositor>) -> Rc<CompositionBrush>) {
    let services = TestCompositor::new();
    let brush = factory(&services.compositor);

    brush.set_relative_transform(Some(Rc::new(ImmutableTransform::new(test_matrix()))));
    services.run_jobs();

    let server = services.compositor.server().get_object(brush.server()).expect("the server object exists");
    let server = server.as_brush().expect("a server-side brush");
    assert_eq!(test_matrix(), server.relative_transform().expect("the relative transform").value());
}

#[test]
fn composition_solid_color_brush_relative_transform_should_reach_the_server() {
    assert_composition_brush_reaches_server(|c| Rc::clone(&c.create_solid_color_brush_with(Colors::RED)));
}

#[test]
fn composition_linear_gradient_brush_relative_transform_should_reach_the_server() {
    assert_composition_brush_reaches_server(|c| Rc::clone(&c.create_linear_gradient_brush()));
}

#[test]
fn composition_radial_gradient_brush_relative_transform_should_reach_the_server() {
    assert_composition_brush_reaches_server(|c| Rc::clone(&c.create_radial_gradient_brush()));
}

#[test]
fn composition_conic_gradient_brush_relative_transform_should_reach_the_server() {
    assert_composition_brush_reaches_server(|c| Rc::clone(&c.create_conic_gradient_brush()));
}

#[test]
fn solid_color_brush_relative_transform_should_reach_the_server() {
    assert_brush_reaches_server(SolidColorBrush::with_color(Colors::RED));
}

#[test]
fn linear_gradient_brush_relative_transform_should_reach_the_server() {
    assert_brush_reaches_server(with_red_stop(LinearGradientBrush::new()));
}

#[test]
fn radial_gradient_brush_relative_transform_should_reach_the_server() {
    assert_brush_reaches_server(with_red_stop(RadialGradientBrush::new()));
}

#[test]
fn conic_gradient_brush_relative_transform_should_reach_the_server() {
    assert_brush_reaches_server(with_red_stop(ConicGradientBrush::new()));
}

#[test]
fn image_brush_relative_transform_should_reach_the_server() {
    assert_brush_reaches_server(ImageBrush::new());
}

#[test]
fn drawing_brush_relative_transform_should_reach_the_server() {
    assert_brush_reaches_server(DrawingBrush::new());
}

// Not from upstream.
#[test]
fn visual_brush_relative_transform_should_reach_the_server() {
    assert_brush_reaches_server(VisualBrush::new());
}

#[test]
fn changing_relative_transform_raises_invalidated() {
    let target = SolidColorBrush::new();
    RenderResourceTestHelper::assert_resource_invalidation(brush_resource(&target), || {
        target.set_relative_transform(Some(Rc::new(ImmutableTransform::new(test_matrix()))))
    });
}

#[test]
fn relative_transform_is_referenced_on_the_compositor() {
    let helper = RenderResourceTestHelper::new();
    let transform = RotateTransform::with_angle(45.0);
    let brush = SolidColorBrush::with_color(Colors::RED);
    brush.set_relative_transform(Some((&transform).into()));

    let resource = brush_resource(&brush);
    let transform_resource = transform_resource(&transform);
    helper.add_to_compositor(&resource);
    helper.run_jobs();
    assert!(helper.try_get_server(&transform_resource).is_some());
    // The server-side transform holds the value of the transform.
    let server_transform = helper.server::<ServerCompositionSimpleTransform>(&transform_resource);
    assert_eq!(server_transform.value(), transform.value());
    (resource.release)(&helper.compositor);

    assert!(helper.try_get_server(&transform_resource).is_none());
    helper.run_jobs();
    assert!(server_transform.is_disposed());
}

// --- the server-side classes -----------------------------------------------------------

#[test]
fn brush_properties_reach_the_server() {
    let helper = RenderResourceTestHelper::new();

    let solid = SolidColorBrush::with_color_and_opacity(Colors::RED, 0.5);
    solid.set_transform_origin(RelativePoint::CENTER);
    solid.set_transform(Some((&TranslateTransform::with_offset(3.0, 4.0)).into()));
    let solid_resource = brush_resource(&solid);
    helper.add_to_compositor(&solid_resource);

    let linear = LinearGradientBrush::new();
    linear.set_spread_method(GradientSpreadMethod::Repeat);
    linear.gradient_stops().add(GradientStop::with_color_and_offset(Colors::RED, 0.0));
    linear.gradient_stops().add(GradientStop::with_color_and_offset(Colors::BLUE, 1.0));
    linear.set_start_point(RelativePoint::new(1.0, 2.0, RelativeUnit::Absolute));
    let linear_resource = brush_resource(&linear);
    helper.add_to_compositor(&linear_resource);

    let radial = RadialGradientBrush::new();
    radial.set_radius_x(RelativeScalar::new(7.0, RelativeUnit::Absolute));
    let radial_resource = brush_resource(&radial);
    helper.add_to_compositor(&radial_resource);

    let conic = ConicGradientBrush::new();
    conic.set_angle(33.0);
    let conic_resource = brush_resource(&conic);
    helper.add_to_compositor(&conic_resource);

    // Nothing exists on the server before the batch.
    assert_eq!(helper.compositor.server().object_count(), 0);
    helper.run_jobs();

    let server = helper.server::<ServerCompositionSimpleSolidColorBrush>(&solid_resource);
    let brush: &dyn IBrush = &*server;
    assert_eq!(brush.as_solid_color_brush().unwrap().color(), Colors::RED);
    assert_eq!(brush.opacity(), 0.5);
    assert_eq!(brush.transform_origin(), RelativePoint::CENTER);
    // A mutable transform is sent as an immutable copy of its value.
    let transform = brush.transform().unwrap();
    assert_eq!(transform.value(), Matrix::create_translation(3.0, 4.0));
    assert!(transform.as_any().is::<ImmutableTransform>());
    assert!(brush.relative_transform().is_none());
    assert!(brush.as_gradient_brush().is_none());

    let server = helper.server::<ServerCompositionSimpleLinearGradientBrush>(&linear_resource);
    let brush: &dyn IBrush = &*server;
    let gradient = brush.as_gradient_brush().unwrap();
    assert_eq!(gradient.spread_method(), GradientSpreadMethod::Repeat);
    let gradient_stops = gradient.gradient_stops();
    assert_eq!(gradient_stops.len(), 2);
    assert_eq!((gradient_stops[1].color(), gradient_stops[1].offset()), (Colors::BLUE, 1.0));
    let linear_brush = brush.as_linear_gradient_brush().unwrap();
    assert_eq!(linear_brush.start_point(), RelativePoint::new(1.0, 2.0, RelativeUnit::Absolute));
    assert_eq!(linear_brush.end_point(), RelativePoint::BOTTOM_RIGHT);
    assert_eq!(brush.opacity(), 1.0);

    let server = helper.server::<ServerCompositionSimpleRadialGradientBrush>(&radial_resource);
    let radial_brush = IBrush::as_radial_gradient_brush(&*server).unwrap();
    assert_eq!(radial_brush.radius_x(), RelativeScalar::new(7.0, RelativeUnit::Absolute));
    assert_eq!(radial_brush.radius_y(), RelativeScalar::MIDDLE);
    assert_eq!(radial_brush.center(), RelativePoint::CENTER);
    assert_eq!(radial_brush.gradient_origin(), RelativePoint::CENTER);

    let server = helper.server::<ServerCompositionSimpleConicGradientBrush>(&conic_resource);
    let conic_brush = IBrush::as_conic_gradient_brush(&*server).unwrap();
    assert_eq!(conic_brush.angle(), 33.0);
    assert_eq!(conic_brush.center(), RelativePoint::CENTER);

    // A change of a gradient stop reaches the server with the next batch.
    linear.gradient_stops().get(0).set_color(Colors::GREEN);
    helper.run_jobs();
    let server = helper.server::<ServerCompositionSimpleLinearGradientBrush>(&linear_resource);
    assert_eq!(IBrush::as_gradient_brush(&*server).unwrap().gradient_stops()[0].color(), Colors::GREEN);
}

#[test]
fn pen_properties_reach_the_server_and_the_pen_observes_its_brush() {
    let helper = RenderResourceTestHelper::new();
    let brush = SolidColorBrush::with_color(Colors::RED);
    let dashes = DashStyle::with_dashes(Some(&[1.0, 2.0]), 3.0);
    let pen = Pen::with_all(
        Some((&brush).into()),
        2.5,
        Some((&dashes).into()),
        PenLineCap::Round,
        PenLineJoin::Bevel,
        4.0,
    );
    let resource = pen_resource(&pen);
    helper.add_to_compositor(&resource);
    helper.run_jobs();

    let server = helper.server::<ServerCompositionSimplePen>(&resource);
    let server_pen: &dyn IPen = &*server;
    assert_eq!(server_pen.thickness(), 2.5);
    assert_eq!(server_pen.line_cap(), PenLineCap::Round);
    assert_eq!(server_pen.line_join(), PenLineJoin::Bevel);
    assert_eq!(server_pen.miter_limit(), 4.0);
    let dash_style = server_pen.dash_style().unwrap();
    assert_eq!((dash_style.dashes(), dash_style.offset()), (Some(vec![1.0, 2.0]), 3.0));
    assert!(dash_style.as_any().is::<ImmutableDashStyle>());
    // The brush of the server pen is the server-side brush.
    let server_brush = helper.server::<ServerCompositionSimpleSolidColorBrush>(&brush_resource(&brush));
    let pen_brush = server_pen.brush().unwrap();
    assert!(std::ptr::addr_eq(Rc::as_ptr(&pen_brush), Rc::as_ptr(&server_brush)));
    assert_eq!(pen_brush.as_solid_color_brush().unwrap().color(), Colors::RED);
    assert_eq!(server_brush.brush_props().opacity(), 1.0);

    // The pen observes its brush: a change of the brush invalidates an
    // observer of the pen.
    struct Observer(Cell<u32>);
    impl IServerRenderResourceObserver for Observer {
        fn dependency_queued_invalidate(&self, _sender: &dyn IServerRenderResource) {
            self.0.set(self.0.get() + 1);
        }
    }
    let observer = Rc::new(Observer(Cell::new(0)));
    let as_observer: Rc<dyn IServerRenderResourceObserver> = observer.clone();
    server.add_observer(&as_observer);
    brush.set_color(Colors::BLUE);
    helper.run_jobs();
    assert_eq!(pen_brush.as_solid_color_brush().unwrap().color(), Colors::BLUE);
    assert!(observer.0.get() >= 1);

    // A change of the dash style reaches the server.
    dashes.set_offset(5.0);
    helper.run_jobs();
    assert_eq!(server_pen.dash_style().unwrap().offset(), 5.0);

    // An immutable brush is sent as it is; the mutable brush is released.
    let immutable: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::GREEN));
    pen.set_brush(Some(immutable.clone()));
    helper.assert_exists_on_compositor(&brush_resource(&brush), false);
    helper.run_jobs();
    assert!(std::ptr::addr_eq(Rc::as_ptr(&server_pen.brush().unwrap()), Rc::as_ptr(&immutable)));
    assert!(server_brush.is_disposed());

    // Releasing the pen disposes the server pen, which stops observing.
    (resource.release)(&helper.compositor);
    helper.run_jobs();
    assert!(server.is_disposed());
    assert!(server_pen.brush().is_none());
}

#[test]
fn every_brush_class_is_a_render_resource() {
    let brushes: [Rc<dyn IBrush>; 7] = [
        SolidColorBrush::new().into(),
        LinearGradientBrush::new().into(),
        RadialGradientBrush::new().into(),
        ConicGradientBrush::new().into(),
        ImageBrush::new().into(),
        VisualBrush::new().into(),
        DrawingBrush::new().into(),
    ];
    for brush in &brushes {
        assert!(brush.as_composition_render_resource().is_some());
    }
    let immutable: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::RED));
    assert!(immutable.as_composition_render_resource().is_none());

    // The scene brushes are tile brushes with recorded content; they have
    // no immutable form.
    for scene in &brushes[5..] {
        assert!(scene.as_scene_brush().is_some());
        assert!(scene.as_tile_brush().is_some());
        assert!(scene.as_image_brush().is_none());
        assert!(scene.as_mutable_brush().is_none());
    }
    assert!(brushes[4].as_scene_brush().is_none());
    assert!(brushes[4].as_mutable_brush().is_some());
}

// --- tile, image and scene brushes ------------------------------------------------------

fn mock_bitmap(helper: &RenderResourceTestHelper) -> Rc<Bitmap> {
    let log = helper.render_interface.log().clone();
    Rc::new(Bitmap::from_impl(Rc::new(MockDrawingContextLayerImpl::new(log, PixelSize::new(4, 2)))))
}

fn same_bitmap(platform_bitmap: &Rc<dyn crate::platform::IBitmapImpl>, bitmap: &Bitmap) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(platform_bitmap), Rc::as_ptr(&bitmap.platform_impl().item()))
}

#[test]
fn image_brush_properties_and_bitmap_reach_the_server() {
    let helper = RenderResourceTestHelper::new();
    let bitmap = mock_bitmap(&helper);
    let source: Rc<dyn IImageBrushSource> = bitmap.clone();

    let brush = ImageBrush::with_source(Some(source));
    brush.set_alignment_x(AlignmentX::Left);
    brush.set_alignment_y(AlignmentY::Bottom);
    brush.set_destination_rect(RelativeRect::new(1.0, 2.0, 3.0, 4.0, RelativeUnit::Absolute));
    brush.set_source_rect(RelativeRect::new(0.0, 0.0, 0.5, 0.5, RelativeUnit::Relative));
    brush.set_stretch(Stretch::UniformToFill);
    brush.set_tile_mode(TileMode::FlipX);
    brush.set_opacity(0.5);
    let resource = brush_resource(&brush);
    helper.add_to_compositor(&resource);
    assert_eq!(1, bitmap.platform_impl().ref_count());
    helper.run_jobs();

    let server = helper.server::<ServerCompositionSimpleImageBrush>(&resource);
    let server_brush: &dyn IBrush = &*server;
    assert_eq!(server_brush.opacity(), 0.5);
    assert!(server_brush.as_solid_color_brush().is_none());
    assert!(server_brush.as_scene_brush().is_none());
    let tile = server_brush.as_tile_brush().expect("a tile brush");
    assert_eq!(tile.alignment_x(), AlignmentX::Left);
    assert_eq!(tile.alignment_y(), AlignmentY::Bottom);
    assert_eq!(tile.destination_rect(), RelativeRect::new(1.0, 2.0, 3.0, 4.0, RelativeUnit::Absolute));
    assert_eq!(tile.source_rect(), RelativeRect::new(0.0, 0.0, 0.5, 0.5, RelativeUnit::Relative));
    assert_eq!(tile.stretch(), Stretch::UniformToFill);
    assert_eq!(tile.tile_mode(), TileMode::FlipX);

    // The server-side brush draws the platform bitmap through a reference
    // of its own.
    let image = server_brush.as_image_brush().expect("an image brush");
    let drawn = image.source().expect("the brush is its own source").get_bitmap().expect("a bitmap");
    assert!(same_bitmap(&drawn, &bitmap));
    assert!(same_bitmap(&server.bitmap().unwrap(), &bitmap));
    assert_eq!(2, bitmap.platform_impl().ref_count());

    // Every change sends the bitmap again; the reference is replaced.
    brush.set_tile_mode(TileMode::Tile);
    helper.run_jobs();
    assert_eq!(tile.tile_mode(), TileMode::Tile);
    assert_eq!(2, bitmap.platform_impl().ref_count());

    // Another bitmap, and none.
    let other = mock_bitmap(&helper);
    let other_source: Rc<dyn IImageBrushSource> = other.clone();
    brush.set_source(Some(other_source));
    helper.run_jobs();
    assert_eq!(1, bitmap.platform_impl().ref_count());
    assert_eq!(2, other.platform_impl().ref_count());
    assert!(same_bitmap(&server.bitmap().unwrap(), &other));

    brush.set_source(None);
    helper.run_jobs();
    assert_eq!(1, other.platform_impl().ref_count());
    assert!(server.bitmap().is_none());
    assert!(image.source().expect("the brush is its own source").get_bitmap().is_none());

    // Disposing the server-side brush releases its reference.
    let source: Rc<dyn IImageBrushSource> = bitmap.clone();
    brush.set_source(Some(source));
    helper.run_jobs();
    assert_eq!(2, bitmap.platform_impl().ref_count());
    (resource.release)(&helper.compositor);
    helper.run_jobs();
    assert!(server.is_disposed());
    assert!(server.bitmap().is_none());
    assert_eq!(1, bitmap.platform_impl().ref_count());
}

#[test]
fn changing_tile_brush_properties_raises_invalidated() {
    let target = ImageBrush::new();
    RenderResourceTestHelper::assert_resource_invalidation(brush_resource(&target), || {
        target.set_stretch(Stretch::Fill)
    });

    let helper = RenderResourceTestHelper::new();
    let target = ImageBrush::new();
    let source: Rc<dyn IImageBrushSource> = mock_bitmap(&helper);
    helper.assert_invalidation(&brush_resource(&target), || target.set_source(Some(source)));
}

fn render_content(content: &dyn ISceneBrushContent) -> Vec<String> {
    let log = DrawingLog::new();
    let mut context = MockDrawingContextImpl::new(log.clone());
    context.log_transforms = false;
    content.render(&mut context, None);
    log.entries()
}

fn server_content(brush: &ServerCompositionSimpleContentBrush) -> Option<Rc<dyn ISceneBrushContent>> {
    IBrush::as_scene_brush(brush).expect("a scene brush").create_content()
}

#[test]
fn drawing_brush_content_is_recorded_for_the_server_and_follows_changes() {
    let helper = RenderResourceTestHelper::new();
    let server = helper.compositor.server().clone();

    let geometry = RectangleGeometry::with_rect(Rect::new(1.5, 2.5, 10.0, 20.0));
    let fill = SolidColorBrush::with_color(Colors::RED);
    let drawing = GeometryDrawing::new();
    drawing.set_brush(Some((&fill).into()));
    drawing.set_geometry(geometry.clone().upcast::<Geometry>());
    let brush = DrawingBrush::with_drawing(&drawing);
    brush.set_stretch(Stretch::Fill);
    let handle: Rc<dyn IBrush> = (&brush).into();
    let (resource, fill_resource, geometry_resource) =
        (brush_resource(&brush), brush_resource(&fill), geometry_resource(&geometry));

    let render_data = helper.record(|context| context.fill_rectangle(&handle, Rect::new(0.0, 0.0, 50.0, 50.0), 0.0));
    helper.assert_exists_on_compositor(&resource, true);
    // The content is recorded when the brush is serialized, not before.
    helper.assert_exists_on_compositor(&fill_resource, false);
    assert_eq!(server.object_count(), 0);
    helper.run_jobs();
    helper.assert_exists_on_compositor(&fill_resource, true);
    helper.assert_exists_on_compositor(&geometry_resource, true);
    // The render data that draws with the brush, the brush, the render data
    // of its content, and the fill and the geometry the content draws with.
    assert_eq!(server.object_count(), 5);
    assert_eq!(helper.replay(&render_data), ["DrawRectangle brush none 0, 0, 50, 50 shadows=0"]);

    let server_brush = helper.server::<ServerCompositionSimpleContentBrush>(&resource);
    assert_eq!(IBrush::as_tile_brush(&*server_brush).unwrap().stretch(), Stretch::Fill);
    assert!(IBrush::as_image_brush(&*server_brush).is_none());
    let content = server_content(&server_brush).expect("the drawing draws something");
    // The bounds of what was drawn, rounded outwards to whole units.
    assert_eq!(content.rect(), Rect::new(1.0, 2.0, 11.0, 21.0));
    assert!(content.use_scalable_rasterization());
    assert_eq!(content.brush().stretch(), Stretch::Fill);
    assert_eq!(render_content(&*content), ["DrawGeometry Red none 1.5, 2.5, 10, 20"]);

    // Whatever draws with the brush is invalidated by a change of a
    // resource of the content: the fill is observed by the render data of
    // the content, that by the brush, and the brush by the render data that
    // draws with it.
    struct Observer(Cell<u32>);
    impl IServerRenderResourceObserver for Observer {
        fn dependency_queued_invalidate(&self, _sender: &dyn IServerRenderResource) {
            self.0.set(self.0.get() + 1);
        }
    }
    let observer = Rc::new(Observer(Cell::new(0)));
    let as_observer: Rc<dyn IServerRenderResourceObserver> = observer.clone();
    let server_data = server.get::<ServerCompositionRenderData>(render_data.server()).unwrap();
    server_data.add_observer(&as_observer);
    helper.run_jobs();
    assert_eq!(observer.0.get(), 0);

    // A change within a resource of the content reaches the server without
    // recording the content again.
    fill.set_color(Colors::BLUE);
    assert!(!helper.is_invalidated(&resource));
    helper.run_jobs();
    assert!(observer.0.get() >= 1);
    assert_eq!(server.object_count(), 5);
    assert_eq!(render_content(&*content), ["DrawGeometry Blue none 1.5, 2.5, 10, 20"]);

    let before = observer.0.get();
    geometry.set_rect(Rect::new(0.0, 0.0, 30.0, 15.0));
    helper.run_jobs();
    assert!(observer.0.get() > before);
    assert_eq!(content.rect(), Rect::new(0.0, 0.0, 30.0, 15.0));

    // A structural change of the drawing records the content again: the
    // brush gets new render data, and the old one is disposed with what
    // only it referenced.
    let old_content_data = content.clone();
    let before = observer.0.get();
    drawing.set_brush(Some(Brushes::lime()));
    assert!(helper.is_invalidated(&resource));
    helper.run_jobs();
    assert!(observer.0.get() > before);
    helper.assert_exists_on_compositor(&fill_resource, false);
    helper.assert_exists_on_compositor(&geometry_resource, true);
    assert_eq!(server.object_count(), 4);
    let content = server_content(&server_brush).expect("the drawing draws something");
    assert_eq!(render_content(&*content), ["DrawGeometry Lime none 0, 0, 30, 15"]);
    // The content made before draws nothing anymore.
    assert!(render_content(&*old_content_data).is_empty());

    // A property of the brush is serialized without recording again.
    brush.set_stretch(Stretch::None);
    helper.run_jobs();
    assert_eq!(server.object_count(), 4);
    assert_eq!(IBrush::as_tile_brush(&*server_brush).unwrap().stretch(), Stretch::None);
    assert_eq!(render_content(&*server_content(&server_brush).unwrap()), ["DrawGeometry Lime none 0, 0, 30, 15"]);

    // Without a drawing there is no content.
    brush.set_drawing(None);
    helper.run_jobs();
    assert!(server_content(&server_brush).is_none());
    helper.assert_exists_on_compositor(&geometry_resource, false);
    assert_eq!(server.object_count(), 2);

    // Content again, and then the last use of the brush goes away: the
    // brush releases the render data of its content.
    brush.set_drawing(&drawing);
    helper.run_jobs();
    assert_eq!(server.object_count(), 4);
    render_data.dispose();
    helper.assert_exists_on_compositor(&resource, false);
    helper.assert_exists_on_compositor(&geometry_resource, false);
    helper.run_jobs();
    assert_eq!(server.object_count(), 0);
    assert!(server_brush.is_disposed());
    assert!(server_content(&server_brush).is_none());

    // A detached brush no longer registers for serialization.
    drawing.set_brush(Some(Brushes::red()));
    assert!(!helper.is_invalidated(&resource));
}

#[test]
fn scene_brush_is_recorded_once_per_compositor_and_shared_by_its_uses() {
    let helper = RenderResourceTestHelper::new();
    let server = helper.compositor.server().clone();

    let drawing = GeometryDrawing::new();
    drawing.set_brush(Some(Brushes::red()));
    drawing.set_geometry(RectangleGeometry::with_rect(Rect::new(0.0, 0.0, 4.0, 4.0)).upcast::<Geometry>());
    let brush = DrawingBrush::with_drawing(&drawing);
    let handle: Rc<dyn IBrush> = (&brush).into();
    let resource = brush_resource(&brush);

    // As a fill, as the brush of a pen, and as an opacity mask.
    let pen = Pen::with_brush(Some(handle.clone()), 2.0);
    let pen_handle: Rc<dyn IPen> = (&pen).into();
    let blue: Rc<dyn IBrush> = Brushes::blue();
    let first = helper.record(|context| context.fill_rectangle(&handle, Rect::new(0.0, 0.0, 5.0, 5.0), 0.0));
    let second = helper.record(|context| {
        context.draw_line(&pen_handle, Point::new(0.0, 0.0), Point::new(4.0, 0.0));
        let state = context.push_opacity_mask(&handle, Rect::new(0.0, 0.0, 8.0, 8.0));
        context.fill_rectangle(&blue, Rect::new(0.0, 0.0, 8.0, 8.0), 0.0);
        context.pop(state);
    });
    helper.run_jobs();
    // Two render data, the pen, the brush, the render data of its content
    // and the geometry of the drawing.
    assert_eq!(server.object_count(), 6);
    assert_eq!(
        helper.replay(&second),
        [
            "DrawLine brush@2 0, 0 4, 0",
            "PushOpacityMask brush 0, 0, 8, 8",
            "DrawRectangle Blue none 0, 0, 8, 8 shadows=0",
            "PopOpacityMask",
        ]
    );
    let server_brush = helper.server::<ServerCompositionSimpleContentBrush>(&resource);
    let server_pen = helper.server::<ServerCompositionSimplePen>(&pen_resource(&pen));
    let pen_brush = IPen::brush(&*server_pen).expect("the pen has a brush");
    let content = pen_brush.as_scene_brush().expect("a scene brush").create_content().expect("content");
    assert_eq!(render_content(&*content), ["DrawGeometry Red none 0, 0, 4, 4"]);

    first.dispose();
    helper.run_jobs();
    assert!(!server_brush.is_disposed());
    assert_eq!(server.object_count(), 5);

    second.dispose();
    helper.assert_exists_on_compositor(&resource, false);
    helper.run_jobs();
    assert!(server_brush.is_disposed());
    assert_eq!(server.object_count(), 0);
}

// --- end to end through recorded render data ---------------------------------------------

#[test]
fn drawing_with_mutable_resources_follows_their_changes_without_recording_again() {
    let helper = RenderResourceTestHelper::new();
    let server = helper.compositor.server().clone();

    let brush = SolidColorBrush::with_color(Colors::RED);
    let pen_brush = SolidColorBrush::with_color(Colors::BLUE);
    let pen = Pen::with_brush(Some((&pen_brush).into()), 2.0);
    let geometry = RectangleGeometry::with_rect(Rect::new(10.0, 10.0, 20.0, 20.0));
    let (brush_handle, pen_handle): (Rc<dyn IBrush>, Rc<dyn IPen>) = ((&brush).into(), (&pen).into());

    let render_data = helper.record(|context| {
        context.fill_rectangle(&brush_handle, Rect::new(0.0, 0.0, 5.0, 5.0), 0.0);
        context.draw_line(&pen_handle, Point::new(0.0, 0.0), Point::new(4.0, 0.0));
        context.draw_geometry(Some(&brush_handle), Some(&pen_handle), &geometry.clone().upcast());
    });
    // Recording referenced the resources on the compositor, transitively.
    let (brush_res, pen_res, pen_brush_res, geometry_res) =
        (brush_resource(&brush), pen_resource(&pen), brush_resource(&pen_brush), geometry_resource(&geometry));
    for resource in [&brush_res, &pen_res, &pen_brush_res, &geometry_res] {
        helper.assert_exists_on_compositor(resource, true);
    }
    // The UI-thread copy is hit tested with the client objects.
    assert!(render_data.hit_test(Point::new(2.0, 2.0)));
    assert!(render_data.hit_test(Point::new(15.0, 15.0)));
    assert!(!render_data.hit_test(Point::new(50.0, 50.0)));

    assert_eq!(server.object_count(), 0);
    helper.run_jobs();
    // The render data, two brushes, the pen and the geometry.
    assert_eq!(server.object_count(), 5);
    assert_eq!(
        helper.replay(&render_data),
        [
            "DrawRectangle Red none 0, 0, 5, 5 shadows=0",
            "DrawLine Blue@2 0, 0 4, 0",
            "DrawGeometry Red Blue@2 10, 10, 20, 20",
        ]
    );
    let server_data = server.get::<ServerCompositionRenderData>(render_data.server()).unwrap();
    let bounds = server_data.bounds().unwrap();
    assert_eq!((bounds.left, bounds.top, bounds.right, bounds.bottom), (0.0, -1.0, 31.0, 31.0));

    // The server render data observes the resources it draws with.
    struct Observer(Cell<u32>);
    impl IServerRenderResourceObserver for Observer {
        fn dependency_queued_invalidate(&self, _sender: &dyn IServerRenderResource) {
            self.0.set(self.0.get() + 1);
        }
    }
    let observer = Rc::new(Observer(Cell::new(0)));
    let as_observer: Rc<dyn IServerRenderResourceObserver> = observer.clone();
    server_data.add_observer(&as_observer);
    helper.run_jobs();
    assert_eq!(observer.0.get(), 0);

    // Changing the brush color: only the brush is serialized, and the
    // render data replays the new color without being recorded again.
    brush.set_color(Colors::GREEN);
    assert!(helper.is_invalidated(&brush_res));
    assert!(!helper.is_invalidated(&pen_res));
    helper.run_jobs();
    assert_eq!(observer.0.get(), 1);
    assert_eq!(server.object_count(), 5);
    assert_eq!(
        helper.replay(&render_data),
        [
            "DrawRectangle Green none 0, 0, 5, 5 shadows=0",
            "DrawLine Blue@2 0, 0 4, 0",
            "DrawGeometry Green Blue@2 10, 10, 20, 20",
        ]
    );

    // The pen: its thickness, and the color of its brush (the pen observes
    // the brush, the render data the pen).
    pen.set_thickness(4.0);
    pen_brush.set_color(Colors::YELLOW);
    helper.run_jobs();
    assert!(observer.0.get() >= 2);
    assert_eq!(helper.replay(&render_data)[1], "DrawLine Yellow@4 0, 0 4, 0");
    // The bounds were invalidated with the pen and follow its thickness.
    let bounds = server_data.bounds().unwrap();
    assert_eq!((bounds.left, bounds.top, bounds.right, bounds.bottom), (0.0, -2.0, 32.0, 32.0));

    // The geometry.
    let before = observer.0.get();
    geometry.set_rect(Rect::new(0.0, 0.0, 40.0, 50.0));
    helper.run_jobs();
    assert!(observer.0.get() > before);
    assert_eq!(helper.replay(&render_data)[2], "DrawGeometry Green Yellow@4 0, 0, 40, 50");
    let bounds = server_data.bounds().unwrap();
    assert_eq!((bounds.right, bounds.bottom), (42.0, 52.0));
    // Hit testing on the UI thread sees the new geometry too.
    assert!(render_data.hit_test(Point::new(35.0, 45.0)));

    // Disposing the last render data releases the resources: their server
    // objects are disposed with the next batch.
    let server_brush = helper.server::<ServerCompositionSimpleSolidColorBrush>(&brush_res);
    let server_pen = helper.server::<ServerCompositionSimplePen>(&pen_res);
    let server_geometry = helper.server::<ServerCompositionSimpleGeometry>(&geometry_res);
    render_data.dispose();
    for resource in [&brush_res, &pen_res, &pen_brush_res, &geometry_res] {
        helper.assert_exists_on_compositor(resource, false);
    }
    assert_eq!(server.object_count(), 5);
    helper.run_jobs();
    assert_eq!(server.object_count(), 0);
    assert!(server_brush.is_disposed() && server_pen.is_disposed() && server_geometry.is_disposed());
    assert!(server_data.is_disposed());

    // A detached brush no longer registers for serialization.
    brush.set_color(Colors::RED);
    assert!(!helper.is_invalidated(&brush_res));
}

#[test]
fn a_resource_shared_by_two_render_data_lives_until_the_last_one_is_disposed() {
    let helper = RenderResourceTestHelper::new();
    let server = helper.compositor.server().clone();
    let brush = SolidColorBrush::with_color(Colors::RED);
    let handle: Rc<dyn IBrush> = (&brush).into();
    let resource = brush_resource(&brush);

    let first = helper.record(|context| context.fill_rectangle(&handle, Rect::new(0.0, 0.0, 5.0, 5.0), 0.0));
    let second = helper.record(|context| context.fill_rectangle(&handle, Rect::new(5.0, 5.0, 5.0, 5.0), 0.0));
    helper.run_jobs();
    // One server brush for both.
    assert_eq!(server.object_count(), 3);
    let server_brush = helper.server::<ServerCompositionSimpleSolidColorBrush>(&resource);

    first.dispose();
    helper.run_jobs();
    helper.assert_exists_on_compositor(&resource, true);
    assert!(!server_brush.is_disposed());
    assert_eq!(server.object_count(), 2);

    brush.set_color(Colors::BLUE);
    helper.run_jobs();
    assert_eq!(helper.replay(&second), ["DrawRectangle Blue none 5, 5, 5, 5 shadows=0"]);

    second.dispose();
    helper.assert_exists_on_compositor(&resource, false);
    helper.run_jobs();
    assert!(server_brush.is_disposed());
    assert_eq!(server.object_count(), 0);
    let _ = helper.render_interface.log();
}

// --- RelativeTransformBrushTests (media) -------------------------------------------------

fn assert_carried_to_immutable<T: crate::ObjectType + Upcast<Brush>>(brush: Ref<T>) {
    Upcast::<Brush>::upcast(&*brush).set_relative_transform(Some(Rc::new(ImmutableTransform::new(test_matrix()))));
    let handle: Rc<dyn IBrush> = brush.into();
    let immutable = crate::media::BrushExtensions::to_immutable(&handle);
    assert_eq!(immutable.relative_transform().unwrap().value(), test_matrix());
}

#[test]
fn solid_color_brush_to_immutable_carries_relative_transform() {
    assert_carried_to_immutable(SolidColorBrush::with_color(Colors::RED));
}

#[test]
fn linear_gradient_brush_to_immutable_carries_relative_transform() {
    assert_carried_to_immutable(LinearGradientBrush::new());
}

#[test]
fn radial_gradient_brush_to_immutable_carries_relative_transform() {
    assert_carried_to_immutable(RadialGradientBrush::new());
}

#[test]
fn conic_gradient_brush_to_immutable_carries_relative_transform() {
    assert_carried_to_immutable(ConicGradientBrush::new());
}

#[test]
fn image_brush_to_immutable_carries_relative_transform() {
    assert_carried_to_immutable(ImageBrush::new());
}

#[test]
fn immutable_solid_color_brushes_differing_only_in_relative_transform_are_not_equal() {
    let first: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::RED));
    let second: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::with_transforms(
        Colors::RED,
        1.0,
        None,
        Some(Rc::new(ImmutableTransform::new(test_matrix()))),
    ));
    assert!(*first != *second);
}
