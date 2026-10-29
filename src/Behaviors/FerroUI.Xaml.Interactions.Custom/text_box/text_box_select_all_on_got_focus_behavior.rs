//! Port of `TextBox/TextBoxSelectAllOnGotFocusBehavior.cs`.

use ferroui_base::input::{FocusChangedEventArgs, InputElement};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StaticType, StyledElementImpl, WeakRef,
};
use ferroui_controls::TextBox;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::cell::RefCell;

/// A behavior that allows to select all [`TextBox`] text on got focus event.
#[repr(C)]
pub struct TextBoxSelectAllOnGotFocusBehavior {
    base: StyledElementBehaviorOfT,
    // The handler of the got focus event (`RemoveHandler` of the reference).
    got_focus: RefCell<Option<(WeakRef<TextBox>, RoutedEventHandlerToken)>>,
}

ferro_class!(TextBoxSelectAllOnGotFocusBehavior: StyledElementBehaviorOfT);
ferro_class_info!(TextBoxSelectAllOnGotFocusBehavior { new: TextBoxSelectAllOnGotFocusBehavior::new });
ferroui_base::ferro_impl_classes!(TextBoxSelectAllOnGotFocusBehavior: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for TextBoxSelectAllOnGotFocusBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            if let Some(associated_object) = this.associated_object_as::<TextBox>() {
                this.remove_handler();
                let behavior = this.to_ref().downgrade();
                let token = associated_object.add_handler_with(
                    InputElement::got_focus_event(),
                    move |_sender: &Interactive, e: &FocusChangedEventArgs| {
                        if let Some(behavior) = behavior.upgrade() {
                            behavior.associated_object_got_focus(e);
                        }
                    },
                    RoutingStrategies::BUBBLE,
                    false,
                );
                *this.got_focus.borrow_mut() = Some((associated_object.downgrade(), token));
            }
        }

        fn on_detached_from_visual_tree(this: &Self) {
            this.remove_handler();
        }
    }
}

impl TextBoxSelectAllOnGotFocusBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementBehaviorOfT::construct(<TextBox as StaticType>::TYPE), got_focus: RefCell::new(None) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn remove_handler(&self) {
        let handler = self.got_focus.borrow_mut().take();
        if let Some((text_box, token)) = handler {
            if let Some(text_box) = text_box.upgrade() {
                text_box.remove_handler(InputElement::got_focus_event(), token);
            }
        }
    }

    fn associated_object_got_focus(&self, _e: &FocusChangedEventArgs) {
        if let Some(text_box) = self.associated_object_as::<TextBox>() {
            text_box.select_all();
        }
    }
}
