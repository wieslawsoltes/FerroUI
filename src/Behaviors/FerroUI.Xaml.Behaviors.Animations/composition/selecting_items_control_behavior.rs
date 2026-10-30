//! Ported from `Composition/SelectingItemsControlBehavior.cs`.
//!
//! Based on the code of the list box animation samples of adirh3, as the reference is (see
//! `NOTICE.md`).

use crate::SelectionIndicatorAnimation;
use ferroui_base::{AttachedProperty, FerroProperty, FerroPropertyChangedEventArgs};
use ferroui_controls::primitives::{SelectingItemsControl, TemplatedControl};
use ferroui_controls::Control;
use std::rc::Rc;

/// Enables the standard selection indicator animation on selecting items controls.
pub struct SelectingItemsControlBehavior;

ferroui_base::ferro_static_type!(SelectingItemsControlBehavior);

ferroui_base::ferro_properties! {
    impl SelectingItemsControlBehavior {
        /// Identifies the `EnableSelectionAnimation` attached property
        /// ([`get_enable_selection_animation`](SelectingItemsControlBehavior::get_enable_selection_animation)).
        pub fn enable_selection_animation_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<SelectingItemsControlBehavior, SelectingItemsControl, bool>(
                "EnableSelectionAnimation",
                false,
            )
        }

        /// The subscription of the property changed handler of a control whose selection is
        /// animated: the token of the handler, to remove it when the animation is disabled.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the reference adds and removes the same static
        /// method with `PropertyChanged +=` and `-=`; a handler of a Rust object has no identity
        /// to remove it by, so the token of the subscription is kept on the control.
        fn selection_animation_subscription_property() -> AttachedProperty<Option<u64>> {
            FerroProperty::register_attached::<SelectingItemsControlBehavior, SelectingItemsControl, Option<u64>>(
                "SelectionAnimationSubscription",
                None,
            )
        }
    }
}

impl SelectingItemsControlBehavior {
    /// Initializes the attached-property change handler.
    fn static_constructor() {
        // The handler stays registered for the life of the thread, as the class handler of the
        // static constructor of the reference does for the life of the process.
        let _ = Self::enable_selection_animation_property()
            .changed()
            .add_class_handler::<Control>(Self::on_enable_selection_animation);
    }

    fn on_enable_selection_animation(control: &Control, args: &FerroPropertyChangedEventArgs<'_>) {
        let Some(selecting_items_control) = control.downcast_ref::<SelectingItemsControl>() else {
            return;
        };

        let subscription = Self::selection_animation_subscription_property();
        if args.new_value().downcast_ref::<bool>() == Some(&true) {
            let token = selecting_items_control
                .add_property_changed_handler(Rc::new(Self::selecting_items_control_property_changed));
            selecting_items_control.set_value(subscription, Some(token));
        } else if let Some(token) = selecting_items_control.get_value(subscription) {
            selecting_items_control.remove_property_changed_handler(token);
            selecting_items_control.clear_value(subscription);
        }
    }

    fn selecting_items_control_property_changed(args: &FerroPropertyChangedEventArgs<'_>) {
        let Some(selecting_items_control) = args.sender().downcast_ref::<SelectingItemsControl>() else {
            return;
        };
        if args.property() != SelectingItemsControl::selected_index_property().as_property() {
            return;
        }
        let (Some(old_index), Some(new_index)) = (
            args.old_value().and_then(|value| value.downcast_ref::<i32>()).copied(),
            args.new_value().downcast_ref::<i32>().copied(),
        ) else {
            return;
        };

        let (Some(new_selection), Some(old_selection)) = (
            selecting_items_control.container_from_index(new_index).and_then(|c| c.cast::<TemplatedControl>()),
            selecting_items_control.container_from_index(old_index).and_then(|c| c.cast::<TemplatedControl>()),
        ) else {
            return;
        };

        SelectionIndicatorAnimation::try_start(
            Some(&new_selection),
            Some(&old_selection),
            SelectionIndicatorAnimation::default_duration(),
        );
    }

    /// Gets whether selection indicator animation is enabled for the selecting items control.
    pub fn get_enable_selection_animation(element: &SelectingItemsControl) -> bool {
        element.get_value(Self::enable_selection_animation_property())
    }

    /// Sets whether selection indicator animation is enabled for the selecting items control.
    pub fn set_enable_selection_animation(element: &SelectingItemsControl, value: bool) {
        element.set_value(Self::enable_selection_animation_property(), value);
    }
}
