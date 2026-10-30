//! Port of `Logic/ConditionalAction.cs`.

use super::case::attach_actions;
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledElementImplExt, StyledProperty,
};
use ferroui_xaml_interactivity::{ActionCollection, Interaction, StyledElementAction, StyledElementActionImpl};

/// An action that executes its [`actions`](Self::actions) (its content in markup) when
/// [`condition`](Self::condition) is true, its [`else_actions`](Self::else_actions) otherwise.
///
/// While the action is in the logical tree the styled element actions of both collections are
/// its logical children (one way).
#[repr(C)]
pub struct ConditionalAction {
    base: StyledElementAction,
}

ferro_class!(ConditionalAction: StyledElementAction);
ferro_class_info!(ConditionalAction {
    new: ConditionalAction::new,
    markup: {
        content: Actions,
    },
});

ferro_properties! {
    impl ConditionalAction {
        /// Identifies the `Condition` property.
        pub fn condition_property() -> StyledProperty<bool> {
            FerroProperty::register::<ConditionalAction, _>("Condition", false)
        }

        /// Identifies the `Actions` property.
        pub fn actions_property() -> StyledProperty<Option<ActionCollection>> {
            FerroProperty::register::<ConditionalAction, _>("Actions", None)
        }

        /// Identifies the `ElseActions` property.
        pub fn else_actions_property() -> StyledProperty<Option<ActionCollection>> {
            FerroProperty::register::<ConditionalAction, _>("ElseActions", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for ConditionalAction {
        /// The constructor body: the action starts with empty collections of actions.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);
            this.set_current_value(Self::actions_property(), Some(ActionCollection::new()));
            this.set_current_value(Self::else_actions_property(), Some(ActionCollection::new()));
        }

        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            let property = change.property();
            if property == Self::actions_property().as_property() || property == Self::else_actions_property().as_property() {
                let old_actions = change.get_old_value::<Option<ActionCollection>>().flatten();
                let new_actions = change.get_new_value::<Option<ActionCollection>>();

                if old_actions.is_some() && this.is_attached_to_logical_tree() {
                    attach_actions::detach(old_actions.as_ref(), this);
                }

                if new_actions.is_some() && this.is_attached_to_logical_tree() {
                    attach_actions::attach(new_actions.as_ref(), this);
                }
            }
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementImpl for ConditionalAction {
        fn on_attached_to_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
            Self::parent_on_attached_to_logical_tree(this, e);
            attach_actions::attach(this.actions().as_ref(), this);
            attach_actions::attach(this.else_actions().as_ref(), this);
        }

        fn on_detached_from_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
            attach_actions::detach(this.actions().as_ref(), this);
            attach_actions::detach(this.else_actions().as_ref(), this);
            Self::parent_on_detached_from_logical_tree(this, e);
        }
    }
}

impl StyledElementActionImpl for ConditionalAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if this.condition() {
            if let Some(actions) = this.actions() {
                Interaction::execute_actions(sender, Some(&actions), parameter);
            }
        } else if let Some(else_actions) = this.else_actions() {
            Interaction::execute_actions(sender, Some(&else_actions), parameter);
        }

        None
    }
}

impl ConditionalAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
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

    /// Gets or sets the actions executed when the condition is true. This is a property of the
    /// object model.
    pub fn actions(&self) -> Option<ActionCollection> {
        self.get_value(Self::actions_property())
    }

    pub fn set_actions(&self, value: Option<ActionCollection>) {
        self.set_value(Self::actions_property(), value)
    }

    /// Gets or sets the actions executed when the condition is false. This is a property of the
    /// object model.
    pub fn else_actions(&self) -> Option<ActionCollection> {
        self.get_value(Self::else_actions_property())
    }

    pub fn set_else_actions(&self, value: Option<ActionCollection>) {
        self.set_value(Self::else_actions_property(), value)
    }
}
