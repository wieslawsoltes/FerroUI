// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{DockTargetBase, DockTargetBaseImpl};
use ferrodock_model::core::DockOperation;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::ControlImpl;
/// Original global target, whose unresolved operation is None.
#[repr(C)]
pub struct GlobalDockTarget {
    base: DockTargetBase,
}
ferro_class!(GlobalDockTarget:DockTargetBase);
ferro_class_info!(GlobalDockTarget {
    new: GlobalDockTarget::new
});
ferro_impl_classes!(GlobalDockTarget:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl DockTargetBaseImpl for GlobalDockTarget {
    fn default_dock_operation(_this: &Self) -> DockOperation {
        DockOperation::None
    }
}
impl GlobalDockTarget {
    /// Constructs the source state for a derived global target.
    pub fn construct() -> Self {
        Self {
            base: DockTargetBase::construct(),
        }
    }
    /// Creates the global dock target.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
