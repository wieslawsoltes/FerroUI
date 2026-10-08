//! Port of upstream's `HitTesting.cs` of the Skia unit tests.
//!
//! Upstream's fixture `CompositorTestServices(size, renderInterface)` of
//! the shared unit test project (`tests/*.UnitTests/CompositorTestServices.cs`)
//! is ported below with the members these tests use: an
//! `EmbeddableControlRoot` whose template is a single content presenter,
//! over a top-level implementation with the given client size, rendering
//! through the compositor of the controls testing harness
//! (`ferroui_controls::testing::CompositorTestServices`, driven by a manual
//! render loop). Its debug events are not used here and are not installed.
//!
//! `using (Locator.EnterScope())` is a [`LocatorScope`] guard that disposes
//! the scope when dropped.

use super::mock_platform_render_interface;
use crate::SkiaPlatform;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{Brushes, Geometry, GeometryHitTestResult, IntersectionResult, RectangleGeometry};
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::rendering::composition::CompositingRenderer;
use ferroui_base::rendering::IHitTester;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{FerroLocator, LocatorExtensions, Point, Rect, Ref, Size, Thickness, Visual};
use ferroui_controls::embedding::EmbeddableControlRoot;
use ferroui_controls::presentation_source::ITopLevelRenderer;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::shapes::{Ellipse, Line};
use ferroui_controls::templates::FuncControlTemplate;
use ferroui_controls::testing::{self, MockImplKind, MockWindowImpl};
use ferroui_controls::{Border, ContentControl, Control};
use ferroui_base::data::TemplateBinding;
use ferroui_base::FerroObject;
use std::rc::Rc;

/// Upstream's `using (Locator.EnterScope())`.
struct LocatorScope(Rc<dyn IDisposable>);

impl LocatorScope {
    fn enter() -> LocatorScope {
        LocatorScope(FerroLocator::enter_scope())
    }
}

impl Drop for LocatorScope {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

/// Upstream's `CompositorTestServices` with a client size and a render
/// interface.
struct CompositorTestServices {
    // Declared first: dropped before the application of the harness.
    top_level: Ref<EmbeddableControlRoot>,
    renderer: Rc<dyn ITopLevelRenderer>,
    services: testing::CompositorTestServices,
}

impl CompositorTestServices {
    fn new(size: Size, render_interface: Rc<dyn IPlatformRenderInterface>) -> CompositorTestServices {
        let services = testing::CompositorTestServices::start(
            mock_platform_render_interface().with_render_interface(render_interface),
        );
        let platform_impl = MockWindowImpl::bare(MockImplKind::TopLevel);
        platform_impl.client_size.set(size);
        services.setup(&platform_impl);

        let top_level = EmbeddableControlRoot::with_impl(platform_impl);
        top_level.set_template(Some(FuncControlTemplate::new(|_, scope| {
            let presenter = ContentPresenter::new();
            presenter.bind_binding(
                ContentPresenter::content_property().as_property(),
                &TemplateBinding::new(ContentControl::content_property().as_property()),
            );
            scope.register("PART_ContentPresenter", presenter.clone().upcast::<FerroObject>());
            presenter.upcast()
        })));
        top_level.prepare();
        top_level.start_rendering();
        services.run_jobs();
        let renderer = top_level.renderer();

        CompositorTestServices { top_level, renderer, services }
    }

    /// `TopLevel.Content = content`.
    fn set_content(&self, content: &Control) {
        self.top_level.set_content(Some(Control::boxed(content.to_ref())));
    }

    fn renderer(&self) -> &CompositingRenderer {
        self.renderer.as_any().downcast_ref::<CompositingRenderer>().expect("the top-level renders through composition")
    }

    fn run_jobs(&self) {
        self.services.run_jobs();
    }

    fn assert_hit_test(&self, x: f64, y: f64, filter: Option<&dyn Fn(&Visual) -> bool>, expected: &[Ref<Visual>]) {
        self.run_jobs();
        let tested = self.renderer().hit_test(Point::new(x, y), &self.top_level, filter);
        assert_eq!(expected, tested.as_slice());
    }

