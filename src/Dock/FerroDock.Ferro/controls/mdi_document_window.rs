// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{ManagedDockWindowDocument, MdiLayoutPanel};
use crate::{
    automation::peers::MdiDocumentWindowAutomationPeer,
    internal::{
        event_type::EventType, managed_host_window_access::managed_host,
        window_activation_helper::WindowActivationHelper, window_drag_helper::WindowDragHelper,
    },
    mdi::{ClassicMdiLayoutManager, IMdiLayoutManager, MdiLayoutEntry, MdiResizeDirection},
    model_projection,
};
use ferrodock_model::{
    collections::ListHandle, controls::IMdiDocument, core::*, DockCapabilityResolver,
};
use ferrodock_settings::{DockProperties, DockSettings};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{
        IPointer, InputElement, InputElementImpl, PointerCaptureLostEventArgs, PointerEventArgs,
        PointerPressedEventArgs, PointerReleasedEventArgs,
    },
    instantiate,
    interactivity::{InteractiveImpl, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies},
    layout::LayoutableImpl,
    reactive::{AnonymousObserver, IDisposable},
    styling::ControlTheme,
    BoxedValue, FerroObject, FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Point, Rect, Ref, Size, StyledElement,
    StyledElementImpl, StyledProperty, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{
    automation::peers::AutomationPeer,
    primitives::{
        TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
    },
    templates::IDataTemplate,
    Button, ContextMenu, Control, ControlImpl,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
struct ModelSubscription(Box<dyn Fn()>);
impl ModelSubscription {
    fn dispose(self) {
        (self.0)()
    }
}
struct PointerSubscriptions {
    moved: RoutedEventHandlerToken,
    released: RoutedEventHandlerToken,
    lost: RoutedEventHandlerToken,
}
/// Original templated MDI document window, using the shared layout and docking engine.
#[repr(C)]
pub struct MdiDocumentWindow {
    base: TemplatedControl,
    data_context_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    dock_subscription: RefCell<Option<ModelSubscription>>,
    document_subscription: RefCell<Option<ModelSubscription>>,
    current_document: RefCell<Option<Rc<dyn IMdiDocument>>>,
    current_dock: RefCell<Option<Rc<dyn IDock>>>,
    header: RefCell<Option<Ref<Control>>>,
    drag_handle: RefCell<Option<Ref<Control>>>,
    content_border: RefCell<Option<Ref<Control>>>,
    minimize_button: RefCell<Option<Ref<Button>>>,
    maximize_restore_button: RefCell<Option<Ref<Button>>>,
    content_pressed: RefCell<Option<RoutedEventHandlerToken>>,
    pointer_subscriptions: RefCell<Vec<PointerSubscriptions>>,
    last_non_minimized_state: Cell<MdiWindowState>,
    is_dragging: Cell<bool>,
    is_resizing: Cell<bool>,
    captured_pointer: RefCell<Option<Rc<dyn IPointer>>>,
    drag_start_point: Cell<Point>,
    drag_start_bounds: Cell<Rect>,
    resize_direction: Cell<MdiResizeDirection>,
    last_pointer_position: Cell<Point>,
    managed_window_drag_active: Cell<bool>,
    managed_drag_window: RefCell<Option<Rc<dyn IDockWindow>>>,
}
ferro_class!(MdiDocumentWindow:TemplatedControl);
ferro_class_info!(MdiDocumentWindow {new:MdiDocumentWindow::new,markup:{attributes:[PseudoClasses(":active",":maximized",":minimized")]}});
ferro_impl_classes!(MdiDocumentWindow:StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl);
impl FerroObjectImpl for MdiDocumentWindow {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_pseudo_classes(this.is_active(), this.mdi_state());
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::mdi_state_property().as_property()
            && this.mdi_state() != MdiWindowState::Minimized
        {
            this.last_non_minimized_state.set(this.mdi_state());
        }
        if change.property() == Self::is_active_property().as_property()
            || change.property() == Self::mdi_state_property().as_property()
        {
            this.update_pseudo_classes(this.is_active(), this.mdi_state());
        }
    }
}
impl ControlImpl for MdiDocumentWindow {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        MdiDocumentWindowAutomationPeer::new(this).upcast()
    }
}
impl VisualImpl for MdiDocumentWindow {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let owner = this.to_ref();
        let sub = FerroObject::get_observable(this, StyledElement::data_context_property())
            .subscribe(Rc::new(AnonymousObserver::new(move |v| {
                owner.on_data_context_changed(v)
            })));
        this.data_context_subscription.replace(Some(sub));
        let owner = this.to_ref();
        let moved = this.add_handler_with(
            InputElement::pointer_moved_event(),
            move |_, e| owner.pointer_moved_handler(e),
            RoutingStrategies::TUNNEL,
            false,
        );
        let owner = this.to_ref();
        let released = this.add_handler_with(
            InputElement::pointer_released_event(),
            move |_, e| owner.pointer_released_handler(e),
            RoutingStrategies::TUNNEL,
            false,
        );
        let owner = this.to_ref();
        let lost = this.add_handler_with(
            InputElement::pointer_capture_lost_event(),
            move |_, e| owner.pointer_capture_lost_handler(e),
            RoutingStrategies::TUNNEL,
            false,
        );
        this.pointer_subscriptions
            .borrow_mut()
            .push(PointerSubscriptions {
                moved,
                released,
                lost,
            });
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        let sub = this.data_context_subscription.borrow_mut().take();
        if let Some(sub) = sub {
            sub.dispose();
        }
        this.detach_dock_subscription();
        let sub = this.pointer_subscriptions.borrow_mut().pop();
        if let Some(sub) = sub {
            this.remove_handler(InputElement::pointer_moved_event(), sub.moved);
            this.remove_handler(InputElement::pointer_released_event(), sub.released);
            this.remove_handler(InputElement::pointer_capture_lost_event(), sub.lost);
        }
    }
}
impl TemplatedControlImpl for MdiDocumentWindow {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        let border = this.content_border.borrow().clone();
        let token = this.content_pressed.borrow_mut().take();
        if let Some((border, token)) = border.zip(token) {
            border.remove_handler(InputElement::pointer_pressed_event(), token);
        }
        this.header
            .replace(e.name_scope().find_as::<Control>("PART_Header"));
        this.drag_handle
            .replace(e.name_scope().find_as::<Control>("PART_DragHandle"));
        this.content_border
            .replace(e.name_scope().find_as::<Control>("PART_ContentBorder"));
        this.minimize_button
            .replace(e.name_scope().find_as::<Button>("PART_MinimizeButton"));
        this.maximize_restore_button.replace(
            e.name_scope()
                .find_as::<Button>("PART_MaximizeRestoreButton"),
        );
        let header = this.header.borrow().clone();
        if let Some(header) = header {
            let owner = this.to_ref();
            header.add_handler_with(
                InputElement::pointer_pressed_event(),
                move |_, e| owner.header_pressed_handler(e),
                RoutingStrategies::TUNNEL,
                false,
            );
            let owner = this.to_ref();
            header.add_handler_with(
                InputElement::double_tapped_event(),
                move |_, e| owner.header_double_tapped_handler(e),
                RoutingStrategies::BUBBLE | RoutingStrategies::TUNNEL,
                false,
            );
        }
        let border = this.content_border.borrow().clone();
        if let Some(border) = border {
            let owner = this.to_ref();
            let token = border.add_handler_with(
                InputElement::pointer_pressed_event(),
                move |_, e| owner.content_pressed_handler(e),
                RoutingStrategies::TUNNEL,
                false,
            );
            this.content_pressed.replace(Some(token));
        }
        let button = this.minimize_button.borrow().clone();
        if let Some(button) = button {
            let owner = this.to_ref();
            button.click(move |_, _| owner.minimize_button_clicked());
        }
        let button = this.maximize_restore_button.borrow().clone();
        if let Some(button) = button {
            let owner = this.to_ref();
            button.click(move |_, _| owner.maximize_restore_button_clicked());
        }
        for (name, direction) in [
            ("PART_ResizeTop", MdiResizeDirection::TOP),
            ("PART_ResizeBottom", MdiResizeDirection::BOTTOM),
            ("PART_ResizeLeft", MdiResizeDirection::LEFT),
            ("PART_ResizeRight", MdiResizeDirection::RIGHT),
            (
                "PART_ResizeTopLeft",
                MdiResizeDirection::TOP | MdiResizeDirection::LEFT,
            ),
            (
                "PART_ResizeTopRight",
                MdiResizeDirection::TOP | MdiResizeDirection::RIGHT,
            ),
            (
                "PART_ResizeBottomLeft",
                MdiResizeDirection::BOTTOM | MdiResizeDirection::LEFT,
            ),
            (
                "PART_ResizeBottomRight",
                MdiResizeDirection::BOTTOM | MdiResizeDirection::RIGHT,
            ),
        ] {
            this.attach_resize_handle(e.name_scope().find_as::<Control>(name), direction);
        }
    }
}
ferro_properties! {impl MdiDocumentWindow{
    /// Identifies IconTemplate.
    pub fn icon_template_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("IconTemplate",None)}
    /// Identifies HeaderTemplate.
    pub fn header_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>>{FerroProperty::register::<Self,_>("HeaderTemplate",None)}
    /// Identifies ModifiedTemplate.
    pub fn modified_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>>{FerroProperty::register::<Self,_>("ModifiedTemplate",None)}
    /// Identifies CloseTemplate.
    pub fn close_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>>{FerroProperty::register::<Self,_>("CloseTemplate",None)}
    /// Identifies CloseButtonTheme.
    pub fn close_button_theme_property()->StyledProperty<Option<Ref<ControlTheme>>>{FerroProperty::register::<Self,_>("CloseButtonTheme",None)}
    /// Identifies DocumentContextMenu.
    pub fn document_context_menu_property()->StyledProperty<Option<Ref<ContextMenu>>>{FerroProperty::register::<Self,_>("DocumentContextMenu",None)}
    /// Identifies IsActive.
    pub fn is_active_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsActive",false)}
    /// Identifies MdiState.
    pub fn mdi_state_property()->StyledProperty<MdiWindowState>{FerroProperty::register::<Self,_>("MdiState",MdiWindowState::Normal)}
}}
impl MdiDocumentWindow {
    /// Constructs the original field state for this or a derived native control.
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            data_context_subscription: RefCell::new(None),
            dock_subscription: RefCell::new(None),
            document_subscription: RefCell::new(None),
            current_document: RefCell::new(None),
            current_dock: RefCell::new(None),
            header: RefCell::new(None),
            drag_handle: RefCell::new(None),
            content_border: RefCell::new(None),
            minimize_button: RefCell::new(None),
            maximize_restore_button: RefCell::new(None),
            content_pressed: RefCell::new(None),
            pointer_subscriptions: RefCell::new(Vec::new()),
            last_non_minimized_state: Cell::new(MdiWindowState::Normal),
            is_dragging: Cell::new(false),
            is_resizing: Cell::new(false),
            captured_pointer: RefCell::new(None),
            drag_start_point: Cell::new(Point::default()),
            drag_start_bounds: Cell::new(Rect::default()),
            resize_direction: Cell::new(MdiResizeDirection::NONE),
            last_pointer_position: Cell::new(Point::default()),
            managed_window_drag_active: Cell::new(false),
            managed_drag_window: RefCell::new(None),
        }
    }
    /// Creates the original native MDI document window.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub(crate) fn try_get_content_offset(&self) -> Option<Point> {
        if self.content_border.borrow().is_none() {
            self.apply_template();
        }
        let border = self.content_border.borrow().clone()?;
        border.translate_point(Point::default(), self)
    }
    fn document(&self) -> Option<Rc<dyn IMdiDocument>> {
        self.current_document.borrow().clone()
    }
    fn dock(&self) -> Option<Rc<dyn IDock>> {
        self.current_dock.borrow().clone()
    }
    fn on_data_context_changed(&self, value: Option<BoxedValue>) {
        self.detach_dock_subscription();
        let document = model_projection::mdi_document(value.as_ref());
        self.current_document.replace(document.clone());
        let dock = document
            .as_ref()
            .and_then(|d| d.owner())
            .and_then(|d| d.as_dock());
        self.current_dock.replace(dock.clone());
        self.last_non_minimized_state.set(
            document
                .as_ref()
                .map(|d| d.mdi_state())
                .filter(|s| *s != MdiWindowState::Minimized)
                .unwrap_or(MdiWindowState::Normal),
        );
        if let Some(document) = document {
            if let Some(notifier) = document.as_notify_property_changed() {
                let owner = self.to_ref();
                let token = notifier
                    .property_changed()
                    .add(Rc::new(move |name| owner.document_property_changed(name)));
                self.document_subscription
                    .replace(Some(ModelSubscription(Box::new(move || {
                        if let Some(n) = document.as_notify_property_changed() {
                            n.property_changed().remove(token);
                        }
                    }))));
            }
        }
        if let Some(dock) = dock {
            if let Some(notifier) = dock.as_notify_property_changed() {
                let owner = self.to_ref();
                let token = notifier
                    .property_changed()
                    .add(Rc::new(move |name| owner.dock_property_changed(name)));
                self.dock_subscription
                    .replace(Some(ModelSubscription(Box::new(move || {
                        if let Some(n) = dock.as_notify_property_changed() {
                            n.property_changed().remove(token);
                        }
                    }))));
            }
        }
        self.update_is_active();
    }
    fn dock_property_changed(&self, name: &str) {
        if name == "ActiveDockable" {
            self.update_is_active();
        }
    }
    fn document_property_changed(&self, name: &str) {
        if name.is_empty() || matches!(name, "MdiBounds" | "MdiState" | "MdiZIndex") {
            self.invalidate_mdi_layout();
        }
    }
    fn update_is_active(&self) {
        let active = self
            .dock()
            .zip(self.document())
            .is_some_and(|(dock, document)| {
                dock.active_dockable()
                    .is_some_and(|d| std::ptr::addr_eq(Rc::as_ptr(&d), Rc::as_ptr(&document)))
            });
        self.set_current_value(Self::is_active_property(), active);
    }
    fn detach_dock_subscription(&self) {
        let sub = self.document_subscription.borrow_mut().take();
        if let Some(sub) = sub {
            sub.dispose();
        }
        let sub = self.dock_subscription.borrow_mut().take();
        if let Some(sub) = sub {
            sub.dispose();
        }
        self.current_document.replace(None);
        self.current_dock.replace(None);
    }
    fn header_pressed_handler(&self, e: &PointerPressedEventArgs) {
        if self
            .document()
            .is_none_or(|d| d.mdi_state() != MdiWindowState::Normal)
            || !e
                .get_current_point(Some(self))
                .properties
                .is_left_button_pressed
        {
            return;
        }
        let source = e.source().and_then(|s| s.cast::<Control>());
        if self.is_within_drag_handle(source.as_deref()) || self.is_within_button(source.as_deref())
        {
            return;
        }
        self.activate_document();
        self.begin_drag(e);
    }
    fn header_double_tapped_handler(&self, _e: &RoutedEventArgs) {
        if self.document().is_some() {
            self.toggle_maximize_restore();
        }
    }
    fn content_pressed_handler(&self, e: &PointerPressedEventArgs) {
        if self.document().is_some()
            && e.get_current_point(Some(self))
                .properties
                .is_left_button_pressed
        {
            self.activate_document();
        }
    }
    fn begin_drag(&self, e: &PointerPressedEventArgs) {
        if self.document().is_none() || !DockProperties::get_is_drag_enabled(self) {
            return;
        }
        let document: Rc<dyn IDockable> = self.document().expect("source current document");
        if !DockCapabilityResolver::is_enabled(
            Some(document.clone()),
            DockCapability::Drag,
            DockCapabilityResolver::resolve_operation_dock(Some(document)),
        ) {
            return;
        }
        if !self.try_begin_managed_window_drag() {
            return;
        }
        self.is_dragging.set(true);
        self.drag_start_point.set(self.get_pointer_position(e));
        self.last_pointer_position.set(self.drag_start_point.get());
        self.drag_start_bounds.set(to_ferro_rect(
            self.document()
                .expect("source current document")
                .mdi_bounds(),
        ));
        self.captured_pointer.replace(Some(e.pointer().clone()));
        e.pointer().capture(Some(&self.to_ref().upcast()));
        e.set_handled(true);
        self.process_managed_window_drag(self.last_pointer_position.get(), EventType::Pressed);
    }
    fn pointer_moved_handler(&self, e: &PointerEventArgs) {
        if self.is_dragging.get() {
            self.drag_move(e);
            return;
        }
        if self.is_resizing.get() {
            self.resize_move(e);
        }
    }
    fn pointer_released_handler(&self, e: &PointerReleasedEventArgs) {
        if self.is_dragging.get() {
            self.last_pointer_position.set(self.get_pointer_position(e));
            self.process_managed_window_drag(self.last_pointer_position.get(), EventType::Released);
        }
        self.end_drag_or_resize();
    }
    fn pointer_capture_lost_handler(&self, _e: &PointerCaptureLostEventArgs) {
        if self.is_dragging.get() {
            self.process_managed_window_drag(
                self.last_pointer_position.get(),
                EventType::CaptureLost,
            );
        }
        self.end_drag_or_resize();
    }
    fn drag_move(&self, e: &PointerEventArgs) {
        if self
            .document()
            .is_none_or(|d| d.mdi_state() != MdiWindowState::Normal)
        {
            return;
        }
        let position = self.get_pointer_position(e);
        self.last_pointer_position.set(position);
        let delta = position - self.drag_start_point.get();
        let (manager, entries, final_size) = self.get_layout_manager();
        let mut bounds = manager.get_drag_bounds(
            self.document().expect("source current document"),
            self.drag_start_bounds.get(),
            delta.into(),
            final_size,
            &entries,
        );
        if DockSettings::enable_window_magnetism() && !entries.is_empty() {
            bounds = apply_window_magnetism(
                bounds,
                &self.document().expect("source current document"),
                &entries,
            );
            bounds = clamp_to_available_size(bounds, &entries, final_size);
        }
        self.document()
            .expect("source current document")
            .set_mdi_bounds(to_dock_rect(bounds));
        self.invalidate_mdi_layout();
        e.set_handled(true);
        self.process_managed_window_drag(self.last_pointer_position.get(), EventType::Moved);
        self.notify_managed_window_drag();
    }
    fn resize_move(&self, e: &PointerEventArgs) {
        if self
            .document()
            .is_none_or(|d| d.mdi_state() != MdiWindowState::Normal)
        {
            return;
        }
        let delta = self.get_pointer_position(e) - self.drag_start_point.get();
        let (manager, entries, size) = self.get_layout_manager();
        let bounds = manager.get_resize_bounds(
            self.document().expect("source current document"),
            self.drag_start_bounds.get(),
            delta.into(),
            self.resize_direction.get(),
            size,
            &entries,
        );
        self.document()
            .expect("source current document")
            .set_mdi_bounds(to_dock_rect(bounds));
        self.invalidate_mdi_layout();
        e.set_handled(true);
    }
    fn end_drag_or_resize(&self) {
        if self.managed_window_drag_active.get() {
            self.notify_managed_window_drag_end();
        }
        self.is_dragging.set(false);
        self.is_resizing.set(false);
        self.resize_direction.set(MdiResizeDirection::NONE);
        let pointer = self.captured_pointer.borrow().clone();
        if let Some(pointer) = pointer {
            pointer.capture(None);
            self.captured_pointer.replace(None);
        }
    }
    fn managed_document(&self) -> Option<Rc<ManagedDockWindowDocument>> {
        self.document()?
            .boxed_reference()
            .downcast_ref::<Rc<ManagedDockWindowDocument>>()
            .cloned()
    }
    fn process_managed_window_drag(&self, position: Point, phase: EventType) {
        let Some(host) = self
            .managed_document()
            .and_then(|d| d.window())
            .and_then(|w| w.host())
            .and_then(|h| managed_host(&h))
        else {
            return;
        };
        let relative: Ref<Visual> = self
            .find_ancestor_of_type::<MdiLayoutPanel>(false)
            .map(|p| p.upcast())
            .unwrap_or_else(|| self.to_ref().upcast());
        host.process_drag(relative.point_to_screen(position), phase);
    }
    fn try_begin_managed_window_drag(&self) -> bool {
        let Some((window, factory)) = self
            .managed_document()
            .and_then(|d| d.window())
            .and_then(|w| w.factory().map(|f| (w, f)))
        else {
            self.managed_window_drag_active.set(false);
            self.managed_drag_window.replace(None);
            return true;
        };
        if !factory.on_window_move_drag_begin(Some(window.clone())) {
            self.managed_window_drag_active.set(false);
            self.managed_drag_window.replace(None);
            return false;
        }
        if DockSettings::bring_windows_to_front_on_drag() {
            WindowActivationHelper::activate_all_windows(&*factory, Some(self));
        }
        self.managed_window_drag_active.set(true);
        self.managed_drag_window.replace(Some(window));
        true
    }
    fn notify_managed_window_drag(&self) {
        if self.managed_window_drag_active.get() {
            let window = self.managed_drag_window.borrow().clone();
            if let Some(factory) = window.as_ref().and_then(|w| w.factory()) {
                factory.on_window_move_drag(window);
            }
        }
    }
    fn notify_managed_window_drag_end(&self) {
        if self.managed_window_drag_active.get() {
            let window = self.managed_drag_window.borrow().clone();
            if let Some(factory) = window.as_ref().and_then(|w| w.factory()) {
                factory.on_window_move_drag_end(window);
            }
        }
        self.managed_window_drag_active.set(false);
        self.managed_drag_window.replace(None);
    }
    fn attach_resize_handle(&self, handle: Option<Ref<Control>>, direction: MdiResizeDirection) {
        if let Some(handle) = handle {
            let owner = self.to_ref();
            handle.add_handler_with(
                InputElement::pointer_pressed_event(),
                move |_, e| owner.begin_resize(e, direction),
                RoutingStrategies::TUNNEL,
                false,
            );
        }
    }
    fn begin_resize(&self, e: &PointerPressedEventArgs, direction: MdiResizeDirection) {
        if self
            .document()
            .is_none_or(|d| d.mdi_state() != MdiWindowState::Normal)
            || !e
                .get_current_point(Some(self))
                .properties
                .is_left_button_pressed
        {
            return;
        }
        self.activate_document();
        self.is_resizing.set(true);
        self.resize_direction.set(direction);
        self.drag_start_point.set(self.get_pointer_position(e));
        self.drag_start_bounds.set(to_ferro_rect(
            self.document()
                .expect("source current document")
                .mdi_bounds(),
        ));
        self.captured_pointer.replace(Some(e.pointer().clone()));
        e.pointer().capture(Some(&self.to_ref().upcast()));
        e.set_handled(true);
    }
    fn minimize_button_clicked(&self) {
        let Some(document) = self.document() else {
            return;
        };
        if document.mdi_state() == MdiWindowState::Minimized {
            document.set_mdi_state(self.last_non_minimized_state.get());
        } else {
            self.last_non_minimized_state.set(document.mdi_state());
            document.set_mdi_state(MdiWindowState::Minimized);
        }
        self.invalidate_mdi_layout();
    }
    fn maximize_restore_button_clicked(&self) {
        if self.document().is_some() {
            self.toggle_maximize_restore();
        }
    }
    fn toggle_maximize_restore(&self) {
        let Some(document) = self.document() else {
            return;
        };
        document.set_mdi_state(match document.mdi_state() {
            MdiWindowState::Maximized | MdiWindowState::Minimized => MdiWindowState::Normal,
            _ => MdiWindowState::Maximized,
        });
        self.invalidate_mdi_layout();
    }
    fn activate_document(&self) {
        let Some(document) = self.document() else {
            return;
        };
        let Some(owner) = document
            .owner()
            .and_then(|o| o.as_dock())
            .filter(|o| o.factory().is_some())
        else {
            return;
        };
        owner
            .factory()
            .expect("source owner factory")
            .set_active_dockable(Some(document.clone()));
        if let Some(root) = owner
            .factory()
            .expect("source owner factory")
            .find_root(Some(document.clone()), Some(Rc::new(|_| true)))
        {
            owner
                .factory()
                .expect("source owner factory")
                .set_focused_dockable(Some(root), Some(document));
        }
        let (manager, entries, _) = self.get_layout_manager();
        let documents = if entries.is_empty() {
            vec![self.document().expect("source current document")]
        } else {
            entries.iter().map(|e| e.document()).collect()
        };
        manager.update_z_order(&ListHandle::from_plain(documents), self.document());
    }
    fn get_layout_manager(&self) -> (Rc<dyn IMdiLayoutManager>, ListHandle<MdiLayoutEntry>, Size) {
        if let Some(panel) = self.find_ancestor_of_type::<MdiLayoutPanel>(false) {
            let size = panel.bounds().size();
            let entries = build_layout_entries(&panel);
            return (
                panel
                    .layout_manager()
                    .unwrap_or_else(|| ClassicMdiLayoutManager::instance()),
                entries,
                size,
            );
        }
        let entries = self
            .document()
            .map(|d| vec![MdiLayoutEntry::new(self.to_ref().upcast(), d)])
            .unwrap_or_default();
        (
            ClassicMdiLayoutManager::instance(),
            ListHandle::from_plain(entries),
            self.bounds().size(),
        )
    }
    fn get_pointer_position(&self, e: &PointerEventArgs) -> Point {
        if let Some(panel) = self.find_ancestor_of_type::<MdiLayoutPanel>(false) {
            e.get_position(Some(&panel))
        } else {
            e.get_position(Some(self))
        }
    }
    fn invalidate_mdi_layout(&self) {
        if let Some(panel) = self.find_ancestor_of_type::<MdiLayoutPanel>(false) {
            panel.invalidate_arrange();
        }
    }
    fn is_within_button(&self, source: Option<&Control>) -> bool {
        let header = self.header.borrow().clone();
        match (header, source) {
            (Some(header), Some(source)) => {
                source.to_ref().is::<Button>()
                    || WindowDragHelper::is_child_of_type::<Button>(&header, source)
            }
            _ => false,
        }
    }
    fn is_within_drag_handle(&self, source: Option<&Control>) -> bool {
        let root = self.drag_handle.borrow().clone();
        root.zip(source)
            .is_some_and(|(root, source)| is_descendant_of(&root, Some(source)))
    }
    fn update_pseudo_classes(&self, active: bool, state: MdiWindowState) {
        self.pseudo_classes().set(":active", active);
        self.pseudo_classes()
            .set(":maximized", state == MdiWindowState::Maximized);
        self.pseudo_classes()
            .set(":minimized", state == MdiWindowState::Minimized);
    }
}
fn build_layout_entries(panel: &MdiLayoutPanel) -> ListHandle<MdiLayoutEntry> {
    let children = panel.children();
    let mut entries = Vec::with_capacity(children.count());
    let mut index = 0;
    while index < children.count() {
        let child = children.get(index);
        if let Some(document) = model_projection::mdi_document(child.data_context().as_ref()) {
            entries.push(MdiLayoutEntry::new(child, document));
        }
        index += 1;
    }
    ListHandle::from_plain(entries)
}
fn to_ferro_rect(b: DockRect) -> Rect {
    Rect::new(b.x, b.y, b.width, b.height)
}
fn to_dock_rect(b: Rect) -> DockRect {
    DockRect::new(b.x, b.y, b.width, b.height)
}
fn apply_window_magnetism(
    bounds: Rect,
    current: &Rc<dyn IMdiDocument>,
    entries: &ListHandle<MdiLayoutEntry>,
) -> Rect {
    let snap = DockSettings::window_magnet_distance();
    if snap <= 0.0 {
        return bounds;
    }
    let (mut x, mut y) = (bounds.x, bounds.y);
    for entry in entries.iter() {
        if Rc::ptr_eq(&entry.document(), current)
            || entry.document().mdi_state() != MdiWindowState::Normal
        {
            continue;
        }
        let other = to_ferro_rect(entry.document().mdi_bounds());
        if !is_valid_bounds(other) {
            continue;
        }
        let vertical = bounds.top() < other.bottom() && bounds.bottom() > other.top();
        let horizontal = bounds.left() < other.right() && bounds.right() > other.left();
        if vertical {
            if (bounds.left() - other.right()).abs() <= snap {
                x = other.right();
            } else if (bounds.right() - other.left()).abs() <= snap {
                x = other.left() - bounds.width;
            }
        }
        if horizontal {
            if (bounds.top() - other.bottom()).abs() <= snap {
                y = other.bottom();
            } else if (bounds.bottom() - other.top()).abs() <= snap {
                y = other.top() - bounds.height;
            }
        }
    }
    if x == bounds.x && y == bounds.y {
        bounds
    } else {
        Rect::new(x, y, bounds.width, bounds.height)
    }
}
fn clamp_to_available_size(bounds: Rect, entries: &ListHandle<MdiLayoutEntry>, size: Size) -> Rect {
    let available = get_available_size(entries, size);
    let max_x = max(0.0, available.width - bounds.width);
    let max_y = max(0.0, available.height - bounds.height);
    let (mut x, mut y) = (bounds.x, bounds.y);
    if x < 0.0 {
        x = 0.0;
    } else if x > max_x {
        x = max_x;
    }
    if y < 0.0 {
        y = 0.0;
    } else if y > max_y {
        y = max_y;
    }
    Rect::new(x, y, bounds.width, bounds.height)
}
fn get_available_size(entries: &ListHandle<MdiLayoutEntry>, size: Size) -> Size {
    let count = entries
        .iter()
        .filter(|e| e.document().mdi_state() == MdiWindowState::Minimized)
        .count();
    let reserved = if count > 0 {
        MdiLayoutDefaults::MINIMIZED_HEIGHT + MdiLayoutDefaults::MINIMIZED_SPACING
    } else {
        0.0
    };
    Size::new(size.width, max(0.0, size.height - reserved))
}
fn is_valid_bounds(b: Rect) -> bool {
    !b.width.is_nan() && !b.height.is_nan() && b.width > 0.0 && b.height > 0.0
}
fn is_descendant_of(root: &Control, control: Option<&Control>) -> bool {
    let mut current = control.map(Control::to_ref);
    while let Some(control) = current {
        if control.ptr_eq(&root.to_ref()) {
            return true;
        }
        current = control.parent().and_then(|p| p.cast::<Control>());
    }
    false
}
fn max(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        a
    } else if b.is_nan() {
        b
    } else if a > b || (a == b && a.is_sign_positive()) {
        a
    } else {
        b
    }
}

