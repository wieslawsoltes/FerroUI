// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{controls::HostWindow, internal::window_activation_helper::WindowActivationHelper};
use ferrodock_model::core::IDockWindow;
use ferrodock_settings::{DockSettings, WindowDragDockScope};
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    input::{InputElement, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs},
    interactivity::{Interactive, RoutingStrategies},
    BoxedValue, Point, Ref,
};
use ferroui_controls::{Control, TopLevel, Window};
use ferroui_system::IDisposable;
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
type CanStartDrag = dyn Fn(Option<&Control>) -> bool;
type GetDockScope = dyn Fn(Option<&Control>) -> WindowDragDockScope;
/// Original helper for tracked moves initiated by custom control chrome.
pub struct WindowDragHelper {
    runtime: Weak<Self>,
    owner: Ref<Control>,
    is_enabled: Rc<dyn Fn() -> bool>,
    can_start_drag: Rc<CanStartDrag>,
    get_dock_scope: Rc<GetDockScope>,
    handle_pointer_pressed: bool,
    handled_pointer_pressed: Cell<bool>,
    drag_start_point: Cell<Point>,
    pointer_pressed: Cell<bool>,
    is_dragging: Cell<bool>,
    dock_scope: Cell<WindowDragDockScope>,
    last_pointer_pressed_args: RefCell<Option<PointerPressedEventArgs>>,
    drag_window: RefCell<Option<Ref<Window>>>,
    // Source declares this removal slot but does not install a position handler.
    position_changed_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    disposables: RefCell<Option<Vec<Rc<dyn IDisposable>>>>,
    released_event_disposable: RefCell<Option<Rc<dyn IDisposable>>>,
}
impl WindowDragHelper {
    /// Creates the source helper with its original control/callback ownership.
    pub fn new(
        owner: Ref<Control>,
        is_enabled: Rc<dyn Fn() -> bool>,
        can_start_drag: Rc<CanStartDrag>,
        handle_pointer_pressed: bool,
        get_dock_scope: Option<Rc<GetDockScope>>,
    ) -> Rc<Self> {
        Rc::new_cyclic(|runtime| Self {
            runtime: runtime.clone(),
            owner,
            is_enabled,
            can_start_drag,
            get_dock_scope: get_dock_scope
                .unwrap_or_else(|| Rc::new(|_| WindowDragDockScope::FullWindow)),
            handle_pointer_pressed,
            handled_pointer_pressed: Cell::new(false),
            drag_start_point: Cell::new(Point::default()),
            pointer_pressed: Cell::new(false),
            is_dragging: Cell::new(false),
            dock_scope: Cell::new(WindowDragDockScope::FullWindow),
            last_pointer_pressed_args: RefCell::new(None),
            drag_window: RefCell::new(None),
            position_changed_subscription: RefCell::new(None),
            disposables: RefCell::new(None),
            released_event_disposable: RefCell::new(None),
        })
    }
    fn this(&self) -> Rc<Self> {
        self.runtime
            .upgrade()
            .expect("live original window drag helper")
    }
    /// Attaches the source tunnel press and move handlers after explicit detach.
    pub fn attach(&self) {
        self.detach();
        let this = self.this();
        let pressed = self.owner.add_disposable_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, e| this.on_pointer_pressed(e),
            RoutingStrategies::TUNNEL,
            false,
        );
        let pressed = RetainedSubscription::retain(self.owner.clone().upcast(), pressed);
        let this = self.this();
        let moved = self.owner.add_disposable_handler_with(
            InputElement::pointer_moved_event(),
            move |_, e| this.on_pointer_moved(e),
            RoutingStrategies::TUNNEL,
            false,
        );
        let moved = RetainedSubscription::retain(self.owner.clone().upcast(), moved);
        self.disposables.replace(Some(vec![pressed, moved]));
    }
    /// Explicitly removes the source subscriptions and cancels an in-progress move.
    pub fn detach(&self) {
        let disposables = self.disposables.borrow().clone();
        if let Some(disposables) = disposables {
            for disposable in disposables {
                disposable.dispose();
            }
            self.disposables.replace(None);
        }
        self.cancel_drag();
    }
    fn drag_window(&self) -> Option<Ref<Window>> {
        self.drag_window.borrow().clone()
    }
    fn dispose_release(&self) {
        // Keep the source field until Dispose returns; callbacks may reenter.
        let disposable = self.released_event_disposable.borrow().clone();
        if let Some(disposable) = disposable {
            disposable.dispose();
        }
    }
    fn dispose_position(&self) {
        let disposable = self.position_changed_subscription.borrow().clone();
        if let Some(disposable) = disposable {
            disposable.dispose();
        }
        self.position_changed_subscription.replace(None);
    }
    fn cancel_drag(&self) {
        self.dispose_release();
        self.released_event_disposable.replace(None);
        let host_window = self.drag_window().and_then(|w| w.cast::<HostWindow>());
        if self.drag_window().is_some() && self.position_changed_subscription.borrow().is_some() {
            self.dispose_position();
        }
        self.drag_window.replace(None);
        self.pointer_pressed.set(false);
        self.is_dragging.set(false);
        self.handled_pointer_pressed.set(false);
        self.dock_scope.set(WindowDragDockScope::FullWindow);
        self.last_pointer_pressed_args.replace(None);
        if let Some(host) = host_window {
            host.cancel_external_window_drag();
        }
    }
    fn on_pointer_pressed(&self, e: &PointerPressedEventArgs) {
        if !(self.is_enabled)() {
            return;
        }
        if !e
            .get_current_point(Some(&self.owner))
            .properties
            .is_left_button_pressed
        {
            return;
        }
        self.cancel_drag();
        self.last_pointer_pressed_args.replace(Some(e.clone()));
        let source = e.source().and_then(|s| s.cast::<Control>());
        if (self.can_start_drag)(source.as_deref()) {
            self.drag_start_point.set(e.get_position(Some(&self.owner)));
            self.pointer_pressed.set(true);
            self.dock_scope
                .set((self.get_dock_scope)(source.as_deref()));
            if self.handle_pointer_pressed {
                e.set_handled(true);
                self.handled_pointer_pressed.set(true);
            } else {
                self.handled_pointer_pressed.set(false);
            }
        }
    }
    fn on_pointer_released(&self, e: &PointerReleasedEventArgs) {
        self.dispose_release();
        self.released_event_disposable.replace(None);
        if !self.pointer_pressed.get() && !self.is_dragging.get() {
            return;
        }
        let should_handle_release = self.is_dragging.get() || self.handled_pointer_pressed.get();
        self.pointer_pressed.set(false);
        self.is_dragging.set(false);
        self.handled_pointer_pressed.set(false);
        if self.drag_window().is_some() {
            if self.position_changed_subscription.borrow().is_some() {
                self.dispose_position();
            }
            if let Some(host) = self.drag_window().and_then(|w| w.cast::<HostWindow>()) {
                host.end_external_window_drag(e);
            }
        }
        self.drag_window.replace(None);
        self.dock_scope.set(WindowDragDockScope::FullWindow);
        if should_handle_release {
            e.set_handled(true);
        }
        // Source does not clear the last pressed args in this release path.
    }
    fn on_pointer_moved(&self, e: &PointerEventArgs) {
        if !self.pointer_pressed.get() || self.is_dragging.get() {
            return;
        }
        if !e
            .get_current_point(Some(&self.owner))
            .properties
            .is_left_button_pressed
        {
            self.pointer_pressed.set(false);
            self.handled_pointer_pressed.set(false);
            self.last_pointer_pressed_args.replace(None);
            return;
        }
        let current = e.get_position(Some(&self.owner));
        let delta = current - self.drag_start_point.get();
        if delta
            .x
            .abs()
            .partial_cmp(&DockSettings::minimum_horizontal_drag_distance())
            != Some(std::cmp::Ordering::Greater)
            && delta
                .y
                .abs()
                .partial_cmp(&DockSettings::minimum_vertical_drag_distance())
                != Some(std::cmp::Ordering::Greater)
        {
            return;
        }
        if self.last_pointer_pressed_args.borrow().is_none() {
            return;
        }
        let root = TopLevel::get_top_level(Some(&self.owner));
        let host = root.as_ref().and_then(|r| r.cast::<HostWindow>());
        let Some(host) = host else {
            if let Some(window) = root.and_then(|r| r.cast::<Window>()) {
                if DockSettings::bring_windows_to_front_on_drag() {
                    if let Some(factory) =
                        dock_window(self.owner.data_context()).and_then(|w| w.factory())
                    {
                        WindowActivationHelper::activate_all_windows(&*factory, Some(&self.owner));
                    }
                }
                self.dispose_release();
                let subscription = self.subscribe_to_pointer_released(&window);
                self.released_event_disposable.replace(Some(subscription));
                let args = self
                    .last_pointer_pressed_args
                    .borrow()
                    .clone()
                    .expect("source pressed args re-read");
                window.begin_move_drag(&args);
                self.pointer_pressed.set(false);
                self.is_dragging.set(false);
                e.set_handled(true);
            }
            return;
        };
        self.is_dragging.set(true);
        self.pointer_pressed.set(false);
        let args = self
            .last_pointer_pressed_args
            .borrow()
            .clone()
            .expect("source pressed args re-read");
        if !host.try_begin_external_window_drag(&args, self.dock_scope.get()) {
            self.is_dragging.set(false);
            return;
        }
        self.drag_window.replace(Some(host.clone().upcast()));
        self.dispose_release();
        let subscription = self.subscribe_to_pointer_released(&host);
        self.released_event_disposable.replace(Some(subscription));
        let args = self
            .last_pointer_pressed_args
            .borrow()
            .clone()
            .expect("source pressed args re-read");
        host.begin_move_drag(&args);
        e.set_handled(true);
    }
    fn subscribe_to_pointer_released(&self, window: &Window) -> Rc<dyn IDisposable> {
        let this = self.this();
        let subscription = window.add_disposable_handler_with(
            InputElement::pointer_released_event(),
            move |_, e| this.on_pointer_released(e),
            RoutingStrategies::TUNNEL,
            true,
        );
        RetainedSubscription::retain(window.to_ref().upcast(), subscription)
    }
}
fn dock_window(value: Option<BoxedValue>) -> Option<Rc<dyn IDockWindow>> {
    let value = value?;
    if let Some(window) = value.downcast_ref::<Rc<dyn IDockWindow>>() {
        return Some(window.clone());
    }
    ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IDockWindow>>())?
        .downcast_ref::<Rc<dyn IDockWindow>>()
        .cloned()
}
// The shared native disposable intentionally holds its target weakly. Preserve the
// source strong subscription receiver until explicit Dispose, without changing it.
struct RetainedSubscription {
    receiver: RefCell<Option<Ref<Interactive>>>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}
impl RetainedSubscription {
    fn retain(
        receiver: Ref<Interactive>,
        subscription: Rc<dyn IDisposable>,
    ) -> Rc<dyn IDisposable> {
        Rc::new(Self {
            receiver: RefCell::new(Some(receiver)),
            subscription: RefCell::new(Some(subscription)),
        })
    }
}
impl IDisposable for RetainedSubscription {
    fn dispose(&self) {
        let subscription = self.subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        self.receiver.replace(None);
    }
}

impl WindowDragHelper {
    pub(crate) fn is_child_of_type<T: ferroui_base::ObjectType>(
        owner: &Control,
        control: &Control,
    ) -> bool {
        let mut parent = Some(control.to_ref());
        while let Some(current) = parent {
            if current.ptr_eq(&owner.to_ref()) {
                break;
            }
            if current.is::<T>() {
                return true;
            }
            parent = current.parent().and_then(|p| p.cast::<Control>());
        }
        false
    }
}
