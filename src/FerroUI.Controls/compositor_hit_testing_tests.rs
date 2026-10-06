//! Port of `CompositorHitTestingTests.cs`: hit testing of a control tree
//! through the compositing renderer, by point and by geometry.
//!
//! The upstream fixture `CompositorTestServices(size)` (with its base class
//! `CompositorTestsBase`) maps to [`CompositorTestServices`] below: an
//! `EmbeddableControlRoot` whose template is a single content presenter,
//! over a top-level implementation with the given client size, whose
//! compositor is the test compositor of the controls testing harness
//! (`testing::CompositorTestServices`, driven by a manual render loop).
//! `RunJobs`, `AssertHitTest` and `AssertHitTestFirst` map to the methods
//! of the same names. The debug events of the upstream fixture are not
//! used by these tests and are not installed.
//!
//! `CustomHitTestBorder.cs` is ported as [`CustomHitTestBorder`]: the port
//! expresses `ICustomHitTest` through the `custom_hit_test` and
//! `custom_hit_test_geometry` virtuals of the visual.
//!
//! Upstream theories are ported as one test that runs every inline data
//! row.

use crate::embedding::EmbeddableControlRoot;
use crate::presentation_source::ITopLevelRenderer;
use crate::presenters::{ContentPresenter, ScrollContentPresenter};
use crate::shapes::{Line, Path};
use crate::templates::FuncControlTemplate;
use crate::testing::{self, MockImplKind, MockWindowImpl, TestServices};
use crate::{Border, Canvas, ContentControl, Control, ControlImpl, Panel, StackPanel};
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, Orientation, VerticalAlignment};
use ferroui_base::media::{
    Brushes, Colors, Geometry, GeometryHitTestResult, IntersectionResult, RectangleGeometry, StreamGeometry,
    TranslateTransform,
};
use ferroui_base::rendering::composition::{CompositingRenderer, Compositor, ElementComposition};
use ferroui_base::rendering::IHitTester;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObject, FerroObjectImpl, Point, Rect, Ref, Size,
    StyledElementImpl, Thickness, Vector, Visual, VisualImpl,
};
use std::rc::Rc;

// --- CustomHitTestBorder.cs --------------------------------------------------------

/// A border whose hit testing window is moved halfway to the left.
#[repr(C)]
struct CustomHitTestBorder {
    base: Border,
}

ferro_class!(CustomHitTestBorder: Border);
ferro_impl_classes!(
    CustomHitTestBorder: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl VisualImpl for CustomHitTestBorder {
    fn custom_hit_test(this: &Self, point: Point) -> Option<bool> {
        Some(this.hit_test(point))
    }

    fn custom_hit_test_geometry(this: &Self, geometry: &Ref<Geometry>) -> Option<IntersectionResult> {
        Some(this.hit_test_geometry(geometry))
    }
}

impl CustomHitTestBorder {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Border::construct() })
    }

    fn hit_test(&self, point: Point) -> bool {
        // Move hit testing window halfway to the left
        Rect::new(-self.bounds().width / 2.0, 0.0, self.bounds().width, self.bounds().height).contains(point)
    }

    fn hit_test_geometry(&self, geometry: &Geometry) -> IntersectionResult {
        let window = RectangleGeometry::with_rect(Rect::new(
            -self.bounds().width / 2.0,
            0.0,
            self.bounds().width,
            self.bounds().height,
        ));
        geometry.get_fill_intersection_result(&window).unwrap_or(IntersectionResult::Empty)
    }
}

// --- CompositorTestServices.cs -----------------------------------------------------

/// The upstream `CompositorTestServices` with a client size: an embeddable
/// root rendered through the compositing renderer of the test compositor.
pub(crate) struct CompositorTestServices {
    // Declared first: dropped before the application of the harness.
    top_level: Ref<EmbeddableControlRoot>,
    renderer: Rc<dyn ITopLevelRenderer>,
    services: testing::CompositorTestServices,
}

