// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_controls_proportional_stack_panel::ProportionalStackPanel;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::{
    presenters::ContentPresenter,
    primitives::{TemplatedControl, TemplatedControlImpl},
    ControlImpl,
};
/// Original native tool-dock template host and proportional-parent lookup.
#[repr(C)]
pub struct ToolDockControl {
    base: TemplatedControl,
}
ferro_class!(ToolDockControl:TemplatedControl);
ferro_class_info!(ToolDockControl {
    new: ToolDockControl::new
});
ferro_impl_classes!(ToolDockControl:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl ToolDockControl {
    /// Constructs the original native class part for inheritance.
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
        }
    }
    /// Creates the original native tool-dock template host.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    // The source private method currently has no production caller.
    #[cfg_attr(not(test), allow(dead_code))]
    fn get_panel(&self) -> Option<Ref<ProportionalStackPanel>> {
        if let Some(presenter) = self
            .parent()
            .and_then(|parent| parent.cast::<ContentPresenter>())
        {
            if let Some(panel) = presenter
                .visual_parent()
                .and_then(|parent| parent.cast::<ProportionalStackPanel>())
            {
                return Some(panel);
            }
        } else if let Some(panel) = self
            .visual_parent()
            .and_then(|parent| parent.cast::<ProportionalStackPanel>())
        {
            return Some(panel);
        }
        None
    }
}

#[cfg(test)]
#[path = "container_control_tests.rs"]
mod tests;
