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
/// Original native StackDockControl template host.
#[repr(C)]
pub struct StackDockControl {
    base: TemplatedControl,
}
ferro_class!(StackDockControl:TemplatedControl);
ferro_class_info!(StackDockControl {
    new: StackDockControl::new
});
ferro_impl_classes!(StackDockControl:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl StackDockControl {
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
