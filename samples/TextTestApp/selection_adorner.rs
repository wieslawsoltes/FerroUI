//! Port of `SelectionAdorner.cs`: the adorner of the line control, which draws the rectangles
//! of the selected glyphs or of the selected character hits over the line.

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{BoxShadows, DrawingContext, IBrush, IPen, Pen, PolylineGeometry};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    Matrix, Point, Rect, Ref, StyledElementImpl, StyledProperty, Visual, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct SelectionAdorner {
    base: Control,
    rectangles: RefCell<Option<Vec<Rect>>>,
}

ferro_class!(SelectionAdorner: Control);
ferro_impl_classes!(
    SelectionAdorner: StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);
ferro_class_info!(SelectionAdorner { new: SelectionAdorner::new });

ferro_properties! {
    impl SelectionAdorner {
        pub fn fill_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<SelectionAdorner, _>("Fill", None)
        }

        pub fn stroke_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<SelectionAdorner, _>("Stroke", None)
        }

        pub fn transform_property() -> StyledProperty<Matrix> {
            FerroProperty::register::<SelectionAdorner, _>("Transform", Matrix::IDENTITY)
        }
    }
}

impl FerroObjectImpl for SelectionAdorner {
    /// The body of the constructor of the managed original, which registers the properties
    /// that affect the rendering for every instance it creates.
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        Visual::affects_render::<SelectionAdorner>(&[
            Self::fill_property().as_property(),
            Self::stroke_property().as_property(),
            Self::transform_property().as_property(),
        ]);
    }
}

impl VisualImpl for SelectionAdorner {
    fn render(this: &Self, context: &mut DrawingContext) {
        let Some(rectangles) = this.rectangles() else {
            return;
        };

        let transform = context.push_transform(this.transform());
        {
            let pen: Rc<dyn IPen> = Pen::with_brush(this.stroke(), 1.0).into();
            for rectangle in &rectangles {
                let rectangle = *rectangle;
                let normalized = if rectangle.width < 0.0 {
                    Rect::from_points(rectangle.top_right(), rectangle.bottom_left())
                } else {
                    rectangle
                };

                if rectangle.width == 0.0 {
                    context.draw_line(&pen, rectangle.top_left(), rectangle.bottom_right());
                } else {
                    context.draw_rectangle(this.fill().as_ref(), Some(&pen), normalized, 0.0, 0.0, &BoxShadows::default());
                }

                this.render_cue(context, &pen, rectangle.top_left(), 5.0, true);
                this.render_cue(context, &pen, rectangle.top_right(), 5.0, false);
            }
        }
        context.pop(transform);
    }
}

impl SelectionAdorner {
    pub fn construct() -> Self {
        Self { base: Control::construct(), rectangles: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn transform(&self) -> Matrix {
        self.get_value(Self::transform_property())
    }

    pub fn set_transform(&self, value: Matrix) {
        self.set_value(Self::transform_property(), value)
    }

    pub fn stroke(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::stroke_property())
    }

    pub fn set_stroke(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::stroke_property(), value)
    }

    pub fn fill(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::fill_property())
    }

    pub fn set_fill(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::fill_property(), value)
    }

    pub fn rectangles(&self) -> Option<Vec<Rect>> {
        self.rectangles.borrow().clone()
    }

    pub fn set_rectangles(&self, value: Option<Vec<Rect>>) {
        *self.rectangles.borrow_mut() = value;
        self.invalidate_visual();
    }

    fn render_cue(&self, context: &mut DrawingContext, pen: &Rc<dyn IPen>, p: Point, size: f64, is_filled: bool) {
        context.draw_geometry(
            pen.brush().as_ref(),
            Some(pen),
            &PolylineGeometry::with_points(
                [
                    Point::new(p.x - size / 2.0, p.y - size),
                    Point::new(p.x + size / 2.0, p.y - size),
                    Point::new(p.x, p.y),
                    Point::new(p.x - size / 2.0, p.y - size),
                ],
                is_filled,
            )
            .upcast(),
        );
    }
}
