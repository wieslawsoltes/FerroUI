//! Port of `Show/ShowOnTappedBehavior.cs`.

use crate::{AttachedToVisualTreeBehaviorImpl, ShowBehaviorBase};
use ferroui_base::input::{InputElement, TappedEventArgs};
use ferroui_base::interactivity::Interactive;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// A behavior that allows to show control on tapped event.
#[repr(C)]
pub struct ShowOnTappedBehavior {
    base: ShowBehaviorBase,
}

ferro_class!(ShowOnTappedBehavior: ShowBehaviorBase);
ferro_class_info!(ShowOnTappedBehavior { new: ShowOnTappedBehavior::new });
ferroui_base::ferro_impl_classes!(
    ShowOnTappedBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for ShowOnTappedBehavior {
        /// Called when the behavior is attached to the visual tree.
        ///
        /// Returns a disposable that removes the event handler.
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return DisposableAction::empty();
            };

            let behavior = this.to_ref().downgrade();
            associated_object.add_disposable_handler_with(
                InputElement::tapped_event(),
                move |_sender: &Interactive, _e: &TappedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.show();
                    }
                },
                this.event_routing_strategy(),
                false,
            )
        }
    }
}

impl ShowOnTappedBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ShowBehaviorBase::construct() }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
