use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::media::{DashStyle, DrawingContext, IBrush, IPen, Pen, PenLineCap, PenLineJoin};
use ferroui_base::{
    FerroObjectImpl, Point, Ref, Size, StyledElementImpl, StyledProperty, Thickness, Visual,
    VisualImpl, ferro_class, ferro_impl_classes, ferro_properties, instantiate,
};
use ferroui_controls::shapes::Shape;
use ferroui_controls::{Control, ControlImpl};
use std::rc::Rc;

/// A vertical dotted separator for the editor gutter.
#[repr(C)]
pub struct DottedLineMargin {
    base: Control,
}

ferro_class!(DottedLineMargin: Control);
ferroui_base::ferro_class_info!(DottedLineMargin {
    new: DottedLineMargin::new
});
ferro_impl_classes!(DottedLineMargin: FerroObjectImpl, StyledElementImpl, InteractiveImpl,
    InputElementImpl, ControlImpl);

ferro_properties! { impl DottedLineMargin {
    pub fn stroke_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        Shape::stroke_property().add_owner::<DottedLineMargin>()
    }
} }

impl DottedLineMargin {
    pub(crate) const STROKE_WIDTH: f64 = 1.0;
    pub(crate) const DEFAULT_MARGIN: Thickness = Thickness::symmetric(2.0, 0.0);

    fn static_constructor() {
        Layoutable::margin_property().override_default_value::<Self>(Self::DEFAULT_MARGIN);
        Visual::affects_render::<Self>(&[Self::stroke_property().as_property()]);
    }

    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn stroke(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::stroke_property())
    }

    pub fn set_stroke(&self, brush: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::stroke_property(), brush);
    }
}

impl LayoutableImpl for DottedLineMargin {
    fn measure_override(_: &Self, _: Size) -> Size {
        Size::new(Self::STROKE_WIDTH, 0.0)
    }
}

impl VisualImpl for DottedLineMargin {
    fn render(this: &Self, context: &mut DrawingContext) {
        let Some(brush) = this.stroke() else {
            return;
        };
        let bounds = this.bounds();
        if bounds.width <= 0.0 || bounds.height <= Self::STROKE_WIDTH {
            return;
        }
        let pen: Rc<dyn IPen> = Pen::with_all(
            Some(brush),
            Self::STROKE_WIDTH,
            Some(DashStyle::dot()),
            PenLineCap::Round,
            PenLineJoin::Miter,
            10.0,
        )
        .into();
        let x = bounds.width * 0.5;
        // Start on a pixel center at 1× so antialiasing leaves visible gaps
        // between the round caps of the zero-length dashes.
        let inset = Self::STROKE_WIDTH * 0.5;
        context.draw_line(&pen, Point::new(x, inset), Point::new(x, bounds.height - inset));
    }
}