    fn assert_hit_test_geometry(
        &self,
        geometry: &Geometry,
        filter: Option<&dyn Fn(&Visual) -> bool>,
        expected: &[GeometryHitTestResult],
    ) {
        self.run_jobs();
        let tested = self.renderer().hit_test_geometry(geometry, &self.top_level, filter);
        assert_eq!(expected, tested.as_slice());
    }
}

impl Drop for CompositorTestServices {
    fn drop(&mut self) {
        if std::thread::panicking() {
            return;
        }
        self.top_level.stop_rendering();
        self.top_level.dispose();
    }
}

/// `Locator.Current.GetRequiredService<IPlatformRenderInterface>()`.
fn required_render_interface() -> Rc<dyn IPlatformRenderInterface> {
    FerroLocator::current().get_required_service::<dyn IPlatformRenderInterface>()
}

/// `new RectangleGeometry(new Rect(x, y, width, height))`.
fn rectangle_geometry(x: f64, y: f64, width: f64, height: f64) -> Ref<RectangleGeometry> {
    RectangleGeometry::with_rect(Rect::new(x, y, width, height))
}

/// `new GeometryHitTestResult(visual, result)`.
fn geometry_hit_test_result(visual: &Visual, result: IntersectionResult) -> GeometryHitTestResult {
    GeometryHitTestResult::new(visual.to_ref(), result)
}

#[test]
fn hit_test_should_respect_fill() {
    let _scope = LocatorScope::enter();
    SkiaPlatform::initialize();

    let services = CompositorTestServices::new(Size::new(100.0, 100.0), required_render_interface());
    let content = Ellipse::new();
    content.set_width(100.0);
    content.set_height(100.0);
    content.set_fill(Some(Brushes::red()));
    content.set_horizontal_alignment(HorizontalAlignment::Center);
    content.set_vertical_alignment(VerticalAlignment::Center);
    services.set_content(&content);

    services.assert_hit_test(10.0, 10.0, None, &[]);
    services.assert_hit_test(50.0, 50.0, None, &[content.to_ref().upcast()]);
}

#[test]
fn hit_test_should_respect_stroke() {
    let _scope = LocatorScope::enter();
    SkiaPlatform::initialize();

    let services = CompositorTestServices::new(Size::new(100.0, 100.0), required_render_interface());
    let content = Ellipse::new();
    content.set_width(100.0);
    content.set_height(100.0);
    content.set_stroke(Some(Brushes::red()));
    content.set_stroke_thickness(5.0);
    content.set_horizontal_alignment(HorizontalAlignment::Center);
    content.set_vertical_alignment(VerticalAlignment::Center);
    services.set_content(&content);

    services.assert_hit_test(50.0, 50.0, None, &[]);
    services.assert_hit_test(1.0, 50.0, None, &[content.to_ref().upcast()]);
}

#[test]
fn geometry_hit_test_should_detect_stroke_only_shapes() {
    let _scope = LocatorScope::enter();
    SkiaPlatform::initialize();

    let services = CompositorTestServices::new(Size::new(100.0, 100.0), required_render_interface());
    let line = Line::new();
    line.set_start_point(Point::new(0.0, 0.0));
    line.set_end_point(Point::new(100.0, 0.0));
    line.set_stroke(Some(Brushes::red()));
    line.set_stroke_thickness(5.0);
    line.set_horizontal_alignment(HorizontalAlignment::Center);
    line.set_vertical_alignment(VerticalAlignment::Center);
    services.set_content(&line);

    services.assert_hit_test_geometry(&rectangle_geometry(25.0, 25.0, 10.0, 10.0), None, &[]);
    services.assert_hit_test_geometry(
        &rectangle_geometry(45.0, 45.0, 10.0, 10.0),
        None,
        &[geometry_hit_test_result(&line, IntersectionResult::Intersects)],
    );
}

#[test]
fn geometry_hit_test_border_with_background_and_border() {
    let _scope = LocatorScope::enter();
    SkiaPlatform::initialize();

    let services = CompositorTestServices::new(Size::new(400.0, 400.0), required_render_interface());
    let border = Border::new();
    border.set_width(200.0);
    border.set_height(200.0);
    border.set_background(Some(Brushes::red()));
    border.set_border_brush(Some(Brushes::black()));
    border.set_border_thickness(Thickness::uniform(10.0));
    border.set_horizontal_alignment(HorizontalAlignment::Center);
    border.set_vertical_alignment(VerticalAlignment::Center);
    services.set_content(&border);

    // Geometry fully inside the border -> FullyContains
    services.assert_hit_test_geometry(
        &rectangle_geometry(100.0, 100.0, 50.0, 50.0),
        None,
        &[geometry_hit_test_result(&border, IntersectionResult::FullyContains)],
    );

    // Geometry overlapping the border stroke -> Intersects
    services.assert_hit_test_geometry(
        &rectangle_geometry(195.0, 95.0, 10.0, 10.0),
        None,
        &[geometry_hit_test_result(&border, IntersectionResult::Intersects)],
    );

    // Geometry outside -> no hit
    services.assert_hit_test_geometry(&rectangle_geometry(310.0, 310.0, 10.0, 10.0), None, &[]);
}

#[test]
fn geometry_hit_test_border_with_background_and_null_border() {
    let _scope = LocatorScope::enter();
    SkiaPlatform::initialize();

    let services = CompositorTestServices::new(Size::new(400.0, 400.0), required_render_interface());
    let border = Border::new();
    border.set_width(200.0);
    border.set_height(200.0);
    border.set_background(Some(Brushes::red()));
    border.set_border_brush(None);
    border.set_border_thickness(Thickness::uniform(0.0));
    border.set_horizontal_alignment(HorizontalAlignment::Center);
    border.set_vertical_alignment(VerticalAlignment::Center);
    services.set_content(&border);

    // Geometry fully inside the border -> FullyContains
    services.assert_hit_test_geometry(
        &rectangle_geometry(100.0, 100.0, 50.0, 50.0),
        None,
        &[geometry_hit_test_result(&border, IntersectionResult::FullyContains)],
    );

    // Geometry overlapping the border stroke -> Intersects
    services.assert_hit_test_geometry(
        &rectangle_geometry(195.0, 95.0, 10.0, 10.0),
        None,
        &[geometry_hit_test_result(&border, IntersectionResult::Intersects)],
    );

    // Geometry outside -> no hit
    services.assert_hit_test_geometry(&rectangle_geometry(310.0, 310.0, 10.0, 10.0), None, &[]);
}

#[test]
fn geometry_hit_test_border_null_background_with_border_brush() {
    let _scope = LocatorScope::enter();
    SkiaPlatform::initialize();

    let services = CompositorTestServices::new(Size::new(400.0, 400.0), required_render_interface());
    let border = Border::new();
    border.set_width(200.0);
    border.set_height(200.0);
    border.set_background(None);
    border.set_border_brush(Some(Brushes::black()));
    border.set_border_thickness(Thickness::uniform(10.0));
    border.set_horizontal_alignment(HorizontalAlignment::Center);
    border.set_vertical_alignment(VerticalAlignment::Center);
    services.set_content(&border);

    // Geometry fully inside (center) should NOT hit because background is null
    services.assert_hit_test_geometry(&rectangle_geometry(150.0, 150.0, 50.0, 50.0), None, &[]);

    // Geometry overlapping the border stroke -> Intersects
    services.assert_hit_test_geometry(
        &rectangle_geometry(190.0, 105.0, 10.0, 10.0),
        None,
        &[geometry_hit_test_result(&border, IntersectionResult::Intersects)],
    );
}

#[test]
fn geometry_hit_test_border_null_background_and_null_border_brush_no_hit() {
    let _scope = LocatorScope::enter();
    SkiaPlatform::initialize();

    let services = CompositorTestServices::new(Size::new(400.0, 400.0), required_render_interface());
    let border = Border::new();
    border.set_width(200.0);
    border.set_height(200.0);
    border.set_background(None);
    border.set_border_brush(None);
    border.set_border_thickness(Thickness::uniform(10.0));
    border.set_horizontal_alignment(HorizontalAlignment::Center);
    border.set_vertical_alignment(VerticalAlignment::Center);
    services.set_content(&border);

    // No background and no border brush -> no hit for any geometry
    services.assert_hit_test_geometry(&rectangle_geometry(100.0, 100.0, 50.0, 50.0), None, &[]);
    services.assert_hit_test_geometry(&rectangle_geometry(195.0, 100.0, 10.0, 10.0), None, &[]);
}

#[test]
fn geometry_hit_test_line_intersects_rectangle_geometry() {
    let _scope = LocatorScope::enter();
    SkiaPlatform::initialize();

    let services = CompositorTestServices::new(Size::new(100.0, 100.0), required_render_interface());
    let line = Line::new();
    line.set_start_point(Point::new(0.0, 0.0));
    line.set_end_point(Point::new(100.0, 100.0));
    line.set_stroke(Some(Brushes::red()));
    line.set_stroke_thickness(5.0);
    line.set_horizontal_alignment(HorizontalAlignment::Center);
    line.set_vertical_alignment(VerticalAlignment::Center);
    services.set_content(&line);

    // Rectangle away from the line -> no hit
    services.assert_hit_test_geometry(&rectangle_geometry(0.0, 80.0, 10.0, 10.0), None, &[]);

    // Rectangle overlapping the diagonal line -> Intersects
    services.assert_hit_test_geometry(
        &rectangle_geometry(45.0, 45.0, 10.0, 10.0),
        None,
        &[geometry_hit_test_result(&line, IntersectionResult::Intersects)],
    );
}
