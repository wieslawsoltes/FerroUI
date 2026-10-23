use super::{CompositionProperty, IAnimatedServerObject, ServerCompositor, ServerObjectId};
use crate::rendering::composition::animations::IAnimationInstance;
use crate::rendering::composition::expressions::ExpressionVariant;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

struct ServerObjectSubscriptionStore {
    is_valid: Cell<bool>,
    /// The subscribed animations with their reference counts.
    subscribers: RefCell<Vec<(Rc<dyn IAnimationInstance>, u32)>>,
}

impl ServerObjectSubscriptionStore {
    fn new() -> Self {
        Self { is_valid: Cell::new(true), subscribers: RefCell::new(Vec::new()) }
    }

    fn invalidate(&self) {
        if !self.is_valid.get() {
            return;
        }
        self.is_valid.set(false);
        let subscribers: Vec<_> = self.subscribers.borrow().iter().map(|(s, _)| s.clone()).collect();
        for subscriber in subscribers {
            subscriber.invalidate();
        }
    }
}

struct ServerObjectAnimationInstance {
    property: &'static CompositionProperty,
    cached_variant: Cell<ExpressionVariant>,
    is_dirty: Cell<bool>,
    needs_update: Cell<bool>,
    animation: Rc<dyn IAnimationInstance>,
}

