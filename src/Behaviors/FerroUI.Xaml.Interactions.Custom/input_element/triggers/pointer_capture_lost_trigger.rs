//! Port of `InputElement/Triggers/PointerCaptureLostTrigger.cs`.

use crate::{AttachedToVisualTreeTriggerBaseImpl, RoutedEventTriggerBaseOfT, RoutedEventTriggerBaseOfTImpl};
use ferroui_base::input::InputElement;
use ferroui_base::interactivity::RoutedEvent;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::{DisposingTriggerImpl, StyledElementBehaviorImpl};

/// A trigger of [`InputElement::pointer_capture_lost_event`] (the loss of the capture of a pointer): while the associated visual is in the
/// visual tree, the event runs the actions with its args (`PointerCaptureLostEventArgs`) as the parameter.
#[repr(C)]
pub struct PointerCaptureLostTrigger {
    base: RoutedEventTriggerBaseOfT,
}

ferro_class!(PointerCaptureLostTrigger: RoutedEventTriggerBaseOfT);
ferro_class_info!(PointerCaptureLostTrigger { new: PointerCaptureLostTrigger::new });
ferroui_base::ferro_impl_classes!(
    PointerCaptureLostTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
    AttachedToVisualTreeTriggerBaseImpl,
);

ferroui_base::ferro_overrides! {
    impl RoutedEventTriggerBaseOfTImpl for PointerCaptureLostTrigger {
        fn routed_event(_this: &Self) -> RoutedEvent {
            InputElement::pointer_capture_lost_event().as_routed_event()
        }
    }
}

impl PointerCaptureLostTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: RoutedEventTriggerBaseOfT::construct() }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
