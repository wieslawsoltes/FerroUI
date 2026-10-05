use super::WindowDecorationProperties;
use crate::{Border, Control, ControlImpl};
use ferroui_base::input::{Cursor, InputElementImpl, StandardCursorType, WindowDecorationsElementRole};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, FerroObjectImplExt, Rect, Ref, Size,
    StyledElementImpl, Thickness, VisualImpl,
};
use std::cell::Cell;
use std::rc::Rc;

/// An invisible layer that provides resize grip hit-test zones at window
/// edges. Grips only cover the frame/shadow area outside the client area.
#[repr(C)]
pub(crate) struct ResizeGripLayer {
    base: Control,
    top: Ref<Control>,
    bottom: Ref<Control>,
    left: Ref<Control>,
    right: Ref<Control>,
    top_left: Ref<Control>,
    top_right: Ref<Control>,
    bottom_left: Ref<Control>,
    bottom_right: Ref<Control>,
    grip_thickness: Cell<Thickness>,
}

ferro_class!(ResizeGripLayer: Control);
ferro_impl_classes!(ResizeGripLayer: StyledElementImpl, VisualImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl FerroObjectImpl for ResizeGripLayer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        this.set_is_hit_test_visible(true);
        let children = this.visual_children();
        children.add(this.top.clone().upcast());
        children.add(this.bottom.clone().upcast());
        children.add(this.left.clone().upcast());
        children.add(this.right.clone().upcast());
        children.add(this.top_left.clone().upcast());
        children.add(this.top_right.clone().upcast());
        children.add(this.bottom_left.clone().upcast());
        children.add(this.bottom_right.clone().upcast());
    }
}

impl LayoutableImpl for ResizeGripLayer {
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let gt = this.grip_thickness.get();
        let w = final_size.width;
        let h = final_size.height;

        // Hide all grips when thickness is zero (e.g. maximized/fullscreen)
        let has_grips = gt.left > 0.0 || gt.top > 0.0 || gt.right > 0.0 || gt.bottom > 0.0;
        this.set_is_hit_test_visible(has_grips);
        if !has_grips {
            let empty = Rect::default();
            this.top.arrange(empty);
            this.bottom.arrange(empty);
            this.left.arrange(empty);
            this.right.arrange(empty);
            this.top_left.arrange(empty);
            this.top_right.arrange(empty);
            this.bottom_left.arrange(empty);
            this.bottom_right.arrange(empty);
            return final_size;
        }

        // Edges fill the space between their adjacent corners
        this.top.arrange(Rect::new(gt.left, 0.0, (w - gt.left - gt.right).max(0.0), gt.top));
        this.bottom.arrange(Rect::new(gt.left, h - gt.bottom, (w - gt.left - gt.right).max(0.0), gt.bottom));
        this.left.arrange(Rect::new(0.0, gt.top, gt.left, (h - gt.top - gt.bottom).max(0.0)));
        this.right.arrange(Rect::new(w - gt.right, gt.top, gt.right, (h - gt.top - gt.bottom).max(0.0)));

        // Corners use the thickness of their adjacent edges
        this.top_left.arrange(Rect::new(0.0, 0.0, gt.left, gt.top));
        this.top_right.arrange(Rect::new(w - gt.right, 0.0, gt.right, gt.top));
        this.bottom_left.arrange(Rect::new(0.0, h - gt.bottom, gt.left, gt.bottom));
        this.bottom_right.arrange(Rect::new(w - gt.right, h - gt.bottom, gt.right, gt.bottom));

        final_size
    }
}

impl ResizeGripLayer {
    pub(crate) fn new() -> Ref<Self> {
        use StandardCursorType as C;
        use WindowDecorationsElementRole as R;
        instantiate(Self {
            base: Control::construct(),
            top: Self::create_grip(R::ResizeN, C::TopSide),
            bottom: Self::create_grip(R::ResizeS, C::BottomSide),
            left: Self::create_grip(R::ResizeW, C::LeftSide),
            right: Self::create_grip(R::ResizeE, C::RightSide),
            top_left: Self::create_grip(R::ResizeNW, C::TopLeftCorner),
            top_right: Self::create_grip(R::ResizeNE, C::TopRightCorner),
            bottom_left: Self::create_grip(R::ResizeSW, C::BottomLeftCorner),
            bottom_right: Self::create_grip(R::ResizeSE, C::BottomRightCorner),
            grip_thickness: Cell::new(Thickness::default()),
        })
    }

    /// The thickness of the resize grip area at each edge. Grips are placed
    /// outside the client area (covering frame + shadow).
    #[allow(dead_code)]
    pub(crate) fn grip_thickness(&self) -> Thickness {
        self.grip_thickness.get()
    }

    pub(crate) fn set_grip_thickness(&self, value: Thickness) {
        if self.grip_thickness.get() != value {
            self.grip_thickness.set(value);
            self.invalidate_arrange();
        }
    }

    fn create_grip(role: WindowDecorationsElementRole, cursor_type: StandardCursorType) -> Ref<Control> {
        let grip = Border::new();
        let transparent: Rc<dyn IBrush> = Brushes::transparent();
        grip.set_background(Some(transparent));
        grip.set_cursor(Some(Cursor::new(cursor_type)));
        grip.set_is_hit_test_visible(true);
        grip.set_horizontal_alignment(HorizontalAlignment::Stretch);
        grip.set_vertical_alignment(VerticalAlignment::Stretch);
        WindowDecorationProperties::set_element_role(&grip, role);
        grip.upcast()
    }
}
