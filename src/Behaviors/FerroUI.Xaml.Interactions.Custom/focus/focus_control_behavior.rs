//! Port of `Focus/FocusControlBehavior.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Focuses the associated control when [`focus_flag`](Self::focus_flag) becomes true, or when it
/// is attached to the visual tree while the flag is set.
#[repr(C)]
pub struct FocusControlBehavior {
    base: AttachedToVisualTreeBehavior,
}

ferro_class!(FocusControlBehavior: AttachedToVisualTreeBehavior);
ferro_class_info!(FocusControlBehavior { new: FocusControlBehavior::new });
ferroui_base::ferro_impl_classes!(
    FocusControlBehavior: StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferro_properties! {
    impl FocusControlBehavior {
        /// Identifies the `FocusFlag` property.
        pub fn focus_flag_property() -> StyledProperty<bool> {
            FerroProperty::register::<FocusControlBehavior, _>("FocusFlag", false)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for FocusControlBehavior {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            if change.property() == Self::focus_flag_property().as_property() {
                let focus_flag = change.get_new_value::<bool>();
                if focus_flag && this.is_enabled() {
                    this.execute();
                }
            }
        }
    }
}

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for FocusControlBehavior {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            if this.focus_flag() && this.is_enabled() {
                this.execute();
            }

            DisposableAction::empty()
        }
    }
}

impl FocusControlBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: AttachedToVisualTreeBehavior::construct(<Control as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets whether the associated control is focused. This is a property of the object
    /// model.
    pub fn focus_flag(&self) -> bool {
        self.get_value(Self::focus_flag_property())
    }

    pub fn set_focus_flag(&self, value: bool) {
        self.set_value(Self::focus_flag_property(), value)
    }

    fn execute(&self) {
        let behavior = self.to_ref().downgrade();
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
