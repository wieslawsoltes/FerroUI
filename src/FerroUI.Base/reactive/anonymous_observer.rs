use super::{IObserver, ObservableError};

/// An observer built from closures.
pub struct AnonymousObserver<T> {
    on_next: Box<dyn Fn(T)>,
    on_error: Option<Box<dyn Fn(ObservableError)>>,
    on_completed: Option<Box<dyn Fn()>>,
}

impl<T> AnonymousObserver<T> {
    pub fn new(on_next: impl Fn(T) + 'static) -> Self {
        Self { on_next: Box::new(on_next), on_error: None, on_completed: None }
    }

    pub fn with_error(mut self, on_error: impl Fn(ObservableError) + 'static) -> Self {
        self.on_error = Some(Box::new(on_error));
        self
    }

    pub fn with_completed(mut self, on_completed: impl Fn() + 'static) -> Self {
        self.on_completed = Some(Box::new(on_completed));
        self
    }
}

impl<T> IObserver<T> for AnonymousObserver<T> {
    fn on_next(&self, value: T) {
        (self.on_next)(value)
    }

    fn on_error(&self, error: ObservableError) {
        if let Some(f) = &self.on_error {
            f(error)
        }
    }

    fn on_completed(&self) {
        if let Some(f) = &self.on_completed {
            f()
        }
    }
}
