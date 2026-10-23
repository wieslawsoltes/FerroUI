use super::{IAnimationInstance, InitialValueExpressionCollection, PropertySetSnapshot};
use crate::rendering::composition::expressions::{
    BuiltInExpressionFfi, Expression, ExpressionKeywords, ExpressionObjectKey, ExpressionVariant, IExpressionObject,
};
use crate::rendering::composition::server::{
    CompositionProperty, IAnimatedServerObject, IServerAnimatedPropertyHost, IServerClockItem, ServerCompositor,
    ServerExpressionObject, ServerObjectAnimations, ServerObjectId,
};
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::{Rc, Weak};

/// The base class for both key-frame and expression animation instances
/// Is responsible for activation tracking and for subscribing to properties used in dependencies
///
/// A class "derives" from it by embedding it and forwarding the members of
/// [`IAnimationInstance`] it does not override. The embedding class is
/// created with `Rc::new_cyclic` and hands the weak handle of itself to the
/// base, which subscribes it to the objects it tracks.
///
/// The target and the tracked objects are held weakly: the animations of
/// the target hold the instance, and the instance must not keep the target
/// alive in turn.
pub struct AnimationInstanceBase {
    this: Weak<dyn IAnimationInstance>,
    tracked_objects: RefCell<Option<Vec<(Weak<dyn IAnimatedServerObject>, &'static CompositionProperty)>>>,
    parameters: Rc<PropertySetSnapshot>,
    target_object: ServerObjectId,
    target: RefCell<Option<Weak<dyn IAnimatedServerObject>>>,
    compositor: RefCell<Weak<ServerCompositor>>,
    property: Cell<Option<&'static CompositionProperty>>,
    invalidated: Cell<bool>,
}

impl AnimationInstanceBase {
    pub fn new(this: Weak<dyn IAnimationInstance>, target: ServerObjectId, parameters: Rc<PropertySetSnapshot>) -> Self {
        Self {
            this,
            tracked_objects: RefCell::new(None),
            parameters,
            target_object: target,
            target: RefCell::new(None),
            compositor: RefCell::new(Weak::new()),
            property: Cell::new(None),
            invalidated: Cell::new(false),
        }
    }

    pub fn parameters(&self) -> &Rc<PropertySetSnapshot> {
        &self.parameters
    }

    /// The id of the object whose property is animated.
    pub fn target_object(&self) -> ServerObjectId {
        self.target_object
    }

    /// The object whose property is animated, once resolved and while it is
    /// alive.
    pub fn target(&self) -> Option<Rc<dyn IAnimatedServerObject>> {
        self.target.borrow().as_ref().and_then(Weak::upgrade)
    }

    /// The compositor of the target (`TargetObject.Compositor`), once
    /// resolved and while it is alive.
    pub fn compositor(&self) -> Option<Rc<ServerCompositor>> {
        self.compositor.borrow().upgrade()
    }

    /// The property the instance animates, once initialized.
    pub fn property(&self) -> Option<&'static CompositionProperty> {
        self.property.get()
    }

    /// The handle of the embedding instance as a clock item.
    pub fn this_clock_item(&self) -> Option<Rc<dyn IServerClockItem>> {
        self.this.upgrade().map(|this| this as Rc<dyn IServerClockItem>)
    }

    /// See [`IAnimationInstance::resolve`]: resolves the target and the
    /// objects of the reference parameters on `compositor`.
    ///
    /// Upstream the instance holds its target. The target is the object
    /// whose batch changes carry the instance, so its id names it here; an
    /// id that names no animatable object leaves the instance without a
    /// target, which it then treats as gone.
    pub fn resolve(&self, compositor: &Rc<ServerCompositor>) {
        *self.compositor.borrow_mut() = Rc::downgrade(compositor);
        *self.target.borrow_mut() =
            compositor.get_animated_object(self.target_object).map(|target| Rc::downgrade(&target));
        self.parameters.resolve(compositor);
    }

    /// Calls `f` with the target as the expressions of the instance see it.
    pub fn with_target_expression_object<R>(&self, f: impl FnOnce(Option<&dyn IExpressionObject>) -> R) -> R {
        match self.target() {
            Some(target) => {
                let target = ServerExpressionObject(target);
                f(Some(&target))
            }
            None => f(None),
        }
    }

