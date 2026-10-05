//! Port of `Pages/Gestures/GesturePinchRotationPage.xaml.cs`: the class of
//! the document `Pages/Gestures/GesturePinchRotationPage.xaml`.

use crate::markup::xaml_class;
use ferroui_base::input::{InputElement, InputElementImpl, PinchEventArgs};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, ControlImpl, Panel, Slider, UserControl};
use std::cell::Cell;

#[repr(C)]
pub struct GesturePinchRotationPage {
    base: UserControl,
    is_init: Cell<bool>,
}

ferro_class!(GesturePinchRotationPage: UserControl);
ferro_impl_classes!(
    GesturePinchRotationPage: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(GesturePinchRotationPage { new: GesturePinchRotationPage::new });
xaml_class!(GesturePinchRotationPage, "/Pages/Gestures/GesturePinchRotationPage.xaml");

impl VisualImpl for GesturePinchRotationPage {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        if this.is_init.get() {
            return;
        }

        this.is_init.set(true);

        // The handler belongs to a child of the page: it holds the page weakly.
        let weak = this.to_ref().downgrade();
        this.rotation_gesture().add_handler(InputElement::pinch_event(), move |_s, e: &PinchEventArgs| {
            if let Some(this) = weak.upgrade() {
                this.angle_slider().set_range_value(e.angle());
            }
        });
    }
}

impl GesturePinchRotationPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), is_init: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn angle_slider(&self) -> Ref<Slider> {
        self.get_control::<Slider>("AngleSlider")
    }

    fn rotation_gesture(&self) -> Ref<Panel> {
        self.get_control::<Panel>("RotationGesture")
    }
}