impl CompositorTestServices {
    pub(crate) fn new(size: Size) -> CompositorTestServices {
        let services = testing::CompositorTestServices::start(TestServices::mock_platform_render_interface());
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

    fn compositor(&self) -> &Rc<Compositor> {
        self.services.compositor()
    }

    pub(crate) fn set_content(&self, content: &Control) {
        self.top_level.set_content(Some(Control::boxed(content.to_ref())));
    }

    fn renderer(&self) -> &CompositingRenderer {
        self.renderer.as_any().downcast_ref::<CompositingRenderer>().expect("the top-level renders through composition")
    }

    pub(crate) fn run_jobs(&self) {
        self.services.run_jobs();
    }

    fn assert_hit_test(&self, pt: Point, filter: Option<&dyn Fn(&Visual) -> bool>, expected: &[Ref<Visual>]) {
        self.run_jobs();
        let tested = self.renderer().hit_test(pt, &self.top_level, filter);
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

    fn assert_hit_test_first(&self, pt: Point, filter: Option<&dyn Fn(&Visual) -> bool>, expected: Option<&Visual>) {
        self.run_jobs();
        let tested = self.renderer().hit_test_first(pt, &self.top_level, filter);
        assert_eq!(expected.map(Visual::to_ref), tested);
    }

    fn assert_hit_test_first_geometry(
        &self,
        geometry: &Geometry,
        filter: Option<&dyn Fn(&Visual) -> bool>,
        expected: Option<&Visual>,
    ) {
        self.run_jobs();
        let tested = self.renderer().hit_test_first_geometry(geometry, &self.top_level, filter);
        assert_eq!(expected.map(Visual::to_ref), tested.map(|result| result.visual_hit));
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

// --- helpers (not from upstream) ---------------------------------------------------

/// The visual as an expected hit test result (not from upstream: the
/// handle form of a `Visual` argument).
fn v(visual: &Visual) -> Ref<Visual> {
    visual.to_ref()
}

/// The upstream `new GeometryHitTestResult(visual, result)` (not from
/// upstream: a shorthand).
fn ghr(visual: &Visual, result: IntersectionResult) -> GeometryHitTestResult {
    GeometryHitTestResult::new(visual.to_ref(), result)
}

/// The upstream `new RectangleGeometry(new Rect(x, y, width, height))`
/// (not from upstream: a shorthand).
fn rect_geometry(x: f64, y: f64, width: f64, height: f64) -> Ref<RectangleGeometry> {
    RectangleGeometry::with_rect(Rect::new(x, y, width, height))
}

/// A border with a size, a background and alignments (not from upstream:
/// the object initializer most tests repeat).
fn border(
    width: f64,
    height: f64,
    horizontal: HorizontalAlignment,
    vertical: VerticalAlignment,
) -> Ref<Border> {
    let border = Border::new();
    border.set_width(width);
    border.set_height(height);
    border.set_background(Some(Brushes::red()));
    border.set_horizontal_alignment(horizontal);
    border.set_vertical_alignment(vertical);
    border
}

/// A centered red 100x100 border (not from upstream: the border of many
/// tests).
fn centered_border() -> Ref<Border> {
    border(100.0, 100.0, HorizontalAlignment::Center, VerticalAlignment::Center)
}

/// A red border that stretches in both directions (not from upstream: the
/// child of several tests).
fn stretched_border() -> Ref<Border> {
    let border = Border::new();
    border.set_background(Some(Brushes::red()));
    border.set_horizontal_alignment(HorizontalAlignment::Stretch);
    border.set_vertical_alignment(VerticalAlignment::Stretch);
    border
}

fn is_not(target: &Visual) -> impl Fn(&Visual) -> bool + '_ {
    move |visual| !std::ptr::eq(visual, target)
}

// --- tests -------------------------------------------------------------------------

#[test]
fn hit_test_should_find_controls_at_point() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let border = centered_border();

    s.set_content(&border);

    s.assert_hit_test(Point::new(100.0, 100.0), None, &[v(&border)]);
}

#[test]
fn hit_test_should_find_controls_at_geometry() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let border = centered_border();

    s.set_content(&border);

    s.assert_hit_test_geometry(
        &rect_geometry(100.0, 100.0, 50.0, 50.0),
        None,
        &[ghr(&border, IntersectionResult::FullyContains)],
    );
}

#[test]
fn hit_test_should_not_find_empty_controls_at_point() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let border = centered_border();
    border.set_background(None);

    s.set_content(&border);

    s.assert_hit_test(Point::new(100.0, 100.0), None, &[]);
}

#[test]
fn hit_test_should_not_find_empty_controls_at_geometry() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let border = centered_border();
    border.set_background(None);

    s.set_content(&border);

    s.assert_hit_test_geometry(&rect_geometry(100.0, 100.0, 50.0, 50.0), None, &[]);
}

fn invisible_border_with_visible_child() -> Ref<Border> {
    let border = centered_border();
    border.set_is_visible(false);
    border.set_child(stretched_border());
    border
}

#[test]
fn hit_test_should_not_find_invisible_controls_at_point() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let border = invisible_border_with_visible_child();
    s.set_content(&border);

    s.assert_hit_test(Point::new(100.0, 100.0), None, &[]);
}

#[test]
fn hit_test_should_not_find_invisible_controls_at_geometry() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let border = invisible_border_with_visible_child();
    s.set_content(&border);

    s.assert_hit_test_geometry(&rect_geometry(100.0, 100.0, 50.0, 50.0), None, &[]);
}

fn zero_opacity_tree(parent: bool, child: bool) -> (Ref<Border>, Ref<Border>) {
    let border = centered_border();
    border.set_opacity(if parent { 0.0 } else { 1.0 });
    let visible = stretched_border();
    visible.set_opacity(if child { 0.0 } else { 1.0 });
    border.set_child(visible.clone());
    (border, visible)
}

#[test]
fn hit_test_should_find_zero_opacity_controls_at_point() {
    for (parent, child) in [(false, false), (true, false), (false, true), (true, true)] {
        let s = CompositorTestServices::new(Size::new(200.0, 200.0));
        let (border, visible) = zero_opacity_tree(parent, child);
        s.set_content(&border);

        s.assert_hit_test(Point::new(100.0, 100.0), None, &[v(&visible), v(&border)]);
    }
}

