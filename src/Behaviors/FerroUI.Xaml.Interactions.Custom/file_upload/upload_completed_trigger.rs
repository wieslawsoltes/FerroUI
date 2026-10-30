//! Port of `FileUpload/UploadCompletedTrigger.cs`.

use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{Interaction, StyledElementBehaviorImpl, StyledElementTrigger};
use std::rc::Rc;

/// Executes actions when the bound value becomes `true`.
#[repr(C)]
pub struct UploadCompletedTrigger {
    base: StyledElementTrigger,
}

ferro_class!(UploadCompletedTrigger: StyledElementTrigger);
ferro_class_info!(UploadCompletedTrigger { new: UploadCompletedTrigger::new });
ferroui_base::ferro_impl_classes!(UploadCompletedTrigger: StyledElementImpl, StyledElementBehaviorImpl);

ferro_properties! {
    impl UploadCompletedTrigger {
        /// Identifies the `IsCompleted` property.
        pub fn is_completed_property() -> StyledProperty<bool> {
            FerroProperty::register::<UploadCompletedTrigger, _>("IsCompleted", false)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for UploadCompletedTrigger {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            if change.property() == Self::is_completed_property().as_property() {
                this.on_is_completed_changed(change);
            }
        }
    }
}

impl UploadCompletedTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTrigger::construct() }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets a value indicating upload completion. This is a property of the object model.
    pub fn is_completed(&self) -> bool {
        self.get_value(Self::is_completed_property())
    }

    pub fn set_is_completed(&self, value: bool) {
        self.set_value(Self::is_completed_property(), value)
    }

    fn on_is_completed_changed(&self, args: &FerroPropertyChangedEventArgs<'_>) {
        if args.get_new_value::<bool>() {
            let trigger: Ref<UploadCompletedTrigger> = self.to_ref();
            Dispatcher::ui_thread().post_local(move || trigger.execute(), DispatcherPriority::DEFAULT);
        }
    }

    fn execute(&self) {
        if !self.is_enabled() {
            return;
        }
        let Some(associated_object) = self.associated_object() else {
            return;
        };

        let sender: BoxedValue = Rc::new(associated_object);
        Interaction::execute_actions(Some(&sender), self.actions().as_ref(), None);
    }
}
