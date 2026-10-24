//! Port of `AutoCompleteBox/AutoCompleteBoxSelectionChangedTrigger.cs`.

use crate::{
    AttachedToVisualTreeTriggerBaseImpl, RoutedEventTriggerBase, RoutedEventTriggerBaseOfT,
    RoutedEventTriggerBaseOfTImpl,
};
use ferroui_base::interactivity::{RoutedEvent, RoutingStrategies};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_controls::AutoCompleteBox;
use ferroui_xaml_interactivity::{DisposingTriggerImpl, StyledElementBehaviorImpl};

/// Triggers actions when the [`AutoCompleteBox`] selection changes.
///
/// The [`SelectionChangedEventArgs`](ferroui_controls::SelectionChangedEventArgs) of the event
/// are the parameter of the actions.
#[repr(C)]
pub struct AutoCompleteBoxSelectionChangedTrigger {
    base: RoutedEventTriggerBaseOfT,
}

ferro_class!(AutoCompleteBoxSelectionChangedTrigger: RoutedEventTriggerBaseOfT);
ferro_class_info!(AutoCompleteBoxSelectionChangedTrigger { new: AutoCompleteBoxSelectionChangedTrigger::new });
ferroui_base::ferro_impl_classes!(
    AutoCompleteBoxSelectionChangedTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
    AttachedToVisualTreeTriggerBaseImpl,
);

ferroui_base::ferro_overrides! {
    impl RoutedEventTriggerBaseOfTImpl for AutoCompleteBoxSelectionChangedTrigger {
        fn routed_event(_this: &Self) -> RoutedEvent {
            AutoCompleteBox::selection_changed_event().as_routed_event()
        }
    }
}

impl AutoCompleteBoxSelectionChangedTrigger {
    /// The static constructor: the default routing strategies of the class.
    fn static_constructor() {
        RoutedEventTriggerBase::event_routing_strategy_property()
            .override_default_value::<AutoCompleteBoxSelectionChangedTrigger>(RoutingStrategies::BUBBLE);
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