#[test]
fn hit_test_should_find_zero_opacity_controls_at_geometry() {
    for (parent, child) in [(false, false), (true, false), (false, true), (true, true)] {
        let s = CompositorTestServices::new(Size::new(200.0, 200.0));
        let (border, visible) = zero_opacity_tree(parent, child);
        s.set_content(&border);

        s.assert_hit_test_geometry(
            &rect_geometry(100.0, 100.0, 50.0, 50.0),
            None,
            &[ghr(&visible, IntersectionResult::FullyContains), ghr(&border, IntersectionResult::FullyContains)],
        );
    }
}

#[test]
fn hit_test_should_not_find_control_outside_point() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let border = centered_border();
    s.set_content(&border);

    s.assert_hit_test(Point::new(10.0, 10.0), None, &[]);
}

#[test]
fn hit_test_should_not_find_control_outside_geometry() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let border = centered_border();
    s.set_content(&border);

    s.assert_hit_test_geometry(&rect_geometry(10.0, 10.0, 10.0, 10.0), None, &[]);
}

fn two_centered_borders() -> Ref<Panel> {
    let container = Panel::new();
    container.set_width(200.0);
    container.set_height(200.0);
    container.children().add(border(100.0, 100.0, HorizontalAlignment::Center, VerticalAlignment::Center));
    let blue = border(50.0, 50.0, HorizontalAlignment::Center, VerticalAlignment::Center);
    blue.set_background(Some(Brushes::blue()));
    container.children().add(blue);
    container
}

#[test]
fn hit_test_should_return_top_controls_first() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let container = two_centered_borders();
    s.set_content(&container);

    s.assert_hit_test(
        Point::new(100.0, 100.0),
        None,
        &[v(&container.children().get(1)), v(&container.children().get(0))],
    );
}

#[test]
fn hit_test_geometry_should_return_top_controls_first() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let container = two_centered_borders();
    s.set_content(&container);

    s.assert_hit_test_geometry(
        &rect_geometry(100.0, 100.0, 50.0, 50.0),
        None,
        &[
            ghr(&container.children().get(1), IntersectionResult::Intersects),
            ghr(&container.children().get(0), IntersectionResult::FullyContains),
        ],
    );
}

fn z_indexed_borders() -> Ref<Panel> {
    let container = Panel::new();
    container.set_width(200.0);
    container.set_height(200.0);
    let first = border(100.0, 100.0, HorizontalAlignment::Center, VerticalAlignment::Center);
    first.set_z_index(1);
    container.children().add(first);
    container.children().add(border(50.0, 50.0, HorizontalAlignment::Center, VerticalAlignment::Center));
    let third = border(75.0, 75.0, HorizontalAlignment::Center, VerticalAlignment::Center);
    third.set_z_index(2);
    container.children().add(third);
    container
}

#[test]
fn hit_test_should_return_top_controls_first_with_z_index() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let container = z_indexed_borders();
    s.set_content(&container);

    s.assert_hit_test(
        Point::new(100.0, 100.0),
        None,
        &[v(&container.children().get(2)), v(&container.children().get(0)), v(&container.children().get(1))],
    );
}

#[test]
fn hit_test_geometry_should_return_top_controls_first_with_z_index() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let container = z_indexed_borders();
    s.set_content(&container);

    s.assert_hit_test_geometry(
        &rect_geometry(100.0, 100.0, 50.0, 50.0),
        None,
        &[
            ghr(&container.children().get(2), IntersectionResult::Intersects),
            ghr(&container.children().get(0), IntersectionResult::FullyContains),
            ghr(&container.children().get(1), IntersectionResult::Intersects),
        ],
    );
}

fn translated_outside_parent() -> (Ref<Panel>, Ref<Border>) {
    let container = Panel::new();
    container.set_width(200.0);
    container.set_height(200.0);
    container.set_background(Some(Brushes::red()));
    container.set_clip_to_bounds(false);
    let outer = border(100.0, 100.0, HorizontalAlignment::Left, VerticalAlignment::Top);
    outer.set_z_index(1);
    let target = border(50.0, 50.0, HorizontalAlignment::Left, VerticalAlignment::Top);
    target.set_render_transform(Some(TranslateTransform::with_offset(110.0, 110.0).into()));
    outer.set_child(target.clone());
    container.children().add(outer);
    (container, target)
}

#[test]
fn hit_test_should_find_control_translated_outside_parent_bounds() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let (container, target) = translated_outside_parent();
    s.set_content(&container);

    s.assert_hit_test(Point::new(120.0, 120.0), None, &[v(&target), v(&container)]);
}

#[test]
fn hit_test_geometry_find_control_translated_outside_parent_bounds() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let (container, target) = translated_outside_parent();
    s.set_content(&container);

    s.assert_hit_test_geometry(
        &rect_geometry(120.0, 120.0, 50.0, 50.0),
        None,
        &[ghr(&target, IntersectionResult::Intersects), ghr(&container, IntersectionResult::FullyContains)],
    );
}

