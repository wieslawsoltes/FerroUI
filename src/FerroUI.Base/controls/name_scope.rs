use super::{INameScope, NameScopeError, NameScopeRef, NameScopeValueError};
use crate::utilities::{SynchronousCompletionAsyncResult, SynchronousCompletionAsyncResultSource};
use crate::data::core::ValueTypes;
use crate::{BoxedValue, ferro_property, AttachedProperty, FerroObject, FerroProperty, Ref, StyledElement, WeakRef};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

type FindResult = Option<BoxedValue>;

/// An element of a name scope. Deviation (DEVIATIONS.md, Name scopes): the
/// element the scope is attached to holds the scope, so the scope holds that
/// one weakly: held as a reference, as
/// the managed original holds every element for its collector, an element
/// with a name of its own and its scope would keep each other alive. Every
/// other element is held as a reference.
#[derive(Clone)]
enum Named {
    Element(Ref<FerroObject>),
    Owner(WeakRef<FerroObject>),
    Value(BoxedValue),
}

impl Named {
    // Keep existing typed native lookup allocation-free.
    fn element(&self) -> Option<Ref<FerroObject>> {
        match self {
            Named::Element(element) => Some(element.clone()),
            Named::Owner(owner) => owner.upgrade(),
            Named::Value(_) => None,
        }
    }

    fn value(&self) -> FindResult {
        match self {
            Named::Element(element) => Some(std::rc::Rc::new(element.clone())),
            Named::Owner(owner) => owner.upgrade().map(|element| std::rc::Rc::new(element) as BoxedValue),
            Named::Value(value) => Some(value.clone()),
        }
    }

    fn is(&self, value: &BoxedValue) -> bool {
        match self {
            Named::Element(named) => ValueTypes::as_object(&**value).as_ref() == Some(named),
            Named::Owner(owner) => ValueTypes::as_object(&**value).is_some_and(|element| owner.points_to(&element)),
            Named::Value(named) => ValueTypes::reference_equals(named, value),
        }
    }
}

/// Implements a name scope.
#[derive(Default)]
pub struct NameScope {
    is_completed: Cell<bool>,
    inner: RefCell<HashMap<String, Named>>,
    // The elements the scope is attached to (`NameScope::set_name_scope`).
    owners: RefCell<Vec<WeakRef<FerroObject>>>,
    pending_searches: RefCell<HashMap<String, SynchronousCompletionAsyncResultSource<FindResult>>>,
}

crate::ferro_static_type!(NameScope);

crate::ferro_properties! { impl NameScope {
    ferro_property!(
        /// Defines the NameScope attached property.
        pub fn name_scope_property() -> AttachedProperty<Option<NameScopeRef>> {
            FerroProperty::register_attached::<NameScope, StyledElement, _>("NameScope", None)
        }
    );
} }

impl NameScope {
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the value of the attached NameScope property on a styled element.
    pub fn get_name_scope(styled: &StyledElement) -> Option<NameScopeRef> {
        styled.get_value(Self::name_scope_property())
    }

    /// Sets the value of the attached NameScope property on a styled element.
    ///
    /// The scope is told which element holds it, and the scope the element
    /// held before that it no longer does (see [`INameScope::attached_to`]).
    pub fn set_name_scope(styled: &StyledElement, value: Option<NameScopeRef>) {
        let owner = styled.to_ref().upcast::<FerroObject>();
        let previous = Self::get_name_scope(styled);
        if previous == value {
            return;
        }
        if let Some(previous) = &previous {
            previous.detached_from(&owner);
        }
        styled.set_value(Self::name_scope_property(), value.clone());
        if let Some(scope) = &value {
            scope.attached_to(&owner);
        }
    }

    fn is_owner(&self, element: &Ref<FerroObject>) -> bool {
        self.owners.borrow().iter().any(|owner| owner.points_to(element))
    }
}

