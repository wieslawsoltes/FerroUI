use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::platform::ISystemNavigationManagerImpl;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use std::rc::Rc;

/// The system navigation of the page: the back navigation of the browser,
/// once [`add_back_handler`](crate::interop::navigation_helper::add_back_handler)
/// has made it a back request.
#[derive(Default)]
pub struct BrowserSystemNavigationManagerImpl {
    back_requested: Rc<HandlerList<dyn Fn(&Rc<RoutedEventArgs>)>>,
}

impl BrowserSystemNavigationManagerImpl {
    /// Raises the back request. Returns whether a handler handled it.
    pub fn on_back_requested(&self) -> bool {
        let routed_event_args = Rc::new(RoutedEventArgs::new());

        for (_, handler) in self.back_requested.snapshot().iter() {
            handler(&routed_event_args);
        }

        routed_event_args.handled()
    }
}

impl ISystemNavigationManagerImpl for BrowserSystemNavigationManagerImpl {
    fn back_requested(&self, handler: Rc<dyn Fn(&Rc<RoutedEventArgs>)>) -> Rc<dyn IDisposable> {
        let token = self.back_requested.add(handler);
        let handlers = self.back_requested.clone();
        Disposable::create(move || {
            handlers.remove(token);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn a_back_request_is_not_handled_without_handlers() {
        assert!(!BrowserSystemNavigationManagerImpl::default().on_back_requested());
    }

    #[test]
    fn a_back_request_is_handled_when_a_handler_marks_it_handled() {
        let manager = BrowserSystemNavigationManagerImpl::default();
        let calls = Rc::new(Cell::new(0));
        let subscription = manager.back_requested({
            let calls = calls.clone();
            Rc::new(move |args: &Rc<RoutedEventArgs>| {
                calls.set(calls.get() + 1);
                args.set_handled(true);
            })
        });

        assert!(manager.on_back_requested());
        assert_eq!(1, calls.get());

        subscription.dispose();
        assert!(!manager.on_back_requested());
        assert_eq!(1, calls.get());
    }

    #[test]
    fn every_handler_sees_the_request() {
        let manager = BrowserSystemNavigationManagerImpl::default();
        let seen = Rc::new(Cell::new(0));
        let mut subscriptions = Vec::new();
        for _ in 0..2 {
            let seen = seen.clone();
            subscriptions.push(manager.back_requested(Rc::new(move |_: &Rc<RoutedEventArgs>| seen.set(seen.get() + 1))));
        }

        assert!(!manager.on_back_requested());
        assert_eq!(2, seen.get());
    }
}
