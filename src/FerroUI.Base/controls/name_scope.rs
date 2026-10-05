use super::{INameScope, NameScopeError, NameScopeRef};
use crate::utilities::{SynchronousCompletionAsyncResult, SynchronousCompletionAsyncResultSource};
use crate::{ferro_property, AttachedProperty, FerroObject, FerroProperty, Ref, StyledElement};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

type FindResult = Option<Ref<FerroObject>>;

/// Implements a name scope.
#[derive(Default)]
pub struct NameScope {
    is_completed: Cell<bool>,
    inner: RefCell<HashMap<String, Ref<FerroObject>>>,
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
    pub fn set_name_scope(styled: &StyledElement, value: Option<NameScopeRef>) {
        styled.set_value(Self::name_scope_property(), value);
    }
}

impl INameScope for NameScope {
    fn try_register(&self, name: &str, element: Ref<FerroObject>) -> Result<(), NameScopeError> {
        if self.is_completed.get() {
            return Err(NameScopeError::Completed);
        }

        let existing = self.inner.borrow().get(name).cloned();
        match existing {
            Some(existing) => {
                if existing != element {
                    return Err(NameScopeError::DuplicateName(name.to_string()));
                }
            }
            None => {
                self.inner.borrow_mut().insert(name.to_string(), element.clone());
                let pending = self.pending_searches.borrow_mut().remove(name);
                if let Some(tcs) = pending {
                    tcs.set_result(Some(element));
                }
            }
        }
        Ok(())
    }

    fn find_async(&self, name: &str) -> SynchronousCompletionAsyncResult<FindResult> {
        let found = self.find(name);
        if found.is_some() {
            return SynchronousCompletionAsyncResult::new(found);
        }
        if self.is_completed.get() {
            return SynchronousCompletionAsyncResult::new(None);
        }
        // Continuations intentionally run synchronously here.
        self.pending_searches.borrow_mut().entry(name.to_string()).or_default().async_result()
    }

    fn find(&self, name: &str) -> FindResult {
        self.inner.borrow().get(name).cloned()
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
}
