//! Port of `TextBox/TextBoxSelectAllTextBehavior.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StaticType, StyledElementImpl};
use ferroui_controls::TextBox;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Selects all the text of the associated [`TextBox`] when it is attached to the visual tree.
#[repr(C)]
pub struct TextBoxSelectAllTextBehavior {
    base: AttachedToVisualTreeBehavior,
}

ferro_class!(TextBoxSelectAllTextBehavior: AttachedToVisualTreeBehavior);
ferro_class_info!(TextBoxSelectAllTextBehavior { new: TextBoxSelectAllTextBehavior::new });
ferroui_base::ferro_impl_classes!(
    TextBoxSelectAllTextBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for TextBoxSelectAllTextBehavior {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            if let Some(associated_object) = this.associated_object_as::<TextBox>() {
                associated_object.select_all();
            }

            DisposableAction::empty()
        }
    }
}

impl TextBoxSelectAllTextBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: AttachedToVisualTreeBehavior::construct(<TextBox as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
