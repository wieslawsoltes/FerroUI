// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::i_global_docking_service::IGlobalDockingService;
use crate::{controls::DockControl, model_projection};
use ferrodock_model::core::{DockOperation, IDock, IDockable};
use ferrodock_settings::{DockGlobalDockingPreset, DockSettings};
use ferroui_base::ferro_thread_local;
use ferroui_controls::Control;
use std::rc::Rc;
ferro_thread_local! {
    static INSTANCE: Rc<dyn IGlobalDockingService> = Rc::new(GlobalDockingService);
}
pub(crate) struct GlobalDockingService;
impl GlobalDockingService {
    pub(crate) fn instance() -> Rc<dyn IGlobalDockingService> {
        INSTANCE.with(Clone::clone)
    }
    fn resolve_outermost_global_target_dock(
        dockable: Option<Rc<dyn IDockable>>,
    ) -> Option<Rc<dyn IDock>> {
        let mut current = dockable;
        let mut resolved = None;
        while let Some(item) = current {
            if let Some(dock) = item.clone().as_dock() {
                if item.clone().as_global_target().is_some() {
                    resolved = Some(dock);
                }
            }
            current = item.owner();
        }
        resolved
    }
    fn active_dock(control: Option<&Control>) -> Option<Rc<dyn IDock>> {
        control?
            .find_ancestor_of_type::<DockControl>(false)?
            .layout()?
            .active_dockable()?
            .as_dock()
    }
}
impl IGlobalDockingService for GlobalDockingService {
    fn resolve_global_target_dock(&self, control: Option<&Control>) -> Option<Rc<dyn IDock>> {
        if DockSettings::global_docking_preset() == DockGlobalDockingPreset::LocalFirst {
            if let Some(item) =
                control.and_then(|c| model_projection::dockable(c.data_context().as_ref()))
            {
                return item
                    .clone()
                    .as_dock()
                    .or_else(|| item.owner().and_then(|o| o.as_dock()));
            }
            return Self::active_dock(control);
        }
        if let Some(item) =
            control.and_then(|c| model_projection::dockable(c.data_context().as_ref()))
        {
            return Self::resolve_outermost_global_target_dock(Some(item.clone()))
                .or_else(|| item.clone().as_dock())
                .or_else(|| item.owner().and_then(|o| o.as_dock()));
        }
        if let Some(active) = Self::active_dock(control) {
            return Self::resolve_outermost_global_target_dock(Some(active.clone()))
                .or(Some(active));
        }
        None
    }
    fn should_use_global_operation(
        &self,
        has_local: bool,
        local: DockOperation,
        global: DockOperation,
    ) -> bool {
        if global == DockOperation::None {
            return false;
        }
        if DockSettings::global_docking_preset() == DockGlobalDockingPreset::LocalFirst {
            if !has_local {
                return true;
            }
            return local == DockOperation::Window;
        }
        true
    }
    fn try_apply_global_docking_proportion(
        &self,
        source: Rc<dyn IDockable>,
        source_root: Option<Rc<dyn IDockable>>,
        target_root: Option<Rc<dyn IDockable>>,
        proportion: f64,
    ) -> bool {
        if source_root.is_none() || target_root.is_none() || source.owner().is_none() {
            return false;
        }
        source
            .owner()
            .expect("source owner re-read")
            .set_proportion(proportion);
        source
            .owner()
            .expect("source owner after proportion callback")
            .set_collapsed_proportion(proportion);
        true
    }
}
