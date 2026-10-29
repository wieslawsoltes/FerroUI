//! Port of `Validation/PropertyValidationBehavior.cs` (`PropertyValidationBehavior<TControl, TValue>`).

use super::contract::IValidationRule;
use ferroui_base::collections::{FerroList, NotifyCollectionChangedEventArgs};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::BindingMode;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, DirectProperty, FerroObject,
    FerroObjectImpl, FerroProperty, FerroPropertyChangedEventArgs, Ref, StaticType, StyledElementImpl,
    StyledProperty, StyledPropertyOptions, TypeInfo, WeakRef,
};
use ferroui_controls::{Control, DataValidationErrors};
use ferroui_xaml_interactivity::{
    DisposableAction, DisposingBehavior, DisposingBehaviorImpl, StyledElementBehaviorImpl, StyledElementBehaviorImplExt,
};
use std::any::TypeId;
use std::cell::{OnceCell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

/// The validation rules of a [`PropertyValidationBehavior`]: a list of objects that implement
/// [`IValidationRule`].
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference types the collection as the generic list of
/// the framework of `IValidationRule<TValue>`; here it is a named list of objects of the class
/// model (as `ActionCollection` of the interactivity crate), so that markup can add the rules of
/// a document to it; an object that does not implement the contract is skipped by the validation.
///
/// It is a reference object: clones refer to the same collection and compare by identity.
#[derive(Clone, PartialEq)]
pub struct ValidationRules(FerroList<Ref<FerroObject>>);

impl ValidationRules {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self(FerroList::new())
    }

    /// The list of the collection.
    pub fn list(&self) -> &FerroList<Ref<FerroObject>> {
        &self.0
    }
}

impl Default for ValidationRules {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for ValidationRules {
    type Target = FerroList<Ref<FerroObject>>;

    fn deref(&self) -> &FerroList<Ref<FerroObject>> {
        &self.0
    }
}

impl std::fmt::Debug for ValidationRules {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ValidationRules({})", self.0.count())
    }
}

/// The rules a behavior subscribed to, with the tokens of their handlers.
type SubscribedRules = RefCell<Vec<(Ref<FerroObject>, u64)>>;

/// The handler of the property changes of the rules.
type RulePropertyChanged = Rc<dyn Fn(&FerroPropertyChangedEventArgs<'_>)>;

/// The line separator of the joined error messages (`Environment.NewLine`).
const NEW_LINE: &str = if cfg!(windows) { "\r\n" } else { "\n" };

/// Base behavior that validates a property value using a set of rules.
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference class is generic over the type of the
/// associated control (`TControl`) and of the validated property (`TValue`); classes of the class
/// model are not generic, so both are runtime types given at creation
/// ([`new_of`](Self::new_of)`::<TControl, TValue>()`, [`construct`](Self::construct)): the associated
/// object is checked against the first as the reference does, and the property is validated only
/// when its value type is the second (`Property is FerroProperty<TValue>` of the reference).
/// The rules receive the value untyped (see [`IValidationRule`]).
///
/// Deviation (DEVIATIONS.md, Behaviors): markup cannot give a class of the object model type
/// arguments (`x:TypeArguments="TextBox, sys:String"`), so the behavior markup creates
/// ([`new`](Self::new)) is attached to any object and validates the property
/// whatever the type of its value: the `PropertyValidationBehavior<FerroObject, object>` of the
/// reference, where every property is a property of `object`.
#[repr(C)]
pub struct PropertyValidationBehavior {
    base: DisposingBehavior,
    /// The type of the value of the validated property, or `None` for any type.
    value_type: Option<TypeId>,
    rules: OnceCell<ValidationRules>,
}

ferro_class!(PropertyValidationBehavior: DisposingBehavior);
ferro_class_info!(PropertyValidationBehavior {
    new: PropertyValidationBehavior::new,
    markup: {
        content: Rules,
    },
});
ferroui_base::ferro_impl_classes!(PropertyValidationBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl PropertyValidationBehavior {
        /// Identifies the `Property` property.
        pub fn property_property() -> StyledProperty<Option<&'static FerroProperty>> {
            FerroProperty::register::<PropertyValidationBehavior, _>("Property", None)
        }

        /// Identifies the `IsValid` property.
        pub fn is_valid_property() -> StyledProperty<bool> {
            FerroProperty::register_with::<PropertyValidationBehavior, _>(
                "IsValid",
                StyledPropertyOptions::new(true).default_binding_mode(BindingMode::TwoWay),
            )
        }

        /// Identifies the `Error` property.
        pub fn error_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<PropertyValidationBehavior, _>("Error", None)
        }

        /// Identifies the `Rules` property.
        pub fn rules_property() -> DirectProperty<PropertyValidationBehavior, ValidationRules> {
            FerroProperty::register_direct::<PropertyValidationBehavior, _>(
                "Rules",
                |behavior| behavior.rules(),
                None,
                ValidationRules::new(),
            )
        }
    }
}

