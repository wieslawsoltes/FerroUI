//! Port of `Focus/FocusOnAttachedToVisualTreeBehavior.cs`.

use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StaticType, StyledElementImpl};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};

/// Focuses the associated control when it is attached to the visual tree.
#[repr(C)]
pub struct FocusOnAttachedToVisualTreeBehavior {
    base: StyledElementBehaviorOfT,
}

ferro_class!(FocusOnAttachedToVisualTreeBehavior: StyledElementBehaviorOfT);
ferro_class_info!(FocusOnAttachedToVisualTreeBehavior { new: FocusOnAttachedToVisualTreeBehavior::new });
ferroui_base::ferro_impl_classes!(FocusOnAttachedToVisualTreeBehavior: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for FocusOnAttachedToVisualTreeBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let behavior = this.to_ref().downgrade();
            Dispatcher::ui_thread().post_local(
                move || {
                    if let Some(associated_object) =
                        behavior.upgrade().and_then(|behavior| behavior.associated_object_as::<Control>())
                    {
                        associated_object.focus();
                    }
                },
                DispatcherPriority::DEFAULT,
            );
        }
    }
}

impl FocusOnAttachedToVisualTreeBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
