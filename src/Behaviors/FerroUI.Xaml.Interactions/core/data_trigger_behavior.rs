//! Port of `Core/DataTriggerBehavior.cs`.

use super::ReversibleActionExecution;
use ferroui_base::collections::NotifyCollectionChangedEventArgs;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty,
    UnsetValueType,
};
use ferroui_xaml_interactivity::helpers::ComparisonConditionTypeHelper;
use ferroui_xaml_interactivity::{
    ActionCollection, ComparisonConditionType, Interaction, StyledElementBehavior, StyledElementBehaviorImpl,
    StyledElementBehaviorImplExt, StyledElementTrigger,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A behavior that performs actions when the bound data meets a specified condition.
///
/// The comparison of [`binding`](Self::binding) with [`value`](Self::value) runs in a job of the
/// dispatcher after one of them changed. With [`revert_on_false`](Self::revert_on_false) the
/// reversible actions are reverted when the condition stops being met.
#[repr(C)]
pub struct DataTriggerBehavior {
    base: StyledElementTrigger,
    is_condition_met: Cell<bool>,
    has_condition_state: Cell<bool>,
    applied_actions: RefCell<Vec<Ref<FerroObject>>>,
    subscribed_actions: RefCell<Option<(ActionCollection, u64)>>,
}

ferro_class!(DataTriggerBehavior: StyledElementTrigger);
ferro_class_info!(DataTriggerBehavior { new: DataTriggerBehavior::new });
ferroui_base::ferro_impl_classes!(DataTriggerBehavior: StyledElementImpl);

ferro_properties! {
    impl DataTriggerBehavior {
        /// Identifies the `Binding` property.
        pub fn binding_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<DataTriggerBehavior, _>("Binding", None)
        }

        /// Identifies the `ComparisonCondition` property.
        pub fn comparison_condition_property() -> StyledProperty<ComparisonConditionType> {
            FerroProperty::register::<DataTriggerBehavior, _>("ComparisonCondition", ComparisonConditionType::Equal)
        }

        /// Identifies the `Value` property.
        pub fn value_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<DataTriggerBehavior, _>("Value", None)
        }

        /// Identifies the `RevertOnFalse` property.
        pub fn revert_on_false_property() -> StyledProperty<bool> {
            FerroProperty::register::<DataTriggerBehavior, _>("RevertOnFalse", false)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for DataTriggerBehavior {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            let property = change.property();

            if property == Self::binding_property().as_property() {
                this.on_value_changed(change);
            }

            if property == Self::comparison_condition_property().as_property() {
                this.on_value_changed(change);
            }

            if property == Self::value_property().as_property() {
                this.on_value_changed(change);
            }

            if property == Self::revert_on_false_property().as_property() {
                if change.get_old_value::<bool>().unwrap_or(false) && !change.get_new_value::<bool>() {
                    this.revert_actions(Some(&parameter_of(change)));
                }

                this.has_condition_state.set(false);
                this.on_value_changed(change);
            }

            if property == StyledElementBehavior::is_enabled_property().as_property() && this.revert_on_false() {
                let is_enabled = change.get_new_value::<bool>();
                if !is_enabled {
                    this.revert_actions(Some(&parameter_of(change)));
                }

                this.has_condition_state.set(false);
                if is_enabled {
                    this.on_value_changed(change);
                }
            }

            if property == StyledElementTrigger::actions_property().as_property() && this.associated_object().is_some() {
                this.update_action_subscription(change.get_new_value::<Option<ActionCollection>>());
                if this.revert_on_false() {
                    this.revert_actions(Some(&parameter_of(change)));
                    this.has_condition_state.set(false);
                    this.on_value_changed(change);
                }
            }
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for DataTriggerBehavior {
        fn on_initialized_event(this: &Self) {
            Self::parent_on_initialized_event(this);

            this.execute(None);
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
    }
}

/// The change as the parameter of the actions (the property changed event args of the reference).
fn parameter_of(change: &FerroPropertyChangedEventArgs<'_>) -> BoxedValue {
    Rc::new(change.to_owned_args())
}

impl DataTriggerBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementTrigger::construct(),
            is_condition_met: Cell::new(false),
            has_condition_state: Cell::new(false),
            applied_actions: RefCell::new(Vec::new()),
            subscribed_actions: RefCell::new(None),
        }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the bound object that the trigger will listen to. This is a property of the
    /// object model.
    pub fn binding(&self) -> Option<BoxedValue> {
        self.get_value(Self::binding_property())
    }

    pub fn set_binding(&self, value: Option<BoxedValue>) {
        self.set_value(Self::binding_property(), value)
    }

    /// Gets or sets the type of comparison to be performed between [`binding`](Self::binding) and
    /// [`value`](Self::value). This is a property of the object model.
    pub fn comparison_condition(&self) -> ComparisonConditionType {
        self.get_value(Self::comparison_condition_property())
    }

    pub fn set_comparison_condition(&self, value: ComparisonConditionType) {
        self.set_value(Self::comparison_condition_property(), value)
    }

    /// Gets or sets the value to be compared with the value of [`binding`](Self::binding). This is
    /// a property of the object model.
    pub fn value(&self) -> Option<BoxedValue> {
        self.get_value(Self::value_property())
    }

    /// Sets the value to be compared (named apart from the setter of every property of the object,
    /// as the condition of the object model names its own).
    pub fn set_trigger_value(&self, value: Option<BoxedValue>) {
        self.set_value(Self::value_property(), value)
    }

    /// Gets or sets a value indicating whether reversible actions should be reverted when the
    /// condition becomes false. When false, the trigger matches the legacy semantics and only
    /// executes actions when the condition is true.
    pub fn revert_on_false(&self) -> bool {
        self.get_value(Self::revert_on_false_property())
    }

    pub fn set_revert_on_false(&self, value: bool) {
        self.set_value(Self::revert_on_false_property(), value)
    }

    fn on_value_changed(&self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.sender().downcast_ref::<DataTriggerBehavior>().is_none() {
            return;
        }

        let behavior: Ref<DataTriggerBehavior> = self.to_ref();
        let parameter = parameter_of(change);
        Dispatcher::ui_thread().post_local(
            move || {
                behavior.execute(Some(&parameter));
            },
            DispatcherPriority::DEFAULT,
        );
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

        let binding = self.binding();
        if !self.is_set(Self::binding_property())
            || binding.as_ref().is_some_and(|binding| binding.is::<UnsetValueType>())
        {
            return;
        }

        if binding.is_none()
            && !matches!(self.comparison_condition(), ComparisonConditionType::Equal | ComparisonConditionType::NotEqual)
        {
            return;
        }

        let compare = || {
            ComparisonConditionTypeHelper::compare(binding.as_ref(), self.comparison_condition(), self.value().as_ref())
                .unwrap_or_else(|error| panic!("{error}"))
        };

        if !self.revert_on_false() {
            // Preserve legacy behavior: execute whenever condition evaluates true.
            if compare() {
                Interaction::execute_actions(Some(&associated_object), self.actions().as_ref(), parameter);
            }

            return;
        }

        let is_condition_met = compare();

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
        let behavior: Ref<DataTriggerBehavior> = self.to_ref();
        Dispatcher::ui_thread().post_local(
            move || {
                behavior.execute(Some(&parameter));
            },
            DispatcherPriority::DEFAULT,
        );
    }
}
