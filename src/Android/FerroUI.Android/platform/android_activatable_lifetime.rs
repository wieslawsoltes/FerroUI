use crate::i_ferro_activity::IFerroActivity;
use ferroui_base::reactive::IDisposable;
use ferroui_controls::application_lifetimes::{
    ActivatableLifetimeBase, ActivatedEventArgs, ActivationKind, IActivatableLifetime,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Activation of the application follows its activities.
pub struct AndroidActivatableLifetime {
    base: Rc<ActivatableLifetimeBase>,
    main_activity: RefCell<Option<Rc<dyn IFerroActivity>>>,
    intend_activity: RefCell<Option<Rc<dyn IFerroActivity>>>,
    main_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    intend_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}

fn dispose_all(subscriptions: &RefCell<Vec<Rc<dyn IDisposable>>>) {
    let subscriptions = std::mem::take(&mut *subscriptions.borrow_mut());
    for subscription in subscriptions {
        subscription.dispose();
    }
}

fn is_intend_activation(kind: ActivationKind) -> bool {
    matches!(kind, ActivationKind::File | ActivationKind::OpenUri)
}

impl AndroidActivatableLifetime {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            base: ActivatableLifetimeBase::new(),
            main_activity: RefCell::new(None),
            intend_activity: RefCell::new(None),
            main_subscriptions: RefCell::new(Vec::new()),
            intend_subscriptions: RefCell::new(Vec::new()),
        })
    }

    /// While we primarily handle main activity lifecycle events.
    /// Any secondary activity might send protocol or file activation.
    pub fn current_intend_activity(&self) -> Option<Rc<dyn IFerroActivity>> {
        self.intend_activity.borrow().clone()
    }

    pub fn set_current_intend_activity(&self, value: Option<Rc<dyn IFerroActivity>>) {
        dispose_all(&self.intend_subscriptions);

        let previous = self.intend_activity.replace(value.clone());
        drop(previous);

        if let Some(intend_activity) = value {
            let base = self.base.clone();
            let subscription = intend_activity.activated(Rc::new(move |e: &ActivatedEventArgs| {
                if is_intend_activation(e.kind()) {
                    base.on_activated(e.clone());
                }
            }));
            self.intend_subscriptions.borrow_mut().push(subscription);
        }
    }

    pub fn current_main_activity(&self) -> Option<Rc<dyn IFerroActivity>> {
        self.main_activity.borrow().clone()
    }

    pub fn set_current_main_activity(&self, value: Option<Rc<dyn IFerroActivity>>) {
        dispose_all(&self.main_subscriptions);

        let previous = self.main_activity.replace(value.clone());
        drop(previous);

        if let Some(main_activity) = value {
            let base = self.base.clone();
            let activated = main_activity.activated(Rc::new(move |e: &ActivatedEventArgs| {
                if !is_intend_activation(e.kind()) {
                    base.on_activated(e.clone());
                }
            }));
            let base = self.base.clone();
            let deactivated =
                main_activity.deactivated(Rc::new(move |e: &ActivatedEventArgs| base.on_deactivated(e.clone())));
            self.main_subscriptions.borrow_mut().extend([activated, deactivated]);
        }
    }
}

impl IActivatableLifetime for AndroidActivatableLifetime {
    fn activated(&self, handler: Rc<dyn Fn(&ActivatedEventArgs)>) -> Rc<dyn IDisposable> {
        self.base.activated(handler)
    }

    fn deactivated(&self, handler: Rc<dyn Fn(&ActivatedEventArgs)>) -> Rc<dyn IDisposable> {
        self.base.deactivated(handler)
    }

    fn try_leave_background(&self) -> bool {
        self.base.try_leave_background()
    }

