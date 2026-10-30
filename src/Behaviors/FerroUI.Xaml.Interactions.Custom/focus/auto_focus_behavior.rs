//! Port of `Focus/AutoFocusBehavior.cs`.

use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StaticType, StyledElementImpl};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementBehaviorOfT};

/// Sets the keyboard focus on the associated control when it is loaded.
#[repr(C)]
pub struct AutoFocusBehavior {
    base: StyledElementBehaviorOfT,
}

ferro_class!(AutoFocusBehavior: StyledElementBehaviorOfT);
ferro_class_info!(AutoFocusBehavior { new: AutoFocusBehavior::new });
ferroui_base::ferro_impl_classes!(AutoFocusBehavior: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for AutoFocusBehavior {
        fn on_loaded(this: &Self) {
            Self::parent_on_loaded(this);

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

impl AutoFocusBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
