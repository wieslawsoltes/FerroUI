// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{
    dock_helpers::DockHelpers, dock_manager_state::DockManagerState, event_type::EventType,
    services::i_global_docking_service::IGlobalDockingService,
};
use crate::{
    controls::{DockControl, DockTarget, GlobalDockTarget, HostWindow},
    model_projection,
};
use ferrodock_model::{
    core::{
        DockCapability, DockGroupValidator, DockInheritanceHelper, DockOperation, DragAction,
        IDock, IDockManager, IDockManagerState, IHostWindowState,
    },
    DockCapabilityResolver,
};
use ferrodock_settings::{DockProperties, DockSettings, WindowDragDockScope};
use ferroui_base::{PixelPoint, Point, Ref, Visual};
use ferroui_controls::Control;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
macro_rules! host_reject {
    ($stage:expr,$message:expr $(,)?) => {{
        #[cfg(debug_assertions)]
        DockManagerState::log_drop_rejection($stage, $message);
    }};
}
struct WindowDragContext {
    drag_start_point: Cell<PixelPoint>,
    window_start_position: Cell<PixelPoint>,
    window_offset: Cell<PixelPoint>,
    pointer_pressed: Cell<bool>,
    do_drag_drop: Cell<bool>,
    target_dock_control: RefCell<Option<Ref<DockControl>>>,
    target_point: Cell<Point>,
    drag_action: Cell<DragAction>,
}
impl WindowDragContext {
    fn new() -> Self {
        Self {
            drag_start_point: Cell::new(PixelPoint::default()),
            window_start_position: Cell::new(PixelPoint::default()),
            window_offset: Cell::new(PixelPoint::default()),
            pointer_pressed: Cell::new(false),
            do_drag_drop: Cell::new(false),
            target_dock_control: RefCell::new(None),
            target_point: Cell::new(Point::default()),
            drag_action: Cell::new(DragAction::NONE),
        }
    }
    fn start(&self, point: PixelPoint, window_position: PixelPoint, window_offset: PixelPoint) {
        self.drag_start_point.set(point);
        self.window_start_position.set(window_position);
        self.window_offset.set(window_offset);
        self.pointer_pressed.set(true);
        self.do_drag_drop.set(false);
        self.target_dock_control.replace(None);
        self.target_point.set(Point::default());
        self.drag_action.set(DragAction::MOVE);
    }
    fn end(&self) {
        self.drag_start_point.set(PixelPoint::default());
        self.window_start_position.set(PixelPoint::default());
        self.window_offset.set(PixelPoint::default());
        self.pointer_pressed.set(false);
        self.do_drag_drop.set(false);
        self.target_dock_control.replace(None);
        self.target_point.set(Point::default());
        self.drag_action.set(DragAction::MOVE);
    }
    fn target(&self) -> Option<Ref<DockControl>> {
        self.target_dock_control.borrow().clone()
    }
}
pub(crate) struct HostWindowState {
    base: DockManagerState,
    // Source readonly host reference is STRONG; do not hide the native Rc/CLR GC cycle.
    host_window: Ref<HostWindow>,
    context: WindowDragContext,
}
impl IDockManagerState for HostWindowState {}
impl IHostWindowState for HostWindowState {}
impl HostWindowState {
    pub(crate) fn new(
        manager: Rc<dyn IDockManager>,
        host: Ref<HostWindow>,
        global: Option<Rc<dyn IGlobalDockingService>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: DockManagerState::new(manager, global),
            host_window: host,
            context: WindowDragContext::new(),
        })
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
    fn local_operation(
        &self,
        point: Point,
        action: DragAction,
        drop: &Control,
        relative: &Visual,
        fallback: DockOperation,
    ) -> DockOperation {
        let adorner = self.base.local_adorner_helper.adorner.borrow().clone();
        adorner
            .and_then(|a| a.cast::<DockTarget>())
            .map_or(fallback, |t| {
                t.get_dock_operation(
                    point,
                    drop,
                    relative,
                    action,
                    &|p, o, a, r| self.validate_local(p, o, a, r),
                    Some(&|p, o, a, r| self.is_dock_target_visible(p, o, a, r)),
                )
            })
    }
    fn global_operation(
        &self,
        point: Point,
        action: DragAction,
        drop: &Control,
        relative: &Visual,
    ) -> DockOperation {
        let adorner = self.base.global_adorner_helper.adorner.borrow().clone();
        adorner
            .and_then(|a| a.cast::<GlobalDockTarget>())
            .map_or(DockOperation::None, |t| {
                t.get_dock_operation(
                    point,
                    drop,
                    relative,
                    action,
                    &|p, o, a, r| self.validate_global(p, o, a, r),
                    Some(&|p, o, a, r| self.is_dock_target_visible(p, o, a, r)),
                )
            })
    }
    fn over(&self, point: Point, action: DragAction, drop: &Control, relative: &Visual) {
        self.base.update_adorner_geometry();
        let local = self.local_operation(point, action, drop, relative, DockOperation::Fill);
        let global = self.global_operation(point, action, drop, relative);
        if global != DockOperation::None {
            self.validate_global(point, global, action, relative);
        } else if local != DockOperation::Window {
            self.validate_local(point, local, action, relative);
        }
        self.base
            .local_adorner_helper
            .set_global_dock_active(global != DockOperation::None);
    }
    fn drop(&self, point: Point, action: DragAction, drop: &Control, relative: &Visual) -> bool {
        self.base.update_adorner_geometry();
        let local = self.local_operation(point, action, drop, relative, DockOperation::Window);
        let global = self.global_operation(point, action, drop, relative);
        self.base.remove_adorners();
        if self.base.drop_control().is_none() {
            return false;
        }
        let layout = self.host_window.window().and_then(|w| w.layout());
        if global != DockOperation::None {
            let Some(drop) = self.base.drop_control() else {
                return false;
            };
            if let Some(source) = layout.and_then(|l| l.active_dockable()) {
                if let Some(target) = self.base.resolve_global_target_dock(Some(&drop)) {
                    if !self
                        .base
                        .validate_global_target(source.clone(), target.clone())
                    {
                        return false;
                    }
                    self.base
                        .execute(point, global, action, relative, source, target);
                    return true;
                }
            }
        } else if let Some(source) = layout.and_then(|l| l.active_dockable()) {
            if let Some(target) = self
                .base
                .drop_control()
                .and_then(|c| model_projection::dockable(c.data_context().as_ref()))
            {
                if local != DockOperation::Window {
                    self.base
                        .execute(point, local, action, relative, source, target);
                    return true;
                }
            }
        }
        false
    }
    fn leave(&self) {
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
            host_reject!("ValidateLocal", "Docking is disabled.");
            return false;
        }
        let Some(source) = self
            .host_window
            .window()
            .and_then(|w| w.layout())
            .and_then(|l| l.focused_dockable())
        else {
            host_reject!(
                "ValidateLocal",
                "No focused dockable available for local validation."
            );
            return false;
        };
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
        let target = self
            .base
            .drop_control()
            .is_some_and(|c| c.get_value(DockProperties::is_dock_target_property()));
        if !target {
            // The original local computation remains outside the conditional debug call.
            let _control_name = if self.base.drop_control().is_none() {
                "null"
            } else {
                self.base
                    .drop_control()
                    .expect("source drop re-read")
                    .get_type()
                    .name()
            };
            host_reject!(
                "ValidateLocal",
                &format!(
                    "Control '{}' is not marked as a dock target.",
                    _control_name
                )
            );
        }
        target
    }
    fn validate_global(
        &self,
        point: Point,
        operation: DockOperation,
        action: DragAction,
        relative: &Visual,
    ) -> bool {
        if !self.base.dock_manager.is_docking_enabled() {
            host_reject!("ValidateGlobal", "Docking is disabled.");
            return false;
        }
        let Some(source) = self
            .host_window
            .window()
            .and_then(|w| w.layout())
            .and_then(|l| l.focused_dockable())
        else {
            host_reject!(
                "ValidateGlobal",
                "No focused dockable available for global validation."
            );
            return false;
        };
        let Some(drop) = self.base.drop_control() else {
            host_reject!(
                "ValidateGlobal",
                "No DropControl available for global validation."
            );
            return false;
        };
        let Some(target) = self.base.resolve_global_target_dock(Some(&drop)) else {
            host_reject!(
                "ValidateGlobal",
                "Unable to resolve global docking target for DropControl."
            );
            return false;
        };
        if !DockInheritanceHelper::get_effective_enable_global_docking(target.clone()) {
            host_reject!(
                "ValidateGlobal",
                &format!("Global docking is disabled for dock '{}'.", target.title())
            );
            return false;
        }
        self.base
            .dock_manager
            .set_position(DockHelpers::to_dock_point(point));
        if relative.visual_root().is_none() {
            host_reject!(
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
            host_reject!(
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
            host_reject!(
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
            .host_window
            .window()
            .and_then(|w| w.layout())
            .and_then(|l| l.focused_dockable())
        else {
            return true;
        };
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
    pub(crate) fn process(&self, point: PixelPoint, phase: EventType) {
        if !self.base.dock_manager.is_docking_enabled() {
            if self.context.pointer_pressed.get() || self.context.do_drag_drop.get() {
                self.leave();
                self.context.end();
                self.base.drop_control.replace(None);
            }
            return;
        }
        match phase {
            EventType::Pressed => {
                if !self
                    .host_window
                    .get_value(DockProperties::is_drag_enabled_property())
                {
                    return;
                }
                if let Some(dockable) =
                    model_projection::dockable(self.host_window.data_context().as_ref())
                {
                    if !DockCapabilityResolver::is_enabled(
                        Some(dockable.clone()),
                        DockCapability::Drag,
                        DockCapabilityResolver::resolve_operation_dock(Some(dockable)),
                    ) {
                        return;
                    }
                }
                let screen = self.host_window.point_to_screen(Point::default());
                let offset = PixelPoint::new(
                    screen.x.wrapping_sub(self.host_window.position().x),
                    screen.y.wrapping_sub(self.host_window.position().y),
                );
                self.context.end();
                self.base.drop_control.replace(None);
                self.leave();
                self.context
                    .start(point, self.host_window.position(), offset);
                self.base.drop_control.replace(None);
            }
            EventType::Released => {
                if self.context.do_drag_drop.get() {
                    let mut executed = false;
                    if self.context.target().is_some() && self.base.drop_control().is_some() {
                        let mut enabled = true;
                        if let Some(drop) = self.base.drop_control() {
                            enabled = drop.get_value(DockProperties::is_drop_enabled_property());
                        }
                        if enabled {
                            let drop = self.base.drop_control().expect("source drop re-read");
                            let target = self.context.target().expect("source target re-read");
                            executed = self.drop(
                                self.context.target_point.get(),
                                self.context.drag_action.get(),
                                &drop,
                                &target,
                            );
                        }
                    }
                    if !executed {
                        if let Some(dockable) =
                            model_projection::dockable(self.host_window.data_context().as_ref())
                        {
                            if DockCapabilityResolver::is_enabled(
                                Some(dockable.clone()),
                                DockCapability::Float,
                                DockCapabilityResolver::resolve_operation_dock(Some(
                                    dockable.clone(),
                                )),
                            ) {
                                if let Some(factory) =
                                    self.host_window.window().and_then(|w| w.factory())
                                {
                                    dockable.set_pointer_screen_position(
                                        point.x as f64,
                                        point.y as f64,
                                    );
                                    factory.float_dockable(Some(dockable));
                                }
                            }
                        }
                    }
                }
                self.leave();
                self.context.end();
                self.base.drop_control.replace(None);
            }
            EventType::Moved => {
                if !self.context.pointer_pressed.get() {
                    return;
                }
                if !self.context.do_drag_drop.get() {
                    let diff = point - self.context.window_start_position.get();
                    if DockSettings::is_minimum_drag_distance_pixels(diff) {
                        self.context.do_drag_drop.set(true);
                    }
                }
                if !self.context.do_drag_drop.get() {
                    return;
                }
                let Some(factory) = self
                    .host_window
                    .window()
                    .and_then(|w| w.layout())
                    .and_then(|l| l.factory())
                else {
                    self.leave();
                    self.base.drop_control.replace(None);
                    self.context.target_dock_control.replace(None);
                    self.context.target_point.set(Point::default());
                    return;
                };
                let mut found = false;
                for control in DockHelpers::get_z_ordered_dock_controls(factory.dock_controls()) {
                    if control.layout()
                        == self
                            .host_window
                            .window()
                            .and_then(|w| w.layout())
                            .map(|r| r as Rc<dyn IDock>)
                    {
                        continue;
                    }
                    let screen = PixelPoint::new(
                        point
                            .x
                            .wrapping_add(self.context.window_offset.get().x)
                            .wrapping_add(self.context.drag_start_point.get().x),
                        point
                            .y
                            .wrapping_add(self.context.window_offset.get().y)
                            .wrapping_add(self.context.drag_start_point.get().y),
                    );
                    if control.visual_root().is_none() {
                        continue;
                    }
                    let client = control.point_to_client(screen);
                    let drop = if self.host_window.window_drag_dock_scope()
                        == WindowDragDockScope::WindowChrome
                    {
                        DockHelpers::get_external_control(
                            &control,
                            client,
                            DockProperties::is_drop_area_property(),
                        )
                    } else {
                        DockHelpers::get_control_including_external(
                            &control,
                            client,
                            DockProperties::is_drop_area_property(),
                        )
                    };
                    if let Some(drop) = drop {
                        if !drop.get_value(DockProperties::is_drop_enabled_property()) {
                            self.leave();
                            self.context.target_dock_control.replace(None);
                            self.context.target_point.set(Point::default());
                            self.base.drop_control.replace(None);
                        } else {
                            if self.base.drop_control().as_ref() == Some(&drop) {
                                self.context.target_dock_control.replace(Some(control));
                                self.context.target_point.set(client);
                                self.base.drop_control.replace(Some(drop.clone()));
                                self.context.drag_action.set(DragAction::MOVE);
                                let target = self.context.target().expect("source target re-read");
                                self.over(
                                    self.context.target_point.get(),
                                    self.context.drag_action.get(),
                                    &drop,
                                    &target,
                                );
                                found = true;
                                break;
                            }
                            if self.base.drop_control().is_some() {
                                self.leave();
                                self.base.drop_control.replace(None);
                            }
                            self.context.target_dock_control.replace(Some(control));
                            self.context.target_point.set(client);
                            self.base.drop_control.replace(Some(drop));
                            self.context.drag_action.set(DragAction::MOVE);
                            let target = self.context.target().expect("source target re-read");
                            self.enter(
                                self.context.target_point.get(),
                                self.context.drag_action.get(),
                                &target,
                            );
                            found = true;
                            break;
                        }
                    }
                }
                if !found && self.base.drop_control().is_some() {
                    self.leave();
                    self.base.drop_control.replace(None);
                    self.context.target_dock_control.replace(None);
                    self.context.target_point.set(Point::default());
                }
            }
            EventType::CaptureLost => {
                self.context.end();
                self.base.drop_control.replace(None);
                self.leave();
            }
            EventType::Enter | EventType::Leave | EventType::WheelChanged => {}
        }
    }
}
