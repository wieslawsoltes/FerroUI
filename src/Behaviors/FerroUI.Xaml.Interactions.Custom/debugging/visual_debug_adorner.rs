//! Port of `Debugging/VisualDebugAdorner.cs`.

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{BoxShadows, DrawingContext, IBrush, IPen, Pen, SolidColorBrush};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Rect, Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use std::rc::Rc;

/// The adorner [`VisualDebugBehavior`](crate::VisualDebugBehavior) shows over its control: a
/// border and a semi-transparent overlay in the highlight color.
///
/// An internal class of the reference: the module does not export it.
#[repr(C)]
pub struct VisualDebugAdorner {
    base: Control,
    adorned_element: Ref<Control>,
    brush: Ref<SolidColorBrush>,
}

ferro_class!(VisualDebugAdorner: Control);
ferro_impl_classes!(
    VisualDebugAdorner: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl VisualImpl for VisualDebugAdorner {
    fn render(this: &Self, context: &mut DrawingContext) {
        let _rect = this.adorned_element.bounds();
        let bounds = this.bounds();
        let area = Rect::new(0.0, 0.0, bounds.width, bounds.height);

        // Draw a border around the element.
        let brush: Rc<dyn IBrush> = this.brush.clone().into();
        let pen: Rc<dyn IPen> = Pen::with_brush(Some(brush), 2.0).into();
        context.draw_rectangle(None, Some(&pen), area, 0.0, 0.0, &BoxShadows::default());

        // Draw a semi-transparent overlay.
        let overlay_brush: Rc<dyn IBrush> = SolidColorBrush::with_color_and_opacity(this.brush.color(), 0.3).into();
        context.draw_rectangle(Some(&overlay_brush), None, area, 0.0, 0.0, &BoxShadows::default());
    }
}

impl VisualDebugAdorner {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    fn construct(adorned_element: Ref<Control>, brush: Ref<SolidColorBrush>) -> Self {
        Self { base: Control::construct(), adorned_element, brush }
    }

    /// Creates the adorner of `adorned_element`, drawn with `brush`.
    pub fn new(adorned_element: Ref<Control>, brush: Ref<SolidColorBrush>) -> Ref<Self> {
        let this = instantiate(Self::construct(adorned_element, brush));
        this.set_is_hit_test_visible(false);
        this
    }

    /// The element the adorner is drawn over.
    #[allow(dead_code)]
    pub fn adorned_element(&self) -> Ref<Control> {
        self.adorned_element.clone()
    }
}
