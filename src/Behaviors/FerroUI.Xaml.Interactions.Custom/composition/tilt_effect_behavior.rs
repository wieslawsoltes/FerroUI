//! Port of `Composition/TiltEffectBehavior.cs`.

use ferroui_base::input::{InputElement, PointerEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Ref, StaticType,
    StyledProperty, Visual,
};
use ferroui_controls::{Control, SizeChangedEventArgs};
use ferroui_xaml_behaviors_animations::TiltAnimation;
use ferroui_xaml_interactivity::{BehaviorImpl, BehaviorImplExt, BehaviorOfT};
use std::cell::Cell;

/// A behavior that applies a 3D tilt rotation to the element based on the pointer position.
#[repr(C)]
pub struct TiltEffectBehavior {
    base: BehaviorOfT,
    // The handlers added to the associated object (the reference removes its delegates).
    pointer_moved_handler: Cell<Option<RoutedEventHandlerToken>>,
    pointer_exited_handler: Cell<Option<RoutedEventHandlerToken>>,
    size_changed_handler: Cell<Option<RoutedEventHandlerToken>>,
}

ferro_class!(TiltEffectBehavior: BehaviorOfT);
ferro_class_info!(TiltEffectBehavior { new: TiltEffectBehavior::new });
ferroui_base::ferro_impl_classes!(TiltEffectBehavior: FerroObjectImpl);

ferro_properties! {
    impl TiltEffectBehavior {
        /// Identifies the `TiltStrength` property.
        pub fn tilt_strength_property() -> StyledProperty<f64> {
            FerroProperty::register::<TiltEffectBehavior, _>("TiltStrength", 5.0)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl BehaviorImpl for TiltEffectBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            Self::parent_on_attached_to_visual_tree(this);

            if let Some(associated_object) = this.associated_object_as::<Control>() {
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
                this.pointer_exited_handler.set(Some(associated_object.add_handler(
                    InputElement::pointer_exited_event(),
                    move |_sender: &Interactive, _e: &PointerEventArgs| {
                        if let Some(behavior) = behavior.upgrade() {
                            behavior.on_pointer_exited();
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

                TiltAnimation::update_center_point(Some(&*associated_object));
            }
        }

        fn on_detached_from_visual_tree(this: &Self) {
            Self::parent_on_detached_from_visual_tree(this);

            if let Some(associated_object) = this.associated_object_as::<Control>() {
                if let Some(handler) = this.pointer_moved_handler.take() {
                    associated_object.remove_handler(InputElement::pointer_moved_event(), handler);
                }
                if let Some(handler) = this.pointer_exited_handler.take() {
                    associated_object.remove_handler(InputElement::pointer_exited_event(), handler);
                }
                if let Some(handler) = this.size_changed_handler.take() {
                    associated_object.remove_handler(Control::size_changed_event(), handler);
                }
            }
        }
    }
}

impl TiltEffectBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: BehaviorOfT::construct(<Control as StaticType>::TYPE),
            pointer_moved_handler: Cell::new(None),
            pointer_exited_handler: Cell::new(None),
            size_changed_handler: Cell::new(None),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the maximum tilt angle in degrees.
    pub fn tilt_strength(&self) -> f64 {
        self.get_value(Self::tilt_strength_property())
    }

    pub fn set_tilt_strength(&self, value: f64) {
        self.set_value(Self::tilt_strength_property(), value)
    }

    fn on_pointer_moved(&self, e: &PointerEventArgs) {
        if let Some(associated_object) = self.associated_object_as::<Control>() {
            let visual: &Visual = &associated_object;
            TiltAnimation::apply(Some(&*associated_object), e.get_position(Some(visual)), self.tilt_strength());
        }
    }

    fn on_pointer_exited(&self) {
        TiltAnimation::reset(self.associated_object_as::<Control>().as_deref());
    }

    fn on_size_changed(&self) {
        TiltAnimation::update_center_point(self.associated_object_as::<Control>().as_deref());
    }
}
