use crate::utilities::SynchronousCompletionAsyncResult;
use crate::{FerroObject, Ref};
use std::ops::Deref;
use std::rc::Rc;

/// Why an element could not be registered in a name scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NameScopeError {
    /// The name scope is completed: no further registrations are allowed.
    Completed,
    /// Another element is registered with the name.
    DuplicateName(String),
}

impl std::fmt::Display for NameScopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Completed => f.write_str("NameScope is completed, no further registrations are allowed"),
            Self::DuplicateName(name) => write!(f, "Control with the name '{name}' already registered."),
        }
    }
}

impl std::error::Error for NameScopeError {}

/// Defines a name scope.
pub trait INameScope {
    /// Registers an element in the name scope. Panics if the scope is
    /// completed or another element is registered with the name: see
    /// [`try_register`](Self::try_register) for the form that reports these
    /// as errors.
    fn register(&self, name: &str, element: Ref<FerroObject>) {
        if let Err(error) = self.try_register(name, element) {
            panic!("{error}");
        }
    }

    /// Registers an element in the name scope, reporting a completed scope
    /// and a name that another element is registered with as errors.
    /// Registering an element again with its name does nothing.
    fn try_register(&self, name: &str, element: Ref<FerroObject>) -> Result<(), NameScopeError>;

    /// Finds a named element in the name scope, waits for the scope to be
    /// completely populated before returning `None`. Returned results always
    /// complete synchronously.
    fn find_async(&self, name: &str) -> SynchronousCompletionAsyncResult<Option<Ref<FerroObject>>>;

    /// Finds a named element in the name scope, returns immediately, doesn't
    /// traverse the name scope stack.
    fn find(&self, name: &str) -> Option<Ref<FerroObject>>;

    /// Marks the name scope as completed, no further registrations will be
    /// allowed.
    fn complete(&self);

    /// Whether the name scope is completed, no further registrations are
    /// allowed.
    fn is_completed(&self) -> bool;
}

/// A shared handle to a name scope, compared by identity so that it can be a
/// property value.
#[derive(Clone)]
pub struct NameScopeRef(pub Rc<dyn INameScope>);

impl NameScopeRef {
    pub fn new(scope: impl INameScope + 'static) -> Self {
        NameScopeRef(Rc::new(scope))
    }
}

impl Deref for NameScopeRef {
    type Target = dyn INameScope;

    fn deref(&self) -> &(dyn INameScope + 'static) {
        &*self.0
    }
}

impl PartialEq for NameScopeRef {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(Rc::as_ptr(&self.0), Rc::as_ptr(&other.0))
    }
}

impl From<Rc<dyn INameScope>> for NameScopeRef {
    fn from(value: Rc<dyn INameScope>) -> Self {
        NameScopeRef(value)
    }
}

/// Handles compare by identity (reference equality), so that they can be
/// held in property and untyped values.
impl PartialEq for dyn INameScope {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
