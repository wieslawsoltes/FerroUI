//! Port of `ToolTips/ToolTipOpeningTrigger.cs`.

use crate::{
    AttachedToVisualTreeTriggerBaseImpl, RoutedEventTriggerBase, RoutedEventTriggerBaseOfT,
    RoutedEventTriggerBaseOfTImpl,
};
use ferroui_base::interactivity::{RoutedEvent, RoutingStrategies};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_controls::ToolTip;
use ferroui_xaml_interactivity::{DisposingTriggerImpl, StyledElementBehaviorImpl};

/// Trigger that listens for the [`ToolTip::tool_tip_opening_event`].
///
/// The args of the event (`CancelRoutedEventArgs`) are the parameter of the actions.
#[repr(C)]
pub struct ToolTipOpeningTrigger {
    base: RoutedEventTriggerBaseOfT,
}

ferro_class!(ToolTipOpeningTrigger: RoutedEventTriggerBaseOfT);
ferro_class_info!(ToolTipOpeningTrigger { new: ToolTipOpeningTrigger::new });
ferroui_base::ferro_impl_classes!(
    ToolTipOpeningTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
    AttachedToVisualTreeTriggerBaseImpl,
);

ferroui_base::ferro_overrides! {
    impl RoutedEventTriggerBaseOfTImpl for ToolTipOpeningTrigger {
        fn routed_event(_this: &Self) -> RoutedEvent {
            ToolTip::tool_tip_opening_event().as_routed_event()
        }
    }
}

impl ToolTipOpeningTrigger {
    /// The static constructor: the event is raised on the trigger's element directly.
    fn static_constructor() {
        RoutedEventTriggerBase::event_routing_strategy_property()
            .override_default_value::<ToolTipOpeningTrigger>(RoutingStrategies::DIRECT);
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
