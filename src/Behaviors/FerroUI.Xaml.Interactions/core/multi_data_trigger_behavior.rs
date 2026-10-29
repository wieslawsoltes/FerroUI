//! Port of `Core/MultiDataTriggerBehavior.cs`.

use super::ReversibleActionExecution;
use ferroui_base::collections::{NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_base::controls::{NameScope, NameScopeExtensions};
use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectExtensions,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElement,
    StyledElementImpl, StyledProperty, UnsetValueType,
};
use ferroui_xaml_interactivity::helpers::ComparisonConditionTypeHelper;
use ferroui_xaml_interactivity::{
    ActionCollection, Condition, ConditionCollection, Interaction, StyledElementBehavior, StyledElementBehaviorImpl,
    StyledElementBehaviorImplExt, StyledElementTrigger,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A behavior that performs actions when all bound data conditions are satisfied.
///
/// A [`Condition`] compares the value of a binding, or of a property of the associated object or
/// of a named element, with its value. The conditions are evaluated in a job of the dispatcher
/// after one of them changed. With [`revert_on_false`](Self::revert_on_false) the reversible
/// actions are reverted when the conditions stop being met.
#[repr(C)]
pub struct MultiDataTriggerBehavior {
    base: StyledElementTrigger,
    is_condition_met: Cell<bool>,
    has_condition_state: Cell<bool>,
    applied_actions: RefCell<Vec<Ref<FerroObject>>>,
    /// The conditions the trigger listens to, with the subscription to their property changes.
    subscribed_conditions: RefCell<Vec<(Ref<Condition>, Rc<dyn IDisposable>)>>,
    /// The subscriptions to the property of the source of the conditions that name one.
    property_subscriptions: RefCell<Vec<(Ref<Condition>, Option<Rc<dyn IDisposable>>)>>,
    subscribed_conditions_collection: RefCell<Option<(ConditionCollection, u64)>>,
    subscribed_actions: RefCell<Option<(ActionCollection, u64)>>,
}

ferro_class!(MultiDataTriggerBehavior: StyledElementTrigger);
ferro_class_info!(MultiDataTriggerBehavior { new: MultiDataTriggerBehavior::new });
ferroui_base::ferro_impl_classes!(MultiDataTriggerBehavior: StyledElementImpl);

ferro_properties! {
    impl MultiDataTriggerBehavior {
        /// Identifies the `RevertOnFalse` property.
        pub fn revert_on_false_property() -> StyledProperty<bool> {
            FerroProperty::register::<MultiDataTriggerBehavior, _>("RevertOnFalse", false)
        }

        /// Identifies the `Conditions` property.
        pub fn conditions_property() -> StyledProperty<Option<ConditionCollection>> {
            FerroProperty::register::<MultiDataTriggerBehavior, _>("Conditions", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for MultiDataTriggerBehavior {
        /// The constructor body: the trigger starts with an empty collection of conditions.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);

            let default_conditions = ConditionCollection::new();
            this.set_current_value(Self::conditions_property(), Some(default_conditions.clone()));
            this.attach_conditions(Some(&default_conditions));
        }

        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            let property = change.property();

            if property == Self::conditions_property().as_property() {
                if let Some(old_conditions) = change.get_old_value::<Option<ConditionCollection>>().flatten() {
                    this.detach_conditions(Some(&old_conditions));
                }

                if let Some(new_conditions) = change.get_new_value::<Option<ConditionCollection>>() {
                    this.attach_conditions(Some(&new_conditions));
                }

                this.schedule_execute(Some(parameter_of(change)));
            }

            if property == Self::revert_on_false_property().as_property() {
                if change.get_old_value::<bool>().unwrap_or(false) && !change.get_new_value::<bool>() {
                    this.revert_actions(Some(&parameter_of(change)));
                }

                this.has_condition_state.set(false);
                this.schedule_execute(Some(parameter_of(change)));
            }

            if property == StyledElementBehavior::is_enabled_property().as_property() && this.revert_on_false() {
                let is_enabled = change.get_new_value::<bool>();
                if !is_enabled {
                    this.revert_actions(Some(&parameter_of(change)));
                }

                this.has_condition_state.set(false);
                if is_enabled {
                    this.schedule_execute(Some(parameter_of(change)));
                }
            }

            if property == StyledElementTrigger::actions_property().as_property() && this.associated_object().is_some() {
                this.update_action_subscription(change.get_new_value::<Option<ActionCollection>>());
                if this.revert_on_false() {
                    this.revert_actions(Some(&parameter_of(change)));
                    this.has_condition_state.set(false);
                    this.schedule_execute(Some(parameter_of(change)));
                }
            }
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for MultiDataTriggerBehavior {
        fn on_initialized_event(this: &Self) {
            Self::parent_on_initialized_event(this);

            this.execute(None);
            this.refresh_condition_subscriptions();
        }

        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);
            this.update_action_subscription(this.actions());
        }

        fn on_detaching(this: &Self) {
            if this.revert_on_false() {
                this.revert_actions(None);
            }

            this.has_condition_state.set(false);
            this.update_action_subscription(None);
            Self::parent_on_detaching(this);
        }

        fn on_attached_to_logical_tree_event(this: &Self) {
            Self::parent_on_attached_to_logical_tree_event(this);

            this.refresh_condition_subscriptions();
        }

        fn on_loaded(this: &Self) {
            Self::parent_on_loaded(this);

            this.refresh_condition_subscriptions();
        }
    }
}

/// The change as the parameter of the actions (the property changed event args of the reference).
fn parameter_of(change: &FerroPropertyChangedEventArgs<'_>) -> BoxedValue {
    Rc::new(change.to_owned_args())
}

/// Whether an untyped value is the unset marker (`Equals(value, UnsetValue)` of the reference).
fn is_unset(value: Option<&BoxedValue>) -> bool {
    value.is_some_and(|value| value.is::<UnsetValueType>())
}

impl MultiDataTriggerBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementTrigger::construct(),
            is_condition_met: Cell::new(false),
            has_condition_state: Cell::new(false),
            applied_actions: RefCell::new(Vec::new()),
            subscribed_conditions: RefCell::new(Vec::new()),
            property_subscriptions: RefCell::new(Vec::new()),
            subscribed_conditions_collection: RefCell::new(None),
            subscribed_actions: RefCell::new(None),
        }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the collection of conditions that must all be satisfied before actions are
    /// executed. This is a property of the object model.
    pub fn conditions(&self) -> Option<ConditionCollection> {
        self.get_value(Self::conditions_property())
    }

    pub fn set_conditions(&self, value: Option<ConditionCollection>) {
        self.set_value(Self::conditions_property(), value)
    }

    /// Gets or sets a value indicating whether reversible actions should be reverted when the
    /// conditions become false. When false, the trigger matches the legacy semantics and only
    /// executes actions when all conditions are true.
    pub fn revert_on_false(&self) -> bool {
        self.get_value(Self::revert_on_false_property())
    }

    pub fn set_revert_on_false(&self, value: bool) {
        self.set_value(Self::revert_on_false_property(), value)
    }

    fn attach_conditions(&self, conditions: Option<&ConditionCollection>) {
        let Some(conditions) = conditions else {
            return;
        };

        // The constructor of the reference subscribes to its default collection when the property
        // change already did; a collection is subscribed to once here.
        let is_subscribed = self
            .subscribed_conditions_collection
            .borrow()
            .as_ref()
            .is_some_and(|(subscribed, _)| subscribed == conditions);
        if !is_subscribed {
            let previous = self.subscribed_conditions_collection.borrow_mut().take();
            if let Some((previous, token)) = previous {
                previous.remove_collection_changed(token);
            }

            let behavior = self.to_ref().downgrade();
            let collection = conditions.clone();
            let token = conditions.add_collection_changed(Rc::new(
                move |e: &NotifyCollectionChangedEventArgs<'_, Ref<Condition>>| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.conditions_collection_changed(&collection, e);
                    }
                },
            ));
            *self.subscribed_conditions_collection.borrow_mut() = Some((conditions.clone(), token));
        }

        for condition in conditions.snapshot().iter() {
            self.attach_condition(condition);
        }
    }

    fn detach_conditions(&self, conditions: Option<&ConditionCollection>) {
        let Some(conditions) = conditions else {
            return;
        };

        let subscription = {
            let mut subscribed = self.subscribed_conditions_collection.borrow_mut();
            match subscribed.as_ref() {
                Some((collection, _)) if collection == conditions => subscribed.take(),
                _ => None,
            }
        };
        if let Some((collection, token)) = subscription {
            collection.remove_collection_changed(token);
        }

        self.detach_subscribed_conditions();
    }

    fn conditions_collection_changed(
        &self,
        sender: &ConditionCollection,
        e: &NotifyCollectionChangedEventArgs<'_, Ref<Condition>>,
    ) {
        match e.action {
            NotifyCollectionChangedAction::Reset => {
                self.detach_subscribed_conditions();

                for condition in sender.snapshot().iter() {
                    self.attach_condition(condition);
                }
            }

            NotifyCollectionChangedAction::Remove | NotifyCollectionChangedAction::Replace => {
                for condition in e.old_items {
                    self.detach_condition(condition);
                }

                for condition in e.new_items {
                    self.attach_condition(condition);
                }
            }

            NotifyCollectionChangedAction::Add => {
                for condition in e.new_items {
                    self.attach_condition(condition);
                }
            }

            _ => {}
        }

        self.schedule_execute(None);
    }

    fn attach_condition(&self, condition: &Ref<Condition>) {
        if self.subscribed_conditions.borrow().iter().any(|(subscribed, _)| subscribed.ptr_eq(condition)) {
            return;
        }

        let behavior = self.to_ref().downgrade();
        let sender = condition.downgrade();
        let subscription = condition.property_changed(move |e: &FerroPropertyChangedEventArgs<'_>| {
            if let (Some(behavior), Some(sender)) = (behavior.upgrade(), sender.upgrade()) {
                behavior.condition_property_changed(&sender, e);
            }
        });
        self.subscribed_conditions.borrow_mut().push((condition.clone(), subscription));
        self.update_condition_subscription(condition);
    }

    fn detach_condition(&self, condition: &Ref<Condition>) {
        let removed = {
            let mut subscribed = self.subscribed_conditions.borrow_mut();
            subscribed.iter().position(|(subscribed, _)| subscribed.ptr_eq(condition)).map(|index| subscribed.remove(index))
        };

        if let Some((condition, subscription)) = removed {
            subscription.dispose();
            self.dispose_property_subscription(&condition);
        }
    }

    fn condition_property_changed(&self, sender: &Ref<Condition>, e: &FerroPropertyChangedEventArgs<'_>) {
        let property = e.property();
        if property == Condition::binding_value_property().as_property()
            || property == Condition::value_property().as_property()
            || property == Condition::comparison_condition_property().as_property()
        {
            self.schedule_execute(Some(parameter_of(e)));
            return;
        }

        if property == Condition::property_property().as_property()
            || property == Condition::source_name_property().as_property()
        {
            self.update_condition_subscription(sender);
            self.schedule_execute(Some(parameter_of(e)));
        }
    }

    fn schedule_execute(&self, parameter: Option<BoxedValue>) {
        let behavior: Ref<MultiDataTriggerBehavior> = self.to_ref();
        Dispatcher::ui_thread().post_local(
            move || {
                behavior.execute(parameter.as_ref());
            },
            DispatcherPriority::DEFAULT,
        );
    }

    fn update_condition_subscription(&self, condition: &Ref<Condition>) {
        self.dispose_property_subscription(condition);

        let Some(property) = condition.property() else {
            return;
        };

        let Some(target) = self.resolve_condition_target(condition) else {
            return;
        };

        let observable = FerroObjectExtensions::get_observable_untyped(&*target, property);
        let behavior = self.to_ref().downgrade();
        let subscription = observable.subscribe_fn(move |_value: BoxedValue| {
            if let Some(behavior) = behavior.upgrade() {
                behavior.schedule_execute(None);
            }
        });
        self.property_subscriptions.borrow_mut().push((condition.clone(), Some(subscription)));
    }

    fn resolve_condition_target(&self, condition: &Condition) -> Option<Ref<FerroObject>> {
        let Some(source_name) = condition.source_name().filter(|name| !name.is_empty()) else {
            return self.associated_object();
        };

        let associated_styled_element: Option<Ref<FerroObject>> =
            self.associated_styled_element().map(|element| element.upcast());
        let named_target = Self::find_in_name_scope(self.associated_object().as_ref(), &source_name)
            .or_else(|| Self::find_in_name_scope(associated_styled_element.as_ref(), &source_name));
        if named_target.is_some() {
            return named_target;
        }

        if let Some(logical) = self.associated_object().and_then(|associated_object| associated_object.cast::<StyledElement>())
        {
            let logical_target = NameScopeExtensions::find::<FerroObject>(&logical, &source_name);
            if logical_target.is_some() {
                return logical_target;
            }
        }

        if let Some(styled_logical) = self.associated_styled_element() {
            return NameScopeExtensions::find::<FerroObject>(&styled_logical, &source_name);
        }

        None
    }

    fn find_in_name_scope(source: Option<&Ref<FerroObject>>, source_name: &str) -> Option<Ref<FerroObject>> {
        let logical_source = source?.clone().cast::<StyledElement>()?;

        for logical in logical_source.get_self_and_logical_ancestors() {
            if let Some(name_scope) = NameScope::get_name_scope(&logical) {
                if let Some(found) = name_scope.find(source_name) {
                    return Some(found);
                }
            }
        }

        None
    }

    fn refresh_condition_subscriptions(&self) {
        for condition in self.snapshot_subscribed_conditions() {
            self.update_condition_subscription(&condition);
        }
    }

    fn detach_subscribed_conditions(&self) {
        for condition in self.snapshot_subscribed_conditions() {
            self.detach_condition(&condition);
        }
    }

    fn dispose_property_subscription(&self, condition: &Ref<Condition>) {
        let removed = {
            let mut subscriptions = self.property_subscriptions.borrow_mut();
            subscriptions
                .iter()
                .position(|(subscribed, _)| subscribed.ptr_eq(condition))
                .map(|index| subscriptions.remove(index))
        };

        if let Some((_, Some(disposable))) = removed {
            disposable.dispose();
        }
    }

    fn snapshot_subscribed_conditions(&self) -> Vec<Ref<Condition>> {
        self.subscribed_conditions.borrow().iter().map(|(condition, _)| condition.clone()).collect()
    }

    /// The associated object as the sender of the actions.
    fn associated_sender(&self) -> Option<BoxedValue> {
        self.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue)
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        let Some(associated_object) = self.associated_sender() else {
            return;
        };

        if !self.is_enabled() {
            return;
        }

        let is_condition_met = self.is_condition_met();

        if !self.revert_on_false() {
            if is_condition_met {
                Interaction::execute_actions(Some(&associated_object), self.actions().as_ref(), parameter);
            }

            return;
        }

        if !self.has_condition_state.get() {
            self.has_condition_state.set(true);
            self.is_condition_met.set(is_condition_met);

            if is_condition_met {
                self.apply_actions(parameter);
            }

            return;
        }

        if self.is_condition_met.get() == is_condition_met {
            return;
        }

        self.is_condition_met.set(is_condition_met);

        if is_condition_met {
            self.apply_actions(parameter);
            return;
        }

        self.revert_actions(parameter);
    }

    fn is_condition_met(&self) -> bool {
        let Some(conditions) = self.conditions().filter(|conditions| conditions.count() > 0) else {
            return false;
        };

        for condition in conditions.snapshot().iter() {
            let Some(value) = self.try_get_condition_value(condition) else {
                return false;
            };

            let met = ComparisonConditionTypeHelper::compare(
                value.as_ref(),
                condition.comparison_condition(),
                condition.value().as_ref(),
            )
            .unwrap_or_else(|error| panic!("{error}"));
            if !met {
                return false;
            }
        }

        true
    }

    fn revert_actions(&self, parameter: Option<&BoxedValue>) {
        let Some(associated_object) = self.associated_sender() else {
            return;
        };

        let applied_actions = std::mem::take(&mut *self.applied_actions.borrow_mut());
        ReversibleActionExecution::revert(&applied_actions, Some(&associated_object), parameter);
    }

    fn apply_actions(&self, parameter: Option<&BoxedValue>) {
        self.applied_actions.borrow_mut().clear();
        let associated_object = self.associated_sender();
        let applied = ReversibleActionExecution::execute(associated_object.as_ref(), self.actions().as_ref(), parameter);
        self.applied_actions.borrow_mut().extend(applied);
    }

    fn update_action_subscription(&self, actions: Option<ActionCollection>) {
        let previous = self.subscribed_actions.borrow_mut().take();
        if let Some((subscribed_actions, token)) = previous {
            subscribed_actions.remove_collection_changed(token);
        }

        let Some(actions) = actions else {
            return;
        };

        let behavior = self.to_ref().downgrade();
        let token = actions.add_collection_changed(Rc::new(
            move |event_args: &NotifyCollectionChangedEventArgs<'_, Ref<FerroObject>>| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.actions_collection_changed(event_args);
                }
            },
        ));
        *self.subscribed_actions.borrow_mut() = Some((actions, token));
    }

    fn actions_collection_changed(&self, event_args: &NotifyCollectionChangedEventArgs<'_, Ref<FerroObject>>) {
        if !self.revert_on_false() || self.associated_object().is_none() {
            return;
        }

        let parameter: BoxedValue = Rc::new(event_args.share());
        self.revert_actions(Some(&parameter));
        self.has_condition_state.set(false);
        let behavior: Ref<MultiDataTriggerBehavior> = self.to_ref();
        Dispatcher::ui_thread().post_local(
            move || {
                behavior.execute(Some(&parameter));
            },
            DispatcherPriority::DEFAULT,
        );
    }

    /// The value of the left operand of a condition: `Some(value)` when it is known
    /// (`TryGetConditionValue(condition, out value)`).
    fn try_get_condition_value(&self, condition: &Condition) -> Option<Option<BoxedValue>> {
        if let Some(property) = condition.property() {
            let target = self.resolve_condition_target(condition)?;

            let value = target.get_value_untyped(property);
            if value.is::<UnsetValueType>() {
                return None;
            }

            return Some(Some(value));
        }

        if condition.binding().is_some() {
            if !condition.is_set(Condition::binding_value_property()) {
                return None;
            }

            let value = condition.binding_value();
            if is_unset(value.as_ref()) {
                return None;
            }

            return Some(value);
        }

        let value = condition.binding_value();
        if is_unset(value.as_ref()) {
            None
        } else {
            Some(value)
        }
    }
}
