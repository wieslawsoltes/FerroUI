//! Port of `MouseDragElementBehavior.cs`.

use crate::routed_handlers::RoutedHandlers;
use ferroui_base::input::{
    InputElement, MouseButton, PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs,
};
use ferroui_base::interactivity::{Interactive, RoutingStrategies};
use ferroui_base::media::{ITransform, TranslateTransform};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Point, Rect, Ref,
    StaticType, StyledElementImpl, StyledProperty, Visual,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Enables dragging of a control with the mouse using a [`TranslateTransform`].
#[repr(C)]
pub struct MouseDragElementBehavior {
    base: StyledElementBehaviorOfT,
    captured: Cell<bool>,
    start: Cell<Point>,
    parent: RefCell<Option<Ref<Control>>>,
    transform: RefCell<Option<Ref<TranslateTransform>>>,
    handlers: RoutedHandlers,
}

ferro_class!(MouseDragElementBehavior: StyledElementBehaviorOfT);
ferro_class_info!(MouseDragElementBehavior { new: MouseDragElementBehavior::new });
ferroui_base::ferro_impl_classes!(MouseDragElementBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl MouseDragElementBehavior {
        /// Identifies the `ConstrainToParentBounds` property.
        pub fn constrain_to_parent_bounds_property() -> StyledProperty<bool> {
            FerroProperty::register::<MouseDragElementBehavior, _>("ConstrainToParentBounds", false)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for MouseDragElementBehavior {
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

/// Not from the reference: the translation `(x, y)` of an element whose bounds are `bounds`,
/// clamped so that the element stays within the bounds of its parent (`parent_bounds`), as
/// `ConstrainToParentBounds` does.
pub(crate) fn constrain_to_parent(x: f64, y: f64, parent_bounds: Rect, bounds: Rect) -> (f64, f64) {
    let min_x = -bounds.x;
    let min_y = -bounds.y;
    let max_x = parent_bounds.width - bounds.width - bounds.x;
    let max_y = parent_bounds.height - bounds.height - bounds.y;

    (x.max(min_x).min(max_x), y.max(min_y).min(max_y))
}

/// Not from the reference: the translate transform of a control (`RenderTransform is
/// TranslateTransform`), or a new one set as its render transform.
pub(crate) fn translate_transform_of(control: &Control) -> Ref<TranslateTransform> {
    let current = control
        .render_transform()
        .and_then(|transform| transform.as_object().and_then(|object| object.to_ref().cast::<TranslateTransform>()));
    match current {
        Some(transform) => transform,
        None => {
            let transform = TranslateTransform::new();
            let render_transform: Rc<dyn ITransform> = (&transform).into();
            control.set_render_transform(Some(render_transform));
            transform
        }
    }
}

impl MouseDragElementBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE),
            captured: Cell::new(false),
            start: Cell::new(Point::default()),
            parent: RefCell::new(None),
            transform: RefCell::new(None),
            handlers: RoutedHandlers::new(),
        }
    }

    /// Creates the behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
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
        *self.transform.borrow_mut() = Some(translate_transform_of(&associated_object));

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
        let transform = self.transform.borrow().clone();
        let (Some(parent), Some(transform)) = (parent, transform) else {
            return;
        };
        if !self.captured.get() || !properties.is_left_button_pressed || !self.is_enabled() {
            return;
        }

        let parent_visual: &Visual = &parent;
        let position = e.get_position(Some(parent_visual));
        let start = self.start.get();
        let delta_x = position.x - start.x;
        let delta_y = position.y - start.y;
        self.start.set(position);

        let mut new_x = transform.x() + delta_x;
        let mut new_y = transform.y() + delta_y;

        if self.constrain_to_parent_bounds() {
            if let Some(element) = self.associated_object_as::<Control>() {
                (new_x, new_y) = constrain_to_parent(new_x, new_y, parent.bounds(), element.bounds());
            }
        }

        transform.set_x(new_x);
        transform.set_y(new_y);
    }

    fn end_drag(&self) {
        self.captured.set(false);
        *self.parent.borrow_mut() = None;
        *self.transform.borrow_mut() = None;
    }
}
