//! Port of `Core/IfElseTrigger.cs`.

use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, DirectProperty, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{
    ActionCollection, Interaction, StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementTrigger,
};
use std::cell::OnceCell;
use std::rc::Rc;

/// A behavior that executes different collections of actions depending on the specified
/// condition.
///
/// The actions of [`if_actions`](Self::if_actions) (the content of the trigger in markup) run
/// when [`condition`](Self::condition) is true, the actions of
/// [`else_actions`](Self::else_actions) when it is false: once when the trigger is initialized,
/// then in a job of the dispatcher after each change of the condition.
#[repr(C)]
pub struct IfElseTrigger {
    base: StyledElementTrigger,
    if_actions: OnceCell<ActionCollection>,
    else_actions: OnceCell<ActionCollection>,
}

ferro_class!(IfElseTrigger: StyledElementTrigger);
ferro_class_info!(IfElseTrigger {
    new: IfElseTrigger::new,
    markup: {
        content: IfActions,
    },
});
ferroui_base::ferro_impl_classes!(IfElseTrigger: StyledElementImpl);

ferro_properties! {
    impl IfElseTrigger {
        /// Identifies the `Condition` property.
        pub fn condition_property() -> StyledProperty<bool> {
            FerroProperty::register::<IfElseTrigger, _>("Condition", false)
        }

        /// Identifies the `IfActions` property.
        pub fn if_actions_property() -> DirectProperty<IfElseTrigger, ActionCollection> {
            FerroProperty::register_direct::<IfElseTrigger, _>(
                "IfActions",
                |trigger| trigger.if_actions(),
                None,
                ActionCollection::new(),
            )
        }

        /// Identifies the `ElseActions` property.
        pub fn else_actions_property() -> DirectProperty<IfElseTrigger, ActionCollection> {
            FerroProperty::register_direct::<IfElseTrigger, _>(
                "ElseActions",
                |trigger| trigger.else_actions(),
                None,
                ActionCollection::new(),
            )
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for IfElseTrigger {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            if change.property() == Self::condition_property().as_property() {
                this.on_condition_changed(change);
            }
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for IfElseTrigger {
        fn on_initialized_event(this: &Self) {
            Self::parent_on_initialized_event(this);

            this.execute(None);
        }
    }
}

impl IfElseTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTrigger::construct(), if_actions: OnceCell::new(), else_actions: OnceCell::new() }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the condition that determines which actions are executed. This is a property
    /// of the object model.
    pub fn condition(&self) -> bool {
        self.get_value(Self::condition_property())
    }

    pub fn set_condition(&self, value: bool) {
        self.set_value(Self::condition_property(), value)
    }

    /// Gets the actions executed when [`condition`](Self::condition) evaluates to `true`. This is
    /// a property of the object model.
    pub fn if_actions(&self) -> ActionCollection {
        self.if_actions.get_or_init(ActionCollection::new).clone()
    }

    /// Gets the actions executed when [`condition`](Self::condition) evaluates to `false`. This
    /// is a property of the object model.
    pub fn else_actions(&self) -> ActionCollection {
        self.else_actions.get_or_init(ActionCollection::new).clone()
    }

    fn on_condition_changed(&self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.sender().downcast_ref::<IfElseTrigger>().is_none() {
            return;
        }

        let behavior: Ref<IfElseTrigger> = self.to_ref();
        // The change is the parameter of the actions (the property changed event args of the
        // reference).
        let parameter: BoxedValue = Rc::new(change.to_owned_args());
        Dispatcher::ui_thread().post_local(
            move || {
                behavior.execute(Some(&parameter));
            },
            DispatcherPriority::DEFAULT,
        );
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        let Some(associated_object) = self.associated_object() else {
            return;
        };

        if !self.is_enabled() {
            return;
        }

        let actions = if self.condition() { self.if_actions() } else { self.else_actions() };
        let sender: BoxedValue = Rc::new(associated_object);
        Interaction::execute_actions(Some(&sender), Some(&actions), parameter);
    }
}
