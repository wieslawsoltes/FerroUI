// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::{
    primitives::{TemplatedControl, TemplatedControlImpl},
    ControlImpl,
};
/// Original native ProportionalDockControl template host.
#[repr(C)]
pub struct ProportionalDockControl {
    base: TemplatedControl,
}
ferro_class!(ProportionalDockControl:TemplatedControl);
ferro_class_info!(ProportionalDockControl{new:ProportionalDockControl::new,markup:{attributes:[TemplatePart("PART_ItemsControl",type(Ref<ferroui_controls::ItemsControl>),IsRequired=true)]}});
ferro_impl_classes!(ProportionalDockControl:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl ProportionalDockControl {
    /// Constructs this source class part for native inheritance.
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
        }
    }
    /// Creates the original native template host.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
