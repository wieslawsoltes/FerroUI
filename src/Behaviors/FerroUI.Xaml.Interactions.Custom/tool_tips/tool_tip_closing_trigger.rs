//! Port of `ToolTips/ToolTipClosingTrigger.cs`.

use crate::{
    AttachedToVisualTreeTriggerBaseImpl, RoutedEventTrigger, RoutedEventTriggerBase, RoutedEventTriggerImpl,
};
use ferroui_base::interactivity::{RoutedEvent, RoutingStrategies};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_controls::ToolTip;
use ferroui_xaml_interactivity::{DisposingTriggerImpl, StyledElementBehaviorImpl};

/// Trigger that listens for the [`ToolTip::tool_tip_closing_event`].
///
/// The args of the event are the parameter of the actions.
#[repr(C)]
pub struct ToolTipClosingTrigger {
    base: RoutedEventTrigger,
}

ferro_class!(ToolTipClosingTrigger: RoutedEventTrigger);
ferro_class_info!(ToolTipClosingTrigger { new: ToolTipClosingTrigger::new });
ferroui_base::ferro_impl_classes!(
    ToolTipClosingTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
    AttachedToVisualTreeTriggerBaseImpl,
);

ferroui_base::ferro_overrides! {
    impl RoutedEventTriggerImpl for ToolTipClosingTrigger {
        fn routed_event(_this: &Self) -> RoutedEvent {
            ToolTip::tool_tip_closing_event().as_routed_event()
        }
    }
}

impl ToolTipClosingTrigger {
    /// The static constructor: the event is raised on the trigger's element directly.
    fn static_constructor() {
        RoutedEventTriggerBase::event_routing_strategy_property()
            .override_default_value::<ToolTipClosingTrigger>(RoutingStrategies::DIRECT);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: RoutedEventTrigger::construct() }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