    fn try_enter_background(&self) -> bool {
        let main_activity = self.main_activity.borrow().clone();
        main_activity.is_some_and(|main_activity| main_activity.move_task_to_back(true))
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of the lifetime.
    use super::*;
    use ferroui_base::reactive::Disposable;
    use ferroui_base::utilities::HandlerList;
    use ferroui_base::BoxedValue;
    use std::cell::Cell;

    #[derive(Default)]
    struct Activity {
        activated: Rc<HandlerList<dyn Fn(&ActivatedEventArgs)>>,
        deactivated: Rc<HandlerList<dyn Fn(&ActivatedEventArgs)>>,
        moved_to_back: Cell<bool>,
    }

    impl Activity {
        fn raise(handlers: &HandlerList<dyn Fn(&ActivatedEventArgs)>, kind: ActivationKind) {
            for (_, handler) in handlers.snapshot().iter() {
                handler(&ActivatedEventArgs::new(kind));
            }
        }
    }

    impl crate::i_activity_result_handler::IActivityResultHandler for Activity {
        fn activity_result(&self) -> Option<crate::i_activity_result_handler::ActivityResultHandler> {
            None
        }

        fn set_activity_result(&self, _value: Option<crate::i_activity_result_handler::ActivityResultHandler>) {}

        fn request_permissions_result(
            &self,
        ) -> Option<crate::i_activity_result_handler::RequestPermissionsResultHandler> {
            None
        }

        fn set_request_permissions_result(
            &self,
            _value: Option<crate::i_activity_result_handler::RequestPermissionsResultHandler>,
        ) {
        }
    }

    impl crate::i_android_navigation_service::IActivityNavigationService for Activity {
        fn back_requested(
            &self,
            _handler: Rc<dyn Fn(&crate::i_android_navigation_service::AndroidBackRequestedEventArgs)>,
        ) -> Rc<dyn IDisposable> {
            ferroui_base::reactive::Disposable::create(|| {})
        }
    }

    impl IFerroActivity for Activity {
        fn content(&self) -> Option<BoxedValue> {
            None
        }

        fn set_content(&self, _value: Option<BoxedValue>) {}

        fn activated(&self, handler: Rc<dyn Fn(&ActivatedEventArgs)>) -> Rc<dyn IDisposable> {
            let token = self.activated.add(handler);
            let handlers = self.activated.clone();
            Disposable::create(move || {
                handlers.remove(token);
            })
        }

        fn deactivated(&self, handler: Rc<dyn Fn(&ActivatedEventArgs)>) -> Rc<dyn IDisposable> {
            let token = self.deactivated.add(handler);
            let handlers = self.deactivated.clone();
            Disposable::create(move || {
                handlers.remove(token);
            })
        }

        fn move_task_to_back(&self, non_root: bool) -> bool {
            self.moved_to_back.set(non_root);
            true
        }
    }

    fn counted(lifetime: &AndroidActivatableLifetime) -> (Rc<RefCell<Vec<ActivationKind>>>, Rc<Cell<i32>>) {
        let activated = Rc::new(RefCell::new(Vec::new()));
        let deactivated = Rc::new(Cell::new(0));
        let _ = lifetime.activated(Rc::new({
            let activated = activated.clone();
            move |e: &ActivatedEventArgs| activated.borrow_mut().push(e.kind())
        }));
        let _ = lifetime.deactivated(Rc::new({
            let deactivated = deactivated.clone();
            move |_: &ActivatedEventArgs| deactivated.set(deactivated.get() + 1)
        }));
        (activated, deactivated)
    }

    #[test]
    fn the_main_activity_activates_and_deactivates_but_not_for_an_intent() {
        let _scope = ferroui_base::threading::Dispatcher::unit_test_scope();
        let lifetime = AndroidActivatableLifetime::new();
        let (activated, deactivated) = counted(&lifetime);
        let activity = Rc::new(Activity::default());
        lifetime.set_current_main_activity(Some(activity.clone()));

        Activity::raise(&activity.activated, ActivationKind::Background);
        Activity::raise(&activity.activated, ActivationKind::OpenUri);
        Activity::raise(&activity.deactivated, ActivationKind::Background);

        assert_eq!(*activated.borrow(), [ActivationKind::Background]);
        assert_eq!(deactivated.get(), 1);
    }

    #[test]
    fn the_intent_activity_activates_only_for_an_intent() {
        let _scope = ferroui_base::threading::Dispatcher::unit_test_scope();
        let lifetime = AndroidActivatableLifetime::new();
        let (activated, _) = counted(&lifetime);
        let activity = Rc::new(Activity::default());
        lifetime.set_current_intend_activity(Some(activity.clone()));

        Activity::raise(&activity.activated, ActivationKind::Background);
        Activity::raise(&activity.activated, ActivationKind::File);

        assert_eq!(*activated.borrow(), [ActivationKind::File]);
    }

    #[test]
    fn an_activity_that_is_replaced_is_no_longer_listened_to() {
        let _scope = ferroui_base::threading::Dispatcher::unit_test_scope();
        let lifetime = AndroidActivatableLifetime::new();
        let (activated, deactivated) = counted(&lifetime);
        let activity = Rc::new(Activity::default());
        lifetime.set_current_main_activity(Some(activity.clone()));
        lifetime.set_current_main_activity(None);

        Activity::raise(&activity.activated, ActivationKind::Background);
        Activity::raise(&activity.deactivated, ActivationKind::Background);

        assert!(activated.borrow().is_empty());
        assert_eq!(deactivated.get(), 0);
        assert!(activity.activated.is_empty());
    }

    #[test]
    fn entering_the_background_moves_the_task_of_the_main_activity_back() {
        let _scope = ferroui_base::threading::Dispatcher::unit_test_scope();
        let lifetime = AndroidActivatableLifetime::new();
        assert!(!lifetime.try_enter_background());

        let activity = Rc::new(Activity::default());
        lifetime.set_current_main_activity(Some(activity.clone()));

        assert!(lifetime.try_enter_background());
        assert!(activity.moved_to_back.get());
    }
}
