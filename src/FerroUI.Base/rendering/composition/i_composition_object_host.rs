//! The seam between the generated property blocks of composition objects
//! and the hand-written classes that embed them.

use super::animations::{IAnimationInstance, ICompositionAnimationBase};
use super::expressions::ExpressionVariant;
use super::server::{CompositionProperty, ServerObjectId};
use super::transport::IRegisterForSerialization;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// A UI-thread composition object as other composition objects refer to
/// it: something with a server-side counterpart.
pub trait ICompositionObject: 'static {
    /// The id of the server-side counterpart of the object.
    fn server(&self) -> ServerObjectId;

    fn as_any(&self) -> &dyn Any;

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any>;
}

/// The animation instances created on the UI thread that have not been
/// sent to the server yet, by the property they animate.
#[derive(Default)]
pub struct PendingAnimations {
    items: RefCell<Vec<(i32, Rc<dyn IAnimationInstance>)>>,
}

impl PendingAnimations {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn count(&self) -> usize {
        self.items.borrow().len()
    }

    /// Drops the pending animation of a property. Returns whether there
    /// was one.
    pub fn remove(&self, property: &CompositionProperty) -> bool {
        self.get_and_remove(property).is_some()
    }

    /// Sets the pending animation of a property, replacing the previous
    /// one.
    pub fn set(&self, property: &CompositionProperty, animation: Rc<dyn IAnimationInstance>) {
        let mut items = self.items.borrow_mut();
        match items.iter_mut().find(|(id, _)| *id == property.id()) {
            Some(entry) => entry.1 = animation,
            None => items.push((property.id(), animation)),
        }
    }

    /// Takes the pending animation of a property.
    pub fn get_and_remove(&self, property: &CompositionProperty) -> Option<Rc<dyn IAnimationInstance>> {
        let mut items = self.items.borrow_mut();
        let index = items.iter().position(|(id, _)| *id == property.id())?;
        Some(items.remove(index).1)
    }
}

/// What a generated client property block needs from the composition
/// object that embeds it (the members of upstream `CompositionObject` the
/// generated code uses).
pub trait ICompositionObjectHost: IRegisterForSerialization {
    /// The id of the server-side counterpart of the object.
    fn server(&self) -> ServerObjectId;

    /// The animations waiting to be sent with the next batch.
    fn pending_animations(&self) -> &PendingAnimations;

    /// The implicit animation attached to a property of the object, if
    /// there is one (`ImplicitAnimations?.TryGetValue(name)`).
    fn implicit_animation(&self, property_name: &str) -> Option<Rc<dyn ICompositionAnimationBase>>;

    /// Starts an animation or an animation group triggered by the change
    /// of property `target` to `final_value`. Returns whether one of the
    /// animations targets that property.
    fn start_animation_group(
        &self,
        grp: &Rc<dyn ICompositionAnimationBase>,
        target: &str,
        final_value: ExpressionVariant,
    ) -> bool;
}
