// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, input::InputElementImpl, instantiate,
    interactivity::InteractiveImpl, layout::LayoutableImpl, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::{
    primitives::TemplatedControlImpl, ContentControlImpl, ControlImpl, UserControl,
};
/// Original passive HomeView, populated solely from compiled source markup.
#[repr(C)]
pub struct HomeView {
    base: UserControl,
}
ferro_class!(HomeView:UserControl);
ferro_class_info!(HomeView { new: HomeView::new });
ferro_impl_classes!(HomeView:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl);
impl HomeView {
    /// Creates the source view and initializes its compiled markup.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self {
            base: UserControl::construct(),
        });
        crate::register_types();
        crate::compiled_home_view::populate(None, &this).unwrap_or_else(|error| panic!("{error}"));
        this
    }
}
