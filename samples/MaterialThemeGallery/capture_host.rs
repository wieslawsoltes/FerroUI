//! The host of a page (the class of `CaptureHost.xaml`): the page in a theme variant scope on the
//! page background of the theme, with the flow direction of the page. The window of the gallery
//! shows the selected page in one; the capture mode and the comparison host give one a fixed
//! size and draw it into the pictures.

use crate::markup::xaml_class;
use crate::theme::Variant;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::FlowDirection;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{Border, ContentControlImpl, Control, ControlImpl, ThemeVariantScope, UserControl};

#[repr(C)]
pub struct CaptureHost {
    base: UserControl,
}

ferro_class!(CaptureHost: UserControl);
ferro_impl_classes!(
    CaptureHost: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(CaptureHost { new: CaptureHost::new });
xaml_class!(CaptureHost, "/CaptureHost.xaml");

impl CaptureHost {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn scope(&self) -> Ref<ThemeVariantScope> {
        self.get_control::<ThemeVariantScope>("Scope")
    }

    fn surface(&self) -> Ref<Border> {
        self.get_control::<Border>("Surface")
    }

    /// The page the host shows.
    pub fn page(&self) -> Option<Ref<Control>> {
        self.surface().child()
    }

    /// Shows `page` (or nothing).
    pub fn set_page(&self, page: Option<Ref<Control>>) {
        self.surface().set_child(page);
    }

    /// The theme variant of the page.
    pub fn set_variant(&self, variant: Variant) {
        self.scope().set_requested_theme_variant(Some(variant.theme_variant()));
    }

    /// The flow direction of the page.
    pub fn set_page_flow_direction(&self, flow_direction: FlowDirection) {
        self.scope().set_flow_direction(flow_direction);
    }
}
