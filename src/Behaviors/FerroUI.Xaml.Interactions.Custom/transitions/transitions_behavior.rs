//! Port of `Transitions/TransitionsBehavior.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::animation::Transitions;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Ref, StaticType,
    StyledElementImpl, StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_behaviors_animations::TransitionOperations;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::cell::RefCell;
use std::rc::Rc;

/// Sets the transitions collection on the associated control when attached.
#[repr(C)]
pub struct TransitionsBehavior {
    base: AttachedToVisualTreeBehavior,
    old_transitions: RefCell<Option<Transitions>>,
}

ferro_class!(TransitionsBehavior: AttachedToVisualTreeBehavior);
ferro_class_info!(TransitionsBehavior { new: TransitionsBehavior::new });
ferroui_base::ferro_impl_classes!(
    TransitionsBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferro_properties! {
    impl TransitionsBehavior {
        /// Identifies the `TransitionsSource` property.
        pub fn transitions_source_property() -> StyledProperty<Option<Transitions>> {
            FerroProperty::register::<TransitionsBehavior, _>("TransitionsSource", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for TransitionsBehavior {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return DisposableAction::empty();
            };

            let old_transitions = TransitionOperations::replace(&associated_object, this.transitions_source());
            *this.old_transitions.borrow_mut() = old_transitions;

            let behavior = this.to_ref().downgrade();
            DisposableAction::create(move || {
                let Some(behavior) = behavior.upgrade() else {
                    return;
                };

                if let Some(associated_object) = behavior.associated_object_as::<Control>() {
                    let old_transitions = behavior.old_transitions.borrow().clone();
                    TransitionOperations::replace(&associated_object, old_transitions);
                }
            })
        }
    }
}

impl TransitionsBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: AttachedToVisualTreeBehavior::construct(<Control as StaticType>::TYPE),
            old_transitions: RefCell::new(None),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the transitions collection to apply. This is a property of the object model.
    pub fn transitions_source(&self) -> Option<Transitions> {
        self.get_value(Self::transitions_source_property())
    }

    pub fn set_transitions_source(&self, value: Option<Transitions>) {
        self.set_value(Self::transitions_source_property(), value)
    }
}
