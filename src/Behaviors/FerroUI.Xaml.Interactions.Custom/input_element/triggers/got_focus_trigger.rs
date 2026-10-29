//! Port of `InputElement/Triggers/GotFocusTrigger.cs`.

use crate::{AttachedToVisualTreeTriggerBaseImpl, RoutedEventTriggerBaseOfT, RoutedEventTriggerBaseOfTImpl, RoutedEventTriggerBase};
use ferroui_base::input::InputElement;
use ferroui_base::interactivity::{RoutedEvent, RoutingStrategies};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::{DisposingTriggerImpl, StyledElementBehaviorImpl};

/// A trigger of [`InputElement::got_focus_event`] (the focus coming to the element): while the associated visual is in the
/// visual tree, the event runs the actions with its args (`FocusChangedEventArgs`) as the parameter.
#[repr(C)]
pub struct GotFocusTrigger {
    base: RoutedEventTriggerBaseOfT,
}

ferro_class!(GotFocusTrigger: RoutedEventTriggerBaseOfT);
ferro_class_info!(GotFocusTrigger { new: GotFocusTrigger::new });
ferroui_base::ferro_impl_classes!(
    GotFocusTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
    AttachedToVisualTreeTriggerBaseImpl,
);

ferroui_base::ferro_overrides! {
    impl RoutedEventTriggerBaseOfTImpl for GotFocusTrigger {
        fn routed_event(_this: &Self) -> RoutedEvent {
            InputElement::got_focus_event().as_routed_event()
        }
    }
}

impl GotFocusTrigger {
    /// The static constructor: the default routing strategies of the class.
    fn static_constructor() {
        RoutedEventTriggerBase::event_routing_strategy_property()
            .override_default_value::<GotFocusTrigger>(RoutingStrategies::BUBBLE);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: RoutedEventTriggerBaseOfT::construct() }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
