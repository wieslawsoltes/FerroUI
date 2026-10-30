//! Port of `Behaviors/InteractiveBehaviorBase.cs`.

use crate::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use ferroui_base::interactivity::{Interactive, RoutingStrategies};
use ferroui_base::{ferro_class, ferro_properties, FerroObjectImpl, FerroProperty, StaticType, StyledElementImpl, StyledProperty};

/// Base class for behaviors that listen for routed events (a behavior attached to an
/// [`Interactive`]).
#[repr(C)]
pub struct InteractiveBehaviorBase {
    base: StyledElementBehaviorOfT,
}

ferro_class!(InteractiveBehaviorBase: StyledElementBehaviorOfT);
ferroui_base::ferro_impl_classes!(InteractiveBehaviorBase: FerroObjectImpl, StyledElementImpl, StyledElementBehaviorImpl);

ferro_properties! {
    impl InteractiveBehaviorBase {
        /// Identifies the `RoutingStrategies` property.
        pub fn routing_strategies_property() -> StyledProperty<RoutingStrategies> {
            FerroProperty::register::<InteractiveBehaviorBase, _>("RoutingStrategies", RoutingStrategies::BUBBLE)
        }
    }
}

impl InteractiveBehaviorBase {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementBehaviorOfT::construct(<Interactive as StaticType>::TYPE) }
    }

    /// Gets or sets the routing strategies used when subscribing to events.
    pub fn routing_strategies(&self) -> RoutingStrategies {
        self.get_value(Self::routing_strategies_property())
    }

    pub fn set_routing_strategies(&self, value: RoutingStrategies) {
        self.set_value(Self::routing_strategies_property(), value)
    }
}
