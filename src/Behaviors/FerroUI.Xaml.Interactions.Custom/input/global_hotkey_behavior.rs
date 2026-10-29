//! Port of `Input/GlobalHotkeyBehavior.cs`.

use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StaticType, StyledElementImpl, StyledProperty, Visual, WeakRef,
};
use ferroui_controls::{Control, TopLevel};
use ferroui_xaml_interactivity::{Interaction, StyledElementBehaviorImpl, StyledElementTriggerOfT};
use std::cell::RefCell;
use std::rc::Rc;

/// A behavior that registers a global hotkey (window-wide) to execute actions: a key down event
/// anywhere in the top level of the control (on the tunnel route) with the key and the key
/// modifiers runs the actions, with the event args as the parameter, and is marked as handled.
#[repr(C)]
pub struct GlobalHotkeyBehavior {
    base: StyledElementTriggerOfT,
    // The top level and the handler of its key down event.
    top_level: RefCell<Option<(WeakRef<TopLevel>, RoutedEventHandlerToken)>>,
}

ferro_class!(GlobalHotkeyBehavior: StyledElementTriggerOfT);
ferro_class_info!(GlobalHotkeyBehavior { new: GlobalHotkeyBehavior::new });
ferroui_base::ferro_impl_classes!(GlobalHotkeyBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl GlobalHotkeyBehavior {
        /// Identifies the `Key` property.
        pub fn key_property() -> StyledProperty<Key> {
            FerroProperty::register::<GlobalHotkeyBehavior, _>("Key", Key::None)
        }

        /// Identifies the `KeyModifiers` property.
        pub fn key_modifiers_property() -> StyledProperty<KeyModifiers> {
            FerroProperty::register::<GlobalHotkeyBehavior, _>("KeyModifiers", KeyModifiers::NONE)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for GlobalHotkeyBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            this.remove_key_down_handler();

            let top_level = this.associated_object_as::<Control>().and_then(|control| {
                let visual: &Visual = &control;
                TopLevel::get_top_level(Some(visual))
            });
            if let Some(top_level) = top_level {
                let behavior = this.to_ref().downgrade();
                let token = top_level.add_handler_with(
                    InputElement::key_down_event(),
                    move |_sender: &Interactive, e: &KeyEventArgs| {
                        if let Some(behavior) = behavior.upgrade() {
                            behavior.on_key_down(e);
                        }
                    },
                    RoutingStrategies::TUNNEL,
                    false,
                );
                *this.top_level.borrow_mut() = Some((top_level.downgrade(), token));
            }
        }

        fn on_detached_from_visual_tree(this: &Self) {
            this.remove_key_down_handler();
        }
    }
}

impl GlobalHotkeyBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTriggerOfT::construct(<Control as StaticType>::TYPE), top_level: RefCell::new(None) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the key to listen for.
    pub fn key(&self) -> Key {
        self.get_value(Self::key_property())
    }

    pub fn set_key(&self, value: Key) {
        self.set_value(Self::key_property(), value)
    }

    /// Gets or sets the key modifiers to listen for.
    pub fn key_modifiers(&self) -> KeyModifiers {
        self.get_value(Self::key_modifiers_property())
    }

    pub fn set_key_modifiers(&self, value: KeyModifiers) {
        self.set_value(Self::key_modifiers_property(), value)
    }

    fn remove_key_down_handler(&self) {
        let top_level = self.top_level.borrow_mut().take();
        if let Some((top_level, token)) = top_level {
            if let Some(top_level) = top_level.upgrade() {
                top_level.remove_handler(InputElement::key_down_event(), token);
            }
        }
    }

    fn on_key_down(&self, e: &KeyEventArgs) {
        if e.key == self.key() && e.key_modifiers == self.key_modifiers() {
            let sender = self.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
            let parameter: BoxedValue = Rc::new(IRoutedEventArgs::share(e));
            Interaction::execute_actions(sender.as_ref(), self.actions().as_ref(), Some(&parameter));
            e.set_handled(true);
        }
    }
}
