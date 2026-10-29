//! Port of `ListBox/ListBoxUnselectAllBehavior.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StaticType, StyledElementImpl};
use ferroui_controls::ListBox;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Unselects all the items of the associated [`ListBox`] when it is attached to the visual tree.
#[repr(C)]
pub struct ListBoxUnselectAllBehavior {
    base: AttachedToVisualTreeBehavior,
}

ferro_class!(ListBoxUnselectAllBehavior: AttachedToVisualTreeBehavior);
ferro_class_info!(ListBoxUnselectAllBehavior { new: ListBoxUnselectAllBehavior::new });
ferroui_base::ferro_impl_classes!(
    ListBoxUnselectAllBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for ListBoxUnselectAllBehavior {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            if let Some(associated_object) = this.associated_object_as::<ListBox>() {
                associated_object.unselect_all();
            }

            DisposableAction::empty()
        }
    }
}

impl ListBoxUnselectAllBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: AttachedToVisualTreeBehavior::construct(<ListBox as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
