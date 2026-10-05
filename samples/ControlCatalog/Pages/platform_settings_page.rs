//! Port of `Pages/PlatformSettingsPage.xaml.cs`: the class of the document
//! `Pages/PlatformSettingsPage.xaml`.

use crate::markup::xaml_class;
use crate::view_models::PlatformSettingsViewModel;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{Application, ContentPage, ControlImpl, PageImpl};
use std::rc::Rc;

#[repr(C)]
pub struct PlatformSettingsPage {
    base: ContentPage,
    view_model: Rc<PlatformSettingsViewModel>,
}

ferro_class!(PlatformSettingsPage: ContentPage);
ferro_impl_classes!(
    PlatformSettingsPage: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    PageImpl
);
ferro_class_info!(PlatformSettingsPage { new: PlatformSettingsPage::new });
xaml_class!(PlatformSettingsPage, "/Pages/PlatformSettingsPage.xaml");

impl VisualImpl for PlatformSettingsPage {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        this.view_model.subscribe(
            this.get_platform_settings()
                .or_else(|| Application::current().and_then(|application| application.platform_settings())),
        );
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        this.view_model.unsubscribe();

        Self::parent_on_detached_from_visual_tree(this, e);
    }
}

impl PlatformSettingsPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct(), view_model: PlatformSettingsViewModel::new() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(this.view_model.clone() as BoxedValue));
        this
    }
}
