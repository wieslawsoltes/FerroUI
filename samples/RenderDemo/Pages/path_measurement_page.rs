//! Port of `Pages/PathMeasurementPage.cs`.

use ferroui_base::StyledElementImplExt;
use ferroui_base::VisualImplExt;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::media::immutable::ImmutablePen;
use ferroui_base::media::{
    BoxShadows, Brushes, DrawingContext, IImmutableBrush, IPen, PathGeometry, PenLineCap, PenLineJoin,
};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, PixelSize, Point, Rect, Ref,
    StyledElementImpl, Vector, Visual, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct PathMeasurementPage {
    base: Control,
    bitmap: RefCell<Option<RenderTargetBitmap>>,
    stroke_pen: Rc<dyn IPen>,
    stroke_pen1: Rc<dyn IPen>,
    stroke_pen2: Rc<dyn IPen>,
    stroke_pen3: Rc<dyn IPen>,
    stroke_pen4: Rc<dyn IPen>,
}

ferro_class!(PathMeasurementPage: Control);
ferro_impl_classes!(PathMeasurementPage: FerroObjectImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);
ferro_class_info!(PathMeasurementPage { new: PathMeasurementPage::new });

impl StyledElementImpl for PathMeasurementPage {
    fn on_attached_to_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        *this.bitmap.borrow_mut() = Some(RenderTargetBitmap::with_dpi(PixelSize::new(500, 500), Vector::new(96.0, 96.0)));
        Self::parent_on_attached_to_logical_tree(this, e);
    }

    fn on_detached_from_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        let bitmap = this.bitmap.borrow_mut().take();
        if let Some(bitmap) = bitmap {
            bitmap.dispose();
        }
        Self::parent_on_detached_from_logical_tree(this, e);
    }
}

impl VisualImpl for PathMeasurementPage {
    fn render(this: &Self, context: &mut DrawingContext) {
        let bitmap = this.bitmap.borrow();
        let Some(bitmap) = bitmap.as_ref() else {
            return;
        };

        {
            let mut bitmap_ctx = bitmap.create_drawing_context();

            let base_path = PathGeometry::new();

            {
                let mut base_path_ctx = base_path.open();
                base_path_ctx.begin_figure(Point::new(20.0, 20.0), false);
                base_path_ctx.line_to(Point::new(400.0, 50.0), true);
                base_path_ctx.line_to(Point::new(80.0, 100.0), true);
                base_path_ctx.line_to(Point::new(300.0, 150.0), true);
                base_path_ctx.end_figure(false);
                base_path_ctx.dispose();
            }

            bitmap_ctx.draw_geometry(None, Some(&this.stroke_pen), &base_path.clone().upcast());

            let length = base_path.contour_length();

            if let Some(dst1) = base_path.try_get_segment(length * 0.05, length * 0.2, true) {
                bitmap_ctx.draw_geometry(None, Some(&this.stroke_pen1), &dst1);
            }

            if let Some(dst2) = base_path.try_get_segment(length * 0.2, length * 0.8, true) {
                bitmap_ctx.draw_geometry(None, Some(&this.stroke_pen2), &dst2);
            }

            if let Some(dst3) = base_path.try_get_segment(length * 0.8, length * 0.95, true) {
                bitmap_ctx.draw_geometry(None, Some(&this.stroke_pen3), &dst3);
            }

            let path_bounds = base_path.get_render_bounds(&*this.stroke_pen);

            bitmap_ctx.draw_rectangle(None, Some(&this.stroke_pen4), path_bounds, 0.0, 0.0, &BoxShadows::default());

            bitmap_ctx.dispose();
        }

        context.draw_image_with_rects(bitmap, Rect::new(0.0, 0.0, 500.0, 500.0), Rect::new(0.0, 0.0, 500.0, 500.0));

        Self::parent_render(this, context);
    }
}

/// `new ImmutablePen(brush, thickness, null, PenLineCap.Round, PenLineJoin.Round)`.
fn round_pen(brush: Rc<dyn IImmutableBrush>, thickness: f64) -> Rc<dyn IPen> {
    Rc::new(ImmutablePen::new(Some(brush), thickness, None, PenLineCap::Round, PenLineJoin::Round, 10.0))
}

impl PathMeasurementPage {
    fn static_constructor() {
        Visual::affects_render::<PathMeasurementPage>(&[Visual::bounds_property().as_property()]);
    }

    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            bitmap: RefCell::new(None),
            stroke_pen: round_pen(Brushes::dark_blue(), 10.0),
            stroke_pen1: round_pen(Brushes::purple(), 10.0),
            stroke_pen2: round_pen(Brushes::green(), 10.0),
            stroke_pen3: round_pen(Brushes::light_blue(), 10.0),
            stroke_pen4: round_pen(Brushes::red(), 1.0),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
