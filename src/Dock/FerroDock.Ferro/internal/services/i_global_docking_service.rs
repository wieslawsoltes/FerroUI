// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Original global-target resolution and operation-policy contract.
use ferrodock_model::core::{DockOperation, IDock, IDockable};
use ferroui_controls::Control;
use std::rc::Rc;
pub(crate) trait IGlobalDockingService {
    fn resolve_global_target_dock(&self, drop_control: Option<&Control>) -> Option<Rc<dyn IDock>>;
    fn should_use_global_operation(
        &self,
        has_local: bool,
        local: DockOperation,
        global: DockOperation,
    ) -> bool;
    fn try_apply_global_docking_proportion(
        &self,
        source: Rc<dyn IDockable>,
        source_root: Option<Rc<dyn IDockable>>,
        target_root: Option<Rc<dyn IDockable>>,
        proportion: f64,
    ) -> bool;
}
