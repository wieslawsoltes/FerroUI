//! Port of `EagerParentStackEnumerator.cs`.

use crate::xaml_il::runtime::IFerroXamlIlEagerParentStackProvider;
use crate::FromXamlObject;
use ferroui_base::BoxedValue;
use std::rc::Rc;

/// Walks the parents of an eager parent stack provider, the immediate
/// parent first, without building the list of all parents.
pub struct EagerParentStackEnumerator {
    provider: Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>>,
    current_parents_stack: Option<Rc<Vec<BoxedValue>>>,
    /// Only valid while `current_parents_stack` is set.
    current_index: usize,
}

impl EagerParentStackEnumerator {
    pub fn new(provider: Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>>) -> Self {
        Self { provider, current_parents_stack: None, current_index: 0 }
    }

    /// The next parent, or `None` when there are no more.
    pub fn try_get_next(&mut self) -> Option<BoxedValue> {
        while let Some(provider) = &self.provider {
            if self.current_parents_stack.is_none() {
                let stack = provider.direct_parents_stack();
                self.current_index = stack.len();
                self.current_parents_stack = Some(stack);
            }

            if self.current_index > 0 {
                self.current_index -= 1;
                let stack = self.current_parents_stack.as_ref().expect("the stack was just set");
                return Some(stack[self.current_index].clone());
            }

            self.current_parents_stack = None;
            self.provider = provider.parent_provider();
        }

        None
    }

    /// The next parent that is a `T`, or `None` when there are no more.
    pub fn try_get_next_of_type<T: FromXamlObject>(&mut self) -> Option<T> {
        while let Some(parent) = self.try_get_next() {
            if let Some(typed_parent) = T::from_xaml_object(&parent) {
                return Some(typed_parent);
            }
        }

        None
    }
}
