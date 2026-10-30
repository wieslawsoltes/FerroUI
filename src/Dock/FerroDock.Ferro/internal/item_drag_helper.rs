// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::model_projection;
use ferrodock_model::collections::ListHandle;
use ferrodock_settings::DockSettings;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::{
    InputElement, MouseButton, PointerCaptureLostEventArgs, PointerEventArgs,
    PointerPressedEventArgs, PointerReleasedEventArgs,
};
use ferroui_base::interactivity::{RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::layout::Orientation;
use ferroui_base::media::transformation::TransformOperationsBuilder;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::{BoxedValue, Point, Rect, Ref, Vector, Visual, WeakRef};
use ferroui_controls::primitives::SelectingItemsControl;
use ferroui_controls::{items_equal, Control, ItemsControl, ScrollViewer};
use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;
pub(crate) type DragOutside = Rc<dyn Fn(Option<&PointerEventArgs>, &PointerEventArgs)>;
struct Subscriptions {
    released: RoutedEventHandlerToken,
    pressed: RoutedEventHandlerToken,
    moved: RoutedEventHandlerToken,
    lost: RoutedEventHandlerToken,
}
struct AutoScroll {
    timer: Rc<DispatcherTimer>,
    tick: Rc<dyn IDisposable>,
}
pub(crate) struct ItemDragHelper {
    owner: WeakRef<Control>,
    get_items: Rc<dyn Fn() -> Option<Ref<ItemsControl>>>,
    get_bounds: Rc<dyn Fn() -> Option<Ref<Control>>>,
    get_orientation: Rc<dyn Fn() -> Orientation>,
    get_horizontal: Rc<dyn Fn() -> f64>,
    get_vertical: Rc<dyn Fn() -> f64>,
    drag_outside: Option<DragOutside>,
    enable_drag: Cell<bool>,
    drag_started: Cell<bool>,
    start: Cell<Point>,
    dragged_index: Cell<i32>,
    target_index: Cell<i32>,
    items: RefCell<Option<Ref<ItemsControl>>>,
    dragged: RefCell<Option<Ref<Control>>>,
    captured: Cell<bool>,
    auto_scroll: RefCell<Option<AutoScroll>>,
    scroll: RefCell<Option<Ref<ScrollViewer>>>,
    velocity: Cell<Vector>,
    pressed_args: RefCell<Option<PointerPressedEventArgs>>,
    subscriptions: RefCell<Vec<Subscriptions>>,
}
impl ItemDragHelper {
    pub(crate) fn new(
        owner: &Control,
        get_items: Rc<dyn Fn() -> Option<Ref<ItemsControl>>>,
        get_orientation: Rc<dyn Fn() -> Orientation>,
        get_horizontal: Option<Rc<dyn Fn() -> f64>>,
        get_vertical: Option<Rc<dyn Fn() -> f64>>,
        drag_outside: Option<DragOutside>,
        get_bounds: Option<Rc<dyn Fn() -> Option<Ref<Control>>>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            owner: owner.to_ref().downgrade(),
            get_items,
            get_bounds: get_bounds.unwrap_or_else(|| Rc::new(|| None)),
            get_orientation,
            get_horizontal: get_horizontal
                .unwrap_or_else(|| Rc::new(DockSettings::minimum_horizontal_drag_distance)),
            get_vertical: get_vertical
                .unwrap_or_else(|| Rc::new(DockSettings::minimum_vertical_drag_distance)),
            drag_outside,
            enable_drag: Cell::new(false),
            drag_started: Cell::new(false),
            start: Cell::new(Point::default()),
            dragged_index: Cell::new(0),
            target_index: Cell::new(0),
            items: RefCell::new(None),
            dragged: RefCell::new(None),
            captured: Cell::new(false),
            auto_scroll: RefCell::new(None),
            scroll: RefCell::new(None),
            velocity: Cell::new(Vector::default()),
            pressed_args: RefCell::new(None),
            subscriptions: RefCell::new(Vec::new()),
        })
    }
    fn owner(&self) -> Ref<Control> {
        self.owner.upgrade().expect("attached drag helper owner")
    }
    pub(crate) fn attach(self: &Rc<Self>) {
        let owner = self.owner();
        let this = self.clone();
        let released = owner.add_handler_with(
            InputElement::pointer_released_event(),
            move |_, e| this.pointer_released(e),
            RoutingStrategies::TUNNEL,
            false,
        );
        let this = self.clone();
        let pressed = owner.add_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, e| this.pointer_pressed(e),
            RoutingStrategies::TUNNEL,
            false,
        );
        let this = self.clone();
        let moved = owner.add_handler_with(
            InputElement::pointer_moved_event(),
            move |_, e| this.pointer_moved(e),
            RoutingStrategies::TUNNEL,
            false,
        );
        let this = self.clone();
        let lost = owner.add_handler_with(
            InputElement::pointer_capture_lost_event(),
            move |_, e| this.pointer_capture_lost(e),
            RoutingStrategies::TUNNEL,
            false,
        );
        self.subscriptions.borrow_mut().push(Subscriptions {
            released,
            pressed,
            moved,
            lost,
        });
    }
    pub(crate) fn detach(&self) {
        if let Some(sub) = self.subscriptions.borrow_mut().pop() {
            let owner = self.owner();
            owner.remove_handler(InputElement::pointer_released_event(), sub.released);
            owner.remove_handler(InputElement::pointer_pressed_event(), sub.pressed);
            owner.remove_handler(InputElement::pointer_moved_event(), sub.moved);
            owner.remove_handler(InputElement::pointer_capture_lost_event(), sub.lost);
        }
        self.stop_auto_scroll();
    }
    fn can_drag_item(&self) -> bool {
        let Some(dockable) = model_projection::dockable(self.owner().data_context().as_ref())
        else {
            return true;
        };
        dockable
            .factory()
            .and_then(|factory| factory.as_dock_item_drag_policy())
            .is_none_or(|policy| policy.can_drag_item(dockable))
    }
    fn can_reorder_item(&self) -> bool {
        let Some(dockable) = model_projection::dockable(self.owner().data_context().as_ref())
        else {
            return true;
        };
        dockable
            .factory()
            .and_then(|factory| factory.as_dock_item_drag_policy())
            .is_none_or(|policy| policy.can_reorder_item(dockable))
    }
    fn pointer_pressed(&self, e: &PointerPressedEventArgs) {
        let owner = self.owner();
        let properties = e.get_current_point(Some(&owner)).properties;
        if properties.is_left_button_pressed && self.can_drag_item() {
            if let Some(items) = (self.get_items)() {
                self.enable_drag.set(true);
                self.drag_started.set(false);
                self.start.set(e.get_position(Some(&items)));
                self.dragged_index.set(-1);
                self.target_index.set(-1);
                self.items.replace(Some(items.clone()));
                self.dragged.replace(Some(owner.clone()));
                dragging(&owner, true);
                owner.set_current_value(
                    Visual::z_index_property(),
                    get_dragged_z_index(&items, &owner),
                );
                let current_items = self.items.borrow().clone();
                self.reset_transforms(current_items);
                self.captured.set(true);
                self.pressed_args.replace(Some(e.clone()));
            }
        }
    }
    fn pointer_released(self: &Rc<Self>, e: &PointerReleasedEventArgs) {
        if self.captured.get() {
            if e.initial_press_mouse_button() == MouseButton::Left {
                self.released();
            }
            self.captured.set(false);
            self.pressed_args.replace(None);
        }
    }
    fn pointer_capture_lost(self: &Rc<Self>, _e: &PointerCaptureLostEventArgs) {
        self.released();
        self.captured.set(false);
        self.pressed_args.replace(None);
    }
    fn released(self: &Rc<Self>) {
        if !self.enable_drag.get() {
            return;
        }
        self.stop_auto_scroll();
        let dragged = self.dragged.borrow().clone();
        if let Some(dragged) = dragged {
            dragged.clear_value(Visual::z_index_property());
        }
        let current_items = self.items.borrow().clone();
        self.reset_transforms(current_items);
        let items = self.items.borrow().clone();
        if let Some(items) = items {
            for control in items.get_realized_containers() {
                dragging(&control, true);
            }
        }
        if self.drag_started.get()
            && self.dragged_index.get() >= 0
            && self.target_index.get() >= 0
            && self.dragged_index.get() != self.target_index.get()
        {
            let current_items = self.items.borrow().clone();
            self.move_dragged_item(
                current_items,
                self.dragged_index.get(),
                self.target_index.get(),
            );
        }
        let items = self.items.borrow().clone();
        if let Some(items) = items {
            for control in items.get_realized_containers() {
                dragging(&control, false);
            }
        }
        let dragged = self.dragged.borrow().clone();
        if let Some(dragged) = dragged {
            dragging(&dragged, false);
        }
        self.dragged_index.set(-1);
        self.target_index.set(-1);
        self.enable_drag.set(false);
        self.drag_started.set(false);
        self.items.replace(None);
        self.dragged.replace(None);
    }
    fn reset_transforms(&self, items: Option<Ref<ItemsControl>>) {
        if let Some(items) = items {
            for (index, _) in items.items().iter().enumerate() {
                if let Some(container) = items.container_from_index(index as i32) {
                    translate(&container, 0.0, 0.0);
                }
            }
        }
    }
    fn move_dragged_item(
        &self,
        items: Option<Ref<ItemsControl>>,
        dragged_index: i32,
        target_index: i32,
    ) {
        let Some(items) = items else {
            return;
        };
        if dragged_index == target_index || !self.can_drag_item() || !self.can_reorder_item() {
            return;
        }
        let Some(target) = items
            .container_from_index(target_index)
            .filter(|target| target.is_visible() && target.is_hit_test_visible())
        else {
            return;
        };
        let pair = items
            .container_from_index(dragged_index)
            .and_then(|dragged| model_projection::dockable(dragged.data_context().as_ref()))
            .zip(model_projection::dockable(target.data_context().as_ref()));
        if let Some((dragged, target)) = pair {
            if let Some((dock, factory)) = dragged
                .owner()
                .and_then(|owner| owner.as_dock())
                .and_then(|dock| dock.factory().map(|factory| (dock, factory)))
            {
                if factory.is_dockable_pinned(Some(dragged.clone()), None)
                    && factory.is_dockable_pinned(Some(target.clone()), None)
                {
                    factory.move_pinned_dockable(Some(dragged), Some(target));
                } else {
                    factory.move_dockable(Some(dock), Some(dragged), Some(target));
                }
                set_selected_index(&items, target_index);
                return;
            }
        }
        if let Some(source) = items.items_source() {
            if let Some(list) = ValueTypes::try_convert_for_property(
                source.list().as_any(),
                TypeId::of::<ListHandle<Option<BoxedValue>>>(),
            )
            .and_then(|value| {
                value
                    .downcast_ref::<ListHandle<Option<BoxedValue>>>()
                    .cloned()
            }) {
                let item = list.get(dragged_index as usize);
                list.remove_at(dragged_index as usize);
                list.insert(target_index as usize, item);
                set_selected_index(&items, target_index);
                return;
            }
        }
        let collection = items.items();
        if !collection.is_read_only() {
            let item = collection.get_at(dragged_index as usize);
            collection.remove_at(dragged_index as usize);
            collection.insert(target_index as usize, item);
            set_selected_index(&items, target_index);
        }
    }
    fn start_auto_scroll(self: &Rc<Self>, scroll: Ref<ScrollViewer>, velocity: Vector) {
        if self
            .auto_scroll
            .borrow()
            .as_ref()
            .is_some_and(|s| s.timer.is_enabled())
            && self.scroll.borrow().as_ref() == Some(&scroll)
            && same_vector(self.velocity.get(), velocity)
        {
            return;
        }
        self.stop_auto_scroll();
        self.scroll.replace(Some(scroll));
        self.velocity.set(velocity);
        let timer = DispatcherTimer::new();
        timer.set_interval(Duration::from_millis(16));
        let weak = Rc::downgrade(self);
        let tick = timer.tick(move |_| {
            if let Some(this) = weak.upgrade() {
                this.on_auto_scroll_tick();
            }
        });
        self.auto_scroll.replace(Some(AutoScroll {
            timer: timer.clone(),
            tick,
        }));
        timer.start();
    }
    fn stop_auto_scroll(&self) {
        let state = self.auto_scroll.borrow_mut().take();
        if let Some(state) = state {
            state.tick.dispose();
            state.timer.stop();
        }
        self.scroll.replace(None);
        self.velocity.set(Vector::default());
    }
    fn on_auto_scroll_tick(&self) {
        let scroll = self.scroll.borrow().clone();
        let Some(scroll) = scroll else {
            self.stop_auto_scroll();
            return;
        };
        let x = max(
            0.0,
            min(
                scroll.extent().width - scroll.viewport().width,
                scroll.offset().x + self.velocity.get().x * 0.016,
            ),
        );
        let y = max(
            0.0,
            min(
                scroll.extent().height - scroll.viewport().height,
                scroll.offset().y + self.velocity.get().y * 0.016,
            ),
        );
        scroll.set_offset(Vector::new(x, y));
    }
    fn calculate_auto_scroll_velocity(&self, position: Point, scroll: &ScrollViewer) -> Vector {
        let viewport = scroll.viewport();
        let x = if position.x < 20.0 {
            -200.0 * ((20.0 - position.x) / 20.0)
        } else if position.x > viewport.width - 20.0 {
            200.0 * ((position.x - (viewport.width - 20.0)) / 20.0)
        } else {
            0.0
        };
        let y = if position.y < 20.0 {
            -200.0 * ((20.0 - position.y) / 20.0)
        } else if position.y > viewport.height - 20.0 {
            200.0 * ((position.y - (viewport.height - 20.0)) / 20.0)
        } else {
            0.0
        };
        Vector::new(x, y)
    }
    fn position_within_drag_bounds(self: &Rc<Self>, position: Point, items: &ItemsControl) -> bool {
        let bounds = (self.get_bounds)().unwrap_or_else(|| items.to_ref().upcast());
        let mut container_position = position;
        if !bounds.ptr_eq(&items.to_ref()) {
            if let Some(transform) = items.transform_to_visual(&bounds) {
                container_position = transform.transform(position);
            }
        }
        let bounds_rect = Rect::from_size(bounds.bounds().size());
        if let Some(scroll) = items.find_descendant_of_type::<ScrollViewer>(false) {
            let mut scroll_position = position;
            if !scroll.ptr_eq(&items.to_ref()) {
                if let Some(transform) = items.transform_to_visual(&scroll) {
                    scroll_position = transform.transform(position);
                }
            }
            let velocity = self.calculate_auto_scroll_velocity(scroll_position, &scroll);
            if velocity.length() > 0.0 {
                self.start_auto_scroll(scroll.clone(), velocity);
            } else {
                self.stop_auto_scroll();
            }
            let scroll_bounds = Rect::new(
                -scroll.offset().x,
                -scroll.offset().y,
                max(scroll.viewport().width, scroll.extent().width),
                max(scroll.viewport().height, scroll.extent().height),
            );
            if !scroll_bounds.contains(scroll_position) {
                return false;
            }
            return bounds_rect.contains(container_position);
        }
        self.stop_auto_scroll();
        bounds_rect.contains(container_position)
    }
    fn pointer_moved(self: &Rc<Self>, e: &PointerEventArgs) {
        let properties = e.get_current_point(Some(&self.owner())).properties;
        if self.captured.get() && !self.can_drag_item() {
            self.drag_started.set(false);
            self.released();
            self.captured.set(false);
            self.pressed_args.replace(None);
            return;
        }
        if !(self.captured.get()
            && properties.is_left_button_pressed
            && self.pressed_args.borrow().is_some())
        {
            return;
        }
        let dragged = self.dragged.borrow().clone();
        if self.items.borrow().is_none()
            || dragged.is_none_or(|dragged| dragged.render_transform().is_none())
            || !self.enable_drag.get()
        {
            return;
        }
        let items = self.items.borrow().clone().expect("source drag items");
        let position = e.get_position(Some(&items));
        if !self.position_within_drag_bounds(position, &items) {
            self.drag_started.set(false);
            self.select_dragged_item_if_needed();
            self.released();
            self.captured.set(false);
            if let Some(outside) = &self.drag_outside {
                let args = self.pressed_args.borrow().clone();
                outside(args.as_deref(), e);
            }
            self.pressed_args.replace(None);
            return;
        }
        if !self.can_reorder_item() {
            self.drag_started.set(false);
            self.target_index.set(-1);
            let current_items = self.items.borrow().clone();
            self.reset_transforms(current_items);
            return;
        }
        let orientation = (self.get_orientation)();
        let delta = if orientation == Orientation::Horizontal {
            position.x - self.start.get().x
        } else {
            position.y - self.start.get().y
        };
        if !self.drag_started.get() {
            let diff = self.start.get() - position;
            let horizontal = (self.get_horizontal)();
            let vertical = (self.get_vertical)();
            let passes = if orientation == Orientation::Horizontal {
                diff.x.abs() > horizontal
            } else {
                diff.y.abs() > vertical
            };
            if !passes {
                return;
            }
            self.drag_started.set(true);
            self.select_dragged_item_if_needed();
        }
        let dragged = self
            .dragged
            .borrow()
            .clone()
            .expect("source dragged container");
        if orientation == Orientation::Horizontal {
            translate(&dragged, delta, 0.0);
        } else {
            translate(&dragged, 0.0, delta);
        }
        let items = self.items.borrow().clone().expect("source drag items");
        self.dragged_index.set(items.index_from_container(&dragged));
        self.target_index.set(-1);
        let rect = dragged.bounds();
        let (start, delta_start, delta_end) = if orientation == Orientation::Horizontal {
            (rect.x, rect.x + delta, rect.x + delta + rect.width)
        } else {
            (rect.y, rect.y + delta, rect.y + delta + rect.height)
        };
        for (index, _) in items.items().iter().enumerate() {
            let current_items = self.items.borrow().clone().expect("source drag items");
            let Some(target) = current_items.container_from_index(index as i32) else {
                continue;
            };
            if target.render_transform().is_none()
                || !target.is_visible()
                || !target.is_hit_test_visible()
                || target == dragged
            {
                continue;
            }
            let bounds = target.bounds();
            let (target_start, mid) = if orientation == Orientation::Horizontal {
                (bounds.x, bounds.x + bounds.width / 2.0)
            } else {
                (bounds.y, bounds.y + bounds.height / 2.0)
            };
            let current_items = self.items.borrow().clone().expect("source drag items");
            let target_index = current_items.index_from_container(&target);
            if target_start > start && delta_end >= mid {
                if orientation == Orientation::Horizontal {
                    translate(&target, -rect.width, 0.0);
                } else {
                    translate(&target, 0.0, -rect.height);
                }
                self.target_index.set(if self.target_index.get() == -1 {
                    target_index
                } else {
                    target_index.max(self.target_index.get())
                });
            } else if target_start < start && delta_start <= mid {
                if orientation == Orientation::Horizontal {
                    translate(&target, rect.width, 0.0);
                } else {
                    translate(&target, 0.0, rect.height);
                }
                self.target_index.set(if self.target_index.get() == -1 {
                    target_index
                } else {
                    target_index.min(self.target_index.get())
                });
            } else {
                translate(&target, 0.0, 0.0);
            }
        }
    }
    fn select_dragged_item_if_needed(&self) {
        let Some(items) = self
            .items
            .borrow()
            .clone()
            .and_then(|items| items.cast::<SelectingItemsControl>())
        else {
            return;
        };
        let Some(dragged) = self.dragged.borrow().clone() else {
            return;
        };
        if let Some(item) = dragged.data_context() {
            if !items_equal(&items.selected_item(), &Some(item.clone())) {
                items.set_selected_item(Some(item));
                return;
            }
        }
        let dragged = self
            .dragged
            .borrow()
            .clone()
            .expect("source dragged container");
        let index = items.index_from_container(&dragged);
        if index >= 0 && items.selected_index() != index {
            items.set_selected_index(index);
        }
    }
}
fn set_selected_index(items: &ItemsControl, index: i32) {
    if let Some(selecting) = items.to_ref().cast::<SelectingItemsControl>() {
        selecting.set_selected_index(index);
    }
}
fn dragging(control: &Control, yes: bool) {
    control.pseudo_classes().set(":dragging", yes);
}
fn translate(control: &Control, x: f64, y: f64) {
    let mut builder = TransformOperationsBuilder::new(1);
    builder.append_translate(x, y);
    control.set_render_transform(Some(builder.build()));
}
fn get_dragged_z_index(items: &ItemsControl, dragged: &Control) -> i32 {
    let mut max_z = 0;
    for container in items.get_realized_containers() {
        if !container.ptr_eq(&dragged.to_ref()) {
            max_z = max_z.max(container.z_index());
        }
    }
    1.max(max_z.wrapping_add(1))
}
fn same_vector(a: Vector, b: Vector) -> bool {
    (a.x == b.x || (a.x.is_nan() && b.x.is_nan())) && (a.y == b.y || (a.y.is_nan() && b.y.is_nan()))
}
fn min(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        a
    } else if b.is_nan() {
        b
    } else if a < b || (a == b && a.is_sign_negative()) {
        a
    } else {
        b
    }
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

#[cfg(test)]
#[path = "item_drag_helper_tests.rs"]
mod tests;
