//! The subscription overloads of `ObservableExtensions` (`Observable.Extensions.cs`).


/// The subscription methods of `ObservableExtensions` (the provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! subscribe_methods {
    () => {
        /// Subscribes an observer, with the safeguards of a subscription (`SubscribeSafe`): an
        /// observer that panics while it handles a notification disposes the subscription first.
        fn subscribe_safe(&self, observer: std::rc::Rc<dyn ferroui_system::IObserver<T>>) -> std::rc::Rc<dyn ferroui_system::IDisposable> {
            // Every observable of the crate subscribes through the safeguards of a sink or of user
            // code (`SubscribeSafe` short-cuts producers the same way upstream).
            self.observable().subscribe(observer)
        }

        /// Subscribes and ignores every notification but errors, which panic (`Subscribe()`).
        fn subscribe_ignore(&self) -> std::rc::Rc<dyn ferroui_system::IDisposable> {
            self.observable().subscribe(std::rc::Rc::new(crate::observer::AnonymousObserver::new(|_| {}, crate::observer::throw, || {})))
        }

        /// Subscribes a function of values; an error panics (`Subscribe(Action<T>)`).
        fn subscribe_next(&self, on_next: impl Fn(T) + 'static) -> std::rc::Rc<dyn ferroui_system::IDisposable> {
            self.observable().subscribe(std::rc::Rc::new(crate::observer::AnonymousObserver::new(on_next, crate::observer::throw, || {})))
        }

        /// Subscribes functions of values and errors (`Subscribe(Action<T>, Action<Exception>)`).
        fn subscribe_next_error(
            &self,
            on_next: impl Fn(T) + 'static,
            on_error: impl Fn(ferroui_system::ObservableError) + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IDisposable> {
            self.observable().subscribe(std::rc::Rc::new(crate::observer::AnonymousObserver::new(on_next, on_error, || {})))
        }

        /// Subscribes functions of values and the end; an error panics (`Subscribe(Action<T>, Action)`).
        fn subscribe_next_completed(&self, on_next: impl Fn(T) + 'static, on_completed: impl Fn() + 'static) -> std::rc::Rc<dyn ferroui_system::IDisposable> {
            self.observable().subscribe(std::rc::Rc::new(crate::observer::AnonymousObserver::new(on_next, crate::observer::throw, on_completed)))
        }

        /// Subscribes functions of all three notifications (`Subscribe(Action<T>, Action<Exception>, Action)`).
        fn subscribe_all(
            &self,
            on_next: impl Fn(T) + 'static,
            on_error: impl Fn(ferroui_system::ObservableError) + 'static,
            on_completed: impl Fn() + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IDisposable> {
            self.observable().subscribe(std::rc::Rc::new(crate::observer::AnonymousObserver::new(on_next, on_error, on_completed)))
        }
    };
}
pub(crate) use subscribe_methods;
