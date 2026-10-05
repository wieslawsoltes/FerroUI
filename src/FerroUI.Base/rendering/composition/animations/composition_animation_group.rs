use super::{ICompositionAnimation, ICompositionAnimationBase};
use crate::rendering::composition::{AsCompositionObject, CompositionObject, Compositor};
use std::cell::RefCell;
use std::rc::Rc;

/// A group of animations started and stopped together on a composition
/// object, each on its target property.
pub struct CompositionAnimationGroup {
    object: CompositionObject,
    animations: RefCell<Vec<Rc<dyn ICompositionAnimation>>>,
}

impl CompositionAnimationGroup {
    pub fn new(compositor: &Rc<Compositor>) -> Rc<CompositionAnimationGroup> {
        Rc::new(CompositionAnimationGroup {
            object: CompositionObject::new(compositor, None),
            animations: RefCell::new(Vec::new()),
        })
    }

    /// The animations of the group, in the order they were added.
    pub(crate) fn animations(&self) -> Vec<Rc<dyn ICompositionAnimation>> {
        self.animations.borrow().clone()
    }

    pub fn add(&self, value: Rc<dyn ICompositionAnimation>) {
        self.animations.borrow_mut().push(value)
    }

    /// Removes the first occurrence of `value` (compared by reference).
    pub fn remove(&self, value: &Rc<dyn ICompositionAnimation>) {
        let mut animations = self.animations.borrow_mut();
        if let Some(index) = animations.iter().position(|a| std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(value))) {
            animations.remove(index);
        }
    }

    pub fn remove_all(&self) {
        self.animations.borrow_mut().clear()
    }
}

impl ICompositionAnimationBase for CompositionAnimationGroup {
    fn as_composition_animation_group(&self) -> Option<&CompositionAnimationGroup> {
        Some(self)
    }
}

impl AsCompositionObject for CompositionAnimationGroup {
    fn as_composition_object(&self) -> &CompositionObject {
        &self.object
    }

    fn composition_type_name(&self) -> &'static str {
        "CompositionAnimationGroup"
    }
}
