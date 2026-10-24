//! Port of `Input/SelectAllOnFocusBehavior.cs`.

use ferroui_base::input::{FocusChangedEventArgs, InputElement};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StaticType, WeakRef};
use ferroui_controls::{Control, SelectableTextBlock, TextBox};
use ferroui_xaml_interactivity::{BehaviorImpl, BehaviorImplExt, BehaviorOfT};
use std::cell::RefCell;

/// A behavior that automatically selects all text in a [`TextBox`] when it receives focus.
#[repr(C)]
pub struct SelectAllOnFocusBehavior {
    base: BehaviorOfT,
    // The control the got focus handler is attached to.
    got_focus: RefCell<Option<(WeakRef<Control>, RoutedEventHandlerToken)>>,
}

ferro_class!(SelectAllOnFocusBehavior: BehaviorOfT);
ferro_class_info!(SelectAllOnFocusBehavior { new: SelectAllOnFocusBehavior::new });
ferroui_base::ferro_impl_classes!(SelectAllOnFocusBehavior: FerroObjectImpl);

ferroui_base::ferro_overrides! {
    impl BehaviorImpl for SelectAllOnFocusBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            Self::parent_on_attached_to_visual_tree(this);

            if let Some(associated_object) = this.associated_object_as::<Control>() {
                this.remove_got_focus_handler();
                let behavior = this.to_ref().downgrade();
                let token = associated_object.add_handler(
                    InputElement::got_focus_event(),
                    move |_sender: &Interactive, e: &FocusChangedEventArgs| {
                        if let Some(behavior) = behavior.upgrade() {
                            behavior.associated_object_got_focus(e);
                        }
                    },
                );
                *this.got_focus.borrow_mut() = Some((associated_object.downgrade(), token));
            }
        }

        fn on_detached_from_visual_tree(this: &Self) {
            Self::parent_on_detached_from_visual_tree(this);

            this.remove_got_focus_handler();
        }
    }
}

impl SelectAllOnFocusBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: BehaviorOfT::construct(<Control as StaticType>::TYPE), got_focus: RefCell::new(None) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn remove_got_focus_handler(&self) {
        let got_focus = self.got_focus.borrow_mut().take();
        if let Some((associated_object, token)) = got_focus {
            if let Some(associated_object) = associated_object.upgrade() {
                associated_object.remove_handler(InputElement::got_focus_event(), token);
            }
        }
    }

    fn associated_object_got_focus(&self, _e: &FocusChangedEventArgs) {
        if let Some(text_box) = self.associated_object_as::<TextBox>() {
            text_box.select_all();
        } else if let Some(_selectable_text_block) = self.associated_object_as::<SelectableTextBlock>() {
            // The selectable text block is not supported (as in the reference): only the text
            // box selects its text.
        }
    }
}
