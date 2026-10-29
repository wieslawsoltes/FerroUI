//! Not from the reference: the handle of the screens of a top level that the actions and triggers
//! of the folder hold in a property.
//!
//! A property value compares with `==`; the screens object of the framework does not, so the
//! properties hold it in a handle that compares by identity, as the reference compares its
//! object references.
//!
//! Deviation (DEVIATIONS.md, Behaviors): the properties typed `Screens` in the reference hold a
//! [`ScreensRef`].

use ferroui_controls::Screens;
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

/// The screens of a top level (`Screens`), compared by identity.
#[derive(Clone)]
pub struct ScreensRef(pub Rc<Screens>);

impl ScreensRef {
    /// The handle of `value`.
    pub fn new(value: Rc<Screens>) -> Self {
        Self(value)
    }

    /// The shared object.
    pub fn get(&self) -> &Rc<Screens> {
        &self.0
    }
}

impl Deref for ScreensRef {
    type Target = Screens;

    fn deref(&self) -> &Screens {
        &self.0
    }
}

impl PartialEq for ScreensRef {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl From<Rc<Screens>> for ScreensRef {
    fn from(value: Rc<Screens>) -> Self {
        Self(value)
    }
}

impl fmt::Debug for ScreensRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ScreensRef({:p})", Rc::as_ptr(&self.0))
    }
}
