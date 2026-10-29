//! Port of `Triggers/PointerPressedDisposingTrigger.cs` of the reference sample.

use ferroui_base::input::{InputElement, PointerPressedEventArgs};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutingStrategies};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, Ref, StaticType,
    StyledElementImpl,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{
    DisposableAction, DisposingTrigger, DisposingTriggerImpl, Interaction,
    StyledElementBehaviorImpl,
};
use std::rc::Rc;

/// Executes its actions when its control is pressed; the handler is removed by the disposable of
/// the trigger (`PointerPressedDisposingTrigger : DisposingTrigger<Control>`).
#[repr(C)]
pub struct PointerPressedDisposingTrigger {
    base: DisposingTrigger,
}

ferro_class!(PointerPressedDisposingTrigger: DisposingTrigger);
ferro_class_info!(PointerPressedDisposingTrigger {
    new: PointerPressedDisposingTrigger::new
});
ferroui_base::ferro_impl_classes!(
    PointerPressedDisposingTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
);

ferroui_base::ferro_overrides! {
    impl DisposingTriggerImpl for PointerPressedDisposingTrigger {
        fn on_attached_override(this: &Self) -> Rc<dyn IDisposable> {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return DisposableAction::empty();
            };

            let trigger = this.to_ref().downgrade();
            let handler = associated_object.add_handler_with(
                &InputElement::pointer_pressed_event(),
                move |_sender: &Interactive, e: &PointerPressedEventArgs| {
                    if let Some(trigger) = trigger.upgrade() {
                        trigger.execute(e);
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );

            let associated_object = associated_object.downgrade();
            DisposableAction::create(move || {
                if let Some(associated_object) = associated_object.upgrade() {
                    associated_object.remove_handler(&InputElement::pointer_pressed_event(), handler);
                }
            })
        }
    }
}

impl PointerPressedDisposingTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: DisposingTrigger::construct(<Control as StaticType>::TYPE),
        }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// `Interaction.ExecuteActions(AssociatedObject, Actions, e)`.
    fn execute(&self, e: &PointerPressedEventArgs) {
        // The args are the parameter of the actions (a shared handle to the live args).
        let parameter: BoxedValue = Rc::new(IRoutedEventArgs::share(e));
        let sender = self
            .associated_object()
            .map(|associated_object| Rc::new(associated_object) as BoxedValue);
        Interaction::execute_actions(sender.as_ref(), self.actions().as_ref(), Some(&parameter));
    }
}
