//! Port of `Show/ShowBehaviorBase.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, ElementRef, FerroObjectImpl, FerroProperty, Nullable, Ref,
    StaticType, StyledElementImpl, StyledProperty, Visual,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{DisposingBehaviorImpl, StyledElementBehaviorImpl};

/// Base class for behaviors that show a control in response to an event.
///
/// The class is abstract: a deriving class implements
/// [`AttachedToVisualTreeBehaviorImpl::on_attached_to_visual_tree_override`] to listen for its
/// event, and calls [`show`](Self::show) when it fires.
#[repr(C)]
pub struct ShowBehaviorBase {
    base: AttachedToVisualTreeBehavior,
}

ferro_class!(ShowBehaviorBase: AttachedToVisualTreeBehavior);
ferro_class_info!(ShowBehaviorBase {
    markup: {
        property_attributes: [
            TargetControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(
    ShowBehaviorBase: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
    AttachedToVisualTreeBehaviorImpl,
);

ferro_properties! {
    impl ShowBehaviorBase {
        /// Identifies the `TargetControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the behavior does not own the control, so the
        /// value is an element reference.
        pub fn target_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<ShowBehaviorBase, _>("TargetControl", None)
        }

        /// Identifies the `EventRoutingStrategy` property: the routing strategy used for the
        /// triggering event.
        pub fn event_routing_strategy_property() -> StyledProperty<RoutingStrategies> {
            FerroProperty::register::<ShowBehaviorBase, _>("EventRoutingStrategy", RoutingStrategies::BUBBLE)
        }
    }
}

impl ShowBehaviorBase {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: AttachedToVisualTreeBehavior::construct(<Control as StaticType>::TYPE) }
    }

    /// Gets or sets the target control. Markup resolves a name given for it to the named element.
    /// This is a property of the object model.
    pub fn target_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::target_control_property()))
    }

    pub fn set_target_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::target_control_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets or sets the routing strategy used for the triggering event.
    pub fn event_routing_strategy(&self) -> RoutingStrategies {
        self.get_value(Self::event_routing_strategy_property())
    }

    pub fn set_event_routing_strategy(&self, value: RoutingStrategies) {
        self.set_value(Self::event_routing_strategy_property(), value)
    }

    /// Shows the [`target_control`](Self::target_control) when the behavior is triggered.
    ///
    /// Returns true if the control was shown; otherwise, false.
    pub fn show(&self) -> bool {
        if !self.is_enabled() {
            return false;
        }

        let Some(target_control) = self.target_control().filter(|target_control| !target_control.is_visible())
        else {
            return false;
        };

        target_control.set_current_value(Visual::is_visible_property(), true);

        Dispatcher::ui_thread().post_local(
            move || {
                target_control.focus();
            },
            DispatcherPriority::DEFAULT,
        );

        true
    }
}
