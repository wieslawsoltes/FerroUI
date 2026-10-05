use super::{Shape, ShapeImpl};
use crate::ControlImpl;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{EllipseGeometry, Geometry};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Rect, Ref, Size,
    StyledElementImpl, Visual, VisualImpl,
};

/// Represents an ellipse which fills its bounds.
#[repr(C)]
pub struct Ellipse {
    base: Shape,
}

ferro_class!(Ellipse: Shape);
ferroui_base::ferro_class_info!(Ellipse { new: Ellipse::new });
ferro_impl_classes!(Ellipse: StyledElementImpl, VisualImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl FerroObjectImpl for Ellipse {}

impl ShapeImpl for Ellipse {
    fn create_defining_geometry(this: &Self) -> Option<Ref<Geometry>> {
        let rect = Rect::from_size(this.bounds().size()).deflate(this.stroke_thickness() / 2.0);
        Some(EllipseGeometry::with_rect(rect).upcast())
    }
}

impl LayoutableImpl for Ellipse {
    fn measure_override(this: &Self, _available_size: Size) -> Size {
        Size::new(this.stroke_thickness(), this.stroke_thickness())
    }
}

impl Ellipse {
    fn static_constructor() {
        Shape::affects_geometry::<Ellipse>(&[
            Visual::bounds_property().as_property(),
            Shape::stroke_thickness_property().as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Shape::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
