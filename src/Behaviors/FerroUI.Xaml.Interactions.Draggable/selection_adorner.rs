//! Port of `SelectionAdorner.cs`.

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{Colors, DrawingContext, Geometry, IBrush, IPen, Pen, RectangleGeometry, SolidColorBrush};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Rect, Ref, StyledElementImpl,
    VisualImpl,
};
use ferroui_controls::primitives::AdornerLayer;
use ferroui_controls::{Control, ControlImpl};
use std::rc::Rc;

/// Simple adorner used to show selection rectangles while dragging: four handles at the corners of
/// the adorned element.
#[repr(C)]
pub struct SelectionAdorner {
    base: Control,
}

ferro_class!(SelectionAdorner: Control);
ferro_class_info!(SelectionAdorner { new: SelectionAdorner::new });
ferro_impl_classes!(
    SelectionAdorner: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl VisualImpl for SelectionAdorner {
    /// Renders the selection rectangles.
    fn render(this: &Self, context: &mut DrawingContext) {
        let Some(adorned_element) = AdornerLayer::get_adorned_element(this) else {
            return;
        };

        let bounds = adorned_element.bounds();
        let brush: Rc<dyn IBrush> = SolidColorBrush::with_color_and_opacity(Colors::WHITE, 0.5).into();
        let pen: Rc<dyn IPen> = Pen::with_brush(Some(SolidColorBrush::with_color(Colors::BLACK).into()), 1.5).into();
        let r = 5.0;
        let top_left: Ref<Geometry> = RectangleGeometry::with_rect(Rect::new(-r, -r, r + r, r + r)).upcast();
        let top_right: Ref<Geometry> =
            RectangleGeometry::with_rect(Rect::new(-r, bounds.height - r, r + r, r + r)).upcast();
        let bottom_left: Ref<Geometry> =
            RectangleGeometry::with_rect(Rect::new(bounds.width - r, -r, r + r, r + r)).upcast();
        let bottom_right: Ref<Geometry> =
            RectangleGeometry::with_rect(Rect::new(bounds.width - r, bounds.height - r, r + r, r + r)).upcast();

        context.draw_geometry(Some(&brush), Some(&pen), &top_left);
        context.draw_geometry(Some(&brush), Some(&pen), &top_right);
        context.draw_geometry(Some(&brush), Some(&pen), &bottom_left);
        context.draw_geometry(Some(&brush), Some(&pen), &bottom_right);
    }
}

impl SelectionAdorner {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Control::construct() }
    }

    /// Creates the adorner.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
