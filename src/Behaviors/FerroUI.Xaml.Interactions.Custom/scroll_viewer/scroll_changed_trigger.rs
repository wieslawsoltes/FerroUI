//! Port of `ScrollViewer/ScrollChangedTrigger.cs`.

use crate::{
    AttachedToVisualTreeTriggerBaseImpl, RoutedEventTriggerBase, RoutedEventTriggerBaseOfT,
    RoutedEventTriggerBaseOfTImpl,
};
use ferroui_base::interactivity::{RoutedEvent, RoutingStrategies};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_controls::ScrollViewer;
use ferroui_xaml_interactivity::{DisposingTriggerImpl, StyledElementBehaviorImpl};

/// Executes actions when the [`ScrollViewer::scroll_changed_event`] event occurs.
///
/// The args of the event (`ScrollChangedEventArgs`) are the parameter of the actions.
#[repr(C)]
pub struct ScrollChangedTrigger {
    base: RoutedEventTriggerBaseOfT,
}

ferro_class!(ScrollChangedTrigger: RoutedEventTriggerBaseOfT);
ferro_class_info!(ScrollChangedTrigger { new: ScrollChangedTrigger::new });
ferroui_base::ferro_impl_classes!(
    ScrollChangedTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
    AttachedToVisualTreeTriggerBaseImpl,
);

ferroui_base::ferro_overrides! {
    impl RoutedEventTriggerBaseOfTImpl for ScrollChangedTrigger {
        fn routed_event(_this: &Self) -> RoutedEvent {
            ScrollViewer::scroll_changed_event().as_routed_event()
        }
    }
}

impl ScrollChangedTrigger {
    /// The static constructor: the event bubbles to the trigger by default.
    fn static_constructor() {
        RoutedEventTriggerBase::event_routing_strategy_property()
            .override_default_value::<ScrollChangedTrigger>(RoutingStrategies::BUBBLE);
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
