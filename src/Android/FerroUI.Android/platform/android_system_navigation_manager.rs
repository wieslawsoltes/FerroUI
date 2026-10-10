//! The system navigation manager of a top-level: the back request of the
//! activity as the back request of the framework.

use crate::i_android_navigation_service::{AndroidBackRequestedEventArgs, IActivityNavigationService};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::platform::ISystemNavigationManagerImpl;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use std::cell::RefCell;
use std::rc::Rc;

type BackRequestedHandlers = Rc<HandlerList<dyn Fn(&Rc<RoutedEventArgs>)>>;

pub struct AndroidSystemNavigationManagerImpl {
    back_requested: BackRequestedHandlers,
    /// The subscription to the navigation service (`BackRequested +=`).
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl AndroidSystemNavigationManagerImpl {
    pub fn new(navigation_service: Option<Rc<dyn IActivityNavigationService>>) -> Rc<Self> {
        let back_requested: BackRequestedHandlers = Rc::new(HandlerList::new());
        let subscription = navigation_service.map(|navigation_service| {
            let back_requested = back_requested.clone();
            navigation_service.back_requested(Rc::new(move |e: &AndroidBackRequestedEventArgs| {
                Self::on_back_requested(&back_requested, e);
            }))
        });
        Rc::new(Self { back_requested, subscription: RefCell::new(subscription) })
    }

    fn on_back_requested(back_requested: &BackRequestedHandlers, e: &AndroidBackRequestedEventArgs) {
        let routed_event_args = Rc::new(RoutedEventArgs::new());

        for (_, handler) in back_requested.snapshot().iter() {
            handler(&routed_event_args);
        }

        e.set_handled(routed_event_args.handled());
    }
}

impl ISystemNavigationManagerImpl for AndroidSystemNavigationManagerImpl {
    fn back_requested(&self, handler: Rc<dyn Fn(&Rc<RoutedEventArgs>)>) -> Rc<dyn IDisposable> {
        let token = self.back_requested.add(handler);
        let back_requested = self.back_requested.clone();
        Disposable::create(move || {
            back_requested.remove(token);
        })
    }
}

impl IDisposable for AndroidSystemNavigationManagerImpl {
    fn dispose(&self) {
        let subscription = self.subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of the manager.
    use super::*;
    use std::cell::Cell;

    #[derive(Default)]
    struct Service {
        handlers: Rc<HandlerList<dyn Fn(&AndroidBackRequestedEventArgs)>>,
    }

    impl Service {
        /// The back button: whether the request was handled.
        fn back(&self) -> bool {
            let e = AndroidBackRequestedEventArgs::new();
            for (_, handler) in self.handlers.snapshot().iter() {
                handler(&e);
            }
            e.handled()
        }
    }

    impl IActivityNavigationService for Service {
        fn back_requested(&self, handler: Rc<dyn Fn(&AndroidBackRequestedEventArgs)>) -> Rc<dyn IDisposable> {
            let token = self.handlers.add(handler);
            let handlers = self.handlers.clone();
            Disposable::create(move || {
                handlers.remove(token);
            })
        }
    }

    #[test]
    fn a_back_request_of_the_activity_is_handled_when_a_handler_of_the_framework_handles_it() {
        let service = Rc::new(Service::default());
        let manager = AndroidSystemNavigationManagerImpl::new(Some(service.clone()));

        // Nobody listens: not handled.
        assert!(!service.back());

        let (seen, handle) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(true)));
        let subscription = manager.back_requested(Rc::new({
            let (seen, handle) = (seen.clone(), handle.clone());
            move |e: &Rc<RoutedEventArgs>| {
                seen.set(seen.get() + 1);
                e.set_handled(handle.get());
            }
        }));
        assert!(service.back());
        handle.set(false);
        assert!(!service.back());
        assert_eq!(seen.get(), 2);

        subscription.dispose();
        service.back();
        assert_eq!(seen.get(), 2);
    }

    #[test]
    fn a_disposed_manager_no_longer_listens_and_one_without_a_service_never_does() {
        let service = Rc::new(Service::default());
        let manager = AndroidSystemNavigationManagerImpl::new(Some(service.clone()));
        let seen = Rc::new(Cell::new(0));
        let _subscription = manager.back_requested(Rc::new({
            let seen = seen.clone();
            move |_: &Rc<RoutedEventArgs>| seen.set(seen.get() + 1)
        }));
        manager.dispose();
        manager.dispose();
        service.back();
        assert_eq!(seen.get(), 0);
        assert!(service.handlers.is_empty());

        let without = AndroidSystemNavigationManagerImpl::new(None);
        let _subscription = without.back_requested(Rc::new(|_: &Rc<RoutedEventArgs>| {}));
        without.dispose();
    }
}
