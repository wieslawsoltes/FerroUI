//! Port of `ListReorderDragBehavior.cs`.

use crate::item_drag_behavior::{exceeds_drag_threshold, next_target_index, target_shift};
use crate::routed_handlers::RoutedHandlers;
use crate::ItemDragBehavior;
use ferroui_base::input::{
    InputElement, MouseButton, PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs,
};
use ferroui_base::interactivity::{Interactive, RoutingStrategies};
use ferroui_base::layout::Orientation;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::ITemplate;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Point,
    Ref, StyledElementImpl, StyledProperty, Visual,
};
use ferroui_controls::primitives::AdornerLayer;
use ferroui_controls::{Control, ItemsControl};
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorImplExt};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Allows reordering of items inside an [`ItemsControl`] while displaying a placeholder at the
/// insertion point.
///
/// The placeholder is built from [`placeholder_template`](Self::placeholder_template) and shown in
/// the adorner layer over the container at the index the item would be dropped at.
#[repr(C)]
pub struct ListReorderDragBehavior {
    base: ItemDragBehavior,
    enable_drag: Cell<bool>,
    drag_started: Cell<bool>,
    start: Cell<Point>,
    dragged_index: Cell<i32>,
    target_index: Cell<i32>,
    items_control: RefCell<Option<Ref<ItemsControl>>>,
    dragged_container: RefCell<Option<Ref<Control>>>,
    captured: Cell<bool>,
    placeholder: RefCell<Option<Ref<Control>>>,
    placeholder_container: RefCell<Option<Ref<Control>>>,
    handlers: RoutedHandlers,
}

ferro_class!(ListReorderDragBehavior: ItemDragBehavior);
ferro_class_info!(ListReorderDragBehavior { new: ListReorderDragBehavior::new });
ferroui_base::ferro_impl_classes!(ListReorderDragBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ListReorderDragBehavior {
        /// Identifies the `PlaceholderTemplate` property.
        pub fn placeholder_template_property() -> StyledProperty<Option<Rc<dyn ITemplate>>> {
            FerroProperty::register::<ListReorderDragBehavior, _>("PlaceholderTemplate", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ListReorderDragBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            Self::parent_on_attached_to_visual_tree(this);

            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &associated_object,
                InputElement::pointer_released_event(),
                move |_: &Interactive, e: &PointerReleasedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.on_pointer_released(e);
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );
            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &associated_object,
                InputElement::pointer_pressed_event(),
                move |_: &Interactive, e: &PointerPressedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.on_pointer_pressed(e);
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );
            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &associated_object,
                InputElement::pointer_moved_event(),
                move |_: &Interactive, e: &PointerEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.on_pointer_moved(e);
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );
            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &associated_object,
                InputElement::pointer_capture_lost_event(),
                move |_: &Interactive, e: &PointerCaptureLostEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.on_pointer_capture_lost(e);
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );
        }

        fn on_detached_from_visual_tree(this: &Self) {
            Self::parent_on_detached_from_visual_tree(this);
            this.handlers.clear();
        }
    }
}

/// Not from the reference: the control an [`ITemplate`] built (`Build() is Control`): the value
/// itself, or the content of an object template, which builds an optional untyped value.
fn built_control(built: BoxedValue) -> Option<Ref<Control>> {
    let value = match built.downcast_ref::<Option<BoxedValue>>() {
        Some(inner) => inner.clone(),
        None => Some(built),
    };
    from_markup_value::<Ref<Control>>(&value)
}

