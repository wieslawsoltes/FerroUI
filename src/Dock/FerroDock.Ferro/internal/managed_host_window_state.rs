// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{
    dock_helpers::DockHelpers, dock_manager_state::DockManagerState,
    drag_preview_helper::DragPreviewHelper, event_type::EventType,
    managed_window_registry::ManagedWindowRegistry,
    services::i_global_docking_service::IGlobalDockingService,
};
use crate::{
    controls::{
        DockControl, DockTarget, GlobalDockTarget, ManagedDockWindowDocument, ManagedHostWindow,
        ManagedWindowLayer,
    },
    model_projection,
};
use ferrodock_controls_recycling::from_recycling_value;
use ferrodock_model::{
    core::{
        DockCapability, DockGroupValidator, DockInheritanceHelper, DockOperation, DragAction,
        IDock, IDockManager, IDockManagerState, IDockable, IHostWindow, IHostWindowState,
    },
    DockCapabilityResolver,
};
use ferrodock_settings::{DockProperties, DockSettings};
use ferroui_base::{data::core::ValueTypes, PixelPoint, Point, Ref, Visual};
use ferroui_controls::Control;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
struct ManagedWindowDragContext {
    drag_start_point: Cell<PixelPoint>,
    drag_offset: Cell<PixelPoint>,
    drag_dockable: RefCell<Option<Rc<dyn IDockable>>>,
    pointer_pressed: Cell<bool>,
    do_drag_drop: Cell<bool>,
    target_dock_control: RefCell<Option<Ref<DockControl>>>,
    target_point: Cell<Point>,
    drag_action: Cell<DragAction>,
}
impl ManagedWindowDragContext {
    fn new() -> Self {
        Self {
            drag_start_point: Cell::new(PixelPoint::default()),
            drag_offset: Cell::new(PixelPoint::default()),
            drag_dockable: RefCell::new(None),
            pointer_pressed: Cell::new(false),
            do_drag_drop: Cell::new(false),
            target_dock_control: RefCell::new(None),
            target_point: Cell::new(Point::default()),
            drag_action: Cell::new(DragAction::MOVE),
        }
    }
    fn start(&self, point: PixelPoint) {
        self.drag_start_point.set(point);
        self.drag_offset.set(PixelPoint::default());
        self.drag_dockable.replace(None);
        self.pointer_pressed.set(true);
        self.do_drag_drop.set(false);
        self.target_dock_control.replace(None);
        self.target_point.set(Point::default());
        self.drag_action.set(DragAction::MOVE);
    }
    fn end(&self) {
        self.drag_start_point.set(PixelPoint::default());
        self.drag_offset.set(PixelPoint::default());
        self.drag_dockable.replace(None);
        self.pointer_pressed.set(false);
        self.do_drag_drop.set(false);
        self.target_dock_control.replace(None);
        self.target_point.set(Point::default());
        self.drag_action.set(DragAction::MOVE);
    }
    fn dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.drag_dockable.borrow().clone()
    }
    fn target(&self) -> Option<Ref<DockControl>> {
        self.target_dock_control.borrow().clone()
    }
}
pub(crate) struct ManagedHostWindowState {
    base: DockManagerState,
    host_window: Rc<ManagedHostWindow>,
    context: ManagedWindowDragContext,
    drag_preview_helper: DragPreviewHelper,
}
impl IDockManagerState for ManagedHostWindowState {}
impl IHostWindowState for ManagedHostWindowState {}
impl ManagedHostWindowState {
    pub(crate) fn new(
        manager: Rc<dyn IDockManager>,
        host: Rc<ManagedHostWindow>,
        global: Option<Rc<dyn IGlobalDockingService>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: DockManagerState::new(manager, global),
            host_window: host,
            context: ManagedWindowDragContext::new(),
            drag_preview_helper: DragPreviewHelper::new(),
        })
    }
    pub(crate) fn process(&self, screen: PixelPoint, phase: EventType) {
        if !self.base.dock_manager.is_docking_enabled() {
            if self.context.pointer_pressed.get() || self.context.do_drag_drop.get() {
                self.drag_preview_helper.hide();
                self.leave();
                self.context.end();
                self.base.drop_control.replace(None);
            }
            return;
        }
        match phase {
            EventType::Pressed => {
                self.context.start(screen);
                self.context.drag_dockable.replace(
                    self.resolve_managed_dockable()
                        .map(|d| d as Rc<dyn IDockable>),
                );
                self.context.drag_offset.set(Self::calculate_drag_offset(
                    self.context.dockable().and_then(Self::managed_document),
                    screen,
                ));
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
                            executed = self.drop(
                                self.context.target_point.get(),
                                self.context.drag_action.get(),
                                &self.base.drop_control().expect("source drop control"),
                                &self.context.target().expect("source target"),
                            );
                        }
                    }
                    if !executed {
                        if let Some(item) = self.context.dockable() {
                            if Self::managed_document(item.clone()).is_none()
                                && DockCapabilityResolver::is_enabled(
                                    Some(item.clone()),
                                    DockCapability::Float,
                                    DockCapabilityResolver::resolve_operation_dock(Some(
                                        item.clone(),
                                    )),
                                )
                            {
                                if let Some(factory) =
                                    self.host_window.window().and_then(|w| w.factory())
                                {
                                    item.set_pointer_screen_position(
                                        screen.x as f64,
                                        screen.y as f64,
                                    );
                                    factory.float_dockable(Some(item));
                                }
                            }
                        }
                    }
                }
                self.drag_preview_helper.hide();
                self.leave();
                self.context.end();
                self.base.drop_control.replace(None);
            }
            EventType::Moved => {
                if !self.context.pointer_pressed.get() {
                    return;
                }
                if !self.context.do_drag_drop.get() {
                    let start = self.context.drag_start_point.get();
                    let diff = PixelPoint::new(
                        screen.x.wrapping_sub(start.x),
                        screen.y.wrapping_sub(start.y),
                    );
                    if DockSettings::is_minimum_drag_distance_pixels(diff) {
                        if self.context.dockable().is_none() {
                            self.context.drag_dockable.replace(
                                self.resolve_managed_dockable()
                                    .map(|d| d as Rc<dyn IDockable>),
                            );
                            self.context.drag_offset.set(Self::calculate_drag_offset(
                                self.context.dockable().and_then(Self::managed_document),
                                self.context.drag_start_point.get(),
                            ));
                        }
                        let show = Self::should_show_managed_drag_preview(self.context.dockable());
                        if let Some(item) = self.context.dockable().filter(|_| show) {
                            let context = Self::managed_document(item.clone())
                                .and_then(|d| Self::document_visual(&d));
                            self.drag_preview_helper.show(
                                item,
                                screen,
                                self.context.drag_offset.get(),
                                context.as_deref(),
                                None,
                            );
                        }
                        self.context.do_drag_drop.set(true);
                    }
                }
                if !self.context.do_drag_drop.get() {
                    return;
                }
                let mut preview = "None";
                let Some(factory) = self
                    .host_window
                    .window()
                    .and_then(|w| w.layout())
                    .and_then(|l| l.factory())
                else {
                    self.drag_preview_helper.hide();
                    self.leave();
                    self.base.drop_control.replace(None);
                    self.context.target_dock_control.replace(None);
                    self.context.target_point.set(Point::default());
                    return;
                };
                let mut found = false;
                for dock in DockHelpers::get_z_ordered_dock_controls(factory.dock_controls()) {
                    let dock_layout = dock.layout();
                    let host_layout = self
                        .host_window
                        .window()
                        .and_then(|w| w.layout())
                        .map(|r| r as Rc<dyn IDock>);
                    if dock_layout == host_layout {
                        continue;
                    }
                    if dock.visual_root().is_none() {
                        continue;
                    }
                    let point = dock.point_to_client(screen);
                    if let Some(drop) = DockHelpers::get_control_including_external(
                        &dock,
                        point,
                        DockProperties::is_drop_area_property(),
                    ) {
                        if !drop.get_value(DockProperties::is_drop_enabled_property()) {
                            self.leave();
                            self.context.target_dock_control.replace(None);
                            self.context.target_point.set(Point::default());
                            self.base.drop_control.replace(None);
                        } else {
                            if self.base.drop_control().as_ref() == Some(&drop) {
                                self.context.target_dock_control.replace(Some(dock));
                                self.context.target_point.set(point);
                                self.base.drop_control.replace(Some(drop));
                                self.context.drag_action.set(DragAction::MOVE);
                                self.over(
                                    self.context.target_point.get(),
                                    self.context.drag_action.get(),
                                    &self.base.drop_control().expect("source current drop"),
                                    &self.context.target().expect("source target"),
                                );
                                found = true;
                                break;
                            }
                            if self.base.drop_control().is_some() {
                                self.leave();
                                self.base.drop_control.replace(None);
                            }
                            self.context.target_dock_control.replace(Some(dock));
                            self.context.target_point.set(point);
                            self.base.drop_control.replace(Some(drop));
                            self.context.drag_action.set(DragAction::MOVE);
                            self.enter(
                                self.context.target_point.get(),
                                self.context.drag_action.get(),
                                &self.context.target().expect("source target"),
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
                if self.base.drop_control().is_some() {
                    if let Some(target) = self.context.target() {
                        if self
                            .base
                            .drop_control()
                            .expect("source drop")
                            .get_value(DockProperties::is_drop_enabled_property())
                        {
                            let global = self.global_operation(
                                self.context.target_point.get(),
                                &self.base.drop_control().expect("source drop"),
                                &target,
                                self.context.drag_action.get(),
                            );
                            let local = self.local_operation(
                                self.context.target_point.get(),
                                &self.base.drop_control().expect("source drop"),
                                &target,
                                self.context.drag_action.get(),
                                DockOperation::Fill,
                            );
                            if global != DockOperation::None {
                                preview = if self.validate_global(
                                    self.context.target_point.get(),
                                    global,
                                    self.context.drag_action.get(),
                                    &target,
                                ) {
                                    "Dock"
                                } else {
                                    "None"
                                };
                            } else {
                                let valid = self.validate_local(
                                    self.context.target_point.get(),
                                    local,
                                    self.context.drag_action.get(),
                                    &target,
                                );
                                preview = if valid {
                                    if local == DockOperation::Window {
                                        "Float"
                                    } else {
                                        "Dock"
                                    }
                                } else {
                                    "None"
                                };
                            }
                        }
                    }
                }
                let source = self
                    .host_window
                    .window()
                    .and_then(|w| w.layout())
                    .and_then(|l| l.focused_dockable())
                    .or_else(|| {
                        self.host_window
                            .window()
                            .and_then(|w| w.layout())
                            .and_then(|l| l.active_dockable())
                    });
                if preview == "None"
                    && source.is_some_and(|d| {
                        DockCapabilityResolver::is_enabled(
                            Some(d.clone()),
                            DockCapability::Float,
                            DockCapabilityResolver::resolve_operation_dock(Some(d)),
                        )
                    })
                {
                    preview = "Float";
                }
                if Self::should_show_managed_drag_preview(self.context.dockable()) {
                    self.drag_preview_helper
                        .move_(screen, self.context.drag_offset.get(), preview);
                }
            }
            EventType::CaptureLost => {
                self.drag_preview_helper.hide();
                self.leave();
                self.context.end();
                self.base.drop_control.replace(None);
            }
            _ => {}
        }
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
        drop: &Control,
        relative: &Visual,
        action: DragAction,
        fallback: DockOperation,
    ) -> DockOperation {
        let adorner = self.base.local_adorner_helper.adorner.borrow().clone();
        adorner
            .and_then(|a| a.cast::<DockTarget>())
            .map_or(fallback, |target| {
                target.get_dock_operation(
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
        drop: &Control,
        relative: &Visual,
        action: DragAction,
    ) -> DockOperation {
        let adorner = self.base.global_adorner_helper.adorner.borrow().clone();
        adorner
            .and_then(|a| a.cast::<GlobalDockTarget>())
            .map_or(DockOperation::None, |target| {
                target.get_dock_operation(
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
        let local = self.local_operation(point, drop, relative, action, DockOperation::Fill);
        let global = self.global_operation(point, drop, relative, action);
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
        let local = self.local_operation(point, drop, relative, action, DockOperation::Window);
        let global = self.global_operation(point, drop, relative, action);
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
            return false;
        }
        let Some(source) = self
            .host_window
            .window()
            .and_then(|w| w.layout())
            .and_then(|l| l.focused_dockable())
        else {
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
        self.base
            .drop_control()
            .is_some_and(|c| c.get_value(DockProperties::is_dock_target_property()))
    }
    fn validate_global(
        &self,
        point: Point,
        operation: DockOperation,
        action: DragAction,
        relative: &Visual,
    ) -> bool {
        if !self.base.dock_manager.is_docking_enabled() {
            return false;
        }
        let Some(source) = self
            .host_window
            .window()
            .and_then(|w| w.layout())
            .and_then(|l| l.focused_dockable())
        else {
            return false;
        };
        let Some(drop) = self.base.drop_control() else {
            return false;
        };
        let Some(target) = self.base.resolve_global_target_dock(Some(&drop)) else {
            return false;
        };
        if !DockInheritanceHelper::get_effective_enable_global_docking(target.clone()) {
            return false;
        }
        self.base
            .dock_manager
            .set_position(DockHelpers::to_dock_point(point));
        if relative.visual_root().is_none() {
            return false;
        }
        let screen = DockHelpers::get_screen_point(relative, point);
        self.base
            .dock_manager
            .set_screen_position(DockHelpers::to_dock_point(screen));
        if !DockGroupValidator::validate_global_docking(source.clone(), target.clone()) {
            return false;
        }
        self.base
            .dock_manager
            .validate_dockable(source, target, action, operation, false)
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
    fn resolve_managed_dockable(&self) -> Option<Rc<ManagedDockWindowDocument>> {
        let factory = self.host_window.window()?.factory()?;
        let dock = ManagedWindowRegistry::get_or_create_dock(&factory);
        for item in dock.visible_dockables()?.iter() {
            if let Some(document) = item.and_then(Self::managed_document) {
                if document.window() == self.host_window.window() {
                    return Some(document);
                }
            }
        }
        None
    }
    fn managed_document(item: Rc<dyn IDockable>) -> Option<Rc<ManagedDockWindowDocument>> {
        item.boxed_reference()
            .downcast_ref::<Rc<ManagedDockWindowDocument>>()
            .cloned()
    }
    fn document_visual(document: &ManagedDockWindowDocument) -> Option<Ref<Visual>> {
        let content = document.content()?;
        ValueTypes::as_object(&*from_recycling_value(&content))?.cast::<Visual>()
    }
    fn should_show_managed_drag_preview(item: Option<Rc<dyn IDockable>>) -> bool {
        DockSettings::show_dockable_preview_on_drag()
            && item.and_then(Self::managed_document).is_none()
    }
    fn calculate_drag_offset(
        document: Option<Rc<ManagedDockWindowDocument>>,
        pointer: PixelPoint,
    ) -> PixelPoint {
        let Some(document) = document else {
            return PixelPoint::default();
        };
        let Some(layer) = Self::try_resolve_managed_layer(Some(&document)) else {
            return PixelPoint::default();
        };
        // Source evaluates MdiBounds separately for X and Y.
        let top =
            layer.point_to_screen(Point::new(document.mdi_bounds().x, document.mdi_bounds().y));
        PixelPoint::new(top.x.wrapping_sub(pointer.x), top.y.wrapping_sub(pointer.y))
    }
    fn try_resolve_managed_layer(
        document: Option<&ManagedDockWindowDocument>,
    ) -> Option<Ref<ManagedWindowLayer>> {
        let document = document?;
        let context = Self::document_visual(document);
        let root = context.as_ref().and_then(|c| c.visual_root());
        if let Some(context) = context {
            let resolved = ManagedWindowLayer::try_get_layer(Some(&context));
            if Self::is_layer_ready(resolved.as_deref())
                && Self::is_layer_in_root(resolved.as_deref(), root.as_deref())
            {
                return resolved;
            }
        }
        if let Some(factory) = document.factory() {
            let registered = ManagedWindowRegistry::try_get_layer(&factory);
            if Self::is_layer_ready(registered.as_deref())
                && Self::is_layer_in_root(registered.as_deref(), root.as_deref())
            {
                return registered;
            }
        }
        None
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
}
