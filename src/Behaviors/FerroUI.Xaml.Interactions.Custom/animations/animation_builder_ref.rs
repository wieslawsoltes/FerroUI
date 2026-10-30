//! Not from the reference: the handle of an animation builder that the `AnimationBuilder`
//! properties of the animation behaviors, triggers and actions hold.
//!
//! Deviation (DEVIATIONS.md, Behaviors): a property value compares with `==`, and the contract
//! [`IAnimationBuilder`] of the animations crate does not; the properties typed
//! `IAnimationBuilder` in the reference hold this handle, which compares by identity as the
//! reference compares its interface references.

use ferroui_controls::Control;
use ferroui_xaml_behaviors_animations::IAnimationBuilder;
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

/// An animation builder ([`IAnimationBuilder`]) held by a property.
#[derive(Clone)]
pub struct AnimationBuilderRef(pub Rc<dyn IAnimationBuilder>);

impl AnimationBuilderRef {
    /// The handle of `builder`.
    pub fn new(builder: Rc<dyn IAnimationBuilder>) -> Self {
        Self(builder)
    }

    /// The shared builder.
    pub fn get(&self) -> &Rc<dyn IAnimationBuilder> {
        &self.0
    }
}

impl Deref for AnimationBuilderRef {
    type Target = dyn IAnimationBuilder;

    fn deref(&self) -> &(dyn IAnimationBuilder + 'static) {
        &*self.0
    }
}

impl IAnimationBuilder for AnimationBuilderRef {
    fn build(&self, control: &Control) -> Option<ferroui_base::Ref<ferroui_base::animation::Animation>> {
        self.0.build(control)
    }
}

impl PartialEq for AnimationBuilderRef {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(Rc::as_ptr(&self.0), Rc::as_ptr(&other.0))
    }
}

impl From<Rc<dyn IAnimationBuilder>> for AnimationBuilderRef {
    fn from(value: Rc<dyn IAnimationBuilder>) -> Self {
        Self(value)
    }
}

impl fmt::Debug for AnimationBuilderRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnimationBuilderRef({:p})", Rc::as_ptr(&self.0))
    }
}
