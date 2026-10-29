//! Port of `InputElement/Triggers/TextInputTrigger.cs`.

use crate::{
    AttachedToVisualTreeTriggerBaseImpl, RoutedEventTriggerBase, RoutedEventTriggerBaseOfT,
    RoutedEventTriggerBaseOfTImpl,
};
use ferroui_base::input::{InputElement, TextInputEventArgs};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEvent, RoutingStrategies};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{DisposingTriggerImpl, StyledElementBehaviorImpl};

/// A trigger of [`InputElement::text_input_event`]: runs the actions with the text input args as
/// the parameter when the text is input (any text when it is not set).
#[repr(C)]
pub struct TextInputTrigger {
    base: RoutedEventTriggerBaseOfT,
}

ferro_class!(TextInputTrigger: RoutedEventTriggerBaseOfT);
ferro_class_info!(TextInputTrigger { new: TextInputTrigger::new });
ferroui_base::ferro_impl_classes!(
    TextInputTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
    AttachedToVisualTreeTriggerBaseImpl,
);

ferro_properties! {
    impl TextInputTrigger {
        /// Identifies the `Text` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the reference registers this property with
        /// `KeyDownTrigger` as its owner, so markup does not find it on this trigger; here it is
        /// registered on its own class (defect corrected).
        pub fn text_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<TextInputTrigger, _>("Text", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl RoutedEventTriggerBaseOfTImpl for TextInputTrigger {
        fn routed_event(_this: &Self) -> RoutedEvent {
            InputElement::text_input_event().as_routed_event()
        }

        fn handler(this: &Self, _sender: &Interactive, e: &dyn IRoutedEventArgs) {
            if !this.is_enabled() {
                return;
            }

            let Some(text_input_event) = e.downcast_ref::<TextInputEventArgs>() else {
                return;
            };

            let is_text_set = this.is_set(Self::text_property());
            let text = this.text();
            let have_text = is_text_set && text_input_event.text == text;

            if !is_text_set || have_text {
                this.execute(e);
            }
        }
    }
}

impl TextInputTrigger {
    /// The static constructor: the default routing strategies of the class.
    fn static_constructor() {
        RoutedEventTriggerBase::event_routing_strategy_property()
            .override_default_value::<TextInputTrigger>(RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: RoutedEventTriggerBaseOfT::construct() }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the text that fires the trigger.
    pub fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }

    pub fn set_text(&self, value: Option<String>) {
        self.set_value(Self::text_property(), value)
    }
}
