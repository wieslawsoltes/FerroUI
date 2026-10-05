//! Port of `XamlIl/Runtime/XamlIlParentStackProviderWrapper.cs`.

use super::{IFerroXamlIlEagerParentStackProvider, IFerroXamlIlParentStackProvider};
use ferroui_base::BoxedValue;
use std::cell::RefCell;
use std::rc::Rc;

/// Presents a parent stack provider that only enumerates as an eager one.
pub(crate) struct XamlIlParentStackProviderWrapper {
    provider: Rc<dyn IFerroXamlIlParentStackProvider>,
    direct_parents_stack: RefCell<Option<Rc<Vec<BoxedValue>>>>,
}

impl XamlIlParentStackProviderWrapper {
    pub(crate) fn new(provider: Rc<dyn IFerroXamlIlParentStackProvider>) -> Rc<Self> {
        Rc::new(Self { provider, direct_parents_stack: RefCell::new(None) })
    }
}

impl IFerroXamlIlParentStackProvider for XamlIlParentStackProviderWrapper {
    fn parents(&self) -> Vec<BoxedValue> {
        self.provider.parents()
    }

    fn as_eager_parent_stack_provider(self: Rc<Self>) -> Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>> {
        Some(self)
    }
}

impl IFerroXamlIlEagerParentStackProvider for XamlIlParentStackProviderWrapper {
    fn direct_parents_stack(&self) -> Rc<Vec<BoxedValue>> {
        if let Some(stack) = &*self.direct_parents_stack.borrow() {
            return stack.clone();
        }
        let mut parents = self.provider.parents();
        parents.reverse();
        let stack = Rc::new(parents);
        *self.direct_parents_stack.borrow_mut() = Some(stack.clone());
        stack
    }

    fn parent_provider(&self) -> Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>> {
        None
    }
}
