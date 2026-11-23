//! `SystemBackdropElement` of WinUI (`controls/dev/SystemBackdropElement` of microsoft-ui-xaml):
//! an element that draws a system backdrop in its bounds, with its corner radius.

use super::system_backdrop::{ElementSystemBackdrop, SystemBackdrop};
use crate::primitives::{TemplatedControl, TemplatedControlImpl};
use crate::ControlImpl;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{BoxShadows, DrawingContext};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_property, instantiate, CornerRadius, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref, RoundedRect, StyledElementImpl,
    StyledProperty, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};

/// An element that hosts a system backdrop (WinUI
/// `Microsoft.UI.Xaml.Controls.SystemBackdropElement`).
///
/// The backdrop is connected while the element is in a window, and its brush is the element's
/// background. WinUI derives the class from `FrameworkElement`; here it is a templated control
/// without a template, which has the background and corner radius the backdrop target uses.
#[repr(C)]
pub struct SystemBackdropElement {
    base: TemplatedControl,
    element_backdrop: ElementSystemBackdrop,
}

ferro_class!(SystemBackdropElement: TemplatedControl);
ferro_class_info!(SystemBackdropElement { new: SystemBackdropElement::new });

ferro_impl_classes!(
    SystemBackdropElement: StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl FerroObjectImpl for SystemBackdropElement {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::system_backdrop_property().as_property() {
            if this.is_attached_to_visual_tree() {
                this.element_backdrop.connect(this, this.system_backdrop());
            }
        } else if property == TemplatedControl::background_property().as_property()
            || property == TemplatedControl::corner_radius_property().as_property()
        {
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
        if let Some(background) = this.background() {
            let size = this.bounds().size();
            let radius = clamped_radius(this.corner_radius(), size.width.min(size.height) / 2.0);
            let rect = RoundedRect::from_corner_radius(Rect::from_size(size), radius);
            context.draw_rounded_rectangle(Some(&background), None, rect, &BoxShadows::default());
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
} }

impl SystemBackdropElement {
    /// Creates the class data of the element.
    pub fn construct() -> Self {
        Self { base: TemplatedControl::construct(), element_backdrop: Default::default() }
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
}
