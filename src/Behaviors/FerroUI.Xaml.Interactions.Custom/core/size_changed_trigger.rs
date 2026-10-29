//! Port of `Core/SizeChangedTrigger.cs`.

use ferroui_base::interactivity::{IRoutedEventArgs, Interactive};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, Ref, StaticType, StyledElementImpl,
};
use ferroui_controls::{Control, SizeChangedEventArgs};
use ferroui_xaml_interactivity::{
    DisposableAction, DisposingTrigger, DisposingTriggerImpl, Interaction, StyledElementBehaviorImpl,
};
use std::rc::Rc;

/// A behavior that triggers its actions whenever the associated control size changes.
///
/// The args of the size change are the parameter of the actions.
#[repr(C)]
pub struct SizeChangedTrigger {
    base: DisposingTrigger,
}

ferro_class!(SizeChangedTrigger: DisposingTrigger);
ferro_class_info!(SizeChangedTrigger { new: SizeChangedTrigger::new });
ferroui_base::ferro_impl_classes!(SizeChangedTrigger: FerroObjectImpl, StyledElementImpl, StyledElementBehaviorImpl);

ferroui_base::ferro_overrides! {
    impl DisposingTriggerImpl for SizeChangedTrigger {
        fn on_attached_override(this: &Self) -> Rc<dyn IDisposable> {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return DisposableAction::empty();
            };

            let trigger = this.to_ref().downgrade();
            let handler = associated_object.size_changed(move |_sender: &Interactive, e: &SizeChangedEventArgs| {
                if let Some(trigger) = trigger.upgrade() {
                    trigger.associated_object_on_size_changed(e);
                }
            });

            let associated_object = associated_object.downgrade();
            DisposableAction::create(move || {
                if let Some(associated_object) = associated_object.upgrade() {
                    associated_object.remove_handler(Control::size_changed_event(), handler);
                }
            })
        }
    }
}

impl SizeChangedTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: DisposingTrigger::construct(<Control as StaticType>::TYPE) }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn associated_object_on_size_changed(&self, e: &SizeChangedEventArgs) {
        // The args are the parameter of the actions (a shared handle to the live args).
        let parameter: BoxedValue = Rc::new(IRoutedEventArgs::share(e));
        self.execute(Some(&parameter));
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        if !self.is_enabled() {
            return;
        }

        let sender = self.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
        Interaction::execute_actions(sender.as_ref(), self.actions().as_ref(), parameter);
    }
}
