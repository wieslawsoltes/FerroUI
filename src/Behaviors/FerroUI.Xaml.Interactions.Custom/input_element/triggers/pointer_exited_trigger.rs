//! Port of `InputElement/Triggers/PointerExitedTrigger.cs`.

use crate::{AttachedToVisualTreeTriggerBaseImpl, RoutedEventTriggerBaseOfT, RoutedEventTriggerBaseOfTImpl};
use ferroui_base::input::InputElement;
use ferroui_base::interactivity::RoutedEvent;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::{DisposingTriggerImpl, StyledElementBehaviorImpl};

/// A trigger of [`InputElement::pointer_exited_event`] (a pointer leaving the element): while the associated visual is in the
/// visual tree, the event runs the actions with its args (`PointerEventArgs`) as the parameter.
#[repr(C)]
pub struct PointerExitedTrigger {
    base: RoutedEventTriggerBaseOfT,
}

ferro_class!(PointerExitedTrigger: RoutedEventTriggerBaseOfT);
ferro_class_info!(PointerExitedTrigger { new: PointerExitedTrigger::new });
ferroui_base::ferro_impl_classes!(
    PointerExitedTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
    AttachedToVisualTreeTriggerBaseImpl,
);

ferroui_base::ferro_overrides! {
    impl RoutedEventTriggerBaseOfTImpl for PointerExitedTrigger {
        fn routed_event(_this: &Self) -> RoutedEvent {
            InputElement::pointer_exited_event().as_routed_event()
        }
    }
}

impl PointerExitedTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: RoutedEventTriggerBaseOfT::construct() }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
