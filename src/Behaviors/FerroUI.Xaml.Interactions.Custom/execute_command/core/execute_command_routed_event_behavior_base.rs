//! Port of `ExecuteCommand/Core/ExecuteCommandRoutedEventBehaviorBase.cs`.

use crate::{AttachedToVisualTreeBehaviorImpl, ExecuteCommandBehaviorBase, ExecuteCommandBehaviorBaseImpl};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, FerroObjectImpl, FerroProperty, Ref, StyledElementImpl,
    StyledProperty,
};
use ferroui_xaml_interactivity::{DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::cell::Cell;

/// The base class of the behaviors that execute a command on a routed event: the routing
/// strategies the handler listens to, and whether the event is marked as handled when the command
/// runs.
///
/// The class is abstract.
#[repr(C)]
pub struct ExecuteCommandRoutedEventBehaviorBase {
    base: ExecuteCommandBehaviorBase,
    mark_as_handled: Cell<bool>,
}

ferro_class!(ExecuteCommandRoutedEventBehaviorBase: ExecuteCommandBehaviorBase);
ferro_class_info!(ExecuteCommandRoutedEventBehaviorBase {
    markup: {
        properties: [
            MarkAsHandled: bool {
                get: |this: &Ref<ExecuteCommandRoutedEventBehaviorBase>| this.mark_as_handled(),
                set: |this: &Ref<ExecuteCommandRoutedEventBehaviorBase>, value: bool| this.set_mark_as_handled(value)
            },
        ],
    },
});
ferroui_base::ferro_impl_classes!(
    ExecuteCommandRoutedEventBehaviorBase: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
    AttachedToVisualTreeBehaviorImpl,
    ExecuteCommandBehaviorBaseImpl,
);

ferro_properties! {
    impl ExecuteCommandRoutedEventBehaviorBase {
        /// Identifies the `EventRoutingStrategy` property.
        pub fn event_routing_strategy_property() -> StyledProperty<RoutingStrategies> {
            FerroProperty::register::<ExecuteCommandRoutedEventBehaviorBase, _>(
                "EventRoutingStrategy",
                RoutingStrategies::BUBBLE,
            )
        }
    }
}

impl ExecuteCommandRoutedEventBehaviorBase {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ExecuteCommandBehaviorBase::construct(), mark_as_handled: Cell::new(true) }
    }

    /// Gets or sets the routing strategies the handler of the event listens to.
    pub fn event_routing_strategy(&self) -> RoutingStrategies {
        self.get_value(Self::event_routing_strategy_property())
    }

    pub fn set_event_routing_strategy(&self, value: RoutingStrategies) {
        self.set_value(Self::event_routing_strategy_property(), value)
    }

    /// Gets or sets the value the handled flag of the event is set to when the command runs (true
    /// by default). This is a plain property, not a property of the object model.
    pub fn mark_as_handled(&self) -> bool {
        self.mark_as_handled.get()
    }

    pub fn set_mark_as_handled(&self, value: bool) {
        self.mark_as_handled.set(value);
    }
}
