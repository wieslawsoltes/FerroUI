//! Port of upstream's `Media/ImageDrawingTests.cs`.
//!
//! Upstream loads the source image from its output directory, which is
//! also the directory of its expected images; the port loads it from the
//! directory of the expected images.

use crate::test_base::TestBase;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{
    BoxShadows, Brushes, Colors, DrawingBrush, DrawingCollection, DrawingContext, DrawingGroup, DrawingImage,
    GeometryDrawing, IBrush, ImageDrawing, RectangleGeometry, SolidColorBrush, StreamGeometry, TileMode,
    TranslateTransform,
};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Matrix, Rect, Ref, RelativeRect, RelativeUnit,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::{Border, Control, ControlImpl, Decorator, Image};
use std::rc::Rc;

fn base() -> TestBase {
    TestBase::new(r"Media\ImageDrawing")
}

fn bitmap_path(t: &TestBase) -> String {
    t.expected_path().join("github_icon.png").to_str().expect("the path is text").to_string()
}

#[test]
fn image_drawing_fill() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Image::new();
    let source = DrawingImage::new();
    let drawing = ImageDrawing::new();
    drawing.set_image_source(Some(Rc::new(Bitmap::from_file(&bitmap_path(&t)).unwrap())));
    drawing.set_rect(Rect::new(0.0, 0.0, 200.0, 200.0));
    source.set_drawing(drawing);
    child.set_source(Some(source.into()));
    target.set_child(child);

    t.render_to_file(&target, "ImageDrawing_Fill");
    t.compare_images("ImageDrawing_Fill");
}

#[test]
fn image_drawing_viewbox() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Image::new();
    let source = DrawingImage::new();
    source.set_viewbox(Some(Rect::new(48.0, 37.0, 100.0, 125.0)));
    let drawing = ImageDrawing::new();
    drawing.set_image_source(Some(Rc::new(Bitmap::from_file(&bitmap_path(&t)).unwrap())));
    drawing.set_rect(Rect::new(0.0, 0.0, 200.0, 200.0));
    source.set_drawing(drawing);
    child.set_source(Some(source.into()));
    target.set_child(child);

    t.render_to_file(&target, "ImageDrawing_Viewbox");
    t.compare_images("ImageDrawing_Viewbox");
}

#[test]
fn image_drawing_bottom_right() {
    let t = base();
    let target = Decorator::new();
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Image::new();
    let source = DrawingImage::new();
    let group = DrawingGroup::new();
    let geometry_drawing = GeometryDrawing::new();
    geometry_drawing.set_geometry(StreamGeometry::parse("m0,0 l200,200").unwrap());
    geometry_drawing.set_brush(Some(Brushes::black()));
    group.children().add(geometry_drawing.upcast());
    let image_drawing = ImageDrawing::new();
    image_drawing.set_image_source(Some(Rc::new(Bitmap::from_file(&bitmap_path(&t)).unwrap())));
    image_drawing.set_rect(Rect::new(100.0, 100.0, 100.0, 100.0));
    group.children().add(image_drawing.upcast());
    source.set_drawing(group);
    child.set_source(Some(source.into()));
    target.set_child(child);

    t.render_to_file(&target, "ImageDrawing_BottomRight");
    t.compare_images("ImageDrawing_BottomRight");
}

#[test]
fn should_render_drawing_brush_transform() {
    let t = base();
    let target = Border::new();
    target.set_width(400.0);
    target.set_height(400.0);
    target.set_child(DrawingBrushTransformTest::new());

    t.render_to_file(&target, "Should_Render_DrawingBrushTransform");
    t.compare_images("Should_Render_DrawingBrushTransform");
}

#[repr(C)]
pub struct DrawingBrushTransformTest {
    base: Control,
    brush: Rc<dyn IBrush>,
}

ferro_class!(DrawingBrushTransformTest: Control);
ferro_impl_classes!(
    DrawingBrushTransformTest: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl DrawingBrushTransformTest {
    pub fn new() -> Ref<Self> {
        let brush = DrawingBrush::new();
        brush.set_tile_mode(TileMode::None);
        brush.set_source_rect(RelativeRect::new(0.0, 0.0, 1.0, 1.0, RelativeUnit::Relative));
        brush.set_destination_rect(RelativeRect::new(0.0, 0.0, 50.0, 50.0, RelativeUnit::Absolute));
        brush.set_transform(Some(TranslateTransform::with_offset(150.0, 150.0).into()));
        let group = DrawingGroup::new();
        let children = DrawingCollection::new();
        let drawing = GeometryDrawing::new();
        drawing.set_brush(Some(Brushes::crimson()));
        drawing.set_geometry(RectangleGeometry::with_rect(Rect::new(0.0, 0.0, 100.0, 100.0)));
        children.add(drawing.upcast());
        let drawing = GeometryDrawing::new();
        drawing.set_brush(Some(Brushes::blue()));
        drawing.set_geometry(RectangleGeometry::with_rect(Rect::new(20.0, 20.0, 60.0, 60.0)));
        children.add(drawing.upcast());
        group.set_children(children);
        brush.set_drawing(group);

        instantiate(Self { base: Control::construct(), brush: brush.into() })
    }
}

impl VisualImpl for DrawingBrushTransformTest {
    fn render(this: &Self, drawing_context: &mut DrawingContext) {
        let pop = drawing_context.push_transform(Matrix::create_translation(100.0, 100.0));
        let rc = Rect::new(0.0, 0.0, 200.0, 200.0);
        let dim_gray: Rc<dyn IBrush> = SolidColorBrush::with_color(Colors::DIM_GRAY).into();
        drawing_context.draw_rectangle(Some(&dim_gray), None, rc, 0.0, 0.0, &BoxShadows::default());
        drawing_context.draw_rectangle(Some(&this.brush), None, rc, 0.0, 0.0, &BoxShadows::default());

        drawing_context.pop(pop);
    }
}