fn same_animation(a: &Rc<dyn IAnimationInstance>, b: &Rc<dyn IAnimationInstance>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

/// The animations of a server object and the animations of other objects
/// that depend on its properties.
pub struct ServerObjectAnimations {
    owner: Weak<dyn IAnimatedServerObject>,
    compositor: Weak<ServerCompositor>,
    subscriptions: RefCell<Vec<(i32, Rc<ServerObjectSubscriptionStore>)>>,
    animations: RefCell<Vec<Rc<ServerObjectAnimationInstance>>>,
}

impl ServerObjectAnimations {
    pub fn new(owner: Weak<dyn IAnimatedServerObject>, compositor: Weak<ServerCompositor>) -> Self {
        Self { owner, compositor, subscriptions: RefCell::new(Vec::new()), animations: RefCell::new(Vec::new()) }
    }

    fn owner_is_active(&self) -> bool {
        self.owner.upgrade().is_some_and(|owner| owner.server_object().is_active())
    }

    fn subscription(&self, property: &CompositionProperty) -> Option<Rc<ServerObjectSubscriptionStore>> {
        self.subscriptions.borrow().iter().find(|(id, _)| *id == property.id()).map(|(_, store)| store.clone())
    }

    fn animation(&self, property: &CompositionProperty) -> Option<Rc<ServerObjectAnimationInstance>> {
        self.animations.borrow().iter().find(|a| a.property.id() == property.id()).cloned()
    }

    fn get_variant(&self, instance: &ServerObjectAnimationInstance) -> ExpressionVariant {
        if !instance.is_dirty.get() {
            return instance.cached_variant.get();
        }
        // Set before evaluating the animation to prevent stack overflows
        // due to potential cyclic references.
        instance.is_dirty.set(false);
        // The time of the animations: the time of the compositor scaled by
        // the global playback rate.
        let now = self.compositor.upgrade().map_or(Duration::ZERO, |c| c.animations().animation_time(c.server_now()));
        let value = instance.animation.evaluate(now, instance.cached_variant.get());
        instance.cached_variant.set(value);
        value
    }

    fn update_target_property(&self, instance: &ServerObjectAnimationInstance) {
        if !instance.needs_update.get() {
            return;
        }
        instance.needs_update.set(false);
        let Some(owner) = self.owner.upgrade() else { return };
        let value = self.get_variant(instance);
        owner.write_property_variant(instance.property, value);
        owner.notify_animated_value_changed(instance.property);
        self.on_set_direct_value(instance.property);
    }

    pub fn activated(&self) {
        let animations = self.animations.borrow().clone();
        let any = !animations.is_empty();
        for instance in animations {
            instance.animation.activate();
            instance.is_dirty.set(true);
            instance.needs_update.set(true);
        }
        if any {
            self.schedule_evaluation();
        }
    }

    /// Queues the object for the evaluation of its dirty animations with the next frame.
    ///
    /// Not from upstream, where an animation is first evaluated when something it reads
    /// invalidates it, or its clock ticks: an animation that reads nothing (a constant
    /// expression, `"0.25"`) never wrote its property. WinUI applies it, so a started or
    /// activated animation is evaluated once (DEVIATIONS.md, WinUI composition).
    fn schedule_evaluation(&self) {
        let (Some(owner), Some(compositor)) = (self.owner.upgrade(), self.compositor.upgrade()) else { return };
        if let Some(this) = owner.server_object().animations() {
            compositor.animations().add_dirty_animated_object(this);
        }
    }

    pub fn deactivated(&self) {
        let animations = self.animations.borrow().clone();
        for instance in animations {
            instance.animation.deactivate();
        }
    }

    pub fn on_set_direct_value(&self, property: &CompositionProperty) {
        if let Some(subscriptions) = self.subscription(property) {
            subscriptions.invalidate();
        }
    }

    pub fn on_set_animated_value(
        &self,
        property: &'static CompositionProperty,
        current_value: ExpressionVariant,
        committed_at: Duration,
        animation: Rc<dyn IAnimationInstance>,
    ) {
        // Animations run on the clock of the animations, which the global
        // playback rate scales.
        let committed_at = self
            .compositor
            .upgrade()
            .map_or(committed_at, |c| c.animations().animation_time(committed_at));
        let is_active = self.owner_is_active();
        let old = self.animation(property);
        // An animation of components of the property (`Offset.Y`) joins the
        // animation of other components that runs already (`Offset.X`).
        if let Some(old) = &old {
            if let (Some(running), Some(started)) = (old.animation.as_subchannel(), animation.as_subchannel()) {
                if let Some(compositor) = self.compositor.upgrade() {
                    animation.resolve(&compositor);
                }
                running.merge(started, committed_at, current_value);
                self.on_set_direct_value(property);
                return;
            }
        }
        if let (true, Some(old)) = (is_active, &old) {
            old.animation.deactivate();
        }
        // A replaced animation is over for the batches it belongs to; the
        // new one starts from the current value (WinUI applies the stop
        // behavior on `StopAnimation` only).
        if let Some(old) = &old {
            old.animation.release_completion();
        }
        {
            let mut animations = self.animations.borrow_mut();
            animations.retain(|a| a.property.id() != property.id());
            animations.push(Rc::new(ServerObjectAnimationInstance {
                property,
                cached_variant: Cell::new(ExpressionVariant::default()),
                is_dirty: Cell::new(true),
                needs_update: Cell::new(true),
                animation: animation.clone(),
            }));
        }
        // The instance names the objects it works with by id; they are
        // resolved here, on the server, before it is initialized.
        if let Some(compositor) = self.compositor.upgrade() {
            animation.resolve(&compositor);
        }
        animation.initialize(committed_at, current_value, property);
        if is_active {
            animation.activate();
        }
        self.on_set_direct_value(property);
        if is_active {
            self.schedule_evaluation();
        }
    }

    pub fn remove_animation_for_property(&self, property: &CompositionProperty) {
        let removed = {
            let mut animations = self.animations.borrow_mut();
            let index = animations.iter().position(|a| a.property.id() == property.id());
            index.map(|index| animations.remove(index))
        };
        if let Some(removed) = removed {
            if self.owner_is_active() {
                removed.animation.deactivate();
            }
            removed.animation.release_completion();
        }
        self.on_set_direct_value(property);
    }

    /// `StopAnimation` of the property: removes its animation and applies
    /// the stop behavior of the animation (WinUI `AnimationStopBehavior`;
    /// upstream only removes the animation).
    pub fn stop_animation_for_property(&self, property: &'static CompositionProperty) {
        let removed = {
            let mut animations = self.animations.borrow_mut();
            let index = animations.iter().position(|a| a.property.id() == property.id());
            index.map(|index| animations.remove(index))
        };
        let Some(removed) = removed else { return };
        if self.owner_is_active() {
            removed.animation.deactivate();
        }
        if let Some(owner) = self.owner.upgrade() {
            let current = owner.read_property_variant(property);
            if let Some(value) = removed.animation.value_on_stop(current) {
                owner.write_property_variant(property, value);
                owner.notify_animated_value_changed(property);
            }
        }
        removed.animation.release_completion();
        self.on_set_direct_value(property);
    }

    /// `StopAnimation` of a sub-channel of the property (`Offset.X`): stops
    /// the animations of the sub-channels that share a component with
    /// `components` and applies their stop behavior. An animation of the
    /// whole property is left running.
    pub fn stop_subchannel_animation(&self, property: &'static CompositionProperty, components: &[usize]) {
        let Some(instance) = self.animation(property) else { return };
        // Deviation (winui-composition.md): stopping a sub-channel of a
        // property animated as a whole does nothing.
        let Some(subchannels) = instance.animation.as_subchannel() else { return };
        let Some(owner) = self.owner.upgrade() else { return };
        let current = owner.read_property_variant(property);
        let (value, empty) = subchannels.stop_components(components, current);
        if empty {
            self.animations.borrow_mut().retain(|a| a.property.id() != property.id());
        }
        if let Some(value) = value {
            instance.cached_variant.set(value);
            owner.write_property_variant(property, value);
            owner.notify_animated_value_changed(property);
        }
        self.on_set_direct_value(property);
    }

    /// Puts the animation of `property`, if it has one, under the
    /// animation controller whose property set has the id `controller`
    /// (`TryGetAnimationController` of an animation started without one).
    pub fn attach_controller(&self, property: &CompositionProperty, controller: ServerObjectId) {
        if let Some(instance) = self.animation(property) {
            instance.animation.attach_controller(controller);
        }
    }

    /// Releases the batches of every animation of the object: its
    /// composition object is disposed, which stops its animations for the
    /// batches (WinUI).
    pub fn release_completions(&self) {
        let animations = self.animations.borrow().clone();
        for instance in animations {
            instance.animation.release_completion();
        }
    }

    pub fn subscribe_to_invalidation(&self, member: &CompositionProperty, animation: Rc<dyn IAnimationInstance>) {
        let store = match self.subscription(member) {
            Some(store) => store,
            None => {
                let store = Rc::new(ServerObjectSubscriptionStore::new());
                self.subscriptions.borrow_mut().push((member.id(), store.clone()));
                store
            }
        };
        let mut subscribers = store.subscribers.borrow_mut();
        match subscribers.iter_mut().find(|(s, _)| same_animation(s, &animation)) {
            Some(entry) => entry.1 += 1,
            None => subscribers.push((animation, 1)),
        }
    }

    pub fn unsubscribe_from_invalidation(&self, member: &CompositionProperty, animation: &Rc<dyn IAnimationInstance>) {
        if let Some(store) = self.subscription(member) {
            let mut subscribers = store.subscribers.borrow_mut();
            if let Some(index) = subscribers.iter().position(|(s, _)| same_animation(s, animation)) {
                subscribers[index].1 -= 1;
                if subscribers[index].1 == 0 {
                    subscribers.remove(index);
                }
            }
        }
    }

    pub fn get_property_for_animation(&self, name: &str) -> ExpressionVariant {
        let Some(owner) = self.owner.upgrade() else { return ExpressionVariant::default() };
        let Some(property) = owner.get_composition_property(name) else { return ExpressionVariant::default() };
        if let Some(subscriptions) = self.subscription(property) {
            subscriptions.is_valid.set(true);
        }
        if let Some(animation) = self.animation(property) {
            return self.get_variant(&animation);
        }
        owner.read_property_variant(property)
    }

    pub fn evaluate_animations(&self) {
        let animations = self.animations.borrow().clone();
        for animation in animations {
            if animation.is_dirty.get() {
                self.update_target_property(&animation);
            }
        }
    }

    /// `this` is the handle of these animations.
    pub fn notify_animation_instance_invalidated(this: &Rc<Self>, property: &CompositionProperty) {
        match this.animation(property) {
            Some(instance) => {
                instance.is_dirty.set(true);
                instance.needs_update.set(true);
                if let Some(compositor) = this.compositor.upgrade() {
                    compositor.animations().add_dirty_animated_object(this.clone());
                }
            }
            None => debug_assert!(false, "the property has no animation"),
        }
    }
}