    /// `Initialize(property, trackedObjects)`.
    pub fn initialize(&self, property: &'static CompositionProperty, tracked_objects: &HashSet<(String, String)>) {
        if !tracked_objects.is_empty() {
            let mut tracked_list = Vec::new();
            for (name, member) in tracked_objects {
                let obj = if name == ExpressionKeywords::TARGET {
                    self.target()
                } else {
                    self.parameters.get_server_object_parameter(name)
                };
                if let Some(tracked) = obj {
                    let Some(off) = tracked.get_composition_property(member) else {
                        if cfg!(debug_assertions) {
                            panic!("Attempting to subscribe to unknown field");
                        }
                        continue;
                    };
                    tracked_list.push((Rc::downgrade(&tracked), off));
                }
            }
            *self.tracked_objects.borrow_mut() = Some(tracked_list);
        }

        self.property.set(Some(property));
    }

    /// The head of `Evaluate`: the evaluation makes the instance valid
    /// again. The embedding class then evaluates its own core.
    pub fn begin_evaluate(&self) {
        self.invalidated.set(false);
    }

    fn tracked(&self) -> Vec<(Rc<dyn IAnimatedServerObject>, &'static CompositionProperty)> {
        self.tracked_objects
            .borrow()
            .iter()
            .flatten()
            .filter_map(|(obj, member)| obj.upgrade().map(|obj| (obj, *member)))
            .collect()
    }

    pub fn activate(&self) {
        let Some(this) = self.this.upgrade() else { return };
        for (obj, member) in self.tracked() {
            obj.server_object().get_or_create_animations().subscribe_to_invalidation(member, this.clone());
        }
    }

    pub fn deactivate(&self) {
        let Some(this) = self.this.upgrade() else { return };
        for (obj, member) in self.tracked() {
            if let Some(animations) = obj.server_object().animations() {
                animations.unsubscribe_from_invalidation(member, &this);
            }
        }
    }

    pub fn invalidate(&self) {
        if self.invalidated.get() {
            return;
        }
        self.invalidated.set(true);
        let (Some(target), Some(property)) = (self.target(), self.property.get()) else { return };
        if let Some(animations) = target.server_object().animations() {
            ServerObjectAnimations::notify_animation_instance_invalidated(&animations, property);
        }
    }

    pub fn on_tick(&self) {
        self.invalidate()
    }

    /// Applies the initial value expressions of the animation
    /// (`CompositionAnimation.InitialValueExpressions`) as it starts: each
    /// expression is evaluated against the target and the parameters, and the
    /// named property of the target is set to the result. Returns the value
    /// the animated `property` starts from: `starting_value`, or the initial
    /// value of the property when an expression names it.
    ///
    /// Called by the instance when it is initialized, after
    /// [`resolve`](Self::resolve); the expressions read the animated
    /// property as `starting_value` (the instance is not initialized yet, so
    /// it cannot be evaluated).
    pub fn apply_initial_values(
        &self,
        expressions: &[(String, Expression)],
        property: &'static CompositionProperty,
        starting_value: ExpressionVariant,
    ) -> ExpressionVariant {
        if expressions.is_empty() {
            return starting_value;
        }
        let Some(target) = self.target() else { return starting_value };
        let reader = StartingTarget {
            target: ServerExpressionObject(target.clone()),
            property_name: property.name(),
            starting_value,
        };
        let values = InitialValueExpressionCollection::evaluate_parsed(
            expressions,
            &reader,
            Some(&**self.parameters()),
            BuiltInExpressionFfi::instance(),
        );
        let mut starting = starting_value;
        for (name, value) in values {
            let Some(initial) = target.get_composition_property(&name) else { continue };
            target.write_property_variant(initial, value);
            target.notify_animated_value_changed(initial);
            if let Some(animations) = target.server_object().animations() {
                animations.on_set_direct_value(initial);
            }
            if initial.id() == property.id() {
                // Read back: the value in the type of the property.
                starting = target.read_property_variant(initial);
            }
        }
        starting
    }
}

/// The target of a starting animation as its initial value expressions read
/// it: the animated property is the value it has before the animation
/// starts.
struct StartingTarget {
    target: ServerExpressionObject,
    property_name: &'static str,
    starting_value: ExpressionVariant,
}

impl IExpressionObject for StartingTarget {
    fn get_property(&self, name: &str) -> ExpressionVariant {
        if name == self.property_name {
            self.starting_value
        } else {
            self.target.get_property(name)
        }
    }

    fn key(&self) -> ExpressionObjectKey {
        self.target.key()
    }
}

