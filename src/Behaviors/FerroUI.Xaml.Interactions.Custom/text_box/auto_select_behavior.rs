//! Port of `TextBox/AutoSelectBehavior.cs`.

use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StaticType, StyledElementImpl};
use ferroui_controls::TextBox;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementBehaviorOfT};

/// Automatically selects the entire content of the associated [`TextBox`] when it is loaded.
#[repr(C)]
pub struct AutoSelectBehavior {
    base: StyledElementBehaviorOfT,
}

ferro_class!(AutoSelectBehavior: StyledElementBehaviorOfT);
ferro_class_info!(AutoSelectBehavior { new: AutoSelectBehavior::new });
ferroui_base::ferro_impl_classes!(AutoSelectBehavior: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for AutoSelectBehavior {
        fn on_loaded(this: &Self) {
            Self::parent_on_loaded(this);

            let behavior = this.to_ref().downgrade();
            Dispatcher::ui_thread().post_local(
                move || {
                    if let Some(text_box) =
                        behavior.upgrade().and_then(|behavior| behavior.associated_object_as::<TextBox>())
                    {
                        text_box.select_all();
                    }
                },
                DispatcherPriority::DEFAULT,
            );
        }
    }
}

impl AutoSelectBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementBehaviorOfT::construct(<TextBox as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
