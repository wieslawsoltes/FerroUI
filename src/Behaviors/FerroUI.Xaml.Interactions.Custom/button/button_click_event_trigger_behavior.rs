//! Port of `Button/ButtonClickEventTriggerBehavior.cs`.

use ferroui_base::input::{InputElement, KeyEventArgs, KeyModifiers};
use ferroui_base::interactivity::{
    IRoutedEventArgs, Interactive, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::Button;
use ferroui_xaml_interactivity::{Interaction, StyledElementBehaviorImpl, StyledElementTriggerOfT};
use std::cell::Cell;
use std::rc::Rc;

/// The handlers the trigger adds to its button, removed by their tokens.
#[derive(Clone, Copy)]
struct Handlers {
    click: RoutedEventHandlerToken,
    key_down: RoutedEventHandlerToken,
    key_up: RoutedEventHandlerToken,
}

/// A behavior that listens for a [`Button::click_event`] event on its source and executes its
/// actions when that event is fired.
///
/// The args of the click are the parameter of the actions.
#[repr(C)]
pub struct ButtonClickEventTriggerBehavior {
    base: StyledElementTriggerOfT,
    saved_key_modifiers: Cell<KeyModifiers>,
    // The handlers are removed by their tokens (a closure has no identity to remove it by, as the
    // reference removes its methods).
    handlers: Cell<Option<Handlers>>,
}

ferro_class!(ButtonClickEventTriggerBehavior: StyledElementTriggerOfT);
ferro_class_info!(ButtonClickEventTriggerBehavior { new: ButtonClickEventTriggerBehavior::new });
ferroui_base::ferro_impl_classes!(ButtonClickEventTriggerBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ButtonClickEventTriggerBehavior {
        /// Identifies the `KeyModifiers` property.
        pub fn key_modifiers_property() -> StyledProperty<KeyModifiers> {
            FerroProperty::register::<ButtonClickEventTriggerBehavior, _>("KeyModifiers", KeyModifiers::NONE)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ButtonClickEventTriggerBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Button>() else {
                return;
            };

            this.remove_handlers(&associated_object);

            let trigger = this.to_ref().downgrade();
            let click = associated_object.click(move |_sender: &Interactive, e: &RoutedEventArgs| {
                if let Some(trigger) = trigger.upgrade() {
                    trigger.associated_object_on_click(e);
                }
            });

            let trigger = this.to_ref().downgrade();
            let key_down = associated_object.add_handler_with(
                InputElement::key_down_event(),
                move |_sender: &Interactive, e: &KeyEventArgs| {
                    if let Some(trigger) = trigger.upgrade() {
                        trigger.button_on_key_down(e);
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );

            let trigger = this.to_ref().downgrade();
            let key_up = associated_object.add_handler_with(
                InputElement::key_up_event(),
                move |_sender: &Interactive, e: &KeyEventArgs| {
                    if let Some(trigger) = trigger.upgrade() {
                        trigger.button_on_key_up(e);
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );

            this.handlers.set(Some(Handlers { click, key_down, key_up }));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            if let Some(associated_object) = this.associated_object_as::<Button>() {
                this.remove_handlers(&associated_object);
            }
        }
    }
}

impl ButtonClickEventTriggerBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementTriggerOfT::construct(<Button as StaticType>::TYPE),
            saved_key_modifiers: Cell::new(KeyModifiers::NONE),
            handlers: Cell::new(None),
        }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the required key modifiers to execute [`Button::click_event`] event handler.
    /// This is a property of the object model.
    pub fn key_modifiers(&self) -> KeyModifiers {
        self.get_value(Self::key_modifiers_property())
    }

    pub fn set_key_modifiers(&self, value: KeyModifiers) {
        self.set_value(Self::key_modifiers_property(), value)
    }

    fn remove_handlers(&self, button: &Button) {
        if let Some(handlers) = self.handlers.take() {
            button.remove_handler(Button::click_event(), handlers.click);
            button.remove_handler(InputElement::key_down_event(), handlers.key_down);
            button.remove_handler(InputElement::key_up_event(), handlers.key_up);
        }
    }

    fn associated_object_on_click(&self, e: &RoutedEventArgs) {
        // The args are the parameter of the actions (a shared handle to the live args).
        let parameter: BoxedValue = Rc::new(IRoutedEventArgs::share(e));
        self.execute(Some(&parameter));
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        if !self.is_enabled() {
            return;
        }

        if let Some(associated_object) = self.associated_object() {
            if self.key_modifiers() == self.saved_key_modifiers.get() {
                let sender: BoxedValue = Rc::new(associated_object);
                Interaction::execute_actions(Some(&sender), self.actions().as_ref(), parameter);
            }
        }
    }

    fn button_on_key_down(&self, e: &KeyEventArgs) {
        self.saved_key_modifiers.set(e.key_modifiers);
    }

    fn button_on_key_up(&self, _e: &KeyEventArgs) {
        self.saved_key_modifiers.set(KeyModifiers::NONE);
    }
}