fn clipped_tree() -> (Ref<Panel>, Ref<Border>) {
    let container = Panel::new();
    container.set_width(100.0);
    container.set_height(200.0);
    container.set_background(Some(Brushes::red()));
    let clipped = Panel::new();
    clipped.set_width(100.0);
    clipped.set_height(100.0);
    clipped.set_background(Some(Brushes::red()));
    clipped.set_margin(Thickness::new(0.0, 100.0, 0.0, 0.0));
    clipped.set_clip_to_bounds(true);
    let target = Border::new();
    target.set_width(100.0);
    target.set_height(100.0);
    target.set_background(Some(Brushes::red()));
    target.set_margin(Thickness::new(0.0, -100.0, 0.0, 0.0));
    clipped.children().add(target.clone());
    container.children().add(clipped);
    (container, target)
}

#[test]
fn hit_test_should_not_find_control_outside_parent_bounds_when_clipped() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let (container, _target) = clipped_tree();
    s.set_content(&container);

    s.assert_hit_test(Point::new(50.0, 50.0), None, &[v(&container)]);
}

#[test]
fn hit_test_geometry_should_not_find_control_outside_parent_bounds_when_clipped() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let (container, _target) = clipped_tree();
    s.set_content(&container);

    s.assert_hit_test_geometry(
        &rect_geometry(50.0, 50.0, 50.0, 50.0),
        None,
        &[ghr(&container, IntersectionResult::FullyContains)],
    );
}

struct ScrollTree {
    container: Ref<Panel>,
    target: Ref<Border>,
    item1: Ref<Border>,
    item2: Ref<Border>,
    scroll: Ref<ScrollContentPresenter>,
}

fn named_border(name: &str) -> Ref<Border> {
    let border = Border::new();
    border.set_name(Some(name.to_string()));
    border.set_width(100.0);
    border.set_height(100.0);
    border.set_background(Some(Brushes::red()));
    border
}

fn scroll_tree() -> ScrollTree {
    let container = Panel::new();
    container.set_width(100.0);
    container.set_height(200.0);
    container.set_background(Some(Brushes::red()));

    let target = named_border("b1");
    container.children().add(target.clone());

    let b2 = named_border("b2");
    b2.set_margin(Thickness::new(0.0, 100.0, 0.0, 0.0));
    let scroll = ScrollContentPresenter::new();
    scroll.set_can_horizontally_scroll(true);
    scroll.set_can_vertically_scroll(true);
    let stack = StackPanel::new();
    let item1 = named_border("b3");
    let item2 = named_border("b4");
    stack.children().add(item1.clone());
    stack.children().add(item2.clone());
    scroll.set_content(Some(Control::boxed(stack)));
    b2.set_child(scroll.clone());
    container.children().add(b2);

    ScrollTree { container, target, item1, item2, scroll }
}

#[test]
fn hit_test_should_not_find_control_outside_scroll_viewport() {
    let s = CompositorTestServices::new(Size::new(100.0, 200.0));
    let t = scroll_tree();
    s.set_content(&t.container);

    t.scroll.update_child();

    s.assert_hit_test_first(Point::new(50.0, 150.0), None, Some(&t.item1));

    s.assert_hit_test_first(Point::new(50.0, 50.0), None, Some(&t.target));

    t.scroll.set_offset(Vector::new(0.0, 100.0));

    s.assert_hit_test_first(Point::new(50.0, 150.0), None, Some(&t.item2));

    s.assert_hit_test_first(Point::new(50.0, 50.0), None, Some(&t.target));
}

#[test]
fn hit_test_geometry_should_not_find_control_outside_scroll_viewport() {
    let s = CompositorTestServices::new(Size::new(100.0, 200.0));
    let t = scroll_tree();
    s.set_content(&t.container);

    t.scroll.update_child();

    s.assert_hit_test_first_geometry(&rect_geometry(50.0, 150.0, 50.0, 50.0), None, Some(&t.item1));

    s.assert_hit_test_first_geometry(&rect_geometry(50.0, 50.0, 50.0, 50.0), None, Some(&t.target));

    t.scroll.set_offset(Vector::new(0.0, 100.0));

    s.assert_hit_test_first_geometry(&rect_geometry(50.0, 150.0, 50.0, 50.0), None, Some(&t.item2));

    s.assert_hit_test_first_geometry(&rect_geometry(50.0, 50.0, 50.0, 50.0), None, Some(&t.target));
}

#[test]
fn hit_test_should_not_find_path_when_outside_fill() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let path = Path::new();
    path.set_width(200.0);
    path.set_height(200.0);
    path.set_fill(Some(Brushes::red()));
    path.set_data(StreamGeometry::parse("M100,0 L0,100 100,100").expect("the path data is valid"));
    s.set_content(&path);

    s.assert_hit_test(Point::new(100.0, 100.0), None, &[v(&path)]);
    s.assert_hit_test(Point::new(10.0, 10.0), None, &[]);
}

