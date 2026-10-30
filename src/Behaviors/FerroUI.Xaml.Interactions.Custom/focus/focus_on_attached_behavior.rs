//! Port of `Focus/FocusOnAttachedBehavior.cs`.

use crate::{AttachedToVisualTreeBehaviorImpl, FocusBehaviorBase, FocusBehaviorBaseImpl};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Focuses the associated control when it is attached to the visual tree.
#[repr(C)]
pub struct FocusOnAttachedBehavior {
    base: FocusBehaviorBase,
}

ferro_class!(FocusOnAttachedBehavior: FocusBehaviorBase);
ferro_class_info!(FocusOnAttachedBehavior { new: FocusOnAttachedBehavior::new });
ferroui_base::ferro_impl_classes!(
    FocusOnAttachedBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
    FocusBehaviorBaseImpl,
);

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for FocusOnAttachedBehavior {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            this.focus();

            DisposableAction::empty()
        }
    }
}

impl FocusOnAttachedBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FocusBehaviorBase::construct() }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