ferroui_base::ferro_overrides! {
    impl DisposingBehaviorImpl for PropertyValidationBehavior {
        fn on_attached_override(this: &Self) -> Rc<dyn IDisposable> {
            this.subscribe()
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for PropertyValidationBehavior {
        fn on_loaded(this: &Self) {
            Self::parent_on_loaded(this);

            this.validate();
        }
    }
}

impl PropertyValidationBehavior {
    /// Creates the class data of a behavior attached to objects of `associated_type` (the
    /// `TControl` of the reference) that validates a property whose value is of type `value_type`
    /// (the `TValue` of the reference); see [`ferroui_base::FerroObject::construct`].
    pub fn construct(associated_type: &'static TypeInfo, value_type: TypeId) -> Self {
        Self { base: DisposingBehavior::construct(associated_type), value_type: Some(value_type), rules: OnceCell::new() }
    }

    /// Creates the class data of a behavior attached to any object that validates a property
    /// whatever the type of its value (the behavior of markup, [`new`](Self::new)).
    pub fn construct_untyped() -> Self {
        Self { base: DisposingBehavior::construct(FerroObject::TYPE), value_type: None, rules: OnceCell::new() }
    }

    /// Creates a behavior attached to objects of class `TControl` that validates a property whose
    /// value is of type `TValue` (`new PropertyValidationBehavior<TControl, TValue>()`).
    pub fn new_of<TControl: StaticType, TValue: 'static>() -> Ref<Self> {
        instantiate(Self::construct(TControl::TYPE, TypeId::of::<TValue>()))
    }

    /// Creates a behavior attached to any object that validates a property whatever the type of
    /// its value (`new PropertyValidationBehavior<FerroObject, object>()` of the reference, the
    /// form markup creates: `<PropertyValidationBehavior Property="{x:Static TextBox.TextProperty}">`).
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct_untyped())
    }

    /// Gets or sets the property to validate. This is a property of the object model.
    pub fn property(&self) -> Option<&'static FerroProperty> {
        self.get_value(Self::property_property())
    }

    pub fn set_property(&self, value: Option<&'static FerroProperty>) {
        self.set_value(Self::property_property(), value)
    }

    /// Gets validation rules collection. This is a property of the object model, and the content
    /// of the behavior in markup.
    pub fn rules(&self) -> ValidationRules {
        self.rules.get_or_init(ValidationRules::new).clone()
    }

    /// Gets or sets value indicating whether the property value is valid. This is a property of
    /// the object model.
    pub fn is_valid(&self) -> bool {
        self.get_value(Self::is_valid_property())
    }

    pub fn set_is_valid(&self, value: bool) {
        self.set_value(Self::is_valid_property(), value)
    }

    /// Gets or sets the validation error message. This is a property of the object model.
    pub fn error(&self) -> Option<String> {
        self.get_value(Self::error_property())
    }

    pub fn set_error(&self, value: Option<String>) {
        self.set_value(Self::error_property(), value)
    }

    /// The property to validate, when its value is of the type the behavior validates.
    fn validated_property(&self) -> Option<&'static FerroProperty> {
        self.property().filter(|property| self.value_type.is_none_or(|value_type| property.property_type() == value_type))
    }

    /// `OnAttachedOverride` of the reference: validates when the property of the associated
    /// object, a rule or the collection of the rules changes.
    fn subscribe(&self) -> Rc<dyn IDisposable> {
        let Some(associated_object) = self.associated_object() else {
            return DisposableAction::empty();
        };

        let Some(property) = self.validated_property() else {
            return DisposableAction::empty();
        };

        let rules = self.rules();
        let behavior: WeakRef<PropertyValidationBehavior> = self.to_ref().downgrade();
        let subscribed_rules: Rc<SubscribedRules> = Rc::new(RefCell::new(Vec::new()));

        let rule_property_changed: RulePropertyChanged = {
            let behavior = behavior.clone();
            Rc::new(move |_e: &FerroPropertyChangedEventArgs<'_>| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.validate();
                }
            })
        };

        let rules_collection_changed = {
            let behavior = behavior.clone();
            let subscribed_rules = subscribed_rules.clone();
            let rule_property_changed = rule_property_changed.clone();
            move |_e: &NotifyCollectionChangedEventArgs<'_, Ref<FerroObject>>| {
                Self::detach_rules(&subscribed_rules);

                if let Some(behavior) = behavior.upgrade() {
                    for rule in behavior.rules().snapshot().iter() {
                        Self::attach_rule(&subscribed_rules, rule, &rule_property_changed);
                    }

                    behavior.validate();
                }
            }
        };

        let handler = {
            let behavior = behavior.clone();
            associated_object.property_changed(move |e: &FerroPropertyChangedEventArgs<'_>| {
                if e.property() == property {
                    if let Some(behavior) = behavior.upgrade() {
                        let value = e.to_owned_args().new_value().and_then(ValueTypes::normalize);
                        behavior.validate_value(value.as_ref());
                    }
                }
            })
        };
        let rules_token = rules.add_collection_changed(Rc::new(rules_collection_changed));

        for rule in rules.snapshot().iter() {
            Self::attach_rule(&subscribed_rules, rule, &rule_property_changed);
        }

        DisposableAction::create(move || {
            handler.dispose();
            rules.remove_collection_changed(rules_token);
            Self::detach_rules(&subscribed_rules);
        })
    }

    /// Subscribes to the property changes of `rule`, once.
    fn attach_rule(subscribed_rules: &SubscribedRules, rule: &Ref<FerroObject>, handler: &RulePropertyChanged) {
        let mut subscribed_rules = subscribed_rules.borrow_mut();
        if subscribed_rules.iter().any(|(subscribed, _)| subscribed.ptr_eq(rule)) {
            return;
        }

        let token = rule.add_property_changed_handler(handler.clone());
        subscribed_rules.push((rule.clone(), token));
    }

    /// Ends the subscriptions to the property changes of the rules.
    fn detach_rules(subscribed_rules: &SubscribedRules) {
        let subscribed = std::mem::take(&mut *subscribed_rules.borrow_mut());
        for (rule, token) in subscribed {
            rule.remove_property_changed_handler(token);
        }
    }

    /// Validates the current value of the property.
    fn validate(&self) {
        let (Some(associated_object), Some(property)) = (self.associated_object(), self.validated_property()) else {
            return;
        };

        let value = ValueTypes::normalize(associated_object.get_value_untyped(property));
        self.validate_value(value.as_ref());
    }

    /// Validates `value` with every rule, and publishes the result and the error messages (also
    /// as the data validation errors of the associated control).
    fn validate_value(&self, value: Option<&BoxedValue>) {
        let mut errors: Vec<String> = Vec::new();
        let mut valid = true;

        for rule in self.rules().snapshot().iter() {
            let Some(rule) = <dyn IValidationRule>::of(rule) else {
                continue;
            };

            if !rule.validate(value) {
                valid = false;
                if let Some(error_message) = rule.error_message().filter(|message| !message.is_empty()) {
                    errors.push(error_message);
                }
            }
        }

        self.set_is_valid(valid);
        self.set_error(if errors.is_empty() { None } else { Some(errors.join(NEW_LINE)) });

        if let Some(control) = self.associated_object_as::<Control>() {
            let errors: Option<Vec<BoxedValue>> = if errors.is_empty() {
                None
            } else {
                Some(errors.into_iter().map(|error| Rc::new(error) as BoxedValue).collect())
            };
            DataValidationErrors::set_errors(&control, errors);
        }
    }
}
