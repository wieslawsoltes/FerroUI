use super::{INameScope, NameScope, NameScopeError, NameScopeRef};
use crate::utilities::{SynchronousCompletionAsyncResult, SynchronousCompletionAsyncResultSource};
use crate::{FerroObject, Ref};

type FindResult = Option<Ref<FerroObject>>;

/// A name scope nested in another one: names that are not found once the
/// scope is complete are looked up in the parent scope.
pub struct ChildNameScope {
    parent_scope: NameScopeRef,
    inner: NameScope,
}

impl ChildNameScope {
    pub fn new(parent_scope: NameScopeRef) -> Self {
        Self { parent_scope, inner: NameScope::new() }
    }

    fn do_find_async(&self, name: &str) -> SynchronousCompletionAsyncResult<FindResult> {
        let src = SynchronousCompletionAsyncResultSource::<FindResult>::new();

        fn parent_search(
            parent_scope: &NameScopeRef,
            name: &str,
            src: SynchronousCompletionAsyncResultSource<FindResult>,
        ) {
            let parent_search = parent_scope.find_async(name);
            if parent_search.is_completed() {
                src.set_result(parent_search.get_result());
            } else {
                let search = parent_search.clone();
                parent_search.on_completed(move || src.set_result(search.get_result()));
            }
        }

        if !self.inner.is_completed() {
            // Guaranteed to be incomplete at this point.
            let inner_search = self.inner.find_async(name);
            let search = inner_search.clone();
            let src = src.clone();
            let parent_scope = self.parent_scope.clone();
            let name = name.to_string();
            inner_search.on_completed(move || {
                let value = search.get_result();
                if value.is_some() {
                    src.set_result(value);
                } else {
                    parent_search(&parent_scope, &name, src);
                }
            });
        } else {
            parent_search(&self.parent_scope, name, src.clone());
        }

        src.async_result()
    }
}

impl INameScope for ChildNameScope {
    fn try_register(&self, name: &str, element: Ref<FerroObject>) -> Result<(), NameScopeError> {
        self.inner.try_register(name, element)
    }

    fn find_async(&self, name: &str) -> SynchronousCompletionAsyncResult<FindResult> {
        let found = self.find(name);
        if found.is_some() {
            return SynchronousCompletionAsyncResult::new(found);
        }
        // Not found and both the current and the parent scope are completed.
        if self.is_completed() {
            return SynchronousCompletionAsyncResult::new(None);
        }
        self.do_find_async(name)
    }

    fn find(&self, name: &str) -> FindResult {
        let found = self.inner.find(name);
        if found.is_some() {
            return found;
        }
        if self.inner.is_completed() {
            return self.parent_scope.find(name);
        }
        None
    }

    fn complete(&self) {
        self.inner.complete()
    }

    fn is_completed(&self) -> bool {
        self.inner.is_completed() && self.parent_scope.is_completed()
    }

    fn attached_to(&self, owner: &Ref<FerroObject>) {
        self.inner.attached_to(owner)
    }

    fn detached_from(&self, owner: &Ref<FerroObject>) {
        self.inner.detached_from(owner)
    }
}
