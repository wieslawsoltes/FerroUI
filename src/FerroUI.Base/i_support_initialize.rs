use crate::styling::DuplicateSetterError;
use crate::{ObjectType, Ref, StyledElement, Upcast};
use std::rc::Rc;

/// Why the initialization of an object could not be completed
/// ([`ISupportInitialize::try_end_init`]): what `EndInit` throws in the
/// managed original.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InitializationError {
    /// A style or control theme that applies to the element has two setters
    /// for the same property (an invalid operation).
    DuplicateSetter(DuplicateSetterError),
}

impl std::fmt::Display for InitializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateSetter(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for InitializationError {}

impl From<DuplicateSetterError> for InitializationError {
    fn from(error: DuplicateSetterError) -> Self {
        Self::DuplicateSetter(error)
    }
}

/// Specifies that an object supports a simple, transacted notification for
/// batch initialization: the counterpart of the component model contract of
/// the same name, which markup calls around the initialization of an
/// object.
pub trait ISupportInitialize {
    /// Signals the object that initialization is starting.
    fn begin_init(&self);

    /// Signals the object that initialization is complete.
    fn end_init(&self);

    /// Signals the object that initialization is complete, reporting a
    /// failure of the initialization (the exception `EndInit` raises in the
    /// managed original) instead of panicking. Markup calls this form. By
    /// default initialization cannot fail.
    fn try_end_init(&self) -> Result<(), InitializationError> {
        self.end_init();
        Ok(())
    }

    /// The identity of the object, used for reference equality.
    #[doc(hidden)]
    fn reference_id(&self) -> *const () {
        self as *const Self as *const ()
    }
}

/// Handles compare by identity (reference equality): two handles are equal
/// when they refer to the same object, whichever adapter they were made
/// from.
impl PartialEq for dyn ISupportInitialize {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.reference_id() == other.reference_id()
    }
}

/// Implements the contract for the styled element behind a class handle.
struct StyledElementInitialize(Ref<StyledElement>);

impl ISupportInitialize for StyledElementInitialize {
    fn begin_init(&self) {
        self.0.begin_init()
    }

    fn end_init(&self) {
        self.0.end_init()
    }

    fn try_end_init(&self) -> Result<(), InitializationError> {
        self.0.try_end_init()
    }

    fn reference_id(&self) -> *const () {
        &*self.0 as *const StyledElement as *const ()
    }
}

impl<T: ObjectType + Upcast<StyledElement>> From<Ref<T>> for Rc<dyn ISupportInitialize> {
    fn from(value: Ref<T>) -> Self {
        Rc::new(StyledElementInitialize(value.upcast()))
    }
}

impl<T: ObjectType + Upcast<StyledElement>> From<&Ref<T>> for Rc<dyn ISupportInitialize> {
    fn from(value: &Ref<T>) -> Self {
        Rc::new(StyledElementInitialize(value.clone().upcast()))
    }
}
