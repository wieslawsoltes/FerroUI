//! An observable of binding values that one observer subscribes to and the
//! benchmark publishes through.

use ferroui_base::data::BindingValue;
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub struct TestBindingObservable<T> {
    this: Weak<TestBindingObservable<T>>,
    value: T,
    observer: RefCell<Option<Rc<dyn IObserver<BindingValue<T>>>>>,
}

impl<T: Clone + 'static> TestBindingObservable<T> {
    pub fn new(initial_value: T) -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), value: initial_value, observer: RefCell::new(None) })
    }

    fn observer(&self) -> Option<Rc<dyn IObserver<BindingValue<T>>>> {
        self.observer.borrow().clone()
    }

    pub fn on_next(&self, value: T) {
        if let Some(observer) = self.observer() {
            observer.on_next(BindingValue::new(value));
        }
    }

    pub fn publish_completed(&self) {
        let observer = self.observer.borrow_mut().take();
        if let Some(observer) = observer {
            observer.on_completed();
        }
    }

    pub fn publish_error(&self, error: ObservableError) {
        let observer = self.observer.borrow_mut().take();
        if let Some(observer) = observer {
            observer.on_error(error);
        }
    }
}

impl<T: Clone + 'static> IObservable<BindingValue<T>> for TestBindingObservable<T> {
    /// # Panics
    ///
    /// Panics when the observable is subscribed to a second time.
    fn subscribe(&self, observer: Rc<dyn IObserver<BindingValue<T>>>) -> Rc<dyn IDisposable> {
        if self.observer.borrow().is_some() {
            panic!("The observable can only be subscribed once.");
        }

        *self.observer.borrow_mut() = Some(observer.clone());
        observer.on_next(BindingValue::new(self.value.clone()));
        self.this.upgrade().expect("the observable is alive while it is subscribed to")
    }
}

impl<T: Clone + 'static> IDisposable for TestBindingObservable<T> {
    fn dispose(&self) {
        *self.observer.borrow_mut() = None;
    }
}
