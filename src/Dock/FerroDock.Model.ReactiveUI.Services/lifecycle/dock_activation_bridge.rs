// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::IActivationBridge;
use ferroui_reactiveui::{IActivatableViewModel, IViewFor};
use ferroui_system::IDisposable;
use std::rc::Rc;
/// Source default activation bridge, with no replacement activation state.
#[derive(Default)]
pub struct DockActivationBridge;
impl DockActivationBridge {
    /// Creates the stateless source bridge.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
    /// Delegates directly to the supplied model's activator. The required view
    /// argument is not inspected, subscribed, modified or retained by the bridge.
    pub fn track(
        &self,
        view_model: Rc<dyn IActivatableViewModel>,
        _view: Rc<dyn IViewFor>,
    ) -> Rc<dyn IDisposable> {
        view_model.activator().activate()
    }
}
impl IActivationBridge for DockActivationBridge {
    fn track(
        &self,
        view_model: Rc<dyn IActivatableViewModel>,
        view: Rc<dyn IViewFor>,
    ) -> Rc<dyn IDisposable> {
        self.track(view_model, view)
    }
}
#[cfg(test)]
#[path = "dock_activation_bridge_tests.rs"]
mod tests;
