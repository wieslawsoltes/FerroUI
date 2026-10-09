use super::animations::{ICompositionAnimation, ICompositionAnimationBase, ImplicitAnimationCollection};
use super::expressions::ExpressionVariant;
use super::server::{CompositionProperty, ServerObjectId};
use super::{CompositionPropertySet, Compositor, ICompositorSerializable, PendingAnimations};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The part every composition object embeds: what upstream's abstract
/// `CompositionObject` holds.
///
/// A class "derives" from `CompositionObject` by embedding this value,
/// implementing [`ICompositorSerializable`] through it, and forwarding the
/// animation members to the functions of this module.
///
/// The server-side counterpart is known by id. Disposing the object
/// disposes its counterpart with the next batch, which keeps it under its id
/// while this object is alive: upstream the UI-thread object holds its
/// server object, so jobs and animations that name it after the disposal
/// still reach it, and the id cannot be reused meanwhile. When the object is
/// dropped, its counterpart is disposed (if it was not) and released with a
/// later batch: this stands in for the garbage collector, which reclaims a
/// server object upstream once the UI-thread object is unreachable. A
/// counterpart that was disposed out of band (a composition target) is kept
/// and released the same way.
pub struct CompositionObject {
    compositor: Rc<Compositor>,
    server: Option<ServerObjectId>,
    is_disposed: Cell<bool>,
    registered_for_serialization: Cell<bool>,
    pending_animations: PendingAnimations,
    implicit_animations: RefCell<Option<Rc<ImplicitAnimationCollection>>>,
}

impl CompositionObject {
    pub fn new(compositor: &Rc<Compositor>, server: Option<ServerObjectId>) -> Self {
        Self {
            compositor: compositor.clone(),
            server,
            is_disposed: Cell::new(false),
            registered_for_serialization: Cell::new(false),
            pending_animations: PendingAnimations::new(),
            implicit_animations: RefCell::new(None),
        }
    }

    /// The collection of implicit animations attached to this object.
    pub fn implicit_animations(&self) -> Option<Rc<ImplicitAnimationCollection>> {
        self.implicit_animations.borrow().clone()
    }

    pub fn set_implicit_animations(&self, value: Option<Rc<ImplicitAnimationCollection>>) {
        *self.implicit_animations.borrow_mut() = value;
    }

    /// The implicit animation attached to a property of the object, if
    /// there is one (`ImplicitAnimations?.TryGetValue(name)` of the
    /// generated setters).
    pub fn implicit_animation(&self, property_name: &str) -> Option<Rc<dyn ICompositionAnimationBase>> {
        let animations = self.implicit_animations.borrow().clone()?;
        animations.try_get_value(property_name)
    }

    pub fn compositor(&self) -> &Rc<Compositor> {
        &self.compositor
    }

    /// The id of the server-side counterpart, if the object has one.
    pub fn server(&self) -> Option<ServerObjectId> {
        self.server
    }

    /// The id of the server-side counterpart. Panics if there is none.
    pub fn required_server(&self) -> ServerObjectId {
        match self.server {
            Some(server) => server,
            None => panic!("There is no server-side counterpart for this object"),
        }
    }

    /// `ICompositorSerializable.TryGetServer`. A disposed object has no
    /// counterpart any more, so its pending changes are not written.
    pub fn try_get_server(&self, c: &Compositor) -> Option<ServerObjectId> {
        debug_assert!(std::ptr::eq(c, &*self.compositor));
        if self.is_disposed.get() {
            return None;
        }
        Some(self.required_server())
    }

    pub fn is_disposed(&self) -> bool {
        self.is_disposed.get()
    }

    pub fn pending_animations(&self) -> &PendingAnimations {
        &self.pending_animations
    }

    pub fn dispose(&self) {
        if !self.is_disposed.get() {
            if let Some(server) = self.server {
                self.compositor.dispose_and_keep_on_next_batch(server);
            }
        }
        self.is_disposed.set(true);
    }

    /// Marks the object disposed without queueing the disposal of its
    /// server side, which is disposed out of band
    /// ([`Compositor::oob_dispose`]). The server side stays under its id
    /// until this object is dropped, as after [`dispose`](Self::dispose).
    pub(crate) fn mark_disposed(&self) {
        self.is_disposed.set(true);
    }

    /// Queues the object for serialization. `this` yields the handle of the
    /// object that embeds this part.
    pub fn register_for_serialization(&self, this: impl FnOnce() -> Option<Rc<dyn ICompositorSerializable>>) {
        if self.server.is_none() {
            panic!("The object doesn't have an associated server counterpart");
        }
        if self.registered_for_serialization.get() || self.is_disposed.get() {
            return;
        }
        if let Some(this) = this() {
            self.registered_for_serialization.set(true);
            self.compositor.register_for_serialization(this);
        }
    }

    /// The head of `ICompositorSerializable.SerializeChanges`: the object
    /// may be registered again from now on.
    pub fn begin_serialize_changes(&self, c: &Compositor) {
        debug_assert!(std::ptr::eq(c, &*self.compositor));
        self.registered_for_serialization.set(false);
    }

