//! Port of `Icon/PathIconDataChangedTrigger.cs`.

use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectExtensions, FerroObjectImpl, Ref, StaticType,
    StyledElementImpl,
};
use ferroui_controls::PathIcon;
use ferroui_xaml_interactivity::{
    DisposableAction, DisposingTrigger, DisposingTriggerImpl, Interaction, StyledElementBehaviorImpl,
};
use std::rc::Rc;

/// Invokes actions whenever the [`PathIcon::data`] property changes (and once when the trigger is
/// attached, as the observable of the property reports its current value).
///
/// The actions run with no parameter.
#[repr(C)]
pub struct PathIconDataChangedTrigger {
    base: DisposingTrigger,
}

ferro_class!(PathIconDataChangedTrigger: DisposingTrigger);
ferro_class_info!(PathIconDataChangedTrigger { new: PathIconDataChangedTrigger::new });
ferroui_base::ferro_impl_classes!(PathIconDataChangedTrigger: FerroObjectImpl, StyledElementImpl, StyledElementBehaviorImpl);

ferroui_base::ferro_overrides! {
    impl DisposingTriggerImpl for PathIconDataChangedTrigger {
        fn on_attached_override(this: &Self) -> Rc<dyn IDisposable> {
            let Some(associated_object) = this.associated_object_as::<PathIcon>() else {
                return DisposableAction::empty();
            };

            let trigger = this.to_ref().downgrade();
            FerroObjectExtensions::get_observable(&*associated_object as &ferroui_base::FerroObject, PathIcon::data_property()).subscribe_fn(move |_| {
                if let Some(trigger) = trigger.upgrade() {
                    trigger.execute(None);
                }
            })
        }
    }
}

impl PathIconDataChangedTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: DisposingTrigger::construct(<PathIcon as StaticType>::TYPE) }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        let Some(associated_object) = self.associated_object() else {
            return;
        };
        if !self.is_enabled() {
            return;
        }

        let sender: BoxedValue = Rc::new(associated_object);
        Interaction::execute_actions(Some(&sender), self.actions().as_ref(), parameter);
    }
}
