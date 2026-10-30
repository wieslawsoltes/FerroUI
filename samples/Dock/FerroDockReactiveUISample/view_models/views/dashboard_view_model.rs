// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model_reactiveui::{
    core::{DockBase, DockableBaseImpl},
    impl_dock,
};
use ferroui_reactiveui::reactive_object;
use std::rc::Rc;
/// Original DashboardViewModel, preserving its source DockBase behavior.
pub struct DashboardViewModel {
    base: DockBase,
}
reactive_object!(DashboardViewModel, base);
impl DockableBaseImpl for DashboardViewModel {}
impl_dock!(@base DashboardViewModel,DockBase);
impl DashboardViewModel {
    /// Creates the model and initializes original DockBase observations.
    pub fn new() -> Rc<Self> {
        let model = Rc::new_cyclic(|runtime| Self {
            base: DockBase::construct(runtime.clone()),
        });
        model.base.initialize(&model);
        model
    }
}