impl INameScope for NameScope {
    fn try_register(&self, name: &str, element: Ref<FerroObject>) -> Result<(), NameScopeError> {
        // An element of the object model is neither null nor unsupported.
        self.try_register_value(name, std::rc::Rc::new(element)).map_err(|error| match error {
            NameScopeValueError::Completed => NameScopeError::Completed,
            NameScopeValueError::DuplicateName(name) => NameScopeError::DuplicateName(name),
            NameScopeValueError::UnsupportedValue | NameScopeValueError::NullValue => {
                unreachable!("an element of the object model is a supported, non-null value")
            }
        })
    }

    fn try_register_value(&self, name: &str, value: BoxedValue) -> Result<(), NameScopeValueError> {
        if self.is_completed.get() {
            return Err(NameScopeValueError::Completed);
        }

        let value = ValueTypes::normalize(value).ok_or(NameScopeValueError::NullValue)?;

        // A name whose element was dropped (the element the scope was
        // attached to) stays taken, as it is while the element is alive.
        let existing = self.inner.borrow().get(name).cloned();
        let existing = existing.map(|existing| existing.is(&value));
        match existing {
            Some(is_element) => {
                if !is_element {
                    return Err(NameScopeValueError::DuplicateName(name.to_string()));
                }
            }
            None => {
                let named = match ValueTypes::as_object(&*value) {
                    Some(element) if self.is_owner(&element) => Named::Owner(element.downgrade()),
                    Some(element) => Named::Element(element),
                    None => Named::Value(value.clone()),
                };
                self.inner.borrow_mut().insert(name.to_string(), named);
                let pending = self.pending_searches.borrow_mut().remove(name);
                if let Some(tcs) = pending {
                    tcs.set_result(Some(value));
                }
            }
        }
        Ok(())
    }

    fn find_async(&self, name: &str) -> SynchronousCompletionAsyncResult<Option<Ref<FerroObject>>> {
        if let Some(element) = self.find(name) {
            return SynchronousCompletionAsyncResult::new(Some(element));
        }
        if self.is_completed.get() || self.find_value(name).is_some() {
            return SynchronousCompletionAsyncResult::new(None);
        }
        super::i_name_scope::map_result(self.find_value_async(name), |value| value.and_then(|value| ValueTypes::as_object(&*value)))
    }

    fn find_value_async(&self, name: &str) -> SynchronousCompletionAsyncResult<FindResult> {
        let found = self.find_value(name);
        if found.is_some() {
            return SynchronousCompletionAsyncResult::new(found);
        }
        if self.is_completed.get() {
            return SynchronousCompletionAsyncResult::new(None);
        }
        // Continuations intentionally run synchronously here.
        self.pending_searches.borrow_mut().entry(name.to_string()).or_default().async_result()
    }

    fn find(&self, name: &str) -> Option<Ref<FerroObject>> {
        self.inner.borrow().get(name).and_then(Named::element)
    }

    fn find_value(&self, name: &str) -> FindResult {
        self.inner.borrow().get(name).and_then(Named::value)
    }

    fn complete(&self) {
        self.is_completed.set(true);
        let pending = std::mem::take(&mut *self.pending_searches.borrow_mut());
        for (_, tcs) in pending {
            tcs.try_set_result(None);
        }
    }

    fn is_completed(&self) -> bool {
        self.is_completed.get()
    }

    fn attached_to(&self, owner: &Ref<FerroObject>) {
        let mut owners = self.owners.borrow_mut();
        owners.retain(|known| known.upgrade().is_some());
        if !owners.iter().any(|known| known.points_to(owner)) {
            owners.push(owner.downgrade());
        }
        drop(owners);
        for named in self.inner.borrow_mut().values_mut() {
            if matches!(named, Named::Element(element) if element == owner) {
                *named = Named::Owner(owner.downgrade());
            }
        }
    }

    fn detached_from(&self, owner: &Ref<FerroObject>) {
        self.owners.borrow_mut().retain(|known| known.upgrade().is_some() && !known.points_to(owner));
        for named in self.inner.borrow_mut().values_mut() {
            if matches!(named, Named::Owner(known) if known.points_to(owner)) {
                *named = Named::Element(owner.clone());
            }
        }
    }
}
