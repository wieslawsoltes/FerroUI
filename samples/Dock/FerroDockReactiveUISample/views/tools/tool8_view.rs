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
/// Original passive Tool8View, populated solely from compiled source markup.
#[repr(C)]
pub struct Tool8View {
    base: UserControl,
}
ferro_class!(Tool8View:UserControl);
ferro_class_info!(Tool8View {
    new: Tool8View::new
});
ferro_impl_classes!(Tool8View:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl);
impl Tool8View {
    /// Creates the source view and initializes its compiled markup.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self {
            base: UserControl::construct(),
        });
        crate::register_types();
        crate::compiled_tool8_view::populate(None, &this).unwrap_or_else(|error| panic!("{error}"));
        this
    }
}
