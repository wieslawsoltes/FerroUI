use super::{CompositeDisposable, IDisposable};
use std::rc::Rc;

/// Adds a disposable to a group in the expression that creates it.
pub trait DisposableMixin: Sized {
    /// Adds the disposable to `composite_disposable` and returns it.
    fn dispose_with(self, composite_disposable: &CompositeDisposable) -> Self;
}

impl DisposableMixin for Rc<dyn IDisposable> {
    fn dispose_with(self, composite_disposable: &CompositeDisposable) -> Self {
        composite_disposable.add(self.clone());
        self
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the original has no tests of this type.
    use super::*;
    use crate::reactive::Disposable;
    use std::cell::Cell;

    #[test]
    fn dispose_with_adds_the_disposable_to_the_group() {
        let disposed = Rc::new(Cell::new(false));
        let d = disposed.clone();
        let group = CompositeDisposable::new();

        let item = Disposable::create(move || d.set(true)).dispose_with(&group);

        assert!(group.contains(&item));
        group.dispose();
        assert!(disposed.get());
    }
}