#[test]
fn hit_test_geometry_should_not_find_path_when_outside_fill() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let path = Path::new();
    path.set_width(200.0);
    path.set_height(200.0);
    path.set_fill(Some(Brushes::red()));
    path.set_data(rect_geometry(50.0, 50.0, 100.0, 100.0));
    s.set_content(&path);

    s.assert_hit_test_geometry(
        &rect_geometry(95.0, 95.0, 50.0, 50.0),
        None,
        &[ghr(&path, IntersectionResult::FullyContains)],
    );
    s.assert_hit_test_geometry(&rect_geometry(0.0, 0.0, 10.0, 10.0), None, &[]);
}

fn clipped_border_with_canvas() -> (Ref<Border>, Ref<Canvas>) {
    let border = Border::new();
    border.set_background(Some(Brushes::red()));
    border.set_clip(StreamGeometry::parse("M100,0 L0,100 100,100").expect("the clip data is valid"));
    border.set_width(200.0);
    border.set_height(200.0);
    let canvas = Canvas::new();
    canvas.set_background(Some(Brushes::yellow()));
    canvas.set_margin(Thickness::uniform(10.0));
    border.set_child(canvas.clone());
    (border, canvas)
}

#[test]
fn hit_test_should_respect_geometry_clip() {
    let s = CompositorTestServices::new(Size::new(400.0, 400.0));
    let (border, canvas) = clipped_border_with_canvas();
    s.set_content(&border);

    s.run_jobs();
    assert_eq!(Rect::new(100.0, 100.0, 200.0, 200.0), border.bounds());

    s.assert_hit_test(Point::new(200.0, 200.0), None, &[v(&canvas), v(&border)]);

    s.assert_hit_test(Point::new(110.0, 110.0), None, &[]);
}

#[test]
fn hit_test_geometry_should_respect_geometry_clip() {
    let s = CompositorTestServices::new(Size::new(400.0, 400.0));
    let (border, canvas) = clipped_border_with_canvas();
    s.set_content(&border);

    s.run_jobs();
    assert_eq!(Rect::new(100.0, 100.0, 200.0, 200.0), border.bounds());

    s.assert_hit_test_geometry(
        &rect_geometry(195.0, 195.0, 10.0, 10.0),
        None,
        &[ghr(&canvas, IntersectionResult::FullyContains), ghr(&border, IntersectionResult::FullyContains)],
    );

    s.assert_hit_test_geometry(&rect_geometry(110.0, 110.0, 10.0, 10.0), None, &[]);
}

fn centered_custom_hit_test_border() -> Ref<CustomHitTestBorder> {
    let border = CustomHitTestBorder::new();
    border.set_clip_to_bounds(false);
    border.set_width(100.0);
    border.set_height(100.0);
    border.set_background(Some(Brushes::red()));
    border.set_horizontal_alignment(HorizontalAlignment::Center);
    border.set_vertical_alignment(VerticalAlignment::Center);
    border
}

#[test]
fn hit_test_should_accommodate_i_custom_hit_test() {
    let s = CompositorTestServices::new(Size::new(300.0, 200.0));
    let border = centered_custom_hit_test_border();

    s.set_content(&border);

    s.assert_hit_test(Point::new(75.0, 100.0), None, &[v(&border)]);
    s.assert_hit_test(Point::new(125.0, 100.0), None, &[v(&border)]);
    s.assert_hit_test(Point::new(175.0, 100.0), None, &[]);
}

#[test]
fn hit_test_geometry_should_accommodate_i_custom_hit_test() {
    let s = CompositorTestServices::new(Size::new(300.0, 200.0));
    let border = centered_custom_hit_test_border();

    s.set_content(&border);

    s.assert_hit_test_geometry(
        &rect_geometry(75.0, 100.0, 10.0, 10.0),
        None,
        &[ghr(&border, IntersectionResult::FullyContains)],
    );
    s.assert_hit_test_geometry(
        &rect_geometry(125.0, 45.0, 10.0, 10.0),
        None,
        &[ghr(&border, IntersectionResult::Intersects)],
    );
    s.assert_hit_test_geometry(&rect_geometry(175.0, 100.0, 10.0, 10.0), None, &[]);
}

#[test]
fn removing_i_custom_hit_test_child_should_restore_sub_tree_bounds_optimization() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let custom = CustomHitTestBorder::new();
    custom.set_width(100.0);
    custom.set_height(100.0);
    custom.set_background(Some(Brushes::red()));
    let container = Panel::new();
    container.set_width(200.0);
    container.set_height(200.0);
    container.children().add(custom.clone());

    s.set_content(&container);
    s.run_jobs();

    let container_visual = ElementComposition::get_element_visual(&container);
    assert!(container_visual.is_some());
    let container_visual = container_visual.unwrap();

    // While the subtree contains an ICustomHitTest visual, the bounds-based optimization must be disabled.
    assert!(container_visual.disable_sub_tree_bounds_hit_test_optimization());

    container.children().remove(&custom);
    s.run_jobs();

    // Once the custom hit test visual leaves the subtree, the accounting must return to zero and re-enable
    // the optimization. A sign bug when attaching to a new parent leaves the count stuck at a non-zero value.
    assert!(!container_visual.disable_sub_tree_bounds_hit_test_optimization());
}

