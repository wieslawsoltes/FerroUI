// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{
    adorner_helper::AdornerHelper,
    dock_helpers::DockHelpers,
    drag_dockable_resolver::DragDockableResolver,
    global_docking_helper::GlobalDockingHelper,
    managed_window_registry::ManagedWindowRegistry,
    services::{
        global_docking_service::GlobalDockingService,
        i_global_docking_service::IGlobalDockingService,
    },
};
use crate::{
    contract::DockOperationHandler,
    controls::{DockControl, DockTarget, GlobalDockTarget, ManagedWindowLayer, MdiDocumentWindow},
    model_projection,
};
use ferrodock_model::core::{
    DockGroupValidator, DockInheritanceHelper, DockOperation, DragAction, IDock, IDockManager,
    IDockManagerState, IDockable, IFactory,
};
use ferrodock_settings::{DockLogger, DockProperties, DockSettings};
use ferroui_base::{
    data::core::ValueType, metadata::MarkupType, BoxedValue, PixelPoint, Point, Ref, Visual,
};
use ferroui_controls::{Control, TopLevel};
use std::{any::TypeId, cell::RefCell, rc::Rc};
macro_rules! reject {
    ($stage:expr,$message:expr $(,)?) => {{
        #[cfg(debug_assertions)]
        DockManagerState::log_drop_rejection($stage, $message);
    }};
}
pub(crate) struct DockManagerState {
    pub(crate) dock_manager: Rc<dyn IDockManager>,
    pub(crate) drop_control: RefCell<Option<Ref<Control>>>,
    pub(crate) local_adorner_helper: Rc<AdornerHelper<DockTarget>>,
    pub(crate) global_adorner_helper: Rc<AdornerHelper<GlobalDockTarget>>,
    pub(crate) global_docking: Rc<dyn IGlobalDockingService>,
}
impl IDockManagerState for DockManagerState {}
impl DockManagerState {
    pub(crate) fn new(
        manager: Rc<dyn IDockManager>,
        global: Option<Rc<dyn IGlobalDockingService>>,
    ) -> Self {
        // Source reads the setting separately for each helper construction.
        Self {
            dock_manager: manager,
            global_docking: global.unwrap_or_else(GlobalDockingService::instance),
            local_adorner_helper: AdornerHelper::new(DockSettings::use_floating_dock_adorner()),
            global_adorner_helper: AdornerHelper::new(DockSettings::use_floating_dock_adorner()),
            drop_control: RefCell::new(None),
        }
    }
    pub(crate) fn drop_control(&self) -> Option<Ref<Control>> {
        self.drop_control.borrow().clone()
    }
    #[cfg(debug_assertions)]
    pub(crate) fn log_drop_rejection(stage: &str, message: &str) {
        DockLogger::log_debug(stage, message);
    }
    #[cfg(debug_assertions)]
    pub(crate) fn log_drag_state(message: &str) {
        DockLogger::log_debug("DragState", message);
    }
    // Source callers are arguments of diagnostics omitted from release builds.
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    pub(crate) fn with_capability_diagnostics(&self, message: String) -> String {
        match self.dock_manager.last_capability_evaluation() {
            None => message,
            Some(e) => format!("{message} {}", e.diagnostic_message()),
        }
    }
    pub(crate) fn resolve_global_target_dock(
        &self,
        drop: Option<&Control>,
    ) -> Option<Rc<dyn IDock>> {
        self.global_docking.resolve_global_target_dock(drop)
    }
    pub(crate) fn update_adorner_geometry(&self) {
        self.local_adorner_helper.update_geometry();
    }
    pub(crate) fn is_local_target_available(
        &self,
        point: Point,
        action: DragAction,
        relative: &Visual,
        validate: &DockOperationHandler<'_>,
    ) -> bool {
        if validate(point, DockOperation::Fill, action, relative) {
            return true;
        }
        self.drop_control()
            .and_then(|c| model_projection::dockable(c.data_context().as_ref()))
            .and_then(|d| d.factory())
            .and_then(|f| f.as_dock_outer_split_policy())
            .is_some()
            && (validate(point, DockOperation::OuterLeft, action, relative)
                || validate(point, DockOperation::OuterTop, action, relative)
                || validate(point, DockOperation::OuterRight, action, relative)
                || validate(point, DockOperation::OuterBottom, action, relative))
    }
    pub(crate) fn add_adorners(&self, local_valid: bool, global_valid: bool) {
        let local = if local_valid {
            self.drop_control()
                .filter(|c| c.get_value(DockProperties::is_dock_target_property()))
        } else {
            None
        };
        if let Some(control) = local {
            let host =
                DockProperties::get_dock_adorner_host(&control).unwrap_or_else(|| control.clone());
            let indicators = DockProperties::get_show_dock_indicator_only(&control);
            let empty_root = model_projection::root(control.data_context().as_ref())
                .is_some_and(|r| r.opened_dockables_count() == 0);
            self.local_adorner_helper
                .add_adorner(&host, indicators, !empty_root, !empty_root);
            self.local_adorner_helper
                .set_global_dock_availability(global_valid);
            self.local_adorner_helper.set_global_dock_active(false);
        } else {
            self.local_adorner_helper
                .set_global_dock_availability(false);
            self.local_adorner_helper.set_global_dock_active(false);
        }
        if let Some(drop) = global_valid.then(|| self.drop_control()).flatten() {
            let mut horizontal = true;
            let mut vertical = true;
            let current = self.drop_control().expect("source live drop control");
            if let Some(item) = model_projection::dockable(current.data_context().as_ref()) {
                if let Some(factory) = item.factory() {
                    if let Some(root) = factory
                        .find_root(Some(item), None)
                        .filter(|r| r.enable_adaptive_global_dock_targets())
                    {
                        (horizontal, vertical) = GlobalDockingHelper::can_global_dock(root);
                    }
                }
            }
            let control_type = drop.get_type().name();
            let context_type = Self::context_type_name(drop.data_context().as_ref());
            DockLogger::log_debug("GlobalAdorner",&format!("Evaluating global adorners for drop control '{control_type}' (DataContext={context_type}) with horizontal={}, vertical={}.", Self::boolean(horizontal),Self::boolean(vertical)));
            let mut dock_control = drop.find_ancestor_of_type::<DockControl>(false);
            if dock_control.is_none() {
                let mut current = drop.visual_parent();
                while let Some(visual) = current {
                    if let Some(found) = visual.cast::<DockControl>() {
                        dock_control = Some(found);
                        break;
                    }
                    current = visual.visual_parent();
                }
            }
            let target = self.resolve_global_target_dock(Some(&drop));
            if let (Some(control), Some(target)) = (dock_control, target) {
                if DockInheritanceHelper::get_effective_enable_global_docking(target.clone()) {
                    let indicators = DockProperties::get_show_dock_indicator_only(&drop);
                    self.global_adorner_helper
                        .add_adorner(&control, indicators, horizontal, vertical);
                    DockLogger::log_debug(
                        "GlobalAdorner",
                        &format!(
                            "Added global adorners to dock control '{}' targeting dock '{}'.",
                            control.get_type().name(),
                            target.title()
                        ),
                    );
                } else {
                    DockLogger::log_debug("GlobalAdorner", &format!("Skipped global adorners because EnableGlobalDocking is disabled for '{}'.",target.title()));
                }
            } else {
                DockLogger::log_debug("GlobalAdorner",&format!("Unable to resolve dock control host for drop control '{control_type}'. Global adorners not added."));
            }
        }
    }
    pub(crate) fn remove_adorners(&self) {
        self.local_adorner_helper.remove_adorner(None);
        self.global_adorner_helper.remove_adorner(None);
    }
    pub(crate) fn execute(
        &self,
        point: Point,
        operation: DockOperation,
        action: DragAction,
        relative: &Visual,
        source: Rc<dyn IDockable>,
        target: Rc<dyn IDockable>,
    ) {
        self.dock_manager
            .set_position(DockHelpers::to_dock_point(point));
        if relative.visual_root().is_none() {
            return;
        }
        let mut screen = DockHelpers::get_screen_point(relative, point);
        if DockHelpers::is_managed_window_hosting_enabled(Some(relative)) {
            if let Some(managed) = Self::try_get_managed_screen_position(relative, point) {
                screen = managed;
            }
        }
        self.dock_manager
            .set_screen_position(DockHelpers::to_dock_point(screen));
        self.dock_manager
            .validate_dockable(source, target, action, operation, true);
    }
    pub(crate) fn validate_dockable(
        &self,
        point: Point,
        operation: DockOperation,
        action: DragAction,
        relative: &Visual,
        source: Rc<dyn IDockable>,
    ) -> bool {
        let Some(target) = self
            .drop_control()
            .and_then(|c| model_projection::dockable(c.data_context().as_ref()))
        else {
            reject!(
                "ValidateDockable",
                &format!(
                    "DropControl data context is not IDockable (control={}).",
                    self.drop_control().map_or("", |c| c.get_type().name())
                )
            );
            return false;
        };
        if !self.validate_local_target(source.clone(), target.clone()) {
            return false;
        }
        self.dock_manager
            .set_position(DockHelpers::to_dock_point(point));
        if relative.visual_root().is_none() {
            reject!(
                "ValidateDockable",
                "Relative visual is not attached to a visual root."
            );
            return false;
        }
        let mut screen = DockHelpers::get_screen_point(relative, point);
        if DockHelpers::is_managed_window_hosting_enabled(Some(relative)) {
            if let Some(managed) = Self::try_get_managed_screen_position(relative, point) {
                screen = managed;
            }
        }
        self.dock_manager
            .set_screen_position(DockHelpers::to_dock_point(screen));
        let valid = self.dock_manager.validate_dockable(
            source.clone(),
            target.clone(),
            action,
            operation,
            false,
        );
        if !valid {
            reject!(
                "ValidateDockable",
                &self.with_capability_diagnostics(format!(
                    "DockManager rejected operation {operation:?} from '{}' to '{}'.",
                    source.title(),
                    target.title()
                ))
            );
        }
        valid
    }
    pub(crate) fn validate_local_target(
        &self,
        source: Rc<dyn IDockable>,
        target: Rc<dyn IDockable>,
    ) -> bool {
        if target.clone().as_local_target().is_none() {
            reject!(
                "ValidateLocalTarget",
                &format!(
                    "Target '{}' does not implement ILocalTarget.",
                    target.title()
                )
            );
            return false;
        }
        let valid = DockGroupValidator::validate_docking_groups(source.clone(), target.clone());
        if !valid {
            reject!(
                "ValidateLocalTarget",
                &format!(
                    "Dock group validation failed for '{}' -> '{}'.",
                    source.title(),
                    target.title()
                )
            );
        }
        valid
    }
    pub(crate) fn validate_global_target(
        &self,
        source: Rc<dyn IDockable>,
        target: Rc<dyn IDockable>,
    ) -> bool {
        if let Some(dock) = target.clone().as_dock() {
            let valid = DockGroupValidator::validate_global_docking(source.clone(), dock.clone());
            if !valid {
                reject!(
                    "ValidateGlobalTarget",
                    &format!(
                        "Global docking validation failed for '{}' -> '{}'.",
                        source.title(),
                        dock.title()
                    )
                );
            }
            return valid;
        }
        if target.clone().as_global_target().is_none() {
            reject!(
                "ValidateGlobalTarget",
                &format!(
                    "Target '{}' does not implement IGlobalTarget.",
                    target.title()
                )
            );
            return false;
        }
        let valid = DockGroupValidator::validate_docking_groups(source.clone(), target.clone());
        if !valid {
            reject!(
                "ValidateGlobalTarget",
                &format!(
                    "Dock group validation failed for '{}' -> '{}'.",
                    source.title(),
                    target.title()
                )
            );
        }
        valid
    }
    pub(crate) fn float(
        point: Point,
        input: &DockControl,
        dockable: Rc<dyn IDockable>,
        factory: Rc<dyn IFactory>,
        offset: PixelPoint,
    ) {
        let dockable = DragDockableResolver::resolve(dockable);
        let screen = input.point_to_screen(point);
        let adjusted = PixelPoint::new(
            screen.x.wrapping_add(offset.x),
            screen.y.wrapping_add(offset.y),
        );
        let mut pointer = Point::new(adjusted.x as f64, adjusted.y as f64);
        if DockHelpers::is_managed_window_hosting_enabled(Some(input)) {
            if let Some(managed) =
                Self::try_get_managed_pointer_position(input, factory.clone(), point, offset)
            {
                pointer = managed;
            }
        }
        dockable.set_pointer_screen_position(pointer.x, pointer.y);
        factory.float_dockable(Some(dockable));
    }
    fn try_get_managed_pointer_position(
        input: &DockControl,
        factory: Rc<dyn IFactory>,
        point: Point,
        offset: PixelPoint,
    ) -> Option<Point> {
        let layer = Self::try_resolve_managed_layer(input, factory.clone())?;
        if let Some(translated) = input.translate_point(point, &layer) {
            let dip = Self::get_dip_offset(input, offset);
            let mut pointer = Point::new(translated.x + dip.x, translated.y + dip.y);
            if let Some(content) = Self::try_get_managed_content_offset(input, factory) {
                pointer = Point::new(pointer.x - content.x, pointer.y - content.y);
            }
            return Some(pointer);
        }
        let top = layer.visual_root()?.cast::<TopLevel>()?;
        let screen = input.point_to_screen(point);
        let adjusted = PixelPoint::new(
            screen.x.wrapping_add(offset.x),
            screen.y.wrapping_add(offset.y),
        );
        let client = top.point_to_client(adjusted);
        let origin = layer
            .translate_point(Point::default(), &top)
            .unwrap_or_default();
        let mut pointer = Point::new(client.x - origin.x, client.y - origin.y);
        if let Some(content) = Self::try_get_managed_content_offset(input, factory) {
            pointer = Point::new(pointer.x - content.x, pointer.y - content.y);
        }
        Some(pointer)
    }
    fn try_get_managed_screen_position(relative: &Visual, point: Point) -> Option<Point> {
        let dock = relative
            .to_ref()
            .cast::<DockControl>()
            .or_else(|| relative.find_ancestor_of_type::<DockControl>(false))?;
        let factory = dock.layout()?.factory()?;
        let layer = Self::try_resolve_managed_layer(&dock, factory)?;
        if let Some(translated) = relative.translate_point(point, &layer) {
            return Some(translated);
        }
        let top = layer.visual_root()?.cast::<TopLevel>()?;
        let screen = relative.point_to_screen(point);
        let client = top.point_to_client(screen);
        let origin = layer
            .translate_point(Point::default(), &top)
            .unwrap_or_default();
        Some(Point::new(client.x - origin.x, client.y - origin.y))
    }
    fn try_resolve_managed_layer(
        context: &DockControl,
        factory: Rc<dyn IFactory>,
    ) -> Option<Ref<ManagedWindowLayer>> {
        let root = context.visual_root();
        let resolved = ManagedWindowLayer::try_get_layer(Some(context));
        if Self::is_layer_ready(resolved.as_deref())
            && Self::is_layer_in_root(resolved.as_deref(), root.as_deref())
        {
            return resolved;
        }
        let registered = ManagedWindowRegistry::try_get_layer(&factory);
        if Self::is_layer_ready(registered.as_deref())
            && Self::is_layer_in_root(registered.as_deref(), root.as_deref())
        {
            return registered;
        }
        for control in factory.dock_controls().iter() {
            let Some(control) = control
                .boxed_interface(TypeId::of::<Ref<DockControl>>())
                .and_then(|v| v.downcast_ref::<Ref<DockControl>>().cloned())
            else {
                continue;
            };
            let candidate = control
                .get_visual_descendants()
                .into_iter()
                .find_map(|v| v.cast::<ManagedWindowLayer>());
            if Self::is_layer_ready(candidate.as_deref())
                && Self::is_layer_in_root(candidate.as_deref(), root.as_deref())
            {
                return candidate;
            }
        }
        None
    }
    fn try_get_managed_content_offset(
        context: &DockControl,
        factory: Rc<dyn IFactory>,
    ) -> Option<Point> {
        if let Some(window) = context.find_ancestor_of_type::<MdiDocumentWindow>(false) {
            if let Some(offset) = window.try_get_content_offset() {
                return Some(offset);
            }
        }
        Self::try_resolve_managed_layer(context, factory)?.try_get_window_content_offset()
    }
    fn get_dip_offset(visual: &Visual, offset: PixelPoint) -> Point {
        // Preserve the source GetVisualRoot cast. Native TopLevelHost divergence is not hidden.
        let scaling = visual
            .visual_root()
            .and_then(|v| v.cast::<TopLevel>())
            .and_then(|t| t.screens())
            .and_then(|s| s.screen_from_visual(visual))
            .map_or(1.0, |s| s.scaling());
        Point::new(offset.x as f64 / scaling, offset.y as f64 / scaling)
    }
    fn is_layer_ready(layer: Option<&ManagedWindowLayer>) -> bool {
        layer.is_some_and(|l| l.is_visible() && l.visual_root().is_some())
    }
    fn is_layer_in_root(layer: Option<&ManagedWindowLayer>, root: Option<&Visual>) -> bool {
        let Some(layer) = layer else {
            return false;
        };
        root.is_none_or(|r| layer.visual_root().is_some_and(|l| l.ptr_eq(&r.to_ref())))
    }
    fn boolean(v: bool) -> &'static str {
        if v {
            "True"
        } else {
            "False"
        }
    }
    fn context_type_name(value: Option<&BoxedValue>) -> String {
        let Some(value) = value else {
            return "null".into();
        };
        if let Some(item) = model_projection::dockable(Some(value)) {
            return item.runtime_type_name().into();
        }
        let ty = ValueType::of_value(&**value);
        MarkupType::find_by_handle(ty.id())
            .map(|t| t.name.to_owned())
            .unwrap_or_else(|| ty.name().to_owned())
    }
}
