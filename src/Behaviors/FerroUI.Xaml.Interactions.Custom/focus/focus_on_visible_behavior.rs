//! Port of `Focus/FocusOnVisibleBehavior.cs`.

use crate::{AttachedToVisualTreeBehaviorImpl, FocusBehaviorBase, FocusBehaviorBaseImpl};
use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, FerroObject, FerroObjectExtensions, FerroObjectImpl, Ref,
    StyledElementImpl, Visual,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Focuses the associated control when it is attached to the visual tree while visible, and each
/// time it becomes visible.
#[repr(C)]
pub struct FocusOnVisibleBehavior {
    base: FocusBehaviorBase,
}

ferro_class!(FocusOnVisibleBehavior: FocusBehaviorBase);
ferro_class_info!(FocusOnVisibleBehavior { new: FocusOnVisibleBehavior::new });
ferroui_base::ferro_impl_classes!(
    FocusOnVisibleBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
    FocusBehaviorBaseImpl,
);

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for FocusOnVisibleBehavior {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return DisposableAction::empty();
            };

            if associated_object.is_visible() {
                this.focus();
            }

            let behavior = this.to_ref().downgrade();
            let source: &FerroObject = &associated_object;
            FerroObjectExtensions::get_observable(source, Visual::is_visible_property()).subscribe_fn(
                move |visible: bool| {
                    if visible {
                        if let Some(behavior) = behavior.upgrade() {
                            behavior.focus();
                        }
                    }
                },
            )
        }
    }
}

impl FocusOnVisibleBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FocusBehaviorBase::construct() }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