fn next_pixel_stack_panel(target_name: Option<&str>) -> (Ref<StackPanel>, Ref<Border>) {
    let stack_panel = StackPanel::new();
    stack_panel.set_orientation(Orientation::Vertical);
    stack_panel.set_horizontal_alignment(HorizontalAlignment::Left);
    let first = Border::new();
    first.set_width(10.0);
    first.set_height(10.0);
    first.set_background(Some(Brushes::red()));
    stack_panel.children().add(first);
    let target_rectangle = Border::new();
    target_rectangle.set_width(10.0);
    target_rectangle.set_height(10.0);
    target_rectangle.set_background(Some(Brushes::green()));
    if let Some(name) = target_name {
        target_rectangle.set_name(Some(name.to_string()));
    }
    stack_panel.children().add(target_rectangle.clone());
    (stack_panel, target_rectangle)
}

#[test]
fn hit_test_should_not_hit_controls_next_pixel() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let (stack_panel, target_rectangle) = next_pixel_stack_panel(None);

    s.set_content(&stack_panel);

    s.assert_hit_test(Point::new(5.0, 10.0), None, &[v(&target_rectangle)]);
}

#[test]
fn hit_test_geometry_should_not_hit_controls_next_pixel() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let (stack_panel, target_rectangle) = next_pixel_stack_panel(Some("Target"));

    s.set_content(&stack_panel);

    s.assert_hit_test_geometry(
        &rect_geometry(5.0, 10.0, 10.0, 10.0),
        None,
        &[ghr(&target_rectangle, IntersectionResult::Intersects)],
    );
}

fn parent_with_stretched_child() -> (Ref<Border>, Ref<Border>) {
    let parent = centered_border();
    let child = stretched_border();
    parent.set_child(child.clone());
    (parent, child)
}

#[test]
fn hit_test_filter_should_filter_out_children() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let (parent, child) = parent_with_stretched_child();
    s.set_content(&parent);

    s.assert_hit_test(Point::new(100.0, 100.0), None, &[v(&child), v(&parent)]);
    s.assert_hit_test(Point::new(100.0, 100.0), Some(&is_not(&parent)), &[]);
}

#[test]
fn hit_test_geometry_filter_should_filter_out_children() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let (parent, child) = parent_with_stretched_child();
    s.set_content(&parent);

    s.assert_hit_test_geometry(
        &rect_geometry(100.0, 100.0, 10.0, 10.0),
        None,
        &[ghr(&child, IntersectionResult::FullyContains), ghr(&parent, IntersectionResult::FullyContains)],
    );
    s.assert_hit_test_geometry(&rect_geometry(100.0, 100.0, 10.0, 10.0), Some(&is_not(&parent)), &[]);
}

fn full_red_border() -> Ref<Border> {
    let target = Border::new();
    target.set_width(200.0);
    target.set_height(200.0);
    target.set_background(Some(Brushes::red()));
    target
}

fn set_blue_child_visual(s: &CompositorTestServices, target: &Visual) {
    let child_visual = s.compositor().create_solid_color_visual();
    child_visual.set_size(Vector::new(200.0, 200.0));
    child_visual.set_color(Colors::BLUE);
    ElementComposition::set_element_child_visual(target, Some((*child_visual).clone()));
}

#[test]
fn hit_test_first_should_skip_element_child_composition_visual() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let target = full_red_border();

    s.set_content(&target);
    s.run_jobs();

    set_blue_child_visual(&s, &target);

    s.assert_hit_test_first(Point::new(100.0, 100.0), None, Some(&target));
}

#[test]
fn hit_test_first_geometry_should_skip_element_child_composition_visual() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let target = full_red_border();

    s.set_content(&target);
    s.run_jobs();

    set_blue_child_visual(&s, &target);

    s.assert_hit_test_first_geometry(&rect_geometry(100.0, 100.0, 10.0, 10.0), None, Some(&target));
}

/// A canvas with 70 small borders in a row; returns the canvas and the
/// border at `index`.
fn canvas_with_row(index: usize, names: bool) -> (Ref<Canvas>, Ref<Border>) {
    let canvas = Canvas::new();
    canvas.set_width(1000.0);
    canvas.set_height(200.0);
    let mut found = None;
    for i in 0..70 {
        let child = Border::new();
        child.set_width(8.0);
        child.set_height(8.0);
        child.set_background(Some(Brushes::red()));
        if names {
            child.set_name(Some(format!("{i}")));
        }
        Canvas::set_left(&child, (i * 12) as f64);
        canvas.children().add(child.clone());

        if i == index {
            found = Some(child);
        }
    }
    (canvas, found.expect("the index is in range"))
}

/// A canvas with 70 overlapping borders; returns the canvas and the topmost
/// one.
fn canvas_with_overlapping() -> (Ref<Canvas>, Ref<Border>) {
    let canvas = Canvas::new();
    canvas.set_width(200.0);
    canvas.set_height(200.0);
    let mut top = None;
    for i in 0..70 {
        let child = Border::new();
        child.set_width(100.0);
        child.set_height(100.0);
        child.set_background(Some(Brushes::red()));
        Canvas::set_left(&child, 50.0);
        Canvas::set_top(&child, 50.0);
        canvas.children().add(child.clone());

        if i == 69 {
            top = Some(child);
        }
    }
    (canvas, top.expect("the canvas has 70 children"))
}

