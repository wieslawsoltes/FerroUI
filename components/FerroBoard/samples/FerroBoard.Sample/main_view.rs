//! Passive shared sample view for desktop content and single-view platform lifetimes.
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, input::InputElementImpl,
    interactivity::InteractiveImpl, layout::LayoutableImpl,
};
use ferroui_controls::{
    ContentControlImpl, ControlImpl, UserControl, primitives::TemplatedControlImpl,
};
#[repr(C)]
pub struct MainView {
    base: UserControl,
}
ferroui_base::ferro_class!(MainView:UserControl);
ferroui_base::ferro_class_info!(MainView { new: MainView::new });
ferroui_base::ferro_impl_classes!(MainView:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl);
impl MainView {
    pub fn new() -> Ref<Self> {
        let this = ferroui_base::instantiate(Self {
            base: UserControl::construct(),
        });
        crate::compiled_main_view::populate(None, &this).unwrap_or_else(|error| panic!("{error}"));
        this
    }
}
