//! Port of `Core/ChangePropertyAction.cs`.

use ferroui_base::data::core::{ValueTypes, WeakValue};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::helpers::member_lookup::object_of;
use ferroui_xaml_interactivity::helpers::{PropertyHelper, Setter, TrySetTemporaryValue, UntypedReversiblePropertyChange};
use ferroui_xaml_interactivity::{IAction, IReversibleAction, StyledElementAction, StyledElementActionImpl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// An action that will change a specified property to a specified value when invoked.
///
/// The property is found by name on the target: a registered property (an attached property is
/// named `(Owner.Name)`), else a plain property declared in the markup metadata of the type of the
/// target. Executed reversibly, the action can revert the change (see [`IReversibleAction`]).
#[repr(C)]
pub struct ChangePropertyAction {
    base: StyledElementAction,
    reversible_change: RefCell<Option<Rc<UntypedReversiblePropertyChange>>>,
    // Deviation (DEVIATIONS.md, Behaviors): the target the change applied to is held weakly when
    // it is an object (it is usually the host of the trigger, which holds the action).
    applied_target: RefCell<Option<WeakValue>>,
    applied_property_name: RefCell<Option<String>>,
    preserve_value_source: Rc<Cell<bool>>,
}

ferro_class!(ChangePropertyAction: StyledElementAction);
ferro_class_info!(ChangePropertyAction {
    new: ChangePropertyAction::new,
    markup: {
        property_attributes: [
            TargetObject: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(ChangePropertyAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ChangePropertyAction {
        /// Identifies the `PropertyName` property.
        pub fn property_name_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<ChangePropertyAction, _>("PropertyName", None)
        }

        /// Identifies the `TargetObject` property.
        pub fn target_object_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ChangePropertyAction, _>("TargetObject", None)
        }

        /// Identifies the `Value` property.
        pub fn value_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ChangePropertyAction, _>("Value", None)
        }
    }
}

impl StyledElementActionImpl for ChangePropertyAction {
    /// Executes the action.
    ///
    /// `sender` is the object that is passed to the action by the behavior. Generally this is the
    /// associated object of the behavior or a target object. The value of `parameter` is
    /// determined by the caller.
    ///
    /// Returns true if updating the property value succeeds; else false.
    ///
    /// # Panics
    /// Panics when the property is not found, is read-only or the value does not convert to its
    /// type (the argument exceptions of the reference).
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        Some(Rc::new(this.execute_core(sender, false)))
    }
}

/// Whether two targets are the same object (`ReferenceEquals`).
fn same_target(a: &BoxedValue, b: &BoxedValue) -> bool {
    match (object_of(a), object_of(b)) {
        (Some(a), Some(b)) => a.ptr_eq(&b),
        (None, None) => ValueTypes::identity_equals(Some(a), Some(b)),
        _ => false,
    }
}

impl ChangePropertyAction {
    /// The static constructor: the class implements the reversible action contract.
    fn static_constructor() {
        <dyn IReversibleAction>::register::<ChangePropertyAction>();
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementAction::construct(),
            reversible_change: RefCell::new(None),
            applied_target: RefCell::new(None),
            applied_property_name: RefCell::new(None),
            preserve_value_source: Rc::new(Cell::new(false)),
        }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the name of the property to change. This is a property of the object model.
    pub fn property_name(&self) -> Option<String> {
        self.get_value(Self::property_name_property())
    }

    pub fn set_property_name(&self, value: Option<String>) {
        self.set_value(Self::property_name_property(), value)
    }

    /// Gets or sets the value to set. This is a property of the object model.
    pub fn value(&self) -> Option<BoxedValue> {
        self.get_value(Self::value_property())
    }

    /// Sets the value to set (named apart from the setter of every property of the object, as the
    /// condition of the object model names its own).
    pub fn set_action_value(&self, value: Option<BoxedValue>) {
        self.set_value(Self::value_property(), value)
    }

    /// Gets or sets the object whose property will be changed. If it is not set or cannot be
    /// resolved, the sender of [`execute`](StyledElementActionImpl::execute) is used. Markup
    /// resolves a name given for it to the named element. This is a property of the object model.
    pub fn target_object(&self) -> Option<BoxedValue> {
        self.get_value(Self::target_object_property())
    }

    pub fn set_target_object(&self, value: Option<BoxedValue>) {
        self.set_value(Self::target_object_property(), value)
    }

    /// Executes the action while capturing the state required to revert it.
    pub fn execute_reversibly(&self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        Some(Rc::new(self.execute_core(sender, true)))
    }

    fn execute_core(&self, sender: Option<&BoxedValue>, preserve_value_source: bool) -> bool {
        if !self.is_enabled() {
            return false;
        }

        let target_object = match self.target_object() {
            Some(target_object) => Some(target_object),
            None => sender.cloned(),
        };
        let Some(target_object) = target_object else {
            return false;
        };

        let Some(property_name) = self.property_name() else {
            return false;
        };

        if !preserve_value_source {
            return PropertyHelper::update_property_value(&target_object, &property_name, self.value().as_ref(), false)
                .unwrap_or_else(|error| panic!("{error}"));
        }

        let existing_change = self.reversible_change.borrow().clone();
        if let Some(reversible_change) = existing_change {
            let applied_target = self.applied_target.borrow().as_ref().and_then(WeakValue::upgrade);
            let is_same_target = applied_target.as_ref().is_some_and(|applied| same_target(applied, &target_object));
            let is_same_property = self.applied_property_name.borrow().as_deref() == Some(property_name.as_str());
            if !is_same_target || !is_same_property {
                if !reversible_change.revert() {
                    return false;
                }

                self.clear_applied_state();
            }
        }

        let reversible_change = self
            .reversible_change
            .borrow_mut()
            .get_or_insert_with(|| Rc::new(UntypedReversiblePropertyChange::new(&property_name)))
            .clone();

        let preserve = self.preserve_value_source.clone();
        let try_get_value = {
            let target_object = target_object.clone();
            let property_name = property_name.clone();
            move || -> Option<Option<BoxedValue>> {
                match PropertyHelper::try_get_property_value(&target_object, &property_name) {
                    Ok(Some((value, captured_preserve_value_source))) => {
                        preserve.set(captured_preserve_value_source);
                        Some(value)
                    }
                    Ok(None) => None,
                    Err(error) => panic!("{error}"),
                }
            }
        };

        let set_value: Setter = {
            let target_object = target_object.clone();
            let property_name = property_name.clone();
            let preserve = self.preserve_value_source.clone();
            Rc::new(move |value: Option<&BoxedValue>| {
                PropertyHelper::update_property_value(&target_object, &property_name, value, preserve.get())
                    .unwrap_or_else(|error| panic!("{error}"))
            })
        };

        let set_temporary_value: TrySetTemporaryValue = {
            let target_object = target_object.clone();
            let property_name = property_name.clone();
            Rc::new(move |value: Option<&BoxedValue>| -> Option<Rc<dyn IDisposable>> {
                PropertyHelper::try_set_temporary_ferro_property_value(&target_object, &property_name, value)
                    .unwrap_or_else(|error| panic!("{error}"))
            })
        };

        let applied = reversible_change.apply(
            &target_object,
            self.value().as_ref(),
            &try_get_value,
            set_value,
            set_temporary_value,
            PropertyHelper::is_direct_ferro_property(&target_object, &property_name),
        );
        if applied {
            *self.applied_target.borrow_mut() = Some(WeakValue::new(&target_object));
            *self.applied_property_name.borrow_mut() = Some(property_name);
        }

        applied
    }

    /// Reverts the property value captured when this action was first applied.
    ///
    /// Returns true if reverting the property value succeeds; else false.
    pub fn revert(&self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let reversible_change = self.reversible_change.borrow().clone();
        match reversible_change {
            Some(reversible_change) if reversible_change.revert() => {
                self.clear_applied_state();
                Some(Rc::new(true))
            }
            _ => Some(Rc::new(false)),
        }
    }

    fn clear_applied_state(&self) {
        *self.reversible_change.borrow_mut() = None;
        *self.applied_target.borrow_mut() = None;
        *self.applied_property_name.borrow_mut() = None;
        self.preserve_value_source.set(false);
    }
}

impl IAction for ChangePropertyAction {
    fn execute(&self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        StyledElementAction::execute(self, sender, parameter)
    }
}

impl IReversibleAction for ChangePropertyAction {
    fn execute_reversibly(&self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        ChangePropertyAction::execute_reversibly(self, sender, parameter)
    }

    fn revert(&self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        ChangePropertyAction::revert(self, sender, parameter)
    }
}
