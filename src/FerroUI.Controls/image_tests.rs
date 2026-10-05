use crate::test_support::{test_scope, TestRoot};
use crate::test_support_shapes::MockRenderInterfaceScope;
use crate::Image;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{DrawingImage, GeometryDrawing, IImage, RectangleGeometry, Stretch, StretchDirection};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Rect, Size};
use std::any::Any;
use std::rc::Rc;

/// An image which only has a size.
struct TestBitmap {
    size: Size,
}

impl IImage for TestBitmap {
    fn size(&self) -> Size {
        self.size
    }

    fn draw(&self, _context: &mut ferroui_base::media::DrawingContext<'_>, _source_rect: Rect, _dest_rect: Rect) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn create_bitmap(width: i32, height: i32) -> Option<Rc<dyn IImage>> {
    Some(Rc::new(TestBitmap { size: Size::new(f64::from(width), f64::from(height)) }))
}

#[test]
fn measure_should_return_correct_size_for_no_stretch() {
    let bitmap = create_bitmap(50, 100);
    let target = Image::new();
    target.set_stretch(Stretch::None);
    target.set_source(bitmap);

    target.measure(Size::new(50.0, 50.0));

    assert_eq!(Size::new(50.0, 50.0), target.desired_size());
}

#[test]
fn measure_should_return_correct_size_for_fill_stretch() {
    let bitmap = create_bitmap(50, 100);
    let target = Image::new();
    target.set_stretch(Stretch::Fill);
    target.set_source(bitmap);

    target.measure(Size::new(50.0, 50.0));

    assert_eq!(Size::new(50.0, 50.0), target.desired_size());
}

#[test]
fn measure_should_return_correct_size_for_uniform_stretch() {
    let bitmap = create_bitmap(50, 100);
    let target = Image::new();
    target.set_stretch(Stretch::Uniform);
    target.set_source(bitmap);

    target.measure(Size::new(50.0, 50.0));

    assert_eq!(Size::new(25.0, 50.0), target.desired_size());
}

#[test]
fn measure_should_return_correct_size_for_uniform_to_fill_stretch() {
    let bitmap = create_bitmap(50, 100);
    let target = Image::new();
    target.set_stretch(Stretch::UniformToFill);
    target.set_source(bitmap);

    target.measure(Size::new(50.0, 50.0));

    assert_eq!(Size::new(50.0, 50.0), target.desired_size());
}

#[test]
fn measure_should_return_correct_size_with_stretch_direction_down_only() {
    let bitmap = create_bitmap(50, 100);
    let target = Image::new();
    target.set_stretch_direction(StretchDirection::DownOnly);
    target.set_source(bitmap);

    target.measure(Size::new(150.0, 150.0));

    assert_eq!(Size::new(50.0, 100.0), target.desired_size());
}

#[test]
fn measure_should_return_correct_size_for_infinite_height() {
    let bitmap = create_bitmap(50, 100);
    let image = Image::new();
    image.set_source(bitmap);

    image.measure(Size::new(200.0, f64::INFINITY));

    assert_eq!(Size::new(200.0, 400.0), image.desired_size());
}

#[test]
fn measure_should_return_correct_size_for_infinite_width() {
    let bitmap = create_bitmap(50, 100);
    let image = Image::new();
    image.set_source(bitmap);

    image.measure(Size::new(f64::INFINITY, 400.0));

    assert_eq!(Size::new(200.0, 400.0), image.desired_size());
}

#[test]
fn measure_should_return_correct_size_for_infinite_width_height() {
    let bitmap = create_bitmap(50, 100);
    let image = Image::new();
    image.set_source(bitmap);

    image.measure(Size::new(f64::INFINITY, f64::INFINITY));

    assert_eq!(Size::new(50.0, 100.0), image.desired_size());
}

#[test]
fn arrange_should_return_correct_size_for_no_stretch() {
    let bitmap = create_bitmap(50, 100);
    let target = Image::new();
    target.set_stretch(Stretch::None);
    target.set_source(bitmap);

    target.measure(Size::new(50.0, 50.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 400.0));

    assert_eq!(Size::new(50.0, 100.0), target.bounds().size());
}

#[test]
fn arrange_should_return_correct_size_for_fill_stretch() {
    let bitmap = create_bitmap(50, 100);
    let target = Image::new();
    target.set_stretch(Stretch::Fill);
    target.set_source(bitmap);

    target.measure(Size::new(50.0, 50.0));
    target.arrange(Rect::new(0.0, 0.0, 25.0, 100.0));

    assert_eq!(Size::new(25.0, 100.0), target.bounds().size());
}

#[test]
fn arrange_should_return_correct_size_for_uniform_stretch() {
    let bitmap = create_bitmap(50, 100);
    let target = Image::new();
    target.set_stretch(Stretch::Uniform);
    target.set_source(bitmap);

    target.measure(Size::new(50.0, 50.0));
    target.arrange(Rect::new(0.0, 0.0, 25.0, 100.0));

    assert_eq!(Size::new(25.0, 50.0), target.bounds().size());
}

#[test]
fn arrange_should_return_correct_size_for_uniform_to_fill_stretch() {
    let bitmap = create_bitmap(50, 100);
    let target = Image::new();
    target.set_stretch(Stretch::UniformToFill);
    target.set_source(bitmap);

    target.measure(Size::new(50.0, 50.0));
    target.arrange(Rect::new(0.0, 0.0, 25.0, 100.0));

    assert_eq!(Size::new(25.0, 100.0), target.bounds().size());
}

#[test]
fn drawing_image_source_invalidates_measure() {
    let _scope = test_scope();
    let _app = MockRenderInterfaceScope::install();
    let drawing = GeometryDrawing::new();
    drawing.set_geometry(RectangleGeometry::with_rect(Rect::new(0.0, 0.0, 500.0, 500.0)));
    let image = DrawingImage::with_drawing(&drawing);
    let target = Image::new();
    target.set_stretch(Stretch::None);
    target.set_horizontal_alignment(HorizontalAlignment::Center);
    target.set_vertical_alignment(VerticalAlignment::Center);
    target.set_source(Some(image.into()));

    let root = TestRoot::with_child(&target);
    root.execute_initial_layout_pass();

    assert_eq!(Size::new(500.0, 500.0), target.desired_size());

    drawing.set_geometry(RectangleGeometry::with_rect(Rect::new(0.0, 0.0, 600.0, 600.0)));

    Dispatcher::ui_thread().run_jobs(None);
    root.layout_manager().execute_layout_pass();

    assert_eq!(Size::new(600.0, 600.0), target.desired_size());
}