    /// `StopAnimation`: removes the animation of a property on the server.
    /// `property` is the composition property the class resolved from the
    /// property name. The server object is reached through a job of the
    /// next batch.
    pub fn stop_animation(&self, property: &'static CompositionProperty) {
        let Some(server) = self.server else { return };
        self.compositor.post_server_job(
            move |compositor| {
                if let Some(object) = compositor.get_animated_object(server) {
                    if let Some(animations) = object.server_object().animations() {
                        animations.remove_animation_for_property(property);
                    }
                }
            },
            false,
        );
    }
}

impl Drop for CompositionObject {
    fn drop(&mut self) {
        if let Some(server) = self.server {
            if self.is_disposed.get() {
                self.compositor.release_with_a_later_batch(server);
            } else {
                self.compositor.dispose_with_a_later_batch(server);
            }
        }
    }
}

/// A composition object as code refers to any composition object: a
/// reference parameter of an animation, an object stored in a property set
/// (upstream `CompositionObject` as a parameter type).
pub trait AsCompositionObject: 'static {
    /// The embedded `CompositionObject`.
    fn as_composition_object(&self) -> &CompositionObject;

    /// The name of the class of the object, for messages.
    fn composition_type_name(&self) -> &'static str;

    /// The object as a property set, if it is one (the `is
    /// CompositionPropertySet` test of upstream).
    fn as_property_set(self: Rc<Self>) -> Option<Rc<CompositionPropertySet>> {
        None
    }
}

/// The animation members of `CompositionObject`, written against the class
/// that embeds one.
pub trait ICompositionObjectAnimations {
    /// `StartAnimation(propertyName, animation, finalValue)` of the class:
    /// returns `false` for a property the class does not know.
    fn try_start_animation(
        &self,
        property_name: &str,
        animation: &dyn ICompositionAnimation,
        final_value: Option<ExpressionVariant>,
    ) -> bool;

    /// The composition property with the given name.
    fn get_composition_property(&self, property_name: &str) -> Option<&'static CompositionProperty>;

    /// The embedded `CompositionObject`.
    fn composition_object(&self) -> &CompositionObject;

    /// Connects an animation with the named property of the object and
    /// starts the animation.
    fn start_animation(&self, property_name: &str, animation: &dyn ICompositionAnimation) {
        self.start_animation_with_final_value(property_name, animation, None)
    }

    fn start_animation_with_final_value(
        &self,
        property_name: &str,
        animation: &dyn ICompositionAnimation,
        final_value: Option<ExpressionVariant>,
    ) {
        if !self.try_start_animation(property_name, animation, final_value) {
            panic!("Unknown property {property_name}");
        }
    }

    /// Disconnects an animation from the named property and stops it.
    fn stop_animation(&self, property_name: &str) {
        let Some(property) = self.get_composition_property(property_name) else {
            panic!("Unknown property {property_name}");
        };
        self.composition_object().stop_animation(property);
    }

    /// Starts an animation group.
    /// The StartAnimationGroup method on CompositionObject lets you start CompositionAnimationGroup.
    /// All the animations in the group will be started at the same time on the object.
    fn start_animation_group(&self, grp: &dyn ICompositionAnimationBase) {
        if let Some(animation) = grp.as_composition_animation() {
            let Some(target) = animation.target() else { panic!("Animation Target can't be null") };
            self.start_animation(&target, animation);
        } else if let Some(group) = grp.as_composition_animation_group() {
            for a in group.animations() {
                let Some(target) = a.target() else { panic!("Animation Target can't be null") };
                self.start_animation(&target, &*a);
            }
        }
    }

    /// `StartAnimationGroupPart`.
    fn start_animation_group_part(
        &self,
        animation: &dyn ICompositionAnimation,
        target: &str,
        final_value: ExpressionVariant,
    ) -> bool {
        let Some(animation_target) = animation.target() else { panic!("Animation Target can't be null") };
        if animation_target == target {
            self.start_animation_with_final_value(&animation_target, animation, Some(final_value));
            true
        } else {
            self.start_animation(&animation_target, animation);
            false
        }
    }

    /// Starts an animation or an animation group triggered by the change
    /// of property `target` to `final_value`. Returns whether one of the
    /// animations targets that property.
    fn start_animation_group_for(
        &self,
        grp: &dyn ICompositionAnimationBase,
        target: &str,
        final_value: ExpressionVariant,
    ) -> bool {
        if let Some(animation) = grp.as_composition_animation() {
            return self.start_animation_group_part(animation, target, final_value);
        }
        if let Some(group) = grp.as_composition_animation_group() {
            let mut matched = false;
            for a in group.animations() {
                if a.target().is_none() {
                    panic!("Animation Target can't be null");
                }
                if self.start_animation_group_part(&*a, target, final_value) {
                    matched = true;
                }
            }
            return matched;
        }

        panic!("the animation is neither an animation nor a group of animations");
    }

    /// Stops an animation group.
    fn stop_animation_group(&self, grp: &dyn ICompositionAnimationBase) {
        if let Some(animation) = grp.as_composition_animation() {
            let Some(target) = animation.target() else { panic!("Animation Target can't be null") };
            self.stop_animation(&target);
        } else if let Some(group) = grp.as_composition_animation_group() {
            for a in group.animations() {
                let Some(target) = a.target() else { panic!("Animation Target can't be null") };
                self.stop_animation(&target);
            }
        }
    }
}