#[test]
fn hit_test_should_find_control_with_many_siblings() {
    let s = CompositorTestServices::new(Size::new(1000.0, 200.0));
    let (canvas, target) = canvas_with_row(0, false);

    s.set_content(&canvas);
    s.assert_hit_test_first(Point::new(4.0, 4.0), None, Some(&target));
}

#[test]
fn hit_test_geometry_should_find_control_with_many_siblings() {
    let s = CompositorTestServices::new(Size::new(1000.0, 200.0));
    let (canvas, target) = canvas_with_row(0, true);

    s.set_content(&canvas);
    s.assert_hit_test_first_geometry(&rect_geometry(4.0, 4.0, 2.0, 2.0), None, Some(&target));
}

#[test]
fn hit_test_should_return_top_controls_first_with_many_overlapping_siblings() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let (canvas, top) = canvas_with_overlapping();

    s.set_content(&canvas);
    s.assert_hit_test_first(Point::new(100.0, 100.0), None, Some(&top));
    let expected: Vec<Ref<Visual>> = canvas.children().to_vec().iter().rev().map(|x| v(x)).collect();
    s.assert_hit_test(Point::new(100.0, 100.0), None, &expected);
}

#[test]
fn hit_test_geometry_should_return_top_controls_first_with_many_overlapping_siblings() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let (canvas, top) = canvas_with_overlapping();

    s.set_content(&canvas);
    s.assert_hit_test_first_geometry(&rect_geometry(100.0, 100.0, 10.0, 10.0), None, Some(&top));
    let expected: Vec<GeometryHitTestResult> = canvas
        .children()
        .to_vec()
        .iter()
        .rev()
        .map(|x| ghr(x, IntersectionResult::FullyContains))
        .collect();
    s.assert_hit_test_geometry(&rect_geometry(100.0, 100.0, 10.0, 10.0), None, &expected);
}

#[test]
fn hit_test_should_update_many_sibling_index_when_child_moves() {
    let s = CompositorTestServices::new(Size::new(1000.0, 200.0));
    let (canvas, moving) = canvas_with_row(69, false);

    s.set_content(&canvas);
    s.assert_hit_test_first(Point::new((69 * 12 + 4) as f64, 4.0), None, Some(&moving));

    Canvas::set_left(&moving, 10.0);
    Canvas::set_top(&moving, 100.0);
    s.assert_hit_test_first(Point::new(14.0, 104.0), None, Some(&moving));
}

#[test]
fn hit_test_geometry_should_update_many_sibling_index_when_child_moves() {
    let s = CompositorTestServices::new(Size::new(1000.0, 200.0));
    let (canvas, moving) = canvas_with_row(69, false);

    s.set_content(&canvas);
    s.assert_hit_test_first_geometry(&rect_geometry((69 * 12 + 4) as f64, 4.0, 2.0, 2.0), None, Some(&moving));

    Canvas::set_left(&moving, 10.0);
    Canvas::set_top(&moving, 100.0);
    s.assert_hit_test_first_geometry(&rect_geometry(14.0, 104.0, 2.0, 2.0), None, Some(&moving));
}

fn added_blue_border() -> Ref<Border> {
    let added = Border::new();
    added.set_width(100.0);
    added.set_height(100.0);
    added.set_background(Some(Brushes::blue()));
    Canvas::set_left(&added, 50.0);
    Canvas::set_top(&added, 50.0);
    added
}

#[test]
fn hit_test_should_update_many_sibling_index_when_child_is_added_and_removed() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let (canvas, top) = canvas_with_overlapping();

    s.set_content(&canvas);
    s.assert_hit_test_first(Point::new(100.0, 100.0), None, Some(&top));

    let added = added_blue_border();
    canvas.children().add(added.clone());

    s.assert_hit_test_first(Point::new(100.0, 100.0), None, Some(&added));

    canvas.children().remove(&added);
    s.assert_hit_test_first(Point::new(100.0, 100.0), None, Some(&top));
}

#[test]
fn hit_test_geometry_should_update_many_sibling_index_when_child_is_added_and_removed() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let (canvas, top) = canvas_with_overlapping();

    s.set_content(&canvas);
    s.assert_hit_test_first_geometry(&rect_geometry(100.0, 100.0, 10.0, 10.0), None, Some(&top));

    let added = added_blue_border();
    canvas.children().add(added.clone());

    s.assert_hit_test_first_geometry(&rect_geometry(100.0, 100.0, 10.0, 10.0), None, Some(&added));

    canvas.children().remove(&added);
    s.assert_hit_test_first_geometry(&rect_geometry(100.0, 100.0, 10.0, 10.0), None, Some(&top));
}

#[test]
fn hit_test_geometry_should_find_control_within_geometry_bounds() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let border = centered_border();

    s.set_content(&border);
    s.run_jobs();

    let geometry = RectangleGeometry::new();
    geometry.set_rect(Rect::new(80.0, 80.0, 40.0, 40.0));
    s.assert_hit_test_first_geometry(&geometry, None, Some(&border));
}

