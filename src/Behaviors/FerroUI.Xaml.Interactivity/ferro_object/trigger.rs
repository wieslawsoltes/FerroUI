//! Port of `FerroObject/Trigger.cs`.

use crate::{ActionCollection, Behavior, BehaviorImpl, IBehavior, ITrigger};
use ferroui_base::{ferro_class, ferro_class_info, ferro_properties, DirectProperty, FerroObject, FerroObjectImpl, FerroProperty, Ref};
use std::cell::OnceCell;

/// A base class for behaviors, implementing the basic plumbing of [`ITrigger`].
///
/// The class is abstract. Its content (markup's children of a trigger) is its
/// [`actions`](Self::actions).
#[repr(C)]
pub struct Trigger {
    base: Behavior,
    actions: OnceCell<ActionCollection>,
}

ferro_class!(Trigger: Behavior);
ferro_class_info!(Trigger {
    markup: {
        content: Actions,
    },
});
ferroui_base::ferro_impl_classes!(Trigger: FerroObjectImpl, BehaviorImpl);

ferro_properties! {
    impl Trigger {
        /// Identifies the `Actions` property.
        pub fn actions_property() -> DirectProperty<Trigger, ActionCollection> {
            FerroProperty::register_direct::<Trigger, _>("Actions", |t| t.actions(), None, ActionCollection::new())
        }
    }
}

impl Trigger {
    /// The static constructor: the class implements the trigger contract.
    fn static_constructor() {
        <dyn ITrigger>::register::<Trigger>();
    }

    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Behavior::construct(), actions: OnceCell::new() }
    }

    /// Gets the collection of actions associated with the behavior. This is a property of the
    /// object model.
    pub fn actions(&self) -> ActionCollection {
        self.actions.get_or_init(ActionCollection::new).clone()
    }
}

impl IBehavior for Trigger {
    fn associated_object(&self) -> Option<Ref<FerroObject>> {
        Behavior::associated_object(self)
    }

    fn attach(&self, associated_object: Option<Ref<FerroObject>>) {
        Behavior::attach(self, associated_object)
    }

    fn detach(&self) {
        Behavior::detach(self)
    }
}

impl ITrigger for Trigger {
    fn actions(&self) -> Option<ActionCollection> {
        Some(Trigger::actions(self))
    }
}
