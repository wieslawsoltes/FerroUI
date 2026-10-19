//! `Registration<T>`: an instance or a factory.

use std::rc::Rc;

/// A registration: an instance, which may be `None` (upstream's `null`), or a factory
/// (`Registration<T>`).
pub(crate) enum Registration<T> {
    Instance(Option<T>),
    Factory(Rc<dyn Fn() -> Option<T>>),
}

impl<T: Clone> Clone for Registration<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Instance(instance) => Self::Instance(instance.clone()),
            Self::Factory(factory) => Self::Factory(factory.clone()),
        }
    }
}

/// Instances compare by value, factories by identity, as the fields of the upstream record struct
/// do.
impl<T: PartialEq> PartialEq for Registration<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Instance(a), Self::Instance(b)) => a == b,
            (Self::Factory(a), Self::Factory(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
            _ => false,
        }
    }
}

#[allow(dead_code)]
impl<T: Clone> Registration<T> {
    /// `FromInstance`.
    pub(crate) fn from_instance(instance: Option<T>) -> Self {
        Self::Instance(instance)
    }

    /// `FromFactory`.
    pub(crate) fn from_factory(factory: Rc<dyn Fn() -> Option<T>>) -> Self {
        Self::Factory(factory)
    }

    /// `IsFactory`.
    pub(crate) fn is_factory(&self) -> bool {
        matches!(self, Self::Factory(_))
    }

    /// `GetInstance`: the instance, or `None` for a factory.
    pub(crate) fn get_instance(&self) -> Option<T> {
        match self {
            Self::Instance(instance) => instance.clone(),
            Self::Factory(_) => None,
        }
    }

    /// `GetFactory`: the factory, or `None` for an instance.
    pub(crate) fn get_factory(&self) -> Option<Rc<dyn Fn() -> Option<T>>> {
        match self {
            Self::Instance(_) => None,
            Self::Factory(factory) => Some(factory.clone()),
        }
    }

    /// `TryGetInstance`: `Some` with the instance (which may be `None`) for an instance
    /// registration.
    pub(crate) fn try_get_instance(&self) -> Option<Option<T>> {
        match self {
            Self::Instance(instance) => Some(instance.clone()),
            Self::Factory(_) => None,
        }
    }

    /// `TryGetFactory`.
    pub(crate) fn try_get_factory(&self) -> Option<Rc<dyn Fn() -> Option<T>>> {
        self.get_factory()
    }

    /// The value of the registration: the instance, or what the factory returns
    /// (`ResolveLast` of the containers for one registration).
    pub(crate) fn resolve(&self) -> Option<T> {
        match self {
            Self::Instance(instance) => instance.clone(),
            Self::Factory(factory) => factory(),
        }
    }
}