#[test]
fn hit_test_geometry_should_not_find_control_outside_geometry_bounds() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let border = centered_border();

    s.set_content(&border);
    s.run_jobs();

    let geometry = RectangleGeometry::new();
    geometry.set_rect(Rect::new(10.0, 10.0, 30.0, 30.0));
    s.assert_hit_test_first_geometry(&geometry, None, None);
}

#[test]
fn hit_test_geometry_should_find_multiple_controls_within_bounds() {
    let s = CompositorTestServices::new(Size::new(300.0, 200.0));
    let container = Panel::new();
    container.set_width(300.0);
    container.set_height(200.0);
    let border1 = border(50.0, 50.0, HorizontalAlignment::Left, VerticalAlignment::Top);
    border1.set_name(Some("border1".to_string()));
    container.children().add(border1);
    let border2 = border(50.0, 50.0, HorizontalAlignment::Left, VerticalAlignment::Center);
    border2.set_name(Some("border2".to_string()));
    border2.set_background(Some(Brushes::blue()));
    container.children().add(border2);
    s.set_content(&container);
    s.run_jobs();

    let geometry = RectangleGeometry::new();
    geometry.set_rect(Rect::new(0.0, 0.0, 100.0, 200.0));
    s.assert_hit_test_first_geometry(&geometry, None, Some(&container.children().get(1)));
}

#[test]
fn hit_test_geometry_should_respect_control_visibility() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let border = centered_border();
    let child = stretched_border();
    child.set_background(Some(Brushes::blue()));
    child.set_is_visible(false);
    border.set_child(child);
    s.set_content(&border);

    s.run_jobs();

    let geometry = RectangleGeometry::new();
    geometry.set_rect(Rect::new(50.0, 50.0, 100.0, 100.0));
    s.assert_hit_test_first_geometry(&geometry, None, Some(&border));
}

#[test]
fn hit_test_geometry_should_find_top_control_with_z_index() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let container = Panel::new();
    container.set_width(200.0);
    container.set_height(200.0);
    let first = centered_border();
    first.set_z_index(1);
    container.children().add(first);
    let second = centered_border();
    second.set_background(Some(Brushes::blue()));
    second.set_z_index(2);
    second.set_margin(Thickness::uniform(10.0));
    container.children().add(second);
    s.set_content(&container);
    s.run_jobs();

    let geometry = RectangleGeometry::new();
    geometry.set_rect(Rect::new(50.0, 50.0, 100.0, 100.0));
    s.assert_hit_test_first_geometry(&geometry, None, Some(&container.children().get(1)));
}

#[test]
fn hit_test_geometry_should_filter_results() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let parent = centered_border();
    let child = stretched_border();
    child.set_background(Some(Brushes::blue()));
    parent.set_child(child.clone());
    s.set_content(&parent);
    s.run_jobs();

    let geometry = RectangleGeometry::new();
    geometry.set_rect(Rect::new(50.0, 50.0, 100.0, 100.0));

    s.assert_hit_test_first_geometry(&geometry, None, Some(&child));
    s.assert_hit_test_first_geometry(&geometry, Some(&is_not(&parent)), None);
    s.assert_hit_test_first_geometry(&geometry, Some(&is_not(&child)), Some(&parent));
}

#[test]
fn hit_test_geometry_should_return_correct_intersection_result() {
    let cases = [
        (50.0, 50.0, 100.0, 100.0, 50.0, 50.0, IntersectionResult::FullyInside),
        (80.0, 80.0, 100.0, 100.0, 50.0, 50.0, IntersectionResult::Intersects),
        (95.0, 95.0, 40.0, 40.0, 80.0, 80.0, IntersectionResult::FullyContains),
        (0.0, 0.0, 10.0, 10.0, 50.0, 50.0, IntersectionResult::Empty),
    ];
    for (geom_x, geom_y, geom_width, geom_height, elem_width, elem_height, expected_result) in cases {
        let s = CompositorTestServices::new(Size::new(200.0, 200.0));
        let border = border(elem_width, elem_height, HorizontalAlignment::Center, VerticalAlignment::Center);

        s.set_content(&border);
        s.run_jobs();

        let geometry = rect_geometry(geom_x, geom_y, geom_width, geom_height);

        if expected_result == IntersectionResult::Empty {
            // Geometry doesn't intersect element
            s.assert_hit_test_geometry(&geometry, None, &[]);
        } else {
            // Geometry intersects element
            s.assert_hit_test_geometry(&geometry, None, &[ghr(&border, expected_result)]);
        }
    }
}

#[test]
fn hit_test_geometry_should_handle_line_target() {
    let s = CompositorTestServices::new(Size::new(200.0, 200.0));
    let line = Line::new();
    line.set_stroke(Some(Brushes::red()));
    line.set_stroke_thickness(4.0);
    line.set_start_point(Point::new(5.0, 5.0));
    line.set_end_point(Point::new(190.0, 5.0));

    s.set_content(&line);

    s.assert_hit_test_geometry(
        &rect_geometry(0.0, 0.0, 50.0, 50.0),
        None,
        &[ghr(&line, IntersectionResult::Intersects)],
    );
}
