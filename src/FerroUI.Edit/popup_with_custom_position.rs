use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Ref, StyledElementImpl,
    VisualImpl,
};
use ferroui_controls::primitives::Popup;
use ferroui_controls::ControlImpl;

/// A popup whose horizontal and vertical placement offsets can be set together.
#[repr(C)]
pub struct PopupWithCustomPosition {
    base: Popup,
}
ferro_class!(PopupWithCustomPosition: Popup);
ferroui_base::ferro_class_info!(PopupWithCustomPosition {
    new: PopupWithCustomPosition::new
});
ferro_impl_classes!(PopupWithCustomPosition: FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);
impl PopupWithCustomPosition {
    pub fn construct() -> Self {
        Self {
            base: Popup::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn offset(&self) -> Point {
        Point::new(self.horizontal_offset(), self.vertical_offset())
    }
    pub fn set_offset(&self, value: Point) {
        self.set_horizontal_offset(value.x);
        self.set_vertical_offset(value.y);
    }
}
