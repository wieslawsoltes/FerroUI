//! Port of `InputElement/Triggers/PointerEnteredTrigger.cs`.

use crate::{AttachedToVisualTreeTriggerBaseImpl, RoutedEventTriggerBaseOfT, RoutedEventTriggerBaseOfTImpl};
use ferroui_base::input::InputElement;
use ferroui_base::interactivity::RoutedEvent;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::{DisposingTriggerImpl, StyledElementBehaviorImpl};

/// A trigger of [`InputElement::pointer_entered_event`] (a pointer entering the element): while the associated visual is in the
/// visual tree, the event runs the actions with its args (`PointerEventArgs`) as the parameter.
#[repr(C)]
pub struct PointerEnteredTrigger {
    base: RoutedEventTriggerBaseOfT,
}

ferro_class!(PointerEnteredTrigger: RoutedEventTriggerBaseOfT);
ferro_class_info!(PointerEnteredTrigger { new: PointerEnteredTrigger::new });
ferroui_base::ferro_impl_classes!(
    PointerEnteredTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
    AttachedToVisualTreeTriggerBaseImpl,
);

ferroui_base::ferro_overrides! {
    impl RoutedEventTriggerBaseOfTImpl for PointerEnteredTrigger {
        fn routed_event(_this: &Self) -> RoutedEvent {
            InputElement::pointer_entered_event().as_routed_event()
        }
    }
}

impl PointerEnteredTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: RoutedEventTriggerBaseOfT::construct() }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
