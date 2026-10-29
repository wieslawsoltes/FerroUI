//! Port of `StyledElement/StyledElementTrigger.cs`.

use crate::{
    ActionCollection, IBehavior, ITrigger, StyledElementAction, StyledElementBehavior, StyledElementBehaviorImpl,
    StyledElementBehaviorImplExt,
};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledElement, StyledElementImpl, StyledProperty,
};
use ferroui_controls::TopLevel;

/// A base class for behaviors, implementing the basic plumbing of [`ITrigger`].
///
/// The actions are a styled property, so a style can replace them (an
/// [`ActionCollectionTemplate`](crate::ActionCollectionTemplate) in a setter). While the trigger
/// is in the logical tree its styled element actions are its logical children (one way), so they
/// bind against the trigger and the host.
#[repr(C)]
pub struct StyledElementTrigger {
    base: StyledElementBehavior,
}

ferro_class!(StyledElementTrigger: StyledElementBehavior);
ferro_class_info!(StyledElementTrigger {
    markup: {
        content: Actions,
    },
});
ferroui_base::ferro_impl_classes!(StyledElementTrigger: StyledElementImpl);

ferro_properties! {
    impl StyledElementTrigger {
        /// Identifies the `Actions` property.
        pub fn actions_property() -> StyledProperty<Option<ActionCollection>> {
            FerroProperty::register::<StyledElementTrigger, _>("Actions", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for StyledElementTrigger {
        /// The constructor body: the trigger starts with an empty collection of actions, set as
        /// the current value so that styles can replace it.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);
            this.set_current_value(Self::actions_property(), Some(ActionCollection::new()));
        }

        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            if change.property() == Self::actions_property().as_property() {
                let old_actions = change.get_old_value::<Option<ActionCollection>>().flatten();
                let new_actions = change.get_new_value::<Option<ActionCollection>>();

                if let Some(old_actions) = &old_actions {
                    if this.is_initialized() {
                        this.detach_actions_from_logical_tree(Some(old_actions));
                    }
                }

                if let Some(new_actions) = &new_actions {
                    if this.is_initialized() {
                        this.attach_actions_to_logical_tree(Some(new_actions));
                    }
                }
            }
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for StyledElementTrigger {
        fn initialize(this: &Self) {
            Self::parent_initialize(this);

            this.initialize_actions(this.actions().as_ref());
        }

        fn attach_behavior_to_logical_tree(this: &Self) {
            Self::parent_attach_behavior_to_logical_tree(this);

            this.attach_actions_to_logical_tree(this.actions().as_ref());
        }

        fn detach_behavior_from_logical_tree(this: &Self) {
            this.detach_actions_from_logical_tree(this.actions().as_ref());

            Self::parent_detach_behavior_from_logical_tree(this);
        }
    }
}

impl StyledElementTrigger {
    /// The static constructor: the class implements the trigger contract.
    fn static_constructor() {
        <dyn ITrigger>::register::<StyledElementTrigger>();
    }

    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementBehavior::construct() }
    }

    /// Gets or sets the collection of actions associated with the behavior. This is a property
    /// of the object model.
    pub fn actions(&self) -> Option<ActionCollection> {
        self.get_value(Self::actions_property())
    }

    pub fn set_actions(&self, value: Option<ActionCollection>) {
        self.set_value(Self::actions_property(), value)
    }

    fn attach_actions_to_logical_tree(&self, actions: Option<&ActionCollection>) {
        let Some(actions) = actions else {
            return;
        };

        let parent: Ref<StyledElement> = match self.associated_object().and_then(|o| o.cast::<TopLevel>()) {
            Some(top_level) => top_level.upcast(),
            None => {
                if self.parent().is_none() {
                    return;
                }

                self.to_ref().upcast()
            }
        };

        for action in actions.snapshot().iter() {
            if let Some(styled_element_action) = action.downcast_ref::<StyledElementAction>() {
                styled_element_action.attach_action_to_logical_tree(&parent);
            }
        }
    }

    fn detach_actions_from_logical_tree(&self, actions: Option<&ActionCollection>) {
        let Some(actions) = actions else {
            return;
        };

        let parent: &StyledElement = self;

        for action in actions.snapshot().iter() {
            if let Some(styled_element_action) = action.downcast_ref::<StyledElementAction>() {
                styled_element_action.detach_action_from_logical_tree(parent);
            }
        }
    }

    fn initialize_actions(&self, actions: Option<&ActionCollection>) {
        let Some(actions) = actions else {
            return;
        };

        for action in actions.snapshot().iter() {
            if let Some(styled_element_action) = action.downcast_ref::<StyledElementAction>() {
                styled_element_action.initialize();
            }
        }
    }
}

impl IBehavior for StyledElementTrigger {
    fn associated_object(&self) -> Option<Ref<FerroObject>> {
        StyledElementBehavior::associated_object(self)
    }

    fn attach(&self, associated_object: Option<Ref<FerroObject>>) {
        StyledElementBehavior::attach(self, associated_object)
    }

    fn detach(&self) {
        StyledElementBehavior::detach(self)
    }
}

impl ITrigger for StyledElementTrigger {
    fn actions(&self) -> Option<ActionCollection> {
        StyledElementTrigger::actions(self)
    }
}