impl ListReorderDragBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ItemDragBehavior::construct(),
            enable_drag: Cell::new(false),
            drag_started: Cell::new(false),
            start: Cell::new(Point::default()),
            dragged_index: Cell::new(0),
            target_index: Cell::new(0),
            items_control: RefCell::new(None),
            dragged_container: RefCell::new(None),
            captured: Cell::new(false),
            placeholder: RefCell::new(None),
            placeholder_container: RefCell::new(None),
            handlers: RoutedHandlers::new(),
        }
    }

    /// Creates the behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets template used to build placeholder shown while reordering.
    pub fn placeholder_template(&self) -> Option<Rc<dyn ITemplate>> {
        self.get_value(Self::placeholder_template_property())
    }

    pub fn set_placeholder_template(&self, value: Option<Rc<dyn ITemplate>>) {
        self.set_value(Self::placeholder_template_property(), value)
    }

    fn on_pointer_pressed(&self, e: &PointerPressedEventArgs) {
        let properties = e.properties();
        let items_control = self
            .associated_object_as::<Control>()
            .and_then(|associated_object| associated_object.parent())
            .and_then(|parent| parent.cast::<ItemsControl>());
        let Some(items_control) = items_control.filter(|_| properties.is_left_button_pressed && self.is_enabled())
        else {
            return;
        };

        self.enable_drag.set(true);
        self.drag_started.set(false);
        let items_control_visual: &Visual = &items_control;
        self.start.set(e.get_position(Some(items_control_visual)));
        self.dragged_index.set(-1);
        self.target_index.set(-1);
        *self.items_control.borrow_mut() = Some(items_control);
        *self.dragged_container.borrow_mut() = self.associated_object_as::<Control>();
        self.captured.set(true);
    }

    fn on_pointer_released(&self, e: &PointerReleasedEventArgs) {
        if self.captured.get() {
            if e.initial_press_mouse_button() == MouseButton::Left {
                self.remove_placeholder();
            }
            self.captured.set(false);
        }
    }

    fn on_pointer_capture_lost(&self, _e: &PointerCaptureLostEventArgs) {
        self.remove_placeholder();
        self.captured.set(false);
    }

    fn remove_placeholder(&self) {
        let placeholder_container = self.placeholder_container.borrow().clone();
        let placeholder = self.placeholder.borrow().clone();
        if let (Some(placeholder_container), Some(placeholder)) = (placeholder_container, placeholder) {
            if let Some(layer) = AdornerLayer::get_adorner_layer(&placeholder_container) {
                layer.children().remove(placeholder.clone());
            }
            placeholder.set_parent(None);
        }

        *self.placeholder.borrow_mut() = None;
        *self.placeholder_container.borrow_mut() = None;
        self.enable_drag.set(false);
        self.drag_started.set(false);
        *self.items_control.borrow_mut() = None;
        *self.dragged_container.borrow_mut() = None;
    }

    fn add_placeholder(&self, container: &Ref<Control>) {
        let Some(placeholder_template) = self.placeholder_template() else {
            return;
        };

        if self.placeholder_container.borrow().as_ref().is_some_and(|current| current.ptr_eq(container)) {
            return;
        }

        self.remove_placeholder();

        let Some(layer) = AdornerLayer::get_adorner_layer(container) else {
            return;
        };

        if let Some(adorner) = built_control(placeholder_template.build()) {
            AdornerLayer::set_adorned_element(&adorner, container);
            adorner.set_parent(container);
            layer.children().add(adorner.clone());
            *self.placeholder.borrow_mut() = Some(adorner);
            *self.placeholder_container.borrow_mut() = Some(container.clone());
        }
    }

    fn on_pointer_moved(&self, e: &PointerEventArgs) {
        let properties = e.properties();
        if !(self.captured.get() && properties.is_left_button_pressed && self.is_enabled()) {
            return;
        }

        let items_control = self.items_control.borrow().clone();
        let Some(items_control) = items_control.filter(|_| self.enable_drag.get()) else {
            return;
        };

        let orientation = self.orientation();
        let items_control_visual: &Visual = &items_control;
        let position = e.get_position(Some(items_control_visual));
        let start = self.start.get();
        let delta = match orientation {
            Orientation::Horizontal => position.x - start.x,
            Orientation::Vertical => position.y - start.y,
        };

        if !self.drag_started.get() {
            if exceeds_drag_threshold(
                orientation,
                start,
                position,
                self.horizontal_drag_threshold(),
                self.vertical_drag_threshold(),
            ) {
                self.drag_started.set(true);
            } else {
                return;
            }
        }

        let dragged_container = self.dragged_container.borrow().clone();
        let Some(dragged_container) = dragged_container else {
            return;
        };

        self.dragged_index.set(items_control.index_from_container(&dragged_container));
        self.target_index.set(-1);

        let dragged_bounds = dragged_container.bounds();

        for i in 0..items_control.items().count() {
            let Some(target_container) = items_control.container_from_index(i as i32) else {
                continue;
            };
            if target_container.ptr_eq(&dragged_container) {
                continue;
            }

            let target_index = items_control.index_from_container(&target_container);
            let shift = target_shift(orientation, dragged_bounds, delta, target_container.bounds());
            self.target_index.set(next_target_index(self.target_index.get(), target_index, shift));
        }

        let target_index = self.target_index.get();
        if target_index >= 0 {
            if let Some(container) = items_control.container_from_index(target_index) {
                self.add_placeholder(&container);
            }
        } else {
            self.remove_placeholder();
        }
    }
}
