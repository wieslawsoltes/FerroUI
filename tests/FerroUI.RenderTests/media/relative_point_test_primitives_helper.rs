//! Port of upstream's `Media/RelativePointTestPrimitivesHelper.cs`: a
//! control that draws a rectangle, an ellipse, a line and a geometry with
//! one brush, which the brush tests use to see how a brush is mapped to
//! each primitive.
//!
//! Upstream's geometry is a static field; an object of the port belongs to
//! a thread, so the port has one for each thread.

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{BoxShadows, Brushes, DrawingContext, Geometry, IBrush, IPen, Pen};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Rect, Ref, StyledElementImpl, VisualImpl,
    VisualImplExt,
};
use ferroui_controls::{Control, ControlImpl};
use std::rc::Rc;

thread_local! {
    static S_GEOMETRY: Ref<Geometry> =
        Geometry::parse("m 80 200 c 40 20 150 -40 160 0 l 0 30 c -40 -30 -160 10 -160 -30 z").expect("the path data is valid");
}

#[repr(C)]
pub struct RelativePointTestPrimitivesHelper {
    base: Control,
    brush: Option<Rc<dyn IBrush>>,
    shadow: bool,
    line: Option<Rc<dyn IPen>>,
}

ferro_class!(RelativePointTestPrimitivesHelper: Control);
ferro_impl_classes!(
    RelativePointTestPrimitivesHelper: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl RelativePointTestPrimitivesHelper {
    /// The constructor with upstream's default of `shadow`, false.
    pub fn new(brush: Option<Rc<dyn IBrush>>) -> Ref<Self> {
        Self::with_shadow(brush, false)
    }

    pub fn with_shadow(brush: Option<Rc<dyn IBrush>>, shadow: bool) -> Ref<Self> {
        let line: Option<Rc<dyn IPen>> = brush.as_ref().map(|brush| Pen::with_brush(Some(brush.clone()), 10.0).into());
        let this = instantiate(Self { base: Control::construct(), brush, shadow, line });

        this.set_width(256.0);
        this.set_max_width(256.0);
        this.set_min_width(256.0);
        this.set_height(256.0);
        this.set_max_height(256.0);
        this.set_min_height(256.0);
        this
    }
}

impl VisualImpl for RelativePointTestPrimitivesHelper {
    fn render(this: &Self, context: &mut DrawingContext) {
        let brush = this.brush.as_ref();

        if this.shadow {
            let full = Rect::from_size(this.bounds().size());
            let white: Rc<dyn IBrush> = Brushes::white();
            context.draw_rectangle(Some(&white), None, full, 0.0, 0.0, &BoxShadows::default());
            let opacity = context.push_opacity(0.3);
            context.draw_rectangle(brush, None, full, 0.0, 0.0, &BoxShadows::default());
            context.pop(opacity);
        }

        context.draw_rectangle(brush, None, Rect::new(20.0, 20.0, 200.0, 60.0), 0.0, 0.0, &BoxShadows::default());
        context.draw_ellipse(brush, None, Rect::new(40.0, 100.0, 200.0, 20.0));
        if let Some(line) = this.line.as_ref() {
            context.draw_line(line, Point::new(60.0, 140.0), Point::new(240.0, 160.0));
        }
        S_GEOMETRY.with(|geometry| context.draw_geometry(brush, None, geometry));

        Self::parent_render(this, context);
    }
}
