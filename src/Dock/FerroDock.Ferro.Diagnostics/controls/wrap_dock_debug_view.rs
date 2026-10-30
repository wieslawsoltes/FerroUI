// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, input::InputElementImpl, instantiate,
    interactivity::InteractiveImpl, layout::LayoutableImpl, FerroObjectImpl, FerroObjectImplExt,
    Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::{
    primitives::TemplatedControlImpl, ContentControlImpl, ControlImpl, UserControl,
};
/// Original passive `WrapDockDebugView` inspector property view.
#[repr(C)]
pub struct WrapDockDebugView {
    base: UserControl,
}
ferro_class!(WrapDockDebugView:UserControl);
ferro_class_info!(WrapDockDebugView {
    new: WrapDockDebugView::new
});
ferro_impl_classes!(WrapDockDebugView:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl);
impl FerroObjectImpl for WrapDockDebugView {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.initialize_component();
    }
}
impl WrapDockDebugView {
    /// Initializes the inherited native class fields.
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }
    /// Creates and initializes the original compiled property view.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    fn initialize_component(&self) {
        crate::register_types();
        crate::compiled_wrap_dock_debug_view::populate(None, &self.to_ref())
            .unwrap_or_else(|error| panic!("{error}"));
    }
}
