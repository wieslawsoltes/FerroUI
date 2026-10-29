//! Port of `Input/LoseFocusOnEnterBehavior.cs`.

use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers, NavigationMethod};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken};
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StaticType, StyledElementImpl, Visual, WeakRef,
};
use ferroui_controls::{Control, TopLevel};
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementBehaviorOfT};
use std::cell::RefCell;

/// A behavior that loses focus when the Enter key is pressed.
#[repr(C)]
pub struct LoseFocusOnEnterBehavior {
    base: StyledElementBehaviorOfT,
    // The control the key down handler is attached to.
    key_down: RefCell<Option<(WeakRef<Control>, RoutedEventHandlerToken)>>,
}

ferro_class!(LoseFocusOnEnterBehavior: StyledElementBehaviorOfT);
ferro_class_info!(LoseFocusOnEnterBehavior { new: LoseFocusOnEnterBehavior::new });
ferroui_base::ferro_impl_classes!(LoseFocusOnEnterBehavior: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for LoseFocusOnEnterBehavior {
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);

            if let Some(associated_object) = this.associated_object_as::<Control>() {
                let behavior = this.to_ref().downgrade();
                let token = associated_object.add_handler(
                    InputElement::key_down_event(),
                    move |_sender: &Interactive, e: &KeyEventArgs| {
                        if let Some(behavior) = behavior.upgrade() {
                            behavior.on_key_down(e);
                        }
                    },
                );
                *this.key_down.borrow_mut() = Some((associated_object.downgrade(), token));
            }
        }

        fn on_detaching(this: &Self) {
            Self::parent_on_detaching(this);

            let key_down = this.key_down.borrow_mut().take();
            if let Some((associated_object, token)) = key_down {
                if let Some(associated_object) = associated_object.upgrade() {
                    associated_object.remove_handler(InputElement::key_down_event(), token);
                }
            }
        }
    }
}

impl LoseFocusOnEnterBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE), key_down: RefCell::new(None) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn on_key_down(&self, e: &KeyEventArgs) {
        if e.key == Key::Enter {
            let top_level = self.associated_object_as::<Control>().and_then(|control| {
                let visual: &Visual = &control;
                TopLevel::get_top_level(Some(visual))
            });
            if let Some(top_level) = top_level {
                top_level.focus_manager().focus(None, NavigationMethod::Unspecified, KeyModifiers::NONE);
            }
        }
    }
}
