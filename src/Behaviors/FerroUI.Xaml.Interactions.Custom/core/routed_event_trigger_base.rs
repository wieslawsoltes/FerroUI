//! Port of `Core/RoutedEventTriggerBase.cs`.

use crate::AttachedToVisualTreeTriggerBase;
use crate::AttachedToVisualTreeTriggerBaseImpl;
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, FerroObjectImpl, FerroProperty, Ref, StaticType,
    StyledElementImpl, StyledProperty, Visual,
};
use ferroui_xaml_interactivity::{DisposingTriggerImpl, StyledElementBehaviorImpl};
use std::cell::Cell;

/// The base class of the triggers of a routed event of the associated visual
/// ([`RoutedEventTrigger`](crate::RoutedEventTrigger) and
/// [`RoutedEventTriggerBaseOfT`](crate::RoutedEventTriggerBaseOfT)): the routing strategies the
/// handler listens to, and whether the actions mark the event as handled.
///
/// The class is abstract.
#[repr(C)]
pub struct RoutedEventTriggerBase {
    base: AttachedToVisualTreeTriggerBase,
    mark_as_handled: Cell<bool>,
}

ferro_class!(RoutedEventTriggerBase: AttachedToVisualTreeTriggerBase);
ferro_class_info!(RoutedEventTriggerBase {
    markup: {
        properties: [
            MarkAsHandled: bool {
                get: |this: &Ref<RoutedEventTriggerBase>| this.mark_as_handled(),
                set: |this: &Ref<RoutedEventTriggerBase>, value: bool| this.set_mark_as_handled(value)
            },
        ],
    },
});
ferroui_base::ferro_impl_classes!(
    RoutedEventTriggerBase: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
    AttachedToVisualTreeTriggerBaseImpl,
);

ferro_properties! {
    impl RoutedEventTriggerBase {
        /// Identifies the `EventRoutingStrategy` property. A deriving class overrides its default
        /// value for its own type in its static constructor (`OverrideMetadata` of the reference:
        /// `event_routing_strategy_property().override_default_value::<T>(..)`).
        pub fn event_routing_strategy_property() -> StyledProperty<RoutingStrategies> {
            FerroProperty::register::<RoutedEventTriggerBase, _>("EventRoutingStrategy", RoutingStrategies::DIRECT)
        }
    }
}

impl RoutedEventTriggerBase {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: AttachedToVisualTreeTriggerBase::construct(<Visual as StaticType>::TYPE), mark_as_handled: Cell::new(false) }
    }

    /// Gets or sets the routing strategies the handler of the event listens to.
    pub fn event_routing_strategy(&self) -> RoutingStrategies {
        self.get_value(Self::event_routing_strategy_property())
    }

    pub fn set_event_routing_strategy(&self, value: RoutingStrategies) {
        self.set_value(Self::event_routing_strategy_property(), value)
    }

    /// Gets or sets the value the handled flag of the event is set to before the actions run.
    /// This is a plain property, not a property of the object model.
    pub fn mark_as_handled(&self) -> bool {
        self.mark_as_handled.get()
    }

    pub fn set_mark_as_handled(&self, value: bool) {
        self.mark_as_handled.set(value);
    }
}
