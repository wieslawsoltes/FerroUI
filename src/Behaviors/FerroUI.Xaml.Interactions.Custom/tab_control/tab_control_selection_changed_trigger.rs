//! Port of `TabControl/TabControlSelectionChangedTrigger.cs`.

use crate::{
    AttachedToVisualTreeTriggerBaseImpl, RoutedEventTriggerBase, RoutedEventTriggerBaseOfT,
    RoutedEventTriggerBaseOfTImpl,
};
use ferroui_base::interactivity::{RoutedEvent, RoutingStrategies};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_controls::primitives::SelectingItemsControl;
use ferroui_xaml_interactivity::{DisposingTriggerImpl, StyledElementBehaviorImpl};

/// Executes actions when the associated [`TabControl`](ferroui_controls::TabControl) changes selection.
///
/// The [`SelectionChangedEventArgs`](ferroui_controls::SelectionChangedEventArgs) of the event
/// are the parameter of the actions.
#[repr(C)]
pub struct TabControlSelectionChangedTrigger {
    base: RoutedEventTriggerBaseOfT,
}

ferro_class!(TabControlSelectionChangedTrigger: RoutedEventTriggerBaseOfT);
ferro_class_info!(TabControlSelectionChangedTrigger { new: TabControlSelectionChangedTrigger::new });
ferroui_base::ferro_impl_classes!(
    TabControlSelectionChangedTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
    AttachedToVisualTreeTriggerBaseImpl,
);

ferroui_base::ferro_overrides! {
    impl RoutedEventTriggerBaseOfTImpl for TabControlSelectionChangedTrigger {
        fn routed_event(_this: &Self) -> RoutedEvent {
            SelectingItemsControl::selection_changed_event().as_routed_event()
        }
    }
}

impl TabControlSelectionChangedTrigger {
    /// The static constructor: the default routing strategies of the class.
    fn static_constructor() {
        RoutedEventTriggerBase::event_routing_strategy_property()
            .override_default_value::<TabControlSelectionChangedTrigger>(RoutingStrategies::BUBBLE);
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
