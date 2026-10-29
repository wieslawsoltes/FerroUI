//! Port of `Composition/OrbitEffectBehavior.cs`.

use ferroui_base::input::{InputElement, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Point, Ref,
    StaticType, StyledElementImpl, StyledProperty, Vector, Visual,
};
use ferroui_controls::{Control, SizeChangedEventArgs};
use ferroui_xaml_behaviors_animations::OrbitAnimation;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementBehaviorOfT};
use std::cell::Cell;

/// A behavior that allows rotating the attached control in 3D space using pointer manipulation.
#[repr(C)]
pub struct OrbitEffectBehavior {
    base: StyledElementBehaviorOfT,
    animation: OrbitAnimation,
    is_pressed: Cell<bool>,
    last_position: Cell<Point>,
    // The handlers added to the associated object (the reference removes its delegates).
    pointer_pressed_handler: Cell<Option<RoutedEventHandlerToken>>,
    pointer_moved_handler: Cell<Option<RoutedEventHandlerToken>>,
    pointer_released_handler: Cell<Option<RoutedEventHandlerToken>>,
    size_changed_handler: Cell<Option<RoutedEventHandlerToken>>,
}

ferro_class!(OrbitEffectBehavior: StyledElementBehaviorOfT);
ferro_class_info!(OrbitEffectBehavior { new: OrbitEffectBehavior::new });
ferroui_base::ferro_impl_classes!(OrbitEffectBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl OrbitEffectBehavior {
        /// Identifies the `Sensitivity` property.
        pub fn sensitivity_property() -> StyledProperty<f64> {
            FerroProperty::register::<OrbitEffectBehavior, _>("Sensitivity", 0.5)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for OrbitEffectBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            Self::parent_on_attached_to_visual_tree(this);

            if let Some(associated_object) = this.associated_object_as::<Control>() {
                let behavior = this.to_ref().downgrade();
                this.pointer_pressed_handler.set(Some(associated_object.add_handler(
                    InputElement::pointer_pressed_event(),
                    move |_sender: &Interactive, e: &PointerPressedEventArgs| {
                        if let Some(behavior) = behavior.upgrade() {
                            behavior.on_pointer_pressed(e);
                        }
                    },
                )));

                let behavior = this.to_ref().downgrade();
                this.pointer_moved_handler.set(Some(associated_object.add_handler(
                    InputElement::pointer_moved_event(),
                    move |_sender: &Interactive, e: &PointerEventArgs| {
                        if let Some(behavior) = behavior.upgrade() {
                            behavior.on_pointer_moved(e);
                        }
                    },
                )));

                let behavior = this.to_ref().downgrade();
                this.pointer_released_handler.set(Some(associated_object.add_handler(
                    InputElement::pointer_released_event(),
                    move |_sender: &Interactive, e: &PointerReleasedEventArgs| {
                        if let Some(behavior) = behavior.upgrade() {
                            behavior.on_pointer_released(e);
                        }
                    },
                )));

                let behavior = this.to_ref().downgrade();
                this.size_changed_handler.set(Some(associated_object.size_changed(
                    move |_sender: &Interactive, _e: &SizeChangedEventArgs| {
                        if let Some(behavior) = behavior.upgrade() {
                            behavior.on_size_changed();
                        }
                    },
                )));

                OrbitAnimation::update_center_point(Some(&*associated_object));
            }
        }

        fn on_detached_from_visual_tree(this: &Self) {
            Self::parent_on_detached_from_visual_tree(this);

            if let Some(associated_object) = this.associated_object_as::<Control>() {
                if let Some(handler) = this.pointer_pressed_handler.take() {
                    associated_object.remove_handler(InputElement::pointer_pressed_event(), handler);
                }
                if let Some(handler) = this.pointer_moved_handler.take() {
                    associated_object.remove_handler(InputElement::pointer_moved_event(), handler);
                }
                if let Some(handler) = this.pointer_released_handler.take() {
                    associated_object.remove_handler(InputElement::pointer_released_event(), handler);
                }
                if let Some(handler) = this.size_changed_handler.take() {
                    associated_object.remove_handler(Control::size_changed_event(), handler);
                }
            }

            this.is_pressed.set(false);
        }
    }
}

impl OrbitEffectBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE),
            animation: OrbitAnimation::new(),
            is_pressed: Cell::new(false),
            last_position: Cell::new(Point::default()),
            pointer_pressed_handler: Cell::new(None),
            pointer_moved_handler: Cell::new(None),
            pointer_released_handler: Cell::new(None),
            size_changed_handler: Cell::new(None),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the sensitivity of the rotation.
    pub fn sensitivity(&self) -> f64 {
        self.get_value(Self::sensitivity_property())
    }

    pub fn set_sensitivity(&self, value: f64) {
        self.set_value(Self::sensitivity_property(), value)
    }

    fn on_pointer_pressed(&self, e: &PointerPressedEventArgs) {
        self.is_pressed.set(true);
        let associated_object = self.associated_object_as::<Control>();
        let position = match &associated_object {
            Some(associated_object) => {
                let visual: &Visual = associated_object;
                e.get_position(Some(visual))
            }
            None => e.get_position(None),
        };
        self.last_position.set(position);
        let element: Option<Ref<InputElement>> = associated_object.map(|control| control.upcast());
        e.pointer().capture(element.as_ref());
    }

    fn on_pointer_released(&self, e: &PointerReleasedEventArgs) {
        self.is_pressed.set(false);
        e.pointer().capture(None);
    }

    fn on_pointer_moved(&self, e: &PointerEventArgs) {
        if !self.is_pressed.get() {
            return;
        }
        let Some(associated_object) = self.associated_object_as::<Control>() else {
            return;
        };

        let visual: &Visual = &associated_object;
        let current_position = e.get_position(Some(visual));
        let delta = Vector::from(current_position - self.last_position.get());
        self.last_position.set(current_position);

        self.animation.rotate(Some(&*associated_object), delta, self.sensitivity());
    }

    fn on_size_changed(&self) {
        OrbitAnimation::update_center_point(self.associated_object_as::<Control>().as_deref());
    }
}