impl MdiDocumentWindow {
    /// Gets IconTemplate.
    pub fn icon_template(&self) -> Option<BoxedValue> {
        self.get_value(Self::icon_template_property())
    }
    /// Sets IconTemplate.
    pub fn set_icon_template(&self, value: Option<BoxedValue>) {
        self.set_value(Self::icon_template_property(), value);
    }
}

impl MdiDocumentWindow {
    /// Gets HeaderTemplate.
    pub fn header_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::header_template_property())
    }
    /// Sets HeaderTemplate.
    pub fn set_header_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::header_template_property(), value);
    }
}

impl MdiDocumentWindow {
    /// Gets ModifiedTemplate.
    pub fn modified_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::modified_template_property())
    }
    /// Sets ModifiedTemplate.
    pub fn set_modified_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::modified_template_property(), value);
    }
}

impl MdiDocumentWindow {
    /// Gets CloseTemplate.
    pub fn close_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::close_template_property())
    }
    /// Sets CloseTemplate.
    pub fn set_close_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::close_template_property(), value);
    }
}

impl MdiDocumentWindow {
    /// Gets CloseButtonTheme.
    pub fn close_button_theme(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::close_button_theme_property())
    }
    /// Sets CloseButtonTheme.
    pub fn set_close_button_theme(&self, value: Option<Ref<ControlTheme>>) {
        self.set_value(Self::close_button_theme_property(), value);
    }
}

impl MdiDocumentWindow {
    /// Gets DocumentContextMenu.
    pub fn document_context_menu(&self) -> Option<Ref<ContextMenu>> {
        self.get_value(Self::document_context_menu_property())
    }
    /// Sets DocumentContextMenu.
    pub fn set_document_context_menu(&self, value: Option<Ref<ContextMenu>>) {
        self.set_value(Self::document_context_menu_property(), value);
    }
}

impl MdiDocumentWindow {
    /// Gets IsActive.
    pub fn is_active(&self) -> bool {
        self.get_value(Self::is_active_property())
    }
    /// Sets IsActive.
    pub fn set_is_active(&self, value: bool) {
        self.set_value(Self::is_active_property(), value);
    }
}

impl MdiDocumentWindow {
    /// Gets MdiState.
    pub fn mdi_state(&self) -> MdiWindowState {
        self.get_value(Self::mdi_state_property())
    }
    /// Sets MdiState.
    pub fn set_mdi_state(&self, value: MdiWindowState) {
        self.set_value(Self::mdi_state_property(), value);
    }
}

#[cfg(test)]
#[path = "mdi_document_window_tests.rs"]
mod tests;
