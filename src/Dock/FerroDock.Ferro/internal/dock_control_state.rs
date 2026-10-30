use std::cell::Cell;
// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{
    dock_helpers::DockHelpers, dock_manager_state::DockManagerState,
    drag_dockable_resolver::DragDockableResolver as Resolver,
    drag_preview_helper::DragPreviewHelper, event_type::EventType,
    services::i_global_docking_service::IGlobalDockingService,
};
use crate::{
    contract::IDragOffsetCalculator,
    controls::{DockControl, DockTarget, DocumentTabStripItem, GlobalDockTarget},
    model_projection,
};
use ferrodock_model::{
    collections::ListHandle,
    core::{
        DockCapability, DockGroupValidator, DockInheritanceHelper, DockOperation, DragAction,
        IDockControl, IDockControlState, IDockManager, IDockManagerState, IDockable,
    },
    DockCapabilityResolver,
};
use ferrodock_settings::{DockProperties, DockSettings};
use ferroui_base::{PixelPoint, Point, Ref, Size, Vector, Visual};
use ferroui_controls::{Control, TopLevel};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
macro_rules! drag_log {
    ($message:expr $(,)?) => {{
        #[cfg(debug_assertions)]
        DockManagerState::log_drag_state($message);
    }};
}
macro_rules! reject {
    ($stage:expr,$message:expr $(,)?) => {{
        #[cfg(debug_assertions)]
        DockManagerState::log_drop_rejection($stage, $message);
    }};
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DockOperationResolution {
    local_operation: DockOperation,
    global_operation: DockOperation,
    use_global_operation: bool,
    selected_operation: DockOperation,
    is_valid: bool,
}
pub(crate) struct DockControlState {
    runtime: Weak<Self>,
    base: DockManagerState,
    context: DockDragContext,
    drag_preview_helper: DragPreviewHelper,
    transfer_target: RefCell<Option<Rc<Self>>>,
    active_dock_control: RefCell<Option<Ref<DockControl>>>,
    drag_offset_calculator: RefCell<Rc<dyn IDragOffsetCalculator>>,
}
impl IDockManagerState for DockControlState {}
impl IDockControlState for DockControlState {}
impl DockControlState {
    pub(crate) fn new(
        manager: Rc<dyn IDockManager>,
        calculator: Rc<dyn IDragOffsetCalculator>,
        global: Option<Rc<dyn IGlobalDockingService>>,
    ) -> Rc<Self> {
        Rc::new_cyclic(|runtime| Self {
            runtime: runtime.clone(),
            base: DockManagerState::new(manager, global),
            context: DockDragContext::new(),
            drag_preview_helper: DragPreviewHelper::new(),
            transfer_target: RefCell::new(None),
            active_dock_control: RefCell::new(None),
            drag_offset_calculator: RefCell::new(calculator),
        })
    }
    pub(crate) fn drag_offset_calculator(&self) -> Rc<dyn IDragOffsetCalculator> {
        self.drag_offset_calculator.borrow().clone()
    }
    pub(crate) fn set_drag_offset_calculator(&self, value: Rc<dyn IDragOffsetCalculator>) {
        let old = self.drag_offset_calculator.replace(value);
        drop(old);
    }
    fn this(&self) -> Rc<Self> {
        self.runtime.upgrade().expect("live original dock state")
    }
    fn can_drag_dockable(item: Rc<dyn IDockable>) -> bool {
        let item = Resolver::resolve(item);
        DockCapabilityResolver::is_enabled(
            Some(item.clone()),
            DockCapability::Drag,
            DockCapabilityResolver::resolve_operation_dock(Some(item)),
        )
    }
    fn can_float_dockable(item: Rc<dyn IDockable>) -> bool {
        let item = Resolver::resolve(item);
        DockCapabilityResolver::is_enabled(
            Some(item.clone()),
            DockCapability::Float,
            DockCapabilityResolver::resolve_operation_dock(Some(item)),
        )
    }
    fn get_preferred_preview_size(drag: &Control) -> Option<Size> {
        drag.to_ref().cast::<DocumentTabStripItem>()?;
        let bounds = drag.bounds().size();
        let margin = drag.margin();
        let width = max_preview_dimension(bounds.width + margin.left + margin.right);
        let height = max_preview_dimension(bounds.height + margin.top + margin.bottom);
        let _scaling = drag
            .visual_root()
            .and_then(|r| r.cast::<TopLevel>())
            .map_or(1.0, |t| t.render_scaling());
        let _desired = drag.desired_size();
        let _bounds_px_w = bounds.width * _scaling;
        let _bounds_px_h = bounds.height * _scaling;
        #[cfg(debug_assertions)]
        eprintln!("[Dock PreviewSize Capture] tabBoundsRectDip=({:.2},{:.2},{:.2},{:.2}) tabBoundsDip=({:.2},{:.2}) tabDesiredDip=({:.2},{:.2}) marginDip=({:.2},{:.2},{:.2},{:.2}) preferredDip=({:.2},{:.2}) tabBoundsPxExact=({:.2},{:.2}) tabBoundsPxRound=({:.0},{:.0}) tabBoundsPxCeil=({:.0},{:.0}) preferredPx~=({:.2},{:.2}) scale={:.2}",drag.bounds().x,drag.bounds().y,drag.bounds().width,drag.bounds().height,bounds.width,bounds.height,_desired.width,_desired.height,margin.left,margin.top,margin.right,margin.bottom,width,height,_bounds_px_w,_bounds_px_h,_bounds_px_w.round_ties_even(),_bounds_px_h.round_ties_even(),_bounds_px_w.ceil(),_bounds_px_h.ceil(),width*_scaling,height*_scaling,_scaling);
        Some(Size::new(width, height))
    }
    pub(crate) fn start_drag(
        &self,
        drag: &Control,
        start: Point,
        point: Point,
        active: &DockControl,
    ) {
        if !drag.get_value(DockProperties::is_drag_enabled_property()) {
            return;
        }
        if model_projection::dockable(drag.data_context().as_ref())
            .is_some_and(|d| !Self::can_drag_dockable(d))
        {
            return;
        }
        self.begin_drag(drag, start, active);
        self.base.drop_control.replace(None);
        active.set_is_dragging_dock(true);
        if let Some(item) = model_projection::dockable(drag.data_context().as_ref()) {
            let item = Resolver::resolve(item);
            let (can_show, screen, offset) = self.try_get_preview_placement(drag, active, point);
            if can_show {
                self.context.drag_offset.set(offset);
            }
            DockHelpers::show_windows(item.clone());
            if !self.context.pointer_pressed.get() {
                return;
            }
            let mut shown = false;
            if can_show && active.visual_root().is_some() {
                let preferred = Self::get_preferred_preview_size(drag);
                self.drag_preview_helper.show(
                    item,
                    screen,
                    self.context.drag_offset.get(),
                    Some(active),
                    preferred,
                );
                shown = true;
            }
            self.complete_drag_activation(shown);
        }
    }
    fn begin_drag(&self, drag: &Control, start: Point, active: &DockControl) {
        self.transfer_target.replace(None);
        self.active_dock_control.replace(Some(active.to_ref()));
        self.context.start(drag.to_ref(), start);
    }
    fn try_get_preview_placement(
        &self,
        drag: &Control,
        active: &DockControl,
        point: Point,
    ) -> (bool, PixelPoint, PixelPoint) {
        let mut offset = self.context.drag_offset.get();
        if active.visual_root().is_none() {
            drag_log!(
                "Skipping drag preview placement: active dock control not attached to visual tree."
            );
            return (false, PixelPoint::default(), offset);
        }
        let screen = active.point_to_screen(point);
        if drag.visual_root().is_none() {
            drag_log!("Using cached drag offset: dragged control not attached to visual tree.");
            return (true, screen, offset);
        }
        let calculator = self.drag_offset_calculator();
        offset = calculator.calculate_offset(drag, active, self.context.drag_start_point.get());
        (true, screen, offset)
    }
    fn complete_drag_activation(&self, shown: bool) {
        let owner = self.resolve_current_owner();
        if !owner.context.pointer_pressed.get() {
            if shown {
                self.drag_preview_helper.hide();
            }
            return;
        }
        if !Rc::ptr_eq(&owner, &self.this()) && !shown {
            return;
        }
        owner.context.do_drag_drop.set(true);
        let active = owner.active_dock_control.borrow().clone();
        if let Some(active) = active {
            active.set_is_dragging_dock(true);
        }
    }
    fn resolve_current_owner(&self) -> Rc<Self> {
        let mut owner = self.this();
        loop {
            let target = owner.transfer_target.borrow().clone();
            match target {
                Some(target) if !Rc::ptr_eq(&owner, &target) => owner = target,
                _ => return owner,
            }
        }
    }
    pub(crate) fn has_active_drag(&self) -> bool {
        self.resolve_current_owner().context.pointer_pressed.get()
    }
    pub(crate) fn try_transfer_drag_to(
        &self,
        target: Rc<Self>,
        source: &DockControl,
        target_control: &DockControl,
    ) -> bool {
        let owner = self.resolve_current_owner();
        if !Rc::ptr_eq(&owner, &self.this())
            || !self.context.pointer_pressed.get()
            || Rc::ptr_eq(&self.this(), &target)
            || target.context.pointer_pressed.get()
        {
            return false;
        }
        self.leave();
        self.base.drop_control.replace(None);
        self.context.clear_resolved_operation();
        target.transfer_target.replace(None);
        target.base.drop_control.replace(None);
        target.context.end();
        self.context.copy_to(&target.context);
        target
            .active_dock_control
            .replace(Some(target_control.to_ref()));
        self.transfer_target.replace(Some(target.clone()));
        self.context.end();
        self.active_dock_control.replace(None);
        source.set_is_dragging_dock(false);
        target_control.set_is_dragging_dock(target.context.do_drag_drop.get());
        drag_log!(&format!(
            "Transferred active drag to attached dock control '{}'.",
            target_control.get_type().name()
        ));
        true
    }
    fn enter(&self, point: Point, action: DragAction, relative: &Visual) {
        let local = self
            .base
            .is_local_target_available(point, action, relative, &|p, o, a, r| {
                self.validate_local(p, o, a, r)
            });
        let global = self.validate_global(point, DockOperation::Fill, action, relative);
        self.base.add_adorners(local, global);
    }
    fn over(
        &self,
        point: Point,
        action: DragAction,
        drop: &Control,
        relative: &Visual,
    ) -> DockOperationResolution {
        let resolution = self.resolve_dock_operation(point, action, drop, relative, true);
        self.context
            .resolved_operation
            .set(resolution.selected_operation);
        self.context
            .use_global_operation
            .set(resolution.use_global_operation);
        self.context
            .has_resolved_operation
            .set(resolution.selected_operation != DockOperation::None);
        resolution
    }
    fn resolve_dock_operation(
        &self,
        point: Point,
        action: DragAction,
        drop: &Control,
        relative: &Visual,
        update: bool,
    ) -> DockOperationResolution {
        self.base.update_adorner_geometry();
        let mut local = DockOperation::Fill;
        let mut global = DockOperation::None;
        let mut has_local = false;
        let adorner = self.base.local_adorner_helper.adorner.borrow().clone();
        if let Some(target) = adorner.and_then(|a| a.cast::<DockTarget>()) {
            has_local = true;
            local = target.get_dock_operation(
                point,
                drop,
                relative,
                action,
                &|p, o, a, r| self.validate_local(p, o, a, r),
                Some(&|p, o, a, r| self.is_dock_target_visible(p, o, a, r)),
            );
        }
        let adorner = self.base.global_adorner_helper.adorner.borrow().clone();
        if let Some(target) = adorner.and_then(|a| a.cast::<GlobalDockTarget>()) {
            global = target.get_dock_operation(
                point,
                drop,
                relative,
                action,
                &|p, o, a, r| self.validate_global(p, o, a, r),
                Some(&|p, o, a, r| self.is_dock_target_visible(p, o, a, r)),
            );
        }
        let use_global = self
            .base
            .global_docking
            .should_use_global_operation(has_local, local, global);
        let selected = if use_global { global } else { local };
        let valid = if use_global {
            self.validate_global(point, global, action, relative)
        } else {
            self.validate_local(point, local, action, relative)
        };
        if update {
            self.base
                .local_adorner_helper
                .set_global_dock_active(use_global);
        }
        DockOperationResolution {
            local_operation: local,
            global_operation: global,
            use_global_operation: use_global,
            selected_operation: selected,
            is_valid: valid,
        }
    }
    fn drop(
        &self,
        point: Point,
        action: DragAction,
        _drop: &Control,
        relative: &Visual,
        use_global: bool,
        selected: DockOperation,
    ) -> bool {
        if selected == DockOperation::None {
            return false;
        }
        self.base.remove_adorners();
        if self.context.drag_control().is_none() || self.base.drop_control().is_none() {
            return false;
        }
        if use_global {
            let Some(drop) = self.base.drop_control() else {
                return false;
            };
            let source = model_projection::dockable(
                self.context
                    .drag_control()
                    .expect("source drag control")
                    .data_context()
                    .as_ref(),
            );
            if let Some(source) = source {
                if let Some(target) = self.base.resolve_global_target_dock(Some(&drop)) {
                    let source = Resolver::resolve(source);
                    let source_root = source
                        .factory()
                        .and_then(|f| f.find_root(Some(source.clone()), Some(Rc::new(|_| true))));
                    let target_root = target
                        .factory()
                        .and_then(|f| f.find_root(Some(target.clone()), Some(Rc::new(|_| true))));
                    if !self.validate_global(point, selected, action, relative) {
                        if Self::can_float_dockable(source.clone()) {
                            let active = self
                                .context
                                .drag_control()
                                .expect("source drag control")
                                .find_ancestor_of_type::<DockControl>(false);
                            let factory = active
                                .as_ref()
                                .and_then(|a| a.layout())
                                .and_then(|l| l.factory())
                                .or_else(|| target.factory());
                            if let (Some(active), Some(factory)) = (active, factory) {
                                let screen = DockHelpers::get_screen_point(relative, point);
                                let pixel = round_screen_point(screen);
                                let active_point = active.point_to_client(pixel);
                                DockManagerState::float(
                                    active_point,
                                    &active,
                                    source,
                                    factory,
                                    self.context.drag_offset.get(),
                                );
                                return true;
                            }
                        }
                        return false;
                    }
                    // Source deliberately does not call ValidateGlobalTarget here.
                    self.execute(point, selected, action, relative, source.clone(), target);
                    self.base
                        .global_docking
                        .try_apply_global_docking_proportion(
                            source,
                            source_root.map(|r| r as Rc<dyn IDockable>),
                            target_root.map(|r| r as Rc<dyn IDockable>),
                            DockSettings::global_docking_proportion(),
                        );
                    return true;
                }
            }
        } else {
            let source = model_projection::dockable(
                self.context
                    .drag_control()
                    .expect("source drag control")
                    .data_context()
                    .as_ref(),
            );
            if let Some(source) = source {
                let Some(target) = model_projection::dockable(
                    self.base
                        .drop_control()
                        .expect("source drop control")
                        .data_context()
                        .as_ref(),
                ) else {
                    return false;
                };
                if Resolver::is_no_op_drop(source.clone(), target.clone(), selected) {
                    return true;
                }
                let source = Resolver::resolve(source);
                if !self
                    .base
                    .validate_local_target(source.clone(), target.clone())
                {
                    if Self::can_float_dockable(source.clone()) {
                        let active = self
                            .context
                            .drag_control()
                            .expect("source drag control")
                            .find_ancestor_of_type::<DockControl>(false);
                        let factory = active
                            .as_ref()
                            .and_then(|a| a.layout())
                            .and_then(|l| l.factory())
                            .or_else(|| {
                                self.base
                                    .drop_control()
                                    .expect("source live drop control")
                                    .find_ancestor_of_type::<DockControl>(false)
                                    .and_then(|a| a.layout())
                                    .and_then(|l| l.factory())
                            });
                        if let (Some(active), Some(factory)) = (active, factory) {
                            let screen = DockHelpers::get_screen_point(relative, point);
                            let pixel = round_screen_point(screen);
                            let active_point = active.point_to_client(pixel);
                            DockManagerState::float(
                                active_point,
                                &active,
                                source,
                                factory,
                                self.context.drag_offset.get(),
                            );
                            return true;
                        }
                    }
                    return false;
                }
                if selected == DockOperation::Window {
                    return false;
                }
                self.execute(point, selected, action, relative, source, target);
                return true;
            }
        }
        false
    }
    fn leave(&self) {
        self.context.clear_resolved_operation();
        self.base.remove_adorners();
    }
    fn validate_local(
        &self,
        point: Point,
        operation: DockOperation,
        action: DragAction,
        relative: &Visual,
    ) -> bool {
        if !self.base.dock_manager.is_docking_enabled() {
            reject!("ValidateLocal", "Docking is disabled.");
            return false;
        }
        let Some(source) = self
            .context
            .drag_control()
            .and_then(|c| model_projection::dockable(c.data_context().as_ref()))
        else {
            reject!(
                "ValidateLocal",
                "DragControl DataContext is not an IDockable."
            );
            return false;
        };
        let source = Resolver::resolve(source);
        if self
            .base
            .drop_control()
            .and_then(|c| model_projection::dockable(c.data_context().as_ref()))
            .is_some()
        {
            return self
                .base
                .validate_dockable(point, operation, action, relative, source);
        }
        let is_target = self
            .base
            .drop_control()
            .is_some_and(|c| c.get_value(DockProperties::is_dock_target_property()));
        if !is_target {
            reject!(
                "ValidateLocal",
                &format!(
                    "Control '{}' is not marked as a dock target.",
                    self.base
                        .drop_control()
                        .map_or("null", |c| c.get_type().name())
                )
            );
        }
        is_target
    }
    fn validate_global(
        &self,
        point: Point,
        operation: DockOperation,
        action: DragAction,
        relative: &Visual,
    ) -> bool {
        if !self.base.dock_manager.is_docking_enabled() {
            reject!("ValidateGlobal", "Docking is disabled.");
            return false;
        }
        let Some(source) = self
            .context
            .drag_control()
            .and_then(|c| model_projection::dockable(c.data_context().as_ref()))
        else {
            reject!(
                "ValidateGlobal",
                "DragControl DataContext is not an IDockable."
            );
            return false;
        };
        let source = Resolver::resolve(source);
        let Some(drop) = self.base.drop_control() else {
            reject!(
                "ValidateGlobal",
                "No DropControl available for global validation."
            );
            return false;
        };
        let Some(target) = self.base.resolve_global_target_dock(Some(&drop)) else {
            reject!(
                "ValidateGlobal",
                "Unable to resolve global docking target for DropControl."
            );
            return false;
        };
        if !DockInheritanceHelper::get_effective_enable_global_docking(target.clone()) {
            reject!(
                "ValidateGlobal",
                &format!("Global docking is disabled for dock '{}'.", target.title())
            );
            return false;
        }
        self.base
            .dock_manager
            .set_position(DockHelpers::to_dock_point(point));
        if relative.visual_root().is_none() {
            reject!(
                "ValidateGlobal",
                "Relative visual is not attached to a visual root."
            );
            return false;
        }
        let screen = DockHelpers::get_screen_point(relative, point);
        self.base
            .dock_manager
            .set_screen_position(DockHelpers::to_dock_point(screen));
        if !DockGroupValidator::validate_global_docking(source.clone(), target.clone()) {
            reject!(
                "ValidateGlobal",
                &format!(
                    "Global docking group validation failed for '{}' -> '{}'.",
                    source.title(),
                    target.title()
                )
            );
            return false;
        }
        let valid = self.base.dock_manager.validate_dockable(
            source.clone(),
            target.clone(),
            action,
            operation,
            false,
        );
        if !valid {
            reject!(
                "ValidateGlobal",
                &self.base.with_capability_diagnostics(format!(
                    "DockManager rejected global operation {operation:?} for '{}' -> '{}'.",
                    source.title(),
                    target.title()
                ))
            );
        }
        valid
    }
    fn is_dock_target_visible(
        &self,
        _point: Point,
        operation: DockOperation,
        _action: DragAction,
        _relative: &Visual,
    ) -> bool {
        let Some(source) = self
            .context
            .drag_control()
            .and_then(|c| model_projection::dockable(c.data_context().as_ref()))
        else {
            return true;
        };
        let source = Resolver::resolve(source);
        let Some(target) = self
            .base
            .drop_control()
            .and_then(|c| model_projection::dockable(c.data_context().as_ref()))
        else {
            return true;
        };
        self.base
            .dock_manager
            .is_dock_target_visible(source, target, operation)
    }
    fn execute(
        &self,
        point: Point,
        operation: DockOperation,
        action: DragAction,
        relative: &Visual,
        source: Rc<dyn IDockable>,
        target: Rc<dyn IDockable>,
    ) {
        let source = Resolver::resolve(source);
        self.base
            .execute(point, operation, action, relative, source, target);
    }
    pub(crate) fn process(
        &self,
        point: Point,
        _delta: Vector,
        phase: EventType,
        action: DragAction,
        active: Option<&DockControl>,
        controls: ListHandle<Rc<dyn IDockControl>>,
    ) {
        let Some(input) = active else {
            drag_log!("Process aborted: activeDockControl is null.");
            return;
        };
        if !self.base.dock_manager.is_docking_enabled() {
            if self.context.pointer_pressed.get() || self.context.do_drag_drop.get() {
                self.drag_preview_helper.hide();
                self.leave();
                self.context.end();
                self.base.drop_control.replace(None);
                input.set_is_dragging_dock(false);
            }
            drag_log!("Process skipped: docking disabled.");
            return;
        }
        match phase {
            EventType::Pressed => {
                if let Some(drag) = DockHelpers::get_control(
                    Some(input),
                    point,
                    DockProperties::is_drag_area_property(),
                ) {
                    if !drag.get_value(DockProperties::is_drag_enabled_property()) {
                        drag_log!(&format!(
                            "Pressed ignored: drag disabled on control '{}'.",
                            drag.get_type().name()
                        ));
                        return;
                    }
                    if model_projection::dockable(drag.data_context().as_ref())
                        .is_some_and(|d| !Self::can_drag_dockable(d))
                    {
                        drag_log!("Pressed ignored: dockable cannot be dragged (effective CanDrag=false).");
                        return;
                    }
                    self.begin_drag(&drag, point, input);
                    self.base.drop_control.replace(None);
                    drag_log!(&format!(
                        "Drag started from control '{}' at {point}.",
                        drag.get_type().name()
                    ));
                } else {
                    drag_log!("Pressed ignored: no drag area under pointer.");
                }
            }
            EventType::Released => {
                if self.context.do_drag_drop.get() {
                    let mut executed = false;
                    if let Some(drop) = self.base.drop_control() {
                        if self.context.target_dock_control().is_some() {
                            if drop.get_value(DockProperties::is_drop_enabled_property()) {
                                let mut use_global = self.context.use_global_operation.get();
                                let mut selected = self.context.resolved_operation.get();
                                if !self.context.has_resolved_operation.get() {
                                    let resolution = self.resolve_dock_operation(
                                        self.context.target_point.get(),
                                        action,
                                        &drop,
                                        &self
                                            .context
                                            .target_dock_control()
                                            .expect("source target visual"),
                                        false,
                                    );
                                    use_global = resolution.use_global_operation;
                                    selected = resolution.selected_operation;
                                }
                                executed = self.drop(
                                    self.context.target_point.get(),
                                    action,
                                    &drop,
                                    &self
                                        .context
                                        .target_dock_control()
                                        .expect("source target after resolution"),
                                    use_global,
                                    selected,
                                );
                                drag_log!(&format!(
                                    "Drop executed on '{}' with action '{action:?}'.",
                                    drop.get_type().name()
                                ));
                            } else {
                                drag_log!(&format!(
                                    "Drop skipped: drop disabled on '{}'.",
                                    drop.get_type().name()
                                ));
                            }
                        }
                    }
                    if !executed {
                        if let Some(item) = self
                            .context
                            .drag_control()
                            .and_then(|c| model_projection::dockable(c.data_context().as_ref()))
                        {
                            if Self::can_float_dockable(item.clone()) {
                                if let Some(factory) = input.layout().and_then(|l| l.factory()) {
                                    if input.visual_root().is_some() {
                                        DockManagerState::float(
                                            point,
                                            input,
                                            item.clone(),
                                            factory,
                                            self.context.drag_offset.get(),
                                        );
                                        drag_log!(&format!(
                                            "Drop fallback: floating dockable '{}'.",
                                            item.title()
                                        ));
                                    }
                                }
                            }
                        }
                    }
                } else {
                    drag_log!("Release ignored: drag operation was never activated.");
                }
                self.drag_preview_helper.hide();
                self.leave();
                self.context.end();
                self.base.drop_control.replace(None);
                input.set_is_dragging_dock(false);
                drag_log!("Drag context reset after release.");
            }
            EventType::Moved => {
                if !self.context.pointer_pressed.get() {
                    drag_log!("Move ignored: pointer not pressed.");
                    return;
                }
                if !self.context.do_drag_drop.get() {
                    let diff = self.context.drag_start_point.get() - point;
                    if DockSettings::is_minimum_drag_distance(diff.into()) {
                        if let Some(item) = self
                            .context
                            .drag_control()
                            .and_then(|c| model_projection::dockable(c.data_context().as_ref()))
                        {
                            let item = Resolver::resolve(item);
                            let (can_show, preview_point, offset) = self.try_get_preview_placement(
                                &self
                                    .context
                                    .drag_control()
                                    .expect("source drag for preview placement"),
                                input,
                                point,
                            );
                            if can_show {
                                self.context.drag_offset.set(offset);
                            }
                            DockHelpers::show_windows(item.clone());
                            if !self.context.pointer_pressed.get() {
                                drag_log!(
                                    "Drag activation handed off while showing related windows."
                                );
                                return;
                            }
                            let mut shown = false;
                            if can_show && input.visual_root().is_some() {
                                let preferred = Self::get_preferred_preview_size(
                                    &self
                                        .context
                                        .drag_control()
                                        .expect("source drag for preview size"),
                                );
                                self.drag_preview_helper.show(
                                    item.clone(),
                                    preview_point,
                                    self.context.drag_offset.get(),
                                    Some(input),
                                    preferred,
                                );
                                shown = true;
                                drag_log!(&format!(
                                    "Drag threshold reached for dockable '{}'. Showing preview.",
                                    item.title()
                                ));
                            }
                            self.complete_drag_activation(shown);
                        } else {
                            self.complete_drag_activation(false);
                        }
                        drag_log!("Drag operation activated.");
                    } else {
                        drag_log!(&format!("Move below drag threshold (diff={diff})."));
                    }
                }
                if self.context.do_drag_drop.get() {
                    let mut target_point = Point::default();
                    let mut target: Option<Ref<Visual>> = None;
                    let mut drop: Option<Ref<Control>> = None;
                    if input.visual_root().is_none() {
                        drag_log!(
                            "Move deferred: active dock control not attached to visual tree."
                        );
                        return;
                    }
                    let screen = input.point_to_screen(point);
                    let mut preview = "None";
                    for control in DockHelpers::get_z_ordered_dock_controls(controls) {
                        if control.visual_root().is_none() {
                            drag_log!(&format!(
                                "Skipping dock control '{}': not attached to visual tree.",
                                control.get_type().name()
                            ));
                            continue;
                        }
                        let client = control.point_to_client(screen);
                        drag_log!(&format!("Testing dock control '{}' (Bounds={}) at client point {client} (screen {screen}).",control.get_type().name(),control.bounds()));
                        drop = DockHelpers::get_control_including_external(
                            &control,
                            client,
                            DockProperties::is_drop_area_property(),
                        );
                        if let Some(_drop_hit) = &drop {
                            target_point = client;
                            target = Some(control.clone().upcast());
                            drag_log!(&format!(
                                "Found drop control '{}' in dock '{}'.",
                                _drop_hit.get_type().name(),
                                control.get_type().name()
                            ));
                            break;
                        } else {
                            DockHelpers::log_drop_areas(
                                Some(&control),
                                &format!("Inspect {} at {client}", control.get_type().name()),
                            );
                        }
                    }
                    if drop.is_none() {
                        drop = DockHelpers::get_control_including_external(
                            input,
                            point,
                            DockProperties::is_drop_area_property(),
                        );
                        if drop.is_some() {
                            target_point = point;
                            target = Some(input.to_ref().upcast());
                            drag_log!(&format!(
                                "Using active dock control '{}' as drop target.",
                                input.get_type().name()
                            ));
                        } else {
                            DockHelpers::log_drop_areas(
                                Some(input),
                                &format!("ActiveDockControl fallback at point {point}"),
                            );
                        }
                    }
                    if let (Some(drop), Some(target)) = (drop, target) {
                        if drop.get_value(DockProperties::is_drop_enabled_property()) {
                            let resolution;
                            if self.base.drop_control().as_ref() == Some(&drop) {
                                self.context.target_point.set(target_point);
                                self.context.set_target_dock_control(Some(target.clone()));
                                resolution = self.over(target_point, action, &drop, &target);
                                drag_log!(&format!(
                                    "Dragging over '{}' at {target_point}.",
                                    drop.get_type().name()
                                ));
                            } else {
                                if self.base.drop_control().is_some() {
                                    self.leave();
                                    self.base.drop_control.replace(None);
                                    drag_log!("Cleared previous drop control before assigning new target.");
                                }
                                self.base.drop_control.replace(Some(drop.clone()));
                                self.context.target_point.set(target_point);
                                self.context.set_target_dock_control(Some(target.clone()));
                                self.enter(target_point, action, &target);
                                resolution = self.over(target_point, action, &drop, &target);
                                drag_log!(&format!(
                                    "New drop control '{}' at {target_point}.",
                                    drop.get_type().name()
                                ));
                            }
                            drag_log!(&format!("Operations resolved: global={:?}, local={:?}, selected={:?}, useGlobal={}.",resolution.global_operation,resolution.local_operation,resolution.selected_operation,if resolution.use_global_operation {"True"} else {"False"}));
                            preview = if resolution.is_valid {
                                if resolution.use_global_operation {
                                    "Dock"
                                } else if resolution.selected_operation == DockOperation::Window {
                                    "Float"
                                } else {
                                    "Dock"
                                }
                            } else {
                                "None"
                            };
                            drag_log!(&format!(
                                "{} validation {} for operation {:?}.",
                                if resolution.use_global_operation {
                                    "Global"
                                } else {
                                    "Local"
                                },
                                if resolution.is_valid {
                                    "succeeded"
                                } else {
                                    "failed"
                                },
                                resolution.selected_operation
                            ));
                        } else {
                            drag_log!(&format!(
                                "Drop area '{}' is disabled.",
                                drop.get_type().name()
                            ));
                            if self.base.drop_control().is_some() {
                                self.leave();
                                self.base.drop_control.replace(None);
                                self.context.target_point.set(Point::default());
                                self.context.set_target_dock_control(None);
                                self.context.clear_resolved_operation();
                                drag_log!("Cleared drop control due to disabled target.");
                            }
                        }
                    } else {
                        self.leave();
                        self.base.drop_control.replace(None);
                        self.context.target_point.set(Point::default());
                        self.context.set_target_dock_control(None);
                        self.context.clear_resolved_operation();
                        drag_log!(&format!("No valid drop target at current position (local={point}, screen={screen}); cleared drop context."));
                        let can_float = self
                            .context
                            .drag_control()
                            .and_then(|c| model_projection::dockable(c.data_context().as_ref()))
                            .is_some_and(Self::can_float_dockable);
                        preview = if can_float { "Float" } else { "None" };
                    }
                    if preview == "None"
                        && self
                            .context
                            .drag_control()
                            .and_then(|c| model_projection::dockable(c.data_context().as_ref()))
                            .is_some_and(Self::can_float_dockable)
                    {
                        preview = "Float";
                        drag_log!("Preview overridden to Float because source can float.");
                    }
                    drag_log!(&format!("Preview state set to '{preview}'."));
                    self.drag_preview_helper
                        .move_(screen, self.context.drag_offset.get(), preview);
                }
            }
            EventType::CaptureLost => {
                self.drag_preview_helper.hide();
                self.leave();
                self.context.end();
                self.base.drop_control.replace(None);
                input.set_is_dragging_dock(false);
            }
            EventType::Enter | EventType::Leave | EventType::WheelChanged => {}
        }
    }
}

pub(crate) struct DockDragContext {
    drag_control: RefCell<Option<Ref<Control>>>,
    pub(crate) drag_start_point: Cell<Point>,
    pub(crate) pointer_pressed: Cell<bool>,
    pub(crate) do_drag_drop: Cell<bool>,
    pub(crate) target_point: Cell<Point>,
    target_dock_control: RefCell<Option<Ref<Visual>>>,
    pub(crate) resolved_operation: Cell<DockOperation>,
    pub(crate) use_global_operation: Cell<bool>,
    pub(crate) has_resolved_operation: Cell<bool>,
    pub(crate) drag_offset: Cell<PixelPoint>,
}
impl DockDragContext {
    pub(crate) fn new() -> Self {
        Self {
            drag_control: RefCell::new(None),
            drag_start_point: Cell::new(Point::default()),
            pointer_pressed: Cell::new(false),
            do_drag_drop: Cell::new(false),
            target_point: Cell::new(Point::default()),
            target_dock_control: RefCell::new(None),
            resolved_operation: Cell::new(DockOperation::None),
            use_global_operation: Cell::new(false),
            has_resolved_operation: Cell::new(false),
            drag_offset: Cell::new(PixelPoint::default()),
        }
    }
    pub(crate) fn drag_control(&self) -> Option<Ref<Control>> {
        self.drag_control.borrow().clone()
    }
    pub(crate) fn target_dock_control(&self) -> Option<Ref<Visual>> {
        self.target_dock_control.borrow().clone()
    }
    pub(crate) fn set_target_dock_control(&self, value: Option<Ref<Visual>>) {
        let old = self.target_dock_control.replace(value);
        drop(old);
    }
    pub(crate) fn start(&self, drag: Ref<Control>, point: Point) {
        self.drag_control.replace(Some(drag));
        self.drag_start_point.set(point);
        self.pointer_pressed.set(true);
        self.do_drag_drop.set(false);
        self.target_point.set(Point::default());
        self.target_dock_control.replace(None);
        self.clear_resolved_operation();
    }
    pub(crate) fn end(&self) {
        self.drag_control.replace(None);
        self.drag_start_point.set(Point::default());
        self.pointer_pressed.set(false);
        self.do_drag_drop.set(false);
        self.target_point.set(Point::default());
        self.target_dock_control.replace(None);
        self.clear_resolved_operation();
    }
    pub(crate) fn clear_resolved_operation(&self) {
        self.resolved_operation.set(DockOperation::None);
        self.use_global_operation.set(false);
        self.has_resolved_operation.set(false);
    }
    pub(crate) fn copy_to(&self, target: &Self) {
        target.drag_control.replace(self.drag_control());
        target.drag_start_point.set(self.drag_start_point.get());
        target.pointer_pressed.set(self.pointer_pressed.get());
        target.do_drag_drop.set(self.do_drag_drop.get());
        target.target_point.set(self.target_point.get());
        target
            .target_dock_control
            .replace(self.target_dock_control());
        target.resolved_operation.set(self.resolved_operation.get());
        target
            .use_global_operation
            .set(self.use_global_operation.get());
        target
            .has_resolved_operation
            .set(self.has_resolved_operation.get());
        target.drag_offset.set(self.drag_offset.get());
    }
}
// Exact numeric continuations extracted from original Drop/GetPreferredPreviewSize.
pub(crate) fn round_screen_point(point: Point) -> PixelPoint {
    PixelPoint::new(
        point.x.round_ties_even() as i32,
        point.y.round_ties_even() as i32,
    )
}
pub(crate) fn max_preview_dimension(value: f64) -> f64 {
    if value.is_nan() {
        value
    } else if value <= 0.0 {
        0.0
    } else {
        value
    }
}

#[cfg(test)]
#[path = "dock_drag_context_tests.rs"]
mod tests;
