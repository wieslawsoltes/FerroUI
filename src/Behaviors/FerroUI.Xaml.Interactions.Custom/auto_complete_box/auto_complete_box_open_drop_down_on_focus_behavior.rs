//! Port of `AutoCompleteBox/AutoCompleteBoxOpenDropDownOnFocusBehavior.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::input::{FocusChangedEventArgs, InputElement};
use ferroui_base::interactivity::Interactive;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StaticType, StyledElementImpl};
use ferroui_controls::AutoCompleteBox;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Opens the drop-down of an [`AutoCompleteBox`] when it receives focus.
#[repr(C)]
pub struct AutoCompleteBoxOpenDropDownOnFocusBehavior {
    base: AttachedToVisualTreeBehavior,
}

ferro_class!(AutoCompleteBoxOpenDropDownOnFocusBehavior: AttachedToVisualTreeBehavior);
ferro_class_info!(AutoCompleteBoxOpenDropDownOnFocusBehavior { new: AutoCompleteBoxOpenDropDownOnFocusBehavior::new });
ferroui_base::ferro_impl_classes!(
    AutoCompleteBoxOpenDropDownOnFocusBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for AutoCompleteBoxOpenDropDownOnFocusBehavior {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let Some(associated_object) = this.associated_object_as::<AutoCompleteBox>() else {
                return DisposableAction::empty();
            };

            let behavior = this.to_ref().downgrade();
            let handler = associated_object.add_handler(
                InputElement::got_focus_event(),
                move |_sender: &Interactive, _e: &FocusChangedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.associated_object_on_got_focus();
                    }
                },
            );

            let associated_object = associated_object.downgrade();
            DisposableAction::create(move || {
                if let Some(associated_object) = associated_object.upgrade() {
                    associated_object.remove_handler(InputElement::got_focus_event(), handler);
                }
            })
        }
    }
}

impl AutoCompleteBoxOpenDropDownOnFocusBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: AttachedToVisualTreeBehavior::construct(<AutoCompleteBox as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn associated_object_on_got_focus(&self) {
        let Some(associated_object) = self.associated_object_as::<AutoCompleteBox>() else {
            return;
        };

        Dispatcher::ui_thread().post_local(
            move || {
                associated_object.set_current_value(AutoCompleteBox::is_drop_down_open_property(), true);
            },
            DispatcherPriority::DEFAULT,
        );
    }
}
