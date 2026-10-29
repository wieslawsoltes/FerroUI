//! Port of `Triggers/InteractiveTriggerBase.cs`.

use crate::{Interaction, StyledElementBehaviorImpl, StyledElementTriggerOfT};
use ferroui_base::interactivity::{Interactive, RoutingStrategies};
use ferroui_base::{ferro_class, ferro_properties, BoxedValue, FerroObjectImpl, FerroProperty, StyledElementImpl, StyledProperty};
use std::rc::Rc;

/// Base class for triggers that listen for routed events (a trigger attached to an
/// [`Interactive`]).
#[repr(C)]
pub struct InteractiveTriggerBase {
    base: StyledElementTriggerOfT,
}

ferro_class!(InteractiveTriggerBase: StyledElementTriggerOfT);
ferroui_base::ferro_impl_classes!(InteractiveTriggerBase: FerroObjectImpl, StyledElementImpl, StyledElementBehaviorImpl);

ferro_properties! {
    impl InteractiveTriggerBase {
        /// Identifies the `RoutingStrategies` property.
        pub fn routing_strategies_property() -> StyledProperty<RoutingStrategies> {
            FerroProperty::register::<InteractiveTriggerBase, _>("RoutingStrategies", RoutingStrategies::BUBBLE)
        }
    }
}

impl InteractiveTriggerBase {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTriggerOfT::construct(<Interactive as ferroui_base::StaticType>::TYPE) }
    }

    /// Gets or sets the routing strategies used when subscribing to events.
    pub fn routing_strategies(&self) -> RoutingStrategies {
        self.get_value(Self::routing_strategies_property())
    }

    pub fn set_routing_strategies(&self, value: RoutingStrategies) {
        self.set_value(Self::routing_strategies_property(), value)
    }

    /// Executes the actions associated with this trigger, with the event args as the parameter.
    pub fn execute(&self, parameter: Option<&BoxedValue>) {
        if !self.is_enabled() {
            return;
        }

        let sender = self.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
        Interaction::execute_actions(sender.as_ref(), self.actions().as_ref(), parameter);
    }
}
