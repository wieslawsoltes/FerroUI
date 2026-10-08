use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A value that is available until it is disposed.
pub trait IScopedResource<T> {
    /// The value.
    ///
    /// Panics when the resource has been disposed.
    fn value(&self) -> T;

    /// Releases the value. Further calls do nothing.
    fn dispose(&self);
}

/// An [`IScopedResource`] that runs an action when it is disposed.
pub struct ScopedResource<T> {
    disposed: Cell<bool>,
    value: RefCell<Option<T>>,
    dispose: RefCell<Option<Box<dyn FnOnce()>>>,
}

impl<T: Clone + 'static> ScopedResource<T> {
    fn new(value: T, dispose: Box<dyn FnOnce()>) -> Self {
        Self { disposed: Cell::new(false), value: RefCell::new(Some(value)), dispose: RefCell::new(Some(dispose)) }
    }

    pub fn create(value: T, dispose: impl FnOnce() + 'static) -> Rc<dyn IScopedResource<T>> {
        Rc::new(Self::new(value, Box::new(dispose)))
    }
}

impl<T: Clone + 'static> IScopedResource<T> for ScopedResource<T> {
    fn dispose(&self) {
        if !self.disposed.replace(true) {
            let disp = self.dispose.borrow_mut().take();
            let value = self.value.borrow_mut().take();
            drop(value);
            if let Some(disp) = disp {
                disp();
            }
        }
    }

    fn value(&self) -> T {
        if self.disposed.get() {
            panic!("Cannot access a disposed object: ScopedResource");
        }
        match self.value.borrow().as_ref() {
            Some(value) => value.clone(),
            None => panic!("Cannot access a disposed object: ScopedResource"),
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the original has no tests of this type.
    use super::*;

    #[test]
    fn dispose_runs_the_action_once() {
        let count = Rc::new(Cell::new(0));
        let c = count.clone();
        let resource = ScopedResource::create(5, move || c.set(c.get() + 1));

        assert_eq!(5, resource.value());
        assert_eq!(0, count.get());

        resource.dispose();
        resource.dispose();
        assert_eq!(1, count.get());
    }

    #[test]
    #[should_panic(expected = "Cannot access a disposed object")]
    fn value_of_a_disposed_resource_throws() {
        let resource = ScopedResource::create(5, || {});
        resource.dispose();
        resource.value();
    }
}
