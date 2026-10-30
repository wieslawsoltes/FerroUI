// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{ManagedWindowLayer, ToolPinItemControl};
use crate::{internal::dock_helpers::DockHelpers, model_projection};
use ferrodock_model::core::{IDockable, IDockableControl, TrackingMode};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{InputElement, InputElementImpl, PointerEventArgs},
    instantiate,
    interactivity::{InteractiveImpl, RoutingStrategies},
    layout::LayoutableImpl,
    reactive::{AnonymousObserver, IDisposable},
    BoxedValue, FerroObject, FerroObjectExtensions, FerroObjectImpl, FerroProperty, Rect, Ref,
    StyledElement, StyledElementImpl, StyledProperty, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{ControlImpl, Panel, PanelImpl};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
/// Original panel tracking dockable registrations, bounds and pointer coordinates.
#[repr(C)]
pub struct DockableControl {
    base: Panel,
    bounds_disposable: RefCell<Option<Rc<dyn IDisposable>>>,
    data_context_disposable: RefCell<Option<Rc<dyn IDisposable>>>,
    current_dockable: RefCell<Option<Rc<dyn IDockable>>>,
    pressed: RefCell<Option<Rc<dyn IDisposable>>>,
    moved: RefCell<Option<Rc<dyn IDisposable>>>,
    interface_handle: RefCell<Weak<DockableControlHandle>>,
}
ferro_class!(DockableControl:Panel);
ferro_class_info!(DockableControl {
    new:DockableControl::new,
    interfaces:[Rc<dyn IDockableControl> => DockableControl::as_dockable_control],
});
ferro_impl_classes!(DockableControl:FerroObjectImpl,StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
ferro_properties! {impl DockableControl {
    /// Identifies the source tracking mode.
    pub fn tracking_mode_property()->StyledProperty<TrackingMode> {
        FerroProperty::register::<Self,TrackingMode>("TrackingMode",TrackingMode::Visible)
    }
}}
impl VisualImpl for DockableControl {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let owner = this.to_ref();
        let subscription =
            FerroObject::get_observable(this, StyledElement::data_context_property()).subscribe(
                Rc::new(AnonymousObserver::new(move |value: Option<BoxedValue>| {
                    owner.data_context_tracking(value.as_ref())
                })),
            );
        this.data_context_disposable.replace(Some(subscription));
        let owner = this.to_ref();
        let pressed = this.add_disposable_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, e| owner.set_pointer_tracking(e),
            RoutingStrategies::TUNNEL,
            false,
        );
        this.pressed.replace(Some(pressed));
        let owner = this.to_ref();
        let moved = this.add_disposable_handler_with(
            InputElement::pointer_moved_event(),
            move |_, e| owner.set_pointer_tracking(e),
            RoutingStrategies::TUNNEL,
            false,
        );
        this.moved.replace(Some(moved));
        let owner = this.to_ref();
        let subscription =
            FerroObject::get_observable(this, Visual::bounds_property()).subscribe(Rc::new(
                AnonymousObserver::new(move |bounds: Rect| owner.set_bounds_tracking(bounds)),
            ));
        this.bounds_disposable.replace(Some(subscription));
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        let current = this.current_dockable.borrow().clone();
        if let Some(current) = current {
            this.un_register(&current);
            this.current_dockable.replace(None);
        }
        let pressed = this.pressed.borrow().clone();
        if let Some(pressed) = pressed {
            pressed.dispose();
        }
        let moved = this.moved.borrow().clone();
        if let Some(moved) = moved {
            moved.dispose();
        }
        let bounds = this.bounds_disposable.borrow().clone();
        if let Some(bounds) = bounds {
            bounds.dispose();
        }
        let context = this.data_context_disposable.borrow().clone();
        if let Some(context) = context {
            context.dispose();
        }
    }
}
impl DockableControl {
    /// Constructs the original panel state for native inheritance.
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            bounds_disposable: RefCell::new(None),
            data_context_disposable: RefCell::new(None),
            current_dockable: RefCell::new(None),
            pressed: RefCell::new(None),
            moved: RefCell::new(None),
            interface_handle: RefCell::new(Weak::new()),
        }
    }
    /// Creates an original dockable tracking panel.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets TrackingMode.
    pub fn tracking_mode(&self) -> TrackingMode {
        self.get_value(Self::tracking_mode_property())
    }
    /// Sets TrackingMode without introducing re-registration absent from the source.
    pub fn set_tracking_mode(&self, value: TrackingMode) {
        self.set_value(Self::tracking_mode_property(), value);
    }
    /// Gets the original factory tracking contract for the native receiver.
    pub fn as_dockable_control(owner: Ref<Self>) -> Rc<dyn IDockableControl> {
        if let Some(handle) = owner.interface_handle.borrow().upgrade() {
            return handle;
        }
        let handle = Rc::new(DockableControlHandle(owner.clone()));
        owner.interface_handle.replace(Rc::downgrade(&handle));
        handle
    }
    fn data_context_tracking(&self, value: Option<&BoxedValue>) {
        let old = self.current_dockable.borrow().clone();
        if let Some(old) = old {
            self.un_register(&old);
            self.current_dockable.replace(None);
        }
        if let Some(dockable) = model_projection::dockable(value) {
            self.current_dockable.replace(Some(dockable.clone()));
            self.register(&dockable);
        }
    }
    fn register(&self, dockable: &Rc<dyn IDockable>) {
        let root = self
            .templated_parent()
            .unwrap_or_else(|| self.to_ref().upcast());
        match self.tracking_mode() {
            TrackingMode::Visible => {
                if dockable.factory().is_some() {
                    dockable
                        .factory()
                        .expect("source current factory")
                        .visible_dockable_controls()
                        .borrow_mut()
                        .insert(dockable.clone(), Self::as_dockable_control(self.to_ref()));
                    dockable
                        .factory()
                        .expect("source current factory")
                        .visible_root_controls()
                        .borrow_mut()
                        .insert(dockable.clone(), Rc::new(root));
                }
            }
            TrackingMode::Pinned => {
                if dockable.factory().is_some() {
                    dockable
                        .factory()
                        .expect("source current factory")
                        .pinned_dockable_controls()
                        .borrow_mut()
                        .insert(dockable.clone(), Self::as_dockable_control(self.to_ref()));
                    dockable
                        .factory()
                        .expect("source current factory")
                        .pinned_root_controls()
                        .borrow_mut()
                        .insert(dockable.clone(), Rc::new(root));
                }
            }
            TrackingMode::Tab => {
                if dockable.factory().is_some() {
                    dockable
                        .factory()
                        .expect("source current factory")
                        .tab_dockable_controls()
                        .borrow_mut()
                        .insert(dockable.clone(), Self::as_dockable_control(self.to_ref()));
                    dockable
                        .factory()
                        .expect("source current factory")
                        .tab_root_controls()
                        .borrow_mut()
                        .insert(dockable.clone(), Rc::new(root));
                }
            }
        }
    }
    fn un_register(&self, dockable: &Rc<dyn IDockable>) {
        match self.tracking_mode() {
            TrackingMode::Visible => {
                if dockable.factory().is_some() {
                    dockable
                        .factory()
                        .expect("source current factory")
                        .visible_dockable_controls()
                        .borrow_mut()
                        .remove(dockable);
                    dockable
                        .factory()
                        .expect("source current factory")
                        .visible_root_controls()
                        .borrow_mut()
                        .remove(dockable);
                }
            }
            TrackingMode::Pinned => {
                if dockable.factory().is_some() {
                    dockable
                        .factory()
                        .expect("source current factory")
                        .pinned_dockable_controls()
                        .borrow_mut()
                        .remove(dockable);
                    dockable
                        .factory()
                        .expect("source current factory")
                        .pinned_root_controls()
                        .borrow_mut()
                        .remove(dockable);
                }
            }
            TrackingMode::Tab => {
                if dockable.factory().is_some() {
                    dockable
                        .factory()
                        .expect("source current factory")
                        .tab_dockable_controls()
                        .borrow_mut()
                        .remove(dockable);
                    dockable
                        .factory()
                        .expect("source current factory")
                        .tab_root_controls()
                        .borrow_mut()
                        .remove(dockable);
                }
            }
        }
    }
    fn set_bounds_tracking(&self, bounds: Rect) {
        let Some(dockable) = model_projection::dockable(self.data_context().as_ref()) else {
            return;
        };
        if self.tracking_mode() == TrackingMode::Pinned
            && self
                .find_ancestor_of_type::<ToolPinItemControl>(false)
                .is_some()
        {
            return;
        }
        let (mut x, mut y) = (bounds.x, bounds.y);
        if let Some(root) = self.visual_root().and_then(|r| r.cast::<Visual>()) {
            if let Some(position) = self.translate_point(bounds.position(), &root) {
                x = position.x;
                y = position.y;
            }
        }
        match self.tracking_mode() {
            TrackingMode::Visible => dockable.set_visible_bounds(x, y, bounds.width, bounds.height),
            TrackingMode::Pinned => dockable.set_pinned_bounds(x, y, bounds.width, bounds.height),
            TrackingMode::Tab => dockable.set_tab_bounds(x, y, bounds.width, bounds.height),
        }
    }
    fn set_pointer_tracking(&self, e: &PointerEventArgs) {
        let Some(dockable) = model_projection::dockable(self.data_context().as_ref()) else {
            return;
        };
        let position = e.get_position(Some(self));
        if self.visual_root().is_none() {
            return;
        }
        let screen = DockHelpers::get_screen_point(self, position);
        let (mut x, mut y) = (screen.x, screen.y);
        if DockHelpers::is_managed_window_hosting_enabled(Some(self)) {
            if let Some(layer) = ManagedWindowLayer::try_get_layer(Some(self)) {
                if let Some(point) = self.translate_point(position, &layer) {
                    x = point.x;
                    y = point.y;
                }
            }
        }
        dockable.set_pointer_position(position.x, position.y);
        dockable.set_pointer_screen_position(x, y);
    }
}
// Native foreign interface views retain the original panel receiver.
struct DockableControlHandle(Ref<DockableControl>);
impl IDockableControl for DockableControlHandle {
    fn boxed_reference(self: Rc<Self>) -> ferrodock_model::BoxedValue {
        Rc::new(self.0.clone())
    }
}
