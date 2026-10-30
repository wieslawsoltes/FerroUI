//! Port of `Logic/ConditionalBehavior.cs`.

use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElement, StyledElementImpl, StyledProperty,
};
use ferroui_controls::TopLevel;
use ferroui_xaml_interactivity::{
    ActionCollection, Interaction, StyledElementAction, StyledElementBehaviorImpl, StyledElementBehaviorImplExt,
    StyledElementTrigger,
};
use std::rc::Rc;

/// A trigger that executes its actions (its content in markup) when [`condition`](Self::condition)
/// changes to true, its [`else_actions`](Self::else_actions) when it changes to false.
#[repr(C)]
pub struct ConditionalBehavior {
    base: StyledElementTrigger,
}

ferro_class!(ConditionalBehavior: StyledElementTrigger);
ferro_class_info!(ConditionalBehavior { new: ConditionalBehavior::new });
ferroui_base::ferro_impl_classes!(ConditionalBehavior: StyledElementImpl);

ferro_properties! {
    impl ConditionalBehavior {
        /// Identifies the `Condition` property.
        pub fn condition_property() -> StyledProperty<bool> {
            FerroProperty::register::<ConditionalBehavior, _>("Condition", false)
        }

        /// Identifies the `ElseActions` property.
        pub fn else_actions_property() -> StyledProperty<Option<ActionCollection>> {
            FerroProperty::register::<ConditionalBehavior, _>("ElseActions", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for ConditionalBehavior {
        /// The constructor body: the trigger starts with an empty collection of else actions.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);
            this.set_current_value(Self::else_actions_property(), Some(ActionCollection::new()));
        }

        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            if change.property() == Self::condition_property().as_property() {
                this.execute(change.get_new_value::<bool>());
            } else if change.property() == Self::else_actions_property().as_property() {
                let old_actions = change.get_old_value::<Option<ActionCollection>>().flatten();
                let new_actions = change.get_new_value::<Option<ActionCollection>>();

                if old_actions.is_some() && this.is_attached_to_logical_tree() {
                    this.detach_actions_from_logical_tree(old_actions.as_ref());
                }

                if new_actions.is_some() && this.is_attached_to_logical_tree() {
                    this.attach_actions_to_logical_tree(new_actions.as_ref());
                }
            }
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ConditionalBehavior {
        fn on_attached_to_logical_tree_event(this: &Self) {
            Self::parent_on_attached_to_logical_tree_event(this);
            this.attach_actions_to_logical_tree(this.else_actions().as_ref());
        }

        fn on_detached_from_logical_tree_event(this: &Self) {
            this.detach_actions_from_logical_tree(this.else_actions().as_ref());
            Self::parent_on_detached_from_logical_tree_event(this);
        }
    }
}

impl ConditionalBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTrigger::construct() }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the condition. This is a property of the object model.
    pub fn condition(&self) -> bool {
        self.get_value(Self::condition_property())
    }

    pub fn set_condition(&self, value: bool) {
        self.set_value(Self::condition_property(), value)
    }

    /// Gets or sets the actions executed when the condition changes to false. This is a property
    /// of the object model.
    pub fn else_actions(&self) -> Option<ActionCollection> {
        self.get_value(Self::else_actions_property())
    }

    pub fn set_else_actions(&self, value: Option<ActionCollection>) {
        self.set_value(Self::else_actions_property(), value)
    }

    /// The parent of the actions in the logical tree: the top level that is the associated
    /// object, else the trigger while its associated object has a parent.
    fn actions_parent(&self) -> Option<Ref<StyledElement>> {
        let associated_object = self.associated_object();
        if let Some(top_level) = associated_object.as_ref().and_then(|associated_object| associated_object.cast::<TopLevel>()) {
            return Some(top_level.upcast());
        }

        let styled_element = associated_object.and_then(|associated_object| associated_object.cast::<StyledElement>())?;
        styled_element.parent()?;

        Some(self.to_ref().upcast())
    }

    fn attach_actions_to_logical_tree(&self, actions: Option<&ActionCollection>) {
        let Some(actions) = actions else {
            return;
        };
        let Some(parent) = self.actions_parent() else {
            return;
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
        let Some(parent) = self.actions_parent() else {
            return;
        };

        for action in actions.snapshot().iter() {
            if let Some(styled_element_action) = action.downcast_ref::<StyledElementAction>() {
                styled_element_action.detach_action_from_logical_tree(&parent);
            }
        }
    }

    fn execute(&self, condition: bool) {
        let sender = self.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
        if condition {
            if let Some(actions) = self.actions() {
                Interaction::execute_actions(sender.as_ref(), Some(&actions), None);
            }
        } else if let Some(else_actions) = self.else_actions() {
            Interaction::execute_actions(sender.as_ref(), Some(&else_actions), None);
        }
    }
}
