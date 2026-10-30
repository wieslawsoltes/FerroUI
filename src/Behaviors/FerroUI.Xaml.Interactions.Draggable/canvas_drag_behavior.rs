//! Port of `CanvasDragBehavior.cs`.

use crate::routed_handlers::RoutedHandlers;
use crate::{ItemDragBehavior, SelectionAdorner};
use ferroui_base::input::{
    InputElement, MouseButton, PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs,
};
use ferroui_base::interactivity::{Interactive, RoutingStrategies};
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Point, Ref, StaticType, StyledElementImpl, Visual,
};
use ferroui_controls::primitives::AdornerLayer;
use ferroui_controls::{Canvas, Control};
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::cell::{Cell, RefCell};

/// Enables dragging of child controls within a [`Canvas`]: the associated control follows the
/// pointer by its `Canvas.Left` and `Canvas.Top`.
#[repr(C)]
pub struct CanvasDragBehavior {
    base: StyledElementBehaviorOfT,
    enable_drag: Cell<bool>,
    start: Cell<Point>,
    parent: RefCell<Option<Ref<Control>>>,
    dragged_container: RefCell<Option<Ref<Control>>>,
    adorner: RefCell<Option<Ref<Control>>>,
    captured: Cell<bool>,
    handlers: RoutedHandlers,
}

ferro_class!(CanvasDragBehavior: StyledElementBehaviorOfT);
ferro_class_info!(CanvasDragBehavior { new: CanvasDragBehavior::new });
ferroui_base::ferro_impl_classes!(CanvasDragBehavior: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for CanvasDragBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &associated_object,
                InputElement::pointer_released_event(),
                move |_: &Interactive, e: &PointerReleasedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.released_event(e);
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
                        behavior.pressed(e);
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
                        behavior.moved(e);
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
                        behavior.capture_lost(e);
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );
        }

        fn on_detached_from_visual_tree(this: &Self) {
            this.handlers.clear();
        }
    }
}

impl CanvasDragBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE),
            enable_drag: Cell::new(false),
            start: Cell::new(Point::default()),
            parent: RefCell::new(None),
            dragged_container: RefCell::new(None),
            adorner: RefCell::new(None),
            captured: Cell::new(false),
            handlers: RoutedHandlers::new(),
        }
    }

    /// Creates the behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Shows the selection adorner over `control` (not used by the reference: its call is
    /// commented out).
    #[allow(dead_code)]
    fn add_adorner(&self, control: &Ref<Control>) {
        let Some(layer) = AdornerLayer::get_adorner_layer(control) else {
            return;
        };

        let adorner = SelectionAdorner::new();
        AdornerLayer::set_adorned_element(&adorner, control);

        adorner.set_parent(control);
        layer.children().add(adorner.clone());
        *self.adorner.borrow_mut() = Some(adorner.upcast());
    }

    /// Removes the selection adorner (not used by the reference: its call is commented out).
    #[allow(dead_code)]
    fn remove_adorner(&self, control: &Ref<Control>) {
        let adorner = self.adorner.borrow().clone();
        let (Some(layer), Some(adorner)) = (AdornerLayer::get_adorner_layer(control), adorner) else {
            return;
        };

        layer.children().remove(adorner.clone());
        adorner.set_parent(None);
        *self.adorner.borrow_mut() = None;
    }

    fn pressed(&self, e: &PointerPressedEventArgs) {
        let properties = e.properties();
        let parent = self
            .associated_object_as::<Control>()
            .and_then(|associated_object| associated_object.parent())
            .and_then(|parent| parent.cast::<Control>());
        let Some(parent) = parent.filter(|_| properties.is_left_button_pressed && self.is_enabled()) else {
            return;
        };

        self.enable_drag.set(true);
        let parent_visual: &Visual = &parent;
        self.start.set(e.get_position(Some(parent_visual)));
        *self.parent.borrow_mut() = Some(parent.clone());
        let dragged_container = self.associated_object_as::<Control>();
        *self.dragged_container.borrow_mut() = dragged_container.clone();

        if let Some(dragged_container) = &dragged_container {
            ItemDragBehavior::set_dragging_pseudo_classes(dragged_container, true);
        }

        // AddAdorner(_draggedContainer);

        self.captured.set(true);
    }

    fn released_event(&self, e: &PointerReleasedEventArgs) {
        if self.captured.get() {
            if e.initial_press_mouse_button() == MouseButton::Left {
                self.released();
            }

            self.captured.set(false);
        }
    }

    fn capture_lost(&self, _e: &PointerCaptureLostEventArgs) {
        self.released();
        self.captured.set(false);
    }

    fn moved(&self, e: &PointerEventArgs) {
        let properties = e.properties();
        if !(self.captured.get() && properties.is_left_button_pressed && self.is_enabled()) {
            return;
        }

        let parent = self.parent.borrow().clone();
        let dragged_container = self.dragged_container.borrow().clone();
        let (Some(parent), Some(dragged_container)) = (parent, dragged_container) else {
            return;
        };
        if !self.enable_drag.get() {
            return;
        }

        let parent_visual: &Visual = &parent;
        let position = e.get_position(Some(parent_visual));
        let start = self.start.get();
        let delta_x = position.x - start.x;
        let delta_y = position.y - start.y;
        self.start.set(position);
        let left = Canvas::get_left(&dragged_container);
        let top = Canvas::get_top(&dragged_container);
        Canvas::set_left(&dragged_container, left + delta_x);
        Canvas::set_top(&dragged_container, top + delta_y);
    }

    fn released(&self) {
        if self.enable_drag.get() {
            if self.parent.borrow().is_some() && self.dragged_container.borrow().is_some() {
                // RemoveAdorner(_draggedContainer);
            }

            let dragged_container = self.dragged_container.borrow().clone();
            if let Some(dragged_container) = &dragged_container {
                ItemDragBehavior::set_dragging_pseudo_classes(dragged_container, false);
            }

            self.enable_drag.set(false);
            *self.parent.borrow_mut() = None;
            *self.dragged_container.borrow_mut() = None;
        }
    }
}
