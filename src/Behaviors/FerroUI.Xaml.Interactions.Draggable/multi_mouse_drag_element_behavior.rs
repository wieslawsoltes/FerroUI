//! Port of `MultiMouseDragElementBehavior.cs`.

use crate::mouse_drag_element_behavior::{constrain_to_parent, translate_transform_of};
use crate::routed_handlers::RoutedHandlers;
use ferroui_base::input::{
    InputElement, MouseButton, PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs,
};
use ferroui_base::interactivity::{Interactive, RoutingStrategies};
use ferroui_base::media::TranslateTransform;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, DirectProperty, FerroObjectImpl, FerroProperty,
    Point, Ref, StaticType, StyledElementImpl, StyledProperty, Visual,
};
use ferroui_controls::{Control, Controls};
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::cell::{Cell, RefCell};

/// Enables dragging of multiple controls with the mouse using [`TranslateTransform`]s: the
/// associated control and the [`target_controls`](Self::target_controls) move together.
#[repr(C)]
pub struct MultiMouseDragElementBehavior {
    base: StyledElementBehaviorOfT,
    // Deviation (DEVIATIONS.md, Behaviors): `FerroList<Control>` of the reference is the
    // control collection of the framework (`Controls`, a list of controls).
    target_controls: RefCell<Option<Controls>>,
    captured: Cell<bool>,
    start: Cell<Point>,
    // The dictionary of the reference, in the order its entries were added.
    transforms: RefCell<Vec<(Ref<Control>, Ref<TranslateTransform>)>>,
    parent: RefCell<Option<Ref<Control>>>,
    handlers: RoutedHandlers,
}

ferro_class!(MultiMouseDragElementBehavior: StyledElementBehaviorOfT);
ferro_class_info!(MultiMouseDragElementBehavior {
    new: MultiMouseDragElementBehavior::new,
    markup: {
        content: TargetControls,
    },
});
ferroui_base::ferro_impl_classes!(MultiMouseDragElementBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl MultiMouseDragElementBehavior {
        /// Identifies the `TargetControls` property.
        pub fn target_controls_property() -> DirectProperty<MultiMouseDragElementBehavior, Controls> {
            FerroProperty::register_direct::<MultiMouseDragElementBehavior, _>(
                "TargetControls",
                |behavior| behavior.target_controls(),
                None,
                Controls::new(),
            )
        }

        /// Identifies the `ConstrainToParentBounds` property.
        pub fn constrain_to_parent_bounds_property() -> StyledProperty<bool> {
            FerroProperty::register::<MultiMouseDragElementBehavior, _>("ConstrainToParentBounds", false)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for MultiMouseDragElementBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return;
            };

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
                InputElement::pointer_released_event(),
                move |_: &Interactive, e: &PointerReleasedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.released(e);
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

impl MultiMouseDragElementBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE),
            target_controls: RefCell::new(None),
            captured: Cell::new(false),
            start: Cell::new(Point::default()),
            transforms: RefCell::new(Vec::new()),
            parent: RefCell::new(None),
            handlers: RoutedHandlers::new(),
        }
    }

    /// Creates the behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the collection of controls that should be dragged together (created on first use).
    pub fn target_controls(&self) -> Controls {
        self.target_controls.borrow_mut().get_or_insert_with(Controls::new).clone()
    }

    /// Gets or sets whether dragging should be constrained to the bounds of the parent control.
    pub fn constrain_to_parent_bounds(&self) -> bool {
        self.get_value(Self::constrain_to_parent_bounds_property())
    }

    pub fn set_constrain_to_parent_bounds(&self, value: bool) {
        self.set_value(Self::constrain_to_parent_bounds_property(), value)
    }

    fn pressed(&self, e: &PointerPressedEventArgs) {
        let properties = e.properties();
        let Some(associated_object) = self.associated_object_as::<Control>() else {
            return;
        };
        let parent = associated_object.parent().and_then(|parent| parent.cast::<Control>());
        let Some(parent) = parent.filter(|_| properties.is_left_button_pressed && self.is_enabled()) else {
            return;
        };

        let parent_visual: &Visual = &parent;
        self.start.set(e.get_position(Some(parent_visual)));
        *self.parent.borrow_mut() = Some(parent);
        self.transforms.borrow_mut().clear();

        self.add_transform(Some(&associated_object));
        for control in self.target_controls().to_vec() {
            self.add_transform(Some(&control));
        }

        self.captured.set(true);
    }

    fn released(&self, e: &PointerReleasedEventArgs) {
        if self.captured.get() && e.initial_press_mouse_button() == MouseButton::Left {
            self.end_drag();
        }
    }

    fn capture_lost(&self, _e: &PointerCaptureLostEventArgs) {
        if self.captured.get() {
            self.end_drag();
        }
    }

    fn moved(&self, e: &PointerEventArgs) {
        let properties = e.properties();
        let parent = self.parent.borrow().clone();
        let Some(parent) = parent else {
            return;
        };
        if !self.captured.get() || !properties.is_left_button_pressed || !self.is_enabled() {
            return;
        }

        let parent_visual: &Visual = &parent;
        let position = e.get_position(Some(parent_visual));
        let start = self.start.get();
        let dx = position.x - start.x;
        let dy = position.y - start.y;
        self.start.set(position);

        let transforms = self.transforms.borrow().clone();
        for (element, transform) in transforms {
            let mut new_x = transform.x() + dx;
            let mut new_y = transform.y() + dy;

            if self.constrain_to_parent_bounds() {
                if let Some(p) = element.parent().and_then(|p| p.cast::<Control>()) {
                    (new_x, new_y) = constrain_to_parent(new_x, new_y, p.bounds(), element.bounds());
                }
            }

            transform.set_x(new_x);
            transform.set_y(new_y);
        }
    }

    fn add_transform(&self, element: Option<&Ref<Control>>) {
        let Some(element) = element else {
            return;
        };

        let transform = translate_transform_of(element);
        let mut transforms = self.transforms.borrow_mut();
        match transforms.iter_mut().find(|(key, _)| key.ptr_eq(element)) {
            Some(entry) => entry.1 = transform,
            None => transforms.push((element.clone(), transform)),
        }
    }

    fn end_drag(&self) {
        self.captured.set(false);
        *self.parent.borrow_mut() = None;
        self.transforms.borrow_mut().clear();
    }
}
