use crate::animation::{Clock, IClock, ITransition, TransitionInstance, Transitions};
use crate::collections::{CollectionChangedHandler, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use crate::data::BindingPriority;
use crate::reactive::IDisposable;
use crate::{
    ferro_class, ferro_property, instantiate, BoxedValue, FerroObject, FerroObjectExtensions, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledProperty, StyledPropertyOptions,
    UnsetValueType,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Base class for all animatable objects.
#[repr(C)]
pub struct Animatable {
    base: FerroObject,
    transitions_enabled: Cell<bool>,
    is_subscribed_to_transitions_collection: Cell<bool>,
    transition_state: RefCell<Option<Vec<(Rc<dyn ITransition>, Rc<TransitionState>)>>>,
    /// The collections whose changes are being listened to, with the tokens
    /// of the subscriptions.
    collection_subscriptions: RefCell<Vec<(Transitions, u64)>>,
    /// The progress of the transition that is being applied right now, as
    /// reported by the transition while it binds its values.
    applying_transition_instance: RefCell<Option<Rc<TransitionInstance>>>,
}

struct TransitionState {
    instance: RefCell<Option<Rc<dyn IDisposable>>>,
    /// The progress behind `instance`, when the transition reported one.
    progress: RefCell<Option<Rc<TransitionInstance>>>,
    base_value: RefCell<BoxedValue>,
}

ferro_class!(Animatable: FerroObject);
crate::ferro_class_info!(Animatable { new: Animatable::new });

crate::ferro_overrides! { impl FerroObjectImpl for Animatable {
    /// Starts transitions when the base value of a property with a
    /// transition changes, and tracks the transitions collection.
    ///
    /// Classes deriving from `Animatable` must not override this member;
    /// override `on_property_changed` instead.
    fn on_property_changed_core(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.property() == Self::transitions_property().as_property() && change.is_effective_value_change() {
            let (old_transitions, new_transitions) = change.get_old_and_new_value::<Option<Transitions>>();

            // When transitions are replaced, we add the new transitions before removing the old
            // transitions, so that when the old transition being disposed causes the value to
            // change, there is a corresponding entry in the transition states. This means that we
            // need to account for any transitions present in both the old and new transitions
            // collections.
            if let Some(new_transitions) = &new_transitions {
                let mut to_add = new_transitions.snapshot();

                if let Some(old_transitions) = &old_transitions {
                    if new_transitions.count() > 0 && old_transitions.count() > 0 {
                        to_add = Rc::new(Self::except(&to_add, &old_transitions.snapshot()));
                    }
                }

                // Subscribe to collection changes only if transitions are already enabled,
                // i.e. control is attached to the visual tree
                if this.transitions_enabled.get() {
                    this.subscribe_to_collection(new_transitions);
                    this.is_subscribed_to_transitions_collection.set(true);
                }

                this.add_transitions(&to_add);
            }

            if let Some(old_transitions) = &old_transitions {
                let mut to_remove = old_transitions.snapshot();

                if let Some(new_transitions) = &new_transitions {
                    if old_transitions.count() > 0 && new_transitions.count() > 0 {
                        to_remove = Rc::new(Self::except(&to_remove, &new_transitions.snapshot()));
                    }
                }

                this.unsubscribe_from_collection(old_transitions);
                this.remove_transitions(&to_remove);
            }
        } else if this.transitions_enabled.get()
            && !change.property().is_direct()
            && change.priority() > BindingPriority::Animation
            && this.transition_state.borrow().is_some()
        {
            if let Some(transitions) = this.transitions() {
                let transitions = transitions.snapshot();
                for transition in transitions.iter().rev() {
                    let property = transition.property();
                    if property != change.property() {
                        continue;
                    }
                    let Some(state) = this.try_get_state(transition) else { continue };

                    let mut old_value = state.base_value.borrow().clone();
                    let new_value = this.get_animation_base_value(property);

                    if *old_value != *new_value {
                        *state.base_value.borrow_mut() = new_value.clone();

                        // We need to transition from the current animated value if present,
                        // instead of the old base value.
                        let animated_value = this.get_value_untyped(property);

                        if *new_value != *animated_value {
                            old_value = animated_value;
                        }

                        let clock = this.clock().unwrap_or_else(Clock::global_clock);
                        let instance = state.instance.borrow_mut().take();
                        *state.progress.borrow_mut() = None;
                        if let Some(instance) = instance {
                            instance.dispose();
                        }
                        *this.applying_transition_instance.borrow_mut() = None;
                        let instance = transition.apply(this, clock, &old_value, &new_value);
                        *state.progress.borrow_mut() = this.applying_transition_instance.borrow_mut().take();
                        *state.instance.borrow_mut() = Some(instance);
                        return;
                    }
                }
            }
        }

        Self::parent_on_property_changed_core(this, change);
    }
} }

crate::ferro_properties! { impl Animatable {
    ferro_property!(
        /// Defines the `Clock` property.
        pub fn clock_property() -> StyledProperty<Option<Rc<dyn IClock>>> {
            FerroProperty::register_with::<Animatable, _>("Clock", StyledPropertyOptions::new(None).inherits(true))
        }
    );

    ferro_property!(
        /// Defines the `Transitions` property.
        pub fn transitions_property() -> StyledProperty<Option<Transitions>> {
            FerroProperty::register::<Animatable, _>("Transitions", None)
        }
    );
} }

impl Animatable {
    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            transitions_enabled: Cell::new(true),
            is_subscribed_to_transitions_collection: Cell::new(false),
            transition_state: RefCell::new(None),
            collection_subscriptions: RefCell::new(Vec::new()),
            applying_transition_instance: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The clock which controls the animations and transitions on the
    /// object and, by inheritance, on its descendants. The global clock is
    /// used when there is none.
    pub fn clock(&self) -> Option<Rc<dyn IClock>> {
        self.get_value(Self::clock_property())
    }

    pub fn set_clock(&self, value: Option<Rc<dyn IClock>>) {
        self.set_value(Self::clock_property(), value)
    }

    /// The property transitions of the object.
    pub fn transitions(&self) -> Option<Transitions> {
        self.get_value(Self::transitions_property())
    }

    pub fn set_transitions(&self, value: Option<Transitions>) {
        self.set_value(Self::transitions_property(), value)
    }

    /// Enables transitions for the object.
    ///
    /// This is called when the object is attached to the visual tree, so
    /// that transitions do not run on its initial property values.
    pub fn enable_transitions(&self) {
        if !self.transitions_enabled.get() {
            self.transitions_enabled.set(true);

            if let Some(transitions) = self.transitions() {
                if !self.is_subscribed_to_transitions_collection.get() {
                    self.is_subscribed_to_transitions_collection.set(true);
                    self.subscribe_to_collection(&transitions);
                }

                self.add_transitions(&transitions.snapshot());
            }
        }
    }

    /// Disables transitions for the object.
    ///
    /// This is called when the object is detached from the visual tree.
    pub fn disable_transitions(&self) {
        if self.transitions_enabled.get() {
            self.transitions_enabled.set(false);

            if let Some(transitions) = self.transitions() {
                if self.is_subscribed_to_transitions_collection.get() {
                    self.is_subscribed_to_transitions_collection.set(false);
                    self.unsubscribe_from_collection(&transitions);
                }

                self.remove_transitions(&transitions.snapshot());
            }
        }
    }

    /// The items of `first` that are not in `second`, without duplicates.
    fn except(first: &[Rc<dyn ITransition>], second: &[Rc<dyn ITransition>]) -> Vec<Rc<dyn ITransition>> {
        let mut result: Vec<Rc<dyn ITransition>> = Vec::new();
        for item in first {
            if !second.contains(item) && !result.contains(item) {
                result.push(item.clone());
            }
        }
        result
    }

    fn subscribe_to_collection(&self, transitions: &Transitions) {
        // The collection must not keep the object alive: it is usually owned
        // by the object's own value store.
        let weak = self.to_ref().downgrade();
        let handler: Rc<CollectionChangedHandler<Rc<dyn ITransition>>> =
            Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, Rc<dyn ITransition>>| {
                if let Some(this) = weak.upgrade() {
                    this.transitions_collection_changed(e);
                }
            });
        let token = transitions.add_collection_changed(handler);
        self.collection_subscriptions.borrow_mut().push((transitions.clone(), token));
    }

    fn unsubscribe_from_collection(&self, transitions: &Transitions) {
        let token = {
            let mut subscriptions = self.collection_subscriptions.borrow_mut();
            subscriptions.iter().rposition(|(t, _)| t.ptr_eq(transitions)).map(|index| subscriptions.remove(index).1)
        };
        if let Some(token) = token {
            transitions.remove_collection_changed(token);
        }
    }

    fn transitions_collection_changed(&self, e: &NotifyCollectionChangedEventArgs<'_, Rc<dyn ITransition>>) {
        if !self.transitions_enabled.get() {
            return;
        }

        match e.action {
            NotifyCollectionChangedAction::Add => self.add_transitions(e.new_items),
            NotifyCollectionChangedAction::Remove => self.remove_transitions(e.old_items),
            NotifyCollectionChangedAction::Replace => {
                self.remove_transitions(e.old_items);
                self.add_transitions(e.new_items);
            }
            NotifyCollectionChangedAction::Reset => panic!("Transitions collection cannot be reset."),
            NotifyCollectionChangedAction::Move => {}
        }
    }

    fn add_transitions(&self, items: &[Rc<dyn ITransition>]) {
        if !self.transitions_enabled.get() {
            return;
        }

        for t in items {
            let base_value = self.get_animation_base_value(t.property());
            let mut transition_state = self.transition_state.borrow_mut();
            let transition_state = transition_state.get_or_insert_with(Vec::new);
            if transition_state.iter().any(|(key, _)| key == t) {
                panic!("An item with the same key has already been added.");
            }
            transition_state.push((
                t.clone(),
                Rc::new(TransitionState {
                    instance: RefCell::new(None),
                    progress: RefCell::new(None),
                    base_value: RefCell::new(base_value),
                }),
            ));
        }
        if self.transition_state.borrow().is_none() {
            *self.transition_state.borrow_mut() = Some(Vec::new());
        }
    }

    fn remove_transitions(&self, items: &[Rc<dyn ITransition>]) {
        if self.transition_state.borrow().is_none() {
            return;
        }

        for t in items {
            let Some(state) = self.try_get_state(t) else { continue };
            let instance = state.instance.borrow_mut().take();
            *state.progress.borrow_mut() = None;
            if let Some(instance) = instance {
                instance.dispose();
            }
            if let Some(transition_state) = &mut *self.transition_state.borrow_mut() {
                transition_state.retain(|(key, _)| key != t);
            }
        }
    }

    /// The progress of the transition that `transition` is currently
    /// running on this object: the instance created when the transition was
    /// last applied. `None` when the transition is not one of the object's
    /// transitions or has not been applied.
    pub fn try_get_transition_instance(&self, transition: &Rc<dyn ITransition>) -> Option<Rc<TransitionInstance>> {
        self.try_get_state(transition).and_then(|state| state.progress.borrow().clone())
    }

    /// Called by a transition while it is being applied, with the progress
    /// that drives the values it binds.
    pub(crate) fn report_transition_instance(&self, instance: &Rc<TransitionInstance>) {
        *self.applying_transition_instance.borrow_mut() = Some(instance.clone());
    }

    fn try_get_state(&self, transition: &Rc<dyn ITransition>) -> Option<Rc<TransitionState>> {
        let transition_state = self.transition_state.borrow();
        transition_state
            .as_ref()
            .and_then(|states| states.iter().find(|(key, _)| key == transition).map(|(_, state)| state.clone()))
    }

    fn get_animation_base_value(&self, property: &'static FerroProperty) -> BoxedValue {
        let value = self.get_base_value_untyped(property);

        if value.is::<UnsetValueType>() {
            self.get_value_untyped(property)
        } else {
            value
        }
    }
}
