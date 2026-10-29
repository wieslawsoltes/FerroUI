//! Port of `Logic/SwitchCaseBehavior.cs`.

use super::case::{attach_actions, attach_cases};
use crate::{Case, CaseCollection};
use ferroui_base::collections::NotifyCollectionChangedEventArgs;
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledElementImplExt, StyledProperty,
    WeakRef,
};
use ferroui_xaml_interactivity::{
    ActionCollection, Interaction, StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementTrigger,
};
use std::cell::RefCell;
use std::rc::Rc;

/// A trigger that executes the actions of the first of its [`cases`](Self::cases) (its content
/// in markup) whose value equals [`value`](Self::value), or its
/// [`default_actions`](Self::default_actions) when none does: once when it is initialized, then
/// in a job of the dispatcher after each change of the value.
///
/// While the trigger is in the logical tree its cases and its styled element default actions are
/// its logical children (one way).
#[repr(C)]
pub struct SwitchCaseBehavior {
    base: StyledElementTrigger,
    // The list of cases the trigger follows, with the token of its subscription.
    cases_subscription: RefCell<Option<(CaseCollection, u64)>>,
}

ferro_class!(SwitchCaseBehavior: StyledElementTrigger);
ferro_class_info!(SwitchCaseBehavior {
    new: SwitchCaseBehavior::new,
    markup: {
        content: Cases,
    },
});

ferro_properties! {
    impl SwitchCaseBehavior {
        /// Identifies the `Value` property.
        pub fn value_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<SwitchCaseBehavior, _>("Value", None)
        }

        /// Identifies the `Cases` property.
        pub fn cases_property() -> StyledProperty<Option<CaseCollection>> {
            FerroProperty::register::<SwitchCaseBehavior, _>("Cases", None)
        }

        /// Identifies the `DefaultActions` property.
        pub fn default_actions_property() -> StyledProperty<Option<ActionCollection>> {
            FerroProperty::register::<SwitchCaseBehavior, _>("DefaultActions", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for SwitchCaseBehavior {
        /// The constructor body: the switch starts with an empty list of cases and an empty
        /// collection of default actions.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);
            this.set_current_value(Self::cases_property(), Some(CaseCollection::new()));
            this.set_current_value(Self::default_actions_property(), Some(ActionCollection::new()));
        }

        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            let property = change.property();
            if property == Self::value_property().as_property() {
                this.on_value_changed(change);
            } else if property == Self::cases_property().as_property() {
                let old_cases = change.get_old_value::<Option<CaseCollection>>().flatten();
                let new_cases = change.get_new_value::<Option<CaseCollection>>();

                if let Some(old_cases) = &old_cases {
                    this.unsubscribe_cases(old_cases);
                    if this.is_attached_to_logical_tree() {
                        attach_cases::detach(&old_cases.snapshot());
                    }
                }

                if let Some(new_cases) = &new_cases {
                    this.subscribe_cases(new_cases);
                    if this.is_attached_to_logical_tree() {
                        attach_cases::attach(&new_cases.snapshot(), this);
                    }
                }
            } else if property == Self::default_actions_property().as_property() {
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
    impl StyledElementImpl for SwitchCaseBehavior {
        fn on_attached_to_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
            Self::parent_on_attached_to_logical_tree(this, e);
            if let Some(cases) = this.cases() {
                attach_cases::attach(&cases.snapshot(), this);
            }
            attach_actions::attach(this.default_actions().as_ref(), this);
        }

        fn on_detached_from_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
            if let Some(cases) = this.cases() {
                attach_cases::detach(&cases.snapshot());
            }
            attach_actions::detach(this.default_actions().as_ref(), this);
            Self::parent_on_detached_from_logical_tree(this, e);
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for SwitchCaseBehavior {
        fn on_initialized_event(this: &Self) {
            Self::parent_on_initialized_event(this);
            this.execute(None);
        }
    }
}

impl SwitchCaseBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTrigger::construct(), cases_subscription: RefCell::new(None) }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the value compared with the values of the cases. This is a property of the
    /// object model.
    pub fn value(&self) -> Option<BoxedValue> {
        self.get_value(Self::value_property())
    }

    /// Sets the value compared with the values of the cases (named apart from the setter of
    /// every property of the object, as the condition of the object model names its own).
    pub fn set_switch_value(&self, value: Option<BoxedValue>) {
        self.set_value(Self::value_property(), value)
    }

    /// Gets or sets the cases. This is a property of the object model.
    pub fn cases(&self) -> Option<CaseCollection> {
        self.get_value(Self::cases_property())
    }

    pub fn set_cases(&self, value: Option<CaseCollection>) {
        self.set_value(Self::cases_property(), value)
    }

    /// Gets or sets the actions executed when no case matches. This is a property of the object
    /// model.
    pub fn default_actions(&self) -> Option<ActionCollection> {
        self.get_value(Self::default_actions_property())
    }

    pub fn set_default_actions(&self, value: Option<ActionCollection>) {
        self.set_value(Self::default_actions_property(), value)
    }

    /// Subscribes to the changes of the list of cases (`CollectionChanged +=`).
    fn subscribe_cases(&self, cases: &CaseCollection) {
        let switch: WeakRef<SwitchCaseBehavior> = self.to_ref().downgrade();
        let token = cases.add_collection_changed(Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, Ref<Case>>| {
            if let Some(switch) = switch.upgrade() {
                attach_cases::on_cases_collection_changed(&switch, e);
            }
        }));
        *self.cases_subscription.borrow_mut() = Some((cases.clone(), token));
    }

    /// Unsubscribes from the changes of the list of cases (`CollectionChanged -=`).
    fn unsubscribe_cases(&self, cases: &CaseCollection) {
        let subscription = self.cases_subscription.borrow_mut().take();
        if let Some((subscribed_cases, token)) = subscription {
            if subscribed_cases == *cases {
                subscribed_cases.remove_collection_changed(token);
            } else {
                *self.cases_subscription.borrow_mut() = Some((subscribed_cases, token));
            }
        }
    }

    fn on_value_changed(&self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.sender().downcast_ref::<SwitchCaseBehavior>().is_none() {
            return;
        }

        let behavior: Ref<SwitchCaseBehavior> = self.to_ref();
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
        let associated_object = self.associated_object();
        let Some(associated_object) = associated_object.filter(|_| self.is_enabled()) else {
            return;
        };
        let sender: BoxedValue = Rc::new(associated_object);

        let value = self.value();
        if let Some(cases) = self.cases() {
            for case_item in cases.snapshot().iter() {
                if case_item.value() == value {
                    if let Some(actions) = case_item.actions() {
                        Interaction::execute_actions(Some(&sender), Some(&actions), parameter);
                    }
                    return;
                }
            }
        }

        if let Some(default_actions) = self.default_actions() {
            Interaction::execute_actions(Some(&sender), Some(&default_actions), parameter);
        }
    }
}
