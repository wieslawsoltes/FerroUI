//! `SystemBackdropElement` of WinUI (`controls/dev/SystemBackdropElement` of microsoft-ui-xaml):
//! an element that draws a system backdrop in its bounds, with its corner radius.

use crate::media::{ElementSystemBackdrop, SystemBackdrop};
use crate::primitives::TemplatedControl;
use crate::{Control, ControlImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{BoxShadows, DrawingContext, IBrush};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_property, instantiate, CornerRadius, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref, RoundedRect, StyledElementImpl,
    StyledProperty, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::cell::RefCell;
use std::rc::Rc;

/// An element that hosts a system backdrop (WinUI
/// `Microsoft.UI.Xaml.Controls.SystemBackdropElement`). It has no template. The backdrop is
/// connected while the element is in a window, and the element draws its brush.
#[repr(C)]
pub struct SystemBackdropElement {
    base: Control,
    element_backdrop: ElementSystemBackdrop,
    backdrop_brush: RefCell<Option<Rc<dyn IBrush>>>,
}

ferro_class!(SystemBackdropElement: Control);
ferro_class_info!(SystemBackdropElement { new: SystemBackdropElement::new });

ferro_impl_classes!(
    SystemBackdropElement: StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl FerroObjectImpl for SystemBackdropElement {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::system_backdrop_property().as_property() {
            if this.is_attached_to_visual_tree() {
                this.element_backdrop.connect(this, this.system_backdrop());
            }
        } else if property == Self::corner_radius_property().as_property() {
            this.invalidate_visual();
        }
    }
}

impl VisualImpl for SystemBackdropElement {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.element_backdrop.connect(this, this.system_backdrop());
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.element_backdrop.disconnect();
    }

    fn render(this: &Self, context: &mut DrawingContext) {
        if let Some(brush) = this.backdrop_brush() {
            let size = this.bounds().size();
            let radius = clamped_radius(this.corner_radius(), size.width.min(size.height) / 2.0);
            let rect = RoundedRect::from_corner_radius(Rect::from_size(size), radius);
            context.draw_rounded_rectangle(Some(&brush), None, rect, &BoxShadows::default());
        }
        Self::parent_render(this, context);
    }
}

fn clamped_radius(radius: CornerRadius, largest: f64) -> CornerRadius {
    CornerRadius::new(
        radius.top_left.min(largest),
        radius.top_right.min(largest),
        radius.bottom_right.min(largest),
        radius.bottom_left.min(largest),
    )
}

ferroui_base::ferro_properties! { impl SystemBackdropElement {
    ferro_property!(
        /// Defines the `SystemBackdrop` property.
        pub fn system_backdrop_property() -> StyledProperty<Option<Ref<SystemBackdrop>>> {
            FerroProperty::register::<SystemBackdropElement, _>("SystemBackdrop", None)
        }
    );

    ferro_property!(
        /// Defines the `CornerRadius` property.
        pub fn corner_radius_property() -> StyledProperty<CornerRadius> {
            TemplatedControl::corner_radius_property().add_owner::<SystemBackdropElement>()
        }
    );
} }

impl SystemBackdropElement {
    /// Creates the class data of the element.
    pub fn construct() -> Self {
        Self { base: Control::construct(), element_backdrop: Default::default(), backdrop_brush: RefCell::new(None) }
    }

    /// Creates an element without a backdrop.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// `SystemBackdrop`: the backdrop the element draws.
    pub fn system_backdrop(&self) -> Option<Ref<SystemBackdrop>> {
        self.get_value(Self::system_backdrop_property())
    }

    pub fn set_system_backdrop(&self, value: Option<Ref<SystemBackdrop>>) {
        self.set_value(Self::system_backdrop_property(), value)
    }

    /// `CornerRadius`: the corners of the backdrop.
    pub fn corner_radius(&self) -> CornerRadius {
        self.get_value(Self::corner_radius_property())
    }

    pub fn set_corner_radius(&self, value: CornerRadius) {
        self.set_value(Self::corner_radius_property(), value)
    }

    /// The brush of the connected backdrop.
    pub(crate) fn backdrop_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.backdrop_brush.borrow().clone()
    }

    pub(crate) fn set_backdrop_brush(&self, brush: Option<Rc<dyn IBrush>>) {
        *self.backdrop_brush.borrow_mut() = brush;
        self.invalidate_visual();
    }
}
