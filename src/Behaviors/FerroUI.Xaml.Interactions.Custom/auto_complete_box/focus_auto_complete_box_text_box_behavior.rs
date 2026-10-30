//! Port of `AutoCompleteBox/FocusAutoCompleteBoxTextBoxBehavior.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::input::{FocusChangedEventArgs, InputElement};
use ferroui_base::interactivity::Interactive;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StaticType, StyledElementImpl};
use ferroui_controls::{AutoCompleteBox, TextBox};
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Focuses the text box of the template of an [`AutoCompleteBox`] when the box receives focus.
#[repr(C)]
pub struct FocusAutoCompleteBoxTextBoxBehavior {
    base: AttachedToVisualTreeBehavior,
}

ferro_class!(FocusAutoCompleteBoxTextBoxBehavior: AttachedToVisualTreeBehavior);
ferro_class_info!(FocusAutoCompleteBoxTextBoxBehavior { new: FocusAutoCompleteBoxTextBoxBehavior::new });
ferroui_base::ferro_impl_classes!(
    FocusAutoCompleteBoxTextBoxBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for FocusAutoCompleteBoxTextBoxBehavior {
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

impl FocusAutoCompleteBoxTextBoxBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: AttachedToVisualTreeBehavior::construct(<AutoCompleteBox as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn associated_object_on_got_focus(&self) {
        let text_box = self.associated_object_as::<AutoCompleteBox>().and_then(|associated_object| {
            associated_object.get_visual_descendants().find_map(|visual| visual.cast::<TextBox>())
        });

        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(text_box) = text_box {
                    text_box.focus();
                }
            },
            DispatcherPriority::DEFAULT,
        );
    }
}
