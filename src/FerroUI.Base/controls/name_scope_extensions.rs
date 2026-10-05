use super::{INameScope, NameScope, NameScopeRef};
use crate::{ObjectType, Ref, StyledElement};

impl dyn INameScope + '_ {
    /// Finds a named element in the name scope and casts it to class `T`.
    ///
    /// Returns `None` if the name is not registered; panics if the element is
    /// of a different class.
    pub fn find_as<T: ObjectType>(&self, name: &str) -> Option<Ref<T>> {
        let result = self.find(name)?;
        match result.cast::<T>() {
            Some(typed) => Some(typed),
            None => panic!(
                "Expected control '{name}' to be '{}' but it was '{}'.",
                T::TYPE.name(),
                result.get_type().name()
            ),
        }
    }

    /// Gets a named element from the name scope and casts it to class `T`.
    ///
    /// Panics if the name is not registered or the element is of a different
    /// class.
    pub fn get_as<T: ObjectType>(&self, name: &str) -> Ref<T> {
        match self.find_as::<T>(name) {
            Some(result) => result,
            None => panic!("Could not find control '{name}'."),
        }
    }
}

/// Name scope lookups anchored at a styled element.
pub struct NameScopeExtensions;

impl NameScopeExtensions {
    /// Finds a named element in the name scope attached to `anchor`.
    pub fn find<T: ObjectType>(anchor: &StyledElement, name: &str) -> Option<Ref<T>> {
        NameScope::get_name_scope(anchor)?.find_as::<T>(name)
    }

    /// Gets a named element from the name scope attached to `anchor`.
    ///
    /// Panics if the element has no name scope, the name is not registered or
    /// the named element is of a different class.
    pub fn get<T: ObjectType>(anchor: &StyledElement, name: &str) -> Ref<T> {
        match NameScope::get_name_scope(anchor) {
            Some(scope) => scope.get_as::<T>(name),
            None => panic!(
                "The control doesn't have an associated name scope, probably no registrations has been done yet"
            ),
        }
    }

    /// Finds the name scope of `control` by walking up its logical ancestors.
    pub fn find_name_scope(control: &StyledElement) -> Option<NameScopeRef> {
        let mut current = Some(control.to_ref());
        while let Some(element) = current {
            if let Some(scope) = NameScope::get_name_scope(&element) {
                return Some(scope);
            }
            current = element.parent();
        }
        None
    }
}
